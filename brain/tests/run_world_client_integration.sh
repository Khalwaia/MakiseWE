#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 <client-test> <cargo> <repository-root>" >&2
    exit 2
fi

client_test="$1"
cargo="$2"
repository_root="$3"
test_root="$(mktemp -d /tmp/makise-cross-language.XXXXXX)"
server_pid=""

cleanup() {
    if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
        kill -INT "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    case "$test_root" in
        /tmp/makise-cross-language.*)
            rm -rf -- "$test_root"
            ;;
        *)
            echo "refusing to remove unexpected test path: $test_root" >&2
            ;;
    esac
}
trap cleanup EXIT INT TERM

cd "$repository_root"
"$cargo" build --locked --offline --quiet -p makise-world

socket="$test_root/world.sock"
database="$test_root/world.db"
manifest="$repository_root/world-packages/test-room-v1/manifest.json"
log="$test_root/world-service.log"

"$repository_root/target/debug/makise-world" serve \
    "$socket" \
    "$database" \
    "$manifest" \
    test-makise \
    bed >"$log" 2>&1 &
server_pid="$!"

for _ in $(seq 1 200); do
    if [[ -S "$socket" ]]; then
        break
    fi
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "WorldService exited before creating its socket" >&2
        sed -n '1,200p' "$log" >&2
        exit 1
    fi
    sleep 0.05
done

if [[ ! -S "$socket" ]]; then
    echo "WorldService did not create its socket in time" >&2
    sed -n '1,200p' "$log" >&2
    exit 1
fi

"$client_test" "$socket"
