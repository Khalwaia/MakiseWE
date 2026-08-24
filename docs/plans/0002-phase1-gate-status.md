# Phase 1 Gate Status — causal-kernel organism slices

## Status: PARTIAL — kernel slice complete, full Phase 1 gate not yet claimed

## Что реализовано и доказано тестами

| Slice | Commit | Evidence |
|---|---|---|
| Organism state (chemical + core thermal) | `e65b0ea` | `tests/organism.rs` |
| Circadian sleep phase + gated metabolism | `1b0cb06` | `tests/circadian.rs` |
| Digestion ingestion + circadian modulation | `7a23dfe` | `tests/digestion.rs` |
| Interoception observables + sleep debt | `86e84d1` | `tests/interoception.rs` |
| Ambient sensory thermal exchange | `d68d7f6` | `tests/sensory_transduction.rs` |
| Morphotype packages (Human/Neko data) | `3c92d4a` | `tests/morphotype.rs` |
| CellCohort exact lift/projection | `3c1caf9` | `tests/cell_cohort.rs` |
| NeuralPopulation spike energy accounting | `6db80d8` | `tests/neural_population.rs` |
| Scripted cortex gate → intention | `4a48b53` | `tests/cognitive_gate.rs` |
| 24h integration with restart parity | `3c4d91f` | `tests/phase1_integration.rs` |

## Gate criteria из ROADMAP.md Phase 1

| Criterion | Status | Gap |
|---|---|---|
| Один общий data-driven pipeline | ✅ частично | morphotype как данные, но без anatomy graph bindings |
| Непрерывная цепь food/load/temp/sleep/action | ✅ в kernel | integration test покрывает food→metabolism→thermal→sleep |
| Accepted/rejected/deferred по причинам | ✅ | cognitive_gate.rs |
| Cortex не мутирует body/world | ✅ | Intention.contains_physical_delta() == false, enforced by type |
| 1:1 / acceleration / restart / replay parity | ⚠️ частично | partition + restart есть; explicit acceleration worker test отсутствует в kernel |
| Единый resolution profile | ⚠️ частично | нет runtime resolution switching в этом slice |
| Минимум один explicit ResolutionChanged | ❌ | не реализован; требуется для полного gate |
| Два полных morphotype packages (schema-level) | ❌ частично | physiological parameters есть; anatomy graph/organ bindings не подключены |

## Честная оценка

Causal-kernel содержит рабочие механизмы всех девяти заявленных organism slices с
exact conservation, typed failures и determinism. Это сильный фундамент, но
**полный Phase 1 gate из ROADMAP.md ещё не закрыт**: отсутствуют
`ResolutionChanged`, acceleration/replay parity test на уровне kernel, и
data-driven anatomy graph. Начинать Phase 2 до закрытия этих пунктов запрещено
нормативом.
