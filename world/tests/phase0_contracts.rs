use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("world crate must be in the repository")
        .to_path_buf()
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn schema_path(schema_version: &str) -> &'static str {
    match schema_version {
        "makise.mechanism-contract.v1" => "contracts/schemas/mechanism-contract-v1.schema.json",
        "makise.resolution-contract.v1" => "contracts/schemas/resolution-contract-v1.schema.json",
        "makise.morphotype-definition.v1" => {
            "contracts/schemas/morphotype-definition-v1.schema.json"
        }
        "makise.cognitive-decision.v1" => "contracts/schemas/cognitive-decision-v1.schema.json",
        unknown => panic!("fixture index references unknown schema version {unknown}"),
    }
}

fn assert_valid(schema: &Value, instance: &Value, label: &str) {
    let validator = jsonschema::validator_for(schema)
        .unwrap_or_else(|error| panic!("compile schema for {label}: {error}"));
    let errors = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "{label} failed schema validation: {errors:#?}"
    );
}

#[test]
fn every_contract_schema_compiles_and_every_indexed_fixture_validates() {
    let root = repo_root();
    let schema_dir = root.join("contracts/schemas");
    for entry in fs::read_dir(&schema_dir).expect("read contract schema directory") {
        let path = entry.expect("read schema entry").path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            let schema = read_json(&path);
            jsonschema::validator_for(&schema)
                .unwrap_or_else(|error| panic!("compile {}: {error}", path.display()));
        }
    }

    let index = read_json(&root.join("contracts/fixtures/index.json"));
    for (schema_version, paths) in index.as_object().expect("fixture index object") {
        let schema = read_json(&root.join(schema_path(schema_version)));
        for relative_path in paths.as_array().expect("fixture path array") {
            let relative_path = relative_path.as_str().expect("fixture path string");
            let fixture = read_json(&root.join(relative_path));
            assert_valid(&schema, &fixture, relative_path);
        }
    }
}

#[test]
fn morphotypes_are_independent_data_roots_with_required_anatomy() {
    let root = repo_root();
    let human = read_json(&root.join("contracts/fixtures/morphotypes/human-minimal.json"));
    let neko = read_json(&root.join("contracts/fixtures/morphotypes/neko-minimal.json"));

    assert_eq!(human["root_definition"], true);
    assert_eq!(neko["root_definition"], true);
    assert_ne!(human["morphotype_id"], neko["morphotype_id"]);
    for definition in [&human, &neko] {
        assert!(definition.get("extends").is_none());
        assert!(definition.get("is_neko").is_none());
    }

    let human_kinds = anatomy_kinds(&human);
    let neko_kinds = anatomy_kinds(&neko);
    assert!(human_kinds.contains("human-auricle-pair"));
    assert!(!human_kinds.iter().any(|kind| kind.contains("cat-auricle")));
    assert!(neko_kinds.contains("cat-auricle-pair"));
    for required in [
        "caudal-vertebral-chain",
        "caudal-muscle-group",
        "caudal-vasculature",
        "caudal-innervation",
    ] {
        assert!(neko_kinds.contains(required), "Neko is missing {required}");
    }

    let neko_mechanisms = neko["organ_bindings"]
        .as_array()
        .expect("Neko bindings")
        .iter()
        .map(|binding| binding["mechanism_id"].as_str().expect("mechanism ID"))
        .collect::<BTreeSet<_>>();
    for required in [
        "neko.hearing-transfer",
        "neko.balance-tail-coupling",
        "neko.auricle-thermoregulation",
    ] {
        assert!(
            neko_mechanisms.contains(required),
            "Neko is missing data-defined binding {required}"
        );
    }

    let human_phenotypes = human["phenotypes"].as_array().expect("Human phenotypes");
    assert!(human_phenotypes.iter().any(|phenotype| {
        phenotype["phenotype_id"] == "female-makise-v1" && phenotype["sex"] == "female"
    }));
}

fn anatomy_kinds(definition: &Value) -> BTreeSet<&str> {
    definition["anatomy_graph"]["nodes"]
        .as_array()
        .expect("anatomy nodes")
        .iter()
        .map(|node| node["kind"].as_str().expect("anatomy kind"))
        .collect()
}

#[test]
fn resolution_examples_conserve_quantities_and_observables() {
    let root = repo_root();
    for relative_path in [
        "contracts/fixtures/resolutions/cohort-to-individual-cell.json",
        "contracts/fixtures/resolutions/population-to-individual-neuron.json",
    ] {
        let fixture = read_json(&root.join(relative_path));
        let example = &fixture["validation_example"];
        assert!(
            !example["lineage"]["fine_entity_ids"]
                .as_array()
                .expect("fine lineage IDs")
                .is_empty()
        );
        assert!(
            example["lineage"]["projection_provenance_digest"]
                .as_str()
                .expect("projection provenance")
                .starts_with("sha256:")
        );
        let count_key = if example["coarse_state"].get("cell-count").is_some() {
            "cell-count"
        } else {
            "neuron-count"
        };
        assert_eq!(
            example["lineage"]["fine_entity_ids"]
                .as_array()
                .expect("fine lineage IDs")
                .len() as u64,
            example["coarse_state"][count_key]
                .as_u64()
                .expect("integral represented entity count"),
            "{relative_path} must retain one lineage ID per refined entity"
        );

        let declared_conservation = fixture["conserved_quantities"]
            .as_array()
            .expect("conserved quantities")
            .iter()
            .map(|item| {
                (
                    item["quantity_id"].as_str().expect("quantity ID"),
                    item["tolerance"].as_f64().expect("tolerance"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_checks(
            example,
            example["conservation_checks"]
                .as_array()
                .expect("conservation checks"),
            &declared_conservation,
            relative_path,
        );

        let declared_observables = fixture["observable_continuity_rules"]
            .as_array()
            .expect("observable continuity")
            .iter()
            .map(|item| {
                (
                    item["observable_id"].as_str().expect("observable ID"),
                    item["absolute_error_bound"].as_f64().expect("error bound"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_checks(
            example,
            example["observable_checks"]
                .as_array()
                .expect("observable checks"),
            &declared_observables,
            relative_path,
        );
    }
}

fn assert_checks(example: &Value, checks: &[Value], declared: &BTreeMap<&str, f64>, label: &str) {
    assert_eq!(
        checks
            .iter()
            .map(|check| check["quantity_id"].as_str().expect("check quantity"))
            .collect::<BTreeSet<_>>(),
        declared.keys().copied().collect::<BTreeSet<_>>(),
        "{label} does not check every declared quantity"
    );
    for check in checks {
        let id = check["quantity_id"].as_str().expect("check quantity ID");
        let coarse = check["coarse_value"].as_f64().expect("coarse value");
        let fine = check["fine_value"].as_f64().expect("fine value");
        let projected = check["projected_value"].as_f64().expect("projected value");
        let example_bound = check["absolute_error_bound"]
            .as_f64()
            .expect("example bound");
        let contract_bound = declared[id];
        assert_eq!(
            coarse,
            example["coarse_state"][id]
                .as_f64()
                .expect("coarse state quantity"),
            "{label} check is detached from coarse state for {id}"
        );
        assert_eq!(
            fine,
            example["fine_state"][id]
                .as_f64()
                .expect("fine state quantity"),
            "{label} check is detached from fine state for {id}"
        );
        assert_eq!(
            projected,
            example["projected_state"][id]
                .as_f64()
                .expect("projected state quantity"),
            "{label} check is detached from projected state for {id}"
        );
        assert!(
            example_bound <= contract_bound,
            "{label} weakens bound for {id}"
        );
        assert!(
            (coarse - fine).abs() <= contract_bound,
            "{label} lift violates {id} conservation/continuity"
        );
        assert!(
            (coarse - projected).abs() <= contract_bound,
            "{label} round trip violates {id} conservation/continuity"
        );
    }
}

#[test]
fn cognitive_disposition_controls_adoption_and_proposal_has_no_state_delta() {
    let root = repo_root();
    let mut seen = BTreeSet::new();
    for name in ["accepted", "rejected", "deferred"] {
        let fixture = read_json(&root.join(format!("contracts/fixtures/cognition/{name}.json")));
        let proposal = &fixture["proposal"];
        let disposition = &fixture["disposition"];
        assert_eq!(proposal["proposal_id"], disposition["proposal_id"]);
        assert!(proposal.get("physical_state_delta").is_none());
        assert!(proposal.get("biological_state_delta").is_none());
        let status = disposition["status"].as_str().expect("disposition status");
        seen.insert(status.to_owned());
        if status == "accepted" {
            let transition = fixture["applied_state_transition"]
                .as_object()
                .expect("accepted proposal must have a separate transition");
            assert_eq!(
                transition["caused_by_disposition_id"],
                disposition["disposition_id"]
            );
        } else {
            assert!(
                fixture["applied_state_transition"].is_null(),
                "{status} proposal must not become cognitive state"
            );
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            "accepted".to_owned(),
            "deferred".to_owned(),
            "rejected".to_owned(),
        ])
    );
}

#[test]
fn schemas_reject_hidden_fidelity_scores_morphotype_inheritance_and_llm_mutation() {
    let root = repo_root();

    let mechanism_schema =
        read_json(&root.join("contracts/schemas/mechanism-contract-v1.schema.json"));
    let mut mechanism =
        read_json(&root.join("contracts/fixtures/mechanisms/minimal-mammalian-transport.json"));
    mechanism["authoritative_state_variables"][0]["dimension_kind"] =
        Value::String("normalized_score".to_owned());
    assert!(
        !jsonschema::validator_for(&mechanism_schema)
            .expect("mechanism schema")
            .is_valid(&mechanism)
    );

    let morphotype_schema =
        read_json(&root.join("contracts/schemas/morphotype-definition-v1.schema.json"));
    let mut neko = read_json(&root.join("contracts/fixtures/morphotypes/neko-minimal.json"));
    neko["extends"] = Value::String("human-v1".to_owned());
    assert!(
        !jsonschema::validator_for(&morphotype_schema)
            .expect("morphotype schema")
            .is_valid(&neko)
    );

    let proposal_schema = read_json(&root.join("contracts/schemas/cortex-proposal-v1.schema.json"));
    let decision = read_json(&root.join("contracts/fixtures/cognition/accepted.json"));
    let mut proposal = decision["proposal"].clone();
    proposal["physical_state_delta"] = json!({ "body.temperature": 310.15 });
    assert!(
        !jsonschema::validator_for(&proposal_schema)
            .expect("proposal schema")
            .is_valid(&proposal)
    );

    let decision_schema =
        read_json(&root.join("contracts/schemas/cognitive-decision-v1.schema.json"));
    let mut rejected = read_json(&root.join("contracts/fixtures/cognition/rejected.json"));
    rejected["applied_state_transition"] = decision["applied_state_transition"].clone();
    assert!(
        !jsonschema::validator_for(&decision_schema)
            .expect("decision schema")
            .is_valid(&rejected)
    );
}

#[test]
fn runtime_has_no_known_morphotype_branch() {
    let source_root = repo_root().join("world/src");
    let mut rust_files = Vec::new();
    collect_files(&source_root, "rs", &mut rust_files);
    for path in rust_files {
        let source = fs::read_to_string(&path).expect("read Rust source");
        for forbidden in ["is_neko", "Morphotype::Human", "Morphotype::Neko"] {
            assert!(
                !source.contains(forbidden),
                "{} contains forbidden morphotype branch {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn local_markdown_links_resolve() {
    let root = repo_root();
    let mut markdown_files = Vec::new();
    collect_files(&root, "md", &mut markdown_files);
    for path in markdown_files {
        if path.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git" | ".agents" | "target")
            )
        }) {
            continue;
        }
        let markdown = fs::read_to_string(&path).expect("read Markdown");
        for target in markdown_link_targets(&markdown) {
            if target.is_empty()
                || target.starts_with('#')
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let target = target
                .trim_matches(['<', '>'])
                .split('#')
                .next()
                .expect("link path");
            let resolved = if target.starts_with('/') {
                PathBuf::from(target)
            } else {
                path.parent().expect("Markdown parent").join(target)
            };
            assert!(
                resolved.exists(),
                "{} has broken local link {target}",
                path.display()
            );
        }
    }
}

fn collect_files(directory: &Path, extension: &str, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read directory") {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            if !path.ends_with(".git") && !path.ends_with("target") {
                collect_files(&path, extension, output);
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            output.push(path);
        }
    }
}

fn markdown_link_targets(markdown: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remaining = markdown;
    while let Some(start) = remaining.find("](") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find(')') else {
            break;
        };
        targets.push(&remaining[..end]);
        remaining = &remaining[end + 1..];
    }
    targets
}
