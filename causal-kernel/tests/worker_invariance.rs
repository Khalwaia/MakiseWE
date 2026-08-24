use makise_causal_kernel::{
    CommitRequest, OpenSpec, ProjectionRequest, StorageLocation, TimelineId, WorldEngine, WorldId,
};
use std::thread;

fn spec(name: &str) -> OpenSpec {
    OpenSpec::new(
        WorldId::new(format!("{name}-world")).expect("valid"),
        TimelineId::new(format!("{name}-timeline")).expect("valid"),
    )
}

fn run_with_workers(worker_count: usize, name: &str) -> i64 {
    thread::scope(|scope| {
        let handles: Vec<_> = (0..worker_count)
            .map(|worker| {
                let worker_name = format!("{name}-w{worker}");
                scope.spawn(move || {
                    let directory = tempfile::tempdir().expect("temp dir");
                    let path = directory.path().join("t.sqlite");
                    let (mut engine, _) =
                        WorldEngine::open(spec(&worker_name), StorageLocation::sqlite(path))
                            .expect("open");
                    for index in 0..16u64 {
                        engine
                            .commit(CommitRequest::advance_to(&format!("req-{index}"), index, 1))
                            .expect("advance one canonical second");
                    }
                    let result = engine
                        .project(ProjectionRequest::current())
                        .expect("projection")
                        .simulated_second();
                    // Keep storage alive until the scoped worker completes.
                    std::mem::forget(directory);
                    result
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("worker completed"))
            .max()
            .expect("at least one worker")
    })
}

#[test]
fn one_and_sixteen_workers_produce_identical_canonical_results() {
    let single_worker = run_with_workers(1, "single");
    let sixteen_workers = run_with_workers(16, "many");
    assert_eq!(
        single_worker, sixteen_workers,
        "canonical transition semantics must not depend on worker count"
    );
}
