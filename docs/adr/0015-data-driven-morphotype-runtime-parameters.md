---
status: accepted
date: 2026-09-02
---

# Data-driven morphotype runtime parameters

`MorphotypeDefinition` уже независимый root без наследования (`ADR-0007`). Оставался закрытый `match morphotype_id` в `causal-kernel/src/morphotype.rs:353` — добавление третьего morphotype требовало изменения `WorldEngine`, нарушая `INVARIANTS:22`.

Решение: runtime параметры `awake/asleep/night metabolism`, `core_heat_capacity`, `ambient_conductance` читаются из `physiological_parameters` fixture, а не из хардкода. `morphotype_id` становится только ключом пакета, не ветвлением. Отсутствующий набор → `UnknownMorphotypeParameters` / `InvalidJson`, не silent fallback на human.

Пять канонических `parameter_id`:
`core-heat-capacity-uj-per-mk`, `ambient-conductance-uj-per-mk-s`, `awake-metabolism-uj-per-s`, `asleep-metabolism-uj-per-s`, `night-awake-metabolism-uj-per-s` — единицы `uj_per_mk`, `uj_per_mk_s`, `uj_per_s`, provenance `expert_estimate`/`fictional_assumption`, validity как в `docs/research/biology-realism.md`.

Фикстуры `human-minimal`/`neko-minimal` дополнены — порядок инвариантов Neko < Human сохранён, но значения теперь data-driven. Код больше не содержит `Morphotype::human()`/`neko()` как единственный источник — они остаются только для тестов/бенчмарков.
