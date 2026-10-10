//! Transaction-friendly polygon editing shared by the scene and future UI/API adapters.
//! Edits are applied to a new mesh; the scene owns revision, undo and budgets.

use crate::{
    KernelError, PolygonMesh, Result, polygon_mesh_add_triangle_from_edge, polygon_mesh_split_edge, polygon_mesh_delete_faces, polygon_mesh_fill_hole, pushpull_mesh_face,
};
use serde::{Deserialize, Serialize};

/// User-facing picks must carry the revision at which they were made.
/// The scene revision is the authoritative revision; stale picks are rejected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PolygonSceneEdit {
    DeleteFaces { selected_revision: u64, selected_faces: Vec<u32> },
    AddTriangleFromEdge { selected_revision: u64, edge_vertices: [u32; 2], point_vertex: u32 },
    FillPlanarHole { selected_revision: u64, loop_index: u32 },
    SplitEdge { selected_revision: u64, edge_vertices: [u32; 2], fraction: f64 },
    PushPullFace { selected_revision: u64, face_index: u32, distance: f64 },
}

/// Pure edit adapter: no document mutation and no hidden triangulation.
/// Callers must publish the result using a budgeted scene transaction.
pub fn apply_polygon_scene_edit(source: &PolygonMesh, current_revision: u64, edit: &PolygonSceneEdit) -> Result<PolygonMesh> {
    match edit {
        PolygonSceneEdit::DeleteFaces { selected_revision, selected_faces } => {
            polygon_mesh_delete_faces(source, current_revision, *selected_revision, selected_faces).map(|result| result.mesh)
        }
        PolygonSceneEdit::AddTriangleFromEdge { selected_revision, edge_vertices, point_vertex } => {
            polygon_mesh_add_triangle_from_edge(source, current_revision, *selected_revision, *edge_vertices, *point_vertex).map(|result| result.mesh)
        }
        PolygonSceneEdit::FillPlanarHole { selected_revision, loop_index } => {
            polygon_mesh_fill_hole(source, current_revision, *selected_revision, *loop_index).map(|result| result.mesh)
        }
        PolygonSceneEdit::SplitEdge { selected_revision, edge_vertices, fraction } => {
            polygon_mesh_split_edge(source, current_revision, *selected_revision, *edge_vertices, *fraction).map(|result| result.mesh)
        }
        PolygonSceneEdit::PushPullFace { selected_revision, face_index, distance } => {
            if *selected_revision != current_revision {
                return Err(KernelError::Conflict { expected: *selected_revision, actual: current_revision });
            }
            pushpull_mesh_face(source, *face_index as usize, *distance)
        }
    }
}
