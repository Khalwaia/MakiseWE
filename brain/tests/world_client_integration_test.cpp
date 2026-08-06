#include "world_client.hpp"

#include <chrono>
#include <cstdint>
#include <exception>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {

using namespace std::chrono_literals;

void require(bool condition, const std::string& message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

void require_ok(const grpc::Status& status, const std::string& operation) {
    if (!status.ok()) {
        throw std::runtime_error(
            operation + " failed: " + std::to_string(status.error_code()) + " "
            + status.error_message());
    }
}

void set_now(google::protobuf::Timestamp* timestamp) {
    const auto since_epoch = std::chrono::system_clock::now().time_since_epoch();
    const auto milliseconds = std::chrono::duration_cast<std::chrono::milliseconds>(since_epoch);
    const auto seconds = std::chrono::duration_cast<std::chrono::seconds>(milliseconds);
    const auto nanos = std::chrono::duration_cast<std::chrono::nanoseconds>(milliseconds - seconds);
    timestamp->set_seconds(seconds.count());
    timestamp->set_nanos(static_cast<std::int32_t>(nanos.count()));
}

}  // namespace

int main(int argc, char** argv) {
    try {
        require(argc == 2, "expected the WorldService socket path");
        auto client = makise::brain::WorldClient::connect_uds(argv[1]);
        require(client != nullptr, "failed to create WorldClient");

        makise::v1::HandshakeRequest handshake_request;
        handshake_request.set_client_name("makise-cpp-integration-test");
        handshake_request.set_min_protocol_version(1);
        handshake_request.set_max_protocol_version(1);
        handshake_request.set_expected_identity_id("test-makise");
        handshake_request.add_capabilities("event-replay");
        makise::v1::HandshakeResponse handshake;
        require_ok(client->handshake(handshake_request, &handshake, 5s), "handshake");
        require(handshake.selected_protocol_version() == 1, "unexpected protocol version");
        require(handshake.identity_id() == "test-makise", "unexpected identity");
        require(!handshake.world_definition_hash().empty(), "world hash is empty");
        require(handshake.last_event_seq() == 1, "initial event was not committed");

        makise::v1::PerceptionWindow perception;
        require_ok(client->get_perception("makise", &perception, 5s), "get_perception");
        require(perception.anchor_id() == "bed", "unexpected initial anchor");

        std::uint64_t replayed_seq = 0;
        const auto stream_status = client->subscribe_events(
            0,
            std::stop_token{},
            [&replayed_seq](const makise::v1::EventEnvelope& event) {
                replayed_seq = event.event_seq();
                return false;
            });
        require_ok(stream_status, "subscribe_events");
        require(replayed_seq == 1, "event replay did not start at sequence 1");

        makise::v1::CommandEnvelope command;
        command.set_command_id("cmd-cpp-uds-move");
        command.set_identity_id("test-makise");
        command.set_agent_id("makise");
        command.set_expected_world_version(perception.world_version());
        command.set_schema_version(1);
        command.set_decision_id("decision-cpp-uds-move");
        set_now(command.mutable_issued_at());
        command.mutable_ttl()->set_seconds(30);
        command.mutable_move_to()->set_target_anchor_id("work_desk");

        makise::v1::CommandResult committed;
        require_ok(client->execute_command(command, &committed, 5s), "execute_command");
        require(committed.status() == makise::v1::COMMITTED, "move was not committed");
        require(committed.first_event_seq() == 2, "move produced an unexpected event sequence");

        makise::v1::CommandResult duplicate;
        require_ok(client->execute_command(command, &duplicate, 5s), "execute duplicate");
        require(
            duplicate.status() == makise::v1::ALREADY_COMMITTED,
            "duplicate command was not deduplicated");
        require(
            duplicate.first_event_seq() == committed.first_event_seq(),
            "duplicate command returned a different event range");

        makise::v1::CommandResult stored;
        require_ok(
            client->get_command_result(command.command_id(), &stored, 5s),
            "get_command_result");
        require(stored.status() == makise::v1::COMMITTED, "stored result is not committed");

        std::cout << "C++ Brain -> UDS -> Rust WorldService integration passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
