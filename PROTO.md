# Protocol and persistence design Makise V1

Статус: нормативный Phase 0 design; wire implementation следует отдельной migration
Дата: 2026-08-19
Связанные документы: [ARCHITECTURE.md](ARCHITECTURE.md), [INVARIANTS.md](INVARIANTS.md), [ADR-0010](docs/adr/0010-content-addressed-artifacts.md)

## 1. Module boundary

Public domain API состоит из `open`, `commit`, `project` и `events`, определённых в [ARCHITECTURE.md](ARCHITECTURE.md). RPC/HTTP/CLI являются adapters этих операций и не добавляют mutation endpoints.

`CommitRequest` envelope содержит:

- `request_id`, schema version и expected world/timeline version;
- caller identity/authority и target timeline;
- canonical simulation interval либо внешний cause timestamp;
- ровно один typed intent: advance time, stimulus, cortex response, action, resolution change или admin intent;
- referenced artifact digests и deterministic seed material, если оно требуется;
- causation/correlation IDs.

`CommitReceipt` возвращает committed event range, transition IDs, resulting version/state hash, simulation time и idempotent replay marker. Validation failure не создаёт частичный state.

`ProjectionRequest` задаёт observer/Consciousness, projection kind, simulation point и privacy scope. Projection включает units, uncertainty, provenance и active resolution, но не выдаёт недоступный raw state.

`EventQuery` использует timeline ID, `after_seq`, bounds, event families и page limit. Pagination имеет стабильный ordering по monotonically increasing event sequence.

## 2. Canonical transition record

Каждая transition хранит:

```text
transition_id, event_seq, timeline_id
causes[] and correlation_id
canonical_simulation_interval { start, end }
mechanism/model/resolution/solver content digests
unit_typed_deltas[]
uncertainty_and_error_bounds
conservation_report
representation_lineage_refs[]
deterministic_seed_ref (when used)
previous_state_hash, resulting_state_hash
schema_version, committed_at
```

Physical quantities сериализуются как `{value, unit}` с contract-defined numeric encoding. Dimensionless values дополнительно несут semantic kind; голое число не становится authoritative quantity. Event payload хранит facts и reasons, не художественный summary.

## 3. Required event families

- `ExternalStimulusCommitted`;
- `CanonicalIntervalAdvanced`;
- `MechanismTransitionCommitted`;
- `ResolutionChangeRequested`, `ResolutionChanged`, `ResolutionChangeRolledBack`;
- `CortexFrameProjected`, `CortexProposalRecorded`, `CognitiveDispositionRecorded`, `CognitiveStateAdopted`;
- `MotorPlanValidated`, `PhysicalActionTransitioned`;
- `OrganismCreated`, `ConsciousnessAttached`/`Detached`;
- `CapacityExceeded`, `ConservationViolation`, `NonConvergence`, `SafeStopEntered`;
- `ArtifactRegistered` and migration/recovery evidence.

`CortexProposalRecorded` не содержит state delta. `CognitiveStateAdopted` обязан причинно ссылаться на disposition `Accepted`. `ResolutionChanged` содержит old/new contract digests, seed, lift/projection evidence, conserved quantities, observable comparison, lineage и rollback handle.

## 4. Replay modes

Fast replay проверяет hash-chain и применяет committed unit-typed deltas. Audit replay загружает artifacts по digest, повторно запускает canonical mechanisms и сравнивает deltas, uncertainty, conservation и resulting hash.

Partitioning, worker count и wall-clock mode не являются event semantics. Canonical reduction ordering входит в mechanism scheduling contract. Missing artifact, digest mismatch или расхождение audit replay создаёт `SafeStop`; replay не выбирает «похожую» текущую model version.

## 5. Storage and timelines

Новая многомасштабная V1 использует отдельную timeline и DB. Прежняя DB монтируется immutable archive; никакой migration step не меняет её in place. Snapshots — content-addressed acceleration artifacts и всегда проверяются against event hash-chain.

Timeline metadata связывает world specification, package manifest, artifact roots, schema versions и parent/fork provenance. Schema не ограничивает entity count. Storage admission возвращает явный `CapacityExceeded` до partial commit.

## 6. Compatibility migration

Migration выполняется четырьмя обратимыми стадиями:

1. **Expand** — добавить V2 protocol envelopes, artifact store, новые event/state schemas и dual readers, сохранив текущий `makise.v1` wire API.
2. **Migrate** — скомпилировать legacy world/package в immutable `legacy-makise` bundle; назначить прежнему single agent явные `organism_id` и `consciousness_id`; записывать только новую timeline.
3. **Verify** — доказать чтение старых protobuf wire fixtures, world packages, SQLite DB/snapshots и event logs; сравнить legacy projections и archive hashes.
4. **Contract** — удалить только dual-write/temporary migration tooling после release evidence. V1 reader и immutable archive в этой работе не удаляются.

Rollback переключает запуск на legacy executable/archive либо на предыдущий new-V1 release. Он не downcast-ит и не переписывает новые biological events.

## 7. Failure and idempotency

Повтор `request_id` с тем же canonical payload возвращает исходный receipt; тот же ID с другим payload отклоняется. Optimistic version mismatch, invalid units, missing preconditions, privacy violation, non-convergence, capacity failure и conservation failure имеют разные typed errors.

Timeout транспорта не означает failure commit. Client сначала запрашивает receipt/events по ID. Subscriber после lag продолжает с durable `after_seq`. Recovery report перечисляет verified snapshot, replayed range, required artifacts, pending safe stops и никаким образом не запускает cognition.

## 8. Phase 0 schemas versus wire protocol

JSON Schemas в [contracts/schemas](contracts/schemas) определяют artifact contracts и fixtures до runtime. Они не являются разрешением менять существующий protobuf в Phase 0. Wire evolution начинается только по migration sequence после contract gate и сохраняет нынешние field numbers/readers.
