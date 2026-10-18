//! Canonical rail station and frame solver for arrays, sweeps, pipes and track.
//! Explicitly polyline/arc-length based; exact NURBS derivatives and tolerance
//! certified seam closure are separate acceptance stages.
use crate::{KernelError, Result, polyline_divide_count};
use cadcraft_geom::Vec3;

pub const MAX_RAIL_POINTS: usize = 1024;
pub const MAX_RAIL_STATIONS: usize = 256;
const LIMIT: f64 = 1e12;
const EPS: f64 = 1e-10;

pub(crate) fn finite(p: Vec3) -> bool {
    p.is_finite() && p.x.abs().max(p.y.abs()).max(p.z.abs()) <= LIMIT
}
pub(crate) fn normalized(p: Vec3) -> Result<Vec3> {
    if !finite(p) {
        return Err(KernelError::Invalid("nonfinite frame vector"));
    }
    let d = p.len();
    if !d.is_finite() || d < EPS {
        return Err(KernelError::Invalid("zero frame vector"));
    }
    Ok(p * (1. / d))
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailFrame {
    pub origin: Vec3,
    pub tangent: Vec3,
    pub side: Vec3,
    pub up: Vec3,
}
impl RailFrame {
    pub fn map(self, along: f64, side: f64, up: f64) -> Vec3 {
        self.origin + self.tangent * along + self.side * side + self.up * up
    }
}
fn rodrigues(v: Vec3, axis: Vec3, cosine: f64, sine: f64) -> Vec3 {
    v * cosine + axis.cross(v) * sine + axis * (axis.dot(v) * (1. - cosine))
}
pub(crate) fn transport_up(tangent: Vec3, up: Vec3, next: Vec3) -> Result<Vec3> {
    let next = normalized(next)?;
    let axis = tangent.cross(next);
    let sine = axis.len();
    let cosine = tangent.dot(next).clamp(-1., 1.);
    let rotated = if sine < EPS {
        // Tangent reversal is not uniquely defined; retain guide orientation.
        up
    } else {
        rodrigues(up, axis * (1. / sine), cosine, sine)
    };
    normalized(rotated - next * rotated.dot(next))
}
/// Deterministic equal-length stations, with outgoing tangents at corners.
/// Frame transport suppresses twist on straight sections; no closed-loop
/// seam twist correction is implied.
pub fn rail_frames(path: &[Vec3], stations: usize, guide_up: Vec3) -> Result<Vec<RailFrame>> {
    if !(2..=MAX_RAIL_POINTS).contains(&path.len()) || !(2..=MAX_RAIL_STATIONS).contains(&stations) {
        return Err(KernelError::Budget);
    }
    if path.iter().any(|p| !finite(*p)) || !finite(guide_up) {
        return Err(KernelError::Invalid("rail coordinates or guide"));
    }
    let mut segments: Vec<(f64, Vec3)> = Vec::new();
    let mut length = 0.;
    for edge in path.windows(2) {
        let delta = edge[1] - edge[0];
        let d = delta.len();
        if !d.is_finite() {
            return Err(KernelError::Invalid("rail arc length overflow"));
        }
        if d > EPS {
            segments.push((d, normalized(delta)?));
            length += d;
        }
    }
    if !length.is_finite() || length < EPS {
        return Err(KernelError::Invalid("empty rail"));
    }
    let Some((_, first)) = segments.first().copied() else {
        return Err(KernelError::Invalid("empty rail"));
    };
    let up = normalized(guide_up - first * first.dot(guide_up))?;
    let mut frame = RailFrame { origin: path[0], tangent: first, side: normalized(up.cross(first))?, up };
    let samples = polyline_divide_count(path, stations - 1)?;
    if samples.len() != stations {
        return Err(KernelError::Invalid("rail station count mismatch"));
    }
    let mut frames = Vec::new();
    frames.try_reserve_exact(stations).map_err(|_| KernelError::Budget)?;
    let mut completed = 0.;
    let mut seg = 0usize;
    for (i, point) in samples.into_iter().enumerate() {
        let distance = length * i as f64 / (stations - 1) as f64;
        while seg + 1 < segments.len() && distance >= completed + segments[seg].0 - 1e-10 {
            completed += segments[seg].0;
            seg += 1;
        }
        let next = segments[seg].1;
        let up = transport_up(frame.tangent, frame.up, next)?;
        frame = RailFrame { origin: point, tangent: next, side: normalized(up.cross(next))?, up };
        frames.push(frame);
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    #[test]
    fn corner_and_vertical_path_frames_are_orthogonal_and_equal_station() {
        let rail = [p(0., 0., 0.), p(5., 0., 0.), p(5., 5., 0.), p(5., 5., 5.)];
        let frames = rail_frames(&rail, 4, Vec3::Z).unwrap();
        assert_eq!(frames.len(), 4);
        assert!((frames[2].origin - p(5., 5., 0.)).len() < 1e-9);
        for f in frames {
            assert!((f.tangent.len() - 1.).abs() < 1e-8);
            assert!(f.tangent.dot(f.up).abs() < 1e-8);
            assert!(f.side.dot(f.up).abs() < 1e-8);
            assert!((f.tangent.cross(f.side) - f.up).len() < 1e-8);
        }
    }
    #[test]
    fn invalid_rail_and_up_are_rejected() {
        assert!(rail_frames(&[Vec3::ZERO, Vec3::ZERO], 3, Vec3::Z).is_err());
        assert!(rail_frames(&[Vec3::ZERO, Vec3::new(1., 0., 0.)], 3, Vec3::new(1., 0., 0.)).is_err());
        assert!(rail_frames(&[Vec3::ZERO, Vec3::new(1., 0., 0.)], 257, Vec3::Z).is_err());
    }
}
