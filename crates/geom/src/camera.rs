//! Orthographic camera math shared by native CAD viewports.
use crate::{Vec2, Vec3};
#[derive(Clone, Copy, Debug)]
pub struct OrthoFrame {
    pub yaw: f64,
    pub pitch: f64,
}
impl OrthoFrame {
    pub fn right(self) -> Vec3 {
        Vec3::new(self.yaw.cos(), -self.yaw.sin(), 0.)
    }
    pub fn up(self) -> Vec3 {
        Vec3::new(-self.yaw.sin() * self.pitch.sin(), -self.yaw.cos() * self.pitch.sin(), self.pitch.cos())
    }
    pub fn project(self, point: Vec3, center: Vec3) -> Vec2 {
        let relative = point - center;
        Vec2::new(relative.dot(self.right()), relative.dot(self.up()))
    }
    /// Positive screen motion of the scene moves the camera in the opposite direction.
    pub fn pan(self, center: Vec3, screen_delta: Vec2, scale: f64) -> Option<Vec3> {
        if !scale.is_finite() || scale <= 0. || !screen_delta.x.is_finite() || !screen_delta.y.is_finite() {
            return None;
        }
        let next = center - self.right() * (screen_delta.x / scale) + self.up() * (screen_delta.y / scale);
        finite(next).then_some(next)
    }
    /// Fit a finite control hull. Bounded traversal avoids unlimited viewport work.
    pub fn fit(self, points: impl IntoIterator<Item = Vec3>, width: f64, height: f64) -> Option<(Vec3, f64)> {
        if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. || !self.yaw.is_finite() || !self.pitch.is_finite() {
            return None;
        }
        let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut lo = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut count = 0;
        for p in points {
            count += 1;
            if count > 100_000 || !finite(p) || p.x.abs().max(p.y.abs()).max(p.z.abs()) > 1e12 {
                return None;
            }
            min = Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
            max = Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
            let q = self.project(p, Vec3::ZERO);
            lo = Vec2::new(lo.x.min(q.x), lo.y.min(q.y));
            hi = Vec2::new(hi.x.max(q.x), hi.y.max(q.y));
        }
        if count == 0 {
            return None;
        }
        let base = (min + max) * 0.5;
        let projected = self.project(base, Vec3::ZERO);
        let target = (lo + hi) * 0.5;
        let center = base + self.right() * (target.x - projected.x) + self.up() * (target.y - projected.y);
        let scale = (width * 0.85 / (hi.x - lo.x).max(1e-6)).min(height * 0.85 / (hi.y - lo.y).max(1e-6));
        if scale < 1e-9 {
            return None;
        }
        let scale = scale.min(1e9);
        Some((center, scale))
    }
}
fn finite(p: Vec3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_planes_and_pan() {
        let top = OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 };
        let q = top.project(Vec3::new(2., 3., 99.), Vec3::ZERO);
        assert!((q.x - 2.).abs() < 1e-12 && (q.y - 3.).abs() < 1e-12);
        let front = OrthoFrame { yaw: 0., pitch: 0. };
        assert_eq!(front.project(Vec3::new(2., 99., 3.), Vec3::ZERO), Vec2::new(2., 3.));
        let right = OrthoFrame { yaw: -std::f64::consts::FRAC_PI_2, pitch: 0. };
        let q = right.project(Vec3::new(99., 2., 3.), Vec3::ZERO);
        assert!((q.x - 2.).abs() < 1e-12 && (q.y - 3.).abs() < 1e-12);
        assert_eq!(front.pan(Vec3::ZERO, Vec2::new(20., 10.), 10.), Some(Vec3::new(-2., 0., 1.)));
        assert!(front.pan(Vec3::ZERO, Vec2::ZERO, 0.).is_none());
    }
    #[test]
    fn fit_off_origin_and_reject_hostile_bounds() {
        let frame = OrthoFrame { yaw: 0.7, pitch: 0.6 };
        let points = [Vec3::new(1e6, -2e6, 3e6), Vec3::new(1e6 + 100., -2e6 + 200., 3e6 + 50.)];
        let (center, scale) = frame.fit(points, 800., 600.).unwrap();
        for p in points {
            let q = frame.project(p, center);
            assert!(q.x.abs() * scale <= 400. && q.y.abs() * scale <= 300.);
        }
        assert!(frame.fit([], 800., 600.).is_none());
        assert!(frame.fit([Vec3::new(f64::NAN, 0., 0.)], 800., 600.).is_none());
        assert!(frame.fit(points, 0., 600.).is_none());
        assert!(frame.fit([Vec3::new(-1e12, 0., 0.), Vec3::new(1e12, 0., 0.)], 800., 600.).is_none());
        assert!(frame.fit(std::iter::repeat_n(Vec3::ZERO, 100_001), 800., 600.).is_none());
    }
}
