//! Phase 3 slice 3.2 acceptance: coarse extracellular water/Na balance.
//! Synthetic contract scenario: 1.5 L water and 86.4 mmol sodium intake.
//! These balance bands are not independent empirical validation.

use makise_causal_kernel::{
    CommitRequest, Morphotype, OpenSpec, OrganismState, RenalError, StorageLocation, TimelineId,
    WorldId,
};

const DAY_SECONDS: usize = 86_400;
const WATER_INTAKE_MM3: i64 = 1_500_000;
const SODIUM_INTAKE_UMOL: i64 = 86_400;

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

#[test]
fn daily_fluid_and_sodium_balance_stay_in_declared_reference_bands() {
    let mut organism = OrganismState::physiological_baseline(&Morphotype::human());
    let opening_water = organism.total_body_water_mm3();
    organism
        .stage_fluid_intake(WATER_INTAKE_MM3, SODIUM_INTAKE_UMOL)
        .expect("declared intake is valid");
    for _ in 0..DAY_SECONDS {
        organism
            .apply_renal_for_second()
            .expect("baseline renal interval is valid");
    }

    assert!(
        (opening_water - 200_000..=opening_water + 200_000)
            .contains(&organism.total_body_water_mm3()),
        "24 h water balance must remain within 200 ml"
    );
    assert!(
        (135_000..=145_000).contains(&organism.plasma_sodium_mmol_per_l_milli()),
        "plasma Na must remain 135..145 mmol/L"
    );
    assert_eq!(
        opening_water + WATER_INTAKE_MM3,
        organism.total_body_water_mm3() + organism.urine_water_mm3(),
        "water must be conserved across body and urine boundary"
    );
}

#[test]
fn fluid_bolus_changes_map_through_the_blood_port() {
    let mut organism = OrganismState::physiological_baseline(&Morphotype::human());
    let before = organism.mean_arterial_pressure_mpa();
    organism
        .stage_fluid_intake(500_000, 0)
        .expect("500 ml bolus is inside the coarse envelope");
    let delta = organism.mean_arterial_pressure_mpa() - before;
    assert!(
        (3_000_000..=5_000_000).contains(&delta),
        "500 ml bolus MAP response must be 3..5 kPa, got {delta} mPa"
    );
}

#[test]
fn invalid_fluid_input_rejects_without_partial_application() {
    let mut organism = OrganismState::physiological_baseline(&Morphotype::human());
    let before = organism.clone();
    assert!(matches!(
        organism.stage_fluid_intake(-1, 0),
        Err(RenalError::InvalidInput)
    ));
    assert_eq!(
        organism, before,
        "invalid intake cannot partially mutate state"
    );
}

#[test]
fn out_of_envelope_fluid_input_rejects_without_partial_application() {
    let mut organism = OrganismState::physiological_baseline(&Morphotype::human());
    let before = organism.clone();
    assert!(matches!(
        organism.stage_fluid_intake(3_100_000, 0),
        Err(RenalError::OutsideValidityRange)
    ));
    assert_eq!(organism, before);
}

#[test]
fn fluid_commit_is_idempotent_and_invalid_input_has_no_durable_state() {
    let dir = tempfile::tempdir().expect("temp");
    let (mut engine, _) = makise_causal_kernel::WorldEngine::open(
        spec("renal-idempotency"),
        StorageLocation::sqlite(dir.path().join("renal.sqlite")),
    )
    .expect("open");
    let invalid = CommitRequest::ingest_fluid("invalid", 0, -1, 0);
    assert!(engine.commit(invalid).is_err());
    assert!(
        engine.organism().is_none(),
        "failed commit cannot persist state"
    );

    let request = CommitRequest::ingest_fluid("fluid", 0, WATER_INTAKE_MM3, SODIUM_INTAKE_UMOL);
    let first = engine.commit(request.clone()).expect("first commit");
    let retry = engine.commit(request).expect("idempotent retry");
    assert!(retry.replayed_request());
    assert_eq!(retry.timeline_version(), first.timeline_version());
}

#[test]
fn committed_fluid_input_survives_restart_and_canonical_partitioning() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("renal.sqlite");
    let (mut whole, _) = makise_causal_kernel::WorldEngine::open(
        spec("renal-whole"),
        StorageLocation::sqlite(&path),
    )
    .expect("open");
    whole
        .commit(CommitRequest::ingest_fluid(
            "fluid",
            0,
            WATER_INTAKE_MM3,
            SODIUM_INTAKE_UMOL,
        ))
        .expect("commit intake");
    whole
        .commit(CommitRequest::advance_to("day", 1, DAY_SECONDS as i64))
        .expect("advance day");
    let expected = whole.organism().expect("organism").clone();
    drop(whole);

    let (reopened, _) = makise_causal_kernel::WorldEngine::open(
        spec("renal-whole"),
        StorageLocation::sqlite(&path),
    )
    .expect("reopen");
    assert_eq!(reopened.organism(), Some(&expected));

    let dir = tempfile::tempdir().expect("temp");
    let (mut partitioned, _) = makise_causal_kernel::WorldEngine::open(
        spec("renal-parts"),
        StorageLocation::sqlite(dir.path().join("parts.sqlite")),
    )
    .expect("open partitioned");
    partitioned
        .commit(CommitRequest::ingest_fluid(
            "fluid",
            0,
            WATER_INTAKE_MM3,
            SODIUM_INTAKE_UMOL,
        ))
        .expect("commit intake");
    for hour in 0..24 {
        partitioned
            .commit(CommitRequest::advance_to(
                &format!("h{hour}"),
                hour + 1,
                3_600,
            ))
            .expect("advance hour");
    }
    assert_eq!(partitioned.organism(), Some(&expected));
}
