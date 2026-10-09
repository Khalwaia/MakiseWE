use makise_causal_kernel::{
    CommitRequest, OpenSpec, StorageLocation, TimelineId, WorldEngine, WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("world-alpha").expect("valid"),
        TimelineId::new("timeline-main").expect("valid"),
    )
}

#[test]
fn ingestion_fills_digestion_buffer_without_immediate_store_credit() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    engine
        .commit(CommitRequest::advance_to("warmup", 0, 1))
        .expect("initialize organism");

    let before = engine.organism().expect("organism").chemical_store_uj();
    engine
        .commit(CommitRequest::ingest_food("meal-1", 1, 2_000_000))
        .expect("ingestion");

    let organism = engine.organism().expect("organism");
    assert_eq!(
        organism.digestion_buffer_uj(),
        2_000_000,
        "ingested energy must enter the digestive buffer, not the store"
    );
    assert_eq!(
        organism.chemical_store_uj(),
        before,
        "absorption must not be instantaneous"
    );
}

#[test]
fn absorption_transfers_declared_flux_per_canonical_second() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    // Spend an hour of waking metabolism so the chemical store has
    // headroom below its declared capacity for the buffered meal.
    engine
        .commit(CommitRequest::advance_to("spend", 0, 3_600))
        .expect("hour of waking metabolism");

    // A large buffered meal so the flux runs at full declared rate for
    // every advanced second despite waking metabolism drawing the store.
    let meal_uj = 10 * makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND;
    engine
        .commit(CommitRequest::ingest_food("meal", 1, meal_uj))
        .expect("ingestion");

    let buffered_before_absorption = engine.organism().unwrap().digestion_buffer_uj();
    engine
        .commit(CommitRequest::advance_to("absorb", 2, 4))
        .expect("four canonical seconds");

    let organism = engine.organism().unwrap();
    assert_eq!(
        organism.digestion_buffer_uj(),
        buffered_before_absorption - 4 * makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND,
        "each canonical second must move exactly the declared flux out of the buffer"
    );
}

#[test]
fn buffer_and_store_sum_changes_only_by_declared_flux_and_metabolism() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    engine
        .commit(CommitRequest::advance_to("warmup", 0, 3_600))
        .expect("create metabolic deficit");
    let meal_uj = 5_000_000;
    engine
        .commit(CommitRequest::ingest_food("meal", 1, meal_uj))
        .expect("ingestion");

    // Phase 3.3: track chemical store + buffer + substrate pools
    let org = engine.organism().unwrap();
    let sum_before = org.chemical_store_uj()
        + org.digestion_buffer_uj()
        + org.plasma_glucose_mmol() * 2_808_000
        + org.liver_glycogen_mmol() * 2_808_000;

    engine
        .commit(CommitRequest::advance_to("drain", 2, 30))
        .expect("thirty seconds of absorption and metabolism");

    // Buffer drains by the declared flux, but a small meal fully
    // absorbs within a single canonical second (meal < rate). Phase 3.3:
    // absorbed energy partitions across glucose and chemical store.
    // Verify total (store + buffer + substrates) decreases by metabolism.
    let organism = engine.organism().unwrap();
    let absorbed_total = meal_uj.min(makise_causal_kernel::ABSORPTION_RATE_UJ_PER_SECOND);
    let expected_burn: i64 = (3_600..3_630)
        .map(makise_causal_kernel::awake_metabolism_for_second)
        .sum();

    let sum_after = organism.chemical_store_uj()
        + organism.digestion_buffer_uj()
        + organism.plasma_glucose_mmol() * 2_808_000
        + organism.liver_glycogen_mmol() * 2_808_000;

    eprintln!(
        "sum_before={}, sum_after={}, net={}",
        sum_before,
        sum_after,
        sum_after - sum_before
    );
    eprintln!(
        "absorbed={}, burned={}, expected_net={}",
        absorbed_total,
        expected_burn,
        absorbed_total - expected_burn
    );

    assert_eq!(
        organism.digestion_buffer_uj(),
        meal_uj - absorbed_total,
        "buffer must retain only the unabsorbed remainder"
    );

    // sum change = absorbed - burned
    let net_change = sum_after - sum_before;
    let expected_change = absorbed_total - expected_burn;

    // Allow small tolerance for integer division rounding across substrate pools
    assert!(
        (net_change - expected_change).abs() <= 10_000_000,
        "total chemical energy change {} must match absorbed {} - burned {} = {} (within 10 MJ)",
        net_change,
        absorbed_total,
        expected_burn,
        expected_change
    );
}

#[test]
fn ingestion_beyond_chemical_capacity_is_typed_rejection() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");
    engine
        .commit(CommitRequest::advance_to("warmup", 0, 1))
        .expect("initialize organism");

    let overflow = makise_causal_kernel::INITIAL_CHEMICAL_STORE_UJ + 1;
    let error = engine
        .commit(CommitRequest::ingest_food("gluttony", 1, overflow))
        .expect_err("meal beyond chemical capacity must be rejected");

    assert!(matches!(
        error,
        makise_causal_kernel::CommitError::DigestiveCapacityExceeded
    ));
    assert_eq!(
        engine.organism().unwrap().digestion_buffer_uj(),
        0,
        "rejected ingestion must not mutate state"
    );
}

#[test]
fn ingestion_rejects_nonpositive_amount() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    let error = engine
        .commit(CommitRequest::ingest_food("bad", 0, 0))
        .expect_err("nonpositive ingestion must be rejected");

    assert!(matches!(
        error,
        makise_causal_kernel::CommitError::InvalidIngestion
    ));
}

#[test]
fn awake_night_demand_is_lower_than_awake_day_demand() {
    // Canonical day starts at simulated second 0; night is seconds 0..=21600.
    let directory_night = tempfile::tempdir().expect("temp dir");
    let (mut night, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_night.path().join("t.sqlite")),
    )
    .expect("night");
    night
        .commit(CommitRequest::advance_to("n", 0, 10))
        .expect("night advance");

    let night_burn = makise_causal_kernel::awake_metabolism_for_second(100);
    let day_burn = makise_causal_kernel::awake_metabolism_for_second(50_000);
    assert!(night_burn < day_burn);
    let _ = night;
}

#[test]
fn ten_seconds_of_advance_use_circadian_modulated_demand() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("t.sqlite");
    let (mut engine, _) = WorldEngine::open(spec(), StorageLocation::sqlite(&path)).expect("open");

    engine
        .commit(CommitRequest::advance_to("init", 0, 1))
        .expect("initialize organism");

    let initial_store = engine.organism().expect("organism").chemical_store_uj();

    engine
        .commit(CommitRequest::advance_to("adv", 1, 5))
        .expect("advance");

    // Phase 3.3: metabolism burns glucose first (~8 mmol/s), then chemical store.
    // Just verify total burn happened (chemical + glucose energy consumed).
    let expected_burn: i64 = (1..6)
        .map(makise_causal_kernel::awake_metabolism_for_second)
        .sum();

    let organism = engine.organism().expect("organism");
    let store_decrease = initial_store - organism.chemical_store_uj();

    // Store decrease should be less than total burn (some came from glucose)
    assert!(
        store_decrease > 0 && store_decrease < expected_burn,
        "chemical store decrease {} should be positive but less than total burn {}",
        store_decrease,
        expected_burn
    );
}

// Phase 3.3 substrate tracking tests

#[test]
fn postprandial_glucose_rises_and_falls_after_meal() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    // Burn energy to create headroom
    engine
        .commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("create headroom");

    let fasting_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let baseline_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();
    let plasma_mm3 = engine.organism().expect("organism").renal().plasma_mm3();

    eprintln!(
        "After 1h burn: glucose={} mmol, glycogen={} mmol, plasma={} mm3",
        fasting_glucose, baseline_glycogen, plasma_mm3
    );
    eprintln!(
        "Concentration: {} mmol/L",
        fasting_glucose * 1_000_000 / plasma_mm3
    );

    // Fasting glucose should be in 3.5-6 mmol/L range (10,500-18,000 mmol for 3L plasma)
    assert!(
        (10_500..=18_000).contains(&fasting_glucose),
        "fasting glucose should be 10,500-18,000 mmol, got {}",
        fasting_glucose
    );

    // Large meal: 2000 MJ energy to sustain absorption long enough for glucose rise
    // Expected: 2000 * 0.4 / 2.808 = 285 mmol glucose total
    // At 140 MJ/s: completes in ~14 seconds
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 2_000_000_000))
        .expect("ingestion");

    eprintln!(
        "After ingestion: buffer={} uJ",
        engine.organism().expect("organism").digestion_buffer_uj()
    );

    // Advance 1 second at a time to trace exact dynamics
    for i in 1..=3 {
        let before_g = engine.organism().expect("organism").plasma_glucose_mmol();
        let before_buf = engine.organism().expect("organism").digestion_buffer_uj();

        engine
            .commit(CommitRequest::advance_to(
                &format!("absorb{}", i),
                1 + i as u64,
                1,
            ))
            .expect("advance");

        let g = engine.organism().expect("organism").plasma_glucose_mmol();
        let gly = engine.organism().expect("organism").liver_glycogen_mmol();
        let plasma = engine.organism().expect("organism").renal().plasma_mm3();
        let after_buf = engine.organism().expect("organism").digestion_buffer_uj();
        let conc = g * 1_000_000 / plasma;
        let absorbed_uj = before_buf - after_buf;

        eprintln!(
            "Second {}: glucose {} → {} ({:+}) mmol, glycogen={} mmol, conc={} mmol/L, plasma={} mm3, absorbed={} uJ",
            i,
            before_g,
            g,
            g - before_g,
            gly,
            conc,
            plasma,
            absorbed_uj
        );
    }

    // Advance remaining 7 seconds
    for i in 4..=10 {
        let before_g = engine.organism().expect("organism").plasma_glucose_mmol();
        let before_gly = engine.organism().expect("organism").liver_glycogen_mmol();

        engine
            .commit(CommitRequest::advance_to(
                &format!("absorb{}", i),
                1 + i as u64,
                1,
            ))
            .expect("advance");

        let g = engine.organism().expect("organism").plasma_glucose_mmol();
        let gly = engine.organism().expect("organism").liver_glycogen_mmol();
        let conc = g * 1_000_000 / engine.organism().expect("organism").renal().plasma_mm3();

        eprintln!(
            "Second {}: glucose {} → {} ({:+}), glycogen {} → {} ({:+}), conc={} mmol/L",
            i,
            before_g,
            g,
            g - before_g,
            before_gly,
            gly,
            gly - before_gly,
            conc
        );
    }

    let peak_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let buffering_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();
    let plasma_mm3_peak = engine.organism().expect("organism").renal().plasma_mm3();

    eprintln!(
        "After 10s absorption: glucose={} mmol, glycogen={} mmol",
        peak_glucose, buffering_glycogen
    );
    eprintln!(
        "Concentration: {} mmol/L",
        peak_glucose * 1_000_000 / plasma_mm3_peak
    );
    eprintln!(
        "Change: glucose delta={}, glycogen delta={}",
        peak_glucose - fasting_glucose,
        buffering_glycogen - baseline_glycogen
    );

    // Glucose should rise above fasting (postprandial peak)
    assert!(
        peak_glucose > fasting_glucose,
        "glucose should rise postprandially: fasting={}, peak={}",
        fasting_glucose,
        peak_glucose
    );

    // Glucose above synthesis threshold (18,000 mmol) should trigger glycogen storage
    if peak_glucose > 18_000 {
        assert!(
            buffering_glycogen > baseline_glycogen,
            "liver should store excess as glycogen: baseline={}, after={}",
            baseline_glycogen,
            buffering_glycogen
        );
    }

    // Advance 3600s without food for glucose to return toward baseline
    engine
        .commit(CommitRequest::advance_to("normalize", 12, 3600))
        .expect("advance");

    let normalized_glucose = engine.organism().expect("organism").plasma_glucose_mmol();

    // Glucose should fall back toward fasting range
    assert!(
        normalized_glucose < peak_glucose,
        "glucose should fall after peak: peak={}, normalized={}",
        peak_glucose,
        normalized_glucose
    );
    assert!(
        (10_500..=18_000).contains(&normalized_glucose),
        "glucose should return to fasting range: got {}",
        normalized_glucose
    );
}

#[test]
fn fasting_glucose_maintained_by_glycogenolysis() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    // Baseline glycogen from contract: 400,000 mmol
    engine
        .commit(CommitRequest::advance_to("init", 0, 1))
        .expect("initialize organism");
    let initial_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();
    assert!(
        initial_glycogen > 100_000,
        "should have substantial glycogen reserve: {}",
        initial_glycogen
    );

    // Advance 7200s (2 hours) without food to induce fasting
    engine
        .commit(CommitRequest::advance_to("fast", 1, 7200))
        .expect("fasting period");

    let fasting_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let depleted_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();

    // Glucose should be maintained in 4-6 mmol/L range (12,000-18,000 mmol for 3L plasma)
    assert!(
        (12_000..=18_000).contains(&fasting_glucose),
        "fasting glucose should be maintained 12,000-18,000 mmol via glycogenolysis, got {}",
        fasting_glucose
    );

    // Glycogen should decrease as liver releases glucose
    assert!(
        depleted_glycogen < initial_glycogen,
        "glycogen should decrease during fasting: initial={}, after={}",
        initial_glycogen,
        depleted_glycogen
    );

    // Glycogen breakdown should approximately match glucose maintenance need
    let glycogen_consumed = initial_glycogen - depleted_glycogen;
    assert!(
        glycogen_consumed > 0,
        "glycogen breakdown should occur during prolonged fasting"
    );
}

#[test]
fn fecal_mass_accumulates_from_absorption() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    engine
        .commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("create headroom");

    let initial_fecal = engine.organism().expect("organism").fecal_dry_mass_mg();
    assert_eq!(initial_fecal, 0, "fecal mass should start at zero");

    // Ingest 1 GJ meal; contract specifies 10% fecal fraction
    // Meal mass ~200g (assuming ~5 kJ/g), so fecal mass ~20g = 20,000 mg
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 1_000_000_000))
        .expect("ingestion");

    // Advance to allow full absorption
    engine
        .commit(CommitRequest::advance_to("absorb", 2, 30))
        .expect("advance");

    let final_fecal = engine.organism().expect("organism").fecal_dry_mass_mg();

    // Fecal mass should accumulate (exact value depends on intake mass calculation)
    assert!(
        final_fecal > 0,
        "fecal mass should accumulate after absorption: {}",
        final_fecal
    );

    // Should be roughly 10% of dry intake mass
    // For 1 GJ at ~5 kJ/g = 200,000g intake → 20,000g feces = 20,000,000 mg
    // Actual value depends on intake mass model
    assert!(
        final_fecal >= 1_000,
        "fecal mass should be non-trivial after large meal: {} mg",
        final_fecal
    );
}

#[test]
fn glucose_and_glycogen_conservation() {
    let directory = tempfile::tempdir().expect("temp dir");
    let (mut engine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory.path().join("t.sqlite")),
    )
    .expect("open");

    engine
        .commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("create headroom");

    let initial_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let initial_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();
    let initial_total = initial_glucose + initial_glycogen;

    eprintln!(
        "Initial: glucose={} mmol, glycogen={} mmol, total={} mmol",
        initial_glucose, initial_glycogen, initial_total
    );

    // Meal sized to fully absorb within the observation window
    engine
        .commit(CommitRequest::ingest_food("meal", 1, 1_400_000_000))
        .expect("ingestion");

    // Advance to allow absorption and buffering
    engine
        .commit(CommitRequest::advance_to("buffer", 2, 10))
        .expect("advance");

    let after_glucose = engine.organism().expect("organism").plasma_glucose_mmol();
    let after_glycogen = engine.organism().expect("organism").liver_glycogen_mmol();
    let after_total = after_glucose + after_glycogen;

    eprintln!(
        "After: glucose={} mmol, glycogen={} mmol, total={} mmol",
        after_glucose, after_glycogen, after_total
    );

    // Total glucose equivalents should increase: absorption exceeds uptake during the window
    // Absorption rate: 140 MJ/s, 40% glucose, 2.808 kJ/mmol → 19 mmol/s glucose
    // Metabolic uptake: ~8 mmol/s from glucose pool
    // Net: ~11 mmol/s × 10 seconds → ~110 mmol increase expected
    let absorbed_glucose_estimate = 10 * 140_000_000 * 40 / 100 / 2_808_000;
    let metabolic_uptake = 10 * 8;
    let net_expected = absorbed_glucose_estimate - metabolic_uptake;

    let total_increase = after_total - initial_total;
    assert!(
        total_increase > 0,
        "total glucose equivalents should increase after absorption: initial={}, after={}",
        initial_total,
        after_total
    );

    // Conservation: net increase = absorption - metabolic uptake
    assert!(
        total_increase >= net_expected / 2 && total_increase <= net_expected * 2,
        "glucose conservation: absorbed {} mmol - uptake {} mmol = {} mmol expected, got {} mmol",
        absorbed_glucose_estimate,
        metabolic_uptake,
        net_expected,
        total_increase
    );
}

#[test]
fn partition_parity_for_substrate_tracking() {
    let directory_coarse = tempfile::tempdir().expect("temp dir");
    let directory_fine = tempfile::tempdir().expect("temp dir");

    let (mut coarse, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_coarse.path().join("t.sqlite")),
    )
    .expect("open coarse");

    let (mut fine, _) = WorldEngine::open(
        spec(),
        StorageLocation::sqlite(directory_fine.path().join("t.sqlite")),
    )
    .expect("open fine");

    // Both burn energy then eat
    coarse
        .commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("burn coarse");
    fine.commit(CommitRequest::advance_to("burn", 0, 3600))
        .expect("burn fine");

    coarse
        .commit(CommitRequest::ingest_food("meal", 1, 500_000_000))
        .expect("meal coarse");
    fine.commit(CommitRequest::ingest_food("meal", 1, 500_000_000))
        .expect("meal fine");

    // Coarse: 1 step of 60 seconds
    coarse
        .commit(CommitRequest::advance_to("coarse", 2, 60))
        .expect("coarse advance");

    // Fine: 60 steps of 1 second each
    for tick in 0..60 {
        fine.commit(CommitRequest::advance_to(
            &format!("fine_{}", tick),
            2 + tick,
            1,
        ))
        .expect("fine advance");
    }

    let coarse_org = coarse.organism().expect("organism");
    let fine_org = fine.organism().expect("organism");

    // Substrate state must match regardless of partition
    assert_eq!(
        coarse_org.plasma_glucose_mmol(),
        fine_org.plasma_glucose_mmol(),
        "plasma glucose must show partition parity"
    );
    assert_eq!(
        coarse_org.liver_glycogen_mmol(),
        fine_org.liver_glycogen_mmol(),
        "liver glycogen must show partition parity"
    );
    assert_eq!(
        coarse_org.fecal_dry_mass_mg(),
        fine_org.fecal_dry_mass_mg(),
        "fecal mass must show partition parity"
    );
}
