//! WorldWright exact BRep **candidate**, backed by Truck's independently
//! developed NURBS/topology crates. One backend owns the solid; no second
//! handwritten edge/face/shell implementation is introduced here.
//!
//! Acceptance scope currently includes validated rectangular boxes and
//! **transversal-only** box booleans. Truck's shapeops currently documents
//! tangential intersections as unsupported. The adapter must never silently
//! fall back to a triangulated or voxel approximation.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]
#![forbid(unsafe_code)]

use truck_modeling::{builder, Point3, Solid, Vector3};

const LIMIT:f64=1e9;
const MIN_DIM:f64=1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub absolute:f64,
    pub relative:f64,
    pub angle_radians:f64,
}
impl Tolerance {
    pub fn new(absolute:f64,relative:f64,angle_radians:f64)->Result<Self,BrepError>{
        if !absolute.is_finite() || !(1e-10..=1e-2).contains(&absolute) ||
           !relative.is_finite() || !(1e-12..=0.1).contains(&relative) ||
           !angle_radians.is_finite() || !(1e-10..=0.1).contains(&angle_radians){
            return Err(BrepError::InvalidTolerance);
        }
        Ok(Self{absolute,relative,angle_radians})
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOperation { Union, Intersection, Difference }

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BrepError {
    #[error("BRep tolerance outside supported finite range")]
    InvalidTolerance,
    #[error("BRep primitive requires finite and positive coordinates and dimensions")]
    InvalidSolid,
    #[error("BRep solid is topologically or geometrically inconsistent")]
    InvalidTopology,
    #[error("transversal-only Truck backend does not support touching, tangential, coincident or disjoint box configurations")]
    NonTransversal,
    #[error("Truck boolean could not resolve the specified pair as a validated exact solid")]
    BooleanFailed,
    #[error("Truck backend trapped a topology/boolean panic; input rejected")]
    BackendPanicked,
}
/// An authoritative Truck BRep solid plus source bounding limits. No mesh is
/// baked as a competing geometry representation or silently substituted.
#[derive(Clone,Debug)]
pub struct ExactBox {
    pub origin:[f64;3],
    pub size:[f64;3],
    solid:Solid,
}
impl ExactBox {
    pub fn solid(&self)->&Solid { &self.solid }
    pub fn shape_counts(&self)->(usize,usize,usize){
        // Edge and vertex iterators traverse each oriented face use rather
        // than unique topological identities; only face count is canonical.
        (self.solid.face_iter().count(),self.solid.edge_iter().count(),self.solid.vertex_iter().count())
    }
    pub fn geometrically_consistent(&self)->bool {self.solid.is_geometric_consistent()}
}
fn admissible(v:f64)->bool{v.is_finite() && v.abs()<=LIMIT}
/// Construct native exact BRep prism from topologically connected swept
/// vertices/edge/face. This is not a manually stitched triangle mesh.
pub fn rectangular_solid(origin:[f64;3],size:[f64;3])->Result<ExactBox,BrepError>{
    if origin.into_iter().any(|v|!admissible(v)) ||
       size.into_iter().any(|v|!v.is_finite() || !(MIN_DIM..=LIMIT).contains(&v)) ||
       (0..3).any(|i|!admissible(origin[i]+size[i])) {
        return Err(BrepError::InvalidSolid);
    }
    let v=builder::vertex(Point3::new(origin[0],origin[1],origin[2]));
    let e=builder::tsweep(&v,Vector3::new(size[0],0.,0.));
    let face=builder::tsweep(&e,Vector3::new(0.,size[1],0.));
    let solid:Solid=builder::tsweep(&face,Vector3::new(0.,0.,size[2]));
    if !solid.is_geometric_consistent() || solid.face_iter().count()!=6 {
        return Err(BrepError::InvalidTopology);
    }
    Ok(ExactBox{origin,size,solid})
}
fn transversal_boxes(left:&ExactBox,right:&ExactBox,tol:Tolerance)->bool {
    // Shapeops' intersection algorithm is documented as transversal-only.
    // Reject all coincident/tangent planes before calling the backend, even
    // when such an operation could have a well-defined exact result.
    for axis in 0..3 {
        let a0=left.origin[axis];
        let a1=a0+left.size[axis];
        let b0=right.origin[axis];
        let b1=b0+right.size[axis];
        let overlap=a1.min(b1)-a0.max(b0);
        if overlap<=tol.absolute {return false;}
        if [a0,a1].iter().any(|a|[b0,b1].iter().any(|b|(a-b).abs()<=tol.absolute)){return false;}
    }
    true
}
/// Exact-solid boolean, never a mesh CSG fallback. The geometric broadphase
/// makes conservative admissions; a real surface-intersection classifier
/// and additional seam/trim gates are the next production BRep dependency.
pub fn boolean_boxes(left:&ExactBox,right:&ExactBox,operation:BooleanOperation,tol:Tolerance)->Result<Solid,BrepError>{
    if !left.geometrically_consistent() || !right.geometrically_consistent(){return Err(BrepError::InvalidTopology);}
    if !transversal_boxes(left,right,tol){return Err(BrepError::NonTransversal);}
    // Third-party boolean topology can internally panic on a complex case.
    // Contain this while Truck is an evaluation backend, and never publish
    // a half-complete result on failure.
    let computed=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match operation {
            BooleanOperation::Intersection=>truck_shapeops::and(&left.solid,&right.solid,tol.absolute),
            BooleanOperation::Union=>truck_shapeops::or(&left.solid,&right.solid,tol.absolute),
            BooleanOperation::Difference=>{
                let mut complement=right.solid.clone();
                complement.not();
                truck_shapeops::and(&left.solid,&complement,tol.absolute)
            }
        }
    })).map_err(|_|BrepError::BackendPanicked)?
      .ok_or(BrepError::BooleanFailed)?;
    if !computed.is_geometric_consistent(){return Err(BrepError::InvalidTopology);}
    Ok(computed)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn tolerance()->Tolerance {Tolerance::new(1e-6,1e-8,1e-4).unwrap()}
    #[test]
    fn native_exact_box_has_six_connected_faces_and_consistent_surfaces(){
        let b=rectangular_solid([0.,0.,0.],[2.,3.,4.]).unwrap();
        assert_eq!(b.shape_counts().0,6);
        assert!(b.geometrically_consistent());
        assert!(b.solid().boundaries().len()==1);
    }
    #[test]
    fn out_of_range_inputs_cannot_construct_invalid_topology(){
        assert_eq!(rectangular_solid([0.,0.,0.],[0.,1.,1.]).unwrap_err(),BrepError::InvalidSolid);
        assert_eq!(rectangular_solid([0.,0.,f64::NAN],[1.,1.,1.]).unwrap_err(),BrepError::InvalidSolid);
        assert_eq!(Tolerance::new(0.,1e-6,1e-5).unwrap_err(),BrepError::InvalidTolerance);
    }
    #[test]
    fn nontransversal_and_touching_cases_fail_explicitly(){
        let a=rectangular_solid([0.,0.,0.],[2.,2.,2.]).unwrap();
        for org in [[2.,0.,0.],[0.,0.,0.],[10.,10.,10.]] {
            let b=rectangular_solid(org,[2.,2.,2.]).unwrap();
            assert_eq!(boolean_boxes(&a,&b,BooleanOperation::Union,tolerance()).unwrap_err(),BrepError::NonTransversal);
        }
    }
    #[test]
    fn intersecting_noncoincident_box_predicate_allows_backend_computation(){
        let a=rectangular_solid([0.,0.,0.],[2.,2.,2.]).unwrap();
        let b=rectangular_solid([0.75,0.65,0.55],[2.,2.,2.]).unwrap();
        assert!(transversal_boxes(&a,&b,tolerance()));
        // Shapeops regression gate is intentionally strict: a backend that
        // cannot resolve a simple transversal pair cannot graduate to main.
        let result=boolean_boxes(&a,&b,BooleanOperation::Intersection,tolerance()).unwrap();
        assert!(result.is_geometric_consistent());
        assert!(result.face_iter().count()>0);
    }
}
