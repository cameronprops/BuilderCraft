//! Worldwright CAD/API commands for shared kernel operations.
//! These are numeric, headless commands: no geometry algorithm is duplicated.
use super::*;
use buildercraft_kernel::{SHARED_TOOLS, ToolRequest, ToolValue, TreeMatchPolicy, execute_shared_tool_with_matching};
use serde_json::json;
use std::collections::BTreeMap;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.tool.run", "Run Shared Native Tool", run)
            .params("{operation,inputs:{port:{kind,value},...},matching?:shortest|longest|cross_reference}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.tool.list", "List Paired CAD/OrbWeaver Tools", list).enabled(always).noundo(),
        CommandSpec::new("worldwright.array.linear", "Linear Array of 3D Geometry", array_linear)
            .params("{inputs:{geometry,step,count}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.array.rectangular", "Rectangular/3D Array", array_rectangular)
            .params("{inputs:{geometry,x_step,y_step,z_step,nx,ny,nz}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.array.polar", "Polar Array", array_polar)
            .params("{inputs:{geometry,center,axis,sweep_degrees,count}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.array.path", "Array Along Curve", array_path).params("{inputs:{geometry,path,count}}").enabled(always).noundo(),
        CommandSpec::new("worldwright.project", "Project Geometry to Plane", project)
            .params("{inputs:{geometry,origin,normal,direction}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.flow_along_srf", "Flow Along Surface Patch", flow_along_srf)
            .params("{inputs:{geometry,base,target}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.pushpull", "PushPull Planar Face", pushpull).params("{inputs:{face,distance}}").enabled(always).noundo(),
        CommandSpec::new("worldwright.project.mesh", "Project Geometry Onto Polygon Mesh", project_mesh)
            .params("{inputs:{geometry,target,direction}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.project.nurbs", "Project Geometry Onto Rational NURBS Surface", project_nurbs)
            .params("{inputs:{geometry,target,direction}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.flow_along_nurbs", "Flow Geometry Between Rational NURBS Surfaces", flow_along_nurbs)
            .params("{inputs:{geometry,base,target}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.surface.ruled", "Construct Rational Ruled Surface", ruled_surface)
            .params("{inputs:{a:{kind:curve,value:{degree,control,weights,knots}},b:{kind:curve,value:{...}}}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.point.distance", "Distance Between 3D Points", point_distance)
            .params("{inputs:{a:{kind:point,value:{x,y,z}},b:{kind:point,value:{x,y,z}}}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.point.midpoint", "Midpoint of 3D Points", point_midpoint).enabled(always).noundo(),
        CommandSpec::new("worldwright.point.interpolate", "Interpolate Between 3D Points", point_interpolate).enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.length", "Vector Length", vector_length).enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.normalize", "Normalize Vector", vector_normalize).enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.dot", "Vector Dot Product", vector_dot).enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.cross", "Vector Cross Product", vector_cross).enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.length", "Polyline Length", polyline_length).enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.divide_count", "Divide Polyline by Count", polyline_divide_count).enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.divide_distance", "Divide Polyline by Spacing", polyline_divide_distance).enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.point_at_station", "Evaluate Point at Polyline Station", polyline_point_at_station)
            .params("{inputs:{points:{kind:polyline,value:[{x,y,z},...]},station:{kind:number,value}}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.polyline.tangent_at_station", "Evaluate Tangent at Polyline Station", polyline_tangent_at_station)
            .params("{inputs:{points,station}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.polyline.offset_xy", "Offset Planar Polyline", polyline_offset_xy)
            .params("{inputs:{points,distance,miter_limit}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.join", "Join Open Polyline Curves", curve_join)
            .params("{inputs:{a,b,tolerance}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.split", "Split Polyline Curve at Station", curve_split)
            .params("{inputs:{points,station}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.trim", "Trim Polyline Curve to Interval", curve_trim)
            .params("{inputs:{points,start,end}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.extend", "Extend Polyline Curve End", curve_extend)
            .params("{inputs:{points,distance,at_start}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.rebuild", "Rebuild Curve As Nonrational NURBS", curve_rebuild)
            .params("{inputs:{points,controls,degree}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.offset", "Offset Open Curve in XY Plane", curve_offset)
            .params("{inputs:{points,distance,miter_limit}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.pipe", "Pipe Along Polyline Rail", curve_pipe)
            .params("{inputs:{rail,radius,sides}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.sweep1", "Sweep Convex Section Along One Rail", curve_sweep1)
            .params("{inputs:{rail,section}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.sweep2", "Sweep Convex Section Between Two Rails", curve_sweep2)
            .params("{inputs:{rail_a,rail_b,section}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.curve.blend", "G0/G1/G2 NURBS Curve Blend (Headless)", curve_blend)
            .alias(&["blend"])
            .params("{inputs:{a:curve,b:curve,continuity:count(0|1|2),strength_a:number,strength_b:number,reverse_a:count(0|1),reverse_b:count(0|1)}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.nurbs.evaluate", "Evaluate Exact Rational NURBS", nurbs_evaluate)
            .params("{inputs:{curve:{kind:curve,value:{degree,control,weights,knots}},t:{kind:number,value}}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.nurbs.split", "Split Rational NURBS Without Tessellation", nurbs_split)
            .params("{inputs:{curve,t}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.nurbs.trim", "Trim Rational NURBS Interval", nurbs_trim)
            .params("{inputs:{curve,start,end}}")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.tree.validate", "Validate Data Tree", tree_validate).enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.flatten", "Flatten Data Tree", tree_flatten).enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.graft", "Graft Data Tree", tree_graft).enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.simplify", "Simplify Data Tree", tree_simplify).enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.match", "Match Two Data Trees", tree_match).enabled(always).noundo(),
        CommandSpec::new("worldwright.mesh.decimate", "Simplify Derived Mesh Using QEM", mesh_decimate)
            .params("{inputs:{geometry,target_faces,max_error,normal_degrees,preserve_boundary,crease_degrees}}")
            .enabled(always)
            .noundo(),
    ]
}

fn invalid(message: &str) -> crate::EngineError {
    crate::EngineError::Other(message.into())
}
fn invoke(operation: &str, p: &Value) -> Result<Value> {
    let inputs: BTreeMap<String, ToolValue> =
        serde_json::from_value(p.get("inputs").cloned().ok_or_else(|| invalid("inputs required"))?).map_err(|e| invalid(&e.to_string()))?;
    // Matching is an optional per-command modifier, not a duplicate operator.
    // Omitted matching preserves the native 'shortest' list policy.
    let matching: TreeMatchPolicy = match p.get("matching") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| invalid(&format!("matching: {e}")))?,
        None => TreeMatchPolicy::Shortest,
    };
    let output =
        execute_shared_tool_with_matching(&ToolRequest { operation: operation.into(), inputs }, matching).map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({"operation":operation,"output":output}))
}
fn run(_: &mut Session, p: &Value) -> Result<Value> {
    let operation = p.get("operation").and_then(Value::as_str).ok_or_else(|| invalid("operation required"))?;
    invoke(operation, p)
}
fn list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"paired_tools": SHARED_TOOLS}))
}
macro_rules! paired_command {
    ($function:ident, $operation:literal) => {
        fn $function(_: &mut Session, p: &Value) -> Result<Value> {
            invoke($operation, p)
        }
    };
}
paired_command!(array_linear, "kernel.array.linear");
paired_command!(array_rectangular, "kernel.array.rectangular");
paired_command!(array_polar, "kernel.array.polar");
paired_command!(array_path, "kernel.array.path");
paired_command!(project, "kernel.project.plane");
paired_command!(flow_along_srf, "kernel.surface.flow_patch");
paired_command!(pushpull, "kernel.solid.pushpull_quad");
paired_command!(project_mesh, "kernel.project.mesh");
paired_command!(project_nurbs, "kernel.project.nurbs");
paired_command!(flow_along_nurbs, "kernel.surface.flow_nurbs");
paired_command!(ruled_surface, "kernel.surface.ruled");
paired_command!(point_distance, "kernel.point.distance");
paired_command!(point_midpoint, "kernel.point.midpoint");
paired_command!(point_interpolate, "kernel.point.interpolate");
paired_command!(vector_length, "kernel.vector.length");
paired_command!(vector_normalize, "kernel.vector.normalize");
paired_command!(vector_dot, "kernel.vector.dot");
paired_command!(vector_cross, "kernel.vector.cross");
paired_command!(polyline_length, "kernel.polyline.length");
paired_command!(polyline_divide_count, "kernel.polyline.divide_count");
paired_command!(polyline_divide_distance, "kernel.polyline.divide_distance");
paired_command!(polyline_point_at_station, "kernel.polyline.point_at_station");
paired_command!(polyline_tangent_at_station, "kernel.polyline.tangent_at_station");
paired_command!(polyline_offset_xy, "kernel.polyline.offset_xy");
paired_command!(curve_join, "kernel.curve.join");
paired_command!(curve_split, "kernel.curve.split");
paired_command!(curve_trim, "kernel.curve.trim");
paired_command!(curve_extend, "kernel.curve.extend");
paired_command!(curve_rebuild, "kernel.curve.rebuild");
paired_command!(curve_offset, "kernel.curve.offset");
paired_command!(curve_pipe, "kernel.curve.pipe");
paired_command!(curve_sweep1, "kernel.curve.sweep1");
paired_command!(curve_sweep2, "kernel.curve.sweep2");
paired_command!(curve_blend, "kernel.curve.blend");
paired_command!(nurbs_evaluate, "kernel.nurbs.evaluate");
paired_command!(nurbs_split, "kernel.nurbs.split");
paired_command!(nurbs_trim, "kernel.nurbs.trim");
paired_command!(tree_validate, "kernel.tree.validate");
paired_command!(tree_flatten, "kernel.tree.flatten");
paired_command!(tree_graft, "kernel.tree.graft");
paired_command!(tree_simplify, "kernel.tree.simplify");
paired_command!(tree_match, "kernel.tree.match");
paired_command!(mesh_decimate, "kernel.mesh.decimate");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ruled_surface_direct_command_uses_shared_exact_geometry() {
        use cadcraft_geom::{Vec3, nurbs3d::Curve};
        let a = Curve::from_control(vec![Vec3::ZERO, Vec3::new(5., 0., 0.)], 1).unwrap();
        let b = Curve::from_control(vec![Vec3::new(0., 0., 2.), Vec3::new(5., 0., 2.)], 1).unwrap();
        let inputs = json!({"a":{"kind":"curve","value":a},"b":{"kind":"curve","value":b}});
        let mut session = Session::new();
        let revision = session.state().unwrap().revision;
        let direct = session.execute("worldwright.surface.ruled", &json!({"inputs":inputs})).unwrap();
        let generic = session.execute("worldwright.tool.run", &json!({"operation":"orbweaver.surface.ruled","inputs":inputs})).unwrap();
        assert_eq!(direct["output"], generic["output"]);
        assert_eq!(direct["output"]["kind"], "surface");
        assert_eq!(direct["output"]["value"]["degree_v"], 1);
        assert_eq!(session.state().unwrap().revision, revision);
        let mut invalid = inputs.clone();
        invalid["b"]["value"]["knots"] = json!([0., 0., 0.5, 1.]);
        assert!(session.execute("worldwright.surface.ruled", &json!({"inputs":invalid})).is_err());
        assert_eq!(session.state().unwrap().revision, revision);
    }
    #[test]
    fn qem_cad_and_generic_api_are_non_mutating_and_identical() {
        use buildercraft_kernel::{PolygonFace, PolygonMesh};
        use cadcraft_geom::Vec3;
        let poly = PolygonMesh {
            vertices: vec![
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(0.0, -1.0, 0.0),
            ],
            faces: vec![
                PolygonFace::Triangle([0, 2, 3]),
                PolygonFace::Triangle([0, 3, 4]),
                PolygonFace::Triangle([0, 4, 5]),
                PolygonFace::Triangle([0, 5, 2]),
                PolygonFace::Triangle([1, 3, 2]),
                PolygonFace::Triangle([1, 4, 3]),
                PolygonFace::Triangle([1, 5, 4]),
                PolygonFace::Triangle([1, 2, 5]),
            ],
        };
        let inputs: BTreeMap<String, ToolValue> = BTreeMap::from([
            ("geometry".into(), ToolValue::Mesh(poly)),
            ("target_faces".into(), ToolValue::Count(6)),
            ("max_error".into(), ToolValue::Number(1000.0)),
            ("normal_degrees".into(), ToolValue::Number(85.0)),
            ("preserve_boundary".into(), ToolValue::Count(1)),
            ("crease_degrees".into(), ToolValue::Number(180.0)),
        ]);
        let request = json!({"inputs":inputs});
        let mut session = Session::new();
        let revision = session.state().ok().map(|state| state.revision);
        let direct = session.execute("worldwright.mesh.decimate", &request);
        let generic = session.execute("worldwright.tool.run", &json!({"operation":"kernel.mesh.decimate","inputs":inputs}));
        assert!(direct.is_ok());
        assert_eq!(direct.as_ref().ok(), generic.as_ref().ok());
        if let Ok(value) = direct {
            assert_eq!(value["output"]["kind"], "mesh");
            assert_eq!(value["output"]["value"]["faces"].as_array().map(Vec::len), Some(6));
        }
        assert_eq!(session.state().ok().map(|state| state.revision), revision);
    }

    #[test]
    fn command_and_generic_dispatch_execute_identical_point_distance() {
        let mut session = Session::new();
        let inputs = json!({
            "a":{"kind":"point","value":{"x":0.,"y":0.,"z":0.}},
            "b":{"kind":"point","value":{"x":3.,"y":4.,"z":0.}}
        });
        let direct = session.execute("worldwright.point.distance", &json!({"inputs":inputs})).unwrap();
        let generic = session
            .execute(
                "worldwright.tool.run",
                &json!({
                    "operation":"kernel.point.distance","inputs":inputs
                }),
            )
            .unwrap();
        assert_eq!(direct, generic);
        assert_eq!(direct["output"]["kind"], "number");
        assert_eq!(direct["output"]["value"], 5.);
    }
    #[test]
    fn numeric_modifier_uses_the_same_point_interpolator() {
        let mut session = Session::new();
        let result = session
            .execute(
                "worldwright.point.interpolate",
                &json!({"inputs":{
                    "a":{"kind":"point","value":{"x":0.,"y":0.,"z":0.}},
                    "b":{"kind":"point","value":{"x":8.,"y":0.,"z":0.}},
                    "t":{"kind":"number","value":0.25}
                }}),
            )
            .unwrap();
        assert_eq!(result["output"]["value"]["x"], 2.);
    }
    #[test]
    fn malformed_ports_and_unknown_operations_are_rejected() {
        let mut session = Session::new();
        assert!(session.execute("worldwright.point.distance", &json!({"inputs":{}})).is_err());
        assert!(
            session
                .execute(
                    "worldwright.tool.run",
                    &json!({
                        "operation":"kernel.missing","inputs":{}
                    })
                )
                .is_err()
        );
    }
    #[test]
    fn graft_command_routes_to_the_same_native_kernel_as_orbweaver() {
        let mut session = Session::new();
        let tree = json!({"kind":"tree","value":{"branches":[
            {"path":[0],"items":[{"kind":"number","value":3.0},{"kind":"number","value":7.0}]}
        ]}});
        let inputs = json!({"tree": tree});
        let direct = session.execute("worldwright.tree.graft", &json!({"inputs":inputs})).unwrap();
        let generic = session
            .execute(
                "worldwright.tool.run",
                &json!({
                    "operation":"orbweaver.tree.graft","inputs":inputs
                }),
            )
            .unwrap();
        assert_eq!(direct["output"], generic["output"]);
        assert_eq!(direct["output"]["value"]["branches"].as_array().map(Vec::len), Some(2));
    }
    #[test]
    fn matching_modifier_rejects_invalid_branch_sets_atomically() {
        let mut session = Session::new();
        let result = session.execute(
            "worldwright.tree.match",
            &json!({"inputs":{
                "a":{"kind":"tree","value":{"branches":[{"path":[0],"items":[
                    {"kind":"number","value":1.0}]}]}},
                "b":{"kind":"tree","value":{"branches":[{"path":[1],"items":[
                    {"kind":"number","value":2.0}]}]}},
                "mode":{"kind":"match_mode","value":"longest"}
            }}),
        );
        assert!(result.is_err());
    }
    #[test]
    fn worldwright_command_broadcasts_tree_inputs_without_other_algorithms() {
        let mut session = Session::new();
        let tree = json!({"kind":"tree","value":{"branches":[
            {"path":[0,2],"items":[
                {"kind":"point","value":{"x":3.0,"y":0.0,"z":0.0}},
                {"kind":"point","value":{"x":4.0,"y":0.0,"z":0.0}}
            ]}
        ]}});
        let inputs = json!({
            "a":tree,
            "b":{"kind":"point","value":{"x":0.0,"y":0.0,"z":0.0}}
        });
        let direct = session
            .execute(
                "worldwright.point.distance",
                &json!({
                    "inputs":inputs
                }),
            )
            .unwrap();
        let generic = session
            .execute(
                "worldwright.tool.run",
                &json!({
                    "operation":"orbweaver.point.distance", "inputs":inputs
                }),
            )
            .unwrap();
        assert_eq!(direct["output"], generic["output"]);
        assert_eq!(direct["output"]["kind"], "tree");
        assert_eq!(direct["output"]["value"]["branches"][0]["path"], json!([0, 2]));
        assert_eq!(direct["output"]["value"]["branches"][0]["items"][0]["value"], 3.0);
        assert_eq!(direct["output"]["value"]["branches"][0]["items"][1]["value"], 4.0);
    }

    #[test]
    fn command_list_matching_is_an_explicit_modifier() {
        let mut session = Session::new();
        let a = json!({"kind":"tree","value":{"branches":[{"path":[0],"items":[
            {"kind":"point","value":{"x":1.0,"y":0.0,"z":0.0}},
            {"kind":"point","value":{"x":4.0,"y":0.0,"z":0.0}}
        ]}]}});
        let b = json!({"kind":"tree","value":{"branches":[{"path":[0],"items":[
            {"kind":"point","value":{"x":11.0,"y":0.0,"z":0.0}},
            {"kind":"point","value":{"x":22.0,"y":0.0,"z":0.0}},
            {"kind":"point","value":{"x":33.0,"y":0.0,"z":0.0}}
        ]}]}});
        let inputs = json!({"a":a,"b":b});
        let shortest = session.execute("worldwright.point.distance", &json!({"inputs":inputs})).unwrap();
        let longest = session
            .execute(
                "worldwright.point.distance",
                &json!({
                    "inputs":inputs,"matching":"longest"
                }),
            )
            .unwrap();
        assert_eq!(shortest["output"]["value"]["branches"][0]["items"].as_array().map(Vec::len), Some(2));
        assert_eq!(longest["output"]["value"]["branches"][0]["items"].as_array().map(Vec::len), Some(3));
        assert!(
            session
                .execute(
                    "worldwright.point.distance",
                    &json!({
                        "inputs":inputs,"matching":"not_a_mode"
                    })
                )
                .is_err()
        );
    }

    #[test]
    fn polyline_commands_share_a_kernel_and_preserve_the_document() {
        let mut session = Session::new();
        let revision = session.state().ok().map(|state| state.revision);
        let points = json!({"kind":"polyline","value":[
            {"x":0.,"y":0.,"z":0.},
            {"x":10.,"y":0.,"z":0.},
            {"x":10.,"y":10.,"z":0.}
        ]});
        let station_inputs = json!({"points":points, "station":{"kind":"number","value":15.0}});
        for (cmd, operation, expected) in [
            ("worldwright.polyline.point_at_station", "kernel.polyline.point_at_station", json!("point")),
            ("worldwright.polyline.tangent_at_station", "kernel.polyline.tangent_at_station", json!("vector")),
        ] {
            let direct = session.execute(cmd, &json!({"inputs":station_inputs})).unwrap();
            let generic = session
                .execute(
                    "worldwright.tool.run",
                    &json!({
                        "operation":operation,"inputs":station_inputs
                    }),
                )
                .unwrap();
            assert_eq!(direct, generic);
            assert_eq!(direct["output"]["kind"], expected);
        }
        let offset_inputs = json!({
            "points":points,
            "distance":{"kind":"number","value":1.},
            "miter_limit":{"kind":"number","value":4.}
        });
        let offset = session.execute("worldwright.polyline.offset_xy", &json!({"inputs":offset_inputs})).unwrap();
        assert_eq!(offset["output"]["kind"], "polyline");
        assert_eq!(offset["output"]["value"][1], json!({"x":9.,"y":1.,"z":0.}));
        assert_eq!(session.state().ok().map(|state| state.revision), revision);
    }

    #[test]
    fn nine_curve_commands_are_directly_executable_and_reuse_generic_tool() {
        let mut session = Session::new();
        let path = json!({"kind":"polyline","value":[
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":4.0,"y":0.0,"z":0.0},
            {"x":8.0,"y":0.0,"z":0.0}
        ]});
        let second = json!({"kind":"polyline","value":[{"x":8.0,"y":0.0,"z":0.0},{"x":10.0,"y":0.0,"z":0.0}]});
        let section = json!({"kind":"polyline","value":[
            {"x":0.0,"y":-0.2,"z":0.0},{"x":1.0,"y":-0.2,"z":0.0},
            {"x":1.0,"y":0.2,"z":0.0},{"x":0.0,"y":0.2,"z":0.0}
        ]});
        let rail_b = json!({"kind":"polyline","value":[
            {"x":0.0,"y":1.0,"z":0.0},
            {"x":4.0,"y":1.0,"z":0.0},
            {"x":8.0,"y":1.0,"z":0.0}
        ]});
        let n = |value: f64| json!({"kind":"number","value":value});
        let cnt = |value: u64| json!({"kind":"count","value":value});
        let cases = [
            ("join", json!({"a":path,"b":second,"tolerance":n(0.01)}), "polyline"),
            ("split", json!({"points":path,"station":n(5.0)}), "tree"),
            ("trim", json!({"points":path,"start":n(1.0),"end":n(7.0)}), "polyline"),
            ("extend", json!({"points":path,"distance":n(2.0),"at_start":cnt(0)}), "polyline"),
            ("rebuild", json!({"points":path,"controls":cnt(6),"degree":cnt(3)}), "curve"),
            ("offset", json!({"points":path,"distance":n(1.0),"miter_limit":n(4.0)}), "polyline"),
            ("pipe", json!({"rail":path,"radius":n(0.3),"sides":cnt(12)}), "mesh"),
            ("sweep1", json!({"rail":path,"section":section}), "mesh"),
            ("sweep2", json!({"rail_a":path,"rail_b":rail_b,"section":section}), "mesh"),
        ];
        let revision = session.state().ok().map(|state| state.revision);
        for (suffix, inputs, kind) in cases {
            let command = format!("worldwright.curve.{suffix}");
            let operation = format!("kernel.curve.{suffix}");
            let direct = session.execute(&command, &json!({"inputs":inputs})).unwrap();
            let generic = session
                .execute(
                    "worldwright.tool.run",
                    &json!({
                        "operation":operation,"inputs":inputs
                    }),
                )
                .unwrap();
            assert_eq!(direct, generic, "command mismatch: {suffix}");
            assert_eq!(direct["output"]["kind"], kind, "wrong output type: {suffix}");
        }
        assert_eq!(session.state().ok().map(|state| state.revision), revision);
    }

    #[test]
    fn exact_rational_nurbs_commands_share_generic_adapter_without_scene_changes() {
        let mut session = Session::new();
        let curve = json!({"kind":"curve","value":{
            "degree":2,
            "control":[{"x":0.,"y":0.,"z":0.},{"x":2.,"y":4.,"z":0.},{"x":4.,"y":0.,"z":0.}],
            "weights":[1.,0.5,1.],
            "knots":[0.,0.,0.,1.,1.,1.]
        }});
        let n = |v: f64| json!({"kind":"number","value":v});
        let original = session.state().ok().map(|s| s.revision);
        for (action, inputs, kind) in [
            ("evaluate", json!({"curve":curve,"t":n(0.5)}), "point"),
            ("split", json!({"curve":curve,"t":n(0.25)}), "tree"),
            ("trim", json!({"curve":curve,"start":n(0.2),"end":n(0.8)}), "curve"),
        ] {
            let direct = session.execute(&format!("worldwright.nurbs.{action}"), &json!({"inputs":inputs})).unwrap();
            let generic = session
                .execute(
                    "worldwright.tool.run",
                    &json!({
                        "operation":format!("kernel.nurbs.{action}"),"inputs":inputs
                    }),
                )
                .unwrap();
            assert_eq!(direct, generic);
            assert_eq!(direct["output"]["kind"], kind);
        }
        assert_eq!(session.state().ok().map(|s| s.revision), original);
    }

    #[test]
    fn discovery_contains_shared_node_and_command_pairs() {
        let mut session = Session::new();
        let result = session.execute("worldwright.tool.list", &json!({})).unwrap();
        assert_eq!(result["paired_tools"].as_array().map(Vec::len), Some(SHARED_TOOLS.len()));
        assert!(result["paired_tools"].as_array().unwrap().iter().any(|item| item["orbweaver_node"] == "orbweaver.curve.blend"));

        assert_eq!(result["paired_tools"][0]["orbweaver_node"], "orbweaver.point.distance");
    }
}
