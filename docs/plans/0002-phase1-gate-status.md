# Phase 1 Gate Status — causal-kernel organism slices

## Status: READY FOR GATE CLAIM — all roadmap criteria covered by kernel evidence

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
| Explicit durable ResolutionChanged | this slice | `tests/resolution_transition.rs` |
| Acceleration / restart replay parity | this slice | `tests/acceleration_replay.rs` |
| Data-driven anatomy graph and organ bindings | this slice | `tests/anatomy_binding.rs` |

## Gate criteria из ROADMAP.md Phase 1

| Criterion | Status | Gap |
|---|---|---|
| Один общий data-driven pipeline | ✅ | MorphotypeDefinition binds anatomy nodes/edges and organ bindings into runtime state |
| Непрерывная цепь food/load/temp/sleep/action | ✅ в kernel | integration test покрывает food→metabolism→thermal→sleep |
| Accepted/rejected/deferred по причинам | ✅ | cognitive_gate.rs |
| Cortex не мутирует body/world | ✅ | Intention.contains_physical_delta() == false, enforced by type |
| 1:1 / acceleration / restart / replay parity | ✅ | 120 × 1 s equals one accelerated interval; split/restart matches uninterrupted physical state |
| Единый resolution profile | ✅ | representation changes only through committed ResolutionChanged request |
| Минимум один explicit ResolutionChanged | ✅ | validated, idempotent, durable transition preserves organism energy exactly |
| Два полных morphotype packages (schema-level) | ✅ | Human/Neko fixtures expose graphs and bindings through runtime data API |

## Честная оценка

Causal-kernel теперь покрывает все критерии Phase 1 gate на уровне текущего
kernel seam: девять organism slices, явное resolution transition, ускорение/
restart parity и data-driven anatomy bindings. Это не является claim биологического
realism за пределами declared fixture validity ranges. Отдельный gate commit
остаётся обязательным перед Phase 2.
