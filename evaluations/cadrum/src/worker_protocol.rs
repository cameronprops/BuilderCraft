//! Bounded one-shot JSON API for an out-of-process exact BRep kernel.
//!
//! The worker is not a CAD document model or a native-Rust OCCT rewrite.
//! It moves potentially fatal native FFI calls into a replaceable OS process.
//! The transport stores exact BRep topology, not triangulated preview meshes.

use std::io::{self, Read, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use cadrum::DVec3;
use serde::{Deserialize, Serialize};

use crate::adapter::{AbsoluteTolerance, BrepSolid, CadrumBrepCandidate, ExactBoolean};

/// Limits are intentionally smaller than the native 128 MiB adapter budget.
/// These limits bound JSON transport and encoded native data, not C++ heap RSS.
pub const MAX_REQUEST_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_BINARY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_SOLIDS: usize = 64;
const MAX_PROXY_VERTICES: usize = 30_000;
const MAX_PROXY_TRIANGLES: usize = 50_000;
const MAX_PROXY_EDGE_POINTS: usize = 50_000;
const DEFAULT_TOLERANCE: f64 = 1e-7;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerRequest {
    Box { min: [f64; 3], max: [f64; 3], tolerance: Option<f64> },
    Cylinder { radius: f64, height: [f64; 3], tolerance: Option<f64> },
    Sphere { radius: f64, tolerance: Option<f64> },
    Boolean { operation: BooleanOp, left_brep: String, right_brep: String, tolerance: Option<f64> },
    Inspect { brep: String, tolerance: Option<f64> },
    Tessellate { brep: String, linear_deflection: f64, angular_deflection: f64, tolerance: Option<f64> },
    ToStep { breps: Vec<String>, tolerance: Option<f64> },
    FromStep { step: String, tolerance: Option<f64> },
}

#[derive(Debug, Serialize)]
pub struct WorkerSolid {
    /// Binary OpenCascade BRep encoding; NEVER a display mesh.
    pub brep: String,
    pub volume: f64,
    pub faces: usize,
    pub edges: usize,
}

/// Derived, lossy **viewport** mesh. Triangle indices and transient native
/// OCCT face IDs are not persistent BRep subobject references. Source BRep data
/// stays untouched in the request/geometry store.
#[derive(Debug, Serialize)]
pub struct WorkerMesh {
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub indices: Vec<u32>,
    pub face_ids_session_hex: Vec<String>,
    pub edge_chains: Vec<Vec<[f64; 3]>>,
    pub exact: bool,
}

#[derive(Debug, Serialize)]
pub struct WorkerResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solids: Option<Vec<WorkerSolid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh: Option<WorkerMesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WorkerResponse {
    fn shapes(solids: Vec<WorkerSolid>) -> Self {
        Self { ok: true, solids: Some(solids), step: None, mesh: None, error: None }
    }

    fn step(encoded: String) -> Self {
        Self { ok: true, solids: None, step: Some(encoded), mesh: None, error: None }
    }

    fn mesh(mesh: WorkerMesh) -> Self {
        Self { ok: true, solids: None, step: None, mesh: Some(mesh), error: None }
    }

    fn error(msg: String) -> Self {
        Self { ok: false, solids: None, step: None, mesh: None, error: Some(msg) }
    }
}

fn kernel(tolerance: Option<f64>) -> Result<CadrumBrepCandidate, String> {
    AbsoluteTolerance::new(tolerance.unwrap_or(DEFAULT_TOLERANCE))
        .map(CadrumBrepCandidate::new)
        .map_err(|e| e.to_string())
}

fn vector(values: [f64; 3]) -> DVec3 {
    DVec3::new(values[0], values[1], values[2])
}

fn decode(encoded: &str) -> Result<Vec<u8>, String> {
    // Reject before allocating a potentially huge binary payload.
    if encoded.len() > (MAX_BINARY_BYTES + 2) * 4 / 3 + 4 {
        return Err("encoded exact geometry exceeds worker input budget".into());
    }
    let bytes = STANDARD.decode(encoded).map_err(|e| format!("invalid Base64 encoding: {e}"))?;
    if bytes.is_empty() || bytes.len() > MAX_BINARY_BYTES {
        return Err("exact geometry payload is empty or exceeds worker input budget".into());
    }
    Ok(bytes)
}

fn exact_one(backend: &CadrumBrepCandidate, encoded: &str) -> Result<BrepSolid, String> {
    let data = decode(encoded)?;
    let mut shapes = backend.read_native_brep(&data).map_err(|e| e.to_string())?;
    if shapes.len() != 1 {
        return Err("operation requires exactly one closed solid, not a compound or empty BRep".into());
    }
    shapes.pop().ok_or_else(|| "missing BRep solid".to_string())
}

fn encode_solids(backend: &CadrumBrepCandidate, solids: Vec<BrepSolid>) -> Result<WorkerResponse, String> {
    if solids.len() > MAX_SOLIDS {
        return Err("result has more solids than worker budget".into());
    }
    let mut result = Vec::with_capacity(solids.len());
    for solid in solids {
        let s = solid.statistics();
        if !s.volume.is_finite() || s.volume < 0.0 || s.faces == 0 || s.edges == 0 {
            return Err("native kernel returned invalid or incomplete solid statistics".into());
        }
        let data = backend.write_native_brep(&[solid]).map_err(|e| e.to_string())?;
        if data.len() > MAX_BINARY_BYTES {
            return Err("result BRep exceeds worker budget".into());
        }
        result.push(WorkerSolid { brep: STANDARD.encode(data), volume: s.volume, faces: s.faces, edges: s.edges });
    }
    Ok(WorkerResponse::shapes(result))
}

fn display_proxy(backend: &CadrumBrepCandidate, shape: &BrepSolid, linear: f64, angular: f64) -> Result<WorkerResponse, String> {
    let mesh = backend.display_mesh(shape, linear, angular).map_err(|e| e.to_string())?;
    if mesh.vertices.is_empty() || mesh.vertices.len() > MAX_PROXY_VERTICES
        || mesh.normals.len() != mesh.vertices.len()
        || mesh.indices.is_empty() || mesh.indices.len() % 3 != 0
        || mesh.indices.len() / 3 > MAX_PROXY_TRIANGLES
        || mesh.face_ids.len() != mesh.indices.len() / 3
        || mesh.edges.len() > MAX_PROXY_EDGE_POINTS
    {
        return Err("display tessellation is empty, invalid or exceeds triangle/vertex budget".into());
    }
    let mut vertices = Vec::with_capacity(mesh.vertices.len());
    let mut normals = Vec::with_capacity(mesh.normals.len());
    for (v, n) in mesh.vertices.iter().zip(&mesh.normals) {
        if !v.is_finite() || !n.is_finite() || (n.length() - 1.0).abs() > 1e-3 {
            return Err("display tessellation has nonfinite or nonunit surface normals".into());
        }
        vertices.push(v.to_array());
        normals.push(n.to_array());
    }
    let mut indices = Vec::with_capacity(mesh.indices.len());
    for &index in &mesh.indices {
        if index >= vertices.len() {
            return Err("display tessellation contains out-of-range index".into());
        }
        indices.push(u32::try_from(index).map_err(|_| "display index overflow")?);
    }
    let mut edge_chains = Vec::new();
    let mut chain = Vec::new();
    for point in &mesh.edges {
        if point.x.is_nan() {
            if !chain.is_empty() {
                edge_chains.push(std::mem::take(&mut chain));
            }
        } else {
            if !point.is_finite() {
                return Err("display edge has invalid coordinate".into());
            }
            chain.push(point.to_array());
        }
    }
    if !chain.is_empty() {
        edge_chains.push(chain);
    }
    Ok(WorkerResponse::mesh(WorkerMesh {
        vertices,
        normals,
        indices,
        face_ids_session_hex: mesh.face_ids.into_iter().map(|id| format!("{id:016x}")).collect(),
        edge_chains,
        exact: false,
    }))
}

/// Execute a single request. Transport errors never escape as partial solids.
/// Production applications should spawn this executable per batch (or restart
/// on crash), enforce process timeouts and never trust the worker as a sandbox.
pub fn execute(request: WorkerRequest) -> Result<WorkerResponse, String> {
    match request {
        WorkerRequest::Box { min, max, tolerance } => {
            let backend = kernel(tolerance)?;
            let solid = backend.box_from_corners(vector(min), vector(max)).map_err(|e| e.to_string())?;
            encode_solids(&backend, vec![solid])
        }
        WorkerRequest::Cylinder { radius, height, tolerance } => {
            let backend = kernel(tolerance)?;
            let solid = backend.cylinder(radius, vector(height)).map_err(|e| e.to_string())?;
            encode_solids(&backend, vec![solid])
        }
        WorkerRequest::Sphere { radius, tolerance } => {
            let backend = kernel(tolerance)?;
            let solid = backend.sphere(radius).map_err(|e| e.to_string())?;
            encode_solids(&backend, vec![solid])
        }
        WorkerRequest::Boolean { operation, left_brep, right_brep, tolerance } => {
            let backend = kernel(tolerance)?;
            // Byte-identical OpenCascade shape serializations are provably the
            // same operand, even when read into two different Rust allocations.
            // Handle idempotent booleans BEFORE native CellsBuilder, which is
            // known to fail when operands overlap exactly. This never treats
            // same *volume* or same bounds as proof of identical topology.
            if left_brep == right_brep {
                let source = exact_one(&backend, &left_brep)?;
                return match operation {
                    BooleanOp::Union | BooleanOp::Intersection => encode_solids(&backend, vec![source]),
                    BooleanOp::Difference => Ok(WorkerResponse::shapes(Vec::new())),
                };
            }
            let left = exact_one(&backend, &left_brep)?;
            let right = exact_one(&backend, &right_brep)?;
            let op = match operation {
                BooleanOp::Union => ExactBoolean::Union,
                BooleanOp::Difference => ExactBoolean::Difference,
                BooleanOp::Intersection => ExactBoolean::Intersection,
            };
            let solids = backend.boolean(op, &left, &right).map_err(|e| e.to_string())?;
            encode_solids(&backend, solids)
        }
        WorkerRequest::Inspect { brep, tolerance } => {
            let backend = kernel(tolerance)?;
            let solid = exact_one(&backend, &brep)?;
            encode_solids(&backend, vec![solid])
        }
        WorkerRequest::Tessellate { brep, linear_deflection, angular_deflection, tolerance } => {
            let backend = kernel(tolerance)?;
            let solid = exact_one(&backend, &brep)?;
            display_proxy(&backend, &solid, linear_deflection, angular_deflection)
        }
        WorkerRequest::ToStep { breps, tolerance } => {
            let backend = kernel(tolerance)?;
            if breps.is_empty() || breps.len() > MAX_SOLIDS {
                return Err("STEP export requires 1..64 exact solids".into());
            }
            let mut solids = Vec::with_capacity(breps.len());
            for encoded in breps {
                solids.push(exact_one(&backend, &encoded)?);
            }
            let data = backend.write_step(&solids).map_err(|e| e.to_string())?;
            if data.len() > MAX_BINARY_BYTES {
                return Err("STEP export exceeds worker budget".into());
            }
            Ok(WorkerResponse::step(STANDARD.encode(data)))
        }
        WorkerRequest::FromStep { step, tolerance } => {
            let backend = kernel(tolerance)?;
            let data = decode(&step)?;
            let solids = backend.read_step(&data).map_err(|e| e.to_string())?;
            if solids.is_empty() {
                return Err("STEP input contains no solid".into());
            }
            encode_solids(&backend, solids)
        }
    }
}

/// Pure JSON entry point is used by tests and by the binary process. Nonfatal
/// input errors return JSON ok:false without leaking partial native geometry.
pub fn execute_json(input: &[u8]) -> WorkerResponse {
    if input.len() as u64 > MAX_REQUEST_BYTES {
        return WorkerResponse::error("request exceeds exact BRep worker budget".into());
    }
    let request = match serde_json::from_slice::<WorkerRequest>(input) {
        Ok(value) => value,
        Err(error) => return WorkerResponse::error(format!("invalid request: {error}")),
    };
    execute(request).unwrap_or_else(WorkerResponse::error)
}

/// Read at most one bounded JSON request then write exactly one JSON result.
/// Critical C++ exceptions/process termination cannot be caught by Rust; the
/// parent CAD process can report/restart a failed worker without data mutation.
pub fn serve_once() -> io::Result<()> {
    let mut input = Vec::new();
    let mut stream = io::stdin().lock().take(MAX_REQUEST_BYTES + 1);
    stream.read_to_end(&mut input)?;
    let response = execute_json(&input);
    let output = serde_json::to_vec(&response).unwrap_or_else(|_| br#"{"ok":false,"error":"response encoding failed"}"#.to_vec());
    let output = if output.len() <= MAX_RESPONSE_BYTES {
        output
    } else {
        br#"{"ok":false,"error":"response exceeds worker budget"}"#.to_vec()
    };
    let mut out = io::stdout().lock();
    out.write_all(&output)?;
    out.write_all(b"\n")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn call(data: Value) -> Value {
        serde_json::to_value(execute_json(&serde_json::to_vec(&data).unwrap())).unwrap()
    }
    fn one_brep(data: Value) -> String {
        let result = call(data);
        assert_eq!(result["ok"], true, "{result}");
        let shapes = result["solids"].as_array().unwrap();
        assert_eq!(shapes.len(), 1);
        shapes[0]["brep"].as_str().unwrap().to_owned()
    }

    #[test]
    fn create_exact_shapes_then_roundtrip_in_worker_without_tessellation() {
        let box_brep = one_brep(json!({"op":"box","min":[0.0,0.0,0.0],"max":[2.0,3.0,4.0]}));
        let result = call(json!({"op":"inspect","brep":box_brep}));
        assert_eq!(result["ok"], true);
        assert_eq!(result["solids"][0]["faces"], 6);
        assert_eq!(result["solids"][0]["edges"], 12);
        assert!((result["solids"][0]["volume"].as_f64().unwrap()-24.0).abs()<1e-8);
        let sphere = one_brep(json!({"op":"sphere","radius":2.0}));
        assert!(call(json!({"op":"inspect","brep":sphere}))["solids"][0]["volume"].as_f64().unwrap()>33.0);
        let drill = one_brep(json!({"op":"cylinder","radius":1.0,"height":[0.0,0.0,5.0]}));
        assert!(call(json!({"op":"inspect","brep":drill}))["solids"][0]["faces"].as_u64().unwrap()>=3);
    }

    #[test]
    fn overlapped_boolean_and_byte_identical_idempotence() {
        let a = one_brep(json!({"op":"box","min":[0.0,0.0,0.0],"max":[2.0,2.0,2.0]}));
        let b = one_brep(json!({"op":"box","min":[1.0,1.0,1.0],"max":[3.0,3.0,3.0]}));
        let inter = call(json!({"op":"boolean","operation":"intersection","left_brep":a,"right_brep":b}));
        assert_eq!(inter["ok"], true, "{inter}");
        assert!((inter["solids"][0]["volume"].as_f64().unwrap()-1.0).abs()<1e-8);
        let fused = call(json!({"op":"boolean","operation":"union","left_brep":a,"right_brep":b}));
        assert_eq!(fused["ok"], true, "{fused}");
        assert!((fused["solids"][0]["volume"].as_f64().unwrap()-15.0).abs()<1e-8);
        let same = call(json!({"op":"boolean","operation":"union","left_brep":a,"right_brep":a}));
        assert_eq!(same["ok"], true, "{same}");
        assert!((same["solids"][0]["volume"].as_f64().unwrap()-8.0).abs()<1e-8);
        let empty = call(json!({"op":"boolean","operation":"difference","left_brep":a,"right_brep":a}));
        assert_eq!(empty["ok"], true, "{empty}");
        assert_eq!(empty["solids"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn cube_shaded_display_proxy_preserves_exact_solid_and_edges() {
        let box_brep = one_brep(json!({"op":"box","min":[0.0,0.0,0.0],"max":[2.0,3.0,4.0]}));
        let response = call(json!({"op":"tessellate","brep":box_brep,"linear_deflection":0.05,"angular_deflection":0.3}));
        assert_eq!(response["ok"], true, "{response}");
        let mesh = &response["mesh"];
        assert_eq!(mesh["exact"], false);
        assert!(mesh["vertices"].as_array().unwrap().len() >= 8);
        assert_eq!(mesh["vertices"].as_array().unwrap().len(), mesh["normals"].as_array().unwrap().len());
        assert!(mesh["indices"].as_array().unwrap().len() >= 36);
        assert_eq!(mesh["indices"].as_array().unwrap().len()/3, mesh["face_ids_session_hex"].as_array().unwrap().len());
        assert!(!mesh["edge_chains"].as_array().unwrap().is_empty());
        let exact = call(json!({"op":"inspect","brep":box_brep}));
        assert_eq!(exact["solids"][0]["faces"], 6);
        assert!((exact["solids"][0]["volume"].as_f64().unwrap()-24.0).abs() < 1e-8);
        let refused = call(json!({"op":"tessellate","brep":box_brep,"linear_deflection":0.0,"angular_deflection":0.1}));
        assert_eq!(refused["ok"], false);
    }

    #[test]
    fn sphere_proxy_normals_follow_brep_analytic_surface() {
        let sphere = one_brep(json!({"op":"sphere","radius":3.0}));
        let result = call(json!({"op":"tessellate","brep":sphere,"linear_deflection":0.2,"angular_deflection":0.4}));
        assert_eq!(result["ok"], true, "{result}");
        let mesh = &result["mesh"];
        let vertices = mesh["vertices"].as_array().unwrap();
        let normals = mesh["normals"].as_array().unwrap();
        assert!(vertices.len() > 20);
        for (v, n) in vertices.iter().zip(normals) {
            let a = v.as_array().unwrap();
            let b = n.as_array().unwrap();
            let xyz: Vec<f64> = a.iter().map(|x| x.as_f64().unwrap()).collect();
            let nn: Vec<f64> = b.iter().map(|x| x.as_f64().unwrap()).collect();
            let len = (xyz.iter().map(|x| x*x).sum::<f64>()).sqrt();
            let radial_dot = xyz.iter().zip(&nn).map(|(x,y)| x*y).sum::<f64>()/len;
            assert!(radial_dot > 1.0-1e-5, "surface normal not radial");
        }
    }

    #[test]
    fn worker_step_roundtrip_and_errors_are_bounded() {
        let brep = one_brep(json!({"op":"box","min":[-2.0,1.0,0.0],"max":[2.0,3.0,5.0]}));
        let exported = call(json!({"op":"to_step","breps":[brep]}));
        assert_eq!(exported["ok"], true, "{exported}");
        let imported = call(json!({"op":"from_step","step":exported["step"]}));
        assert_eq!(imported["ok"], true, "{imported}");
        assert_eq!(imported["solids"].as_array().unwrap().len(), 1);
        assert!((imported["solids"][0]["volume"].as_f64().unwrap()-40.0).abs()<1e-7);
        assert_eq!(call(json!({"op":"box","min":[2.0,0.0,0.0],"max":[0.0,1.0,1.0]}))["ok"],false);
        assert_eq!(call(json!({"op":"sphere","radius":-1.0}))["ok"],false);
        assert_eq!(call(json!({"op":"inspect","brep":"!!!"}))["ok"],false);
        assert_eq!(call(json!({"op":"boolean","operation":"illegal","left_brep":"x","right_brep":"x"}))["ok"],false);
        assert!(!execute_json(&vec![b'x'; MAX_REQUEST_BYTES as usize + 1]).ok);
    }
}
