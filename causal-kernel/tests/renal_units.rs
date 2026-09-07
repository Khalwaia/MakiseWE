use makise_causal_kernel::{
    CommitRequest, EventCursor, EventQuery, OpenSpec, StorageLocation, TimelineId, WorldEngine,
    WorldId,
};

fn spec() -> OpenSpec {
    OpenSpec::new(
        WorldId::new("renal-units-world").expect("world"),
        TimelineId::new("renal-units-timeline").expect("timeline"),
    )
}

fn query() -> EventQuery {
    EventQuery::new(EventCursor::start(), 10).expect("query")
}

#[test]
fn sodium_deltas_use_micromoles_across_intake_excretion_and_reopen() {
    let directory = tempfile::tempdir().expect("directory");
    let storage = StorageLocation::sqlite(directory.path().join("timeline.sqlite"));
    let (mut engine, _) = WorldEngine::open(spec(), storage.clone()).expect("open");
    let intake = CommitRequest::ingest_fluid("intake", 0, 250_000, 35_000);
    engine.commit(intake.clone()).expect("intake");
    let fast = engine.fast_replay().expect("fast");
    assert_eq!(engine.audit_replay().expect("audit"), fast);
    engine
        .commit(CommitRequest::advance_to("minute", 1, 60))
        .expect("advance");
    let events = engine.events(query()).expect("events");
    // ADR-0016: 420 mmol baseline + 35 mmol intake - 60 umol excretion.
    for (event, expected) in events.events().iter().zip([
        [(None, Some(455_000)), (None, Some(0))],
        [(Some(455_000), Some(454_940)), (Some(0), Some(60))],
    ]) {
        for (quantity, (before, after)) in [
            "organism.renal.plasma_sodium",
            "organism.renal.urine_sodium",
        ]
        .into_iter()
        .zip(expected)
        {
            let delta = event
                .unit_deltas()
                .expect("deltas")
                .iter()
                .find(|delta| delta.quantity() == quantity)
                .expect("sodium delta");
            assert_eq!(delta.unit(), "umol");
            assert_eq!((delta.before(), delta.after()), (before, after));
        }
    }
    assert_eq!(events.events().len(), 2);
    drop(engine);
    let (mut reopened, _) = WorldEngine::open(spec(), storage).expect("reopen");
    assert_eq!(reopened.events(query()).expect("events"), events);
    assert!(reopened.commit(intake).expect("retry").replayed_request());
    assert_eq!(reopened.events(query()).expect("no retry event"), events);
}

#[test]
fn legacy_sodium_units_remain_readable_but_cannot_pass_dimensional_replay() {
    for quantity in [
        "organism.renal.plasma_sodium",
        "organism.renal.urine_sodium",
    ] {
        let directory = tempfile::tempdir().expect("directory");
        let path = directory.path().join("timeline.sqlite");
        let storage = StorageLocation::sqlite(&path);
        let (mut engine, _) = WorldEngine::open(spec(), storage.clone()).expect("open");
        let intake = CommitRequest::ingest_fluid("intake", 0, 250_000, 35_000);
        engine.commit(intake.clone()).expect("intake");
        let organism = engine.organism().expect("organism").clone();
        let projection = engine
            .project(makise_causal_kernel::ProjectionRequest::current())
            .expect("projection");
        let events = engine.events(query()).expect("events");
        let legacy: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("fixtures/renal-sodium-legacy-deltas.json"))
                .expect("legacy fixture");
        let deltas: Vec<_> = events.events()[0]
            .unit_deltas()
            .expect("deltas")
            .iter()
            .map(|delta| {
                if delta.quantity() == quantity {
                    legacy
                        .iter()
                        .find(|value| value["quantity"] == quantity)
                        .expect("archived sodium delta")
                        .clone()
                } else {
                    serde_json::json!({
                        "quantity": delta.quantity(), "unit": delta.unit(),
                        "before": delta.before(), "after": delta.after(),
                    })
                }
            })
            .collect();
        drop(engine);
        // Compatibility fixture only: reconstruct the pre-fix sodium record.
        let connection = rusqlite::Connection::open(&path).expect("fixture storage");
        connection
            .execute(
                "UPDATE causal_transitions SET unit_deltas=?1",
                [serde_json::to_string(&deltas).expect("encode fixture")],
            )
            .expect("install legacy fixture");
        drop(connection);
        let original = std::fs::read(&path).expect("archive bytes");

        let (mut engine, _) = WorldEngine::open(spec(), storage.clone()).expect("legacy open");
        let archived = engine.events(query()).expect("read legacy events");
        let delta = archived.events()[0]
            .unit_deltas()
            .expect("legacy deltas")
            .iter()
            .find(|delta| delta.quantity() == quantity)
            .expect("legacy sodium");
        assert_eq!(delta.unit(), "centi_umol", "no silent relabeling");
        assert_eq!(engine.organism(), Some(&organism));
        assert_eq!(
            engine
                .project(makise_causal_kernel::ProjectionRequest::current())
                .expect("projection"),
            projection
        );
        assert!(
            engine
                .commit(intake)
                .expect("legacy retry")
                .replayed_request()
        );
        let expected_error =
            format!("stored quantity {quantity} uses incompatible unit centi_umol; expected umol");
        assert_eq!(
            engine
                .fast_replay()
                .expect_err("reject legacy units")
                .to_string(),
            expected_error
        );
        drop(engine);
        assert_eq!(std::fs::read(&path).expect("unchanged archive"), original);

        let (mut engine, _) = WorldEngine::open(spec(), storage.clone()).expect("reopen");
        assert_eq!(
            engine
                .audit_replay()
                .expect_err("reject legacy units")
                .to_string(),
            expected_error
        );
        assert_eq!(
            engine
                .safe_stop()
                .expect("safe stop")
                .expect("stopped")
                .reason(),
            "incompatible_quantity_unit"
        );
        assert_eq!(engine.events(query()).expect("preserved events"), archived);
        assert_eq!(engine.organism(), Some(&organism));
        drop(engine);
        let (mut engine, _) = WorldEngine::open(spec(), storage).expect("reopen stopped");
        assert!(engine.safe_stop().expect("durable stop").is_some());
        assert!(matches!(
            engine.commit(CommitRequest::ingest_fluid("blocked", 1, 1, 0)),
            Err(makise_causal_kernel::CommitError::SafeStopped(_))
        ));
        assert_eq!(engine.events(query()).expect("preserved events"), archived);
    }
}
