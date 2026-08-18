# Roadmap Makise V1

Статус: нормативная последовательность; следующая фаза запрещена до отдельного gate commit
Дата: 2026-08-19

## Phase 0 — contracts and architecture

Deliverables:

- архивировать прежний Stage 5A.1 diff без `.agents` и `skills-lock.json`;
- определить ubiquitous language в [CONTEXT.md](CONTEXT.md);
- согласовать VISION, ARCHITECTURE, WORLD, ROADMAP, INVARIANTS и protocol design;
- сохранить старый [STAGE_5.md](STAGE_5.md) с пометкой superseded;
- зафиксировать ADR stable causal interfaces/resolution upgrades, unitful state, independent morphotypes, cognitive acceptance, canonical time, content-addressed artifacts и unified causal graph;
- добавить JSON Schemas для `MechanismContract`, `ResolutionContract`, `MorphotypeDefinition`, `CortexProposal`, `CognitiveDisposition` и decision envelope;
- добавить fixtures Human, Neko, двух resolution upgrades и accepted/rejected/deferred proposals;
- определить [24-часовой Phase 1 scenario](docs/scenarios/phase1-24h-human-neko.md) и [coverage matrix](docs/coverage/phase0-coverage-matrix.md).

Не входят: `BiologicalEngine`, runtime organism state, solvers, полный anatomy catalog и любой код Phase 1.

Gate:

- нормативные документы называют V1 resolution начальным, не постоянным;
- durable timeline отделена от L0–L7 causal domains; domains образуют единый feedback graph, не последовательный pipeline;
- все fixtures валидируются schemas;
- Human/Neko — independent roots, runtime design не содержит morphotype-specific branches;
- оба upgrade examples сохраняют quantities, lineage и observables в error bounds;
- каждый resolution transition имеет deterministic contract trigger; hidden LOD и субъективная «важность» запрещены;
- rejected/deferred proposal не становится cognitive state, accepted требует отдельной transition;
- arbitrary normalized scores запрещены;
- diff содержит только docs, schemas, fixtures, schema-validation tests и необходимую test dependency metadata;
- formatting, Markdown links, schema gates и workspace tests проходят.

## Phase 1 — 24-hour Human/Neko vertical slice

Реализовать только механизмы заранее определённого сценария: два morphotype packages; air/water/food/ambient temperature; минимальные digestion, circulation, respiration, metabolism, thermoregulation, circadian/sleep, sensory transduction; `CellCohort`/`NeuralPopulation`; Neko ears/tail/hearing/balance; scripted cortex; appraisal, proposal, gate, intention, physical action, perception и causal trace.

Gate: один общий data-driven pipeline; непрерывная цепь food/load/temperature/sleep/action; accepted/rejected/deferred по моделируемым причинам; cortex не мутирует body/world; одинаковые transition stream/state hash при 1:1, acceleration, restart и replay; production/acceleration используют один resolution profile; минимум один explicit `ResolutionChanged`. Отдельный gate commit обязателен.

## Phase 2 — apartment and physical embodiment

Добавить metric 3D geometry, materials, mass/inertia, articulated bodies, active physics islands, fluids, atmosphere, heat/humidity/gases/aerosols, acoustics/light/odors, electricity/water. Anchors остаются semantic projection. Intentions запускают closed-loop motor programs.

Gate: walk, grasp, carry, cook, spill, heat, clean и dress проходят physics, conservation и replay; `duration_ms` и resource/cleanliness/charge scores не authoritative.

## Phase 3 — everyday physiology

Вертикально добавлять cardiovascular/respiratory, renal/fluids/electrolytes, digestive/liver/metabolism, endocrine, thermoregulation/skin, musculoskeletal/fatigue/pain, excretion/hygiene/microbiome.

Gate каждого system: `MechanismContract`, reference observables, upgrade path и focused validation.

## Phase 4 — cells, immunity, pathology and drugs

Добавить division/differentiation/turnover/apoptosis/necrosis/mutation lineage; adaptive cohorts и individual cells для gametes, tumor clones и pathogen lineages; innate/adaptive immunity, inflammation, infections, wounds/bleeding/healing, poisoning/allergy/organ failure/cancer/death; PK/PD, binding/interactions.

Gate: cohort и individual implementations проходят одинаковые causal contract tests, а различия остаются внутри declared uncertainty.

## Phase 5 — reproduction, development and aging

Добавить genetics/phenotype, endocrine cycles, gametes, fertility/conception/pregnancy/fetal development/birth, growth/puberty/aging/senescence/pathology. Новый Organism возникает physical event, Consciousness подключается отдельно; compatibility находится в morphotype data.

## Phase 6 — neuroscience and psychology

Добавить brain regions и replaceable neural resolution; sensory gating, autonomic control, arousal/attention/working memory/motor inhibition; Glu/GABA и dopamine/serotonin/norepinephrine/acetylcholine/histamine/orexin; HPA/HPG/HPT coupling; reinforcement/habits/stress/affect episodes/memory consolidation; отдельные brain/memory streams.

Gate: `NeuralPopulation` и будущий `IndividualNeuronNetwork` имеют одинаковые ports; LLM остаётся cortex proposal source; generic valence/arousal/urgency scores не authoritative.

## Phase 7 — full life and multiple consciousnesses

Добавить physical recipes/food transformations, clothing physics, skills через practice evidence/reaction time/error distributions, speech/hearing/phone/Telegram/music/deliveries/consumables, commitments/relationships/privacy/subjective memory и независимое восприятие shared world. Intimate/reproductive actions требуют accepted intentions всех участников.

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
- ни один subsystem не заявляет неограниченный realism без validity range и upgrade path.
