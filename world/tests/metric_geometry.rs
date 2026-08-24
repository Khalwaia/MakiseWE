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
fn reads_optional_metric_bounds_and_preserves_legacy_packages() {
    let legacy = r#"{
      "schema_version": 1,
      "world_id": "legacy-room",
      "initial_anchor_id": "desk",
      "locations": [{"id":"room","name":"Room","anchors":[{"id":"desk","name":"Desk"}]}],
      "connections": [],
      "objects": []
    }"#;
    let metric = r#"{
      "schema_version": 1,
      "world_id": "metric-room",
      "initial_anchor_id": "desk",
      "locations": [{
        "id":"room","name":"Room",
        "metric_bounds_m":{"x":{"value":4.5,"unit":"m"},"y":{"value":3.0,"unit":"m"},"z":{"value":2.7,"unit":"m"}},
        "anchors":[{"id":"desk","name":"Desk","metric_position_m":{"value":{"x":1.25,"y":0.5,"z":0},"unit":"m"}}]
      }],
      "connections": [],
      "objects": [{
        "id":"box","name":"Box","anchor_id":"desk",
        "metric_bounds_m":{"x":{"value":0.4,"unit":"m"},"y":{"value":0.3,"unit":"m"},"z":{"value":0.2,"unit":"m"}}
      }]
    }"#;

    let (_legacy_temp, legacy_path) = write_manifest(legacy);
    let legacy_definition = WorldDefinition::load(&legacy_path, &PathGuard::default())
        .expect("legacy remains readable");
    assert!(legacy_definition.metric_bounds("room").is_none());
    assert!(legacy_definition.anchor_metric_position("desk").is_none());
    assert!(legacy_definition.object_metric_bounds("box").is_none());

    let (_metric_temp, metric_path) = write_manifest(metric);
    let definition = WorldDefinition::load(&metric_path, &PathGuard::default())
        .expect("metric package is valid");
    assert_eq!(
        definition
            .metric_bounds("room")
            .map(|bounds| bounds.x.value),
        Some(4.5)
    );
    assert_eq!(
        definition
            .anchor_metric_position("desk")
            .map(|position| position.value().x),
        Some(1.25)
    );
    assert_eq!(
        definition
            .object_metric_bounds("box")
            .map(|bounds| bounds.z.value),
        Some(0.2)
    );
}

#[test]
fn rejects_invalid_metric_units_or_dimensions() {
    let manifest = |bounds: &str| {
        format!(
            r#"{{
              "schema_version": 1,
              "world_id": "invalid-metric",
              "initial_anchor_id": "desk",
              "locations": [{{
                "id":"room","name":"Room",
                "metric_bounds_m":{bounds},
                "anchors":[{{"id":"desk","name":"Desk"}}]
              }}],
              "connections": [],
              "objects": []
            }}"#
        )
    };

    let cases = [
        (
            r#"{"x":{"value":-1,"unit":"m"},"y":{"value":1,"unit":"m"},"z":{"value":1,"unit":"m"}}"#,
            "metric",
        ),
        (
            r#"{"x":{"value":1,"unit":"cm"},"y":{"value":1,"unit":"m"},"z":{"value":1,"unit":"m"}}"#,
            "unknown variant",
        ),
    ];
    for (bounds, expected_error) in cases {
        let (_temp, path) = write_manifest(&manifest(bounds));
        let error = WorldDefinition::load(path, &PathGuard::default())
            .expect_err("invalid metric geometry is rejected");
        assert!(error.to_string().contains(expected_error));
    }
}

#[test]
fn metric_geometry_changes_the_semantic_definition_hash() {
    let template = |geometry: &str| {
        format!(
            r#"{{
              "schema_version": 1,
              "world_id": "hash-metric",
              "initial_anchor_id": "desk",
              "locations": [{{
                "id":"room","name":"Room",
                {geometry}
                "anchors":[{{"id":"desk","name":"Desk"}}]
              }}],
              "connections": [],
              "objects": []
            }}"#
        )
    };
    let empty = template("");
    let metric = template(
        r#""metric_bounds_m":{"x":{"value":1,"unit":"m"},"y":{"value":1,"unit":"m"},"z":{"value":1,"unit":"m"}},"#,
    );

    let (_first, first) = write_manifest(&empty);
    let (_second, second) = write_manifest(&metric);
    let empty_definition = WorldDefinition::load(first, &PathGuard::default()).unwrap();
    let metric_definition = WorldDefinition::load(second, &PathGuard::default()).unwrap();

    assert_ne!(empty_definition.hash(), metric_definition.hash());
}
