//! Conservative patching of planar convex INNER boundaries of polygon meshes.
use crate::{KernelError,PolygonFace,PolygonMesh,Result,polygon_mesh_boundary_loops,polygon_mesh_topology,polygon_mesh_validate};
use cadcraft_geom::Vec3;
use serde::{Serialize,Deserialize};

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct PolygonFillResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    pub boundary_vertices: Vec<u32>,
    pub new_face_indices: Vec<u32>,
}
fn norm(v:Vec3)->f64 { v.x.hypot(v.y).hypot(v.z) }

/// Triangulates a selected planar convex inner loop. Outer boundaries are
/// rejected by comparing loop winding with adjacent polygon orientation.
/// Does not support nonplanar holes, concave holes or intersection repair.
pub fn polygon_mesh_fill_hole(
    mesh:&PolygonMesh, revision:u64, picked_revision:u64, loop_index:u32,
)->Result<PolygonFillResult> {
    if revision!=picked_revision {return Err(KernelError::Conflict{expected:picked_revision,actual:revision});}
    polygon_mesh_validate(mesh)?;
    let report=polygon_mesh_boundary_loops(mesh)?;
    if !report.unresolved_edges.is_empty() || !report.non_manifold_edges.is_empty()
        || !report.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("ambiguous source topology"));
    }
    let loop_data=report.closed_loops.get(loop_index as usize)
        .ok_or(KernelError::Invalid("boundary loop index"))?;
    let ids=&loop_data.vertices;
    if ids.len()<3 || ids.len()>256 {return Err(KernelError::Invalid("unsupported boundary size"));}
    if mesh.faces.len().checked_add(ids.len()-2).is_none_or(|n|n>1_000_000) {
        return Err(KernelError::Budget);
    }
    let next_revision=revision.checked_add(1).ok_or(KernelError::Budget)?;
    let origin=mesh.vertices[ids[0] as usize];
    let mut extent=0.0_f64;
    let mut area=Vec3::ZERO;
    for i in 0..ids.len() {
        let a=mesh.vertices[ids[i] as usize]-origin;
        let b=mesh.vertices[ids[(i+1)%ids.len()] as usize]-origin;
        extent=extent.max(norm(a));
        area=area+a.cross(b);
    }
    if !extent.is_finite() || extent<=f64::EPSILON
        || norm(area)<=1e-12*extent*extent {
        return Err(KernelError::Invalid("degenerate boundary"));
    }
    let normal=area*(1.0/norm(area));
    for &id in ids {
        if (mesh.vertices[id as usize]-origin).dot(normal).abs()>1e-7*extent {
            return Err(KernelError::Invalid("nonplanar boundary"));
        }
    }
    for i in 0..ids.len() {
        let a=mesh.vertices[ids[i] as usize];
        let b=mesh.vertices[ids[(i+1)%ids.len()] as usize];
        let c=mesh.vertices[ids[(i+2)%ids.len()] as usize];
        if (b-a).cross(c-b).dot(normal)<=1e-10*extent*extent {
            return Err(KernelError::Invalid("nonconvex hole"));
        }
    }
    let topo=polygon_mesh_topology(mesh)?;
    for &half_id in &loop_data.halfedges {
        let h=&topo.halfedges[half_id as usize];
        let face=mesh.faces[h.face as usize].indices();
        let a=mesh.vertices[face[0] as usize];
        let b=mesh.vertices[face[1] as usize];
        let c=mesh.vertices[face[2] as usize];
        let facing=(b-a).cross(c-a);
        if norm(facing)<=f64::EPSILON || facing.dot(normal)/norm(facing)>=-0.9 {
            return Err(KernelError::Invalid("exterior or nonplanar surrounding face"));
        }
    }
    let mut output=mesh.clone();
    let mut new_face_indices=Vec::new();
    for i in 1..ids.len()-1 {
        let index=u32::try_from(output.faces.len()).map_err(|_|KernelError::Budget)?;
        new_face_indices.push(index);
        output.faces.push(PolygonFace::Triangle([ids[0],ids[i+1],ids[i]]));
    }
    polygon_mesh_validate(&output)?;
    let after=polygon_mesh_topology(&output)?;
    if !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("invalid patch topology"));
    }
    Ok(PolygonFillResult{
        mesh:output,revision:next_revision,boundary_vertices:ids.clone(),new_face_indices
    })
}
