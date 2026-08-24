# Realism hardening of the Phase 1 causal-kernel organism slices

Статус: реализованный evidence record; не является нормативной спецификацией
Дата: 2026-08-25
Связанные: [0002-phase1-gate-status.md](0002-phase1-gate-status.md), [research/biology-realism.md](../research/biology-realism.md), ADR-0014

## Исходный аудит

Read-only аудит 2026-08-25 зафиксировал: детерминизм и консервация образцовые,
но физические константы находились вне физиологических диапазонов на 1–7
порядков, а два механизма нарушали собственные инварианты проекта:

| Находка | Нарушение | Slice |
|---|---|---|
| `default_runtime_parameters_for` игнорировал morphotype_id; neko fixture получал human параметры | data-driven pipeline gate criterion | A |
| Метаболизм 1.2 W (~70x ниже BMR), теплоёмкость 4 J/K (~55000x), conductance 0.05 W/K (~100x), спайк 50 mJ (~10^7x), абсолютные температуры до 5·10^6 K | units/provenance discipline | B |
| `accept_sleep_intention` мгновенно записывал authoritative фазу сна | ADR-0012 / WORLD_V1 §9 outcome mutation | C |
| `ingest_food` кредитовал store мгновенно и без ограничения ёмкости | causal process principle | D |

## Реализованные изменения и evidence

### A — morphotype parameter binding (`7323db2`)

Параметры диспатчатся по зарегистрированному identity; неизвестный id
отклоняется `UnknownMorphotypeParameters`. Evidence:
`tests/morphotype.rs::neko_fixture_binds_neko_runtime_parameters_not_human`,
`::unknown_morphotype_id_is_rejected_without_silent_default`.

### B — physiological recalibration (`66807a9`)

- Метаболизм 95/75/88 Вт (day-awake/asleep/night-awake) → бюджет ~1824 ккал/день;
- теплоёмкость 216_380_000 µJ/mK = 3490 Дж/(кг·К) × 62 кг;
- conductance 5600 µJ/(мК·с) ≈ 5.6 Вт/К → пассивное равновесие 310.11 K при 20 °C комнате;
- baseline состояния строятся как capacity × reference temperature (`physiological_baseline`);
- нейронный учёт переведён в нДж (nJ-order стоимости спайка представим целыми);
- вскрыт и закрыт латентный баг рестарта: ambient heat capacity не персистилась
  и после reopen подменялась toy-значением; capacity теперь часть persisted state
  с expand-миграцией.

Guards: `tests/parameter_realism.rs` выводит каждый ожидаемый диапазон из
независимых опубликованных величин, никогда из production-алгоритма. Смена
константы вне диапазона ломает CI до попадания в main.

### C — sleep as triggered process (`e5ade4d`)

Принятая intention персистится как условие; переходы решает детерминированный
circadian механизм по каноническим секундам: onset требует ночного окна
(22:00–06:00) либо долга ≥ 43200 с; пробуждение требует обнулённого долга внутри
утреннего окна (06:00–12:00). Recovery rate 2:1 — coarse surrogate быстрого
homeostatic распада. Evidence: полный цикл засыпание→пробуждение,
отказ daytime-nap без долга, restart persistence условия и фазы.

### D — staged digestion (`8549f0f`)

Приём пищи наполняет персистентный digestive buffer; перенос в store идёт по
объявленному `ABSORPTION_RATE_UJ_PER_SECOND` (экспертная оценка: стандартный
приём ~4 ч) раз в каноническую секунду перед метаболизмом. Приём сверх declared
chemical capacity отклоняется typed `DigestiveCapacityExceeded` без частичной
мутации.

## Совместимость

Обе schema-миграции organism_state — additive (`ALTER TABLE ADD COLUMN ... DEFAULT`),
старые timelines продолжают открываться; значения строк сохраняются.
Отсутствующие колонки получают declared defaults при первом открытии.

## Честные пределы текущего реализма

Эти пункты остаются открытыми и запрещают claim о biological realism:

- один тепловой резервуар ядра вместо core/skin компартментов, отсутствие
  vasomotor/sweating эффекторов; conductance — passive surrogate;
- среда — квази-бесконечный резервуар с declared дрейфом < 1 K/сутки, без
  airflow/radiation/humidity физики (Phase 2);
- circadian модель — оконные пороги и линейный долг, не two-process dynamics;
- нет renal/water balance, endocrine, substrate-specific метаболизма, O₂/CO₂ цепи;
- Neko остаётся `fictional_assumption`/`species_proxy`; контрактно проверяются
  только порядковые отличия от human;
- absorption rate и circadian thresholds — expert_estimate без calibration dataset.

Допустимая формулировка после этой работы: «детерминированная coarse
organism-level интеграционная модель с параметрами внутри опубликованных
физиологических диапазонов». Формулировка «биологически реалистичная» — только
после mechanism-specific calibration/validation по схеме из
[biology-realism.md](../research/biology-realism.md).
