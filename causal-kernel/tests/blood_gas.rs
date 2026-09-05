use makise_causal_kernel::{
    CommitRequest, Morphotype, OpenSpec, OrganismState, StorageLocation, TimelineId, WorldId,
};

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

#[test]
fn blood_gas_resting_saturation_and_conservation() {
    let mut org = OrganismState::physiological_baseline(&Morphotype::human());
    let cap = org.blood_o2_capacity_umol();
    assert!(cap > 40_000, "human O2 capacity {cap} umol must be >40k");
    let sat = org.arterial_saturation_permille();
    assert!(
        (950..=1000).contains(&sat),
        "resting arterial saturation {sat} permille not in 950..1000"
    );
    let o2_before = org.arterial_o2_umol();
    let co2_before = org.venous_co2_umol();
    let total_before = org.total_accounted_uj() + org.ambient_internal_energy_uj();
    // One second of gas exchange at awake demand should keep saturation in band and conserve.
    org.apply_gas_exchange_for_second(makise_causal_kernel::AWAKE_METABOLISM_UJ_PER_SECOND)
        .expect("gas exchange at rest must succeed");
    let sat2 = org.arterial_saturation_permille();
    assert!(
        (950..=1000).contains(&sat2),
        "post-exchange saturation {sat2} permille not in 950..1000"
    );
    let o2_after = org.arterial_o2_umol();
    let co2_after = org.venous_co2_umol();
    // O2 consumed, CO2 produced; ventilation replenishes quickly -> net drift small
    assert!(o2_after < o2_before + 5_000, "O2 should not grow unbounded");
    assert!(
        co2_after >= co2_before - 500 && co2_after <= co2_before + 500,
        "CO2 should stay near stable with ventilation {co2_before}->{co2_after}"
    );
    // Energy conservation unchanged by blood step alone (O2 energy booked externally).
    let total_after = org.total_accounted_uj() + org.ambient_internal_energy_uj();
    assert_eq!(
        total_before, total_after,
        "gas exchange alone must not leak total energy"
    );
}

#[test]
fn blood_gas_oxygen_overdraft_rejects_without_partial() {
    let mut org = OrganismState::physiological_baseline(&Morphotype::human());
    // Deplete by large demand repeatedly until overdraft. For deterministic test,
    // manually deplete via huge demand.
    let huge = 10_000_000_000i64; // 10 GJ -> demands >21k umol per sec, exceeds capacity quickly
    // Drain many times; first may succeed if ventilation replenishes, so loop until fail.
    let mut failed = false;
    for _ in 0..500 {
        if org.apply_gas_exchange_for_second(huge).is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed, "huge O2 demand must eventually overdraft");
    // Overdraft must not have partially applied chemical burn - chemical stays valid.
    assert!(org.chemical_store_uj() >= 0);
}

#[test]
fn world_engine_advances_with_blood_and_preserves_hash() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("t.sqlite");
    let (mut eng, _) =
        makise_causal_kernel::WorldEngine::open(spec("blood-hash"), StorageLocation::sqlite(&path))
            .expect("open");
    eng.commit(CommitRequest::advance_to("a", 0, 60))
        .expect("60s");
    let org_o2 = eng.organism().unwrap().arterial_o2_umol();
    let org_co2 = eng.organism().unwrap().venous_co2_umol();
    let sat = eng.organism().unwrap().arterial_saturation_permille();
    assert!((950..=1000).contains(&sat), "engine 60s sat {sat}");
    // Partition invariance: 60x1s == 1x60s hash
    let dir2 = tempfile::tempdir().expect("temp");
    let (mut eng2, _) = makise_causal_kernel::WorldEngine::open(
        spec("blood-hash-2"),
        StorageLocation::sqlite(dir2.path().join("t2.sqlite")),
    )
    .expect("open2");
    for i in 0..60 {
        eng2.commit(CommitRequest::advance_to(&format!("s{i}"), i as u64, 1))
            .expect("1s");
    }
    let h1 = eng.organism().unwrap().total_accounted_uj()
        + eng.organism().unwrap().ambient_internal_energy_uj();
    let h2 = eng2.organism().unwrap().total_accounted_uj()
        + eng2.organism().unwrap().ambient_internal_energy_uj();
    assert_eq!(h1, h2, "partition must conserve same total");
    // Restart parity
    let eng_o2 = org_o2;
    let eng_co2 = org_co2;
    drop(eng);
    let (reopened, _) =
        makise_causal_kernel::WorldEngine::open(spec("blood-hash"), StorageLocation::sqlite(&path))
            .expect("reopen");
    let ro = reopened.organism().unwrap();
    assert_eq!(ro.arterial_o2_umol(), eng_o2);
    assert_eq!(ro.venous_co2_umol(), eng_co2);
}

#[test]
fn neko_vs_human_blood_capacity_differs_data_driven() {
    let h = OrganismState::physiological_baseline(&Morphotype::human());
    let n = OrganismState::physiological_baseline(&Morphotype::neko());
    assert_ne!(h.blood_volume_mm3(), n.blood_volume_mm3());
    assert_ne!(h.blood_o2_capacity_umol(), n.blood_o2_capacity_umol());
    assert!(h.blood_o2_capacity_umol() > n.blood_o2_capacity_umol());
}
