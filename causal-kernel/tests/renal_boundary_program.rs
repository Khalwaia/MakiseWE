use makise_causal_kernel::AdmissionError;
use makise_causal_kernel::{ArtifactBundle, BloodState, MechanismContract, ProgramAbi, RenalState};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const INTAKE_CONTRACT: &[u8] =
    include_bytes!("../../contracts/fixtures/mechanisms/renal-intake-v1.json");
const BLOOD_CONTRACT: &[u8] =
    include_bytes!("../../contracts/fixtures/mechanisms/renal-blood-volume-v1.json");

fn edited_bundle(mut contract: Value, program: Value, abi: ProgramAbi) -> ArtifactBundle {
    let program = serde_json::to_vec(&program).unwrap();
    contract["content_digest"] = format!("sha256:{:x}", Sha256::digest(&program)).into();
    ArtifactBundle::new(
        MechanismContract::from_json(&serde_json::to_vec(&contract).unwrap()).unwrap(),
        program,
        abi,
    )
}

fn intake_bundle() -> ArtifactBundle {
    ArtifactBundle::new(
        MechanismContract::from_json(include_bytes!(
            "../../contracts/fixtures/mechanisms/renal-intake-v1.json"
        ))
        .unwrap(),
        include_bytes!("../../contracts/fixtures/mechanisms/renal-intake-v1.program.json").to_vec(),
        ProgramAbi::RenalIntakeV1,
    )
}

fn coupling_bundle() -> ArtifactBundle {
    ArtifactBundle::new(
        MechanismContract::from_json(include_bytes!(
            "../../contracts/fixtures/mechanisms/renal-blood-volume-v1.json"
        ))
        .unwrap(),
        include_bytes!("../../contracts/fixtures/mechanisms/renal-blood-volume-v1.program.json")
            .to_vec(),
        ProgramAbi::RenalBloodVolumeV1,
    )
}

#[test]
fn archived_volume_coupling_preserves_gas_pools_and_tracks_intake_and_excretion() {
    let blood = BloodState::new(5_000_000, 10_000, 39_200, 24_000, 12_000_000, 300).unwrap();
    let artifact = coupling_bundle();
    let intake = artifact.propose_blood_volume(&blood, 250_000).unwrap();
    assert_eq!(
        intake,
        BloodState::new(5_250_000, 10_000, 39_200, 24_000, 14_000_000, 300).unwrap()
    );
    let excretion = artifact.propose_blood_volume(&intake, -17).unwrap();
    assert_eq!(
        excretion,
        BloodState::new(5_249_983, 10_000, 39_200, 24_000, 13_999_864, 300).unwrap()
    );
    assert_eq!(blood.blood_volume_mm3(), 5_000_000);
    assert_eq!(artifact.propose_blood_volume(&blood, 0).unwrap(), blood);
}

#[test]
fn archived_intake_transfers_water_and_sodium_without_mutating_its_input() {
    let state = RenalState::baseline();
    let next = intake_bundle()
        .propose_renal_intake(&state, 250_000, 35_000)
        .unwrap();
    // ADR-0016: 250 ml and 35 mmol enter the declared plasma boundary.
    assert_eq!(
        next,
        RenalState::new(42_250_000, 3_250_000, 455_000, 0, 0).unwrap()
    );
    assert_eq!(state, RenalState::baseline());
}

#[test]
fn archived_intake_budgets_control_admission_of_boundary_inputs() {
    let mut contract: Value = serde_json::from_slice(INTAKE_CONTRACT).unwrap();
    contract["parameters"][0]["value"] = 250_000.into();
    contract["parameters"][1]["value"] = 35_000.into();
    let limited = edited_bundle(
        contract,
        json!({"abi": "renal-intake-v1", "max_water_intake_mm3": 250_000,
               "max_sodium_intake_umol": 35_000}),
        ProgramAbi::RenalIntakeV1,
    );
    let baseline = RenalState::baseline();
    assert!(
        limited
            .propose_renal_intake(&baseline, 250_000, 35_000)
            .is_ok()
    );
    for (water, sodium) in [(250_001, 0), (0, 35_001)] {
        assert_eq!(
            limited.propose_renal_intake(&baseline, water, sodium),
            Err(AdmissionError::InvalidRenalProposal)
        );
        assert!(
            intake_bundle()
                .propose_renal_intake(&baseline, water, sodium)
                .is_ok()
        );
    }
}

#[test]
fn invalid_intakes_and_blood_transfers_leave_the_input_unchanged() {
    let renal = RenalState::new(45_000_000, 5_000_000, 510_000, 100, 10).unwrap();
    let before = renal.clone();
    for (water, sodium) in [
        (-1, 1),
        (1, -1),
        (0, 0),
        (1, 0),
        (0, 1),
        (i64::MAX, 0),
        (0, i64::MAX),
    ] {
        assert_eq!(
            intake_bundle().propose_renal_intake(&renal, water, sodium),
            Err(AdmissionError::InvalidRenalProposal)
        );
        assert_eq!(renal, before);
    }
    // Water and sodium may enter independently; urine remains a boundary ledger.
    let baseline = RenalState::new(42_000_000, 3_000_000, 420_000, 100, 10).unwrap();
    assert_eq!(
        intake_bundle()
            .propose_renal_intake(&baseline, 1, 0)
            .unwrap(),
        RenalState::new(42_000_001, 3_000_001, 420_000, 100, 10).unwrap()
    );
    assert_eq!(
        intake_bundle()
            .propose_renal_intake(&baseline, 0, 1)
            .unwrap(),
        RenalState::new(42_000_000, 3_000_000, 420_001, 100, 10).unwrap()
    );

    for (volume, pressure, delta) in [
        (5_000_000, 12_000_000, i64::MAX),
        (5_000_000, 12_000_000, i64::MIN),
        (i64::MAX, 12_000_000, 1),
        (5_000_000, i64::MAX, 1),
        (1, 12_000_000, -1),
        (5_000_000, 8, -1),
    ] {
        let blood = BloodState::new(volume, 10_000, 39_200, 24_000, pressure, 300).unwrap();
        let before = blood.clone();
        assert_eq!(
            coupling_bundle().propose_blood_volume(&blood, delta),
            Err(AdmissionError::InvalidBloodVolumeProposal)
        );
        assert_eq!(blood, before);
    }
}

#[test]
fn boundary_programs_reject_unknown_or_mismatched_executable_contracts() {
    for (contract_bytes, artifact, abi) in [
        (INTAKE_CONTRACT, intake_bundle(), ProgramAbi::RenalIntakeV1),
        (
            BLOOD_CONTRACT,
            coupling_bundle(),
            ProgramAbi::RenalBloodVolumeV1,
        ),
    ] {
        let contract: Value = serde_json::from_slice(contract_bytes).unwrap();
        let program: Value = serde_json::from_slice(artifact.program_bytes()).unwrap();
        assert_eq!(*artifact.admit().unwrap().program_abi(), abi);
        let mut corrupted = artifact.clone();
        corrupted.mutate_last_program_byte();
        assert!(matches!(
            corrupted.admit(),
            Err(AdmissionError::ProgramDigestMismatch)
        ));

        for field in program.as_object().unwrap().keys() {
            let mut missing = program.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(matches!(
                edited_bundle(contract.clone(), missing, abi).admit(),
                Err(AdmissionError::UnsupportedProgramAbi)
            ));
        }
        for (field, value) in [("unknown", json!(true)), ("abi", json!("future-v9"))] {
            let mut altered = program.clone();
            altered[field] = value;
            assert!(matches!(
                edited_bundle(contract.clone(), altered, abi).admit(),
                Err(AdmissionError::UnsupportedProgramAbi)
            ));
        }
        for field in program
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| *key != "abi")
        {
            for value in [json!(0), json!(-1), json!(i64::MAX), json!(1.5), json!("8")] {
                let mut altered = program.clone();
                altered[field] = value;
                assert!(matches!(
                    edited_bundle(contract.clone(), altered, abi).admit(),
                    Err(AdmissionError::UnsupportedProgramAbi)
                ));
            }
        }
        for pointer in [
            "/parameters/0/value",
            "/parameters/0/unit",
            "/write_set/0",
            "/authoritative_state_variables/0/validity_range/max",
            "/mechanism_id",
        ] {
            let mut altered = contract.clone();
            *altered.pointer_mut(pointer).unwrap() = match pointer {
                "/parameters/0/value" => json!(7),
                "/authoritative_state_variables/0/validity_range/max" => json!(1),
                _ => json!("wrong"),
            };
            assert!(matches!(
                edited_bundle(altered, program.clone(), abi).admit(),
                Err(AdmissionError::InvalidContract)
            ));
        }
        let mut incomplete = contract.clone();
        incomplete
            .as_object_mut()
            .unwrap()
            .remove("uncertainty_model");
        assert!(matches!(
            edited_bundle(incomplete, program.clone(), abi).admit(),
            Err(AdmissionError::InvalidContract)
        ));
        assert!(matches!(
            edited_bundle(contract, program, ProgramAbi::RenalFluidV1).admit(),
            Err(AdmissionError::UnsupportedProgramAbi)
        ));
    }
}

#[test]
fn mixed_renal_proposals_match_the_sixty_second_adr_anchor_from_exact_program_bytes() {
    fn run(reload_each_second: bool) -> (RenalState, BloodState) {
        let program = include_bytes!(
            "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.program.json"
        );
        let load_excretion = || {
            ArtifactBundle::new(
                MechanismContract::from_json(include_bytes!(
                    "../../contracts/fixtures/mechanisms/renal-corrective-excretion-v1.json"
                ))
                .unwrap(),
                program.to_vec(),
                ProgramAbi::RenalFluidV1,
            )
        };
        let mut excretion = load_excretion();
        let mut coupling = coupling_bundle();
        let mut renal = intake_bundle()
            .propose_renal_intake(&RenalState::baseline(), 250_000, 35_000)
            .unwrap();
        let mut blood = coupling
            .propose_blood_volume(
                &BloodState::new(5_000_000, 10_000, 39_200, 24_000, 12_000_000, 300).unwrap(),
                250_000,
            )
            .unwrap();
        for _ in 0..60 {
            if reload_each_second {
                excretion = load_excretion();
                coupling = coupling_bundle();
            }
            let next = excretion.propose_renal_second(&renal).unwrap();
            blood = coupling
                .propose_blood_volume(&blood, next.plasma_mm3() - renal.plasma_mm3())
                .unwrap();
            renal = next;
        }
        renal = intake_bundle()
            .propose_renal_intake(&renal, 250_000, 35_000)
            .unwrap();
        blood = coupling.propose_blood_volume(&blood, 250_000).unwrap();
        (renal, blood)
    }
    let expected = (
        RenalState::new(42_498_980, 3_498_980, 489_940, 1_020, 60).unwrap(),
        BloodState::new(5_498_980, 10_000, 39_200, 24_000, 15_991_840, 300).unwrap(),
    );
    let actual = run(false);
    assert_eq!(actual, expected);
    assert_eq!(run(true), expected);
    assert_eq!(
        actual.0.total_body_water_mm3() + actual.0.urine_water_mm3(),
        42_500_000
    );
    assert_eq!(
        actual.0.plasma_sodium_umol() + actual.0.urine_sodium_umol(),
        490_000
    );
}

#[test]
fn proposal_dependencies_cannot_activate_through_the_legacy_thermal_commit() {
    use makise_causal_kernel::{
        CommitError, CommitRequest, EventCursor, EventQuery, OpenSpec, ProjectionRequest,
        ReservoirPair, ReservoirState, StorageLocation, TimelineId, WorldEngine, WorldId,
    };
    let directory = tempfile::tempdir().unwrap();
    let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
    let spec = OpenSpec::new(
        WorldId::new("boundary").unwrap(),
        TimelineId::new("boundary").unwrap(),
    );
    let (mut engine, _) = WorldEngine::open(spec.clone(), storage.clone()).unwrap();
    let before = engine.project(ProjectionRequest::current()).unwrap();
    let pair = ReservoirPair::new(
        ReservoirState::new(20_000_000, 10),
        ReservoirState::new(10_000_000, 10),
    );
    for dependency in [intake_bundle(), coupling_bundle()] {
        assert!(matches!(
            engine.commit(CommitRequest::thermal_exchange(
                "rejected",
                0,
                pair.clone(),
                dependency
            )),
            Err(CommitError::ProposalRejected(_))
        ));
    }
    assert_eq!(
        engine.project(ProjectionRequest::current()).unwrap(),
        before
    );
    drop(engine);
    let (mut engine, _) = WorldEngine::open(spec, storage).unwrap();
    assert_eq!(
        engine.project(ProjectionRequest::current()).unwrap(),
        before
    );
    assert!(
        engine
            .events(EventQuery::new(EventCursor::start(), 10).unwrap())
            .unwrap()
            .events()
            .is_empty()
    );
    // Same request id can still be used: rejection did not publish a receipt.
    let receipt = engine
        .commit(CommitRequest::ingest_fluid("rejected", 0, 250_000, 35_000))
        .unwrap();
    assert!(!receipt.replayed_request());
    assert_eq!(receipt.timeline_version(), 1);
    assert_eq!(
        engine.fast_replay().unwrap(),
        engine.audit_replay().unwrap()
    );
}
