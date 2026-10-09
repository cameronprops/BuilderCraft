//! Headless CAD/API adapters for the shared closest-point kernel.
//! Scans are linear and explicitly bounded until 3D spatial indexing lands.
use super::*;
use buildercraft_kernel::{Cancellation, TriangleMesh, cloud_closest_point, mesh_closest_point};
use cadcraft_geom::Vec3;
use serde::Deserialize;
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.mesh.closest_point", "Closest Point on Triangle Mesh", mesh)
            .params("{mesh:{vertices:[{x,y,z}],triangles:[[a,b,c]]},query:[x,y,z],max_distance?:positive} -> {point?,distance?,triangle_index?,barycentric?}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.cloud.closest_point", "Closest Sample in Point Cloud", cloud)
            .params("{points:[{x,y,z}],query:[x,y,z],max_distance?:positive} -> {point?,distance?,point_index?}")
            .enabled(always)
            .noundo(),
    ]
}

fn invalid(message: &str) -> EngineError {
    bad("worldwright.closest_point", message)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshQuery {
    mesh: TriangleMesh,
    query: [f64; 3],
    max_distance: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloudQuery {
    points: Vec<Vec3>,
    query: [f64; 3],
    max_distance: Option<f64>,
}

fn point(coords: [f64; 3]) -> Vec3 {
    Vec3::new(coords[0], coords[1], coords[2])
}

fn mesh(_: &mut Session, p: &Value) -> Result<Value> {
    let source = p.get("mesh").ok_or_else(|| invalid("mesh object required"))?;
    let vertex_count = source
        .get("vertices")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| invalid("mesh vertices array required"))?;
    let face_count = source
        .get("triangles")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| invalid("mesh triangles array required"))?;
    if vertex_count == 0 || face_count == 0 || vertex_count > 65_536 || face_count > 65_536 {
        return Err(invalid("mesh query budget exceeded"));
    }
    let query: MeshQuery = serde_json::from_value(p.clone()).map_err(|e| invalid(&e.to_string()))?;
    let result = mesh_closest_point(
        &query.mesh,
        point(query.query),
        query.max_distance,
        &Cancellation::default(),
    )
    .map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({
        "hit": result.map(|hit| json!({
            "point": [hit.point.x, hit.point.y, hit.point.z],
            "distance": hit.distance,
            "triangle_index": hit.triangle_index,
            "barycentric": hit.barycentric,
            "degenerate": hit.degenerate
        })),
        "method": "bounded_linear_scan",
        "signed": false
    }))
}

fn cloud(_: &mut Session, p: &Value) -> Result<Value> {
    let sample_count = p
        .get("points")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| invalid("points array required"))?;
    if sample_count == 0 || sample_count > 65_536 {
        return Err(invalid("point-cloud query budget exceeded"));
    }
    let query: CloudQuery = serde_json::from_value(p.clone()).map_err(|e| invalid(&e.to_string()))?;
    let result = cloud_closest_point(
        &query.points,
        point(query.query),
        query.max_distance,
        &Cancellation::default(),
    )
    .map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({
        "hit": result.map(|hit| json!({
            "point": [hit.point.x, hit.point.y, hit.point.z],
            "distance": hit.distance,
            "point_index": hit.point_index
        })),
        "method": "bounded_linear_scan"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_command_uses_shared_kernel_and_reports_source_triangle() {
        let mut s = Session::new();
        let params = json!({
            "mesh":{
                "vertices":[
                    {"x":0.0,"y":0.0,"z":0.0},
                    {"x":2.0,"y":0.0,"z":0.0},
                    {"x":0.0,"y":2.0,"z":0.0}
                ],
                "triangles":[[0,1,2]]
            },
            "query":[0.5,0.5,3.0]
        });
        let revision = s.state().unwrap().revision;
        let response = s.execute("worldwright.mesh.closest_point", &params).unwrap();
        assert_eq!(response["hit"]["point"], json!([0.5, 0.5, 0.0]));
        assert_eq!(response["hit"]["distance"], 3.0);
        assert_eq!(response["hit"]["triangle_index"], 0);
        assert_eq!(response["hit"]["barycentric"], json!([0.5, 0.25, 0.25]));
        assert_eq!(s.state().unwrap().revision, revision);
        let mut limited = params;
        limited["max_distance"] = json!(2.0);
        assert!(s.execute("worldwright.mesh.closest_point", &limited).unwrap()["hit"].is_null());
    }

    #[test]
    fn cloud_query_returns_stable_sample_and_never_edits_drawing() {
        let mut s = Session::new();
        let revision = s.state().unwrap().revision;
        let args = json!({
            "points":[{"x":-1.0,"y":0.0,"z":0.0},{"x":1.0,"y":0.0,"z":0.0}],
            "query":[0.0,0.0,0.0]
        });
        let out = s.execute("worldwright.cloud.closest_point", &args).unwrap();
        assert_eq!(out["hit"]["point_index"], 0);
        assert_eq!(out["hit"]["distance"], 1.0);
        assert_eq!(s.state().unwrap().revision, revision);
        assert!(s.execute("worldwright.cloud.closest_point", &json!({
            "points":[{"x":0.0,"y":0.0,"z":0.0}],"query":[0.0,0.0,0.0],"max_distance":-1.0
        })).is_err());
    }

    #[test]
    fn invalid_indices_or_nonfinite_limits_are_rejected() {
        let mut s = Session::new();
        assert!(s.execute("worldwright.mesh.closest_point", &json!({
            "mesh":{"vertices":[{"x":0.0,"y":0.0,"z":0.0}],"triangles":[[0,0,3]]},
            "query":[0.0,0.0,0.0]
        })).is_err());
        assert!(s.execute("worldwright.cloud.closest_point", &json!({
            "points":[],"query":[0.0,0.0,0.0]
        })).is_err());
    }
}
