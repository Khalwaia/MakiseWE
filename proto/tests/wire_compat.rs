use makise_proto::v1::{CommandEnvelope, CommandStatus, MoveTo, command_envelope};
use prost::Message;

fn command() -> CommandEnvelope {
    CommandEnvelope {
        command_id: "cmd-wire-1".into(),
        identity_id: "test-makise".into(),
        agent_id: "makise".into(),
        expected_world_version: 7,
        schema_version: 1,
        decision_id: "decision-wire-1".into(),
        issued_at: None,
        ttl: None,
        payload: Some(command_envelope::Payload::MoveTo(MoveTo {
            target_anchor_id: "work_desk".into(),
        })),
    }
}

#[test]
fn current_message_round_trips() {
    let original = command();
    let bytes = original.encode_to_vec();
    let decoded = CommandEnvelope::decode(bytes.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn unknown_future_field_is_accepted() {
    let mut bytes = command().encode_to_vec();
    // Field 99, varint wire type, value 1.
    bytes.extend_from_slice(&[0x98, 0x06, 0x01]);
    assert!(CommandEnvelope::decode(bytes.as_slice()).is_ok());
}

#[test]
fn malformed_length_delimited_field_is_rejected() {
    let mut bytes = command().encode_to_vec();
    // Field 99 claims five bytes, but carries only one.
    bytes.extend_from_slice(&[0x9a, 0x06, 0x05, 0x01]);
    assert!(CommandEnvelope::decode(bytes.as_slice()).is_err());
}

#[test]
fn command_status_numbers_remain_stable() {
    assert_eq!(CommandStatus::Committed as i32, 1);
    assert_eq!(CommandStatus::AlreadyCommitted as i32, 2);
    assert_eq!(CommandStatus::StaleWorld as i32, 5);
    assert_eq!(CommandStatus::ExpiredDecision as i32, 6);
}
