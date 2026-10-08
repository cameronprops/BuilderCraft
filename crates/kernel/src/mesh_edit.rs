//! Manual triangle creation from a vertex selection.
//! UI pick tools can provide the indices; this operation never changes the input.
use crate::{KernelError, Result, TriangleMesh, mesh_face_analysis, mesh_duplicate_faces, mesh_edge_report};

/// Add one oriented triangle referencing existing mesh vertices.
/// This conservative first version rejects repeated/collinear vertices,
/// duplicate faces, and a resulting non-manifold or mismatched-winding edge.
/// Existing topology errors are not automatically repaired.
/// A copy is returned for transaction/undo integration.
pub fn mesh_add_triangle(mesh: &TriangleMesh, vertices: [u32; 3]) -> Result<TriangleMesh> {
    if mesh.vertices.len() > 1_000_000 || mesh.triangles.len() >= 1_000_000 {
        return Err(KernelError::Budget);
    }
    if vertices.iter().any(|&index| index as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("selected triangle vertex index"));
    }
    if vertices[0] == vertices[1] || vertices[1] == vertices[2]
        || vertices[0] == vertices[2] {
        return Err(KernelError::Invalid("triangle repeats selected vertex"));
    }
    let selected = TriangleMesh {
        vertices: mesh.vertices.clone(),
        triangles: vec![vertices],
    };
    let analysis = mesh_face_analysis(&selected, 0.0)?;
    if analysis.first().is_none_or(|face| face.degenerate) {
        return Err(KernelError::Invalid("degenerate selected triangle"));
    }
    // Reuse existing validators for input indices, duplicate faces and topology.
    let _ = mesh_duplicate_faces(mesh)?;
    let mut result = mesh.clone();
    result.triangles.push(vertices);
    if !mesh_duplicate_faces(&result)?.duplicates.is_empty() {
        // Reject newly introduced duplicates; pre-existing duplicates need repair first.
        return Err(KernelError::Invalid("duplicate triangle"));
    }
    let edges = mesh_edge_report(&result)?;
    if !edges.non_manifold_edges.is_empty() {
        return Err(KernelError::Invalid("non-manifold edge after triangle creation"));
    }
    if !edges.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("inconsistent edge winding after triangle creation"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn fixture() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            triangles: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn builds_second_triangle_from_three_selected_vertices() {
        let source = fixture();
        let original = source.clone();
        let added = mesh_add_triangle(&source, [0, 2, 3]);
        assert!(added.is_ok_and(|mesh| mesh.triangles == vec![[0,1,2], [0,2,3]]));
        assert_eq!(source, original);
    }

    #[test]
    fn rejects_duplicates_and_reverse_duplicates() {
        let source = fixture();
        assert!(mesh_add_triangle(&source, [0,1,2]).is_err());
        assert!(mesh_add_triangle(&source, [2,1,0]).is_err());
    }

    #[test]
    fn rejects_repeated_collinear_and_out_of_bounds_selection() {
        let mut source = fixture();
        assert!(mesh_add_triangle(&source, [0,0,1]).is_err());
        assert!(mesh_add_triangle(&source, [0,1,88]).is_err());
        source.vertices[3] = Vec3::new(0.5, 0.0, 0.0);
        assert!(mesh_add_triangle(&source, [0,1,3]).is_err());
    }

    #[test]
    fn rejects_wrong_shared_edge_orientation() {
        assert!(mesh_add_triangle(&fixture(), [2,0,3]).is_err());
    }

    #[test]
    fn rejects_nonmanifold_third_face() {
        let mut source = fixture();
        source.vertices.push(Vec3::new(0.5, -1.0, 0.0));
        source.triangles.push([1, 0, 4]);
        source.vertices.push(Vec3::new(0.5, 0.0, 1.0));
        assert!(mesh_add_triangle(&source, [0, 1, 5]).is_err());
    }
}
