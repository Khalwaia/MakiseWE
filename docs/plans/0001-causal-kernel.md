# Implementation plan 0001: минимальный V1 causal kernel

Статус: готов к исполнению после отдельного разрешения на runtime work  
Дата: 2026-08-24  
Scope: один compatibility-safe causal-kernel slice и один reference mechanism

## 1. Результат

Создать новый `makise-causal-kernel` рядом с legacy `makise-world`. Slice должен доказать, что целевой deep interface, canonical scheduling, propose/validate/commit, immutable artifacts, отдельная V1 timeline и два режима replay работают совместно до добавления biology или semantic actions.

Первый mechanism — теплообмен двух конечных thermal reservoirs. Он переносит энергию по unit-typed physical law, сохраняет суммарную энергию и может завершиться typed failure. Он не моделирует cooking, organisms, cognition, devices или institutions.

Готовый slice предоставляет только четыре публичные операции:

```rust
WorldEngine::open(OpenSpec, StorageLocation)
WorldEngine::commit(CommitRequest)
WorldEngine::project(ProjectionRequest)
WorldEngine::events(EventQuery)
```

Все остальные типы поддерживают этот boundary. Transport, CLI, actor, gRPC и legacy adapters в slice не входят.

## 2. Почему отдельный crate

Текущий `makise-world` и его SQLite schema — legacy-compatible runtime. Встраивание новой timeline в `world/src/store.rs` смешало бы две event semantics, создало риск in-place migration и усложнило rollback.

Новый workspace crate `causal-kernel/` с package name `makise-causal-kernel` получает отдельный storage magic/schema и никогда не открывает legacy DB на запись. Оба crates могут временно существовать в одном workspace, но один process не назначает их writers одной timeline. Rollback — запуск прежнего `makise-world` с прежним archive.

Production dependencies ограничены уже закреплёнными workspace crates: `rusqlite`, `serde`, `serde_json`, `sha2`, `thiserror`. Новая dependency требует отдельного обоснования до изменения lockfile.

## 3. Scope и non-goals

В scope входят:

- отдельный crate и отдельная V1 SQLite timeline;
- public API `open`, `commit`, `project`, `events`;
- `CommitRequest`, `TransitionProposal`, `CausalTransition`, `CommitReceipt`;
- unit/dimension-safe quantities с deterministic numeric encoding;
- runtime admission `MechanismContract`;
- immutable artifact registry и exact content digests;
- propose/validate/commit seam и atomic append;
- deterministic scheduler и canonical reduction order;
- fast replay и audit replay;
- reference thermal-energy mechanism;
- dual-read compatibility test для неизменённого legacy archive.

Не входят:

- изменение `makise.v1` protobuf, UDS, actor или legacy commands;
- migration или dual-write существующей DB;
- organisms, metabolism, cells, neurons, LLM, perception или memory;
- `ControlEpisode`, cooking, walking, applications, services, construction или data centers;
- resolution transitions, distributed state, snapshots или performance optimization;
- UI, network access и external side effects.

## 4. Planned layout

```text
causal-kernel/
├── Cargo.toml
├── src/
│   ├── lib.rs          # четыре операции и минимальные public types
│   ├── api.rs          # requests, receipts, queries, projections, typed errors
│   ├── quantity.rs     # dimensions, units, fixed-point values, checked arithmetic
│   ├── artifact.rs     # bundle digests, immutable bytes, admission records
│   ├── mechanism.rs    # contract admission и proposal interface
│   ├── scheduler.rs    # canonical boundaries и reduction ordering
│   ├── transition.rs   # proposals, validation reports, committed transitions
│   ├── state.rs        # authoritative state и deterministic hash
│   ├── timeline.rs     # sequence, causation, idempotency и replay orchestration
│   ├── storage.rs      # отдельная SQLite schema и atomic transactions
│   └── thermal.rs      # единственный reference mechanism
└── tests/
    ├── public_api.rs
    ├── thermal_conservation.rs
    ├── deterministic_execution.rs
    ├── replay.rs
    ├── failure_atomicity.rs
    └── legacy_compatibility.rs

contracts/fixtures/mechanisms/two-reservoir-thermal-exchange.json
```

Модули остаются private; `lib.rs` экспортирует только deep interface и необходимые request/response/value types. Test-only hooks не становятся production API.

## 5. Domain records первого slice

### 5.1 CommitRequest

Минимальный envelope содержит:

- `request_id`;
- schema version;
- timeline ID и expected timeline version;
- caller/authority для initial local kernel policy;
- typed intent `AdvanceTo { canonical_time }`;
- causation/correlation IDs;
- canonical payload digest.

Повтор того же `request_id` и payload возвращает исходный receipt. Повтор ID с другим payload возвращает `IdempotencyConflict`. Request, receipt и transport metadata не добавляют simulation transitions, зависящие от размера client batches.

`AdvanceTo` является execution demand, а не причиной thermal physics. Поэтому его `request_id`, число requests и correlation metadata не входят в `CausalTransition.causes` или transition ID. Причинами thermal transition являются previous canonical state/transition, admitted mechanism и interval boundary. Receipts различаются между одним и многими client requests, но causal transition stream остаётся идентичным.

### 5.2 TransitionProposal

Proposal не является state. Он содержит:

- mechanism/artifact digests;
- точный canonical interval;
- sorted read/write sets;
- unit-typed proposed deltas;
- preconditions;
- uncertainty/error bounds;
- conservation claims;
- deterministic ordering key;
- optional typed failure evidence.

Worker API возвращает proposal над immutable state view. Только writer проверяет proposal против текущего head.

### 5.3 CausalTransition

Committed record следует [PROTO.md](../../PROTO.md): transition/event IDs, causes, canonical interval, exact artifact digests, deltas, uncertainty, conservation report, lineage refs, fidelity/evidence refs, previous/resulting hashes и schema version. Поля, неприменимые первому mechanism, имеют явное typed absence, а не выдуманные defaults.

### 5.4 CommitReceipt

Receipt содержит committed event range, transition IDs, resulting timeline version/state hash, simulation time и `replayed_request: bool`. Validation failure не создаёт receipt со статусом success и не меняет head.

## 6. Deterministic quantity model

Первый slice не использует binary floating point в authoritative state или transition deltas. Quantity хранит checked signed `i64` magnitude в canonical scale и compile-time/runtime dimension descriptor. Intermediate multiplication использует checked `i128`; result обязан пройти declared rounding и вернуться в `i64`. Overflow возвращает typed failure до commit. Более широкий numeric range требует отдельного representation/version upgrade, а не silent encoding change.

Для thermal fixture минимально нужны:

- energy: microjoule;
- thermodynamic temperature: millikelvin;
- heat capacity: microjoule per millikelvin;
- thermal conductance: microjoule per millikelvin-second;
- time: nanosecond с mechanism boundary, кратной одной second.

Authoritative reservoirs хранят internal energy и heat capacity. Temperature является объявленной projection `energy / heat_capacity`, а не отдельно мутируемым state. Mechanism переносит равные по модулю противоположные energy deltas. Conservation проверяется независимо writer-ом: `sum(delta_energy) == 0` без tolerance для fixed-point records.

Contract обязан определить validity range, temporal resolution, rounding policy, maximum stable transfer и uncertainty. Если proposal выходит за них, writer отклоняет его; он не clamp-ит и не подбирает правдоподобное значение.

## 7. Canonical scheduling rule

Thermal mechanism имеет intrinsic boundaries через каждую canonical second от timeline epoch. `AdvanceTo` может охватывать одну или много boundaries; scheduler всегда выпускает те же ordered one-second proposals. Несколько `AdvanceTo` requests, границы которых совпадают с canonical boundaries, дают ту же causal transition sequence, что один request до того же final time.

Ordering key фиксируется tuple:

```text
(interval_end, causal_domain, mechanism_digest, entity_key, proposal_kind)
```

Keys сериализуются канонически и сравниваются bytewise. Hash maps, thread completion order, SQLite row order без `ORDER BY` и wall clock не определяют semantics. Operational `committed_at` сохраняется вне canonical causal payload и не входит в transition ID, causes, replay equality или state hash.

Для первого slice worker counts `1` и `4` достаточно, хотя один mechanism почти не даёт speedup: тест доказывает отсутствие зависимости writer-а от completion order. Phase 1 расширит matrix до `1` и `16`.

## 8. Propose, validate, commit

Один `commit` проходит следующие этапы:

1. Проверить schema, timeline, expected version, authority и idempotency вне write transaction.
2. Построить canonical boundary schedule от current time до requested time.
3. Получить proposals из exact admitted mechanism artifact над immutable state view.
4. Отсортировать proposals canonical key.
5. Перед каждым применением повторно проверить read version, preconditions, units/dimensions, artifact compatibility, validity range, checked arithmetic и conservation.
6. Рассчитать resulting authoritative state и canonical state hash в памяти.
7. В одной SQLite transaction записать transitions, request receipt, new head и artifact references.
8. Подтвердить `CommitReceipt` только после durable transaction commit.

Ошибка на любом proposal отменяет всю request transaction. Ни transition prefix, ни receipt, ни новый head не остаются. `SafeStop` применяется только к failures, для которых нормативный contract требует остановки timeline; malformed client request остаётся typed rejection без state mutation.

## 9. Storage и artifacts

V1 DB получает собственные application ID, schema version и tables минимум для timeline metadata, admitted artifact bytes, transitions, request receipts и current head. `open` отклоняет legacy DB по magic/schema и никогда не мигрирует её автоматически.

Initial `OpenSpec` содержит world/timeline identity, initial unit-typed reservoir state и artifact bundle. Bundle разделяет:

- exact canonical JSON bytes `MechanismContract` и их `contract_digest` в registry envelope;
- exact executable mechanism program bytes и их `content_digest`, объявленный contract;
- parameter/evidence bytes и их отдельные digests.

Такое разделение исключает circular self-hash: `content_digest` не вычисляется по JSON, внутри которого он находится. Thermal program первого slice — узкий declarative `thermal-exchange-v1` instruction artifact над checked typed operations, а не native function identity или semantic action. Kernel допускает только известные versioned instruction semantics; unknown opcode/ABI отклоняется. Transition хранит contract digest и program content digest.

При создании timeline все bytes сохраняются immutable по SHA-256 digest. При reopen engine:

1. проверяет DB integrity/head hash;
2. пересчитывает digest каждого required artifact;
3. восстанавливает state fast replay;
4. возвращает `RecoveryReport` с verified range и required artifacts.

Artifact с существующим digest и другими bytes невозможен; semantic version не заменяет digest. Если registry/head остаются целыми, missing program bytes или mismatch создаёт один system-authored `SafeStopEntered`, после которого разрешены только diagnostics/export. Если целостность storage не позволяет безопасно append-ить это событие, `open` возвращает sealed recovery error и read-only diagnostics, не продолжая timeline и не притворяясь, что durable `SafeStop` был записан. Оба случая следуют [SECURITY.md](../../SECURITY.md).

Snapshots не нужны этому slice: reopen может replay-ить весь короткий log. Их добавление допускается только после доказанного baseline и не меняет authoritative semantics.

## 10. Replay modes

Fast replay проверяет sequence, hash chain, artifact presence и применяет committed deltas. Audit replay загружает exact contract/mechanism bytes, повторяет canonical proposal и writer validation, затем сравнивает:

- transition identity/order;
- interval and causes;
- unit-typed deltas;
- uncertainty/error bounds;
- conservation report;
- resulting state hash.

Audit mismatch возвращает diagnostic `SafeStop`; текущая версия mechanism не подменяет archived artifact. Оба режима работают только read/verify над committed external receipts; первый slice вообще не содержит external executor.

## 11. Implementation sequence

Каждый шаг — отдельный reviewable change. Следующий начинается только после green completion criterion текущего.

### Step 0 — зафиксировать baseline

Действия:

- сохранить `git status` и не включать unrelated passive-mechanism changes;
- запустить current workspace gates;
- сохранить hash/readability representative legacy DB/log fixture либо создать test fixture через public legacy API без изменения production schema;
- убедиться, что новый crate name/path не конфликтует с package metadata.

Completion criterion: baseline commands green; legacy evidence воспроизводимо; runtime files не изменены.

#### Step 0 evidence — 2026-08-24

- Baseline зафиксирован поверх существующего незакоммиченного Phase 0/passive-mechanism diff; до Step 0 были modified `.gitignore`, корневые normative Markdown, ADR/coverage docs, `world/src/engine.rs`, `world/src/lib.rs`, `world/tests/public_repository.rs`; untracked `AGENTS.md`, `CIVILIZATION.md`, ADR-0012/0013, этот plan и `world/src/mechanisms/`. Эти изменения не включены в Step 0.
- `cargo metadata --no-deps --format-version 1` подтверждает, что workspace package `makise-causal-kernel` и path `causal-kernel/` свободны.
- `public_legacy_reader_preserves_archive_bytes` создаёт representative legacy DB/log через публичный `makise_world::WorldEngine`, повторно читает events тем же public API и требует равенства SHA-256 bytes до/после reopen.
- Baseline gates: `git diff --check`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace --all-targets` — green.
- Step 0 меняет только compatibility test и этот execution record; production runtime не изменён. Следующий разрешённый slice — Step 1.

### Step 1 — создать public seam и red contract tests

Действия:

- добавить workspace member `causal-kernel`;
- определить четыре операции и минимальные opaque IDs/errors;
- написать compile/public behavior tests для create/open, empty projection и stable event pagination;
- запретить публичный доступ к mutable state/storage.

Completion criterion: тесты сначала подтверждённо red из-за отсутствующего behavior, затем green; legacy tests неизменны.

### Step 2 — quantities и state hashing

Действия:

- реализовать checked fixed-point quantities/dimensions;
- определить canonical serialization без platform-dependent layout;
- добавить independent dimensional, overflow и hash test vectors;
- определить initial two-reservoir state и projection.

Completion criterion: invalid dimension/overflow отклоняется; одинаковый logical state имеет один hash на повторных runs; изменение unit/value меняет hash.

### Step 3 — artifact admission и MechanismContract

Действия:

- добавить thermal contract fixture и индекс;
- определить bundle envelope и независимые `contract_digest`/`content_digest` без self-hash;
- принимать exact contract/program bytes, пересчитывать digests, запрещать unknown/missing required fields и program ABI;
- проверить units, validity range, scheduling, conservation и validation evidence;
- сохранить admitted bytes immutable в отдельной V1 DB.

Completion criterion: valid fixture admitted; mutated contract/program byte, wrong declared digest, incomplete contract, unknown ABI и incompatible unit rejected before timeline advancement.

### Step 4 — scheduler и thermal proposal

Действия:

- реализовать intrinsic one-second boundaries;
- реализовать pure proposal над immutable state;
- добавить independent expected delta examples для hotter/cooler/equilibrium cases;
- проверить equal-and-opposite transfer и canonical ordering.

Completion criterion: proposal сам не меняет state; fixture examples совпадают; conservation выполняется точно; out-of-envelope case typed-fails.

### Step 5 — atomic writer и idempotency

Действия:

- реализовать writer validation и SQLite transaction;
- записывать `CausalTransition`, state/hash head и receipt атомарно;
- добавить same-ID retry, conflicting-ID payload, expected-version conflict и injected failure tests;
- доказать отсутствие partial prefix после failure/reopen.

Completion criterion: только validated transaction меняет head; retry возвращает исходный receipt; failure оставляет byte-equivalent logical timeline.

### Step 6 — restart, fast replay и audit replay

Действия:

- реализовать reopen/recovery report;
- реализовать fast и audit replay через один transition validator;
- проверить artifact digest mismatch, missing bytes и tampered delta/hash;
- убедиться, что audit использует archived exact bytes.

Completion criterion: clean reopen восстанавливает тот же state/hash; оба replay совпадают; каждая corruption fixture детерминированно диагностируется и не продолжает timeline.

### Step 7 — determinism и legacy compatibility gate

Действия:

- сравнить один `AdvanceTo(+60 s)` с 60 запросами по `+1 s`;
- повторить после restart и с worker counts `1`/`4`;
- сравнить canonical causal payloads, transition IDs/order и final state hash; request receipts и operational append timestamps сравниваются только по собственной contract semantics;
- открыть representative legacy archive прежним reader до и после V1 run;
- проверить, что V1 open отклоняет legacy path и legacy file hash не изменился.

Completion criterion: все результаты byte-for-byte совпадают там, где contract требует parity; legacy artifacts readable и immutable.

### Step 8 — documentation и gate

Действия:

- обновить coverage matrix фактическим evidence без завышения realism claim;
- обновить README/PROTO только если implemented behavior изменяет их current-status statements;
- записать test commands/results и explicit non-goals;
- провести review по `AGENTS.md`.

Completion criterion: links, focused tests, clippy и workspace tests green; diff содержит только causal-kernel slice, contract fixture и необходимые docs/metadata.

## 12. Acceptance matrix

Slice принят только при одновременном выполнении условий:

| Свойство | Доказательство |
|---|---|
| Public boundary | только `open`, `commit`, `project`, `events` предоставляют state behavior |
| Partition invariance | `+60 s` одним и 60 requests дают identical canonical causal payloads/IDs/order и state hash; receipts могут отражать разные request boundaries |
| Restart invariance | split run с reopen совпадает с uninterrupted run |
| Worker invariance | worker counts `1` и `4` дают identical stream/hash |
| Conservation | thermal transfer сохраняет total energy exactly |
| Unit safety | incompatible dimensions и overflow отклоняются до commit |
| Artifact identity | modified/missing bytes дают digest error или `SafeStop`, без substitution |
| Replay parity | fast state/hash равны audit state/hash |
| Atomicity | failure на последнем proposal не оставляет первые proposals |
| Idempotency | same request возвращает receipt; conflicting payload rejected |
| Compatibility | legacy reader читает прежний archive; archive hash неизменен |
| Scope | отсутствуют biology, semantic actions, transport и external effects |

## 13. Verification commands

Во время реализации сначала запускается focused test текущего шага. Финальный gate:

```bash
git diff --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p makise-causal-kernel --all-targets
cargo test -p makise-world --test public_repository
cargo test --workspace --all-targets
```

Если test environment создаёт DB, paths должны находиться в `tempfile`/`/tmp`, не в repository. Никакой test не использует wall clock, network или shared mutable global state как causal input.

## 14. Rollback

До routing/cutover rollback состоит из удаления нового workspace member и запуска legacy executable; legacy data не менялись. После будущего routing change rollback выбирает legacy executable/archive либо предыдущий V1 release, но никогда не конвертирует V1 transitions обратно в legacy events.

Любая необходимость писать в legacy DB, менять старые protobuf field numbers или dual-write две timelines отменяет этот plan и требует отдельного migration ADR.

## 15. Stop condition и следующий slice

Работа заканчивается, когда causal kernel и один thermal mechanism прошли acceptance matrix. Не добавлять «заодно» metabolism, organisms, cooking, devices, marketplace или Phase 7 entities.

Следующим отдельным plan может стать минимальный `ControlEpisode` над уже доказанным kernel: neutral intention вроде «удерживать измеряемую температуру reservoir в диапазоне» с observation, attempted control, interruption и replanning. Он не должен называться cooking и не должен содержать completion mutation. Начинать его можно только после отдельного causal-kernel gate commit.
