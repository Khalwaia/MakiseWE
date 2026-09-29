# MakiseWE: Detailed Project Description

Description as of 22 September 2026, based on checkout HEAD `9483cab`. This document was compiled from the code, tests, contracts, and plans in this checkout. Architecture sections describe the normative target; actual implementation limits are listed separately. After runtime changes, verify technical details against the referenced source files.

## Navigation

- Purpose, current phase, and target architecture appear first.
- Authority, contracts, persistence, and cognition explain system rules.
- Runtime components and the module map identify implementation locations.
- Current limits and Phase 3 distinguish working code from unfinished work.
- Development commands, checks, and references explain how to reproduce behavior and investigate further.

## Project Purpose

MakiseWE is a persistent, multiscale causal simulation of a world, organisms, individual consciousnesses, and the civilization they create. The project aims to model continuous life through verifiable causal mechanisms rather than LLM storytelling, predefined outcome functions, or arbitrary game-like scores.

The target V1 combines the physical world, physiology, neural and cognitive processes, digital devices, organizations, services, production, and construction. Every result should arise from world state, available observations, granted capabilities, causal mechanisms, and feedback. “Realism” always refers to a declared `FidelityEnvelope`: a range of states, time scales, resolutions, errors, and validation evidence.

## Current Status and Roadmap

Phase 0, contracts and architecture, is complete. It defined the ubiquitous language, invariants, ADRs, JSON Schemas, fixtures, morphotypes, resolution contracts, and a 24-hour acceptance scenario. Phase 1, the first Human/Neko vertical slice, closed with a separate gate commit. Phase 2, apartment and physical embodiment, also closed: the kernel gained metric rigid bodies, contacts, friction, fluids, atmosphere, heat, propagation, infrastructure, and closed-loop control episodes.

Current work belongs to Phase 3, everyday physiology. The coarse `cardiorespiratory.gas-exchange` and `renal.fluid-electrolyte` slices have runtime code and focused tests, but their system gates remain open. The renal slice still requires mixed `ingest_fluid -> advance_to -> ingest_fluid` replay, worker invariance on a single timeline, an executable resolution upgrade, and independent empirical validation. Synthetic fixtures and deterministic replay do not prove biological realism.

After Phase 3, the roadmap covers cells/immunity/pathology/drugs, reproduction/development/aging, neuroscience/psychology, technology/economy/society/multiple consciousnesses, and performance/scaling. A subsequent phase cannot start before a separate gate commit closes its predecessor. `STAGE_5.md` is a superseded historical document and does not define the current normative scope.

## Unified Causal Architecture

MakiseWE uses a single causal graph with feedback and mixed resolution. Domains L0–L9 map state and mechanisms rather than sequential stages of a global tick:

- L0 Physical World: geometry, matter, mass, energy, temperature, air, light, sound, and fluids.
- L1 Organism: anatomy, organs, compartments, circulation, respiration, and metabolism.
- L2 Tissue/Cellular: tissues, `CellCohort`, individual cells, immune cells, and receptors.
- L3 Molecular/Biochemical: substances, amounts, concentrations, reactions, transport, signaling, and PK/PD.
- L4 Neural/Brain: brain regions, `NeuralPopulation`, neurotransmitters, and autonomic control.
- L5 Consciousness: perception, interoception, memory, `CortexFrame`, and proposal disposition.
- L6 Motor Control: accepted intention, motor plan, physical validation, and neural/muscular control.
- L7 Physical Action: muscles, articulated bodies, contacts, object interaction, and outcomes.
- L8 Digital/Computation: devices, machine state, code execution, storage, sensors, radios, and networks.
- L9 Institutional/Economic: organizations, authority, claims, contracts, obligations, payment, and services.

Connections are bidirectional wherever the causal model requires them. Motor control changes the physical world, new observables return to perception, and device operation connects electricity, heat, hardware, software, and institutional contracts. The durable causal timeline records transitions across all domains; it is not an additional `WORLD EVENTS` simulation layer.

## Authority and Mutation Boundary

Under the normative architecture, World Engine is the only author of authoritative physical, biological, neural, digital, and institutional state. Its deep-module public boundary consists of `open`, `commit`, `project`, and `events`. `WorldEngine::commit` is the only mutation path for time, stimuli, model responses, actions, resolution changes, and administrative intents. Transport adapters, schedulers, panels, memory services, model providers, and compute workers do not receive mutable state. This is the target contract for the new V1; the existing legacy API has its own command-shaped adapters.

Workers and LLMs may produce proposals, but the authoritative writer rechecks preconditions, units, conservation, capabilities, artifact digests, deterministic ordering, and idempotency. Projections and subjective memory never correct objective state. Administrators also submit intents through commit instead of writing directly to SQLite.

## Contracts, Units, and Resolution

A mechanism can be loaded only with a complete `MechanismContract`: identifier and semantic version, content digest, causal inputs/outputs, read/write sets, authoritative quantities with units, resolution and scheduling rules, parameters, provenance, uncertainty, validity range, conservation, failure policy, validation scenarios, and a resolution-upgrade path. Unknown, missing, or incompatible fields are rejected during admission.

A `ResolutionContract` defines represented entities, coarse-to-fine lift, fine-to-coarse projection, conserved quantities, observable continuity, uncertainty transformation, lineage, compute estimate, deterministic triggers, and rollback. `CellCohort` and `NeuralPopulation` are initial coarse adapters; future individual-cell and individual-neuron representations use the same causal ports. Representation changes are recorded as `ResolutionChanged` rather than hidden LOD: they require a trigger, conservation proof, lineage, error bounds, and rollback. Insufficient CPU/RAM/storage returns `CapacityExceeded` and does not authorize a silent fidelity downgrade.

Authoritative quantities have units or an explicitly declared dimensionless kind. Arbitrary normalized `health`, `energy`, `urgency`, `cleanliness`, or `importance` scores do not replace physical quantities. Canonical transitions are independent of wall clock, acceleration, restart, downtime, partitioning, and worker count; reductions use deterministic ordering.

## Persistence and Replay

The normative `CausalTransition` format must store causes and correlation IDs, a canonical simulation interval, mechanism/model/resolution/solver digests, unit-typed deltas, uncertainty/error bounds, a conservation report, lineage references, a deterministic seed, validation evidence, authority/capability evidence, previous/resulting state hashes, schema version, and commit time. This is the specification in [PROTO.md](PROTO.md), not a claim that every historical runtime record contains every field.

Target fast replay verifies the hash chain and applies committed deltas. Target audit replay loads exact executable artifacts by digest, reruns canonical mechanisms, and compares transitions, conservation, and the resulting hash. Missing artifacts, digest mismatches, non-convergence, broken chains, or conservation failures require typed rejection or durable `SafeStop`; the system must not substitute a “similar” current model version. The current kernel demonstrates these properties for selected slices; the complete mixed physiology chain is not covered yet.

Repeating the same `request_id` with the same canonical payload returns the original receipt. The same ID with a different payload is rejected. A transport timeout does not prove that a commit failed: the client first queries the receipt or events. Snapshots are verifiable acceleration artifacts and do not replace append-only history.

## Compatibility and Security

Format changes follow `expand -> migrate -> verify -> contract`: add readers and schemas beside the old path, demonstrate migration and replay, and remove temporary tooling in separate work. Legacy protobuf fixtures, world packages, SQLite databases, snapshots, logs, and exact artifact bytes are not rewritten in place. New V1 timelines use a separate database and a reversible compatibility path. Rollback switches to the previous executable/archive and never downcasts new events.

External effects require diegetic permission, host authorization, committed intent, and an idempotent receipt. Replay applies the receipt and never invokes the external executor again. Simulated code runs in a deterministic sandbox without direct access to the host filesystem, network, clock, or secrets. Security reports follow `SECURITY.md` rather than ordinary issues.

## Organisms, Morphotypes, and Cognition

An `Organism` is a physically continuous living system with its own body, boundaries, and causal history. A `Consciousness` attaches separately and has its own perception, memory, goals, and cognitive streams. Human and Neko are independent root `MorphotypeDefinition` packages that connect shared mammalian mechanisms through data. The runtime contains no closed species-specific branches such as `is_neko`; new morphotypes are added as data.

An LLM or scripted cortex creates a `CortexProposal` containing appraisal, a goal, intention, plan hypothesis, memory interpretation, or communication. `CognitiveGate` considers neural state, identity values, traits, memory, commitments, and physical feasibility, then records a `CognitiveDisposition`: `Accepted`, `Rejected`, `Deferred`, or `NeedsRevision`. Only Accepted produces a separate cognitive-state adoption transition. An accepted intention starts a durable `ControlEpisode` driven by perception, feedback, interruption, partial results, failure, and replanning. Proposals contain no biological/physical deltas; the LLM does not assign hormones, neurotransmitters, neural activation, object state, code execution, or action success.

The preceding paragraph describes the target cognitive contract. The implementation in [cognitive.rs](causal-kernel/src/cognitive.rs) is currently scripted: `evaluate` returns `Accepted` with reason `feasible` by default, and overrides provide rejection or deferral. The Rust enum has three states: `Accepted`, `Rejected`, and `Deferred`; `NeedsRevision` belongs to the normative contract. This module does not yet fully evaluate personality, memory, neural state, or physical feasibility. `adopt_intention` checks proposal acceptance but does not implement the entire durable cognitive pipeline.

Subjective memory is a separate stream. It stores available perception, interpretation, provenance, privacy ownership, uncertainty, and links to world transitions. Memory does not create perception, become objective fact, or grant one Consciousness access to another's memory without a policy.

## Runtime Components

### `causal-kernel/`

This Rust crate implements V1 timelines. Its public API includes identifiers and `OpenSpec`, SQLite `StorageLocation`, `CommitRequest`, `CommitReceipt`, `EventQuery`, `EventPage`, `Projection`, `CausalTransition`, `GenesisSnapshot`, `WorldEngine`, `ReplayResult`, and typed errors. The kernel exports quantities, artifact/contract admission, morphotypes, organisms, cognition, cell/neural aggregates, circadian/sleep behavior, digestion, thermal reservoirs, atmosphere, fluids, rigid bodies, articulation, contacts, physics islands, balance, walking, propagation, infrastructure, blood, and renal models.

Integration tests in `causal-kernel/tests/` exercise public seams: determinism, partition parity, restart/reopen, fast/audit replay, retry/conflict, conservation, invalid units, artifact integrity, physics, control episodes, blood gas, and renal behavior. Tests must not assert private call counts or compute expected values using the production algorithm.

### `world/`

Legacy-compatible `makise-world` contains the engine, domain model, SQLite store, bounded actor, RPC server, Unix Domain Socket transport, path guard, weather/environment projections, and data-defined world-package validation. The CLI supports `verify-package`, `status`, and `serve`. WorldService uses a Protobuf/gRPC adapter and does not introduce a second authoritative mutation path.

### `proto/` and `brain/`

`proto/makise/v1/world.proto` is the versioned wire contract for `WorldService`, with Handshake, ExecuteCommand, GetCommandResult, GetPerception, SubscribeEvents, and Health. The Rust `proto` crate generates Prost/Tonic bindings and includes a wire-compatibility test. `brain/` contains the C++20 `WorldClient`; CMake generates Protobuf/gRPC sources, builds static libraries, and runs compile/integration tests.

### Contracts, Packages, and Documentation

`contracts/schemas/` contains schemas for mechanisms, resolutions, morphotypes, cortex proposals, cognitive dispositions, and decision envelopes. `contracts/fixtures/` contains Human/Neko morphotypes, mechanism programs, resolution examples, and accepted/rejected/deferred cognition. `world-packages/` contains manifests, maps, and a package schema for `test-room-v1` and `apartment-v1`.

`docs/adr/` contains accepted and superseded architectural decisions. `docs/plans/` contains implementation/evidence records. `docs/research/` contains primary sources, extracted data, and validation ledgers. `docs/scenarios/` contains acceptance scenarios. `docs/coverage/` contains the evidence matrix. The `panel/`, `gateway/`, `identity/`, `memory/`, `deploy/`, and root `tests/` directories represent future boundaries; their presence does not imply complete implementations.

## Technology Stack and Dependencies

The root [Cargo.toml](Cargo.toml) includes exactly three Rust crates: `makise-causal-kernel`, `makise-world`, and `makise-proto`. The workspace uses Rust edition 2024; the minimum version in package metadata is 1.97, while [rust-toolchain.toml](rust-toolchain.toml) pins 1.97.1 with `rustfmt` and `clippy`.

| Component | Main Tools | Purpose |
|---|---|---|
| Kernel | `rusqlite`, `sha2`, `serde_json`, `jsonschema`, `thiserror` | SQLite state, hashes, artifact parsing, schema checks, typed errors |
| Legacy world | Tokio, Tonic, Prost, Reqwest, Serde, SQLite | asynchronous transport, HTTP weather inputs, serialization, storage |
| Protocol | Prost, Tonic-Prost, `tonic-prost-build`, `protoc-bin-vendored` | Rust binding generation at build time |
| C++ client | C++20, CMake ≥3.24, Protobuf, gRPC | local WorldService client |
| Rust tests | built-in test harness, `tempfile` | public scenarios with temporary databases |
| C++ tests | CTest, Bash/Cargo integration runner | compilation and client/server integration |

`makise-world` does not depend on `makise-causal-kernel`: running the legacy CLI does not automatically run the new physiology. `brain/` is also outside the Cargo workspace. Rust protobuf builds use vendored `protoc`, but C++ requires separate Protobuf/gRPC development packages.

## Kernel Source Map

Modules are declared private and expose selected types through `pub use` in [lib.rs](causal-kernel/src/lib.rs). This file also contains substantial persistence, commit, and replay logic; there is no universal physiology executor yet.

| Files in `causal-kernel/src/` | What to Find |
|---|---|
| `lib.rs` | `WorldEngine`, requests/receipts, timeline format, database, events, replay, hashes |
| `quantity.rs` | dimensions, scales, checked quantities, energy reservoirs |
| `artifact.rs` | `MechanismContract` parsing, `ProgramAbi`, admission, proposals from program bytes |
| `morphotype.rs` | anatomy nodes/edges, organ bindings, physiological parameters from packages |
| `organism.rs` | coarse organism and connections between metabolism, heat, blood, and renal state |
| `blood.rs`, `renal.rs` | gas exchange and water/sodium balance |
| `circadian.rs`, `digestion.rs`, `interoception.rs` | sleep, sleep debt, chemical-energy flow, internal observables |
| `cell_cohort.rs`, `neural_population.rs`, `resolution.rs` | aggregate representations and resolution-change examples |
| `cognitive.rs` | scripted proposals, dispositions, intention adoption |
| `thermal.rs`, `atmosphere.rs` | heat transfer and room atmosphere |
| `rigid_body.rs`, `articulation.rs`, `contact.rs` | metric bodies, joints, collisions, grasping |
| `physics_island.rs`, `balance.rs`, `walk.rs` | interacting body groups, rest, balance, walking steps |
| `fluids.rs`, `infrastructure.rs`, `propagation.rs` | liquids, electricity/water networks, light/sound/odor propagation |
| `episodes.rs` | cooking, cleaning, dressing controllers with observations and blockers |

A module's presence means that a bounded executable mechanism and its tests exist. It does not mean that every quantity is already recorded as a separate canonical event or that the mechanism has independent physical or biological validation.

## Actual API and Current Kernel Limits

`OpenSpec` specifies a `WorldId`, `TimelineId`, and optional format requirement. `StorageLocation::sqlite` selects the file. `WorldEngine::open` returns an engine and `RecoveryReport` with status `Created` or `Recovered`.

| `CommitRequest` Constructor | Current Slice |
|---|---|
| `ingest_food` | chemical-energy input in µJ |
| `ingest_fluid` | water input in mm³ and sodium input in µmol |
| `accept_sleep_intention` | adoption of a sleep intention |
| `advance_to` | despite the name `to`, the current implementation adds the supplied number of seconds; it is not an absolute target time |
| `resolution_changed` | recording a supported representation change |
| `place_body` | persistence of a named rigid body |
| `thermal_exchange` | exchange between reservoirs using an artifact bundle |

The current `CommitReceipt` exposes `timeline_version()` and `replayed_request()`. The target protocol's extended receipt with an event range should not be assumed available. `ProjectionRequest::current()` returns a compact projection of timeline ID, version, time, and an empty-state indicator; it is not yet a universal observer-specific projection with privacy scope. Additional read methods expose organism, bodies, sleep phase, resolution, interoception, genesis, safe-stop, and replay results.

The current `ResolutionChanged` stores source/target IDs and a deterministic seed; commit changes the declared resolution ID. This record does not perform a universal transformation of physical state. Its presence therefore does not demonstrate executable renal refinement with lift/projection and conservation checks.

### Timeline Format and the Transition to Canonical Physiology

New timelines currently use `AggregateV1`: the runtime advances physiology second by second but stores an aggregate transition per request. Partitioning a request can produce the same final state without an identical complete event stream. A matching state hash therefore does not satisfy canonical transition parity.

`CanonicalPhysiologyV2` is reserved in the enum, but creating it returns `UnsupportedTimelineFormat`. `OpenSpec::with_format` checks the requirement before recovery writes; a mismatch produces `FormatMismatch`, while an unknown durable format produces `IncompatibleStorage`. Old metadata without a format field is read as aggregate without adding that column. The target format, separate request version, and event sequence are described in [ADR-0016](docs/adr/0016-canonical-physiology-replay-format.md).

Historical records use versioned state hashes; the current code contains versions 1–4, with the latest incorporating renal quantities. `events()` preserves archived records' original units. New renal sodium deltas use `umol`; earlier `centi_umol` records are not rewritten, and incompatible replay reports `IncompatibleQuantityUnit`. Audit also persists `SafeStop`. Reading a snapshot or obtaining a retry receipt does not prove successful replay of the old format.

### Admission and Executable Artifacts

`MechanismContract::from_json` performs structural parsing compatible with earlier contracts; full JSON Schema validation occurs in `ArtifactBundle::admit`. Admission binds the contract to the exact program digest and checks the supported ABI and program fields. It is not a universal test of the parameters' scientific validity.

The thermal reference already uses an archived program and audit replay. The new `renal-fluid-v1` ABI creates a renal proposal from archived bytes but is not yet activated on the timeline. Its reference contract and program are in `contracts/fixtures/mechanisms/renal-corrective-excretion-v1.*`; bounds and rates are checked during admission. An independent one-step numerical anchor starts with 42,250,000 mm³ of water and 455,000 µmol of plasma Na. Rates of 17 mm³/s and 1 µmol/s produce 42,249,983 mm³ and 454,999 µmol, with urine receiving the corresponding amounts. This is a synthetic example, not a measured kidney result.

## Legacy World: Transport, Packages, and Operations

Requests pass through the generated gRPC service, [rpc.rs](world/src/rpc.rs), the bounded actor in [actor.rs](world/src/actor.rs), legacy [engine.rs](world/src/engine.rs), and [store.rs](world/src/store.rs). Defaults are a 64-entry command queue, a 256-entry event broadcast, and a 100 ms actor tick. Queue overflow returns `Busy`; this actor tick does not define the new kernel's canonical one-second semantics.

`Handshake` negotiates the connection; `ExecuteCommand` submits a command envelope; `GetCommandResult` retrieves the result after a timeout; `GetPerception` reads available observations; `SubscribeEvents` resumes history from `after_seq`; and `Health` reports service status. Command families include movement, actions, inspection, plan management, waiting, and phone actions. These wire commands belong to legacy V1 and do not imply implementation of future digital devices or society.

`test-room-v1` is a small room with `bed` and `work_desk` anchors and a lamp; the fixture separates observed and hidden properties. `apartment-v1` describes a 48 m² apartment in Novosibirsk, room topology, anchors, objects, sensory descriptions, and an SVG map. Weather comes from Open-Meteo; the manifest specifies 15-minute polling, a 45-minute stale threshold, and a 6-hour fallback threshold. These descriptions belong to the legacy package. `apartment-v2` in kernel tests is a separate acceptance scenario, not an automatically upgraded `apartment-v1` manifest.

In [server.rs](world/src/server.rs), the UDS is created with permissions `0600`; an existing socket path is not overwritten. Cleanup verifies the identity of the created socket. The `serve` command has a working Unix implementation; on Windows it returns an error requiring Unix. Use Linux or WSL for the full transport/C++ integration scenario. A passing Windows workspace run does not prove that Unix-only tests executed.

## Phase 3 Physiology

Slice 3.1, `cardiorespiratory.gas-exchange`, stores blood volume, hemoglobin tetramers, arterial O2, venous CO2, MAP, and lung diffusion. Within the current coarse envelope, demand becomes O2 consumption, CO2 production, and replenishment. Tests cover resting saturation, oxygen overdraft, partitioning, restart, and morphotype capacity.

Slice 3.2, `renal.fluid-electrolyte`, stores total body water, plasma volume, plasma sodium, and urine water/sodium boundaries. `CommitRequest::ingest_fluid` goes through WorldEngine commit and changes the blood port. Tests cover synthetic 24-hour balance, bolus MAP response, invalid input without partial commit, restart, retry/conflict, fast/audit replay, exact artifact digests, units, and durable SafeStop. This is a coarse resting-adult envelope; nephron detail, endocrine coupling, pathology, oral absorption, and validated fine resolution are not implemented.

Closing 3.2 requires independent empirical time series with an evidence ledger, known boundary inputs, uncertainty, and a calibration/validation split; synthetic integration or one replay hash is insufficient. Subsequent planned slices cover digestive/liver/metabolism, endocrine signaling, active thermoregulation/skin, musculoskeletal fatigue/pain, and excretion/hygiene/microbiome.

### Empirical Data Status

[renal-validation-evidence.md](docs/research/renal-validation-evidence.md) reviews water/saline experiments and additional ex-vivo studies. Pedersen 2010 is represented by a local XML table and JSON urine-flow transcription; these data have not been admitted as a validation dataset. Oral water loading requires a known absorption/input profile, while the current `ingest_fluid` changes plasma state immediately. One protocol cannot replace another, nor can an excretion rate be tuned to match without the appropriate mechanism.

Weissenbacher 2019 is considered an additional ex-vivo source on isolated human kidneys. These are discarded donor organs on a perfusion machine, so the observations do not replace healthy resting validation or prove Neko physiology. The reviewed supplements did not contain the required complete numeric water/Na time series and individual boundary inputs. A [draft data request](docs/research/renal-data-request.md) has been prepared; according to the evidence record, it has not been sent. This document neither introduces new runtime behavior nor authorizes sending that request.

The current 3.2 scope excludes oral absorption and endocrine coupling. Closing the gate requires compatible renal measurements, an executable upgrade with conservation/continuity, and the full replay matrix. A resolution-upgrade planning digest in a fixture is not a ready-to-run artifact.

## What the Tests Actually Demonstrate

| Area | Main Suites in `causal-kernel/tests/` | Limits of the Conclusion |
|---|---|---|
| Public API, commit | `public_api.rs`, `writer_commit.rs` | available operations and write rules |
| Timeline/replay | `audit_replay.rs`, `restart_replay.rs`, `replay_basis.rs`, `transition_evidence.rs`, `timeline_format.rs` | specific verified formats and chains, not every future mechanism |
| Admission/thermal | `artifact_admission.rs`, `physiology_admission.rs`, `thermal_proposal.rs`, `thermal_replay.rs` | schema/program validation, numerical anchors, archived execution |
| Morphotypes | `morphotype.rs`, `anatomy_binding.rs`, `parameter_realism.rs` | data-driven bindings and parameter constraints |
| Physiology | `blood_gas.rs`, `renal.rs`, `renal_replay.rs`, `renal_units.rs`, `renal_program.rs` | coarse behavior and regression evidence, not full empirical validation |
| Embodiment | `rigid_body.rs`, `collision.rs`, `friction.rs`, `walk_control.rs`, `liquid_pour.rs`, `apartment_v2.rs` | bounded physics/control scenarios |
| Cognition/resolution | `cognitive_gate.rs`, `sensory_transduction.rs`, `resolution_transition.rs` | scripted acceptance and supported examples |

`worker_invariance.rs` runs independent engines in threads and compares simulated time. It does not demonstrate an identical causal timeline with 1/N workers inside one engine; the Phase 3 plan explicitly leaves that gate open. Likewise, a record that Phase 2 closed does not mean that the entire long-term physical-fidelity target is complete.

`world/tests/` contains package validation, recovery, UDS, metric geometry, contract-fixture, and public-documentation checks. `public_repository.rs` checks structure and Markdown reachability from README; `phase0_contracts.rs` checks schemas/fixtures and architectural constraints. `proto/tests/wire_compat.rs` protects protobuf compatibility. No percentage coverage threshold is configured; requirements are expressed through scenarios and the evidence matrix.

## Quality Gates and Contribution Workflow

Before making changes, read `INVARIANTS.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `ROADMAP.md`, and relevant ADRs. For persistence/API work, add `PROTO.md`; for physiology, add `WORLD_V1.md`, the coverage matrix, Phase 3 plan, and research evidence. Record the observable outcome, public seam, acceptance evidence, non-goals, and rollback before changing runtime code.

Work in a narrow vertical slice: add a failing public-seam test, confirm the red result, make the smallest green change, run focused tests, then run the full gate and review the diff for compatibility, units, determinism, security, and documentation drift.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cmake -S brain -B build/brain -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build/brain --parallel
ctest --test-dir build/brain --output-on-failure
```

For a quick legacy-service run on Linux/WSL, create a separate runtime-data directory:

```bash
mkdir -p /tmp/makise-dev
cargo run -p makise-world -- verify-package world-packages/test-room-v1/manifest.json
cargo run -p makise-world -- status /tmp/makise-dev/world.db world-packages/test-room-v1/manifest.json test-makise bed
cargo run -p makise-world -- serve /tmp/makise-dev/world.sock /tmp/makise-dev/world.db world-packages/test-room-v1/manifest.json test-makise bed
```

`verify-package` validates a manifest; `status` opens or creates a database and prints perception JSON; `serve` starts a long-running WorldService. These are legacy-runtime commands. The new kernel is currently accessed through its Rust library API and integration tests.

For C++ on Ubuntu/Debian, install `cmake`, `g++`, `libprotobuf-dev`, `protobuf-compiler`, `protobuf-compiler-grpc`, and `libgrpc++-dev`. CI runs on Ubuntu: the Rust job runs fmt, Clippy with `-D warnings`, and workspace tests; the C++ job first builds `makise-world`, then runs CMake and CTest. Relevant checks for documentation-only changes are `git diff --check` and `cargo test -p makise-world --test public_repository`; schema changes additionally require `phase0_contracts`.

Formatting conventions are UTF-8, LF, a final newline, four spaces in Rust, and two spaces in JSON/YAML/TOML. Rust uses `rustfmt`; C++ uses C++20 and warnings-as-errors for GNU/Clang. Generated files belong in `target/` and `build/`.

## Target Civilization and Release Criteria

[CIVILIZATION.md](CIVILIZATION.md) describes future connections between actions, devices, and institutions. A `CodeArtifact` stores immutable bytes and source/build/release lineage. A `CapabilityGrant` provides a limited, revocable right. An `Organization` stores roles and authority but has no Consciousness of its own. `Possession` means actual control, `TitleClaim` means a recognized claim, `ServiceContract` describes obligations, and `DesignArtifact` describes an intended structure. None of these objects creates a completed result without physical or digital work.

Target scenarios include a character writing and publishing an application, an organization delivering a service, a house built from materials and labor, and a data center connecting power, cooling, hardware, software, networks, and contracts. Executable sandbox infrastructure, a marketplace, and institutions belong to later phases and are not currently available as complete services.

According to [ROADMAP.md](ROADMAP.md), final acceptance includes a 365-day integration/replay run, ensembles over 10/30/80-year horizons and rare events, and 30 calendar days of shadow/closed launch with a real LLM, restarts, and provider failures. The target workstation configuration is Human + Neko and two active Consciousnesses at 1:1 on 16 cores/32 GB, with World Engine using ≤24 GB. These are release targets, not published benchmarks for this checkout.

The future panel must show units, provenance, uncertainty, resolution, and causal traces. Distributed authoritative state requires a separate post-V1 ADR. An external model-improvement control plane may prepare candidates, but activation requires validation, shadow evidence, approval, and an admin commit with old/new digests and a rollback target. Diegetic character applications do not become World Engine code.

PRs must describe the observable outcome, scope/non-goals, phase/gate, causal and compatibility impact, red/green evidence, validation commands, documentation/schema/fixture updates, and rollback or `SafeStop`. Commit messages follow Conventional Commits (`feat(kernel): ...`, `fix(world): ...`, `test: ...`, `docs: ...`). Do not commit secrets, runtime databases, generated output, private conversations, or machine-specific paths. The project is licensed under AGPL-3.0-only.

## Source-of-Truth Hierarchy

When sources conflict, precedence is: `INVARIANTS.md`, accepted ADRs, `ARCHITECTURE.md` and `PROTO.md`, domain specifications and roadmap, then historical/superseded documents. `README.md` provides the public overview and quick start; `CONTRIBUTING.md` describes the contribution workflow; `SECURITY.md` describes the threat model and disclosure policy. This file consolidates detailed project context, but normative rules remain in those sources.

| Question | Where to Read |
|---|---|
| Purpose and release outcome | [VISION.md](VISION.md) |
| Precise domain terminology | [CONTEXT.md](CONTEXT.md) |
| Mandatory constraints | [INVARIANTS.md](INVARIANTS.md) |
| Components and authority | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Protocol, events, storage, migration | [PROTO.md](PROTO.md) |
| Target organisms and validation horizons | [WORLD_V1.md](WORLD_V1.md) |
| Actions, technology, and institutions | [CIVILIZATION.md](CIVILIZATION.md) |
| Subjective history and privacy | [MEMORY.md](MEMORY.md) |
| Security and disclosure | [SECURITY.md](SECURITY.md) |
| Phases and gates | [ROADMAP.md](ROADMAP.md) |
| Current physiology gaps | [Phase 3 plan](docs/plans/0006-phase3-everyday-physiology.md) |
| Phase evidence | [Coverage matrix](docs/coverage/phase0-coverage-matrix.md), [Phase 2 status](docs/plans/0005-phase2-slice-status.md) |
| Reasons behind architectural decisions | [ADR index](docs/adr/README.md) |
| Contributor workflow | [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md) |

When updating this overview, verify the public API against the code, status against implementation plans, and fidelity claims against the evidence ledger. Preserve the distinction between a normative requirement, a passing synthetic test, and independent empirical validation.
