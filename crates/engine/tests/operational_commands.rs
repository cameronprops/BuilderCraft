//! Positive command acceptance cases. Registration or a non-panic error is not a pass.
//!
//! The ignored release gate deliberately fails until every registered command
//! has an independently asserted, realistic success path.

use cadcraft_doc::EntityKind;
use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::json;
use std::collections::HashSet;

type Case = (&'static str, fn());

const CASES: &[Case] = &[
    ("line", line_creates_geometry),
    ("circle", circle_creates_geometry),
    ("rectang", rectangle_creates_geometry),
    ("layer.new", layer_new_assigns_geometry),
    ("move", move_changes_coordinates),
    ("copy", copy_adds_objects),
    ("rotate", rotate_changes_coordinates),
    ("scale", scale_changes_coordinates),
    ("mirror", mirror_preserves_source),
    ("undo", undo_restores_document),
    ("redo", redo_restores_document),
    ("zoom.extents", zoom_fits_drawing),
    ("worldwright.point.distance", native_distance_is_correct),
    ("worldwright.point.interpolate", native_interpolation_is_correct),
    ("worldwright.tool.run", generic_dispatch_matches_direct_command),
    ("mesh3d.create", mesh_creation_is_persistent),
    ("mesh3d.list", mesh_listing_finds_created_object),
    ("pline", polyline_maintains_closed_shape),
    ("arc", arc_creates_expected_radius),
    ("polygon", polygon_creates_six_sides),
    ("ellipse", ellipse_preserves_axes),
    ("point", single_point_uses_requested_coordinates),
    ("point.multiple", multiple_points_are_independent),
    ("ray", ray_keeps_normalized_direction),
    ("dist", distance_query_computes_three_four_five),
    ("area", area_query_finds_rectangle_measurements),
    ("id", id_query_updates_last_point),
    ("layer", layer_list_includes_new_layer),
    ("layer.set", layer_set_changes_lock_state),
    ("layer.current", layer_current_sets_active_layer),
    ("properties", properties_query_reports_selected_line),
    ("properties.set", properties_set_changes_entity_color),
    ("drawing.inspect", drawing_inspection_counts_entities),
    ("entities", entity_filter_excludes_other_shapes),
    ("view.set", view_set_uses_exact_coordinates),
    ("view.get", view_get_reflects_new_view),
    ("pan", pan_changes_camera_not_geometry),
    ("zoom.in", zoom_in_halves_view_height),
    ("zoom.out", zoom_out_doubles_view_height),
    ("regen", regen_invalidates_view_without_geometry_change),
    ("redraw", redraw_does_not_edit_entities),
    ("time", time_query_reports_drawing_revision),
    ("status", status_query_reports_current_layer),
    ("count", count_query_on_empty_blocks),
    ("donut", donut_creates_width_controlled_closed_outline),
    ("circle.cd", circle_cd_uses_half_the_diameter),
    ("circle.2p", circle_two_points_are_diameter_ends),
    ("circle.3p", circle_three_points_finds_circumcircle),
    ("arc.sce", arc_start_center_end_keeps_radius),
    ("ellipse.axis", ellipse_axis_uses_half_minor_axis),
    ("ellipse.arc", elliptical_arc_keeps_ratio),
    ("3dpoly", three_d_polyline_preserves_elevation),
    ("helix", helix_has_expected_height_and_turns),
    ("layer.delete", unused_layer_can_be_deleted),
    ("laymcur", object_layer_becomes_current),
    ("laymch", layer_match_moves_entity),
    ("laycur", layer_current_moves_entities),
    ("layiso", isolate_hides_other_layers),
    ("layuniso", unisolate_restores_hidden_layers),
    ("layoff", layer_off_hides_object_layer),
    ("layon", all_layers_on_restores_visibility),
    ("laylck", lock_object_layer),
    ("layulk", unlock_object_layer),
    ("layfrz", freeze_object_layer),
    ("laythw", thaw_all_layers),
    ("layerp", previous_layer_restores_prior_layer_flags),
    ("layerstate.save", layer_state_save_records_snapshot),
    ("layerstate.list", layer_state_list_reports_saved_name),
    ("layerstate.restore", layer_state_restore_recovers_properties),
    ("layerstate.rename", layer_state_rename_updates_identity),
    ("layerstate.delete", layer_state_delete_removes_saved_state),
    ("zoom", zoom_center_mode_sets_viewport),
    ("zoom.window", zoom_window_uses_requested_bounds),
    ("zoom.previous", zoom_previous_restores_last_camera),
    ("zoom.object", zoom_object_centers_on_selected_geometry),
    ("zoom.all", zoom_all_includes_drawing_limits),
    ("pan.left", pan_left_moves_camera_left),
    ("pan.right", pan_right_moves_camera_right),
    ("pan.up", pan_up_moves_camera_up),
    ("pan.down", pan_down_moves_camera_down),
    ("erase", erase_deletes_selected_line_and_undo_recovers),
    ("arrayrect", rect_array_creates_correct_grid),
    ("arraypolar", polar_array_rotates_copies),
    ("draworder.front", draworder_front_moves_entity_to_end),
    ("draworder.back", draworder_back_moves_entity_to_start),
    ("break", break_removes_middle_segment),
    ("breakatpoint", break_atpoint_splits_line_without_gap),
    ("join", join_chains_two_segments),
    ("explode", explode_splits_polyline_into_lines),
    ("overkill", overkill_removes_duplicate_geometry),
    ("offset", offset_circle_changes_radius),
    ("trim", trim_keeps_expected_segment),
    ("extend", extend_meets_nearest_cutting_boundary),
    ("divide", divide_stations_match_geometry),
    ("revcloud", revcloud_replaces_source_with_closed_boundary),
    ("pedit", pedit_joins_touching_geometry),
    ("text", text_preserves_content_and_position),
    ("mtext", mtext_preserves_paragraphs_and_dimensions),
    ("textedit", textedit_changes_text_without_replacing_handle),
    ("xline", xline_keeps_unit_direction),
    ("mline", mline_builds_two_parallel_rails),
    ("centermark", centermark_crosses_circle_center),
    ("centerline", centerline_bisects_parallel_lines),
    ("worldwright.polyline.point_at_station", native_polyline_station_returns_expected_point),
    ("worldwright.polyline.tangent_at_station", native_polyline_station_returns_unit_tangent),
    ("worldwright.polyline.offset_xy", native_polyline_offset_returns_parallel_rail),
    ("worldwright.curve.join", native_curve_join_preserves_common_end),
    ("worldwright.curve.trim", native_curve_trim_keeps_requested_interval),
    ("worldwright.curve.extend", native_curve_extend_follows_tangent),
    ("worldwright.vector.length", native_vector_length_is_13),
    ("worldwright.vector.normalize", native_vector_normalize_is_unit),
    ("worldwright.vector.dot", native_vector_dot_is_scalar),
    ("worldwright.vector.cross", native_vector_cross_is_right_handed),
    ("worldwright.polyline.length", native_polyline_length_is_7),
    ("worldwright.polyline.divide_count", native_polyline_divides_by_count),
    ("worldwright.polyline.divide_distance", native_polyline_divides_by_spacing),
    ("worldwright.curve.split", native_curve_split_yields_two_actual_fragments),
    ("measure", measure_places_all_four_line_stations),
    ("list", list_reports_line_length_and_handle),
    ("spline.cv", spline_cv_degree_is_preserved),
    ("spline", spline_control_degree_is_preserved),
    ("fillet", fillet_creates_radius_two_arc),
    ("chamfer", chamfer_creates_diagonal_bridge),
    ("arraypath", path_array_places_copies_along_rail),
    ("reverse", reverse_swaps_line_endpoints),
    ("flatten", flatten_projects_3d_polyline),
    ("align", align_translates_with_two_reference_points),
    ("lengthen", lengthen_changes_line_length),
];

fn first_line(s: &Session) -> (f64, f64, f64, f64) {
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Line(line) => (line.a.x, line.a.y, line.b.x, line.b.y),
        other => panic!("expected Line, got {other:?}"),
    }
}

fn line_handle(s: &mut Session) -> String {
    let created = s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    created["handles"][0].as_str().unwrap().to_owned()
}

fn line_creates_geometry() {
    let mut s = Session::new();
    line_handle(&mut s);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    assert_eq!(first_line(&s), (0.0, 0.0, 1.0, 0.0));
}

fn circle_creates_geometry() {
    let mut s = Session::new();
    let r = s.execute("circle", &json!({"center": [2, 3], "radius": 5})).unwrap();
    assert!(r["handle"].is_string());
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Circle(c) => {
            assert_eq!(c.center.x, 2.0);
            assert_eq!(c.center.y, 3.0);
            assert_eq!(c.radius, 5.0);
        }
        other => panic!("expected Circle, got {other:?}"),
    }
}

fn rectangle_creates_geometry() {
    let mut s = Session::new();
    s.execute("rectang", &json!({"p1": [0, 0], "p2": [4, 3]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => assert_eq!(poly.vertices.len(), 4),
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn layer_new_assigns_geometry() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Fixtures", "current": true})).unwrap();
    let h = line_handle(&mut s);
    let handle = cadcraft_doc::Handle::parse_hex(&h).unwrap();
    assert_eq!(s.doc().unwrap().entity(handle).unwrap().common.layer, "Fixtures");
}

fn move_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("move", &json!({"handles": [h], "delta": [3, 4]})).unwrap();
    assert_eq!(first_line(&s), (3.0, 4.0, 4.0, 4.0));
}

fn copy_adds_objects() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("copy", &json!({"handles": [h], "delta": [0, 2], "count": 2})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

fn rotate_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("rotate", &json!({"handles": [h], "base": [0, 0], "angle": 90})).unwrap();
    let (ax, ay, bx, by) = first_line(&s);
    assert!(ax.abs() < 1e-9 && ay.abs() < 1e-9 && bx.abs() < 1e-9 && (by - 1.0).abs() < 1e-9);
}

fn scale_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("scale", &json!({"handles": [h], "base": [0, 0], "factor": 3})).unwrap();
    assert_eq!(first_line(&s), (0.0, 0.0, 3.0, 0.0));
}

fn mirror_preserves_source() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("mirror", &json!({"handles": [h], "p1": [0, 0], "p2": [0, 1]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    assert_eq!(first_line(&s), (0.0, 0.0, 1.0, 0.0));
}

fn undo_restores_document() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn redo_restores_document() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("undo", &json!({})).unwrap();
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
}

fn zoom_fits_drawing() {
    let mut s = Session::new();
    s.viewport_px = (1000.0, 500.0);
    s.execute("line", &json!({"points": [[0, 0], [100, 10]]})).unwrap();
    s.execute("zoom.extents", &json!({})).unwrap();
    let view = s.state().unwrap().view();
    assert!(view.center.x.is_finite() && view.height > 0.0);
    assert!(view.height > 10.0);
}

fn point_inputs() -> serde_json::Value {
    json!({
        "a": {"kind": "point", "value": {"x": 0.0, "y": 0.0, "z": 0.0}},
        "b": {"kind": "point", "value": {"x": 3.0, "y": 4.0, "z": 0.0}}
    })
}

fn native_distance_is_correct() {
    let mut s = Session::new();
    let revision = s.state().unwrap().revision;
    let r = s.execute("worldwright.point.distance", &json!({"inputs": point_inputs()})).unwrap();
    assert_eq!(r["output"]["kind"], "number");
    assert_eq!(r["output"]["value"], 5.0);
    assert_eq!(s.state().unwrap().revision, revision);
}

fn native_interpolation_is_correct() {
    let mut s = Session::new();
    let mut inputs = point_inputs();
    inputs["t"] = json!({"kind": "number", "value": 0.5});
    let r = s.execute("worldwright.point.interpolate", &json!({"inputs": inputs})).unwrap();
    assert_eq!(r["output"]["value"]["x"], 1.5);
    assert_eq!(r["output"]["value"]["y"], 2.0);
}

fn generic_dispatch_matches_direct_command() {
    let mut s = Session::new();
    let inputs = point_inputs();
    let direct = s.execute("worldwright.point.distance", &json!({"inputs": inputs.clone()})).unwrap();
    let generic = s.execute("worldwright.tool.run", &json!({"operation": "kernel.point.distance", "inputs": inputs})).unwrap();
    assert_eq!(direct, generic);
}

fn quad_mesh() -> serde_json::Value {
    json!({
        "vertices": [
            {"x": 0.0, "y": 0.0, "z": 0.0},
            {"x": 2.0, "y": 0.0, "z": 0.0},
            {"x": 2.0, "y": 2.0, "z": 0.0},
            {"x": 0.0, "y": 2.0, "z": 0.0}
        ],
        "faces": [{"quad": [0, 1, 2, 3]}]
    })
}

fn mesh_creation_is_persistent() {
    let mut s = Session::new();
    let r = s.execute("mesh3d.create", &json!({"name": "Tile", "mesh": quad_mesh()})).unwrap();
    assert!(r["id"].as_u64().is_some());
    assert_eq!(s.doc().unwrap().mesh3d.len(), 1);
    let bytes = cadcraft_io::write(s.doc().unwrap(), "operational.dftba").unwrap();
    let reopened = cadcraft_io::read(&bytes, "operational.dftba").unwrap();
    assert_eq!(reopened.mesh3d.len(), 1);
    assert_eq!(reopened.mesh3d[0].name, "Tile");
}

fn mesh_listing_finds_created_object() {
    let mut s = Session::new();
    let r = s.execute("mesh3d.create", &json!({"name": "Tile", "mesh": quad_mesh()})).unwrap();
    let listed = s.execute("mesh3d.list", &json!({})).unwrap();
    assert_eq!(listed["objects"][0]["id"], r["id"]);
}

fn polyline_maintains_closed_shape() {
    let mut s = Session::new();
    s.execute("pline", &json!({"vertices": [[0, 0], [4, 0], [4, 3]], "closed": true})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 3);
            assert!(poly.closed);
        }
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn arc_creates_expected_radius() {
    let mut s = Session::new();
    s.execute("arc", &json!({"center": [0, 0], "radius": 7, "start": 0, "end": 90})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Arc(arc) => assert_eq!(arc.radius, 7.0),
        other => panic!("expected Arc, got {other:?}"),
    }
}

fn polygon_creates_six_sides() {
    let mut s = Session::new();
    s.execute("polygon", &json!({"sides": 6, "center": [0, 0], "radius": 5})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 6);
            assert!(poly.closed);
        }
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn ellipse_preserves_axes() {
    let mut s = Session::new();
    s.execute("ellipse", &json!({"center": [1, 2], "major": [5, 0], "ratio": 0.5})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ellipse(el) => {
            assert_eq!(el.center.x, 1.0);
            assert_eq!(el.center.y, 2.0);
            assert_eq!(el.major.x, 5.0);
            assert_eq!(el.ratio, 0.5);
        }
        other => panic!("expected Ellipse, got {other:?}"),
    }
}

fn single_point_uses_requested_coordinates() {
    let mut s = Session::new();
    let result = s.execute("point", &json!({"at": [6, 9]})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Point(p) => {
            assert_eq!(p.p.x, 6.0);
            assert_eq!(p.p.y, 9.0);
        }
        other => panic!("expected Point, got {other:?}"),
    }
}

fn multiple_points_are_independent() {
    let mut s = Session::new();
    let result = s.execute("point.multiple", &json!({"points": [[0, 0], [2, 3], [4, 5]]})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 3);
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

fn ray_keeps_normalized_direction() {
    let mut s = Session::new();
    s.execute("ray", &json!({"base": [1, 2], "through": [4, 6]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ray(ray) => {
            assert_eq!(ray.base.x, 1.0);
            assert!((ray.dir.x - 0.6).abs() < 1e-9);
            assert!((ray.dir.y - 0.8).abs() < 1e-9);
        }
        other => panic!("expected Ray, got {other:?}"),
    }
}

fn distance_query_computes_three_four_five() {
    let mut s = Session::new();
    let before = s.state().unwrap().revision;
    let r = s.execute("dist", &json!({"p1": [0, 0], "p2": [3, 4]})).unwrap();
    assert_eq!(r["distance"], 5.0);
    assert_eq!(r["dx"], 3.0);
    assert_eq!(r["dy"], 4.0);
    assert_eq!(s.state().unwrap().revision, before);
}

fn area_query_finds_rectangle_measurements() {
    let mut s = Session::new();
    let r = s.execute("area", &json!({"points": [[0, 0], [4, 0], [4, 3], [0, 3]]})).unwrap();
    assert_eq!(r["area"], 12.0);
    assert_eq!(r["perimeter"], 14.0);
}

fn id_query_updates_last_point() {
    let mut s = Session::new();
    let r = s.execute("id", &json!({"at": [6, 8]})).unwrap();
    assert_eq!(r["x"], 6.0);
    assert_eq!(r["y"], 8.0);
    assert_eq!(s.last_point, cadcraft_geom::Vec2::new(6.0, 8.0));
}

fn layer_list_includes_new_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Rockwork"})).unwrap();
    let response = s.execute("layer", &json!({})).unwrap();
    assert!(response["layers"].as_array().unwrap().iter().any(|layer| layer["name"] == "Rockwork"));
}

fn layer_set_changes_lock_state() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Rigging"})).unwrap();
    s.execute("layer.set", &json!({"name": "Rigging", "locked": true})).unwrap();
    let response = s.execute("layer", &json!({})).unwrap();
    assert!(response["layers"].as_array().unwrap().iter().any(|layer| layer["name"] == "Rigging" && layer["locked"] == true));
}

fn layer_current_sets_active_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Lighting"})).unwrap();
    s.execute("layer.current", &json!({"name": "Lighting"})).unwrap();
    assert_eq!(s.doc().unwrap().header.str("CLAYER", "0"), "Lighting");
}

fn properties_query_reports_selected_line() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let result = s.execute("properties", &json!({"handles": [h]})).unwrap();
    assert_eq!(result["count"], 1);
}

fn properties_set_changes_entity_color() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("properties.set", &json!({"handles": [h], "color": 3})).unwrap();
    let result = s.execute("properties", &json!({"handles": [h.clone()]})).unwrap();
    assert_eq!(result["count"], 1);
    let handle = cadcraft_doc::Handle::parse_hex(&h).unwrap();
    assert_eq!(s.doc().unwrap().entity(handle).unwrap().common.color, cadcraft_color::Color::Index(3));
}

fn drawing_inspection_counts_entities() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("circle", &json!({"center": [4, 4], "radius": 2})).unwrap();
    let report = s.execute("drawing.inspect", &json!({"entities": true})).unwrap();
    assert_eq!(report["entityCount"], 2);
    assert_eq!(report["entities"].as_array().unwrap().len(), 2);
}

fn entity_filter_excludes_other_shapes() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("circle", &json!({"center": [4, 4], "radius": 2})).unwrap();
    let report = s.execute("entities", &json!({"type": "Line"})).unwrap();
    assert_eq!(report["count"], 1);
    assert_eq!(report["entities"][0]["type"], "Line");
}

fn view_set_uses_exact_coordinates() {
    let mut s = Session::new();
    s.execute("view.set", &json!({"center": [8, 6], "height": 20})).unwrap();
    let view = s.state().unwrap().view();
    assert_eq!(view.center.x, 8.0);
    assert_eq!(view.center.y, 6.0);
    assert_eq!(view.height, 20.0);
}

fn view_get_reflects_new_view() {
    let mut s = Session::new();
    s.execute("view.set", &json!({"center": [7, 9], "height": 25})).unwrap();
    let result = s.execute("view.get", &json!({})).unwrap();
    assert_eq!(result["center"], json!([7.0, 9.0]));
    assert_eq!(result["height"], 25.0);
}

fn pan_changes_camera_not_geometry() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    let center = s.state().unwrap().view().center;
    s.execute("pan", &json!({"delta": [2, 3]})).unwrap();
    assert_eq!(s.state().unwrap().view().center, center - cadcraft_geom::Vec2::new(2.0, 3.0));
    assert_eq!(first_line(&s), before);
}

fn zoom_in_halves_view_height() {
    let mut s = Session::new();
    let height = s.state().unwrap().view().height;
    s.execute("zoom.in", &json!({})).unwrap();
    assert_eq!(s.state().unwrap().view().height, height / 2.0);
}

fn zoom_out_doubles_view_height() {
    let mut s = Session::new();
    let height = s.state().unwrap().view().height;
    s.execute("zoom.out", &json!({})).unwrap();
    assert_eq!(s.state().unwrap().view().height, height * 2.0);
}

fn regen_invalidates_view_without_geometry_change() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    let revision = s.state().unwrap().revision;
    s.execute("regen", &json!({})).unwrap();
    assert!(s.state().unwrap().revision > revision);
    assert_eq!(first_line(&s), before);
}

fn redraw_does_not_edit_entities() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    s.execute("redraw", &json!({})).unwrap();
    assert_eq!(first_line(&s), before);
}

fn time_query_reports_drawing_revision() {
    let mut s = Session::new();
    let report = s.execute("time", &json!({})).unwrap();
    assert!(report["message"].as_str().unwrap().contains("Drawing revision"));
}

fn status_query_reports_current_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Scenery", "current": true})).unwrap();
    let report = s.execute("status", &json!({})).unwrap();
    assert!(report["message"].as_str().unwrap().contains("Current layer: Scenery"));
}

fn count_query_on_empty_blocks() {
    let mut s = Session::new();
    let report = s.execute("count", &json!({"block": "NotPresent"})).unwrap();
    assert_eq!(report["count"], 0);
}

fn donut_creates_width_controlled_closed_outline() {
    let mut s = Session::new();
    s.execute("donut", &json!({"center": [10, 20], "inside": 2, "outside": 4})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 2);
            assert!(poly.closed);
            assert!((poly.const_width - 1.0).abs() < 1e-9);
        }
        other => panic!("expected donut polyline, got {other:?}"),
    }
}

fn circle_cd_uses_half_the_diameter() {
    let mut s = Session::new();
    s.execute("circle.cd", &json!({"center": [1, 2], "diameter": 6})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Circle(c) => {
            assert_eq!(c.radius, 3.0);
            assert_eq!(c.center.x, 1.0);
        }
        other => panic!("expected circle, got {other:?}"),
    }
}

fn circle_two_points_are_diameter_ends() {
    let mut s = Session::new();
    s.execute("circle.2p", &json!({"p1": [0, 0], "p2": [0, 6]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Circle(c) => {
            assert_eq!(c.radius, 3.0);
            assert_eq!(c.center.y, 3.0);
        }
        other => panic!("expected circle, got {other:?}"),
    }
}

fn circle_three_points_finds_circumcircle() {
    let mut s = Session::new();
    s.execute("circle.3p", &json!({"p1": [0, 0], "p2": [0, 4], "p3": [4, 0]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Circle(c) => {
            assert!((c.center.x - 2.0).abs() < 1e-9);
            assert!((c.center.y - 2.0).abs() < 1e-9);
            assert!((c.radius - 2.0 * 2.0_f64.sqrt()).abs() < 1e-9);
        }
        other => panic!("expected circle, got {other:?}"),
    }
}

fn arc_start_center_end_keeps_radius() {
    let mut s = Session::new();
    s.execute("arc.sce", &json!({"start": [3, 0], "center": [0, 0], "end": [0, 3]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Arc(a) => {
            assert_eq!(a.radius, 3.0);
            assert_eq!(a.center.x, 0.0);
            assert_eq!(a.center.y, 0.0);
        }
        other => panic!("expected arc, got {other:?}"),
    }
}

fn ellipse_axis_uses_half_minor_axis() {
    let mut s = Session::new();
    s.execute("ellipse.axis", &json!({"p1": [-4, 0], "p2": [4, 0], "distance": 2})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ellipse(el) => {
            assert!((el.major.x - 4.0).abs() < 1e-9);
            assert!((el.ratio - 0.5).abs() < 1e-9);
        }
        other => panic!("expected ellipse, got {other:?}"),
    }
}

fn elliptical_arc_keeps_ratio() {
    let mut s = Session::new();
    s.execute("ellipse.arc", &json!({"center": [0, 0], "major": [5, 0], "ratio": 0.5, "start": 0, "end": 90})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ellipse(el) => {
            assert_eq!(el.major.x, 5.0);
            assert_eq!(el.ratio, 0.5);
            assert!(el.start.is_finite() && el.end.is_finite());
        }
        other => panic!("expected elliptical arc, got {other:?}"),
    }
}

fn model_layer(s: &Session) -> String {
    s.doc().unwrap().model.iter().next().unwrap().common.layer.clone()
}

fn line_on_layer(s: &mut Session, name: &str) -> String {
    s.execute("layer.new", &json!({"name": name, "current": true})).unwrap();
    line_handle(s)
}

fn unused_layer_can_be_deleted() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name":"Temp"})).unwrap();
    s.execute("layer.delete", &json!({"name":"Temp"})).unwrap();
    assert!(s.doc().unwrap().layer("Temp").is_none());
}

fn object_layer_becomes_current() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Scenic");
    s.execute("layer.current", &json!({"name":"0"})).unwrap();
    s.execute("laymcur", &json!({"handles":[h]})).unwrap();
    assert_eq!(s.doc().unwrap().header.str("CLAYER", "0"), "Scenic");
}

fn layer_match_moves_entity() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("laymch", &json!({"handles":[h], "layer":"Rigging"})).unwrap();
    assert_eq!(model_layer(&s), "Rigging");
    assert!(s.doc().unwrap().layer("Rigging").is_some());
}

fn layer_current_moves_entities() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("layer.new", &json!({"name":"Rockwork", "current":true})).unwrap();
    s.execute("laycur", &json!({"handles":[h]})).unwrap();
    assert_eq!(model_layer(&s), "Rockwork");
}

fn isolate_hides_other_layers() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Foreground");
    s.execute("layer.new", &json!({"name":"Background"})).unwrap();
    s.execute("layiso", &json!({"handles":[h]})).unwrap();
    assert!(s.doc().unwrap().layer("Foreground").unwrap().on);
    assert!(!s.doc().unwrap().layer("Background").unwrap().on);
}

fn unisolate_restores_hidden_layers() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Foreground");
    s.execute("layer.new", &json!({"name":"Background"})).unwrap();
    s.execute("layiso", &json!({"handles":[h]})).unwrap();
    s.execute("layuniso", &json!({})).unwrap();
    assert!(s.doc().unwrap().layer("Background").unwrap().on);
}

fn layer_off_hides_object_layer() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Effects");
    s.execute("layoff", &json!({"handles":[h]})).unwrap();
    assert!(!s.doc().unwrap().layer("Effects").unwrap().on);
}

fn all_layers_on_restores_visibility() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Effects");
    s.execute("layoff", &json!({"handles":[h]})).unwrap();
    s.execute("layon", &json!({})).unwrap();
    assert!(s.doc().unwrap().layer("Effects").unwrap().on);
}

fn lock_object_layer() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Safe");
    s.execute("laylck", &json!({"handles":[h]})).unwrap();
    assert!(s.doc().unwrap().layer("Safe").unwrap().locked);
}

fn unlock_object_layer() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Safe");
    s.execute("laylck", &json!({"handles":[h.clone()]})).unwrap();
    s.execute("layulk", &json!({"handles":[h]})).unwrap();
    assert!(!s.doc().unwrap().layer("Safe").unwrap().locked);
}

fn freeze_object_layer() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Scenic");
    s.execute("layer.current", &json!({"name":"0"})).unwrap();
    s.execute("layfrz", &json!({"handles":[h]})).unwrap();
    assert!(s.doc().unwrap().layer("Scenic").unwrap().frozen);
}

fn thaw_all_layers() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name":"Closed", "frozen":true})).unwrap();
    assert!(s.doc().unwrap().layer("Closed").unwrap().frozen);
    s.execute("laythw", &json!({})).unwrap();
    assert!(!s.doc().unwrap().layer("Closed").unwrap().frozen);
}

fn previous_layer_restores_prior_layer_flags() {
    let mut s = Session::new();
    let h = line_on_layer(&mut s, "Draft");
    s.execute("layoff", &json!({"handles":[h]})).unwrap();
    assert!(!s.doc().unwrap().layer("Draft").unwrap().on);
    s.execute("layerp", &json!({})).unwrap();
    assert!(s.doc().unwrap().layer("Draft").unwrap().on);
}

fn layer_state_save_records_snapshot() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name":"Set"})).unwrap();
    s.execute("layerstate.save", &json!({"name":"Opening"})).unwrap();
    assert_eq!(s.doc().unwrap().layer_states[0].name, "Opening");
    assert!(s.doc().unwrap().layer_states[0].layers.iter().any(|l| l.name == "Set"));
}

fn layer_state_list_reports_saved_name() {
    let mut s = Session::new();
    s.execute("layerstate.save", &json!({"name":"Reveal"})).unwrap();
    let v = s.execute("layerstate.list", &json!({})).unwrap();
    assert_eq!(v["states"][0]["name"], "Reveal");
}

fn layer_state_restore_recovers_properties() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name":"Ambient", "color":2})).unwrap();
    s.execute("layerstate.save", &json!({"name":"Clean"})).unwrap();
    s.execute("layer.set", &json!({"name":"Ambient", "color":3, "on":false})).unwrap();
    s.execute("layerstate.restore", &json!({"name":"Clean"})).unwrap();
    let l = s.doc().unwrap().layer("Ambient").unwrap();
    assert!(l.on);
    assert_eq!(l.color, cadcraft_color::Color::Index(2));
}

fn layer_state_rename_updates_identity() {
    let mut s = Session::new();
    s.execute("layerstate.save", &json!({"name":"Scene1"})).unwrap();
    s.execute("layerstate.rename", &json!({"from":"Scene1", "to":"Scene2"})).unwrap();
    assert_eq!(s.doc().unwrap().layer_states[0].name, "Scene2");
}

fn layer_state_delete_removes_saved_state() {
    let mut s = Session::new();
    s.execute("layerstate.save", &json!({"name":"Scene1"})).unwrap();
    s.execute("layerstate.delete", &json!({"name":"Scene1"})).unwrap();
    assert!(s.doc().unwrap().layer_states.is_empty());
}

fn zoom_center_mode_sets_viewport() {
    let mut s = Session::new();
    s.execute("zoom", &json!({"mode":"center", "center":[3,4], "height":24})).unwrap();
    let v = s.state().unwrap().view();
    assert_eq!(v.center.x, 3.0);
    assert_eq!(v.center.y, 4.0);
    assert_eq!(v.height, 24.0);
}

fn zoom_window_uses_requested_bounds() {
    let mut s = Session::new();
    s.execute("zoom.window", &json!({"p1":[10, 20], "p2":[30, 40]})).unwrap();
    let v = s.state().unwrap().view();
    assert_eq!(v.center.x, 20.0);
    assert_eq!(v.center.y, 30.0);
    assert!(v.height >= 20.0);
    assert!(v.height <= 40.0);
}

fn zoom_previous_restores_last_camera() {
    let mut s = Session::new();
    s.execute("view.set", &json!({"center":[2,3], "height":40})).unwrap();
    s.execute("zoom.in", &json!({})).unwrap();
    assert_eq!(s.state().unwrap().view().height, 20.0);
    s.execute("zoom.previous", &json!({})).unwrap();
    let v = s.state().unwrap().view();
    assert_eq!(v.height, 40.0);
    assert_eq!(v.center.x, 2.0);
}

fn zoom_object_centers_on_selected_geometry() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("zoom.object", &json!({"handles":[h]})).unwrap();
    let v = s.state().unwrap().view();
    assert!((v.center.x - 0.5).abs() < 1e-9);
    assert!(v.height > 0.0);
}

fn zoom_all_includes_drawing_limits() {
    let mut s = Session::new();
    s.execute("zoom.all", &json!({})).unwrap();
    let v = s.state().unwrap().view();
    assert!(v.height > 0.0 && v.height.is_finite());
    assert!(v.center.is_finite());
}

fn pan_left_moves_camera_left() {
    let mut s = Session::new();
    let old = s.state().unwrap().view().center;
    s.execute("pan.left", &json!({})).unwrap();
    assert!(s.state().unwrap().view().center.x < old.x);
}

fn pan_right_moves_camera_right() {
    let mut s = Session::new();
    let old = s.state().unwrap().view().center;
    s.execute("pan.right", &json!({})).unwrap();
    assert!(s.state().unwrap().view().center.x > old.x);
}

fn pan_up_moves_camera_up() {
    let mut s = Session::new();
    let old = s.state().unwrap().view().center;
    s.execute("pan.up", &json!({})).unwrap();
    assert!(s.state().unwrap().view().center.y > old.y);
}

fn pan_down_moves_camera_down() {
    let mut s = Session::new();
    let old = s.state().unwrap().view().center;
    s.execute("pan.down", &json!({})).unwrap();
    assert!(s.state().unwrap().view().center.y < old.y);
}

fn three_d_polyline_preserves_elevation() {
    let mut s = Session::new();
    s.execute("3dpoly", &json!({"points": [[0, 0, 0], [3, 4, 5], [6, 7, 8]]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Polyline3d(pl) => {
            assert_eq!(pl.points.len(), 3);
            assert_eq!(pl.points[1].z, 5.0);
            assert_eq!(pl.points[2].z, 8.0);
        }
        other => panic!("expected a 3D polyline, got {other:?}"),
    }
}

fn helix_has_expected_height_and_turns() {
    let mut s = Session::new();
    s.execute("helix", &json!({"center": [0, 0], "baseRadius": 2, "topRadius": 2, "height": 8, "turns": 2})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Polyline3d(pl) => {
            assert!(pl.points.len() >= 72);
            assert_eq!(pl.points.first().unwrap().z, 0.0);
            assert!((pl.points.last().unwrap().z - 8.0).abs() < 1e-9);
            assert!(pl.points.iter().all(|point| point.is_finite()));
        }
        other => panic!("expected a 3D helix polyline, got {other:?}"),
    }
}

fn erase_deletes_selected_line_and_undo_recovers() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    assert_eq!(s.execute("erase", &json!({"handles":[h]})).unwrap()["erased"], 1);
    assert_eq!(s.doc().unwrap().model.len(), 0);
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
}

fn rect_array_creates_correct_grid() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let r = s.execute("arrayrect", &json!({"handles":[h], "rows":2, "cols":3, "rowSpacing":10, "colSpacing":5})).unwrap();
    assert_eq!(r["created"], 5);
    assert_eq!(s.doc().unwrap().model.len(), 6);
    let starts: Vec<(f64, f64)> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) => Some((l.a.x, l.a.y)),
            _ => None,
        })
        .collect();
    for y in [0.0, 10.0] {
        for x in [0.0, 5.0, 10.0] {
            assert!(starts.contains(&(x, y)), "grid missing ({x},{y}): {starts:?}");
        }
    }
}

fn polar_array_rotates_copies() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let r = s.execute("arraypolar", &json!({"handles":[h], "center":[0,0], "count":4, "angle":360})).unwrap();
    assert_eq!(r["created"], 3);
    assert_eq!(s.doc().unwrap().model.len(), 4);
    let ends: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) => Some((l.b.x, l.b.y)),
            _ => None,
        })
        .collect();
    for (x, y) in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
        assert!(ends.iter().any(|(a, b)| (a - x).abs() < 1e-9 && (b - y).abs() < 1e-9), "polar array missing ({x},{y}): {ends:?}");
    }
}

fn draworder_front_moves_entity_to_end() {
    let mut s = Session::new();
    let a = line_handle(&mut s);
    let b = line_handle(&mut s);
    s.execute("draworder.front", &json!({"handles":[a.clone()]})).unwrap();
    let order: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    assert_eq!(order, vec![b, a]);
}

fn draworder_back_moves_entity_to_start() {
    let mut s = Session::new();
    let a = line_handle(&mut s);
    let b = line_handle(&mut s);
    s.execute("draworder.back", &json!({"handles":[b.clone()]})).unwrap();
    let order: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    assert_eq!(order, vec![b, a]);
}

fn break_removes_middle_segment() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let r = s.execute("break", &json!({"handle":h, "p1":[3,0], "p2":[7,0]})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 2);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let endpoints: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) => Some((l.a.x, l.b.x)),
            _ => None,
        })
        .collect();
    assert!(endpoints.contains(&(0.0, 3.0)), "{endpoints:?}");
    assert!(endpoints.contains(&(7.0, 10.0)), "{endpoints:?}");
}

fn break_atpoint_splits_line_without_gap() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let r = s.execute("breakatpoint", &json!({"handle":h, "at":[4,0]})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 2);
    let endpoints: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) => Some((l.a.x, l.b.x)),
            _ => None,
        })
        .collect();
    assert!(endpoints.contains(&(0.0, 4.0)), "{endpoints:?}");
    assert!(endpoints.contains(&(4.0, 10.0)), "{endpoints:?}");
}

fn join_chains_two_segments() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[3,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[3,0],[3,4]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let r = s.execute("join", &json!({"handles":[a,b]})).unwrap();
    assert!(r["handle"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(p) => {
            assert_eq!(p.vertices.len(), 3);
            assert_eq!((p.vertices[1].p.x, p.vertices[1].p.y), (3.0, 0.0));
        }
        other => panic!("join produced {other:?} instead of a polyline"),
    }
}

fn explode_splits_polyline_into_lines() {
    let mut s = Session::new();
    let h = s.execute("pline", &json!({"vertices":[[0,0],[3,0],[3,4]]})).unwrap()["handle"].as_str().unwrap().to_string();
    let r = s.execute("explode", &json!({"handles":[h]})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 2);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    assert!(s.doc().unwrap().model.iter().all(|e| matches!(&e.kind, EntityKind::Line(_))));
}

fn overkill_removes_duplicate_geometry() {
    let mut s = Session::new();
    line_handle(&mut s);
    line_handle(&mut s);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let r = s.execute("overkill", &json!({})).unwrap();
    assert_eq!(r["deleted"], 1);
    assert_eq!(s.doc().unwrap().model.len(), 1);
}

fn offset_circle_changes_radius() {
    let mut s = Session::new();
    let h = s.execute("circle", &json!({"center":[0,0],"radius":2})).unwrap()["handle"].as_str().unwrap().to_string();
    let r = s.execute("offset", &json!({"handle":h, "distance":1, "side":[5,0]})).unwrap();
    assert!(r["handle"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let radii: Vec<f64> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Circle(c) => Some(c.radius),
            _ => None,
        })
        .collect();
    assert!(radii.contains(&2.0) && radii.contains(&3.0), "{radii:?}");
}

fn trim_keeps_expected_segment() {
    let mut s = Session::new();
    let target = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let boundary = s.execute("line", &json!({"points":[[5,-2],[5,2]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let result = s.execute("trim", &json!({"handle":target,"pick":[8,0],"edges":[boundary]})).unwrap();
    let handles = result["handles"].as_array().unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let kept = cadcraft_doc::Handle::parse_hex(handles[0].as_str().unwrap()).unwrap();
    match &s.doc().unwrap().entity(kept).unwrap().kind {
        EntityKind::Line(l) => {
            assert_eq!((l.a.x, l.a.y), (0.0, 0.0));
            assert!((l.b.x - 5.0).abs() < 1e-9 && l.b.y.abs() < 1e-9);
        }
        other => panic!("expected retained segment, got {other:?}"),
    }
}

fn extend_meets_nearest_cutting_boundary() {
    let mut s = Session::new();
    let target = s.execute("line", &json!({"points":[[0,0],[2,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let boundary = s.execute("line", &json!({"points":[[5,-2],[5,2]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    s.execute("extend", &json!({"handle":target,"pick":[2,0],"edges":[boundary]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let kept = cadcraft_doc::Handle::parse_hex(&target).unwrap();
    match &s.doc().unwrap().entity(kept).unwrap().kind {
        EntityKind::Line(l) => {
            assert_eq!((l.a.x, l.a.y), (0.0, 0.0));
            assert!((l.b.x - 5.0).abs() < 1e-9 && l.b.y.abs() < 1e-9);
        }
        other => panic!("expected extended line, got {other:?}"),
    }
}

fn divide_stations_match_geometry() {
    let mut s = Session::new();
    let source = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let result = s.execute("divide", &json!({"handle":source,"segments":5})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 4);
    assert_eq!(s.doc().unwrap().model.len(), 5);
    let mut stations: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Point(point) => Some((point.p.x, point.p.y)),
            _ => None,
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(stations, vec![(2.0, 0.0), (4.0, 0.0), (6.0, 0.0), (8.0, 0.0)]);
}

fn revcloud_replaces_source_with_closed_boundary() {
    let mut s = Session::new();
    let source = s.execute("circle", &json!({"center":[0,0],"radius":5})).unwrap()["handle"].as_str().unwrap().to_string();
    let result = s.execute("revcloud", &json!({"handle":source,"arcLength":3})).unwrap();
    let generated = result["handle"].as_str().unwrap();
    assert_ne!(generated, source);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    assert!(s.doc().unwrap().entity(cadcraft_doc::Handle::parse_hex(&source).unwrap()).is_none());
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert!(poly.closed);
            assert!(poly.vertices.len() >= 3);
            assert!(poly.vertices.iter().all(|v| v.p.is_finite()));
        }
        other => panic!("expected finite closed revision cloud, got {other:?}"),
    }
}

fn pedit_joins_touching_geometry() {
    let mut s = Session::new();
    let base = s.execute("pline", &json!({"vertices":[[0,0],[2,0]]})).unwrap()["handle"].as_str().unwrap().to_string();
    let candidate = s.execute("line", &json!({"points":[[2,0],[4,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let result = s.execute("pedit", &json!({"handle":base,"option":"join","handles2":[candidate]})).unwrap();
    assert_eq!(result["joined"], 1);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 3);
            assert_eq!((poly.vertices[0].p.x, poly.vertices[0].p.y), (0.0, 0.0));
            assert_eq!((poly.vertices[2].p.x, poly.vertices[2].p.y), (4.0, 0.0));
            assert!(!poly.closed);
        }
        other => panic!("expected joined polyline, got {other:?}"),
    }
}

fn text_preserves_content_and_position() {
    let mut s = Session::new();
    let result = s.execute("text", &json!({"at":[3,7],"text":"SHOW SET A","height":2.0,"rotation":90})).unwrap();
    assert!(result["handle"].is_string());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Text(text) => {
            assert_eq!(text.value, "SHOW SET A");
            assert_eq!((text.insert.x, text.insert.y), (3.0, 7.0));
            assert_eq!(text.height, 2.0);
            assert!((text.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        }
        other => panic!("expected single-line text, got {other:?}"),
    }
}

fn mtext_preserves_paragraphs_and_dimensions() {
    let mut s = Session::new();
    let result = s.execute("mtext", &json!({"at":[2,4],"text":"Scenic\nFabrication","height":1.5,"width":12.0,"attach":5})).unwrap();
    assert!(result["handle"].is_string());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::MText(text) => {
            assert_eq!(text.contents, "Scenic\\PFabrication");
            assert_eq!((text.insert.x, text.insert.y), (2.0, 4.0));
            assert_eq!(text.width, 12.0);
            assert_eq!(text.height, 1.5);
            assert_eq!(text.attach, 5);
        }
        other => panic!("expected multiline text, got {other:?}"),
    }
}

fn textedit_changes_text_without_replacing_handle() {
    let mut s = Session::new();
    let original = s.execute("text", &json!({"at":[3,7],"text":"REV A","height":1.0})).unwrap()["handle"].as_str().unwrap().to_owned();
    let result = s.execute("textedit", &json!({"handle":original,"text":"REV B"})).unwrap();
    assert_eq!(result["handle"], original);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    let h = cadcraft_doc::Handle::parse_hex(&original).unwrap();
    match &s.doc().unwrap().entity(h).unwrap().kind {
        EntityKind::Text(text) => {
            assert_eq!(text.value, "REV B");
            assert_eq!((text.insert.x, text.insert.y), (3.0, 7.0));
        }
        other => panic!("expected edited text, got {other:?}"),
    }
}

fn xline_keeps_unit_direction() {
    let mut s = Session::new();
    let result = s.execute("xline", &json!({"base":[2,3],"through":[8,3]})).unwrap();
    assert!(result["handle"].is_string());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::XLine(line) => {
            assert_eq!((line.base.x, line.base.y), (2.0, 3.0));
            assert!((line.dir.x - 1.0).abs() < 1e-9);
            assert!(line.dir.y.abs() < 1e-9);
        }
        other => panic!("expected infinite construction line, got {other:?}"),
    }
}

fn mline_builds_two_parallel_rails() {
    let mut s = Session::new();
    let result = s.execute("mline", &json!({"points":[[0,0],[10,0]],"scale":4.0,"justification":"zero"})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 2);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let mut rails = Vec::new();
    for entity in s.doc().unwrap().model.iter() {
        match &entity.kind {
            EntityKind::LwPolyline(poly) => {
                assert_eq!(poly.vertices.len(), 2);
                assert!(!poly.closed);
                let a = poly.vertices[0].p;
                let b = poly.vertices[1].p;
                assert!((a.x - 0.0).abs() < 1e-9 && (b.x - 10.0).abs() < 1e-9);
                assert!((a.y - b.y).abs() < 1e-9);
                rails.push(a.y);
            }
            other => panic!("expected two offset polylines, got {other:?}"),
        }
    }
    rails.sort_by(f64::total_cmp);
    assert_eq!(rails, vec![-2.0, 2.0]);
}

fn centermark_crosses_circle_center() {
    let mut s = Session::new();
    let circle = s.execute("circle", &json!({"center":[3,4],"radius":5})).unwrap()["handle"].as_str().unwrap().to_owned();
    let result = s.execute("centermark", &json!({"handle":circle})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 2);
    assert_eq!(s.doc().unwrap().model.len(), 3);
    let mut horizontal = false;
    let mut vertical = false;
    for entity in s.doc().unwrap().model.iter() {
        if let EntityKind::Line(l) = &entity.kind {
            assert_eq!(entity.common.linetype, "CENTER");
            if (l.a.y - 4.0).abs() < 1e-9 && (l.b.y - 4.0).abs() < 1e-9 {
                assert!(l.a.x < 3.0 && l.b.x > 3.0);
                horizontal = true;
            }
            if (l.a.x - 3.0).abs() < 1e-9 && (l.b.x - 3.0).abs() < 1e-9 {
                assert!(l.a.y < 4.0 && l.b.y > 4.0);
                vertical = true;
            }
        }
    }
    assert!(horizontal && vertical);
}

fn centerline_bisects_parallel_lines() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_owned();
    let b = s.execute("line", &json!({"points":[[0,4],[10,4]]})).unwrap()["handles"][0].as_str().unwrap().to_owned();
    let result = s.execute("centerline", &json!({"h1":a,"h2":b})).unwrap();
    let generated = cadcraft_doc::Handle::parse_hex(result["handle"].as_str().unwrap()).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 3);
    let entity = s.doc().unwrap().entity(generated).unwrap();
    assert_eq!(entity.common.linetype, "CENTER");
    match &entity.kind {
        EntityKind::Line(l) => {
            assert!((l.a.y - 2.0).abs() < 1e-9 && (l.b.y - 2.0).abs() < 1e-9);
            assert!(l.a.x < 0.0 && l.b.x > 10.0);
        }
        other => panic!("expected midpoint center line, got {other:?}"),
    }
}

fn native_polyline_station_returns_expected_point() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let points = json!({"kind":"polyline","value":[
        {"x":0.0,"y":0.0,"z":0.0},
        {"x":10.0,"y":0.0,"z":0.0},
        {"x":10.0,"y":10.0,"z":0.0}
    ]});
    let result = s
        .execute(
            "worldwright.polyline.point_at_station",
            &json!({"inputs":{
                "points":points,"station":{"kind":"number","value":15.0}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "point");
    assert_eq!(result["output"]["value"], json!({"x":10.0,"y":5.0,"z":0.0}));
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
}

fn native_polyline_station_returns_unit_tangent() {
    let mut s = Session::new();
    let result = s
        .execute(
            "worldwright.polyline.tangent_at_station",
            &json!({"inputs":{
                "points":{"kind":"polyline","value":[
                    {"x":0.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":10.0,"z":0.0}
                ]},
                "station":{"kind":"number","value":15.0}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "vector");
    assert_eq!(result["output"]["value"], json!({"x":0.0,"y":1.0,"z":0.0}));
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn native_polyline_offset_returns_parallel_rail() {
    let mut s = Session::new();
    let result = s
        .execute(
            "worldwright.polyline.offset_xy",
            &json!({"inputs":{
                "points":{"kind":"polyline","value":[
                    {"x":0.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":10.0,"z":0.0}
                ]},
                "distance":{"kind":"number","value":1.0},
                "miter_limit":{"kind":"number","value":4.0}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "polyline");
    let points = result["output"]["value"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(points[1], json!({"x":9.0,"y":1.0,"z":0.0}));
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn native_curve_join_preserves_common_end() {
    let mut s = Session::new();
    let result = s
        .execute(
            "worldwright.curve.join",
            &json!({"inputs":{
                "a":{"kind":"polyline","value":[{"x":0.0,"y":0.0,"z":0.0},{"x":4.0,"y":0.0,"z":0.0}]},
                "b":{"kind":"polyline","value":[{"x":4.0,"y":0.0,"z":0.0},{"x":10.0,"y":0.0,"z":0.0}]},
                "tolerance":{"kind":"number","value":0.001}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "polyline");
    assert_eq!(
        result["output"]["value"],
        json!([
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":4.0,"y":0.0,"z":0.0},
            {"x":10.0,"y":0.0,"z":0.0}
        ])
    );
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn native_curve_trim_keeps_requested_interval() {
    let mut s = Session::new();
    let result = s
        .execute(
            "worldwright.curve.trim",
            &json!({"inputs":{
                "points":{"kind":"polyline","value":[
                    {"x":0.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":10.0,"z":0.0}
                ]},
                "start":{"kind":"number","value":2.0},
                "end":{"kind":"number","value":15.0}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "polyline");
    assert_eq!(
        result["output"]["value"],
        json!([
            {"x":2.0,"y":0.0,"z":0.0},
            {"x":10.0,"y":0.0,"z":0.0},
            {"x":10.0,"y":5.0,"z":0.0}
        ])
    );
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn native_curve_extend_follows_tangent() {
    let mut s = Session::new();
    let result = s
        .execute(
            "worldwright.curve.extend",
            &json!({"inputs":{
                "points":{"kind":"polyline","value":[
                    {"x":0.0,"y":0.0,"z":0.0},
                    {"x":10.0,"y":0.0,"z":0.0}
                ]},
                "distance":{"kind":"number","value":3.0},
                "at_start":{"kind":"count","value":0}
            }}),
        )
        .unwrap();
    assert_eq!(result["output"]["kind"], "polyline");
    assert_eq!(
        result["output"]["value"],
        json!([
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":10.0,"y":0.0,"z":0.0},
            {"x":13.0,"y":0.0,"z":0.0}
        ])
    );
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn expect_native_output(command: &str, inputs: serde_json::Value, expected: serde_json::Value) {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    let actual = s.execute(command, &json!({"inputs":inputs})).unwrap();
    assert_eq!(actual["output"], expected, "unexpected native result for {command}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo);
}

fn native_vector_length_is_13() {
    let v = json!({"kind":"vector","value":{"x":3.0,"y":4.0,"z":12.0}});
    expect_native_output(
        "worldwright.vector.length",
        json!({"v":v}),
        json!({"kind":"number","value":13.0}),
    );
}

fn native_vector_normalize_is_unit() {
    let v = json!({"kind":"vector","value":{"x":3.0,"y":4.0,"z":0.0}});
    expect_native_output(
        "worldwright.vector.normalize",
        json!({"v":v}),
        json!({"kind":"vector","value":{"x":0.6,"y":0.8,"z":0.0}}),
    );
}

fn native_vector_dot_is_scalar() {
    let x = json!({"kind":"vector","value":{"x":1.0,"y":2.0,"z":3.0}});
    let y = json!({"kind":"vector","value":{"x":4.0,"y":5.0,"z":6.0}});
    expect_native_output(
        "worldwright.vector.dot",
        json!({"a":x,"b":y}),
        json!({"kind":"number","value":32.0}),
    );
}

fn native_vector_cross_is_right_handed() {
    let x = json!({"kind":"vector","value":{"x":1.0,"y":0.0,"z":0.0}});
    let y = json!({"kind":"vector","value":{"x":0.0,"y":1.0,"z":0.0}});
    expect_native_output(
        "worldwright.vector.cross",
        json!({"a":x,"b":y}),
        json!({"kind":"vector","value":{"x":0.0,"y":0.0,"z":1.0}}),
    );
}

fn native_polyline_length_is_7() {
    let path = json!({"kind":"polyline","value":[
        {"x":0.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":4.0,"z":0.0}
    ]});
    expect_native_output(
        "worldwright.polyline.length",
        json!({"points":path}),
        json!({"kind":"number","value":7.0}),
    );
}

fn native_polyline_divides_by_count() {
    let path = json!({"kind":"polyline","value":[
        {"x":0.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":4.0,"z":0.0}
    ]});
    expect_native_output(
        "worldwright.polyline.divide_count",
        json!({"points":path,"count":{"kind":"count","value":2}}),
        json!({"kind":"polyline","value":[
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":3.0,"y":0.5,"z":0.0},
            {"x":3.0,"y":4.0,"z":0.0}
        ]}),
    );
}

fn native_polyline_divides_by_spacing() {
    let path = json!({"kind":"polyline","value":[
        {"x":0.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":0.0,"z":0.0},
        {"x":3.0,"y":4.0,"z":0.0}
    ]});
    expect_native_output(
        "worldwright.polyline.divide_distance",
        json!({"points":path,"spacing":{"kind":"number","value":2.0}}),
        json!({"kind":"polyline","value":[
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":2.0,"y":0.0,"z":0.0},
            {"x":3.0,"y":1.0,"z":0.0},
            {"x":3.0,"y":3.0,"z":0.0},
            {"x":3.0,"y":4.0,"z":0.0}
        ]}),
    );
}

fn native_curve_split_yields_two_actual_fragments() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let result = s.execute(
        "worldwright.curve.split",
        &json!({"inputs":{
            "points":{"kind":"polyline","value":[
                {"x":0.0,"y":0.0,"z":0.0},
                {"x":10.0,"y":0.0,"z":0.0},
                {"x":10.0,"y":10.0,"z":0.0}
            ]},
            "station":{"kind":"number","value":15.0}
        }}),
    ).unwrap();
    assert_eq!(result["output"]["kind"], "tree");
    let branches = result["output"]["value"]["branches"].as_array().unwrap();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0]["path"], json!([0]));
    assert_eq!(branches[1]["path"], json!([1]));
    assert_eq!(branches[0]["items"][0]["value"], json!([
        {"x":0.0,"y":0.0,"z":0.0},
        {"x":10.0,"y":0.0,"z":0.0},
        {"x":10.0,"y":5.0,"z":0.0}
    ]));
    assert_eq!(branches[1]["items"][0]["value"], json!([
        {"x":10.0,"y":5.0,"z":0.0},
        {"x":10.0,"y":10.0,"z":0.0}
    ]));
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
}

fn path_array_places_copies_along_rail() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let path = s.execute("line", &json!({"points":[[0,5],[30,5]]})).unwrap()["handles"][0].clone();
    let r = s.execute("arraypath", &json!({"handles":[h], "path":path, "count":4, "align":true})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 3);
    assert_eq!(s.doc().unwrap().model.len(), 5);
    let starts: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Line(l) if l.a.y.abs() < 1e-9 => Some(l.a.x),
            _ => None,
        })
        .collect();
    for x in [0.0, 10.0, 20.0, 30.0] {
        assert!(starts.iter().any(|actual| (actual - x).abs() < 1e-8), "missing station {x}: {starts:?}");
    }
}

fn reverse_swaps_line_endpoints() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let result = s.execute("reverse", &json!({"handles":[h]})).unwrap();
    assert_eq!(result["reversed"], 1);
    assert_eq!(first_line(&s), (1.0, 0.0, 0.0, 0.0));
}

fn flatten_projects_3d_polyline() {
    let mut s = Session::new();
    let handle = s.execute("3dpoly", &json!({"points":[[0,0,3],[1,2,4],[3,2,8]]})).unwrap()["handle"].as_str().unwrap().to_string();
    let result = s.execute("flatten", &json!({"handles":[handle]})).unwrap();
    assert_eq!(result["flattened"], 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(p) => {
            assert_eq!(p.vertices.len(), 3);
            assert_eq!(p.vertices[1].p.x, 1.0);
            assert_eq!(p.vertices[1].p.y, 2.0);
            assert_eq!(p.elevation, 0.0);
        }
        other => panic!("expected flattened planar polyline, got {other:?}"),
    }
}

fn align_translates_with_two_reference_points() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let result = s.execute("align", &json!({"handles":[h], "s1":[0,0], "d1":[2,3]})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 1);
    assert_eq!(first_line(&s), (2.0, 3.0, 3.0, 3.0));
}

fn lengthen_changes_line_length() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let result = s.execute("lengthen", &json!({"handle":h, "total":5})).unwrap();
    assert_eq!(result["length"], 5.0);
    assert_eq!(first_line(&s), (0.0, 0.0, 5.0, 0.0));
}

fn fillet_creates_radius_two_arc() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,0],[0,10]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let result = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":[9,0],"p2":[0,9],"radius":2})).unwrap();
    assert!(result["arc"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
    let radii: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|entity| match &entity.kind {
            EntityKind::Arc(arc) => Some(arc.radius),
            _ => None,
        })
        .collect();
    assert_eq!(radii, vec![2.0]);
}

fn chamfer_creates_diagonal_bridge() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,0],[0,10]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let result = s.execute("chamfer", &json!({"h1":a,"h2":b,"p1":[9,0],"p2":[0,9],"d1":2,"d2":3})).unwrap();
    assert!(result["line"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
    let diagonal = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|entity| match &entity.kind {
            EntityKind::Line(line) if (line.a.x - line.b.x).abs() > 0.1 && (line.a.y - line.b.y).abs() > 0.1 => Some(line),
            _ => None,
        })
        .count();
    assert_eq!(diagonal, 1);
}

fn spline_control_degree_is_preserved() {
    let mut s = Session::new();
    s.execute("spline", &json!({"control":[[0,0],[2,3],[4,1],[6,0]],"degree":2})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Spline(sp) => {
            assert_eq!(sp.degree, 2);
            assert_eq!(sp.control.len(), 4);
            assert!(sp.is_valid());
        }
        other => panic!("expected degree-2 spline, got {other:?}"),
    }
}

fn spline_cv_degree_is_preserved() {
    let mut s = Session::new();
    s.execute("spline.cv", &json!({"control":[[0,0],[2,3],[4,1],[6,0]],"degree":3})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Spline(sp) => {
            assert_eq!(sp.degree, 3);
            assert_eq!(sp.control.len(), 4);
            assert!(sp.is_valid());
        }
        other => panic!("expected degree-3 CV spline, got {other:?}"),
    }
}

fn list_reports_line_length_and_handle() {
    let mut s = Session::new();
    let created = s.execute("line", &json!({"points":[[0,0],[3,4]]})).unwrap();
    let handle = created["handles"][0].as_str().unwrap();
    let details = s.execute("list", &json!({"handles":[handle]})).unwrap();
    let text = details["message"].as_str().unwrap();
    assert!(text.contains("LINE"));
    assert!(text.contains(&format!("Handle = {handle}")));
    assert!(text.contains("Length = 5"));
}

fn measure_places_all_four_line_stations() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let result = s.execute("measure", &json!({"handle":h, "length":2})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 4);
    let xs: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Point(p) => Some(p.p.x),
            _ => None,
        })
        .collect();
    for station in [2.0, 4.0, 6.0, 8.0] {
        assert!(xs.contains(&station), "missing station {station}: {xs:?}");
    }
}

#[test]
fn positive_operational_cases() {
    let registry: HashSet<_> = command_specs().iter().map(|c| c.id).collect();
    assert_eq!(registry.len(), command_specs().len(), "duplicate command IDs in the live registry");
    let mut seen = HashSet::new();
    for (id, verify) in CASES {
        assert!(registry.contains(id), "acceptance case refers to missing command {id}");
        assert!(find_command(id).is_some());
        assert!(seen.insert(*id), "duplicate operational case for {id}");
        verify();
    }
    eprintln!("Worldwright positive functional cases: {} / {} registered commands", CASES.len(), registry.len());
}

#[test]
#[ignore = "Operational alpha release gate: needs independently passing success, failure, undo and persistence cases for every command"]
fn release_gate_no_command_without_operational_case() {
    // This must not be reclassified as green by the ordinary test runner.
    // Extend CASES with genuine positive fixtures, not name-only placeholders.
    positive_operational_cases();
    let proven: HashSet<_> = CASES.iter().map(|(id, _)| *id).collect();
    let mut missing: Vec<_> = command_specs().iter().map(|spec| spec.id).filter(|id| !proven.contains(id)).collect();
    missing.sort_unstable();
    assert!(missing.is_empty(), "{} commands lack positive operational fixtures:\n{}", missing.len(), missing.join(", "));
}
