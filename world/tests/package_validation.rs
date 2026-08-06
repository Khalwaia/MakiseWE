use std::io::Write;

use makise_world::{PathGuard, WorldDefinition, WorldEngine};

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

#[test]
fn passive_quantity_consumption_uses_fractional_carry() {
    let (temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "passive-quantity",
          "initial_anchor_id": "pantry",
          "locations": [{"id":"room","name":"Room","anchors":[{"id":"pantry","name":"Pantry"}]}],
          "connections": [],
          "objects": [{
            "id":"food","name":"Food","anchor_id":"pantry",
            "components":["quantity"],
            "initial_state":{"quantity":{"amount":10,"unit":"serving"}},
            "passive_effects":[{
              "kind":"quantity_consumption",
              "id":"background-use",
              "active_amount_per_hour":2
            }]
          }]
        }"#,
    );
    let definition = WorldDefinition::load(path, &PathGuard::default()).unwrap();
    let started_at_ms = 1_000_000;
    let mut engine = WorldEngine::open(
        temp.path().join("world.db"),
        "test-passive-quantity",
        definition,
        "pantry",
        started_at_ms,
        &PathGuard::default(),
    )
    .unwrap();

    engine
        .resume_after_downtime(started_at_ms + 90 * 60_000)
        .unwrap();
    let food = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "food")
        .unwrap();
    assert_eq!(food.observed_properties["quantity_amount"], "7");
}

#[test]
fn rejects_passive_activation_without_component() {
    let (_temp, path) = write_manifest(
        r#"{
          "schema_version": 1,
          "world_id": "invalid-passive-activation",
          "locations": [{"id":"room","name":"Room","anchors":[{"id":"desk","name":"Desk"}]}],
          "connections": [],
          "objects": [{
            "id":"phone","name":"Phone","anchor_id":"desk",
            "components":["chargeable"],
            "initial_state":{"charge_permille":500},
            "passive_effects":[{
              "kind":"charge",
              "id":"charge",
              "when":{"power":true},
              "active_delta_per_hour_permille":100
            }]
          }]
        }"#,
    );
    let error = WorldDefinition::load(path, &PathGuard::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("checks power without powerable component")
    );
}
