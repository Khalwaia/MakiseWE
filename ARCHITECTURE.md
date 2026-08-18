# Архитектура Makise V1

Статус: нормативная архитектурная база Phase 0
Дата: 2026-08-19
Связанные документы: [CONTEXT.md](CONTEXT.md), [VISION.md](VISION.md), [WORLD_V1.md](WORLD_V1.md), [PROTO.md](PROTO.md), [INVARIANTS.md](INVARIANTS.md)

## 1. Глубокий модуль World Engine

Вся объективная мутация скрыта за четырьмя операциями:

```rust
impl WorldEngine {
    pub fn open(
        spec: OpenSpec,
        storage: StorageLocation,
    ) -> Result<(Self, RecoveryReport), OpenError>;

    pub fn commit(
        &mut self,
        request: CommitRequest,
    ) -> Result<CommitReceipt, CommitError>;

    pub fn project(
        &self,
        request: ProjectionRequest,
    ) -> Result<Projection, ProjectionError>;

    pub fn events(
        &self,
        query: EventQuery,
    ) -> Result<EventPage, ReadError>;
}
```

`commit` — единственный mutation path для времени, stimuli, LLM responses, actions, resolution changes и admin intents. Transport adapters, schedulers, panel, memory и model providers не получают mutable state. Внутреннее разрешение механизма не входит в caller API: вызывающая сторона использует stable causal ports и observables.

Phase 0 описывает этот интерфейс, но не заменяет им существующий runtime. Реализация и compatibility adapter начинаются только после gate.

## 2. Владение состоянием

| Данные | Авторитетный владелец |
|---|---|
| geometry, matter, fields, organisms, biology, neural state | World Engine |
| committed causal transitions и artifact digests | append-only world timeline |
| субъективное восприятие и память Consciousness | отдельный consciousness stream |
| model output до gate | immutable CortexProposal event |
| принятые goals/intentions/commitments | cognitive state после Accepted transition |
| identity values и morphotype packages | content-addressed artifacts |
| удобные UI шкалы | неавторитетные projections |

Память не исправляет objective state. Objective event не становится субъективной памятью без доступного perception. Администратор отправляет intent через `commit`, а не пишет DB.

## 3. MechanismContract

Механизм загружается только при наличии всех полей:

- `mechanism_id`, semantic `version` и `content_digest`;
- causal inputs/outputs и read/write sets;
- authoritative state variables с units и допустимым dimension kind;
- resolution/representation и spatial/temporal scales;
- canonical scheduling rules;
- observable projections;
- parameters и units;
- provenance category для механизма и каждого параметра;
- uncertainty/error model и validity range;
- conservation rules;
- failure/non-convergence policy;
- resolution-upgrade paths;
- validation scenarios.

Schema: [mechanism-contract-v1.schema.json](contracts/schemas/mechanism-contract-v1.schema.json). Неизвестное, отсутствующее или несовместимое поле является load error; runtime не дополняет неполный контракт эвристикой.

## 4. ResolutionContract

Разрешение объявляет represented entities, aggregation/refinement, coarse-to-fine state lift, fine-to-coarse projection, conserved quantities, observable continuity, uncertainty transformation, triggers/preconditions, compute estimate, rollback и artifact compatibility.

Lift может детерминированно создавать individual entities из seed, зафиксированного в `ResolutionChanged`. Он сохраняет mass, charge, substance amounts, entity counts и объявленные moments. Projection сохраняет lineage/provenance, поэтому повторный refinement продолжает причинную историю, а не создаёт новую популяцию. Mixed-resolution interaction идёт только через causal ports.

Schema: [resolution-contract-v1.schema.json](contracts/schemas/resolution-contract-v1.schema.json).

## 5. MorphotypeDefinition

Morphotype package содержит собственные anatomy graph, development program, organ bindings, physiological parameters и validation fixtures. Общие mechanisms подключаются ссылками по digest, но package не наследует другой morphotype.

Runtime registry индексирует packages произвольными IDs и разрешает bindings из данных. В runtime запрещены `is_neko`, enum `Human | Neko` и branches по известным morphotype IDs. Human и Neko fixtures демонстрируют два root definitions; добавление третьего не меняет `WorldEngine`.

Schema: [morphotype-definition-v1.schema.json](contracts/schemas/morphotype-definition-v1.schema.json).

## 6. Cognitive pipeline

```text
Perception + interoception + memory + affordances
  -> CortexFrame
  -> LLM/scripted CortexProposal
  -> CognitiveGate(neural state, identity, traits, memory, commitments, feasibility)
  -> CognitiveDisposition
  -> [Accepted only] adopted cognitive state
  -> motor plan
  -> physical validator
  -> simulated physical outcome
```

`CortexProposal` не содержит physical/biological deltas. `CognitiveDisposition` имеет status `accepted`, `rejected`, `deferred` или `needs_revision`, reasons и evidence refs. Rejected/deferred/revision decisions не могут содержать applied state transition. Даже accepted proposal не меняет мир сам: gate создаёт отдельную canonical transition только для разрешённых cognitive fields.

Schemas: [cortex-proposal-v1.schema.json](contracts/schemas/cortex-proposal-v1.schema.json), [cognitive-disposition-v1.schema.json](contracts/schemas/cognitive-disposition-v1.schema.json) и их transaction envelope [cognitive-decision-v1.schema.json](contracts/schemas/cognitive-decision-v1.schema.json).

## 7. Canonical transitions и scheduling

Механизмы строят proposed deltas для точного simulation interval. Authoritative writer проверяет preconditions, units, conservation, uncertainty и artifact compatibility, затем атомарно фиксирует transition и новый state hash. Детерминированный scheduler использует одну и ту же boundary ordering во всех режимах.

Production привязывает продвижение к wall clock; acceleration запрашивает более быстрый прогон; recovery закрывает пропущенный canonical interval; replay применяет или пересчитывает уже зафиксированные transitions. Ни один режим не меняет resolution profile, rules или случайные seeds неявно.

Параллельные workers могут только предлагать результаты. Writer сортирует independent reductions по каноническим ключам. Distributed authoritative state отложен до post-V1 ADR и обязан совпасть с single-node reference.

## 8. Representation lifecycle и failure modes

LOD, sleeping и offloading являются representation transitions с сохранённым полным state, error bound и durable evidence. Runtime admission оценивает требуемые CPU/RAM/storage до refinement. Недостаток capacity возвращает `CapacityExceeded`; отсутствующий artifact, non-convergence или нарушение conservation приводит к `SafeStop` с diagnostic event.

Rollback возвращает предыдущее representation без downcast новых biological events. Recovery никогда не угадывает отсутствующий state и не переключается на менее точную модель скрыто.

## 9. Package и artifact boundary

Механизмы, resolutions, morphotypes, solver coefficients и models являются immutable content-addressed artifacts. Человекочитаемая версия помогает управлению, но transition идентифицирует точные bytes digest-ом. Package manifest связывает совместимые artifacts и подписанные validation evidence.

Human и Neko могут одновременно использовать разные tissue/brain resolutions. World Engine знает contracts и registry, но не каталог органов или видов.

## 10. Phase 0 boundary

В Phase 0 отсутствуют runtime organism state, `BiologicalEngine`, ODE/reaction/physics solvers и полный anatomy catalog. Его deliverables ограничены документами, schemas, fixtures и schema-validation tests. Первый исполнимый biology slice определён заранее в [24-часовом сценарии](docs/scenarios/phase1-24h-human-neko.md).
