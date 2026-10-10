//! Local full-snapshot visualization feed and portable GLB, independent of engine hosts.
use crate::{IoError, Result};
use buildercraft_kernel::{Axes, Cancellation, Frame, GeometryBudget, GeometryData, Id, LengthUnit, TessellationLimits, TessellationOptions};
use cadcraft_doc::Drawing;
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

pub const MAX_PACKAGE_BYTES: usize = 32 * 1024 * 1024;
fn bad(message: impl Into<String>) -> IoError {
    IoError::Format(message.into())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct VisualizationObject {
    pub id: Id,
    pub name: String,
    pub layer: String,
    pub parent: Option<Id>,
    pub visible: bool,
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub polyline: bool,
    pub geometry_key: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub protocol_version: u32,
    pub project_id: Id,
    /// Strings prevent JSON number precision loss across runtimes.
    pub source_revision: String,
    pub sequence: String,
    pub frame: Frame,
    pub glb_file: String,
    pub production: buildercraft_kernel::ProductionModel,
    pub objects: Vec<VisualizationObject>,
    pub diagnostics: Vec<String>,
}
fn key(points: &[[f64; 3]], triangles: &[[u32; 3]], polyline: bool) -> String {
    // Deterministic non-security fingerprint; never used to authenticate a payload.
    let mut h = 0xcbf29ce484222325u64;
    for b in [u8::from(polyline)]
        .into_iter()
        .chain(points.iter().flatten().flat_map(|n| n.to_le_bytes()))
        .chain(triangles.iter().flatten().flat_map(|n| n.to_le_bytes()))
    {
        h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
pub fn snapshot(drawing: &Drawing, project_id: Id, revision: u64, options: TessellationOptions, cancel: &Cancellation) -> Result<Snapshot> {
    cancel.check().map_err(|e| bad(e.to_string()))?;
    if drawing
        .organization
        .nodes
        .len()
        .checked_add(drawing.geometry3d.len())
        .and_then(|n| n.checked_add(drawing.mesh3d.len()))
        .is_none_or(|n| n > 256)
    {
        return Err(bad("visualization object limit (256)"));
    }
    let manifest = cadcraft_doc::kernel::manifest(drawing, project_id, revision, 16 * 1024 * 1024).map_err(|e| bad(e.to_string()))?;
    let target = Frame { unit: LengthUnit::Metre, axes: Axes::LeftHandedZUp };
    let mut objects: Vec<VisualizationObject> = manifest
        .objects
        .into_iter()
        .map(|o| VisualizationObject {
            id: o.id,
            name: o.name,
            layer: o.layer,
            parent: o.parent,
            visible: o.visible,
            positions: Vec::new(),
            triangles: Vec::new(),
            polyline: false,
            geometry_key: String::new(),
        })
        .collect();
    let budget = GeometryBudget::new(8 * 1024 * 1024, 65536);
    let mut total_points = 0usize;
    let mut total_faces = 0usize;
    for object in &drawing.geometry3d {
        cancel.check().map_err(|e| bad(e.to_string()))?;
        let lease = buildercraft_kernel::tessellate(
            &object.shape,
            options,
            TessellationLimits { max_work_units: 2_000_000, ..Default::default() },
            &budget,
            cancel,
        )
        .map_err(|e| bad(e.to_string()))?;
        let id = Id::new(u128::from(object.id) + 1).map_err(|e| bad(e.to_string()))?;
        let output = objects.iter_mut().find(|o| o.id == id).ok_or_else(|| bad("missing scene identity"))?;
        let (points, faces, polyline) = match lease.data() {
            GeometryData::Polyline(p) => (p.as_slice(), &[][..], true),
            GeometryData::Mesh(m) => (m.vertices.as_slice(), m.triangles.as_slice(), false),
            _ => return Err(bad("unsupported preview representation")),
        };
        total_points = total_points.checked_add(points.len()).ok_or_else(|| bad("point overflow"))?;
        total_faces = total_faces.checked_add(faces.len()).ok_or_else(|| bad("triangle overflow"))?;
        if total_points > 50000 || total_faces > 100000 {
            return Err(bad("aggregate visualization sample limit"));
        }
        output.polyline = polyline;
        output.visible &= drawing.layer(&object.layer).is_none_or(|l| l.visible());
        for p in points {
            let p = manifest.frame.convert_point(target, *p).map_err(|e| bad(e.to_string()))?;
            if [p.x, p.y, p.z].iter().any(|n| n.abs() > 1e7) {
                return Err(bad("visualization needs a local origin for large coordinates"));
            }
            output.positions.push([p.x, p.y, p.z]);
        }
        // Source RH to target LH conversion reflects an axis: reverse winding.
        output.triangles = faces.iter().map(|f| [f[0], f[2], f[1]]).collect();
        output.geometry_key = key(&output.positions, &output.triangles, polyline);
    }
    // Native polygon meshes retain quad identity in the CAD document.
    // Triangulation is strictly a derived visualization representation.
    for object in &drawing.mesh3d {
        cancel.check().map_err(|e| bad(e.to_string()))?;
        let id = Id::new(u128::from(object.id) + 1).map_err(|e| bad(e.to_string()))?;
        let output = objects.iter_mut().find(|o| o.id == id).ok_or_else(|| bad("missing polygon scene identity"))?;
        output.visible &= drawing.layer(&object.layer).is_none_or(|l| l.visible());
        if object.mesh.faces.is_empty() {
            continue;
        }
        let triangulated = buildercraft_kernel::polygon_mesh_triangulate(&object.mesh).map_err(|e| bad(e.to_string()))?;
        let mesh = triangulated.mesh;
        total_points = total_points.checked_add(mesh.vertices.len()).ok_or_else(|| bad("point overflow"))?;
        total_faces = total_faces.checked_add(mesh.triangles.len()).ok_or_else(|| bad("triangle overflow"))?;
        if total_points > 50000 || total_faces > 100000 {
            return Err(bad("aggregate visualization sample limit"));
        }
        for point in &mesh.vertices {
            cancel.check().map_err(|e| bad(e.to_string()))?;
            let p = manifest.frame.convert_point(target, *point).map_err(|e| bad(e.to_string()))?;
            if [p.x, p.y, p.z].iter().any(|n| n.abs() > 1e7) {
                return Err(bad("visualization needs a local origin for large coordinates"));
            }
            output.positions.push([p.x, p.y, p.z]);
        }
        output.triangles = mesh.triangles.iter().map(|f| [f[0], f[2], f[1]]).collect();
        output.geometry_key = key(&output.positions, &output.triangles, false);
    }
    drawing.production.validate().map_err(|e| bad(e.to_string()))?;
    let mut diagnostics = if drawing.entity_count() > 0 { vec!["Inherited 2D drafting is excluded from this 3D feed".into()] } else { Vec::new() };
    for b in &drawing.production.bindings {
        if !objects.iter().any(|o| o.id == b.object) {
            diagnostics.push(format!("Production binding object {} is outside the exported 3D scene", String::from(b.object)));
        }
    }
    Ok(Snapshot {
        production: drawing.production.clone(),
        protocol_version: 1,
        project_id,
        source_revision: revision.to_string(),
        sequence: "0".into(),
        frame: target,
        glb_file: String::new(),
        objects,
        diagnostics,
    })
}

fn validate_snapshot(snapshot: &Snapshot) -> Result<()> {
    if snapshot.protocol_version != 1 || snapshot.objects.len() > 256 {
        return Err(bad("snapshot protocol/count"));
    }
    snapshot.production.validate().map_err(|e| bad(e.to_string()))?;
    let objects: BTreeMap<_, _> = snapshot.objects.iter().map(|o| (o.id, o)).collect();
    if objects.len() != snapshot.objects.len() {
        return Err(bad("duplicate scene identity"));
    }
    let mut points = 0usize;
    let mut faces = 0usize;
    for o in &snapshot.objects {
        points = points.checked_add(o.positions.len()).ok_or_else(|| bad("point overflow"))?;
        faces = faces.checked_add(o.triangles.len()).ok_or_else(|| bad("triangle overflow"))?;
        if points > 50000
            || faces > 100000
            || o.name.len() > 256
            || o.layer.len() > 256
            || o.geometry_key.len() > 64
            || o.positions.iter().flatten().any(|n| !n.is_finite() || n.abs() > 1e7)
            || (o.polyline && !o.triangles.is_empty())
            || o.triangles.iter().flatten().any(|i| *i as usize >= o.positions.len())
        {
            return Err(bad("invalid snapshot geometry/metadata"));
        }
        let mut seen = std::collections::BTreeSet::from([o.id]);
        let mut parent = o.parent;
        while let Some(id) = parent {
            if !seen.insert(id) || seen.len() > 64 {
                return Err(bad("snapshot hierarchy"));
            }
            parent = objects.get(&id).ok_or_else(|| bad("missing snapshot parent"))?.parent;
        }
    }
    Ok(())
}

/// glTF RH Y-up metres; hierarchy nodes use absolute geometry and identity transforms.
pub fn glb(snapshot: &Snapshot) -> Result<Vec<u8>> {
    validate_snapshot(snapshot)?;
    let target = Frame { unit: LengthUnit::Metre, axes: Axes::RightHandedYUp };
    let mut bin = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let map: BTreeMap<Id, usize> = snapshot.objects.iter().enumerate().map(|(i, o)| (o.id, i)).collect();
    for o in &snapshot.objects {
        let mut node = json!({"name":o.name,"extras":{"buildercraft_id":o.id,"visible":o.visible,"layer":o.layer,"geometry_key":o.geometry_key}});
        let children: Vec<usize> = snapshot.objects.iter().enumerate().filter_map(|(i, c)| (c.parent == Some(o.id)).then_some(i)).collect();
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        if !o.positions.is_empty() && o.visible {
            let offset = bin.len();
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for p in &o.positions {
                let v = snapshot.frame.convert_point(target, Vec3::new(p[0], p[1], p[2])).map_err(|e| bad(e.to_string()))?;
                for (i, x) in [v.x, v.y, v.z].iter().enumerate() {
                    let n = *x as f32;
                    if !n.is_finite() {
                        return Err(bad("nonfinite glTF coordinate"));
                    }
                    if let Some(a) = min.get_mut(i) {
                        *a = a.min(n);
                    }
                    if let Some(a) = max.get_mut(i) {
                        *a = a.max(n);
                    }
                    bin.extend(n.to_le_bytes());
                }
            }
            let view = views.len();
            views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34962}));
            let pos = accessors.len();
            accessors.push(json!({"bufferView":view,"componentType":5126,"count":o.positions.len(),"type":"VEC3","min":min,"max":max}));
            let mut primitive = json!({"attributes":{"POSITION":pos},"mode":if o.polyline {3}else{4},"material":0});
            if !o.polyline {
                let offset = bin.len();
                for f in &o.triangles {
                    for i in [f[0], f[2], f[1]] {
                        if i as usize >= o.positions.len() {
                            return Err(bad("glTF index out of range"));
                        }
                        bin.extend(i.to_le_bytes());
                    }
                }
                let view = views.len();
                views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34963}));
                let index = accessors.len();
                accessors.push(json!({"bufferView":view,"componentType":5125,"count":o.triangles.len()*3,"type":"SCALAR"}));
                primitive["indices"] = json!(index);
            }
            node["mesh"] = json!(meshes.len());
            meshes.push(json!({"name":o.name,"primitives":[primitive]}));
        }
        nodes.push(node);
        if bin.len() > MAX_PACKAGE_BYTES {
            return Err(bad("GLB buffer limit"));
        }
    }
    let roots: Vec<usize> = snapshot.objects.iter().enumerate().filter_map(|(i, o)| o.parent.is_none().then_some(i)).collect();
    for o in &snapshot.objects {
        if o.parent.is_some_and(|p| !map.contains_key(&p)) {
            return Err(bad("missing GLB parent"));
        }
    }
    let mut doc = json!({"asset":{"version":"2.0","generator":"BuilderCraft native exporter"},"scene":0,"scenes":[{"nodes":roots}],"nodes":nodes,
        "extras":{"project_id":snapshot.project_id,"source_revision":snapshot.source_revision,"production":snapshot.production},
        "materials":[{"name":"Massing","doubleSided":true,"pbrMetallicRoughness":{"baseColorFactor":[0.65,0.68,0.72,1.0],"metallicFactor":0.0,"roughnessFactor":0.8}}]});
    if snapshot.objects.is_empty() {
        doc["nodes"] = json!([{"name":"BuilderCraft empty scene","extras":{"empty_scene":true}}]);
        doc["scenes"] = json!([{"nodes":[0]}]);
    }
    if !bin.is_empty() {
        doc["buffers"] = json!([{"byteLength":bin.len()}]);
        doc["bufferViews"] = json!(views);
        doc["accessors"] = json!(accessors);
        doc["meshes"] = json!(meshes);
    }
    let mut json = serde_json::to_vec(&doc).map_err(|e| bad(e.to_string()))?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12 + 8 + json.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    if total > MAX_PACKAGE_BYTES {
        return Err(bad("GLB package limit"));
    }
    let mut out = Vec::new();
    out.extend(0x46546c67u32.to_le_bytes());
    out.extend(2u32.to_le_bytes());
    out.extend(u32::try_from(total).map_err(|_| bad("GLB length"))?.to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(0x4e4f534au32.to_le_bytes());
    out.extend(json);
    if !bin.is_empty() {
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(0x004e4942u32.to_le_bytes());
        out.extend(bin);
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn publish(directory: &std::path::Path, snapshot: Snapshot) -> Result<u64> {
    publish_cancellable(directory, snapshot, &Cancellation::default())
}

/// Cancellation is checked before staging and immediately before committing the scene marker.
#[cfg(not(target_arch = "wasm32"))]
pub fn publish_cancellable(directory: &std::path::Path, mut snapshot: Snapshot, cancel: &Cancellation) -> Result<u64> {
    cancel.check().map_err(|e| bad(e.to_string()))?;
    use std::fs;
    fs::create_dir_all(directory).map_err(|e| bad(e.to_string()))?;
    let lock = directory.join("writer.lock");
    let file = fs::OpenOptions::new().write(true).create_new(true).open(&lock).map_err(|e| bad(format!("single-writer lock: {e}")))?;
    drop(file); // Keep the lock path, close the handle so Windows can remove it on exit.
    struct Lock(std::path::PathBuf);
    impl Drop for Lock {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _lock = Lock(lock);
    let current = directory.join("snapshot.json");
    let previous = if current.exists() {
        if fs::metadata(&current).map_err(|e| bad(e.to_string()))?.len() > MAX_PACKAGE_BYTES as u64 {
            return Err(bad("previous snapshot size"));
        }
        let bytes = fs::read(&current).map_err(|e| bad(e.to_string()))?;
        let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| bad(e.to_string()))?;
        if v["project_id"] != json!(snapshot.project_id) {
            return Err(bad("output directory belongs to a different project"));
        }
        v["sequence"].as_str().and_then(|s| s.parse::<u64>().ok()).ok_or_else(|| bad("invalid previous sequence"))?
    } else {
        0
    };
    let sequence = previous.checked_add(1).ok_or_else(|| bad("sequence overflow"))?;
    snapshot.sequence = sequence.to_string();
    snapshot.glb_file = format!("scene-{sequence}.glb");
    let glb = glb(&snapshot)?;
    let json = serde_json::to_vec(&snapshot).map_err(|e| bad(e.to_string()))?;
    if json.len() > MAX_PACKAGE_BYTES {
        return Err(bad("snapshot size limit"));
    }
    cancel.check().map_err(|e| bad(e.to_string()))?;
    let asset = directory.join(&snapshot.glb_file);
    fs::write(&asset, glb).map_err(|e| bad(e.to_string()))?;
    let temp = directory.join("snapshot.next");
    fs::write(&temp, json).map_err(|e| bad(e.to_string()))?;
    cancel.check().map_err(|e| bad(e.to_string()))?;
    fs::rename(&temp, &current).map_err(|e| bad(e.to_string()))?;
    if sequence > 2 {
        let _ = fs::remove_file(directory.join(format!("scene-{}.glb", sequence - 2)));
    }
    Ok(sequence)
}
