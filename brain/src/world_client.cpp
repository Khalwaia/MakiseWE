#include "world_client.hpp"

#include <utility>

namespace makise::brain {
namespace {

void set_deadline(grpc::ClientContext& context, std::chrono::milliseconds timeout) {
    context.set_deadline(std::chrono::system_clock::now() + timeout);
}

}  // namespace

std::unique_ptr<WorldClient> WorldClient::connect_uds(const std::string& absolute_socket_path) {
    auto channel = grpc::CreateCustomChannel(
        "unix:" + absolute_socket_path,
        grpc::InsecureChannelCredentials(),
        grpc::ChannelArguments{});
    return std::make_unique<WorldClient>(std::move(channel));
}

WorldClient::WorldClient(std::shared_ptr<grpc::Channel> channel)
    : stub_(::makise::v1::WorldService::NewStub(std::move(channel))) {}

grpc::Status WorldClient::handshake(
    const ::makise::v1::HandshakeRequest& request,
    ::makise::v1::HandshakeResponse* response,
    std::chrono::milliseconds timeout) {
    grpc::ClientContext context;
    set_deadline(context, timeout);
    return stub_->Handshake(&context, request, response);
}

grpc::Status WorldClient::execute_command(
    const ::makise::v1::CommandEnvelope& command,
    ::makise::v1::CommandResult* result,
    std::chrono::milliseconds timeout) {
    grpc::ClientContext context;
    set_deadline(context, timeout);
    return stub_->ExecuteCommand(&context, command, result);
}

grpc::Status WorldClient::get_command_result(
    const std::string& command_id,
    ::makise::v1::CommandResult* result,
    std::chrono::milliseconds timeout) {
    grpc::ClientContext context;
    set_deadline(context, timeout);
    ::makise::v1::GetCommandResultRequest request;
    request.set_command_id(command_id);
    return stub_->GetCommandResult(&context, request, result);
}

grpc::Status WorldClient::get_perception(
    const std::string& agent_id,
    ::makise::v1::PerceptionWindow* perception,
    std::chrono::milliseconds timeout) {
    grpc::ClientContext context;
    set_deadline(context, timeout);
    ::makise::v1::GetPerceptionRequest request;
    request.set_agent_id(agent_id);
    return stub_->GetPerception(&context, request, perception);
}

grpc::Status WorldClient::subscribe_events(
    std::uint64_t after_seq,
    std::stop_token stop_token,
    const EventHandler& handler) {
    if (!handler) {
        return grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "event handler is required");
    }
    grpc::ClientContext context;
    std::stop_callback cancel_on_stop(stop_token, [&context] { context.TryCancel(); });
    ::makise::v1::SubscribeEventsRequest request;
    request.set_after_seq(after_seq);
    auto reader = stub_->SubscribeEvents(&context, request);
    ::makise::v1::EventEnvelope event;
    bool stopped_by_handler = false;
    while (!stop_token.stop_requested() && reader->Read(&event)) {
        if (!handler(event)) {
            stopped_by_handler = true;
            context.TryCancel();
            break;
        }
    }
    auto status = reader->Finish();
    if ((stop_token.stop_requested() || stopped_by_handler)
        && status.error_code() == grpc::StatusCode::CANCELLED) {
        return grpc::Status::OK;
    }
    return status;
}

}  // namespace makise::brain
