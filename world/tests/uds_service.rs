#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper_util::rt::TokioIo;
use makise_proto::v1::world_service_client::WorldServiceClient;
use makise_proto::v1::{
    CommandEnvelope, CommandStatus, GetCommandResultRequest, GetPerceptionRequest,
    HandshakeRequest, MoveTo, SubscribeEventsRequest, command_envelope,
};
use makise_world::{
    PathGuard, WorldActorConfig, WorldActorHandle, WorldDefinition, WorldEngine, WorldRpc,
    serve_uds,
};
use prost_types::{Duration as ProtoDuration, Timestamp};
use tokio::net::UnixStream;
use tonic::Code;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

fn definition() -> WorldDefinition {
    WorldDefinition::load(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../world-packages/test-room-v1/manifest.json"),
        &PathGuard::default(),
    )
    .unwrap()
}

fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn timestamp(milliseconds: i64) -> Timestamp {
    Timestamp {
        seconds: milliseconds.div_euclid(1_000),
        nanos: i32::try_from(milliseconds.rem_euclid(1_000) * 1_000_000).unwrap(),
    }
}

async fn connect(socket: &Path) -> WorldServiceClient<Channel> {
    let mut last_error = None;
    for _ in 0..100 {
        let socket = socket.to_path_buf();
        let connector = service_fn(move |_| {
            let socket = socket.clone();
            async move { UnixStream::connect(socket).await.map(TokioIo::new) }
        });
        match Endpoint::try_from("http://[::]:50051")
            .unwrap()
            .connect_with_connector(connector)
            .await
        {
            Ok(channel) => return WorldServiceClient::new(channel),
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("failed to connect to test WorldService: {last_error:?}");
}

fn handshake(identity_id: &str, definition_hash: &str) -> HandshakeRequest {
    HandshakeRequest {
        client_name: "fake-brain-test".into(),
        min_protocol_version: 1,
        max_protocol_version: 1,
        expected_identity_id: identity_id.into(),
        expected_world_definition_hash: definition_hash.into(),
        capabilities: vec!["event-replay".into()],
    }
}

fn move_command(world_version: u64, issued_at_ms: i64) -> CommandEnvelope {
    CommandEnvelope {
        command_id: "cmd-uds-move".into(),
        identity_id: "test-makise".into(),
        agent_id: "makise".into(),
        expected_world_version: world_version,
        schema_version: 1,
        decision_id: "decision-uds-move".into(),
        issued_at: Some(timestamp(issued_at_ms)),
        ttl: Some(ProtoDuration {
            seconds: 30,
            nanos: 0,
        }),
        payload: Some(command_envelope::Payload::MoveTo(MoveTo {
            target_anchor_id: "work_desk".into(),
        })),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uds_round_trip_deduplicates_and_resumes_event_stream() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("world.sock");
    let database = temp.path().join("world.db");
    let definition = definition();
    let definition_hash = definition.hash().to_owned();
    let opened_at = now_ms();
    let engine = WorldEngine::open(
        &database,
        "test-makise",
        definition,
        "bed",
        opened_at,
        &PathGuard::default(),
    )
    .unwrap();
    let actor = WorldActorHandle::spawn(
        engine,
        WorldActorConfig {
            tick_interval: Duration::from_millis(10),
            ..WorldActorConfig::default()
        },
    )
    .unwrap();
    let rpc = WorldRpc::new(actor);
    let (shutdown, shutdown_signal) = tokio::sync::oneshot::channel();
    let server_socket = socket.clone();
    let server = tokio::spawn(async move {
        serve_uds(server_socket, rpc, async {
            let _ = shutdown_signal.await;
        })
        .await
    });

    let mut client = connect(&socket).await;
    assert_eq!(
        std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let negotiated = client
        .handshake(handshake("test-makise", &definition_hash))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(negotiated.selected_protocol_version, 1);
    assert_eq!(negotiated.last_event_seq, 1);

    let perception = client
        .get_perception(GetPerceptionRequest {
            agent_id: "makise".into(),
            previous_perception_id: None,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(perception.anchor_id, "bed");

    let mut live = client
        .subscribe_events(SubscribeEventsRequest { after_seq: 0 })
        .await
        .unwrap()
        .into_inner();
    let awakened = live.message().await.unwrap().unwrap();
    assert_eq!(awakened.event_seq, 1);
    assert_eq!(awakened.event_type, "agent_awakened");

    let command = move_command(perception.world_version, now_ms());
    let committed = client
        .execute_command(command.clone())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(committed.status, CommandStatus::Committed as i32);
    assert_eq!(committed.first_event_seq, 2);

    let scheduled = live.message().await.unwrap().unwrap();
    assert_eq!(scheduled.event_seq, 2);
    assert_eq!(scheduled.event_type, "activity_scheduled");

    let duplicate = client.execute_command(command).await.unwrap().into_inner();
    assert_eq!(duplicate.status, CommandStatus::AlreadyCommitted as i32);
    assert_eq!(duplicate.first_event_seq, committed.first_event_seq);

    let stored = client
        .get_command_result(GetCommandResultRequest {
            command_id: "cmd-uds-move".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(stored.status, CommandStatus::Committed as i32);

    drop(client);
    let mut reconnected = connect(&socket).await;
    let mut resumed = reconnected
        .subscribe_events(SubscribeEventsRequest { after_seq: 1 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resumed.message().await.unwrap().unwrap().event_seq, 2);

    let mismatch = reconnected
        .handshake(handshake("somebody-else", &definition_hash))
        .await
        .unwrap_err();
    assert_eq!(mismatch.code(), Code::FailedPrecondition);

    drop(resumed);
    drop(reconnected);
    drop(live);
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(
        !socket.exists(),
        "owned socket must be removed after shutdown"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn future_event_cursor_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("world.sock");
    let database = temp.path().join("world.db");
    let engine = WorldEngine::open(
        &database,
        "test-makise",
        definition(),
        "bed",
        now_ms(),
        &PathGuard::default(),
    )
    .unwrap();
    let actor = WorldActorHandle::spawn(engine, WorldActorConfig::default()).unwrap();
    let rpc = WorldRpc::new(actor);
    let (shutdown, shutdown_signal) = tokio::sync::oneshot::channel();
    let server_socket = PathBuf::from(&socket);
    let server = tokio::spawn(async move {
        serve_uds(server_socket, rpc, async {
            let _ = shutdown_signal.await;
        })
        .await
    });

    let mut client = connect(&socket).await;
    assert_eq!(
        std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let error = client
        .subscribe_events(SubscribeEventsRequest { after_seq: 100 })
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);

    drop(client);
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(
        !socket.exists(),
        "owned socket must be removed after shutdown"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn existing_socket_path_is_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("world.sock");
    std::fs::write(&socket, b"sentinel").unwrap();
    let engine = WorldEngine::open(
        temp.path().join("world.db"),
        "test-makise",
        definition(),
        "bed",
        now_ms(),
        &PathGuard::default(),
    )
    .unwrap();
    let actor = WorldActorHandle::spawn(engine, WorldActorConfig::default()).unwrap();
    let error = serve_uds(&socket, WorldRpc::new(actor), async {})
        .await
        .unwrap_err();
    assert!(error.to_string().contains("refusing to replace"));
    assert_eq!(std::fs::read(&socket).unwrap(), b"sentinel");
}
