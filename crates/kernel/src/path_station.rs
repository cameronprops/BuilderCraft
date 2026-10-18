//! Rhino-style polyline stationing and conservative planar offsets.
//! All distances are in document units. These functions operate on linear
//! polylines, not arbitrary rational NURBS; no implied curve approximation.
use crate::{KernelError, Result, polyline_length};
use cadcraft_geom::Vec3;

const MAX_OFFSET_POINTS: usize = 10_000;
const EPS: f64 = 1e-12;

fn valid_station(points: &[Vec3], station: f64) -> Result<f64> {
    let total = polyline_length(points)?;
    if total <= 0.0 || !station.is_finite() || station < 0.0 || station > total {
        return Err(KernelError::Invalid("station outside nonzero polyline"));
    }
    Ok(total)
}

/// WorldBuilder / ride-track station coordinates; corners retain an exact vertex.
pub fn polyline_point_at_station(points: &[Vec3], station: f64) -> Result<Vec3> {
    let total = valid_station(points, station)?;
    if station == 0.0 {
        return Ok(points[0]);
    }
    if station == total {
        return Ok(points[points.len() - 1]);
    }
    let mut traveled = 0.0;
    for pair in points.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.len();
        if length == 0.0 {
            continue;
        }
        if station <= traveled + length {
            let t = ((station - traveled) / length).clamp(0.0, 1.0);
            return Ok(pair[0] + delta * t);
        }
        traveled += length;
    }
    Ok(points[points.len() - 1])
}

/// Unit tangent at an arc-length station. At an internal corner use the
/// incoming nonzero segment; at station 0 use the first outgoing segment.
/// This is a piecewise-linear tangent, NOT curvature-continuous ride geometry.
pub fn polyline_tangent_at_station(points: &[Vec3], station: f64) -> Result<Vec3> {
    let total = valid_station(points, station)?;
    let mut traveled = 0.0;
    let mut last = None;
    for pair in points.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.len();
        if length == 0.0 {
            continue;
        }
        let tangent = delta * (1.0 / length);
        if station == 0.0 {
            return Ok(tangent);
        }
        if station <= traveled + length || station == total && traveled + length >= total {
            return Ok(tangent);
        }
        traveled += length;
        last = Some(tangent);
    }
    last.ok_or(KernelError::Invalid("zero length polyline"))
}

/// Left-hand XY offset with a bounded miter at each bend. Positive distance
/// offsets to the left of vertex order, negative to the right. Z values are
/// preserved at each original station. A corner exceeding miter_limit fails
/// rather than silently producing giant spikes. Output is an untrimmed
/// polyline: it is NOT a planar-curve offset/region Boolean. Self-intersection,
/// closed loops, reversals, and near-vertical XY edges are rejected.
pub fn polyline_offset_xy(points: &[Vec3], distance: f64, miter_limit: f64) -> Result<Vec<Vec3>> {
    polyline_length(points)?; // coordinate and point-count validation
    if points.len() > MAX_OFFSET_POINTS {
        return Err(KernelError::Budget);
    }
    if !distance.is_finite() || distance.abs() > 1e12 || !miter_limit.is_finite() || !(1.0..=20.0).contains(&miter_limit) {
        return Err(KernelError::Invalid("offset distance or miter limit"));
    }
    if points.first() == points.last() {
        return Err(KernelError::Invalid("closed offset requires corner/loop solver"));
    }
    let mut directions = Vec::new();
    directions.try_reserve_exact(points.len() - 1).map_err(|_| KernelError::Budget)?;
    for pair in points.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let length = dx.hypot(dy);
        if !length.is_finite() || length <= EPS {
            return Err(KernelError::Invalid("offset requires nonzero XY segments"));
        }
        directions.push((dx / length, dy / length));
    }
    if distance == 0.0 {
        return Ok(points.to_vec());
    }
    let mut output = Vec::new();
    output.try_reserve_exact(points.len()).map_err(|_| KernelError::Budget)?;
    for i in 0..points.len() {
        let p = points[i];
        let (nx, ny) = if i == 0 || i == points.len() - 1 {
            let (dx, dy) = if i == 0 { directions[0] } else { directions[directions.len() - 1] };
            (-dy, dx)
        } else {
            let (ax, ay) = directions[i - 1];
            let (bx, by) = directions[i];
            let cross = ax * by - ay * bx;
            let dot = ax * bx + ay * by;
            if cross.abs() <= EPS {
                if dot < 0.0 {
                    return Err(KernelError::Invalid("offset hairpin reversal"));
                }
                (-ay, ax)
            } else {
                let (na_x, na_y) = (-ay, ax);
                let (nb_x, nb_y) = (-by, bx);
                // Intersection of adjacent parallel-offset support lines.
                let shift_x = distance * (nb_x - na_x);
                let shift_y = distance * (nb_y - na_y);
                let travel = (shift_x * by - shift_y * bx) / cross;
                let ox = distance * na_x + travel * ax;
                let oy = distance * na_y + travel * ay;
                if !ox.is_finite() || !oy.is_finite() || ox.hypot(oy) > distance.abs() * miter_limit + 1e-9 {
                    return Err(KernelError::Invalid("offset miter limit exceeded"));
                }
                output.push(Vec3::new(p.x + ox, p.y + oy, p.z));
                continue;
            }
        };
        output.push(Vec3::new(p.x + nx * distance, p.y + ny * distance, p.z));
    }
    if output.iter().any(|p| !p.is_finite()) {
        return Err(KernelError::Invalid("offset coordinate overflow"));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn elbow() -> [Vec3; 3] {
        [Vec3::new(0., 0., 0.), Vec3::new(10., 0., 0.), Vec3::new(10., 10., 0.)]
    }
    #[test]
    fn station_points_exact_endpoints_and_corner() {
        let path = elbow();
        assert_eq!(polyline_point_at_station(&path, 0.), Ok(path[0]));
        assert_eq!(polyline_point_at_station(&path, 10.), Ok(path[1]));
        assert_eq!(polyline_point_at_station(&path, 15.), Ok(Vec3::new(10., 5., 0.)));
        assert_eq!(polyline_point_at_station(&path, 20.), Ok(path[2]));
    }
    #[test]
    fn station_tangent_incoming_at_corner() {
        let path = elbow();
        assert_eq!(polyline_tangent_at_station(&path, 10.), Ok(Vec3::new(1., 0., 0.)));
        assert_eq!(polyline_tangent_at_station(&path, 15.), Ok(Vec3::new(0., 1., 0.)));
        assert_eq!(polyline_tangent_at_station(&path, 20.), Ok(Vec3::new(0., 1., 0.)));
    }
    #[test]
    fn duplicate_vertices_skipped_and_invalid_stations_rejected() {
        let path = [Vec3::ZERO, Vec3::ZERO, Vec3::new(0., 0., 5.)];
        assert_eq!(polyline_tangent_at_station(&path, 0.), Ok(Vec3::new(0., 0., 1.)));
        assert_eq!(polyline_point_at_station(&path, 2.), Ok(Vec3::new(0., 0., 2.)));
        assert!(polyline_point_at_station(&path, -1.).is_err());
        assert!(polyline_tangent_at_station(&path, 6.).is_err());
        assert!(polyline_point_at_station(&path, f64::NAN).is_err());
    }
    #[test]
    fn offsets_elbow_both_sides_with_preserved_z() {
        let mut path = elbow();
        path[1].z = 2.;
        let left = polyline_offset_xy(&path, 1., 4.).unwrap();
        assert_eq!(left, vec![Vec3::new(0., 1., 0.), Vec3::new(9., 1., 2.), Vec3::new(9., 10., 0.)]);
        let right = polyline_offset_xy(&path, -1., 4.).unwrap();
        assert_eq!(right[1], Vec3::new(11., -1., 2.));
    }
    #[test]
    fn offsets_reject_invalid_cases_without_partial_result() {
        let path = elbow();
        assert!(polyline_offset_xy(&path, 1., 0.).is_err());
        assert!(polyline_offset_xy(&path, f64::INFINITY, 4.).is_err());
        assert!(polyline_offset_xy(&[Vec3::ZERO, Vec3::new(0., 0., 5.)], 1., 4.).is_err());
        assert!(polyline_offset_xy(&[Vec3::ZERO, Vec3::new(5., 0., 0.), Vec3::ZERO], 1., 4.).is_err());
        assert!(polyline_offset_xy(&[Vec3::ZERO, Vec3::new(5., 0., 0.), Vec3::new(5.1, 0.01, 0.)], 0.5, 4.).is_ok());
    }
}
