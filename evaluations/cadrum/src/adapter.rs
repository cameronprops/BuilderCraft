//! Draft typed boundary for a candidate exact BRep engine.
//! No method in this module installs cadrum as WorldWright's production backend.
//! Tolerance validates input; OCCT's own boolean tolerances are NOT configurable
//! through this wrapper yet and require separate acceptance tests.

use cadrum::{DVec3, Solid};
use std::{error::Error, fmt, io::Cursor};

#[derive(Clone, Copy, Debug)]
pub struct AbsoluteTolerance(f64);

#[derive(Debug)]
pub enum BrepError {
    InvalidTolerance,
    InvalidBoxBounds,
    InvalidBinaryPayload,
    Backend(String),
}

impl fmt::Display for BrepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTolerance => write!(f, "absolute modeling tolerance must be finite and positive"),
            Self::InvalidBoxBounds => write!(f, "box coordinates must be finite with extents larger than the tolerance"),
            Self::InvalidBinaryPayload => write!(f, "binary BRep input is empty or exceeds the probe budget"),
            Self::Backend(message) => write!(f, "exact BRep candidate operation failed: {message}"),
        }
    }
}

impl Error for BrepError {}

impl From<cadrum::Error> for BrepError {
    fn from(error: cadrum::Error) -> Self {
        Self::Backend(error.to_string())
    }
}

impl AbsoluteTolerance {
    pub fn new(value: f64) -> Result<Self, BrepError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(BrepError::InvalidTolerance);
        }
        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactBoolean {
    Union,
    Difference,
    Intersection,
}

#[derive(Debug)]
pub struct BrepSolid(Solid);

#[derive(Clone, Copy, Debug)]
pub struct SolidStatistics {
    pub volume: f64,
    pub faces: usize,
    pub edges: usize,
}

impl BrepSolid {
    pub fn statistics(&self) -> SolidStatistics {
        SolidStatistics {
            volume: self.0.volume(),
            faces: self.0.iter_face().count(),
            edges: self.0.iter_edge().count(),
        }
    }
}

/// This is an isolated backend probe. OCCT may use unsafe C++ code internally:
/// do not assume this wrapper can recover from native process termination.
pub struct CadrumBrepCandidate {
    absolute: AbsoluteTolerance,
}

impl CadrumBrepCandidate {
    pub const MAX_BREP_BYTES: usize = 128 * 1024 * 1024;

    pub fn new(absolute: AbsoluteTolerance) -> Self {
        Self { absolute }
    }

    pub fn box_from_corners(&self, lower: DVec3, upper: DVec3) -> Result<BrepSolid, BrepError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(BrepError::InvalidBoxBounds);
        }
        if lower
            .to_array()
            .into_iter()
            .zip(upper.to_array())
            .any(|(low, high)| !((high - low).is_finite() && high - low > self.absolute.value()))
        {
            return Err(BrepError::InvalidBoxBounds);
        }
        Ok(BrepSolid(Solid::cube(lower, upper)))
    }

    /// Supports empty, single or split multi-body results without inventing
    /// solid geometry for tangent or fully subtractive cases.
    pub fn boolean(
        &self,
        operation: ExactBoolean,
        first: &BrepSolid,
        second: &BrepSolid,
    ) -> Result<Vec<BrepSolid>, BrepError> {
        let pieces = match operation {
            ExactBoolean::Union => (&first.0 + &second.0).build_vec()?,
            ExactBoolean::Difference => (&first.0 - &second.0).build_vec()?,
            ExactBoolean::Intersection => (&first.0 * &second.0).build_vec()?,
        };
        Ok(pieces.into_iter().map(BrepSolid).collect())
    }

    pub fn write_native_brep(&self, solids: &[BrepSolid]) -> Result<Vec<u8>, BrepError> {
        let mut bytes = Vec::new();
        Solid::write_brep(solids.iter().map(|shape| &shape.0), &mut bytes)?;
        if bytes.is_empty() || bytes.len() > Self::MAX_BREP_BYTES {
            return Err(BrepError::InvalidBinaryPayload);
        }
        Ok(bytes)
    }

    pub fn read_native_brep(&self, bytes: &[u8]) -> Result<Vec<BrepSolid>, BrepError> {
        if bytes.is_empty() || bytes.len() > Self::MAX_BREP_BYTES {
            return Err(BrepError::InvalidBinaryPayload);
        }
        let solids = Solid::read_brep(&mut Cursor::new(bytes))?;
        Ok(solids.into_iter().map(BrepSolid).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> CadrumBrepCandidate {
        CadrumBrepCandidate::new(AbsoluteTolerance::new(1e-7).unwrap())
    }

    #[test]
    fn rejects_invalid_tolerances_before_native_ffi() {
        for value in [0.0, -0.0001, f64::INFINITY, f64::NAN] {
            assert!(AbsoluteTolerance::new(value).is_err());
        }
        assert!(AbsoluteTolerance::new(1e-7).is_ok());
    }

    #[test]
    fn rejects_zero_inverted_nonfinite_and_below_tolerance_boxes() {
        let kernel = candidate();
        for (lower, upper) in [
            (DVec3::ZERO, DVec3::ZERO),
            (DVec3::ONE, DVec3::ZERO),
            (DVec3::ZERO, DVec3::new(1e-9, 2.0, 2.0)),
            (DVec3::ZERO, DVec3::new(f64::NAN, 2.0, 2.0)),
            (DVec3::ZERO, DVec3::new(f64::INFINITY, 2.0, 2.0)),
        ] {
            assert!(matches!(
                kernel.box_from_corners(lower, upper),
                Err(BrepError::InvalidBoxBounds)
            ));
        }
    }

    #[test]
    fn typed_boolean_and_native_roundtrip_do_not_lose_volume() {
        let kernel = candidate();
        let a = kernel.box_from_corners(DVec3::ZERO, DVec3::splat(2.0)).unwrap();
        let b = kernel.box_from_corners(DVec3::ONE, DVec3::splat(3.0)).unwrap();
        let result = kernel.boolean(ExactBoolean::Intersection, &a, &b).unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0].statistics().volume - 1.0).abs() < 1e-8);
        let bytes = kernel.write_native_brep(&result).unwrap();
        let restored = kernel.read_native_brep(&bytes).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].statistics().faces, 6);
        assert!((restored[0].statistics().volume - 1.0).abs() < 1e-8);
    }

    #[test]
    fn refuses_empty_binary_data_before_native_parser() {
        assert!(matches!(
            candidate().read_native_brep(&[]),
            Err(BrepError::InvalidBinaryPayload)
        ));
    }
}
