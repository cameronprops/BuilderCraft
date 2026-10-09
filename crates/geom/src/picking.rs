//! Orthographic wire picking in pixel coordinates, independent of any UI toolkit.
use crate::{Vec2, Vec3, camera::OrthoFrame};

#[derive(Clone, Copy, Debug)]
pub struct WirePick {
    pub frame: OrthoFrame,
    pub center: Vec3,
    pub scale: f64,
    pub pixel: Vec2,
    pub viewport: Vec2,
    pub radius: f64,
}
impl WirePick {
    pub fn valid(self) -> bool {
        [
            self.frame.yaw,
            self.frame.pitch,
            self.center.x,
            self.center.y,
            self.center.z,
            self.scale,
            self.pixel.x,
            self.pixel.y,
            self.viewport.x,
            self.viewport.y,
            self.radius,
        ]
        .iter()
        .all(|v| v.is_finite())
            && self.scale > 0.
            && self.viewport.x > 0.
            && self.viewport.y > 0.
            && (0. ..=64.).contains(&self.radius)
            && (0. ..=self.viewport.x).contains(&self.pixel.x)
            && (0. ..=self.viewport.y).contains(&self.pixel.y)
    }
    /// Distance and camera-facing depth at the closest projected point.
    pub fn segment(self, a: Vec3, b: Vec3) -> Option<(f64, f64)> {
        if !self.valid() {
            return None;
        }
        let project = |p| {
            let q = self.frame.project(p, self.center);
            Vec2::new(self.viewport.x * 0.5 + q.x * self.scale, self.viewport.y * 0.5 - q.y * self.scale)
        };
        let (p, q) = (project(a), project(b));
        let edge = q - p;
        let norm = edge.dot(edge);
        let t = if norm > 0. { ((self.pixel - p).dot(edge) / norm).clamp(0., 1.) } else { 0. };
        let delta = self.pixel - (p + edge * t);
        let distance = delta.dot(delta).sqrt();
        let depth = (a * (1. - t) + b * t - self.center).dot(self.frame.right().cross(self.frame.up()));
        (distance.is_finite() && depth.is_finite() && distance <= self.radius).then_some((distance, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixels_depth_degenerate_and_hostile_inputs() {
        let mut pick = WirePick {
            frame: OrthoFrame { yaw: 0., pitch: 0. },
            center: Vec3::ZERO,
            scale: 10.,
            pixel: Vec2::new(200., 203.),
            viewport: Vec2::new(400., 400.),
            radius: 6.,
        };
        let (distance, depth) = pick.segment(Vec3::new(-1., -2., 0.), Vec3::new(1., -2., 0.)).unwrap();
        assert_eq!(distance, 3.);
        assert_eq!(depth, 2.);
        assert!(pick.segment(Vec3::ZERO, Vec3::ZERO).is_some());
        pick.pixel.y = 220.;
        assert!(pick.segment(Vec3::ZERO, Vec3::ZERO).is_none());
        pick.scale = f64::NAN;
        assert!(!pick.valid());
    }
}
