use std::io::Write;

use makise_world::{PathGuard, WorldDefinition};

fn write_manifest(json: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manifest.json");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(json.as_bytes()).unwrap();
    (temp, path)
}

#[test]
fn rejects_unknown_resource() {
    let (_temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "invalid-resource",
          "locations": [{"id":"room","name":"Room","anchors":[{"id":"desk","name":"Desk"}]}],
          "connections": [],
          "objects": [{
            "id":"lamp","name":"Lamp","anchor_id":"desk",
            "actions":[{
              "action_id":"object.toggle_power",
              "description":"Toggle",
              "duration_ms":1,
              "required_resources":["telepathy"]
            }]
          }]
        }"#,
    );
    assert!(WorldDefinition::load(path, &PathGuard::default()).is_err());
}

#[test]
fn rejects_unreachable_anchor() {
    let (_temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "invalid-topology",
          "locations": [{
            "id":"room","name":"Room",
            "anchors":[{"id":"bed","name":"Bed"},{"id":"desk","name":"Desk"}]
          }],
          "connections": [],
          "objects": []
        }"#,
    );
    assert!(WorldDefinition::load(path, &PathGuard::default()).is_err());
}
