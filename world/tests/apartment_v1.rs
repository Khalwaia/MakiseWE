use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use makise_world::{
    ClockSample, CommandEnvelope, CommandPayload, CommandStatus, PathGuard, WorldDefinition,
    WorldEngine,
};

fn package_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../world-packages/apartment-v1")
}

fn manifest_path() -> PathBuf {
    package_dir().join("manifest.json")
}

fn definition() -> WorldDefinition {
    WorldDefinition::load(manifest_path(), &PathGuard::default()).unwrap()
}

#[test]
fn apartment_has_the_fixed_topology() {
    let definition = definition();
    let expected = BTreeMap::from([
        (
            "entryway",
            vec!["front_door", "coat_and_shoe_area", "hall_mirror"],
        ),
        ("corridor", vec!["corridor_center", "utility_cabinet"]),
        (
            "kitchen",
            vec![
                "sink",
                "kitchen_worktop",
                "stove",
                "fridge",
                "dining_table",
                "kitchen_window",
            ],
        ),
        (
            "living_room",
            vec![
                "sofa",
                "work_desk",
                "media_bookshelf",
                "living_room_center",
                "balcony_door",
            ],
        ),
        (
            "bedroom",
            vec!["bed", "wardrobe", "bedside_table", "bedroom_window"],
        ),
        (
            "bathroom",
            vec!["sink_mirror", "shower_bath", "toilet", "washing_machine"],
        ),
        (
            "balcony",
            vec!["balcony_table_chair", "balcony_windows", "balcony_storage"],
        ),
    ]);

    assert_eq!(definition.location_ids().count(), expected.len());
    assert_eq!(definition.anchor_ids().count(), 27);
    for (location, anchors) in expected {
        for anchor in anchors {
            assert_eq!(definition.location_for_anchor(anchor).unwrap().0, location);
        }
    }
    assert_eq!(definition.connections().count(), 64);
    assert_eq!(
        definition.movement_duration("bed", "bedside_table"),
        Some(700)
    );
    assert_eq!(
        definition.movement_duration("bedside_table", "bed"),
        Some(700)
    );
    assert!(
        definition
            .movement_duration("bed", "balcony_windows")
            .is_some()
    );
    assert_eq!(definition.movement_affordances("bed").len(), 26);
}

#[test]
fn apartment_has_fixed_inventory_without_coffee_machine() {
    let definition = definition();
    let object_ids = definition
        .objects()
        .map(|object| object.id)
        .collect::<Vec<_>>();

    assert_eq!(definition.template_ids().count(), 36);
    assert_eq!(object_ids.len(), 73);
    assert!(!object_ids.iter().any(|id| id.contains("coffee_machine")));
    assert!(
        !definition
            .template_ids()
            .any(|id| id.contains("coffee_machine"))
    );
}

#[test]
fn perception_covers_every_anchor_without_hidden_canaries() {
    let definition = definition();
    let anchors = definition
        .anchor_ids()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let temp = tempfile::tempdir().unwrap();
    let mut perceptions = String::new();

    for (index, anchor) in anchors.iter().enumerate() {
        let engine = WorldEngine::open(
            temp.path().join(format!("anchor-{index}.db")),
            &format!("test-makise-{index}"),
            definition.clone(),
            anchor,
            1_000_000,
            &PathGuard::default(),
        )
        .unwrap();
        let perception = engine.perception().unwrap();
        assert_eq!(&perception.anchor_id, anchor);
        assert!(!perception.environment_cues.is_empty());
        perceptions.push_str(&serde_json::to_string(&perception).unwrap());
    }

    assert!(perceptions.contains("metal_and_wood"));
    assert!(perceptions.contains("Погода (локальный fallback)"));
    assert!(!perceptions.contains("hidden-"));
    assert!(!perceptions.contains("canary_"));
}

#[test]
fn definition_hash_is_semantic_but_includes_the_map_asset() {
    let original = definition();
    let source = fs::read_to_string(manifest_path()).unwrap();
    let reordered = source.replacen(
        "{\n  \"schema_version\": 1,\n  \"world_id\": \"apartment-v1\",",
        "{\n  \"world_id\": \"apartment-v1\",\n  \"schema_version\": 1,",
        1,
    );
    assert_ne!(source, reordered);

    let temp = tempfile::tempdir().unwrap();
    let temp_manifest = temp.path().join("manifest.json");
    let temp_map = temp.path().join("map.svg");
    fs::write(&temp_manifest, reordered).unwrap();
    fs::copy(package_dir().join("map.svg"), &temp_map).unwrap();

    let reordered_definition =
        WorldDefinition::load(&temp_manifest, &PathGuard::default()).unwrap();
    assert_eq!(original.hash(), reordered_definition.hash());

    let mut map = fs::read_to_string(&temp_map).unwrap();
    map.push_str("\n<!-- hash sensitivity canary -->\n");
    fs::write(&temp_map, map).unwrap();
    let changed_map_definition =
        WorldDefinition::load(&temp_manifest, &PathGuard::default()).unwrap();
    assert_ne!(original.hash(), changed_map_definition.hash());
}

#[test]
fn closed_container_hides_contents_until_open_action_completes() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = WorldEngine::open(
        temp.path().join("dynamic-fridge.db"),
        "test-makise-dynamic-fridge",
        definition(),
        "fridge",
        2_000_000,
        &PathGuard::default(),
    )
    .unwrap();

    let initial = engine.perception().unwrap();
    let refrigerator = initial
        .observed_objects
        .iter()
        .find(|object| object.object_id == "object.refrigerator")
        .unwrap();
    assert_eq!(refrigerator.observed_properties.get("power").unwrap(), "on");
    assert_eq!(
        refrigerator.observed_properties.get("open").unwrap(),
        "closed"
    );
    assert_eq!(
        refrigerator
            .affordances
            .iter()
            .find(|action| action.action_id == "object.toggle_open")
            .unwrap()
            .description,
        "Открыть"
    );
    assert!(
        !initial
            .observed_objects
            .iter()
            .any(|object| object.object_id == "object.refrigerated_food")
    );

    let command = CommandEnvelope {
        command_id: "cmd-open-fridge".into(),
        identity_id: "test-makise-dynamic-fridge".into(),
        agent_id: "makise".into(),
        expected_world_version: engine.state().world_version(),
        schema_version: 1,
        decision_id: "decision-open-fridge".into(),
        issued_at_ms: 2_000_010,
        ttl_ms: 30_000,
        payload: CommandPayload::Perform {
            action_id: "object.toggle_open".into(),
            target_id: "object.refrigerator".into(),
            parameters: BTreeMap::new(),
        },
    };
    let result = engine.execute_command(&command, 2_000_010).unwrap();
    assert_eq!(result.status, CommandStatus::Committed);
    engine
        .tick(ClockSample {
            utc_ms: 2_000_510,
            monotonic_elapsed_ms: 500,
        })
        .unwrap();

    let opened = engine.perception().unwrap();
    let refrigerator = opened
        .observed_objects
        .iter()
        .find(|object| object.object_id == "object.refrigerator")
        .unwrap();
    assert_eq!(
        refrigerator.observed_properties.get("open").unwrap(),
        "open"
    );
    assert_eq!(
        refrigerator
            .affordances
            .iter()
            .find(|action| action.action_id == "object.toggle_open")
            .unwrap()
            .description,
        "Закрыть"
    );
    assert!(
        opened
            .observed_objects
            .iter()
            .any(|object| object.object_id == "object.refrigerated_food")
    );
    assert!(
        opened
            .environment_cues
            .iter()
            .any(|cue| cue.contains("холодный воздух"))
    );
    drop(engine);
    let recovered = WorldEngine::open(
        temp.path().join("dynamic-fridge.db"),
        "test-makise-dynamic-fridge",
        definition(),
        "fridge",
        2_001_000,
        &PathGuard::default(),
    )
    .unwrap();
    assert!(
        recovered
            .perception()
            .unwrap()
            .observed_objects
            .iter()
            .any(|object| object.object_id == "object.refrigerated_food")
    );
}

fn perform_command(
    engine: &WorldEngine,
    identity_id: &str,
    command_id: &str,
    now_ms: i64,
    action_id: &str,
    target_id: &str,
    parameters: &[(&str, &str)],
) -> CommandEnvelope {
    CommandEnvelope {
        command_id: command_id.into(),
        identity_id: identity_id.into(),
        agent_id: "makise".into(),
        expected_world_version: engine.state().world_version(),
        schema_version: 1,
        decision_id: format!("decision-{command_id}"),
        issued_at_ms: now_ms,
        ttl_ms: 30_000,
        payload: CommandPayload::Perform {
            action_id: action_id.into(),
            target_id: target_id.into(),
            parameters: parameters
                .iter()
                .map(|(key, value)| ((*key).into(), (*value).into()))
                .collect(),
        },
    }
}

fn assert_rejected_code(result: &makise_world::CommandResult, code: &str) {
    assert_eq!(result.status, CommandStatus::RejectedPrecondition);
    assert_eq!(result.error_code.as_deref(), Some(code));
}

#[test]
fn relocation_is_durable_and_closed_containers_hide_contents() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("relocation.db");
    let identity = "test-makise-relocation";
    let mut engine = WorldEngine::open(
        &database,
        identity,
        definition(),
        "work_desk",
        10_000_000,
        &PathGuard::default(),
    )
    .unwrap();

    let headphones = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.wired_headphones")
        .unwrap();
    let relocate = headphones
        .affordances
        .iter()
        .find(|action| action.action_id == "object.relocate")
        .unwrap();
    let schema: serde_json::Value = serde_json::from_str(&relocate.parameters_schema_json).unwrap();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let destination = [
        ("relation", "container"),
        ("anchor_id", "work_desk"),
        ("parent_object_id", "object.work_desk"),
        ("slot_id", "drawer"),
    ];
    let closed = perform_command(
        &engine,
        identity,
        "cmd-relocate-closed",
        10_000_010,
        "object.relocate",
        "object.wired_headphones",
        &destination,
    );
    let result = engine.execute_command(&closed, 10_000_010).unwrap();
    assert_rejected_code(&result, "DESTINATION_CLOSED");

    let open = perform_command(
        &engine,
        identity,
        "cmd-open-desk",
        10_000_020,
        "object.toggle_open",
        "object.work_desk",
        &[],
    );
    assert_eq!(
        engine.execute_command(&open, 10_000_020).unwrap().status,
        CommandStatus::Committed
    );
    engine
        .tick(ClockSample {
            utc_ms: 10_000_520,
            monotonic_elapsed_ms: 520,
        })
        .unwrap();

    let relocate = perform_command(
        &engine,
        identity,
        "cmd-relocate-headphones",
        10_000_530,
        "object.relocate",
        "object.wired_headphones",
        &destination,
    );
    assert_eq!(
        engine
            .execute_command(&relocate, 10_000_530)
            .unwrap()
            .status,
        CommandStatus::Committed
    );
    engine
        .tick(ClockSample {
            utc_ms: 10_001_730,
            monotonic_elapsed_ms: 1_210,
        })
        .unwrap();

    let moved = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.wired_headphones")
        .unwrap();
    assert_eq!(moved.observed_properties["placement_relation"], "container");
    assert_eq!(
        moved.observed_properties["parent_object_id"],
        "object.work_desk"
    );
    assert_eq!(moved.observed_properties["slot_id"], "drawer");

    let close = perform_command(
        &engine,
        identity,
        "cmd-close-desk",
        10_001_740,
        "object.toggle_open",
        "object.work_desk",
        &[],
    );
    engine.execute_command(&close, 10_001_740).unwrap();
    engine
        .tick(ClockSample {
            utc_ms: 10_002_240,
            monotonic_elapsed_ms: 510,
        })
        .unwrap();
    assert!(
        !engine
            .perception()
            .unwrap()
            .observed_objects
            .iter()
            .any(|object| object.object_id == "object.wired_headphones")
    );
    drop(engine);

    let mut recovered = WorldEngine::open(
        &database,
        identity,
        definition(),
        "work_desk",
        10_002_240,
        &PathGuard::default(),
    )
    .unwrap();
    assert!(
        !recovered
            .perception()
            .unwrap()
            .observed_objects
            .iter()
            .any(|object| object.object_id == "object.wired_headphones")
    );
    let reopen = perform_command(
        &recovered,
        identity,
        "cmd-reopen-desk",
        10_002_250,
        "object.toggle_open",
        "object.work_desk",
        &[],
    );
    recovered.execute_command(&reopen, 10_002_250).unwrap();
    recovered
        .tick(ClockSample {
            utc_ms: 10_002_750,
            monotonic_elapsed_ms: 510,
        })
        .unwrap();
    let replayed = recovered
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.wired_headphones")
        .unwrap();
    assert_eq!(replayed.observed_properties["slot_id"], "drawer");
}

#[test]
fn relocation_enforces_dimensions_power_and_cycles() {
    let temp = tempfile::tempdir().unwrap();
    let mut chair = WorldEngine::open(
        temp.path().join("chair.db"),
        "test-makise-chair",
        definition(),
        "dining_table",
        20_000_000,
        &PathGuard::default(),
    )
    .unwrap();
    let command = perform_command(
        &chair,
        "test-makise-chair",
        "cmd-chair-on-table",
        20_000_010,
        "object.relocate",
        "object.dining_chair.1",
        &[
            ("relation", "surface"),
            ("anchor_id", "dining_table"),
            ("parent_object_id", "object.dining_table"),
            ("slot_id", "top"),
        ],
    );
    assert_rejected_code(
        &chair.execute_command(&command, 20_000_010).unwrap(),
        "DOES_NOT_FIT",
    );

    let mut kettle = WorldEngine::open(
        temp.path().join("kettle.db"),
        "test-makise-kettle",
        definition(),
        "kitchen_worktop",
        21_000_000,
        &PathGuard::default(),
    )
    .unwrap();
    let open = perform_command(
        &kettle,
        "test-makise-kettle",
        "cmd-open-worktop",
        21_000_010,
        "object.toggle_open",
        "object.kitchen_worktop",
        &[],
    );
    kettle.execute_command(&open, 21_000_010).unwrap();
    kettle
        .tick(ClockSample {
            utc_ms: 21_000_510,
            monotonic_elapsed_ms: 510,
        })
        .unwrap();
    let command = perform_command(
        &kettle,
        "test-makise-kettle",
        "cmd-kettle-cabinet",
        21_000_520,
        "object.relocate",
        "object.electric_kettle",
        &[
            ("relation", "container"),
            ("anchor_id", "kitchen_worktop"),
            ("parent_object_id", "object.kitchen_worktop"),
            ("slot_id", "cabinet"),
        ],
    );
    assert_rejected_code(
        &kettle.execute_command(&command, 21_000_520).unwrap(),
        "POWER_REQUIRED",
    );

    let mut cycle = WorldEngine::open(
        temp.path().join("cycle.db"),
        "test-makise-cycle",
        definition(),
        "washing_machine",
        22_000_000,
        &PathGuard::default(),
    )
    .unwrap();
    let command = perform_command(
        &cycle,
        "test-makise-cycle",
        "cmd-basket-cycle",
        22_000_010,
        "object.relocate",
        "object.laundry_basket",
        &[
            ("relation", "container"),
            ("anchor_id", "washing_machine"),
            ("parent_object_id", "object.laundry_basket"),
            ("slot_id", "basket"),
        ],
    );
    assert_rejected_code(
        &cycle.execute_command(&command, 22_000_010).unwrap(),
        "PLACEMENT_CYCLE",
    );
}

#[test]
fn causal_conditions_are_visible_and_cleaning_is_durable() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("conditions.db");
    let identity = "test-makise-conditions";
    let mut engine = WorldEngine::open(
        &database,
        identity,
        definition(),
        "hall_mirror",
        30_000_000,
        &PathGuard::default(),
    )
    .unwrap();

    let mirror = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.hall_mirror")
        .unwrap();
    assert_eq!(mirror.observed_properties["cleanliness_permille"], "850");
    assert!(
        mirror
            .affordances
            .iter()
            .any(|action| action.action_id == "object.clean")
    );

    let clean = perform_command(
        &engine,
        identity,
        "cmd-clean-mirror",
        30_000_010,
        "object.clean",
        "object.hall_mirror",
        &[],
    );
    assert_eq!(
        engine.execute_command(&clean, 30_000_010).unwrap().status,
        CommandStatus::Committed
    );
    engine
        .tick(ClockSample {
            utc_ms: 30_020_010,
            monotonic_elapsed_ms: 20_010,
        })
        .unwrap();

    let cleaned = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.hall_mirror")
        .unwrap();
    assert_eq!(cleaned.observed_properties["cleanliness_permille"], "1000");
    assert!(
        cleaned
            .affordances
            .iter()
            .all(|action| action.action_id != "object.clean")
    );
    drop(engine);

    let recovered = WorldEngine::open(
        &database,
        identity,
        definition(),
        "hall_mirror",
        30_020_010,
        &PathGuard::default(),
    )
    .unwrap();
    let replayed = recovered
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.hall_mirror")
        .unwrap();
    assert_eq!(replayed.observed_properties["cleanliness_permille"], "1000");
}

#[test]
fn finite_quantity_consumption_is_checked_and_durable() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("quantity.db");
    let identity = "test-makise-quantity";
    let mut engine = WorldEngine::open(
        &database,
        identity,
        definition(),
        "kitchen_worktop",
        40_000_000,
        &PathGuard::default(),
    )
    .unwrap();

    let open = perform_command(
        &engine,
        identity,
        "cmd-open-pantry",
        40_000_010,
        "object.toggle_open",
        "object.kitchen_worktop",
        &[],
    );
    engine.execute_command(&open, 40_000_010).unwrap();
    engine
        .tick(ClockSample {
            utc_ms: 40_000_510,
            monotonic_elapsed_ms: 510,
        })
        .unwrap();

    let pantry = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.pantry_food")
        .unwrap();
    assert_eq!(pantry.observed_properties["quantity_amount"], "12");
    assert_eq!(pantry.observed_properties["quantity_unit"], "serving");
    let consume = pantry
        .affordances
        .iter()
        .find(|action| action.action_id == "object.consume_quantity")
        .unwrap();
    let schema: serde_json::Value = serde_json::from_str(&consume.parameters_schema_json).unwrap();
    assert_eq!(schema["required"][0], "amount");
    assert_eq!(schema["properties"]["amount"]["type"], "string");

    let consume = perform_command(
        &engine,
        identity,
        "cmd-consume-pantry",
        40_000_520,
        "object.consume_quantity",
        "object.pantry_food",
        &[("amount", "3")],
    );
    engine.execute_command(&consume, 40_000_520).unwrap();
    engine
        .tick(ClockSample {
            utc_ms: 40_001_520,
            monotonic_elapsed_ms: 1_010,
        })
        .unwrap();

    let pantry = engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.pantry_food")
        .unwrap();
    assert_eq!(pantry.observed_properties["quantity_amount"], "9");

    let excessive = perform_command(
        &engine,
        identity,
        "cmd-consume-too-much",
        40_001_530,
        "object.consume_quantity",
        "object.pantry_food",
        &[("amount", "10")],
    );
    assert_rejected_code(
        &engine.execute_command(&excessive, 40_001_530).unwrap(),
        "INSUFFICIENT_QUANTITY",
    );
    drop(engine);

    let recovered = WorldEngine::open(
        &database,
        identity,
        definition(),
        "kitchen_worktop",
        40_001_530,
        &PathGuard::default(),
    )
    .unwrap();
    let pantry = recovered
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.pantry_food")
        .unwrap();
    assert_eq!(pantry.observed_properties["quantity_amount"], "9");
}

#[test]
fn apartment_exposes_typed_charge_and_temperature() {
    let temp = tempfile::tempdir().unwrap();
    let phone_engine = WorldEngine::open(
        temp.path().join("phone.db"),
        "test-makise-phone-condition",
        definition(),
        "work_desk",
        50_000_000,
        &PathGuard::default(),
    )
    .unwrap();
    let phone = phone_engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.makise_phone")
        .unwrap();
    assert_eq!(phone.observed_properties["charge_permille"], "860");

    let fridge_engine = WorldEngine::open(
        temp.path().join("fridge-condition.db"),
        "test-makise-fridge-condition",
        definition(),
        "fridge",
        50_000_000,
        &PathGuard::default(),
    )
    .unwrap();
    let refrigerator = fridge_engine
        .perception()
        .unwrap()
        .observed_objects
        .into_iter()
        .find(|object| object.object_id == "object.refrigerator")
        .unwrap();
    assert_eq!(
        refrigerator.observed_properties["temperature_millicelsius"],
        "4000"
    );
}
