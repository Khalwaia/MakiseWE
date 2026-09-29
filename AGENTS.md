# Repository Guidelines

## Project Structure & Module Organization

MakiseWE is a Rust workspace (`Cargo.toml`) with three crates:

- `causal-kernel/` — authoritative causal simulation, physiology, physics, persistence, and replay; integration tests live in `causal-kernel/tests/`.
- `world/` — legacy-compatible World Engine service, SQLite storage, UDS/gRPC server, and package validation; tests are in `world/tests/`.
- `proto/` — versioned Protobuf definitions and wire-compatibility tests.

`brain/` contains the C++20 `WorldClient` and CMake tests. `contracts/` stores JSON Schemas and fixtures; `world-packages/` stores data-defined worlds. Normative architecture and domain decisions are in `ARCHITECTURE.md`, `INVARIANTS.md`, `CONTEXT.md`, `ROADMAP.md`, and `docs/adr/`. Keep generated output in `build/` or `target/`, never in source directories.

## Build, Test, and Development Commands

Rust 1.97.1 is pinned in `rust-toolchain.toml`.
Run these gates to check formatting, reject lint warnings, and execute all Rust tests:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Use `cargo test -p <crate> --test <name>` for a focused integration test. Verify a world package with `cargo run -p makise-world -- verify-package <manifest>`. For the C++ client (CMake, Protobuf, and gRPC required):

```bash
cmake -S brain -B build/brain -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build/brain --parallel
ctest --test-dir build/brain --output-on-failure
```

## Coding Style & Naming Conventions

Rust uses `rustfmt`, four-space indentation, `snake_case` functions/modules, and `PascalCase` types. C++ uses C++20, warnings as errors, and project-style `snake_case` methods. JSON, YAML, and TOML use two-space indentation. Preserve LF endings and final newlines per `.editorconfig`.

## Testing Guidelines

Use Rust's built-in test harness and CTest for C++. Add tests through public seams and name Rust tests by observable behavior (for example, `daily_fluid_and_sodium_balance_stay_in_declared_reference_bands`). Expected values must come from specifications, measurements, or independent calculations. Contract, replay, restart, determinism, conservation, and compatibility changes require fixtures and focused evidence before the full workspace gate. No numeric coverage threshold is configured.

## Commit & Pull Request Guidelines

Use Conventional Commits such as `feat(kernel): ...`, `fix(world): ...`, `test: ...`, or `docs: ...`. Keep commits and PRs focused. PRs should state observable outcome, scope and non-goals, phase/gate, causal and compatibility impact, red/green evidence, validation commands, and rollback or `SafeStop` behavior. Update schemas, fixtures, coverage, and normative docs when contracts change. Do not commit secrets, runtime databases, generated build output, or machine-specific paths.

## Architecture & Contribution Rules

Follow [CONTRIBUTING.md](CONTRIBUTING.md), [INVARIANTS.md](INVARIANTS.md), and relevant ADRs before implementation. Keep authoritative mutations behind `WorldEngine::commit`, preserve legacy compatibility, and respect roadmap phase gates. Open an issue for substantial changes.

## Project Description

Read [PROJECT.md](PROJECT.md) for the detailed project description, architecture, runtime components, current phase, and open implementation boundaries.
