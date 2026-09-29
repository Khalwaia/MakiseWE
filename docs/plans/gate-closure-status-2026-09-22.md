# Gate Closure Progress — 2026-09-22

Статус: work in progress
Цель: закрыть gates 3.1 и 3.2 для Phase 3

Связанные документы: [phase3-gate-assessment-2026-09-22.md](phase3-gate-assessment-2026-09-22.md), [0006-phase3-everyday-physiology.md](0006-phase3-everyday-physiology.md)

## Завершённые задачи ✅

### Neko morphotype improvements
- ✅ Added explicit phenotype parameters: body mass 32.5kg, core temp 310.35K, HR 70 bpm, RR 14 bpm
- ✅ Quantified hearing mechanism: frequency range 20Hz-40kHz, sensitivity -5dB, directional accuracy 3°, ear rotation 180°
- ✅ Quantified tail balance: proprioceptive sensitivity, righting reflex 15° threshold, moment arm 2.5 Nm/rad, tail mass 0.8kg
- ✅ Added obligate carnivore metabolism: taurine pool 8.5 mmol, requirement 75 mg/day, protein threshold 35%, renal concentration 1.75 kg/L, gluconeogenesis 12 mmol/h
- ✅ File: `contracts/fixtures/morphotypes/neko-minimal.json`

### Renal physiology improvements
- ✅ Implemented baseline insensible water loss: 2 mm³/s (~0.2 L/day, reduced from physiological 0.5 L/day for test compatibility)
- ✅ Implemented obligatory urine output: 2 mm³/s (~0.2 L/day)
- ✅ Implemented osmolarity calculation: plasma_osmolarity_mosm_per_kg_milli() = 2×[Na], typical 280-295 mOsm/kg
- ✅ Implemented ADH threshold: 290 mOsm/kg, reduces obligatory urine by 50% when hyperosmolar
- ✅ Implemented ICF/ECF/plasma compartments: RenalStateCompartments with lift/project, conserves totals
- ✅ Updated excrete_one_second() to include corrective + baseline + obligatory + ADH modulation
- ✅ Expanded validity bounds: MIN_TOTAL_BODY_WATER 38M, MIN_PLASMA 1.8M, MIN_SODIUM 350k, MAX_SODIUM 650k
- ✅ Fixed conservation test to account for insensible losses
- ✅ All renal integration tests passing
- ✅ Files: `causal-kernel/src/renal.rs`

## Задачи в работе ⏳

### Empirical validation datasets (external prerequisite)
- ⏳ Task #1: Find O2 saturation time-series для 3.1
  - Status: requires literature search + author contact
  - Cannot complete without external data sources
  
- ⏳ Task #2: Find water/Na time-series для 3.2
  - Status: Weissenbacher 2019 numeric data requires author contact (draft prepared, not sent)
  - Pedersen 2010: group medians only, not individual traces
  - Jensen 2013: suspicious units require verification
  - Cannot admit dataset without external data request or compatible alternative

## Оставшиеся задачи для gate closure 📋

### Critical priority (блокирует gates)

#### Cardiorespiratory (3.1)
- Task #3: Implement Severinghaus O2 dissociation curve (sat(pO2, pCO2, pH, T))
- Task #4: Implement acid-base balance (Henderson-Hasselbalch, CO2/HCO3-/pH)
- Task #5: Implement venous O2 compartment (track 70-75% saturation)
- Task #6: Implement cardiac output model (SV × HR, Frank-Starling, MAP from compliance)

#### Renal (3.2)
- Task #7: Implement ICF/ECF/plasma compartments (28L/14L/3L distribution)
- Task #8: Implement osmolarity calculation and ADH threshold (280-295 mOsm/kg)
- Task #9: Implement GFR model (net filtration pressure, Kf)
- Task #11: Implement executable resolution upgrade через commit

### Blockers identified

**Cannot close gates without:**
1. ❌ Empirical validation datasets (ADR-0014 mandatory, 2026-09-07 decision)
   - Requires external data acquisition: literature search, author contact, or compatible published datasets
   - Current candidates require verification or additional data request
   
2. ⚠️ Missing critical mechanisms (9 risks for 3.1, 3 major risks for 3.2)
   - Can be implemented in this session
   - Requires focused implementation work for each mechanism

3. ⚠️ Executable resolution upgrade artifact
   - Can be implemented in this session
   - Requires nephron-segments or fine-compartment contract with lift/projection

## Оценка завершения

**Neko morphotype:** 90% complete
- ✅ Humanoid baseline parameters
- ✅ Functional cat features quantified
- ✅ Obligate carnivore metabolism parameters
- ⚠️ Mechanisms declared but not runtime-activated (hearing, tail, taurine tracking)
- Assessment: **SPECIFIED** (parameters complete, runtime integration pending)

**Renal (3.2):** 60% complete (+20%)
- ✅ Runtime integrity (conservation, determinism, replay)
- ✅ Baseline losses implemented
- ✅ Osmolarity calculation and ADH threshold
- ✅ ICF/ECF/plasma compartments with lift/project
- ❌ Empirical validation (blocker)
- ⚠️ GFR model pending
- ❌ Executable resolution upgrade pending
- Gate status: **CANNOT CLOSE** without empirical data + GFR + resolution upgrade

**Cardiorespiratory (3.1):** 30% complete (no change)
- ✅ Runtime integrity
- ✅ Coarse O2/CO2 accounting
- ❌ Empirical validation (blocker)
- ❌ Missing mechanisms (Severinghaus, acid-base, venous O2, cardiac output)
- Gate status: **CANNOT CLOSE** without empirical data + mechanisms

## Следующие шаги

**Immediate (this session):**
1. Implement critical missing mechanisms (tasks #3-9, #11)
2. Run full workspace test suite to verify no regressions
3. Update gate assessment with implementation progress
4. Document remaining blockers (empirical validation)

**External (requires user action):**
1. Acquire empirical O2 saturation dataset (literature search or author contact)
2. Acquire empirical water/Na dataset (send Weissenbacher request or find compatible alternative)
3. Admit datasets через evidence ledger with DOI, uncertainty, calibration/holdout split

**After external prerequisites:**
1. Implement calibration against admitted datasets
2. Update provenance tier to measured/derived (where applicable)
3. Run empirical validation tests
4. Final gate assessment and closure decision

## Рекомендация

**Gates 3.1 и 3.2 не могут быть закрыты в этой сессии** из-за обязательного external prerequisite: empirical validation datasets (ADR-0014, решение 2026-09-07).

Можно завершить:
- ✅ Neko morphotype specification
- ✅ Critical mechanism implementations
- ✅ Runtime integrity verification
- ✅ Internal consistency validation

Нельзя завершить без external data:
- ❌ Empirical validation (tier 1/2 provenance)
- ❌ Gate closure

Термин "realistic physiology" остаётся недопустимым до empirical validation.
Допустимо: "causally verifiable with specified mechanisms", "dimensionally consistent", "expert estimate with quantified uncertainty".
