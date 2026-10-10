//! Shared control-curve construction from point input. No UI dependencies.
use super::*;
use cadcraft_geom::{Vec3, nurbs3d};
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("nurbs.controlcurve3d", "Control Curve From Points", curve)
            .params("{name,points:[[x,y,z],...],degree?:min(3,count-1),expected_uid?,expected_revision?}"),
    ]
}
pub(super) fn curve(s: &mut Session, p: &Value) -> Result<Value> {
    for (key, current) in [("expected_uid", s.state()?.uid), ("expected_revision", s.state()?.revision)] {
        if let Some(value) = p.get(key)
            && value.as_u64() != Some(current)
        {
            return Err(bad("nurbs.controlcurve3d", "Stale point input"));
        }
    }
    let points = p
        .get("points")
        .and_then(Value::as_array)
        .filter(|a| (2..=4096).contains(&a.len()))
        .ok_or_else(|| bad("nurbs.controlcurve3d", "2 to 4096 control points required"))?;
    let degree = p
        .get("degree")
        .map_or(Some(3.min(points.len() - 1)), |v| v.as_u64().and_then(|n| usize::try_from(n).ok()))
        .filter(|n| (1..=5).contains(n) && *n < points.len())
        .ok_or_else(|| bad("nurbs.controlcurve3d", "Invalid curve degree"))?;
    let mut control = Vec::new();
    control.try_reserve_exact(points.len()).map_err(|_| bad("nurbs.controlcurve3d", "Control buffer admission failed"))?;
    for point in points {
        let point: [f64; 3] = serde_json::from_value(point.clone()).map_err(|e| bad("nurbs.controlcurve3d", e.to_string()))?;
        control.push(Vec3::new(point[0], point[1], point[2]));
    }
    let curve = nurbs3d::Curve::from_control(control, degree).ok_or_else(|| bad("nurbs.controlcurve3d", "Invalid control curve"))?;
    let d = s.doc()?;
    if d.geometry3d.len().checked_add(d.mesh3d.len()).is_none_or(|n| n >= 4096)
        || d.handseed == u64::MAX
        || d.geometry3d.iter().any(|o| o.id == d.handseed)
        || d.mesh3d.iter().any(|o| o.id == d.handseed)
        || d.entity(Handle(d.handseed)).is_some()
        || d.layer(&d.header.str("CLAYER", "0")).is_some_and(|l| !l.visible() || l.locked)
    {
        return Err(bad("nurbs.controlcurve3d", "Object limit reached or current layer is hidden/locked"));
    }
    super::buildercraft::curve3d(s, &json!({"name":p.get("name").cloned().unwrap_or_else(||json!("Control curve")),"curve":curve}))
}
