//! Bounded read-only LAS point cloud interchange.
//!
//! LAS records carry more than XYZ: intensity, classification, returns,
//! colors, GPS time, CRS and scan provenance. This is an intentionally
//! lossy *geometry preview*, not an authoritative conversion or round-trip.
//! Keep the source LAS for metadata-preserving workflows.

use crate::{IoError, Result};
use cadcraft_geom::Vec3;
use std::io::{BufReader, Cursor};

/// Bound memory allocated by the LAS reader and exported point buffer.
pub const MAX_LAS_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_LAS_POINTS: usize = 250_000;

#[derive(Clone, Debug, PartialEq)]
pub struct LasPointPreview {
    /// Coordinates as decoded from the LAS header's scale and offset.
    /// No unit or CRS re-projection occurs.
    pub points: Vec<Vec3>,
    pub point_count: u64,
    pub has_crs_metadata: bool,
    pub diagnostics: Vec<String>,
}

/// Read a bounded LAS point cloud in source coordinates.
///
/// This deliberately does not support LAZ, because that needs an additional
/// decompressor with its own memory and licensing review. It also does not
/// pretend the returned XYZ array preserves the full LAS record.
pub fn read_las_preview(bytes: &[u8]) -> Result<LasPointPreview> {
    if bytes.is_empty() || bytes.len() > MAX_LAS_BYTES {
        return Err(IoError::Format("LAS input empty or exceeds 32 MiB".into()));
    }
    if !bytes.starts_with(b"LASF") {
        return Err(IoError::Format("LAS file signature missing".into()));
    }

    // The upstream reader owns its input; cap the copied bytes before
    // entering its parser, so caller-lifetime and streaming contracts are
    // predictable. A future file-backed host can avoid the copy.
    let cursor = Cursor::new(bytes.to_vec());
    let mut reader = las::Reader::new(BufReader::new(cursor)).map_err(|e| IoError::Format(format!("LAS header: {e}")))?;
    let header = reader.header();
    let count = header.number_of_points();
    if count == 0 || count > MAX_LAS_POINTS as u64 {
        return Err(IoError::Format("LAS point count empty or exceeds 250000".into()));
    }
    if header.point_format().is_compressed {
        return Err(IoError::Unsupported("LAZ compressed point clouds are not enabled".into()));
    }
    let has_crs_metadata = header.has_crs_vlrs();

    let slab = reader.read_points(count).map_err(|e| IoError::Format(format!("LAS point data: {e}")))?;
    if slab.len() != count as usize {
        return Err(IoError::Format("LAS truncated point records".into()));
    }

    let mut points = Vec::new();
    points.try_reserve_exact(slab.len()).map_err(|_| IoError::Format("LAS point allocation rejected".into()))?;
    for result in slab.points() {
        let point = result.map_err(|e| IoError::Format(format!("LAS point decode: {e}")))?;
        let pos = Vec3::new(point.x, point.y, point.z);
        if !pos.is_finite() || [pos.x, pos.y, pos.z].iter().any(|n| n.abs() > 1e12) {
            return Err(IoError::Format("LAS nonfinite or out-of-range coordinate".into()));
        }
        points.push(pos);
    }

    let mut diagnostics = vec![
        "Preview retains XYZ only; classification, intensity, color, GPS time, return numbers and other LAS attributes are not transferred".to_string(),
        "Source coordinate reference system and linear units have not been interpreted or converted".to_string(),
    ];
    if has_crs_metadata {
        diagnostics.push("LAS CRS metadata exists but is not applied; retain original file for georeferencing".to_string());
    }

    Ok(LasPointPreview { points, point_count: count, has_crs_metadata, diagnostics })
}
