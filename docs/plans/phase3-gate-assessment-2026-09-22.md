# Phase 3 Gate Assessment — 3.1 и 3.2 статус перед закрытием

Статус: gate review от 2026-09-22
Цель: оценить готовность gates 3.1 (cardiorespiratory) и 3.2 (renal) для закрытия; проверить biological realism для ИИ живущего в мире

Связанные документы: [ROADMAP.md](../../ROADMAP.md), [INVARIANTS.md](../../INVARIANTS.md), [Phase 3 plan](0006-phase3-everyday-physiology.md), [ADR-0014](../adr/0014-fidelity-envelope-and-validation-evidence.md), [phase3-provenance-status.md](../research/phase3-provenance-status.md), [renal-validation-evidence.md](../research/renal-validation-evidence.md)

## 1. Критерии закрытия gates (из ROADMAP и Phase 3 plan)

Для каждого system gate обязательны:
- ✅ `MechanismContract` с units, provenance, uncertainty, conservation
- ✅ Reference observables
- ⚠️ **Upgrade path** — declared but not executable through commit
- ❌ **Focused validation** — synthetic only, no empirical
- ❌ **Empirical evidence** — mandatory per 2026-09-07 decision

Для realism claim (ADR-0014):
- ❌ Provenance tier 1 (`measured`) или tier 2 (`derived`) для всех параметров
- ❌ Independent validation dataset с uncertainty quantification
- ❌ Calibration/holdout split

## 2. Phase 3.1 (Cardiorespiratory / Gas Exchange) — gate assessment

### Реализованный runtime

`BloodState` (`causal-kernel/src/blood.rs`):
- Blood volume, Hb tetramers, arterial O2, venous CO2, MAP, lung diffusion
- Energy per O2: 470,000 µJ/µmol (Weir 1949, expert_estimate ±10%)
- RQ: 0.85 (range 0.7–1.0, expert_estimate)
- Lung diffusion: human 300 µmol/s, neko 180 µmol/s (expert_estimate / synthetic_fixture)

### Runtime integrity (✅ проходит)

✅ Conservation: O2+CO2 ±1 µmol, energy ±1 µJ
✅ Determinism: `1×60s == 60×1s` hash parity
✅ Restart parity, partition parity
✅ Oxygen overdraft rejection без partial commit
✅ Observable: arterial saturation 950-1000 permille в покое

### Biological realism gaps (❌ блокирует gate)

**Критические отсутствующие механизмы:**
1. MAP hardcoded delta `8 * volume_delta` без physiological basis (blood.rs:115)
2. Venous CO2 фиксирован 24,000 µmol, не масштабируется по морфотипу (blood.rs:79-81)
3. Arterial saturation hardcoded 98%, нет Severinghaus curve (pCO2, pH, temp) (blood.rs:158-175)
4. Нет venous O2 compartment или saturation tracking (спец: 70-75%)
5. Нет dissolved O2 in plasma (~3 ml/L)
6. CO2 не разделён на dissolved + HCO3- + carbaminohemoglobin (нет acid-base)
7. Нет cardiac output / blood flow model (diffusion не ограничена 5 L/min CO)
8. CO2 drift tolerance ±500 µmol loose, нет venous sat test (blood_gas.rs:38-40)

**Provenance:**
- Highest tier: `expert_estimate` (tier 3)
- Neko: `synthetic_fixture` (tier 4)

**Validation:**
- Synthetic only (`blood_gas.rs`)
- Нет empirical O2 saturation time-series
- Нет uncertainty quantification

**Envelope:**
- Resting baseline adult only
- Moderate activity undeclared

### Gate 3.1 статус: **OPEN**

Не проходит ADR-0014 для realism claim.
Термин "realistic cardiorespiratory physiology" недопустим.
Допустимо: "causally verifiable gas exchange", "coarse O2/CO2 accounting", "expert estimate envelope".

## 3. Phase 3.2 (Renal / Fluids / Electrolytes) — gate assessment

### Реализованный runtime

`RenalState` (`causal-kernel/src/renal.rs`):
- Total body water, plasma volume/Na, urine water/Na boundaries
- Canonical timeline activated (CanonicalPhysiologyV2)
- Archived executable: intake-v1, corrective-excretion-v1, blood-volume-v1
- Conservation: exact integer accounting mm³/µmol

### Runtime integrity (✅ проходит)

✅ Conservation report: water body+urine, sodium plasma+urine на каждой transition
✅ Determinism: `1×60s == 60×1s` canonical records и hash
✅ Worker parity: independent engines → identical transitions
✅ Fast/audit replay для mixed intake/advance sequences
✅ Artifact corruption → durable `SafeStop`
✅ Restart parity, partition parity, retry/conflict handling

### Biological realism gaps (❌ блокирует gate)

**Критические отсутствующие механизмы:**
1. Linear corrective model без osmolarity threshold и ADH trigger (renal.rs:127-143)
2. Intake routes всю воду прямо в plasma, должна partition ECF→ICF (renal.rs:105-107)
3. Нет baseline insensible loss или obligatory urine (~0.5-1 L/day) (renal.rs:124-125)
4. Acknowledged synthetic resting-only, нет ADH/RAAS/nephron segments (renal.rs:1-7)
5. Plasma Na tracked но osmolarity не рассчитывается (ADH/thirst ~280-295 mOsm/kg) (renal.rs:92-94)
6. Только TBW и plasma tracked, нет ICF/ECF compartmentalization (renal.rs:8-9)
7. MAX_CORRECTIVE rates (17 ml/min, 1 µmol/s Na) — 14% и <<1% GFR/UNA ranges (renal.rs:17-18)

**Provenance:**
- Baselines: `expert_estimate` (tier 3) — 42L TBW, 3L plasma, 420,000 µmol Na
- Mechanism rates: `synthetic_fixture` (tier 4) — 17 mm³/s water, 1 µmol/s Na
- Blood coupling: `synthetic_fixture` (tier 4) — 8 mPa/mm³

**Validation:**
- Synthetic only (renal.rs, renal_replay.rs, canonical_timeline.rs)
- Нет admitted empirical datasets (renal-validation-evidence.md)
- Wouda 2019: oral absorption не измерена
- Wenstedt 2021: NOT independent от Olde Engberink 2017
- Pedersen 2010: medians ± IQR, не individual traces
- Jensen 2013: suspicious units (u-Na 1.24 mmol/min, u-K 26.5 mmol/min)
- Weissenbacher 2019: ex-vivo discarded grafts, numeric series не опубликованы

**Открытые prerequisites:**
- Executable resolution upgrade через `commit` (fixture digest — placeholder)
- N workers → shared canonical writer (текущий parity — independent engines)
- Full dependency manifest (scheduler/circadian/digestion/gas/metabolism)
- Independent empirical time-series (mandatory per 2026-09-07)

### Gate 3.2 статус: **OPEN**

Не проходит ADR-0014 для realism claim.
Обязательное empirical validation (решение 2026-09-07) не выполнено.
Термин "realistic renal physiology" недопустим.
Допустимо: "causally verifiable water/sodium accounting", "coarse fluid balance", "synthetic baseline".

## 4. Neko morphotype realism assessment

**Neko = catgirl (anthropomorphic humanoid hybrid), NOT domestic cat**

Target model: Human baseline + functional cat ears (enhanced hearing) + functional tail (balance) + hybrid obligate carnivore metabolism. Body mass: 30-40 kg (small humanoid frame).

`contracts/fixtures/morphotypes/neko-minimal.json`:

### Параметры в допустимом range (✅)

1. **Body mass** (line 117): 30 kg stated — **acceptable** для small humanoid hybrid (target 30-40 kg)
2. **Heat capacity** (line 112): 104.7M uj/mK для 30kg — **reasonable** (human ~55kg = ~192M, linear scaling ~3.5 kJ/K)
3. **Metabolism** (lines 88-110): 55W awake, 45W asleep — **plausible** для small frame (human ~70-100W resting)
4. **Fur insulation** (line 120): 3.2 W/K conductance vs human 5.6 W/K — **reasonable** для fur coverage

### Отсутствующие humanoid baseline parameters (⚠️ gaps)

1. **Missing reference core temperature** (line 53): No explicit setpoint in physiological_parameters. Human 37°C baseline assumed; catgirl может иметь slightly elevated (37.5-38°C) но это optional enhancement.
2. **Missing initial body mass** (line 129): phenotype parameter_overrides empty. Should explicitly state 30-35 kg для female-neko-v1.
3. **Missing resting heart rate**: humanoid baseline ~60-80 bpm (не cat 110-140 bpm, humanoid circulation)
4. **Missing respiratory rate**: humanoid baseline ~12-16 bpm (не cat 20-30 bpm, humanoid lungs)

### Отсутствующие functional cat-specific mechanisms (⚠️ functional features)

1. **Hearing mechanism без quantitative parameters** (lines 60-64): mechanism_id "neko.hearing-transfer" declared, но нет frequency response curve, sensitivity threshold, directional acuity. Enhanced hearing vs human (extended high-frequency range) требует measurable parameters.
2. **Tail balance underspecified** (lines 66-70): mechanism_id "neko.balance-tail-coupling" declared, tail inertia 120000 mgm², но нет proprioceptive sensitivity model, нет righting reflex threshold, нет integration с vestibular balance.
3. **Ear articulation** (lines 60-64): cat auricles rotate ~180° для sound localization; mechanism не включает voluntary/reflex ear movement model.
4. **Auricle thermoregulation** (lines 72-76): mechanism_id declared, convective area 0.018 m² stated, но нет active vasomotor control (vasoconstriction/dilation) для thermal regulation.

### Отсутствующий hybrid obligate carnivore metabolism (⚠️ metabolic realism)

1. **Taurine requirement**: obligate carnivores не синтезируют достаточно taurine; требует dietary intake ~50-100 mg/day. Нет taurine pool, нет deficiency threshold, нет cardiac/retinal consequences.
2. **Protein requirements**: obligate carnivores требуют higher protein intake (>30% калорий) и gluconeogenesis из amino acids. Нет minimum protein threshold.
3. **Carbohydrate metabolism**: cats имеют reduced amylase и glucose tolerance; hybrid может иметь intermediate capacity, но это не specified.
4. **Renal concentration**: obligate carnivores имеют higher urine osmolality (cats 2-3× human) для water conservation. Neko renal parameters не adjusted от human baseline.

### Neko assessment: **ЧАСТИЧНО СПЕЦИФИЦИРОВАН**

✅ **Работает** (humanoid baseline + cosmetic cat features):
- Body mass 30 kg reasonable для small humanoid
- Thermal parameters plausible для fur-insulated humanoid
- Metabolism 55W awake reasonable для small frame
- Anatomy graph включает ears/tail structures

⚠️ **Gaps** (functional cat features underspecified):
- Enhanced hearing declared но без quantitative frequency response / sensitivity
- Tail balance declared но без proprioceptive / righting reflex models
- Ear articulation / thermoregulation mechanisms declared но без control parameters
- Obligate carnivore metabolism NOT implemented (no taurine, no protein threshold, no renal concentration)

❌ **Блокирует realism claim** (missing measurements):
- Все cat-specific parameters — expert_estimate или schema_only
- Нет measured data для catgirl physiology (fictional species)
- Validation fixtures — schema_only и expert_estimate, не empirical

### Требуется для catgirl realism

1. **Quantitative functional mechanisms**:
   - Hearing: frequency response curve (human 20Hz-20kHz → neko 20Hz-40kHz?), sensitivity threshold in dB
   - Tail balance: proprioceptive sensitivity model, righting reflex threshold (degrees), moment arm contribution
   - Ear articulation: voluntary/reflex rotation range, sound localization accuracy improvement

2. **Hybrid metabolism specification**:
   - Taurine pool (mmol), dietary requirement (mg/day), deficiency threshold
   - Minimum protein intake (% calories or g/day)
   - Intermediate carbohydrate tolerance (vs pure carnivore)
   - Renal concentration capacity (intermediate between human 1.0-1.3 kg/L and cat 2.5-3.0 kg/L?)

3. **Explicit phenotype parameters**:
   - Body mass: 30-35 kg для female-neko-v1
   - Core temperature setpoint: 37.0-37.5°C (slightly elevated optional)
   - Resting HR: 65-75 bpm (humanoid baseline, не cat rate)
   - Resting RR: 12-16 bpm (humanoid baseline)

4. **Provenance improvement**:
   - Fictional species → все parameters остаются expert_estimate или derived
   - Validation требует internal consistency checks (thermal balance, metabolic steady-state)
   - Cannot achieve tier 1/2 (measured/derived from measurements) для fictional species
   - Acceptance criterion: dimensionally consistent, conserved quantities, declared uncertainty

Текущее состояние: **humanoid baseline с объявленными cat features, functional mechanisms underspecified, obligate carnivore metabolism absent**. Это sufficient для synthetic integration tests, но insufficient для "maximally realistic catgirl" без quantitative functional models.

## 5. Runtime integrity vs biological realism

### Что работает (✅ causally verifiable)

✅ Dimensional consistency: explicit units для всех quantities
✅ Conservation: exact integer accounting water/Na/O2/CO2/energy
✅ Determinism: partition/restart/worker parity
✅ Replay integrity: fast/audit agreement, corruption detection
✅ Canonical timeline: per-second transitions, archived dependencies
✅ SafeStop: durable failure без silent corruption
✅ Bounded validity: OutsideValidityRange rejection

**Эти свойства проходят INVARIANTS.md и ADR-0009/0010/0016.**

### Что НЕ работает (❌ biological realism)

❌ Measured parameters: все baselines `expert_estimate` или `synthetic_fixture`
❌ Empirical validation: ни один independent time-series не admitted
❌ Uncertainty quantification: нет propagated uncertainty
❌ Calibration/holdout split: нет datasets для split
❌ Species validation: Neko parameters — fictional species_proxy с mass errors
❌ Fine mechanisms: coarse single-rate models без physiology solvers
❌ Hormonal control: endocrine excluded from scope (будет 3.4)
❌ Substrate metabolism: glucose/AA/FA не моделируются (будет 3.3)
❌ Critical missing mechanisms: см. разделы 2-3 выше

**Эти gaps блокируют ADR-0014 realism claim.**

## 6. Gate decision

### Phase 3.1 gate: **OPEN**
- Провенанс: tier 3 (`expert_estimate`)
- Validation: synthetic only
- Missing: empirical O2 traces, cardiovascular solver, acid-base, venous compartment
- Biological realism: **НЕТ**

### Phase 3.2 gate: **OPEN**
- Провенанс: tier 3 baselines, tier 4 mechanisms
- Validation: synthetic only, no admitted datasets
- Missing: empirical water/Na traces, resolution upgrade, ADH/RAAS, compartments
- Biological realism: **НЕТ**

### Neko morphotype: **ЧАСТИЧНО СПЕЦИФИЦИРОВАН**
- Humanoid baseline: ✅ mass 30kg acceptable, thermal plausible
- Functional cat features: ⚠️ declared but underspecified (hearing, tail, thermoregulation)
- Obligate carnivore metabolism: ❌ not implemented (taurine, protein, renal concentration)
- Cardiovascular/respiratory: ⚠️ humanoid baseline assumed, explicit rates missing

### Overall Phase 3 gate: **OPEN**
- Slices 3.3-3.7: NOT STARTED
- Empirical validation mandatory (2026-09-07 decision): NOT COMPLETED
- ADR-0014 realism criteria: NOT MET (tier 3-4 only)
- Biological realism для "ИИ (catgirl) живущего в мире": **ЧАСТИЧНО** (runtime integrity да, empirical validation нет, functional cat features underspecified)

## 7. Допустимая терминология

### ✅ Допустимо (текущее состояние)

- "Causally verifiable" — conservation, determinism, replay работают
- "Dimensionally consistent" — units и bounds проверены
- "Synthetic baseline" — test anchors для integration
- "Expert estimate envelope" — named sources, declared uncertainty
- "Coarse mechanism" — single-compartment/single-rate models
- "Runtime integrity verified" — INVARIANTS проходят

### ❌ Недопустимо (требует tier 1/2 + empirical)

- "Realistic physiology"
- "Validated model"
- "Calibrated parameters"
- "Biological accuracy"
- "Clinical applicability"
- "Realistic Neko"

## 8. Следующие шаги для закрытия gates

### Prerequisite 1: Уточнить Neko morphotype (catgirl)

**Humanoid baseline clarifications:**
1. Добавить explicit body mass в phenotype: 30-35 kg для female-neko-v1
2. Добавить explicit core temperature setpoint: 37.0-37.5°C (optional slight elevation)
3. Добавить humanoid cardiovascular rates: HR 65-75 bpm, RR 12-16 bpm
4. Current thermal parameters (104.7M uj/mK heat capacity, 3.2 W/K conductance) — acceptable для 30kg fur-insulated humanoid

**Functional cat features quantification:**
5. Hearing mechanism: добавить frequency response curve (20Hz-40kHz extended range?), sensitivity threshold (dB), directional acuity improvement over human
6. Tail balance: добавить proprioceptive sensitivity model, righting reflex threshold (degrees), quantitative balance contribution
7. Ear articulation: добавить voluntary/reflex rotation model (~180° range), sound localization accuracy
8. Auricle thermoregulation: добавить active vasomotor control parameters (vasoconstriction/dilation rates)

**Hybrid obligate carnivore metabolism:**
9. Taurine pool (mmol), dietary requirement (~50-100 mg/day), deficiency threshold, cardiac/retinal consequences
10. Minimum protein intake threshold (>30% calories or g/day)
11. Intermediate carbohydrate tolerance (vs pure carnivore)
12. Renal concentration capacity: intermediate between human 1.0-1.3 and cat 2.5-3.0 kg/L osmolality

**Provenance note:** Fictional species → все parameters остаются expert_estimate; tier 1/2 недостижим, но internal consistency + dimensional correctness обязательны

### Prerequisite 2: Найти compatible empirical datasets

Для 3.2 (renal):
- Renal-only boundary inputs (IV с known composition/rate)
- Healthy resting cohort
- Time-resolved water/Na outputs, individual traces
- Published uncertainty (SD/SEM/CI различены)
- Open license или author permission
- Calibration/holdout split **до** настройки параметров

Для 3.1 (cardiorespiratory):
- O2 saturation time-series при known workload
- Healthy cohort
- Individual traces с measurement uncertainty
- Resting baseline envelope

### Prerequisite 3: Executable resolution upgrade

1. Fine mechanism через `ArtifactBundle::admit`
2. Passing через `WorldEngine::commit`
3. Lift/projection continuity ±declared tolerance
4. Lineage preserved через resolution changes
5. Independent validation против отдельного holdout

### Prerequisite 4: Full dependency manifest

Archived execution contracts для canonical timeline:
- Scheduler/circadian (sleep debt, metabolic demand)
- Digestion (buffer → chemical transfer)
- Gas exchange (O2/CO2 boundary) — частично реализовано
- Metabolism (chemical → thermal)
- Renal (water/Na redistribution) — реализовано

### Prerequisite 5: Исправить missing physiology mechanisms

3.1 (cardiorespiratory):
- Severinghaus O2 dissociation curve
- Acid-base balance (Henderson-Hasselbalch)
- Venous O2 compartment
- Cardiac output model
- MAP from compliance·volume

3.2 (renal):
- ICF/ECF/plasma compartments
- Osmolarity calculation
- GFR model (net filtration pressure, Kf)
- Baseline insensible loss + obligatory urine
- ADH/RAAS hormonal control (или defer to 3.4)

## 9. Рекомендация

**НЕ закрывать gates 3.1 и 3.2** до выполнения критических prerequisite.

Текущее состояние документировать как:
- Runtime integrity: **verified** ✅
- Causal verifiability: **established** ✅ (conservation, determinism, replay)
- Biological realism: **NOT established** ❌ (no empirical validation)
- Provenance tier: **3-4** (expert_estimate / synthetic_fixture)
- Validation: **synthetic only**
- Neko morphotype: **humanoid baseline acceptable**, functional cat features underspecified, obligate carnivore metabolism missing

Продолжить работу по приоритету:

**Критический приоритет (блокирует gate closure):**
1. Найти/admit empirical datasets для 3.1 (O2 saturation) и 3.2 (water/Na)
2. Implement missing critical physiology mechanisms (Severinghaus, acid-base, ADH/RAAS, compartments)
3. Implement executable resolution upgrade через commit

**Высокий приоритет (улучшает realism):**
4. Quantify Neko functional cat features (hearing frequency response, tail balance model, ear articulation)
5. Implement hybrid obligate carnivore metabolism (taurine, protein requirements, renal concentration)
6. Complete dependency manifest для canonical timeline

**Средний приоритет (enhancement):**
7. Explicit Neko phenotype parameters (body mass, HR, RR, core temp)
8. Neko internal consistency validation (thermal balance, metabolic steady-state)

После выполнения критического приоритета — повторная gate assessment.

Термин "realistic physiology for AI (catgirl) living in the world" допустим только после:
- Tier 1/2 provenance для human baseline (где применимо)
- Empirical validation по ADR-0014
- Quantitative functional models для cat-specific features
- Internal consistency для fictional hybrid species
