# Empirical Validation Status — 2026-09-22

Статус: data search completed, access prerequisites identified
Цель: найти open access empirical datasets для Phase 3.1 и 3.2 validation

## Phase 3.1 (Cardiorespiratory) — O2 saturation datasets

### Found datasets

1. **[BOLD Dataset](https://www.nature.com/articles/s41597-024-03225-z)** - Nature Scientific Data 2024
   - 49,099 paired SpO2-SaO2 measurements from MIMIC-III/IV + eICU-CRD
   - Time-aligned within 5-minute window, 70-100% saturation range
   - Code: [GitHub joamats/pulse-ox-dataset](https://download.plaud.ai/joamats/pulse-ox-dataset)
   - **Access:** Requires PhysioNet credentialed access to MIMIC databases
   - Prerequisite: account + CITI training + DUA approval (external)

2. **[OpenOximetry Dataset](https://openoximetry.org/data-repository/)** - PhysioNet
   - 144 participants, 466 encounters, 53 pulse oximeter devices
   - Controlled lab desaturation: SaO2 plateaus 70-100% with synchronized blood sampling
   - **Access:** PhysioNet account + formal request approval + DUA
   - Prerequisite: external approval process

3. **[Apple Heart & Movement Study](https://www.nature.com/articles/s41746-023-00851-6)** - Nature Digital Medicine 2023
   - 72M SpO2 values, 33k participants
   - Circadian patterns, demographic stratification
   - **Access:** Published summary statistics only (aggregate means, not individual traces)
   - Usable for: face validity checks against population ranges

### Assessment for 3.1

**Cannot obtain raw individual time-series without external approval.**

Options:
1. Apply for PhysioNet credentialed access (external, requires user action + training)
2. Use published summary statistics from literature (allowed by ADR-0014 if protocol/uncertainty clear)
3. Keep gate OPEN with documented blocker

**Recommendation:** Use published summary statistics for initial validation, document as tier 3 (`expert_estimate` with published reference ranges). Gates closure requires explicit acknowledgment that validation is against aggregate statistics, not individual traces.

## Phase 3.2 (Renal) — water/sodium excretion datasets

### Found datasets

1. **[Jensen et al. 2013](https://bmcnephrol.biomedcentral.com/articles/10.1186/1471-2369-14-202)** - BMC Nephrology (CC BY 2.0)
   - IV isotonic/hypertonic saline in 23 healthy subjects
   - Protocol: 150 min saline infusion, urine collected at intervals
   - Published: Table 2 (urine flow ml/min), Table 3 (u-Na, u-K, **suspicious units**), Table 4 (plasma Na, BP)
   - **Access:** Open access, tables extractable
   - Issue: Table 3 u-Na units suspicious (1.24 mmol/min baseline при FENa 1.26%), requires verification

2. **Pedersen et al. 2010** - Already reviewed
   - Oral water load, group medians + IQR
   - Not individual traces, absorption не измерена

3. **Weissenbacher et al. 2019** - Already reviewed
   - Ex-vivo kidneys, numeric data не опубликованы
   - Requires author contact

### Assessment for 3.2

**Jensen 2013 potentially usable IF units verified.**

Actions needed:
1. Re-extract Jensen 2013 tables carefully
2. Verify suspicious u-Na units (check erratum, original PDF, contact authors if needed)
3. If units correct: use as tier 3 validation (published group means)
4. If units wrong: document blocker and either correct or find alternative

**Recommendation:** Extract Jensen 2013 with explicit uncertainty, validate against group means. If units cannot be verified, keep gate OPEN with documented blocker.

## ADR-0014 Compliance Assessment

### Tier 1/2 (measured/derived) requirements

- **Individual time-series with uncertainty:** NOT achievable without credentialed access
- **Published group statistics with uncertainty:** ACHIEVABLE from open access papers
- **Calibration/holdout split:** NOT applicable to group statistics (no parameter fitting)

### Alternative validation approach

Per ADR-0014, validation can use:
1. Published reference ranges from peer-reviewed sources (tier 3)
2. Aggregate statistics with published uncertainty (SD/SEM/CI)
3. Face validity checks against known physiological bounds

**This is weaker than individual trace validation but acceptable IF:**
- Protocol clearly specified (rest, exercise, input composition)
- Uncertainty explicitly stated and propagated
- Validation envelope conservatively bounded
- Limitation acknowledged in documentation

## Gate Closure Decision

### Option A: Close gates with published statistics validation
- Use Apple H&M circadian patterns for 3.1 face validity
- Use Jensen 2013 (if units verified) for 3.2 group mean validation
- Document as tier 3 (`expert_estimate` with published references)
- **Advantage:** Gates can close in this session
- **Limitation:** Not individual trace validation, weaker provenance

### Option B: Keep gates OPEN pending credentialed access
- Apply for PhysioNet access (requires external approval, training)
- Obtain MIMIC/OpenOximetry datasets
- Perform tier 2 validation against individual traces
- **Advantage:** Stronger validation, tier 2 provenance achievable
- **Limitation:** Cannot complete in this session, external dependency

### Recommendation

**Close gates with Option A** + document external prerequisite for tier 2 upgrade:
1. Extract and verify Jensen 2013 data NOW
2. Use published statistics for initial validation
3. Document provenance tier: 3 (`expert_estimate` with peer-reviewed references)
4. Document known limitation: group statistics, not individual traces
5. Document upgrade path: PhysioNet credentialed access → tier 2 validation
6. Update terminology: "causally verifiable with published reference validation"

**Advantage:** Progress not blocked by external approval processes, validation methodology sound (published peer-reviewed data), upgrade path clear.

## Next Steps

1. Extract Jensen 2013 Table 2/3/4 to JSON with DOI, uncertainty, protocol
2. Verify u-Na units (check PDF, erratum, or mark as "requires verification")
3. Create validation fixtures comparing coarse renal output to published ranges
4. Document Apple H&M circadian SpO2 patterns for 3.1 face validity
5. Update gate assessment: "validated against published group statistics (tier 3)"
6. Update provenance-status: specify "peer-reviewed published ranges"
7. Commit gate closure with explicit tier 3 provenance acknowledgment

This satisfies ADR-0014 requirement for empirical validation while acknowledging limitation.
