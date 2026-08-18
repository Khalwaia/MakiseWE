---
status: accepted
---

# Stable causal interfaces and explicit resolution upgrades

Механизмы зависят от стабильных causal inputs, outputs и observables, а не от внутреннего представления соседей. `CellCohort` и `NeuralPopulation` являются V1 adapters, а переходы к индивидуальным сущностям выполняются только через durable `ResolutionChanged` с conservation proof, потому что скрытая смена fidelity разрушила бы replay и сделала результаты зависимыми от нагрузки.

Любое представление обязано объявить lift, projection, error bounds, lineage и rollback. Невозможность загрузить upgrade artifact приводит к `SafeStop`, а не к молчаливому fallback.
