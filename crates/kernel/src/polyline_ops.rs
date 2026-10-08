//! Bounded linear-polyline measurements and arc-length divisions.
//! These operations do not approximate or divide NURBS curves.
use crate::{KernelError, Result, point_distance, point_interpolate};
use cadcraft_geom::Vec3;

const MAX_POINTS: usize = 100_000;
const MAX_DIVISIONS: usize = 100_000;

fn validate(points: &[Vec3]) -> Result<()> {
    if points.len() < 2 || points.len() > MAX_POINTS {
        return Err(KernelError::Invalid("polyline point count"));
    }
    if points.iter().any(|p| !p.is_finite() || [p.x, p.y, p.z].iter().any(|n| n.abs() > 1e12)) {
        return Err(KernelError::Invalid("polyline coordinate"));
    }
    Ok(())
}

/// Length of every segment, retaining zeros for coincident consecutive vertices.
pub fn polyline_segment_lengths(points: &[Vec3]) -> Result<Vec<f64>> {
    validate(points)?;
    let mut lengths = Vec::new();
    lengths.try_reserve_exact(points.len() - 1).map_err(|_| KernelError::Budget)?;
    for segment in points.windows(2) {
        lengths.push(point_distance(segment[0], segment[1])?);
    }
    Ok(lengths)
}

/// Sum of all segments in the document's current linear units.
pub fn polyline_length(points: &[Vec3]) -> Result<f64> {
    validate(points)?;
    let mut total = 0.0;
    for segment in points.windows(2) {
        total += point_distance(segment[0], segment[1])?;
        if !total.is_finite() {
            return Err(KernelError::Invalid("polyline length overflow"));
        }
    }
    Ok(total)
}

fn sample_at(points: &[Vec3], distance: f64, total: f64) -> Result<Vec3> {
    if distance <= 0.0 { return Ok(points[0]); }
    if distance >= total { return Ok(points[points.len() - 1]); }
    let mut covered = 0.0;
    for segment in points.windows(2) {
        let length = point_distance(segment[0], segment[1])?;
        if length > 0.0 && distance <= covered + length {
            let t = ((distance - covered) / length).clamp(0.0, 1.0);
            return point_interpolate(segment[0], segment[1], t);
        }
        covered += length;
    }
    Ok(points[points.len() - 1])
}

/// Divide into `count` equal arc-length pieces; return count+1 points including endpoints.
/// Zero-length segments are skipped during sampling.
pub fn polyline_divide_count(points: &[Vec3], count: usize) -> Result<Vec<Vec3>> {
    let total = polyline_length(points)?;
    if count == 0 || count >= MAX_DIVISIONS || total == 0.0 {
        return Err(KernelError::Invalid("polyline division count or length"));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(count + 1).map_err(|_| KernelError::Budget)?;
    for i in 0..=count {
        output.push(sample_at(points, total * (i as f64 / count as f64), total)?);
    }
    Ok(output)
}

/// Sample at regular arc-length spacing from the first vertex.
/// The final endpoint is included even if the final interval is shorter.
pub fn polyline_divide_distance(points: &[Vec3], spacing: f64) -> Result<Vec<Vec3>> {
    let total = polyline_length(points)?;
    if !spacing.is_finite() || spacing <= 0.0 || total == 0.0 {
        return Err(KernelError::Invalid("polyline spacing or length"));
    }
    let intervals = (total / spacing).ceil();
    if !intervals.is_finite() || intervals >= MAX_DIVISIONS as f64 {
        return Err(KernelError::Budget);
    }
    let mut output = Vec::new();
    output.try_reserve_exact(intervals as usize + 1).map_err(|_| KernelError::Budget)?;
    output.push(points[0]);
    let mut index = 1usize;
    while (index as f64) * spacing < total {
        output.push(sample_at(points, (index as f64) * spacing, total)?);
        index += 1;
    }
    output.push(points[points.len() - 1]);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> Vec<Vec3> {
        vec![Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), Vec3::new(3.0, 4.0, 0.0)]
    }

    #[test]
    fn segment_and_total_lengths() {
        assert_eq!(polyline_segment_lengths(&path()), Ok(vec![3.0, 4.0]));
        assert_eq!(polyline_length(&path()), Ok(7.0));
    }

    #[test]
    fn divides_across_a_corner() {
        assert_eq!(polyline_divide_count(&path(), 2), Ok(vec![
            Vec3::ZERO, Vec3::new(3.0, 0.5, 0.0), Vec3::new(3.0, 4.0, 0.0)
        ]));
        assert_eq!(polyline_divide_distance(&path(), 2.0), Ok(vec![
            Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0), Vec3::new(3.0, 3.0, 0.0),
            Vec3::new(3.0, 4.0, 0.0)
        ]));
    }

    #[test]
    fn exact_spacing_does_not_duplicate_endpoint() {
        let line = [Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)];
        assert_eq!(polyline_divide_distance(&line, 2.0), Ok(vec![
            line[0], Vec3::new(2.0, 0.0, 0.0), line[1]
        ]));
    }

    #[test]
    fn duplicate_vertices_do_not_prevent_sampling() {
        let line = [Vec3::ZERO, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)];
        assert_eq!(polyline_segment_lengths(&line), Ok(vec![0.0, 2.0]));
        assert_eq!(polyline_divide_count(&line, 2), Ok(vec![
            line[0], Vec3::new(1.0, 0.0, 0.0), line[2]
        ]));
    }

    #[test]
    fn rejects_invalid_input_and_excessive_divisions() {
        let line = path();
        assert!(polyline_length(&[]).is_err());
        assert!(polyline_length(&[Vec3::ZERO, Vec3::new(f64::NAN, 0.0, 0.0)]).is_err());
        assert!(polyline_divide_count(&line, 0).is_err());
        assert!(polyline_divide_count(&line, MAX_DIVISIONS).is_err());
        assert!(polyline_divide_distance(&line, 0.0).is_err());
        assert!(polyline_divide_distance(&line, 1e-12).is_err());
        assert!(polyline_divide_count(&[Vec3::ZERO, Vec3::ZERO], 2).is_err());
    }
}
