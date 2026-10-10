//! Isolated OpenCascade acceptance fixture, not the WorldWright backend yet.
//! Real OCCT exact BRep boolean operations, including touching planar solids.
//! This crate is deliberately outside WorldWright's default workspace.
#![forbid(unsafe_code)]
use glam::dvec3;
use opencascade::primitives::Shape;

fn assert_faces(shape:&Shape,minimum:usize){
    let count=shape.faces().count();
    assert!(count>=minimum,"expected at least {minimum} BRep faces; found {count}");
}
#[test]
fn exact_brep_box_and_transversal_boolean_fixture() {
    let a=Shape::box_with_dimensions(2.,2.,2.);
    let b=Shape::box_from_corners(dvec3(.75,.65,.55),dvec3(2.75,2.65,2.55));
    assert_faces(&a,6);
    let intersection=a.intersect(&b);
    assert_faces(&intersection,6);
    let union=a.union(&b);
    assert_faces(&union,6);
    let cut=a.subtract(&b);
    assert_faces(&cut,6);
}
#[test]
fn exact_brep_tangency_and_coplanar_contact_regression(){
    let a=Shape::box_with_dimensions(2.,2.,2.);
    let touch=Shape::box_from_corners(dvec3(2.,0.,0.),dvec3(4.,2.,2.));
    let union=a.union(&touch);
    assert_faces(&union,6);
    // Intersections of touching solids can be lower-dimensional shapes;
    // OCCT should not panic even when the result is not an enclosed solid.
    let _contact=a.intersect(&touch);
}
#[test]
fn exact_brep_roundtrip_preserves_faces(){
    let shape=Shape::box_with_dimensions(2.,3.,4.);
    let path=std::env::temp_dir().join(format!("ww_occt_{}_{}.brep",std::process::id(),12345));
    shape.write_brep_bin(&path).expect("write OCCT binary BRep fixture");
    let decoded=Shape::read_brep_bin(&path).expect("read OCCT binary BRep fixture");
    assert_faces(&decoded,6);
    let _=std::fs::remove_file(path);
}
