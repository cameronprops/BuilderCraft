#![cfg(not(target_arch = "wasm32"))]
use cadcraft_engine::Session;
use serde_json::{Value, json};
use std::time::{Duration, Instant};
fn wait(session: &mut Session, revision: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = session.execute("visualization.status", &json!({})).unwrap();
        assert!(status["error"].is_null(), "{status}");
        if status["source_revision"] == revision {
            return status;
        }
        assert!(Instant::now() < deadline, "publisher timed out: {status}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn unsaved_edits_undo_and_document_binding() {
    let directory = std::env::temp_dir().join(format!("buildercraft-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let mut session = Session::new();
    session.execute("visualization.start", &json!({"project_id":"00000000000000000000000000000001","directory":directory})).unwrap();
    let revision = session.state().unwrap().revision.to_string();
    wait(&mut session, &revision);
    for i in 0..25 {
        session.execute("production.set", &json!({"records":[{"id":"00000000000000000000000000000002","kind":"scene","name":format!("Scene {i}"),"parent":null,"attributes":{}}],"bindings":[],"links":[]})).unwrap();
    }
    let revision = session.state().unwrap().revision.to_string();
    let status = wait(&mut session, &revision);
    assert!(status["sequence"].as_str().unwrap().parse::<u64>().unwrap() < 26);
    let snapshot: Value = serde_json::from_slice(&std::fs::read(directory.join("snapshot.json")).unwrap()).unwrap();
    assert_eq!(snapshot["production"]["records"][0]["name"], "Scene 24");
    session.undo().unwrap();
    let revision = session.state().unwrap().revision.to_string();
    wait(&mut session, &revision);
    session.new_drawing(true);
    session.execute("production.set", &json!({"records":[],"bindings":[],"links":[]})).unwrap();
    assert_eq!(session.execute("visualization.status", &json!({})).unwrap()["source_revision"], revision);
    session.active = 0;
    let object = session.execute("nurbs.curve3d", &json!({"name":"Live curve","curve":{"degree":1,"control":[{"x":0.0,"y":0.0,"z":0.0},{"x":1000.0,"y":0.0,"z":0.0}],"weights":[1.0,1.0],"knots":[0.0,0.0,1.0,1.0]}})).unwrap();
    let revision = session.state().unwrap().revision.to_string();
    wait(&mut session, &revision);
    let before: Value = serde_json::from_slice(&std::fs::read(directory.join("snapshot.json")).unwrap()).unwrap();
    session.execute("geometry3d.controlpoint", &json!({"id":object["id"],"index":1,"point":[2000.0,0.0,0.0]})).unwrap();
    let revision = session.state().unwrap().revision.to_string();
    wait(&mut session, &revision);
    let after: Value = serde_json::from_slice(&std::fs::read(directory.join("snapshot.json")).unwrap()).unwrap();
    assert_eq!(before["objects"][0]["id"], after["objects"][0]["id"]);
    assert_ne!(before["objects"][0]["geometry_key"], after["objects"][0]["geometry_key"]);
    session.execute("visualization.stop", &json!({})).unwrap();
    assert!(!directory.join("writer.lock").exists());
    assert_eq!(session.execute("visualization.status", &json!({})).unwrap()["running"], false);
    std::fs::remove_dir_all(directory).unwrap();
}
