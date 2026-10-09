//! Native orthographic 3D point input and construction-plane math.
//! A single CPU implementation is reused by CAD commands and scripting.
use crate::{Vec2, Vec3, camera::OrthoFrame};

const MAX_COORD: f64 = 1e12;

/// A rigid, right-handed construction plane in drawing/world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstructionPlane {
    pub origin: Vec3,
    pub x_axis: Vec3,
    pub y_axis: Vec3,
}

impl ConstructionPlane {
    pub fn world_xy() -> Self {
        Self { origin: Vec3::ZERO, x_axis: Vec3::new(1., 0., 0.), y_axis: Vec3::new(0., 1., 0.) }
    }

    /// Require perpendicular, nondegenerate axes; normalize without silently
    /// reorienting a skewed plane from an invalid file or API request.
    pub fn from_axes(origin: Vec3, x_axis: Vec3, y_axis: Vec3) -> Option<Self> {
        if !bounded(origin) || !bounded(x_axis) || !bounded(y_axis) {
            return None;
        }
        let (xl, yl) = (x_axis.len(), y_axis.len());
        if xl < 1e-9 || yl < 1e-9 || !xl.is_finite() || !yl.is_finite() {
            return None;
        }
        let (x, y) = (x_axis * (1. / xl), y_axis * (1. / yl));
        if x.dot(y).abs() > 1e-8 {
            return None;
        }
        Some(Self { origin, x_axis: x, y_axis: y })
    }

    pub fn normal(self) -> Vec3 {
        self.x_axis.cross(self.y_axis)
    }

    pub fn coordinates(self, point: Vec3) -> Option<Vec2> {
        if !bounded(point) {
            return None;
        }
        let d = point - self.origin;
        let uv = Vec2::new(d.dot(self.x_axis), d.dot(self.y_axis));
        uv.is_finite().then_some(uv)
    }

    pub fn point(self, uv: Vec2) -> Option<Vec3> {
        if !uv.is_finite() {
            return None;
        }
        let point = self.origin + self.x_axis * uv.x + self.y_axis * uv.y;
        bounded(point).then_some(point)
    }

    /// Nearest grid intersection, anchored at this plane's origin.
    pub fn grid_point(self, point: Vec3, spacing: f64) -> Option<Vec3> {
        if !spacing.is_finite() || !(1e-9..=1e9).contains(&spacing) {
            return None;
        }
        let uv = self.coordinates(point)?;
        self.point(Vec2::new((uv.x / spacing).round() * spacing, (uv.y / spacing).round() * spacing))
    }
}

/// An orthographic screen point; screen origin is the top-left pixel.
#[derive(Clone, Copy, Debug)]
pub struct ScreenRay {
    pub frame: OrthoFrame,
    pub center: Vec3,
    pub scale: f64,
    pub pixel: Vec2,
    pub viewport: Vec2,
}

fn bounded(v: Vec3) -> bool {
    v.is_finite() && v.x.abs().max(v.y.abs()).max(v.z.abs()) <= MAX_COORD
}

impl ScreenRay {
    pub fn valid(self) -> bool {
        self.frame.yaw.is_finite()
            && self.frame.pitch.is_finite()
            && bounded(self.center)
            && self.scale.is_finite()
            && (1e-9..=1e9).contains(&self.scale)
            && self.pixel.is_finite()
            && self.viewport.is_finite()
            && self.viewport.x > 0.
            && self.viewport.y > 0.
            && self.viewport.x <= 1e6
            && self.viewport.y <= 1e6
            && (0. ..=self.viewport.x).contains(&self.pixel.x)
            && (0. ..=self.viewport.y).contains(&self.pixel.y)
    }

    pub fn screen_origin(self) -> Option<Vec3> {
        if !self.valid() {
            return None;
        }
        let offset = Vec2::new((self.pixel.x - self.viewport.x * 0.5) / self.scale, (self.viewport.y * 0.5 - self.pixel.y) / self.scale);
        let point = self.center + self.frame.right() * offset.x + self.frame.up() * offset.y;
        bounded(point).then_some(point)
    }

    pub fn on_plane(self, plane: ConstructionPlane) -> Option<Vec3> {
        let start = self.screen_origin()?;
        let direction = self.frame.right().cross(self.frame.up());
        let normal = plane.normal();
        let denominator = direction.dot(normal);
        // Parallel or nearly parallel rays cannot define a unique CPlane point.
        if !denominator.is_finite() || denominator.abs() <= 1e-10 {
            return None;
        }
        let t = (plane.origin - start).dot(normal) / denominator;
        let hit = start + direction * t;
        bounded(hit).then_some(hit)
    }

    /// Return pixel separation and camera-facing depth for a source point.
    pub fn hit(self, point: Vec3, radius: f64) -> Option<(f64, f64)> {
        if !self.valid() || !bounded(point) || !radius.is_finite() || !(0. ..=64.).contains(&radius) {
            return None;
        }
        let projected = self.frame.project(point, self.center);
        let pixel = Vec2::new(self.viewport.x * 0.5 + projected.x * self.scale, self.viewport.y * 0.5 - projected.y * self.scale);
        let distance = (pixel - self.pixel).len();
        let depth = (point - self.center).dot(self.frame.right().cross(self.frame.up()));
        (distance.is_finite() && depth.is_finite() && distance <= radius).then_some((distance, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(pixel: Vec2) -> ScreenRay {
        ScreenRay {
            frame: OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 },
            center: Vec3::ZERO,
            scale: 10.,
            pixel,
            viewport: Vec2::new(400., 400.),
        }
    }

    #[test]
    fn screen_to_plane_and_rotated_plane_axes() {
        let plane = ConstructionPlane::world_xy();
        let point = top(Vec2::new(223., 172.)).on_plane(plane).unwrap();
        assert!((point.x - 2.3).abs() < 1e-10);
        assert!((point.y - 2.8).abs() < 1e-10);
        assert!(point.z.abs() < 1e-10);
        assert_eq!(plane.grid_point(point, 1.), Some(Vec3::new(2., 3., 0.)));
        let rotated = ConstructionPlane::from_axes(Vec3::new(10., 0., 1.), Vec3::new(0., 2., 0.), Vec3::new(-3., 0., 0.)).unwrap();
        assert_eq!(rotated.normal(), Vec3::Z);
        assert_eq!(rotated.point(Vec2::new(2., 3.)), Some(Vec3::new(7., 2., 1.)));
        assert_eq!(rotated.coordinates(Vec3::new(7., 2., 1.)), Some(Vec2::new(2., 3.)));
    }

    #[test]
    fn parallel_and_hostile_inputs_never_make_a_point() {
        let front = ScreenRay { frame: OrthoFrame { yaw: 0., pitch: 0. }, ..top(Vec2::new(200., 200.)) };
        assert!(front.on_plane(ConstructionPlane::world_xy()).is_none());
        let q = top(Vec2::new(200., 200.));
        assert_eq!(q.hit(Vec3::new(0., 0., 3.), 6.).map(|x| x.0), Some(0.));
        assert!(q.hit(Vec3::new(2., 0., 3.), 6.).is_none());
        assert!(q.hit(Vec3::ZERO, f64::NAN).is_none());
        assert!(ConstructionPlane::from_axes(Vec3::ZERO, Vec3::Z, Vec3::Z).is_none());
        assert!(ConstructionPlane::from_axes(Vec3::ZERO, Vec3::ZERO, Vec3::Z).is_none());
        assert!(ConstructionPlane::world_xy().grid_point(Vec3::ZERO, 0.).is_none());
        assert!(ScreenRay { scale: 0., ..q }.on_plane(ConstructionPlane::world_xy()).is_none());
        assert!(ScreenRay { pixel: Vec2::new(-1., 1.), ..q }.screen_origin().is_none());
        assert!(ScreenRay { center: Vec3::new(f64::NAN, 0., 0.), ..q }.screen_origin().is_none());
    }
}
