#pragma once

#include <chrono>
#include <cstdint>
#include <functional>
#include <memory>
#include <stop_token>
#include <string>

#include <grpcpp/grpcpp.h>
#include "makise/v1/world.grpc.pb.h"

namespace makise::brain {

class WorldClient final {
public:
    using EventHandler = std::function<bool(const ::makise::v1::EventEnvelope&)>;

    static std::unique_ptr<WorldClient> connect_uds(const std::string& absolute_socket_path);

    explicit WorldClient(std::shared_ptr<grpc::Channel> channel);

    grpc::Status handshake(
        const ::makise::v1::HandshakeRequest& request,
        ::makise::v1::HandshakeResponse* response,
        std::chrono::milliseconds timeout);

    grpc::Status execute_command(
        const ::makise::v1::CommandEnvelope& command,
        ::makise::v1::CommandResult* result,
        std::chrono::milliseconds timeout);

    grpc::Status get_command_result(
        const std::string& command_id,
        ::makise::v1::CommandResult* result,
        std::chrono::milliseconds timeout);

    grpc::Status get_perception(
        const std::string& agent_id,
        ::makise::v1::PerceptionWindow* perception,
        std::chrono::milliseconds timeout);

    grpc::Status subscribe_events(
        std::uint64_t after_seq,
        std::stop_token stop_token,
        const EventHandler& handler);

private:
    std::unique_ptr<::makise::v1::WorldService::Stub> stub_;
};

}  // namespace makise::brain
