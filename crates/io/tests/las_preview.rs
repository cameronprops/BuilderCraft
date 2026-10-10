use cadcraft_io::las_preview::{MAX_LAS_BYTES, read_las_preview};
use std::io::Cursor;

fn fixture() -> Vec<u8> {
    let mut writer = las::Writer::new(Cursor::new(Vec::new()), las::Header::default()).unwrap();
    for (x, y, z) in [(1., 2., 3.), (4., 5., 6.)] {
        writer.write_point(las::Point { x, y, z, ..Default::default() }).unwrap();
    }
    writer.into_inner().unwrap().into_inner()
}

#[test]
fn reads_xyz_point_count_without_inventing_crs_or_attributes() {
    let data = fixture();
    let imported = read_las_preview(&data).unwrap();
    assert_eq!(imported.point_count, 2);
    assert_eq!(imported.points.len(), 2);
    assert_eq!((imported.points[0].x, imported.points[0].y, imported.points[0].z), (1., 2., 3.));
    assert_eq!((imported.points[1].x, imported.points[1].y, imported.points[1].z), (4., 5., 6.));
    assert!(!imported.has_crs_metadata);
    assert!(imported.diagnostics.iter().any(|d| d.contains("not been interpreted")));
    assert!(imported.diagnostics.iter().any(|d| d.contains("classification")));
}

#[test]
fn malformed_and_oversized_inputs_are_rejected() {
    assert!(read_las_preview(b"").is_err());
    assert!(read_las_preview(b"not an LAS file").is_err());
    assert!(read_las_preview(b"LASF").is_err());
    assert!(read_las_preview(&vec![0u8; MAX_LAS_BYTES + 1]).is_err());
}

#[test]
fn malformed_point_payload_returns_error() {
    let mut bytes = fixture();
    bytes.truncate(bytes.len().saturating_sub(6));
    assert!(read_las_preview(&bytes).is_err());
}

#[test]
fn zero_point_las_is_rejected_as_geometry_preview() {
    let writer = las::Writer::new(Cursor::new(Vec::new()), las::Header::default()).unwrap();
    let empty = writer.into_inner().unwrap().into_inner();
    assert!(read_las_preview(&empty).is_err());
}
