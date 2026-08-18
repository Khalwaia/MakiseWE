# Нормативные инварианты Makise V1

Статус: обязательны для design, packages, runtime, replay и release
Дата: 2026-08-19

## Authority and mutation

1. World Engine — единственный автор objective physical, biological и neural state.
2. `WorldEngine::commit` — единственный mutation path; time, stimuli, LLM responses, actions, resolution changes и admin intents не имеют обхода.
3. Projections, memory, panel, transport, LLM и compute workers никогда не получают право прямой записи authoritative state.
4. Каждый `CausalTransition` содержит causes, canonical interval, unit-typed deltas, artifact digests, uncertainty/error bounds, conservation report и state hash.
5. Commit подтверждается только после durable append; retry идемпотентен.

## Causal contracts and resolution

6. Механизм без полного `MechanismContract` не загружается.
7. Caller зависит только от stable causal inputs, outputs, observables и failure modes; internal resolution не протекает через interface.
8. `CellCohort`/`NeuralPopulation` и individual implementations реализуют одинаковые causal contracts.
9. Разные organs, tissues и brain regions могут одновременно использовать разные resolutions.
10. Любая смена resolution/representation — явный durable `ResolutionChanged`; hidden quality/fidelity settings запрещены.
11. Lift/projection сохраняют объявленные mass, charge, amounts, counts и moments, а также lineage/provenance и observable continuity в error bounds.
12. LOD, sleeping и offloading разрешены только как deterministic representation transitions с conservation proof, full-state recovery и rollback.
13. Missing/incompatible upgrade artifact вызывает `SafeStop`, не silent fallback.

## Quantities and evidence

14. Authoritative number — физическая величина с unit, counter либо определённая безразмерная probability, fraction, receptor occupancy или strain.
15. Произвольные normalized scores, включая generic health, energy, cleanliness, charge, resource, valence, arousal и urgency, запрещены как authoritative state.
16. UI может показывать нормированную projection только с маркировкой non-authoritative и ссылкой на исходные quantities.
17. Каждый parameter/mechanism хранит provenance category, uncertainty/error model и validity range; `expert_estimate` маркируется явно.
18. Conservation/dimensional/range violations не clamp-ятся молча и не исправляются LLM.

## Morphotypes and population

19. Human и Neko — независимые root `MorphotypeDefinition`; Neko не наследует Human и не представлен `is_neko`.
20. Runtime не содержит закрытый enum известных morphotypes и branches по morphotype ID.
21. Morphotype package владеет anatomy graph, development, organ bindings, physiological parameters и validation fixtures.
22. Новый morphotype не требует изменения World Engine.
23. Schema не задаёт cap для Organism, Consciousness, cells или neurons; runtime admission ограничивается только измеримыми CPU/RAM/storage.
24. Capacity exhaustion возвращает `CapacityExceeded` без ослабления causal model.

## Cognition and consent

25. LLM может предлагать только semantic appraisal, goal, intention, plan, memory interpretation и communication.
26. LLM не меняет physical/biological state, hormones, neurotransmitters, neural activation, emotion outcome, adopted goals/commitments, object state или action success.
27. Каждый `CortexProposal` получает durable `CognitiveDisposition`: `Accepted`, `Rejected`, `Deferred` или `NeedsRevision` с причинами.
28. Только `Accepted` разрешает отдельную transition принятия goal/intention; proposal никогда не равен state.
29. Motor plan проходит отдельный physical validator; outcome определяется simulation.
30. Каждое Consciousness имеет отдельные perception, brain, memory, privacy и decision streams.
31. Интимные и репродуктивные действия требуют accepted intentions всех участников; admin authority не заменяет consent.

## Time, execution and replay

32. Production, acceleration, recovery и audit используют одинаковые canonical scheduling rules, mechanisms, resolutions и seeds.
33. Wall clock меняет только темп execution, не causal semantics.
34. Tick partition, thread count, restart, downtime и acceleration не меняют transition stream/state hash.
35. Downtime не создаёт решений, intentions, perceptions или memories от имени Consciousness.
36. Parallel/distributed workers только предлагают transitions; authoritative writer проверяет и канонически редуцирует их.

## Persistence, compatibility and safety

37. Model, mechanism, resolution и solver artifacts immutable и content-addressed; audit replay использует точные архивные bytes.
38. Fast replay применяет committed deltas; audit replay пересчитывает их и сравнивает conservation/state hash.
39. Новая V1 timeline/DB отделена от immutable legacy archive.
40. Legacy wire fixtures, packages, DB и event logs остаются читаемыми dual readers; V1 reader не удаляется этой миграцией.
41. Rollback не требует downcast новых biological events.
42. Corruption, non-convergence, missing artifacts и conservation failure приводят к diagnostic `SafeStop`, а не к правдоподобно выглядящей подмене.
43. Секреты, chain-of-thought и скрытое objective state не попадают в обычные projections.

## Validation scope

44. Каждый mechanism имеет contract conformance, focused validation и resolution-upgrade tests.
45. 365-day run доказывает integration/replay, не biological realism.
46. Long-horizon и rare-event claims принимаются по distributions, incidence ranges, causal prerequisites и declared uncertainty, не по одному seed.
47. Phase N+1 не начинается до отдельного gate commit Phase N.
