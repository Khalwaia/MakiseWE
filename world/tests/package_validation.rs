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

#[test]
fn rejects_condition_without_matching_component() {
    let (_temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "invalid-condition",
          "locations": [{"id":"room","name":"Room","anchors":[{"id":"desk","name":"Desk"}]}],
          "connections": [],
          "objects": [{
            "id":"lamp","name":"Lamp","anchor_id":"desk",
            "initial_state":{"charge_permille":500}
          }]
        }"#,
    );
    let error = WorldDefinition::load(path, &PathGuard::default()).unwrap_err();
    assert!(error.to_string().contains("without chargeable component"));
}

#[test]
fn rejects_condition_outside_physical_bounds() {
    let (_temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "invalid-temperature",
          "locations": [{"id":"room","name":"Room","anchors":[{"id":"desk","name":"Desk"}]}],
          "connections": [],
          "objects": [{
            "id":"heater","name":"Heater","anchor_id":"desk",
            "components":["heatable"],
            "initial_state":{"temperature_millicelsius":500001}
          }]
        }"#,
    );
    let error = WorldDefinition::load(path, &PathGuard::default()).unwrap_err();
    assert!(error.to_string().contains("outside physical bounds"));
}
