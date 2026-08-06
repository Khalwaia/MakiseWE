#include "world_client.hpp"

int main() {
    auto client = makise::brain::WorldClient::connect_uds("/tmp/makise-world-not-running.sock");
    return client ? 0 : 1;
}
