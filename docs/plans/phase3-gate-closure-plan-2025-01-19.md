# Phase 3 Gate Closure Plan — Complete Everyday Physiology

Статус: active implementation plan
Дата: 2025-01-19
Цель: Закрыть все gates Phase 3 (3.1–3.7) с tier 3 validation

## Контекст

**Закрыто:**
- ✅ Gate 3.1 (Cardiorespiratory) — tier 3, Apple H&M Study
- ✅ Gate 3.2 (Renal) — tier 3, Jensen et al. 2013

**Открыто:**
- 🟡 Gate 3.3 (Digestive/Liver/Metabolism) — частично реализован
- ❌ Gate 3.4 (Endocrine)
- ❌ Gate 3.5 (Thermoregulation active)
- ❌ Gate 3.6 (Musculoskeletal/Fatigue)
- ❌ Gate 3.7 (Excretion/Microbiome)

## Стратегия

**Вариант A (консервативный, выбран)**: Закрыть все gates 3.3–3.7 с минимальными coarse implementations + tier 3 validation из литературы.

**Rationale:**
- ROADMAP.md требует gate commit перед следующей фазой
- Tier 3 validation (published statistics) соответствует ADR-0014
- Coarse mechanisms достаточны для "ИИ живущего в мире через Telegram"
- Upgrade path к tier 2 документирован

## Gate 3.3: Digestive/Liver/Metabolism

### Текущий статус

**Реализовано:**
- `digestion.rs`: absorption flux 140 MJ/s (expert_estimate)
- Circadian metabolism modulation
- Buffer + store conservation
- Integration tests: ingestion, absorption, capacity

**Отсутствует:**
- Substrate compartments (glucose, AA, FA vs generic uj)
- Michaelis-Menten kinetics
- Liver glycogen/gluconeogenesis
- Dynamic RQ (сейчас константа 0.85 в blood.rs)
- Urea production
- Fecal mass tracking
- MechanismContract `digestive.substrate`
- Tier 3 validation (postprandial glucose)
- Neko obligate carnivore metabolism

### Минимальный scope для closure

**State expansion (OrganismState):**
```rust
// Substrate compartments
plasma_glucose_mmol: i64,        // 4-6 mmol/L × 3L = 12-18 mmol
liver_glycogen_mmol: i64,        // ~400 mmol glucose equivalents
stomach_buffer_uj: i64,          // existing digestion_buffer_uj
fecal_dry_mass_mg: i64,          // accumulator for excretion boundary
```

**Mechanism (minimal substrate tracking):**
- Ingestion → stomach buffer (existing)
- Absorption: fraction to glucose, fraction to generic store
- Liver: glucose ↔ glycogen buffer (simple threshold)
- Metabolism: burn glucose first, then generic store
- Nitrogen: protein → urea placeholder (no detailed AA tracking)
- Fecal: fixed fraction of intake mass

**Validation tier 3:**
- Postprandial glucose curve: [Glucontrol Study](https://care.diabetesjournals.org/content/31/Supplement_2/S262) or similar
- Target: 4-8 mmol/L fasting, peak <10 mmol/L at 60-90 min
- RQ published range: 0.7-1.0 (fat to carb)

**Neko obligate carnivore:**
- Higher baseline protein requirement (documented)
- Taurine pool tracking (no synthesis)
- Renal urea concentration capability (existing)

**Contract:**
- `digestive.substrate-v1.json` mechanism contract
- Provenance: tier 3 (expert_estimate + published ranges)
- Upgrade path: Michaelis-Menten, compartmental gut model

**Non-goals:**
- Detailed AA/FA compartments
- Insulin/glucagon coupling (deferred to 3.4)
- Microbiome fermentation (deferred to 3.7)

### Tasks

1. [ ] Expand OrganismState with substrate compartments
2. [ ] Implement coarse substrate absorption/metabolism
3. [ ] Add fecal mass accumulator
4. [ ] Create `digestive.substrate-v1` MechanismContract
5. [ ] Find tier 3 postprandial glucose validation data
6. [ ] Document Neko obligate carnivore parameters
7. [ ] Write integration tests: glucose dynamics, conservation
8. [ ] Update phase3 plan with closure evidence

## Gate 3.4: Endocrine

### Minimal scope

**State (plasma concentrations):**
```rust
plasma_cortisol_nmol_per_l: i64,    // circadian 200-600 nmol/L
plasma_insulin_pmol_per_l: i64,     // fasting 30-150 pmol/L
plasma_adh_pmol_per_l: i64,         // linked to renal ADH modulation
```

**Mechanism:**
- Circadian cortisol release (existing circadian clock)
- Insulin response to glucose threshold (coarse step function)
- ADH osmolarity coupling (existing ADH_THRESHOLD in renal.rs)

**Validation tier 3:**
- Cortisol circadian: [Mackowiak 1992](https://jamanetwork.com/journals/jama/article-abstract/400116)
- Insulin-glucose: HOMA-IR published ranges

**Contract:**
- `endocrine.signaling-v1.json`
- Provenance: tier 3

**Non-goals:**
- HPA/HPG/HPT full axis
- Receptor occupancy
- Pulsatile release dynamics

### Tasks

1. [ ] Add hormone plasma concentrations to OrganismState
2. [ ] Implement circadian cortisol release
3. [ ] Implement glucose-insulin threshold coupling
4. [ ] Formalize ADH-osmolarity link (already implemented)
5. [ ] Create `endocrine.signaling-v1` contract
6. [ ] Find tier 3 circadian cortisol data
7. [ ] Write tests: cortisol circadian, insulin response
8. [ ] Update phase3 plan

## Gate 3.5: Thermoregulation Active

### Minimal scope

**State:**
```rust
skin_temperature_mk: i64,           // separate from core
sweat_rate_mg_per_s: i64,           // evaporative cooling
```

**Mechanism:**
- Active sweating above threshold (37.5°C core)
- Evaporative heat loss: 2260 J/g water
- Variable ambient conductance (existing framework)

**Validation tier 3:**
- Sweat onset threshold: published thermoregulation studies
- Evaporative heat: physical constant (measured)

**Contract:**
- `thermoregulation.active-v1.json`
- Provenance: tier 3 (constants measured, thresholds expert_estimate)

**Non-goals:**
- Vasoconstriction/dilation mechanics
- Shivering thermogenesis
- Multi-segment body model (Stolwijk)

### Tasks

1. [ ] Add skin temperature, sweat rate to OrganismState
2. [ ] Implement sweat threshold + evaporative cooling
3. [ ] Create `thermoregulation.active-v1` contract
4. [ ] Find tier 3 sweat onset data
5. [ ] Write tests: thermal stress, evaporative balance
6. [ ] Update phase3 plan

## Gate 3.6: Musculoskeletal/Fatigue

### Minimal scope

**State:**
```rust
muscle_lactate_mmol: i64,           // placeholder for fatigue
peripheral_fatigue_permille: i64,   // 0-1000, affects motor efficiency
```

**Mechanism:**
- Mechanical work → O2 demand (existing gas exchange)
- Efficiency constant 0.22 (published)
- Lactate accumulation on O2 deficit
- Fatigue threshold from lactate

**Validation tier 3:**
- Mechanical efficiency: ergometry studies
- VO2-power relationship: exercise physiology

**Contract:**
- `musculoskeletal.fatigue-v1.json`
- Provenance: tier 3

**Non-goals:**
- Motor unit populations
- Pain/nociception
- ATP/PCr compartments

### Tasks

1. [ ] Add lactate, fatigue to OrganismState
2. [ ] Implement work → O2 coupling
3. [ ] Implement lactate accumulation + fatigue
4. [ ] Create `musculoskeletal.fatigue-v1` contract
5. [ ] Find tier 3 mechanical efficiency data
6. [ ] Write tests: work-VO2 linearity, fatigue
7. [ ] Update phase3 plan

## Gate 3.7: Excretion/Microbiome

### Minimal scope

**State:**
```rust
gut_microbe_count_log10: i64,       // log10(CFU), coarse placeholder
skin_contamination_mg: i64,         // hygiene boundary
```

**Mechanism:**
- Fecal output from digestive (3.3 fecal_mass)
- Urine output from renal (existing)
- Microbe count logistic growth (coarse)
- Skin contamination accumulator (hygiene action clears)

**Validation tier 3:**
- Fecal water content: 60-85% (measured)
- Gut microbiome density: 10^11 CFU/g (16S studies)

**Contract:**
- `excretion.microbiome-v1.json`
- Provenance: tier 3

**Non-goals:**
- Individual microbe lineages
- SCFA production
- Detailed hygiene physics

### Tasks

1. [ ] Add microbe count, skin contamination to OrganismState
2. [ ] Link fecal output to digestive boundary
3. [ ] Implement microbe logistic growth
4. [ ] Create `excretion.microbiome-v1` contract
5. [ ] Find tier 3 microbiome density data
6. [ ] Write tests: fecal conservation, microbe growth
7. [ ] Update phase3 plan

## Cross-cutting Requirements

**All gates:**
- [ ] MechanismContract JSON with complete schema validation
- [ ] Tier 3 validation data extraction (JSON + provenance ledger)
- [ ] Conservation tests (exact i64 accounting)
- [ ] Determinism tests (1×N == N×1 hash parity)
- [ ] Replay tests (fast + audit)
- [ ] Update `ROADMAP.md` gate status
- [ ] Update `0006-phase3-everyday-physiology.md` closure sections

**Documentation:**
- [ ] Extract validation data → `docs/research/*-validation-data.json`
- [ ] Update Neko morphotype specification
- [ ] Document tier 3 provenance + upgrade paths
- [ ] Create final gate closure assessment

## Execution Plan

**Phase A: Gate 3.3 (Digestive) — 3-4 hours**
1. State expansion + minimal substrate tracking
2. Fecal boundary
3. Contract + tests
4. Validation data search
5. Neko obligate carnivore docs

**Phase B: Gates 3.4-3.7 (Parallel minimal implementations) — 4-5 hours**
1. 3.4 Endocrine: cortisol + insulin + ADH formalization
2. 3.5 Thermoregulation: sweat + evaporation
3. 6 Musculoskeletal: work-O2 + lactate
4. 3.7 Excretion: fecal link + microbe placeholder

**Phase C: Validation + Documentation — 2-3 hours**
1. Find all tier 3 validation datasets
2. Extract + document provenance
3. Update all gate status docs
4. Final assessment + commit

**Total estimated: 9-12 hours of focused work**

## Success Criteria

**Each gate:**
- ✅ MechanismContract created + validated
- ✅ Runtime integration tests passing
- ✅ Conservation verified
- ✅ Tier 3 validation data extracted + documented
- ✅ Provenance ledger complete
- ✅ Upgrade path documented

**Phase 3 overall:**
- ✅ All gates 3.1-3.7 CLOSED
- ✅ ROADMAP.md updated
- ✅ 0006-phase3-everyday-physiology.md closure sections complete
- ✅ Neko morphotype fully specified for everyday physiology
- ✅ Ready for Phase 4 gate commit

## Non-Goals (Deferred)

- Individual empirical trace validation (tier 2) — requires PhysioNet access
- Executable resolution upgrades (except where already implemented)
- Full hormonal axis coupling
- Detailed compartmental models
- N-worker canonical timeline (focus on correctness first)

## Rollback Plan

If validation data unavailable or implementation blocked:
- Document gaps explicitly
- Mark gates as "OPEN with documented prerequisites"
- Do NOT proceed to Phase 4
- User decision required for alternate acceptance criteria

---

**Next action**: Start Phase A — Gate 3.3 state expansion
