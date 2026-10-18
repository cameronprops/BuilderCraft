//! Shared geometry acceptance tests for theme-park path placement.
use buildercraft_kernel::{
    ToolRequest, ToolValue, execute_shared_tool, operation_by_id,
    polyline_offset_xy, polyline_point_at_station, polyline_tangent_at_station,
};
use cadcraft_geom::Vec3;
use std::collections::BTreeMap;

fn route() -> Vec<Vec3> {
    vec![
        Vec3::new(0., 0., 0.),
        Vec3::new(10., 0., 0.),
        Vec3::new(10., 10., 2.),
    ]
}
fn request(op: &str, extra: &[(&str, ToolValue)]) -> ToolRequest {
    let mut inputs = BTreeMap::from([("points".to_string(), ToolValue::Polyline(route()))]);
    for (key, value) in extra {
        inputs.insert((*key).to_owned(), value.clone());
    }
    ToolRequest { operation: op.to_owned(), inputs }
}
#[test]
fn registry_and_typed_shared_dispatch_expose_real_path_operations() {
    for operation in [
        "kernel.polyline.point_at_station",
        "kernel.polyline.tangent_at_station",
        "kernel.polyline.offset_xy",
    ] {
        assert!(operation_by_id(operation).is_some());
    }
    let p = request("kernel.polyline.point_at_station", &[("station", ToolValue::Number(10.))]);
    assert_eq!(execute_shared_tool(&p), Ok(ToolValue::Point(Vec3::new(10., 0., 0.))));
    let t = request("kernel.polyline.tangent_at_station", &[("station", ToolValue::Number(10.))]);
    assert_eq!(execute_shared_tool(&t), Ok(ToolValue::Vector(Vec3::new(1., 0., 0.))));
    let o = request("kernel.polyline.offset_xy", &[
        ("distance", ToolValue::Number(1.)),
        ("miter_limit", ToolValue::Number(4.)),
    ]);
    let ToolValue::Polyline(out) = execute_shared_tool(&o).unwrap() else { panic!("expected polyline"); };
    assert_eq!(out.len(), 3);
    assert_eq!(out[1], Vec3::new(9., 1., 0.));
}
#[test]
fn curve_eval_and_offset_have_explicit_invalid_input_handling() {
    assert!(polyline_point_at_station(&route(), 100.).is_err());
    assert!(polyline_tangent_at_station(&route(), f64::NAN).is_err());
    assert!(polyline_offset_xy(&route(), f64::NAN, 4.).is_err());
    assert!(polyline_offset_xy(&route(), 2., f64::NAN).is_err());
    let malformed = request("kernel.polyline.point_at_station", &[
        ("station", ToolValue::Number(-1.)),
    ]);
    assert!(execute_shared_tool(&malformed).is_err());
}
