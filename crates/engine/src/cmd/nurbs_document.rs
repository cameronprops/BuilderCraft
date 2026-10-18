//! Undoable document-native rational NURBS splitting and trimming.
//! Geometry lives in the existing shared kernel; this module only stages
//! persistence, object identities and stale selection/undo protection.
use super::*;
use buildercraft_kernel::{nurbs_curve_split, nurbs_curve_trim};
use cadcraft_doc::organization::{GeometryObject, Shape};
use serde_json::json;
use std::sync::Arc;

const MAX_OBJECTS: usize = 4096;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("nurbs.split_object", "Split Selected Rational NURBS Curve", split).params("{id,t:0..1,selected_revision?}"),
        CommandSpec::new("nurbs.trim_object", "Trim Selected Rational NURBS Curve", trim)
            .params("{id,start:0..1,end:0..1,copy?:true,selected_revision?}"),
    ]
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(message.to_owned())
}

fn revision_check(s: &Session, p: &Value) -> Result<()> {
    if let Some(n) = p.get("selected_revision") {
        let checked = n.as_u64().ok_or_else(|| invalid("selected_revision must be a nonnegative integer"))?;
        if checked != s.state()?.revision {
            return Err(invalid("stale NURBS object selection revision"));
        }
    }
    Ok(())
}

fn source(s: &Session, p: &Value) -> Result<GeometryObject> {
    revision_check(s, p)?;
    let id = p.get("id").and_then(Value::as_u64).filter(|v| *v != 0).ok_or_else(|| invalid("id must be a nonzero NURBS object ID"))?;
    let geometry = s.doc()?.geometry3d.iter().find(|obj| obj.id == id).ok_or_else(|| invalid("selected NURBS object does not exist"))?;
    if !matches!(&geometry.shape, Shape::Curve(_)) {
        return Err(invalid("selected geometry must be a rational NURBS curve"));
    }
    Ok(geometry.clone())
}

fn parameter(p: &Value, key: &str) -> Result<f64> {
    p.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .ok_or_else(|| invalid(&format!("{key} must be a finite normalized parameter in [0,1]")))
}
fn reserve_objects(s: &Session, count: usize) -> Result<()> {
    let doc = s.doc()?;
    if doc
        .geometry3d
        .len()
        .checked_add(doc.mesh3d.len())
        .and_then(|n| n.checked_add(doc.exact_breps.len()))
        .and_then(|n| n.checked_add(count))
        .is_none_or(|n| n > MAX_OBJECTS)
        || doc.handseed.checked_add(count as u64).is_none()
    {
        return Err(invalid("native NURBS document object or identity limit"));
    }
    Ok(())
}
/// Exact rational knot insertion is performed before the drawing mutates.
/// Result is a **single undoable transaction** containing two new curves.
fn split(s: &mut Session, p: &Value) -> Result<Value> {
    let original = source(s, p)?;
    let t = parameter(p, "t")?;
    let Shape::Curve(curve) = &original.shape else {
        return Err(invalid("NURBS curve required"));
    };
    let (a, b) = nurbs_curve_split(curve, t).map_err(|e| invalid(&e.to_string()))?;
    reserve_objects(s, 2)?;
    let names = [format!("{} A", original.name), format!("{} B", original.name)];
    if names.iter().any(|n| n.len() > 256) {
        return Err(invalid("split output names exceed 256 bytes"));
    }

    let doc = s.doc_mut()?;
    let id_a = doc.new_handle().0;
    let id_b = doc.new_handle().0;
    doc.geometry3d.push(GeometryObject {
        id: id_a,
        name: names[0].clone(),
        layer: original.layer.clone(),
        visible: original.visible,
        shape: Shape::Curve(Arc::new(a)),
    });
    doc.geometry3d.push(GeometryObject {
        id: id_b,
        name: names[1].clone(),
        layer: original.layer,
        visible: original.visible,
        shape: Shape::Curve(Arc::new(b)),
    });
    Ok(json!({"source_id":original.id,"ids":[id_a,id_b],"count":2,
        "source_preserved":true,"representation":"rational_nurbs","exact_shape_preserved":true}))
}
/// Preserve the source or replace its rational curve in-place. The object
/// retains stable identity/layer/visibility on in-place trimming.
fn trim(s: &mut Session, p: &Value) -> Result<Value> {
    let original = source(s, p)?;
    let start = parameter(p, "start")?;
    let end = parameter(p, "end")?;
    let copy = p.get("copy").map(|v| v.as_bool().ok_or_else(|| invalid("copy must be boolean"))).transpose()?.unwrap_or(true);
    let Shape::Curve(curve) = &original.shape else {
        return Err(invalid("NURBS curve required"));
    };
    let trimmed = nurbs_curve_trim(curve, start, end).map_err(|e| invalid(&e.to_string()))?;
    if copy {
        reserve_objects(s, 1)?;
        let title = format!("{} Trim", original.name);
        if title.len() > 256 {
            return Err(invalid("trim copy name exceeds 256 bytes"));
        }
        let doc = s.doc_mut()?;
        let id = doc.new_handle().0;
        doc.geometry3d.push(GeometryObject {
            id,
            name: title,
            layer: original.layer,
            visible: original.visible,
            shape: Shape::Curve(Arc::new(trimmed)),
        });
        Ok(json!({"id":id,"source_id":original.id,"copy":true,
            "representation":"rational_nurbs","exact_shape_preserved":true}))
    } else {
        let doc = s.doc_mut()?;
        let target = doc.geometry3d.iter_mut().find(|obj| obj.id == original.id).ok_or_else(|| invalid("source NURBS object disappeared"))?;
        target.shape = Shape::Curve(Arc::new(trimmed));
        Ok(json!({"id":original.id,"source_id":original.id,"copy":false,
            "representation":"rational_nurbs","exact_shape_preserved":true}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::{Vec3, nurbs3d::Curve};

    fn build(s: &mut Session) -> u64 {
        let curve = Curve {
            degree: 2,
            control: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 4., 0.), Vec3::new(4., 0., 0.)],
            weights: vec![1., 0.5, 1.],
            knots: vec![0., 0., 0., 1., 1., 1.],
        };
        s.execute("nurbs.curve3d", &json!({"name":"Ride Centerline","curve":curve})).unwrap()["id"].as_u64().unwrap()
    }
    fn exact(s: &Session, id: u64) -> Arc<Curve> {
        let doc = s.doc().unwrap();
        let obj = doc.geometry3d.iter().find(|o| o.id == id).unwrap();
        let Shape::Curve(curve) = &obj.shape else { panic!("expected curve") };
        curve.clone()
    }
    #[test]
    fn split_creates_two_editable_native_curves_preserves_source_and_undo() {
        let mut s = Session::new();
        let id = build(&mut s);
        let original = exact(&s, id);
        let initial = s.doc().unwrap().geometry3d.len();
        let result = s.execute("nurbs.split_object", &json!({"id":id,"t":0.4})).unwrap();
        let a = result["ids"][0].as_u64().unwrap();
        let b = result["ids"][1].as_u64().unwrap();
        assert_eq!(s.doc().unwrap().geometry3d.len(), initial + 2);
        assert_eq!(exact(&s, id), original);
        for k in 0..=16 {
            let t = k as f64 / 16.;
            let (piece, u) = if t <= 0.4 { (a, t / 0.4) } else { (b, (t - 0.4) / 0.6) };
            let deviation = (exact(&s, piece).evaluate(u).unwrap() - original.evaluate(t).unwrap()).len();
            assert!(deviation < 1e-8);
        }
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().geometry3d.len(), initial);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().geometry3d.len(), initial + 2);
    }
    #[test]
    fn trim_copy_and_in_place_preserve_identity_with_undo_and_io() {
        let mut s = Session::new();
        let id = build(&mut s);
        let source = exact(&s, id);
        let copy = s.execute("nurbs.trim_object", &json!({"id":id,"start":0.2,"end":0.8})).unwrap();
        let copied = copy["id"].as_u64().unwrap();
        assert_eq!(exact(&s, id), source);
        for k in 0..=10 {
            let t = k as f64 / 10.;
            assert!((exact(&s, copied).evaluate(t).unwrap() - source.evaluate(0.2 + 0.6 * t).unwrap()).len() < 1e-8);
        }
        let project = cadcraft_io::write(s.doc().unwrap(), "native.dftba").unwrap();
        let read = cadcraft_io::read(&project, "native.dftba").unwrap();
        assert_eq!(read.geometry3d, s.doc().unwrap().geometry3d);
        let output = s.execute("nurbs.trim_object", &json!({"id":id,"start":0.1,"end":0.9,"copy":false})).unwrap();
        assert_eq!(output["id"], id);
        assert_ne!(exact(&s, id), source);
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(exact(&s, id), source);
    }
    #[test]
    fn invalid_parameter_stale_selection_and_surface_source_are_non_mutating() {
        let mut s = Session::new();
        let id = build(&mut s);
        let rev = s.state().unwrap().revision;
        let initial = s.doc().unwrap().geometry3d.clone();
        for input in
            [json!({"id":id,"t":0.0}), json!({"id":id,"t":1.0}), json!({"id":id,"t":f64::NAN}), json!({"id":id,"t":0.4,"selected_revision":rev+1})]
        {
            assert!(s.execute("nurbs.split_object", &input).is_err());
        }
        assert!(s.execute("nurbs.trim_object", &json!({"id":id,"start":0.8,"end":0.2})).is_err());
        assert_eq!(s.doc().unwrap().geometry3d, initial);
    }
}
