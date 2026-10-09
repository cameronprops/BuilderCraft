use crate::{Frame, GeometryBudget, GeometryData, GeometryLease, Id, KernelError, PolygonSceneEdit, Result, apply_polygon_scene_edit};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Acquire) { Err(KernelError::Cancelled) } else { Ok(()) }
    }
}
#[derive(Clone, Debug)]
pub struct SceneObject {
    pub id: Id,
    pub name: String,
    pub layer: String,
    pub parent: Option<Id>,
    pub visible: bool,
    pub geometry: Option<GeometryLease>,
}
#[derive(Clone, Debug)]
pub enum SceneCommand {
    Insert(SceneObject),
    Remove(Id),
    Rename(Id, String),
    SetGeometry(Id, GeometryLease),
    /// Transactional edit of an already-retained polygon mesh.
    EditPolygon(Id, PolygonSceneEdit),
}
#[derive(Clone, Debug)]
pub struct SceneSnapshot {
    project_id: Id,
    frame: Frame,
    objects: BTreeMap<Id, Arc<SceneObject>>,
    budget: Arc<GeometryBudget>,
}
#[derive(Debug)]
pub struct Scene {
    project_id: Id,
    frame: Frame,
    revision: u64,
    objects: BTreeMap<Id, Arc<SceneObject>>,
    budget: Arc<GeometryBudget>,
    max_objects: usize,
}
impl Scene {
    pub fn new(project_id: Id, frame: Frame, budget: Arc<GeometryBudget>, max_objects: usize) -> Self {
        Self { project_id, frame, budget, max_objects, revision: 0, objects: BTreeMap::new() }
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn project_id(&self) -> Id {
        self.project_id
    }
    pub fn object(&self, id: Id) -> Option<&SceneObject> {
        self.objects.get(&id).map(Arc::as_ref)
    }
    pub fn snapshot(&self) -> SceneSnapshot {
        SceneSnapshot { project_id: self.project_id, frame: self.frame, objects: self.objects.clone(), budget: self.budget.clone() }
    }
    fn next_revision(&self, expected: u64) -> Result<u64> {
        if self.revision != expected {
            return Err(KernelError::Conflict { expected, actual: self.revision });
        }
        self.revision.checked_add(1).ok_or(KernelError::Invalid("revision overflow"))
    }
    fn validate(&self, objects: &BTreeMap<Id, Arc<SceneObject>>, cancellation: &Cancellation) -> Result<()> {
        if objects.len() > self.max_objects {
            return Err(KernelError::Invalid("object limit"));
        }
        for object in objects.values() {
            cancellation.check()?;
            if object.name.trim().is_empty() || object.name.len() > 256 || object.layer.len() > 256 {
                return Err(KernelError::Invalid("metadata"));
            }
            if object.geometry.as_ref().is_some_and(|g| !g.belongs_to(&self.budget)) {
                return Err(KernelError::Invalid("foreign geometry budget"));
            }
            let mut seen = BTreeSet::from([object.id]);
            let mut parent = object.parent;
            while let Some(id) = parent {
                cancellation.check()?;
                if seen.len() >= 64 || !seen.insert(id) {
                    return Err(KernelError::Invalid("cyclic or deep hierarchy"));
                }
                parent = objects.get(&id).ok_or(KernelError::Invalid("missing parent"))?.parent;
            }
        }
        Ok(())
    }
    /// All commands publish together or leave the scene untouched.
    pub fn apply(&mut self, expected_revision: u64, commands: Vec<SceneCommand>, cancellation: &Cancellation) -> Result<u64> {
        cancellation.check()?;
        let revision = self.next_revision(expected_revision)?;
        if commands.is_empty() {
            return Ok(self.revision);
        }
        if commands.len() > self.max_objects.saturating_mul(4) {
            return Err(KernelError::Invalid("command limit"));
        }
        let mut staged = self.objects.clone();
        for command in commands {
            cancellation.check()?;
            match command {
                SceneCommand::Insert(object) => {
                    if staged.contains_key(&object.id) {
                        return Err(KernelError::Object);
                    }
                    if staged.len() >= self.max_objects {
                        return Err(KernelError::Invalid("object limit"));
                    }
                    staged.insert(object.id, Arc::new(object));
                }
                SceneCommand::Remove(id) => {
                    staged.remove(&id).ok_or(KernelError::Object)?;
                }
                SceneCommand::Rename(id, name) => {
                    Arc::make_mut(staged.get_mut(&id).ok_or(KernelError::Object)?).name = name;
                }
                SceneCommand::SetGeometry(id, geometry) => {
                    Arc::make_mut(staged.get_mut(&id).ok_or(KernelError::Object)?).geometry = Some(geometry);
                }
                SceneCommand::EditPolygon(id, edit) => {
                    let existing = staged.get(&id).ok_or(KernelError::Object)?;
                    let geometry = existing.geometry.as_ref().ok_or(KernelError::Invalid("object has no polygon mesh"))?;
                    let GeometryData::PolygonMesh(source) = geometry.data() else {
                        return Err(KernelError::Invalid("object geometry is not an editable polygon mesh"));
                    };
                    // Native mesh editing is synchronous. Bound work until
                    // cooperative cancellation is supported inside each operation.
                    if source.vertices.len() > 100_000 || source.faces.len() > 100_000 {
                        return Err(KernelError::Budget);
                    }
                    cancellation.check()?;
                    let edited = apply_polygon_scene_edit(source, self.revision, &edit)?;
                    cancellation.check()?;
                    let lease = self.budget.retain(GeometryData::PolygonMesh(edited))?;
                    cancellation.check()?;
                    Arc::make_mut(staged.get_mut(&id).ok_or(KernelError::Object)?).geometry = Some(lease);
                }
            }
        }
        self.validate(&staged, cancellation)?;
        cancellation.check()?;
        self.objects = staged;
        self.revision = revision;
        Ok(revision)
    }
    /// Undo/redo callers retain bounded snapshots; restores advance revision.
    pub fn restore(&mut self, expected_revision: u64, snapshot: &SceneSnapshot, cancellation: &Cancellation) -> Result<u64> {
        let revision = self.next_revision(expected_revision)?;
        cancellation.check()?;
        if snapshot.project_id != self.project_id || snapshot.frame != self.frame || !Arc::ptr_eq(&snapshot.budget, &self.budget) {
            return Err(KernelError::ForeignSnapshot);
        }
        self.validate(&snapshot.objects, cancellation)?;
        cancellation.check()?;
        self.objects = snapshot.objects.clone();
        self.revision = revision;
        Ok(revision)
    }
    pub fn manifest(&self) -> Manifest {
        Manifest {
            protocol_version: 1,
            project_id: self.project_id,
            revision: self.revision,
            frame: self.frame,
            objects: self
                .objects
                .values()
                .map(|o| ManifestObject {
                    id: o.id,
                    name: o.name.clone(),
                    layer: o.layer.clone(),
                    parent: o.parent,
                    visible: o.visible,
                    geometry_kind: o.geometry.as_ref().map(|g| g.data().kind().to_string()),
                    estimated_geometry_bytes: o.geometry.as_ref().map_or(0, GeometryLease::estimated_bytes),
                })
                .collect(),
            retained_geometry_bytes: self.budget.used(),
            geometry_budget_bytes: self.budget.limit(),
        }
    }
}
/// Metadata-only exchange DTO: no hidden geometry upload.
#[derive(Debug, Serialize)]
pub struct Manifest {
    pub protocol_version: u32,
    pub project_id: Id,
    pub revision: u64,
    pub frame: Frame,
    pub objects: Vec<ManifestObject>,
    pub retained_geometry_bytes: usize,
    pub geometry_budget_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct ManifestObject {
    pub id: Id,
    pub name: String,
    pub layer: String,
    pub parent: Option<Id>,
    pub visible: bool,
    pub geometry_kind: Option<String>,
    pub estimated_geometry_bytes: usize,
}
