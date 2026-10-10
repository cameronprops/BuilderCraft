//! Bounded, allocation-free 3D least-squares plane fit for CAD and scans.
//! Covariance coordinates are measured relative to a source point, then
//! scaled to avoid cancellation and extreme drawing-unit overflow.
use crate::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BestFitPlane {
    pub origin: Vec3,
    pub normal: Vec3,
    pub rms_distance: f64,
    pub max_distance: f64,
}

/// Fit a total-least-squares plane to at least three non-collinear finite
/// positions. Returns None for nearly collinear, invalid or oversized input.
pub fn best_fit_plane(points: &[Vec3]) -> Option<BestFitPlane> {
    if points.len() < 3 || points.len() > 100_000 {
        return None;
    }
    let base = points[0];
    if !base.is_finite() {
        return None;
    }
    let mut sum = Vec3::ZERO;
    let mut extent: f64 = 0.0;
    for &point in points {
        if !point.is_finite() || [point.x, point.y, point.z].iter().any(|c| c.abs() > 1e12) {
            return None;
        }
        let local = point - base;
        extent = extent.max(local.x.abs().max(local.y.abs()).max(local.z.abs()));
        sum = sum + local;
    }
    if !extent.is_finite() || extent < 1e-12 {
        return None;
    }
    let origin = base + sum * (1.0 / points.len() as f64);
    if !origin.is_finite() {
        return None;
    }
    let scale = extent.recip();
    let mut a = [[0.; 3]; 3];
    for &point in points {
        let p = (point - origin) * scale;
        let d = [p.x, p.y, p.z];
        for row in 0..3 {
            for col in row..3 {
                a[row][col] += d[row] * d[col];
            }
        }
    }
    let n = points.len() as f64;
    for row in 0..3 {
        for col in row..3 {
            a[row][col] /= n;
            a[col][row] = a[row][col];
        }
    }
    // Symmetric Jacobi eigen solve. The eigenvector associated with the
    // smallest eigenvalue is the least-squares plane normal.
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..36 {
        let (mut p, mut q) = (0, 1);
        for (i, j) in [(0, 2), (1, 2)] {
            if a[i][j].abs() > a[p][q].abs() {
                (p, q) = (i, j);
            }
        }
        if a[p][q].abs() < 1e-14 {
            break;
        }
        let tau = (a[q][q] - a[p][p]) / (2. * a[p][q]);
        let t = (if tau >= 0. { 1. } else { -1. }) / (tau.abs() + (1. + tau * tau).sqrt());
        let c = (1. + t * t).sqrt().recip();
        let s = t * c;
        let (app, aqq, apq) = (a[p][p], a[q][q], a[p][q]);
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.;
        a[q][p] = 0.;
        for k in 0..3 {
            if k != p && k != q {
                let (akp, akq) = (a[k][p], a[k][q]);
                a[k][p] = c * akp - s * akq;
                a[p][k] = a[k][p];
                a[k][q] = s * akp + c * akq;
                a[q][k] = a[k][q];
            }
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    // A valid plane must have two measurable axes, not a single line.
    if !a[order[1]][order[1]].is_finite() || a[order[1]][order[1]] <= a[order[2]][order[2]].max(1e-15) * 1e-12 {
        return None;
    }
    let candidate = Vec3::new(v[0][order[0]], v[1][order[0]], v[2][order[0]]);
    let magnitude = candidate.x.hypot(candidate.y).hypot(candidate.z);
    if !magnitude.is_finite() || magnitude <= 0. {
        return None;
    }
    let normal = candidate * magnitude.recip();
    let mut squared = 0.;
    let mut max_distance: f64 = 0.;
    for &p in points {
        let distance = (p - origin).dot(normal).abs();
        squared += distance * distance;
        max_distance = max_distance.max(distance);
    }
    let rms_distance = (squared / n).sqrt();
    (rms_distance.is_finite() && max_distance.is_finite()).then_some(BestFitPlane { origin, normal, rms_distance, max_distance })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_tilted_plane_and_exposes_scan_deviation() {
        let points = [Vec3::new(0., 0., 1.), Vec3::new(2., 0., 3.), Vec3::new(0., 4., 9.), Vec3::new(2., 4., 11.)];
        let plane = best_fit_plane(&points).unwrap();
        assert!(plane.max_distance < 1e-9);
        assert!((plane.normal.dot(Vec3::new(-1., -2., 1.))).abs() / 6_f64.sqrt() > 0.99999);
        let mut noisy = points;
        noisy[0].z += 0.15;
        let fit = best_fit_plane(&noisy).unwrap();
        assert!(fit.rms_distance > 0.);
        assert!(fit.max_distance < 0.15);
    }

    #[test]
    fn vertical_and_translated_plane_stays_stable() {
        let points = [Vec3::new(3e8, 5e8, -2e8), Vec3::new(3e8, 5e8 + 3., -2e8), Vec3::new(3e8, 5e8 + 3., -2e8 + 5.), Vec3::new(3e8, 5e8, -2e8 + 5.)];
        let fit = best_fit_plane(&points).unwrap();
        assert!(fit.normal.x.abs() > 0.999999);
        assert!(fit.max_distance < 1e-8);
    }

    #[test]
    fn rejects_collinear_nonfinite_and_unsupported_budgets() {
        assert!(best_fit_plane(&[Vec3::ZERO, Vec3::new(1., 0., 0.), Vec3::new(2., 0., 0.)]).is_none());
        assert!(best_fit_plane(&[Vec3::ZERO, Vec3::new(1., 0., 0.)]).is_none());
        assert!(best_fit_plane(&[Vec3::ZERO, Vec3::new(1., 0., 0.), Vec3::new(f64::NAN, 0., 1.)]).is_none());
    }
}
