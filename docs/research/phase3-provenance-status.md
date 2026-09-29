# Phase 3 Provenance Status and Gate Assessment

Статус: operational assessment; не закрытие gates
Дата: 2026-09-22
Связанные документы: [ADR-0014](../adr/0014-fidelity-envelope-and-validation-evidence.md), [Phase 3 plan](../plans/0006-phase3-everyday-physiology.md), [renal-validation-evidence.md](renal-validation-evidence.md), [biology-realism.md](biology-realism.md)

## 1. ADR-0014 критерии для realism claim

> Realism claim допустим только если все параметры внутри envelope имеют provenance `measured` или `derived`, validation scenario сравнивает output с independent reference в пределах declared uncertainty, и claim scope ограничен validation horizon.

Provenance tiers (убывание силы):
1. `measured` — published reference data или direct measurement с DOI/URL
2. `derived` — расчёт из measured inputs с указанной формулой
3. `expert_estimate` — оценка с named source
4. `synthetic_fixture` — test-only value без external validity claim

**Realism claim требует tier 1 или 2 для всех параметров envelope.**

## 2. Phase 3.1 (Cardiorespiratory / Gas Exchange) — текущий статус

### Реализованное состояние

`BloodState` (`causal-kernel/src/blood.rs`):
- `blood_volume_mm3: mm3` — baseline 5_000_000 mm³
- `hb_tetramer_umol: umol` — baseline 10_000 µmol
- `arterial_o2_umol: umol`, `venous_co2_umol: umol`
- `map_mpa: mPa` — baseline 12_000_000 mPa
- `lung_diffusion_umol_per_s: umol/s` — human 300, neko 180

### Провенанс параметров

| Parameter | Value | Provenance tier | Source | Status |
|---|---|---|---|---|
| `energy_per_umol_o2` | 470_000 uJ/umol | `expert_estimate` | Weir 1949 equation, ±10% uncertainty | No DOI/table extraction |
| `RQ` | 0.85 (range 0.7–1.0) | `expert_estimate` | Mixed metabolism assumption | No measured time-series |
| `lung_diffusion` human | 300 umol/s | `expert_estimate` | Textbook nominal value | No individual measurement |
| `lung_diffusion` neko | 180 umol/s | `synthetic_fixture` | 60% scaling assumption | Fictional species |
| `blood_volume` baseline | 5_000_000 mm³ | `expert_estimate` | ~5 L human adult | No measured cohort |
| `hb_tetramer` baseline | 10_000 umol | `expert_estimate` | Derived from Hb 150 g/L | No measured individual |
| `map` baseline | 12_000_000 mPa | `expert_estimate` | 120 mmHg nominal | No measured trace |

**Highest tier: `expert_estimate`**

### Validation evidence

Тесты (`causal-kernel/tests/blood_gas.rs`):
- Resting arterial saturation 950–1000 permille держится 60 s
- O2 overdraft rejection без partial metabolism
- `1×60s == 60×1s` hash determinism
- Restart parity
- Neko < human capacity scaling

**Validation type: `synthetic_fixture`** — нет independent measured time-series, нет comparison с human/animal O2 saturation traces, нет uncertainty quantification.

### Отсутствующие механизмы для realism claim

- Нет Severinghaus O2 dissociation curve `sat(PO2, T, pH)`
- Нет acid-base balance `pCO2/pH/HCO3-` Henderson-Hasselbalch
- Нет dissolved vs bound O2
- Нет `cardiac_output = stroke_volume × HR` Frank-Starling
- MAP константа, не из `compliance·volume`

### Gate 3.1 статус

**Провенанс:** tier 3 (`expert_estimate`) и tier 4 (`synthetic_fixture`)
**Validation:** synthetic only, no empirical
**Envelope:** resting baseline adult only; moderate activity undeclared
**Upgrade path:** declared but not implemented

**Gate 3.1: OPEN** — не проходит ADR-0014 для realism claim.

## 3. Phase 3.2 (Renal / Fluids / Electrolytes) — текущий статус

### Реализованное состояние

`RenalState` (`causal-kernel/src/renal.rs`):
- `total_body_water_mm3: mm3` — baseline 42_000_000 mm³
- `plasma_mm3: mm3` — baseline 3_000_000 mm³
- `plasma_sodium_umol: umol` — baseline 420_000 µmol
- `urine_water_mm3: mm3`, `urine_sodium_umol: umol` — boundary ledgers

### Провенанс параметров

| Parameter | Value | Provenance tier | Source | Status |
|---|---|---|---|---|
| Baseline TBW | 42_000_000 mm³ | `expert_estimate` | ~42 L adult 60% body weight | No measured individual |
| Baseline plasma volume | 3_000_000 mm³ | `expert_estimate` | ~3 L nominal | No measured cohort |
| Baseline plasma Na | 420_000 µmol | `expert_estimate` | 3 L × 140 mmol/L | No measured trace |
| Corrective excretion rate | 17 mm³/s water | `synthetic_fixture` | Ceiling для 24 h balance | No physiological measurement |
| Corrective excretion rate | 1 µmol/s sodium | `synthetic_fixture` | Ceiling для 24 h balance | No physiological measurement |
| Blood-volume coupling | 8 mPa/(mm³) | `synthetic_fixture` | Linear MAP response | No cardiovascular validation |

**Highest tier: `expert_estimate`** для baselines, **`synthetic_fixture`** для mechanism coefficients.

### Validation evidence

Тесты (multiple files):
- `renal.rs`: synthetic 24 h balance, bolus MAP response 3–5 kPa на +500 ml
- `renal_replay.rs`: fast/audit replay intake-only sequences, artifact corruption SafeStop
- `renal_boundary_program.rs`: archived intake/excretion/blood proposals, ADR-0016 anchors
- `canonical_timeline.rs`: 1×60s == 60×1s determinism, partition parity
- `canonical_worker_parity.rs`: independent engines produce identical transitions

**Validation type: `synthetic_fixture`** — нет independent measured time-series.

### Evidence ledger gaps (renal-validation-evidence.md)

Найденные datasets **не admitted**:
1. **Wouda et al. 2019** (oral water load): gastro-intestinal absorption не измерена; текущий `ingest_fluid` даёт мгновенный plasma bolus; protocol несовместим с renal-only boundary
2. **Wenstedt et al. 2021** (IV hypertonic): **не независим** от Olde Engberink 2017 (тот же healthy-control dataset); IV saline включает sodium storage и 8-day low-salt diet; выходит за resting envelope
3. **Pedersen et al. 2010** (oral water): Table 2 содержит групповые медианы и IQR, не individual trajectories; median ± IQR/2 не восстанавливает quartile bounds; oral absorption не измерена
4. **Jensen et al. 2013** (isotonic saline): Table 3 содержит подозрительные units (u-Na 1.24 mmol/min при baseline, u-K 26.5 mmol/min); требует сверки с original data/erratum; oral water каждые 30 min не устраняет absorption uncertainty
5. **Weissenbacher et al. 2019** (ex-vivo kidneys): discarded transplant grafts после cold ischemia, не healthy in-vivo; numeric paired perfusate/urine Na time-series не опубликованы; nominal circuit settings не являются measured individual inputs; перенос на healthy physiology или Neko не доказан

**Статус: ни один dataset не прошёл admission для calibration или holdout.**

### Отсутствующие механизмы для realism claim

- Нет compartments: plasma/interstitial/intracellular не разделены
- Нет nephron segments: filtration/reabsorption одной скоростью
- Нет GFR model с net filtration pressure и Kf
- Нет ADH/aldosterone/RAAS hormonal control
- Нет osmolality solver и electroneutrality
- Нет urea handling

### Gate 3.2 статус

**Провенанс:** tier 3 (`expert_estimate`) baselines, tier 4 (`synthetic_fixture`) mechanisms
**Validation:** synthetic only, no admitted empirical datasets
**Envelope:** resting baseline adult only; oral absorption excluded, hormones excluded
**Upgrade path:** declared but not executable through commit
**Canonical timeline:** activated with worker parity, но не с N workers → shared writer

**Gate 3.2: OPEN** — не проходит ADR-0014 для realism claim; обязательное empirical validation (решение 2026-09-07) не выполнено.

## 4. Runtime integrity vs biological realism

### Что работает (causally verifiable)

✅ Dimensional consistency: все quantities имеют explicit units
✅ Conservation: exact integer accounting для water/Na/energy
✅ Determinism: 1×60s == 60×1s, restart parity, worker parity
✅ Replay integrity: fast/audit agreement, artifact corruption detection
✅ Canonical timeline: per-second transitions с archived dependencies
✅ SafeStop: durable failure recording без silent corruption
✅ Bounded validity: OutsideValidityRange rejection без extrapolation

**Эти свойства проходят INVARIANTS и ADR-0009/0010/0016.**

### Что НЕ работает (biological realism)

❌ Measured parameters: все baselines `expert_estimate` или `synthetic_fixture`
❌ Empirical validation: ни один independent time-series не admitted
❌ Uncertainty quantification: нет propagated uncertainty через mechanisms
❌ Calibration/holdout split: нет datasets для split
❌ Species validation: Neko parameters — fictional species_proxy
❌ Fine mechanisms: coarse single-rate models без physiology solvers
❌ Hormonal control: endocrine excluded from scope
❌ Substrate metabolism: glucose/AA/FA не моделируются

**Эти gaps блокируют ADR-0014 realism claim.**

## 5. Термин usage guidance

### Допустимые термины для текущего состояния

✅ "Causally verifiable" — conservation, determinism, replay проходят
✅ "Dimensionally consistent" — units и bounds проверены
✅ "Synthetic baseline" — test anchors для integration
✅ "Expert estimate envelope" — named sources, declared uncertainty
✅ "Coarse mechanism" — single-compartment/single-rate models

### Недопустимые термины без measured validation

❌ "Realistic physiology" — требует tier 1/2 provenance
❌ "Validated model" — требует independent empirical comparison
❌ "Calibrated parameters" — требует fitting и holdout evidence
❌ "Biological accuracy" — требует uncertainty-bounded agreement
❌ "Clinical applicability" — требует measured cohort validation

## 6. Следующие шаги для закрытия gates

### Prerequisite: data admission

1. **Найти совместимые datasets:**
   - Renal-only boundary inputs (IV с known composition/rate)
   - Healthy resting cohort
   - Time-resolved water/Na outputs
   - Individual traces, не только группы медиан
   - Published uncertainty (SD/SEM/CI различены)
   - Open license или author permission

2. **Зафиксировать protocol:**
   - Question of interest
   - Population/exclusion criteria
   - Input route и composition
   - Sampling timepoints
   - Measurement methods
   - Заранее объявленные thresholds (из measurement + model uncertainty)

3. **Calibration/holdout split:**
   - Отделить datasets **до** настройки параметров
   - Запретить изменение model/thresholds по holdout outputs
   - Документировать split decision

4. **Провести validation:**
   - Executable comparison через public seams
   - Uncertainty-bounded residuals
   - Negative cases (outside envelope rejection)
   - Conservation + replay matrix обязательны дополнительно

### Prerequisite: resolution upgrade

1. **Executable fine mechanism:**
   - Nephron segments или plasma/interstitial/intracellular compartments
   - Versioned ABI через `ArtifactBundle::admit`
   - Passing через `WorldEngine::commit`

2. **Lift/projection continuity:**
   - Coarse → fine preserves totals ±declared tolerance
   - Fine → coarse preserves observables ±declared tolerance
   - Lineage preserved через resolution changes

3. **Independent validation:**
   - Fine model validated против отдельного holdout
   - Upgrade не меняет coarse validation evidence

### Prerequisite: full dependency manifest

Для canonical timeline требуются archived execution contracts:
- Scheduler/circadian (sleep debt, metabolic demand)
- Digestion (buffer → chemical transfer)
- Gas exchange (O2/CO2 boundary)
- Metabolism (chemical → thermal)
- Renal (water/Na redistribution) — частично реализовано

### Non-prerequisite clarifications

**НЕ требуется для gate closure:**
- 365-day integration (Phase 8 release gate)
- Workstation capacity sweep (Phase 8)
- Hormones (Phase 3.4)
- Substrate mechanisms (Phase 3.3)
- Active thermoregulation (Phase 3.5)
- Musculoskeletal fatigue (Phase 3.6)
- Microbiome (Phase 3.7)

## 7. Gate decision

**Phase 3.1 gate: OPEN**
- Highest provenance: `expert_estimate`
- Validation: synthetic only
- Missing: empirical O2 saturation traces, cardiovascular solver

**Phase 3.2 gate: OPEN**
- Highest provenance: `expert_estimate` (baselines), `synthetic_fixture` (mechanisms)
- Validation: synthetic only, no admitted datasets
- Missing: empirical water/Na time-series, resolution upgrade, full dependency manifest

**Overall Phase 3 gate: OPEN**
- Slices 3.3–3.7 not implemented
- Empirical validation mandatory per 2026-09-07 decision
- ADR-0014 realism claim criteria not met

**Recommendation:** Document current state as `synthetic_fixture` / `expert_estimate` tier, leave gates explicitly open, continue Phase 3.3+ implementation or pursue empirical validation datasets.
