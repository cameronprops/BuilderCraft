//! Ensure the actual worker executable is usable by a host process, not
//! just a library function. Launch a fresh native process like the CAD host.
use std::{io::Write, process::{Command, Stdio}};

#[test]
fn boxed_exact_geometry_from_child_process_can_be_reopened_by_another() {
    let worker = env!("CARGO_BIN_EXE_worldwright-brep-worker");
    let call = |request: serde_json::Value| -> serde_json::Value {
        let mut child = Command::new(worker)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().expect("start BRep worker");
        child.stdin.take().expect("worker stdin")
            .write_all(serde_json::to_string(&request).unwrap().as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "native worker failed: {}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).unwrap()
    };

    let created = call(serde_json::json!({"op":"box","min":[0.0,0.0,0.0],"max":[1.0,2.0,3.0]}));
    assert_eq!(created["ok"], true, "{created}");
    let brep = created["solids"][0]["brep"].clone();
    let inspected = call(serde_json::json!({"op":"inspect","brep":brep}));
    assert_eq!(inspected["ok"], true, "{inspected}");
    assert_eq!(inspected["solids"][0]["faces"], 6);
    assert!((inspected["solids"][0]["volume"].as_f64().unwrap() - 6.0).abs() < 1e-8);
    let difference = call(serde_json::json!({"op":"boolean","operation":"difference","left_brep":brep,"right_brep":brep}));
    assert_eq!(difference["ok"], true, "{difference}");
    assert_eq!(difference["solids"].as_array().unwrap().len(), 0);
}
