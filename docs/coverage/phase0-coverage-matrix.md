# Phase 0 coverage matrix

Статус: planning evidence; неизвестное обозначено явно

| Area/mechanism | Phase 0 resolution/representation | Contract observables | Validation data in Phase 0 | Unknown or invalid outside range | Planned upgrade |
|---|---|---|---|---|---|
| mammalian mass accounting example | `CellCohort` ports only | total cell count/mass, oxygen amount | schema fixture + dimensional/conservation assertions | no tissue kinetics or clinical calibration | Phase 1 minimal mechanism, Phase 3 organ systems |
| cell representation | cohort → `IndividualCellSet` example | count, mass, charge, O₂ amount, mean cell mass | deterministic lift/projection fixture, lineage and continuity assertions | cell cycle, mutation, spatial contacts | Phase 4 adaptive cohorts/individual lineages |
| neural representation | `NeuralPopulation` → `IndividualNeuronNetwork` example | neuron count, mean firing rate, total transmitter amount | deterministic lift/projection fixture and declared error bound | spikes, synapses, plasticity, regional calibration | Phase 6 replaceable neural implementations |
| Human morphotype | independent schema root | anatomy/binding/parameter references | minimal female Makise fixture and isolation assertions | full anatomy/development/lifespan | phases 1–6 scenario-driven packages |
| Neko morphotype | independent schema root | ear/tail anatomy and hearing/balance/thermal bindings | minimal fixture and isolation assertions | feline-human hybrid empirical reference ranges | focused expert estimates, then measured/calibrated replacements |
| cognition | proposal + disposition envelope | decision status/reasons and optional adopted goal/intention | accepted/rejected/deferred fixtures | neural gate dynamics and real provider behavior | Phase 1 scripted cortex; Phase 6 neurobiology; shadow launch real LLM |
| canonical time/replay | contract/document design | transition stream, state hash, conservation report | link/schema gates only | runtime scheduler equivalence not yet implemented | Phase 1 execution matrix |
| physical apartment | legacy anchors only, non-normative for new state | none in Phase 0 | scenario definition | geometry/material/physics fidelity | Phase 2 metric embodiment |
| disease/injury/drugs/death | not implemented | contract families named only | long-horizon/rare-event plan | all biological parameters and solvers | Phase 4 |
| reproduction/development/aging | not implemented | lifecycle events named only | horizon definitions | compatibility, fetal and lifespan calibration | Phase 5 |
| capacity/scaling | schema has no entity cap | declared compute estimates and typed capacity failure | schema checks | actual workstation capacity | Phase 8 sweep to `CapacityExceeded` |

## Gate interpretation

`schema fixture` означает structural contract example, не работающий biological mechanism. В Phase 0 нет численного solver validation. Любая строка без runtime evidence остаётся unknown до указанной фазы и не может использоваться для claim о biological realism.
