# Phase 1 scenario: 24 hours, Human and Neko

Статус: заранее зафиксированный acceptance scenario; runtime не входит в Phase 0

## Purpose

Один Human organism с женским phenotype Makise и один Neko organism проходят одинаковый package-driven pipeline в общей минимальной среде. Сценарий доказывает causal integration и resolution replacement, а не полноту физиологии.

Начало: `2042-04-17T06:00:00+07:00`. Конец: ровно через 24 simulation hours. У каждого organism отдельный scripted Consciousness, perception/memory/cognitive stream и deterministic seed root. Production и acceleration используют один resolution profile.

## Initial state

- air: oxygen/carbon dioxide amounts, pressure, humidity и temperature с units;
- water и food portions с mass/composition;
- Human/Neko: minimal respiratory, circulatory, digestive, metabolic, thermoregulatory, circadian/sleep, sensory, `CellCohort` и `NeuralPopulation` state;
- Neko package data включает cat auricles, tail, hearing transfer, vestibular coupling и heat-loss surfaces;
- все mechanisms/artifacts закреплены content digests.

## Canonical timeline

| Interval | Stimulus/action | Required causal chain |
|---|---|---|
| 06:00–07:00 | wake, light and room sound | fields → sensory transduction → neural population → perception |
| 07:00–09:00 | measured food and water | ingestion → digestion → absorbed amounts → circulation → metabolism/heat |
| 09:00–10:00 | одинаковая умеренная нагрузка | accepted intention → motor validation → work/oxygen use/CO₂/heat → recovery |
| 10:00–12:00 | directional quiet sound | acoustics → morphotype ear transfer → hearing observable; Neko data changes response |
| 12:00–14:00 | balance task and tail posture | physical feasibility → vestibular/tail coupling → outcome, no morphotype branch |
| 14:00–16:00 | ambient temperature step down | heat exchange → thermoregulation → interoception; morphotype parameters explain difference |
| 16:00–18:00 | explicit cell resolution refinement | request → capacity/preconditions → `ResolutionChanged` → individual cells → projection proof |
| 18:00–21:00 | meal, water and ordinary activity | same minimal mechanisms continue across mixed resolution |
| 21:00–23:00 | three cognitive proposals | one accepted, one rejected, one deferred for state-backed reasons |
| 23:00–06:00 | sleep and circadian recovery | accepted sleep intention → physical preparation → sleep transitions → wake |

## Cognitive triplet

1. **Accepted**: drink available water after modeled interoceptive evidence and feasibility; gate adopts goal/intention, motor validator then determines action.
2. **Rejected**: proposal claims direct reduction of body temperature by changing biological state; gate rejects forbidden authority and no state transition exists.
3. **Deferred**: perform balance task while modeled fatigue/recovery preconditions are outside validity range; proposal remains immutable, gate records reconsideration trigger without commitment.

## Resolution proof

Один declared `CellCohort` is refined through deterministic seeded lift. Cell count, total mass, electric charge, declared substance amounts and moments match the coarse state. Fine observables projected before/after remain inside contract bounds. Projection back preserves lineage, and replay crosses the same `ResolutionChanged`. Missing fine artifact must produce `SafeStop` in a negative fixture/test.

## Required traces

Каждая цепь показывает causes, mechanism/resolution/solver digests, canonical interval, unit deltas, uncertainty, conservation и state hash. Human/Neko differences resolve to morphotype data paths. Cortex traces отдельно show frame → proposal → disposition → optional cognitive adoption → motor validation → physical outcome.

## Gate runs

Один seed и input schedule выполняются:

- production-clock harness 1:1 semantics;
- accelerated harness;
- restart до/после еды, resolution change и sleep transition;
- fast replay;
- audit replay;
- 1 и 16 worker threads.

Все runs дают одинаковый canonical transition stream and final state hash. Biological values принимаются только в declared ranges/reference observables; совпадение hash само по себе не является realism evidence.
