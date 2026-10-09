# Roadmap Makise V1

Статус: нормативная последовательность; следующая фаза запрещена до отдельного gate commit
Дата: 2026-08-19

## Phase 0 — contracts and architecture

Deliverables:

- архивировать прежний Stage 5A.1 diff без `.agents` и `skills-lock.json`;
- определить ubiquitous language в [CONTEXT.md](CONTEXT.md);
- согласовать VISION, ARCHITECTURE, WORLD, CIVILIZATION, ROADMAP, INVARIANTS и protocol design;
- сохранить старый [STAGE_5.md](STAGE_5.md) с пометкой superseded;
- зафиксировать ADR stable causal interfaces/resolution upgrades, unitful state, independent morphotypes, cognitive acceptance, canonical time, content-addressed artifacts, unified causal graph, causal processes и diegetic technology/institutions;
- добавить JSON Schemas для `MechanismContract`, `ResolutionContract`, `MorphotypeDefinition`, `CortexProposal`, `CognitiveDisposition` и decision envelope;
- добавить fixtures Human, Neko, двух resolution upgrades и accepted/rejected/deferred proposals;
- определить [24-часовой Phase 1 scenario](docs/scenarios/phase1-24h-human-neko.md) и [coverage matrix](docs/coverage/phase0-coverage-matrix.md).

Не входят: `BiologicalEngine`, runtime organism state, solvers, полный anatomy catalog и любой код Phase 1.

Gate:

- нормативные документы называют V1 resolution начальным, не постоянным;
- durable timeline отделена от L0–L9 causal domains; domains образуют единый feedback graph, не последовательный pipeline;
- все fixtures валидируются schemas;
- Human/Neko — independent roots, runtime design не содержит morphotype-specific branches;
- оба upgrade examples сохраняют quantities, lineage и observables в error bounds;
- каждый resolution transition имеет deterministic contract trigger; hidden LOD и субъективная «важность» запрещены;
- rejected/deferred proposal не становится cognitive state, accepted требует отдельной transition;
- semantic actions не содержат precomputed outcome; приложения, organizations и designs не получают привилегированный mutation path;
- arbitrary normalized scores запрещены;
- diff содержит только docs, schemas, fixtures, schema-validation tests и необходимую test dependency metadata;
- formatting, Markdown links, schema gates и workspace tests проходят.

## Phase 1 — 24-hour Human/Neko vertical slice

Реализовать только механизмы заранее определённого сценария: два morphotype packages; air/water/food/ambient temperature; минимальные digestion, circulation, respiration, metabolism, thermoregulation, circadian/sleep, sensory transduction; `CellCohort`/`NeuralPopulation`; Neko ears/tail/hearing/balance; scripted cortex; appraisal, proposal, gate, intention, physical action, perception и causal trace.

Gate: один общий data-driven pipeline; непрерывная цепь food/load/temperature/sleep/action; accepted/rejected/deferred по моделируемым причинам; cortex не мутирует body/world; одинаковые transition stream/state hash при 1:1, acceleration, restart и replay; production/acceleration используют один resolution profile; минимум один explicit `ResolutionChanged`. Отдельный gate commit обязателен.

## Phase 2 — apartment and physical embodiment

Добавить metric 3D geometry, materials, mass/inertia, articulated bodies, active physics islands, fluids, atmosphere, heat/humidity/gases/aerosols, acoustics/light/odors, electricity/water. Anchors остаются semantic projection. Intentions запускают closed-loop motor programs.

Gate: walk, grasp, carry, cook, spill, heat, clean и dress развиваются через durable closed-loop control, physics, conservation, feedback, interruption и replay; `duration_ms`, completion mutation и resource/cleanliness/charge scores не authoritative.

## Phase 3 — everyday physiology

Статус: gates 3.1, 3.2, 3.3 закрыты с tier 3 validation. Slices 3.4-3.7 остаются открытыми. Текущее evidence, ограничения replay и следующий acceptance slice — в [плане Phase 3](docs/plans/0006-phase3-everyday-physiology.md).

Вертикально добавлять cardiovascular/respiratory, renal/fluids/electrolytes, digestive/liver/metabolism, endocrine, thermoregulation/skin, musculoskeletal/fatigue/pain, excretion/hygiene/microbiome.

Gate каждого system: `MechanismContract`, reference observables, upgrade path и focused validation.

Для закрытия 3.2 дополнительно обязательны executable renal resolution upgrade и независимые эмпирические time-series; synthetic integration tests не заменяют этот gate. Решения пользователя от 2026-09-07 и границы validation закреплены в [плане Phase 3](docs/plans/0006-phase3-everyday-physiology.md#обязательная-эмпирическая-приёмка-32).

### Closed gates

**Gate 3.1 — cardiorespiratory.gas-exchange** (closed 2026-09-29):
- Commit: [d2e094d](https://github.com/Khalwaia/MakiseWE/commit/d2e094d)
- Validation: [apple-heart-movement-spo2-validation-data.json](docs/research/apple-heart-movement-spo2-validation-data.json)
- Provenance tier: 3 (Apple Heart & Movement Study, 72M measurements)
- Observables validated: SpO2 95-100% circadian patterns, respiratory rate baseline

**Gate 3.2 — renal.fluid-electrolyte** (closed 2026-09-29):
- Commit: [d2e094d](https://github.com/Khalwaia/MakiseWE/commit/d2e094d)
- Validation: [jensen-2013-validation-data.json](docs/research/jensen-2013-validation-data.json)
- Provenance tier: 3 (Jensen et al. 2013 peer-reviewed)
- Observables validated: urine flow 6.9-8.8 ml/min, plasma Na 138-140 mmol/L
- Upgrade path: PhysioNet individual traces for tier 2

**Gate 3.3 — digestive.substrate** (closed 2026-10-09):
- Commit: [2657f5b](https://github.com/Khalwaia/MakiseWE/commit/2657f5b)
- Validation: [phase3-digestive-substrate-validation-evidence.md](docs/research/phase3-digestive-substrate-validation-evidence.md)
- Provenance tier: 3 (published peer-reviewed group statistics)
- Observables validated:
  - Postprandial glucose 4-8 mmol/L, absorption 3-5h (Rose 1999, tier 3)
  - Fasting glucose 3.9-5.5 mmol/L (ADA/WHO standards, tier 3)
  - Fecal dry mass ~29 g/day, ~10% intake (Rose 2015 meta-analysis, tier 3)
- Upgrade path: tier 2 requires individual time-series (PhysioNet, author data)

## Phase 4 — cells, immunity, pathology and drugs

Добавить division/differentiation/turnover/apoptosis/necrosis/mutation lineage; adaptive cohorts и individual cells для gametes, tumor clones и pathogen lineages; innate/adaptive immunity, inflammation, infections, wounds/bleeding/healing, poisoning/allergy/organ failure/cancer/death; PK/PD, binding/interactions.

Gate: cohort и individual implementations проходят одинаковые causal contract tests, а различия остаются внутри declared uncertainty.

## Phase 5 — reproduction, development and aging

Добавить genetics/phenotype, endocrine cycles, gametes, fertility/conception/pregnancy/fetal development/birth, growth/puberty/aging/senescence/pathology. Новый Organism возникает physical event, Consciousness подключается отдельно; compatibility находится в morphotype data.

## Phase 6 — neuroscience and psychology

Добавить brain regions и replaceable neural resolution; sensory gating, autonomic control, arousal/attention/working memory/motor inhibition; Glu/GABA и dopamine/serotonin/norepinephrine/acetylcholine/histamine/orexin; HPA/HPG/HPT coupling; reinforcement/habits/stress/affect episodes/memory consolidation; отдельные brain/memory streams.

Gate: `NeuralPopulation` и будущий `IndividualNeuronNetwork` имеют одинаковые ports; LLM остаётся cortex proposal source; generic valence/arousal/urgency scores не authoritative.

## Phase 7 — technology, economy, society and multiple consciousnesses

Добавить physical recipes/food transformations, clothing physics, skills через practice evidence/reaction time/error distributions, speech/hearing/music/deliveries/consumables, commitments/relationships/privacy/subjective memory и независимое восприятие shared world.

Добавить физико-цифровые devices, deterministic sandbox execution, source/build/binary/release lineage, character-authored applications, marketplace, capabilities, simulated networks и external effect intent/receipt. Добавить organizations без собственного Consciousness, authority delegation, possession/title claims, offers/contracts/obligations, payment/debt/employment/lease, physical/digital/hybrid services, disputes и enforcement.

Добавить data-driven construction/manufacturing: design artifacts отдельно от authoritative geometry, material/energy provenance, work orders, logistics, inspection, operation и failures. Сквозные сценарии: персонаж создаёт и публикует приложение; organization оказывает сервис; дом строится из материалов и труда; датацентр связывает power/cooling/hardware/software/network/contracts.

Gate: ни приложение, ни order, contract, design или semantic action не создаёт outcome напрямую; self-improvement создаёт immutable candidate без расширения authority; simulated compute не получает host resources; external replay не повторяет side effects; intimate/reproductive actions требуют accepted intentions всех участников.

## Phase 8 — performance and scaling

Добавить sparse SoA, dependency graph, batching/SIMD, deterministic reduction, resolution-aware scheduling, explicit sleeping/offloading и stateless compute workers. Entity schema-cap запрещён; capacity sweep продолжается до честного `CapacityExceeded`.

Workstation gate: Human + Neko, два active Consciousness, 1:1 на 16 cores/32 GB, World Engine ≤24 GB. Distributed authoritative state требует отдельного post-V1 ADR и parity с single-node reference.

## Post-V1 research — validated model improvement

Внешний control plane может анализировать validation/shadow evidence и создавать immutable candidate physics, biology или brain artifacts. Candidate не меняет authoritative state и не активируется сам: contract suites, shadow run и explicit approval предшествуют авторизованному admin intent через `WorldEngine::commit`. Автономное изменение production-кода и production activation без approval не входят в V1.

## Release gates

После фазовых gates обязательны:

- 365-day integration/replay run с одинаковым state hash;
- targeted 10/30/80-year и rare-event ensembles в declared acceptance ranges;
- 30 календарных дней shadow/closed launch с real LLM, restart, downtime и provider failures;
- panel с units, provenance, uncertainty, resolution и causal trace;
- сквозные cooking/application/service/house/datacenter сценарии не содержат semantic outcome mutations;
- ни один subsystem не заявляет неограниченный realism без validity range и upgrade path.
