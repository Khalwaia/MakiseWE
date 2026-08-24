# Архитектура Makise V1

Статус: нормативная архитектурная база Phase 0
Дата: 2026-08-19
Связанные документы: [CONTEXT.md](CONTEXT.md), [VISION.md](VISION.md), [WORLD_V1.md](WORLD_V1.md), [CIVILIZATION.md](CIVILIZATION.md), [PROTO.md](PROTO.md), [INVARIANTS.md](INVARIANTS.md)

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
| geometry, matter, fields, organisms, biology, neural, digital и institutional state | World Engine |
| committed causal transitions и artifact digests | append-only world timeline |
| субъективное восприятие и память Consciousness | отдельный consciousness stream |
| model output до gate | immutable CortexProposal event |
| принятые goals/intentions/commitments | cognitive state после Accepted transition |
| identity values и morphotype packages | content-addressed artifacts |
| удобные UI шкалы | неавторитетные projections |

Память не исправляет objective state. Objective event не становится субъективной памятью без доступного perception. Администратор отправляет intent через `commit`, а не пишет DB.

## 3. Единый causal graph

MakiseWE моделирует один causal graph с обратными связями и mixed resolution. Уровни L0–L9 являются causal domains — картой состояния и mechanisms, — но не последовательными стадиями tick, независимыми engines или владельцами отдельных DB:

```text
L0  PHYSICAL WORLD
    geometry, matter, mass, energy, temperature, air, light, sound, fluids

L1  ORGANISM
    anatomy, organs, compartments, circulation, respiration, metabolism

L2  TISSUE / CELLULAR
    tissues, CellCohort, individual cells, immune cells, receptors, targets

L3  MOLECULAR / BIOCHEMICAL
    substances, amounts, concentrations, reactions, transport, signaling, PK/PD

L4  NEURAL / BRAIN
    brain regions, NeuralPopulation, neurotransmitters, autonomic control

L5  CONSCIOUSNESS
    perception, interoception, memory, CortexFrame, proposal disposition

L6  MOTOR CONTROL
    accepted intention, motor plan, physical validation, neural/muscular control

L7  PHYSICAL ACTION
    muscles, articulated body, contacts, object interaction, physical outcome

L8  DIGITAL / COMPUTATION
    devices, machine state, code execution, storage, sensors, radios, networks

L9  INSTITUTIONAL / ECONOMIC
    organizations, authority, claims, contracts, obligations, payment, services
```

Связи L0–L5 двусторонние; принятая intention проходит L6/L7 и изменяет L0, после чего новые physical observables могут снова войти в perception. L8 связывает физические устройства, code execution, sensors, radios и воспринимаемый output; L9 связывает socially recognized authority, claims и obligations с decisions, code и physical work. Один mechanism может соединять несколько domains через stable causal ports: thermoregulation пересекает environment, circulation, endocrine и neural control; работа датацентра пересекает electricity, heat, devices, software, contracts и labor. L7–L9 не являются верхними стадиями pipeline и не дублируют владение physical, cognitive или digital state.

`WORLD EVENTS` не является simulation layer. Дождь, открытие двери, падение объекта, решение и движение руки — committed `CausalTransition` соответствующих mechanisms. Durable causal timeline поперечно записывает transitions всех domains: causes, canonical interval, artifact digests, unit-typed deltas, uncertainty, conservation report и state hash. Она не исполняет physics и не является нижним L0.

Общий tick не обязан проходить L0–L9 по порядку. Canonical scheduler запускает только причинно готовые mechanisms по их scheduling rules; authoritative writer проверяет и фиксирует результаты.

## 4. MechanismContract

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

## 5. ResolutionContract

Разрешение объявляет represented entities, aggregation/refinement, coarse-to-fine state lift, fine-to-coarse projection, conserved quantities, observable continuity, uncertainty transformation, triggers/preconditions, compute estimate, rollback и artifact compatibility.

Каноническая операция называется **Explicit Causally Triggered Resolution Transition**. Она не является скрытым LOD или произвольным повышением «важности». Deterministic trigger возникает только из объявленного contract condition: выход за validity range, превышение uncertainty bound, появление divergent lineage, необходимость fine variables для causal interaction либо обязательная validation policy. Admission по CPU/RAM/storage проверяется до transition; недостаток capacity возвращает `CapacityExceeded`, но не разрешает silent downgrade.

Lift может детерминированно создавать individual entities из seed, зафиксированного в `ResolutionChanged`. Он сохраняет mass, charge, substance amounts, entity counts и объявленные moments. Projection сохраняет lineage/provenance, поэтому повторный refinement продолжает причинную историю, а не создаёт новую популяцию. Mixed-resolution interaction идёт только через causal ports.

Schema: [resolution-contract-v1.schema.json](contracts/schemas/resolution-contract-v1.schema.json).

## 6. MorphotypeDefinition

Morphotype package содержит собственные anatomy graph, development program, organ bindings, physiological parameters и validation fixtures. Общие mechanisms подключаются ссылками по digest, но package не наследует другой morphotype.

Runtime registry индексирует packages произвольными IDs и разрешает bindings из данных. В runtime запрещены `is_neko`, enum `Human | Neko` и branches по известным morphotype IDs. Human и Neko fixtures демонстрируют два root definitions; добавление третьего не меняет `WorldEngine`.

Schema: [morphotype-definition-v1.schema.json](contracts/schemas/morphotype-definition-v1.schema.json).

## 7. Cognitive pipeline

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

## 8. Intention, ControlEpisode и outcome

Semantic action является `Intention` либо `PlanHypothesis`, но не state mutation. Accepted intention может создать durable `ControlEpisode`: текущее closed-loop состояние управления, constraints, acquired capabilities, observations, blockers и следующую reevaluation boundary. В нём нет precomputed completion delta; duration является прогнозом с uncertainty.

Каждый следующий шаг выбирается из текущих affordances, проходит motor, digital либо institutional validation и создаёт только proposed transition. После commit новые observables возвращаются controller; он продолжает, перепланирует, приостанавливает или завершает попытку. Cooking, sleeping, cleaning, programming, service fulfillment и construction возникают как causal episodes и observer projections, а не специальные outcome functions. Нормативная детализация находится в [CIVILIZATION.md](CIVILIZATION.md) и [ADR-0012](docs/adr/0012-causal-processes-not-promised-outcomes.md).

## 9. Digital и institutional modules

Digital execution module скрывает sandbox scheduling, virtual machine state и accounting за interface «machine snapshot + mediated inputs -> proposed machine delta + syscall requests». Capability policy независимо решает, разрешён ли конкретный observation/effect request; grant не выполняет effect. Institutional rules предлагают transitions claims, authority, contracts и obligations на основании durable evidence; `Organization` не получает Consciousness.

External gateway является adapter: он принимает только отдельно авторизованный `ExternalEffectIntent` и возвращает idempotent `ExternalEffectReceipt`. Replay применяет receipt без повторения настоящего side effect. Эти seams не создают второго authoritative writer и не дают simulated code прямого host access. Полное решение закреплено в [CIVILIZATION.md](CIVILIZATION.md) и [ADR-0013](docs/adr/0013-diegetic-technology-and-institutions.md).

## 10. Canonical transitions и scheduling

Механизмы строят proposed deltas для точного simulation interval. Authoritative writer проверяет preconditions, units, conservation, uncertainty и artifact compatibility, затем атомарно фиксирует transition и новый state hash. Детерминированный scheduler использует одну и ту же boundary ordering во всех режимах.

Production привязывает продвижение к wall clock; acceleration запрашивает более быстрый прогон; recovery закрывает пропущенный canonical interval; replay применяет или пересчитывает уже зафиксированные transitions. Ни один режим не меняет resolution profile, rules или случайные seeds неявно.

Параллельные workers могут только предлагать результаты. Writer сортирует independent reductions по каноническим ключам. Distributed authoritative state отложен до post-V1 ADR и обязан совпасть с single-node reference.

## 11. Representation lifecycle и failure modes

LOD, sleeping и offloading являются representation transitions с сохранённым полным state, error bound и durable evidence. Runtime admission оценивает требуемые CPU/RAM/storage до refinement. Недостаток capacity возвращает `CapacityExceeded`; отсутствующий artifact, non-convergence или нарушение conservation приводит к `SafeStop` с diagnostic event.

Rollback возвращает предыдущее representation без downcast новых biological events. Recovery никогда не угадывает отсутствующий state и не переключается на менее точную модель скрыто.

## 12. Package и artifact boundary

Механизмы, resolutions, morphotypes, solver coefficients и models являются immutable content-addressed artifacts. Человекочитаемая версия помогает управлению, но transition идентифицирует точные bytes digest-ом. Package manifest связывает совместимые artifacts и подписанные validation evidence.

Human и Neko могут одновременно использовать разные tissue/brain resolutions. World Engine знает contracts и registry, но не каталог органов или видов.

## 13. Authoritative model improvement control plane

Улучшение authoritative mechanism/model/solver находится вне simulation causal graph и не получает write access к authoritative state. Evidence может породить immutable candidate artifact, но candidate проходит contract validation, focused suites и shadow run до допуска. Это не относится к diegetic приложениям, которые персонажи изменяют внутри мира: их `CodeArtifact` исполняется как simulated software и никогда не становится artifact самого World Engine.

Активация принятого artifact выполняется только как авторизованный admin intent через `WorldEngine::commit` на canonical boundary. Transition фиксирует прежний и новый content digest, compatibility evidence и rollback target. Старые transitions навсегда ссылаются на прежние bytes; audit replay не пересчитывает историю новой моделью. Автономное изменение production-кода или активация candidate без approval не входит в V1.

## 14. Phase 0 boundary

В Phase 0 отсутствуют runtime organism state, `BiologicalEngine`, ODE/reaction/physics solvers и полный anatomy catalog. Его deliverables ограничены документами, schemas, fixtures и schema-validation tests. Первый исполнимый biology slice определён заранее в [24-часовом сценарии](docs/scenarios/phase1-24h-human-neko.md).
