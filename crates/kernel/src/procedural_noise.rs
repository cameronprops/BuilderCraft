//! Deterministic procedural scalar-field sampling for terrain, rockwork
//! and OrbWeaver. Uses the MIT FastNoise Lite implementation once, in a
//! shared headless kernel operation; no terrain-specific duplicate math.

use crate::{KernelError, Result};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseAlgorithm {
    OpenSimplex2,
    OpenSimplex2S,
    Cellular,
    Perlin,
    Value,
    ValueCubic,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseConfig {
    pub seed: i32,
    /// Spatial frequency in inverse input units. Unit conversion is the caller's responsibility.
    pub frequency: f64,
    pub algorithm: NoiseAlgorithm,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self { seed: 1337, frequency: 0.01, algorithm: NoiseAlgorithm::OpenSimplex2 }
    }
}

/// Evaluate a scalar field in nominal [-1,1], with bounded finite input.
/// The f64 coordinate interface retains input precision, but the upstream
/// noise *output* is f32; do not mistake it for exact CAD surface geometry.
pub fn sample_noise3d(config: NoiseConfig, point: Vec3) -> Result<f64> {
    if !config.frequency.is_finite()
        || config.frequency <= 0.0
        || config.frequency > 100.0
        || !point.is_finite()
        || [point.x, point.y, point.z].iter().any(|v| v.abs() > 1e6)
    {
        return Err(KernelError::Invalid("invalid procedural field sample"));
    }

    let algorithm = match config.algorithm {
        NoiseAlgorithm::OpenSimplex2 => fastnoise_lite::NoiseType::OpenSimplex2,
        NoiseAlgorithm::OpenSimplex2S => fastnoise_lite::NoiseType::OpenSimplex2S,
        NoiseAlgorithm::Cellular => fastnoise_lite::NoiseType::Cellular,
        NoiseAlgorithm::Perlin => fastnoise_lite::NoiseType::Perlin,
        NoiseAlgorithm::Value => fastnoise_lite::NoiseType::Value,
        NoiseAlgorithm::ValueCubic => fastnoise_lite::NoiseType::ValueCubic,
    };
    let mut generator = fastnoise_lite::FastNoiseLite::with_seed(config.seed);
    generator.set_frequency(Some(config.frequency as f32));
    generator.set_noise_type(Some(algorithm));
    let value = f64::from(generator.get_noise_3d(point.x, point.y, point.z));
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KernelError::Invalid("nonfinite procedural field output"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_and_input_are_repeatable() {
        let p = Vec3::new(14.3, -50.7, 1.2);
        let config = NoiseConfig::default();
        assert_eq!(sample_noise3d(config, p), sample_noise3d(config, p));
    }

    #[test]
    fn changed_seed_or_algorithm_changes_sample() {
        let p = Vec3::new(19.0, 43.0, 7.0);
        let a = NoiseConfig::default();
        let mut b = a;
        b.seed += 1;
        let mut c = a;
        c.algorithm = NoiseAlgorithm::Perlin;
        assert_ne!(sample_noise3d(a, p), sample_noise3d(b, p));
        assert_ne!(sample_noise3d(a, p), sample_noise3d(c, p));
    }

    #[test]
    fn every_supported_algorithm_is_finite_and_bounded() {
        let point = Vec3::new(5.3, 8.9, -1.2);
        for algorithm in [
            NoiseAlgorithm::OpenSimplex2, NoiseAlgorithm::OpenSimplex2S,
            NoiseAlgorithm::Cellular, NoiseAlgorithm::Perlin,
            NoiseAlgorithm::Value, NoiseAlgorithm::ValueCubic,
        ] {
            let value = sample_noise3d(NoiseConfig { algorithm, ..Default::default() }, point).unwrap();
            assert!(value.is_finite() && (-1.1..=1.1).contains(&value));
        }
    }

    #[test]
    fn rejects_hostile_coordinates_and_frequencies() {
        let p = Vec3::ZERO;
        assert!(sample_noise3d(NoiseConfig { frequency: -1.0, ..Default::default() }, p).is_err());
        assert!(sample_noise3d(NoiseConfig { frequency: f64::NAN, ..Default::default() }, p).is_err());
        assert!(sample_noise3d(NoiseConfig::default(), Vec3::new(f64::INFINITY, 0., 0.)).is_err());
        assert!(sample_noise3d(NoiseConfig::default(), Vec3::new(1e7, 0., 0.)).is_err());
    }
}
