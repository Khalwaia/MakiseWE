# Phase 3 Provenance and Validation Status

## Quick Status

**Phase 3.1 (Cardiorespiratory): OPEN** ❌  
**Phase 3.2 (Renal): OPEN** ❌  
**Phase 3.3–3.7: NOT STARTED** ⚠️

## What Works (Runtime Integrity)

✅ **Causal verification**: conservation, determinism, replay  
✅ **Dimensional consistency**: explicit units, bounds checking  
✅ **Canonical timeline**: per-second transitions with archived dependencies  
✅ **Worker parity**: independent engines produce identical results  
✅ **SafeStop**: durable corruption detection without silent failures

## What Doesn't Work (Biological Realism)

❌ **Measured parameters**: all baselines are `expert_estimate` or `synthetic_fixture`  
❌ **Empirical validation**: no independent time-series admitted  
❌ **Uncertainty quantification**: no propagated uncertainty  
❌ **Calibration/holdout split**: no datasets available  
❌ **Fine mechanisms**: coarse single-rate models only

## ADR-0014 Realism Criteria

For a **realism claim**, all parameters must have provenance tier 1 or 2:

1. `measured` — published data with DOI/URL
2. `derived` — calculation from measured inputs
3. `expert_estimate` — named source estimate ← **Phase 3.1/3.2 are here**
4. `synthetic_fixture` — test-only value ← **Phase 3.1/3.2 mechanisms are here**

**Current Phase 3 status: tier 3/4 only → no realism claim permitted**

## Documentation

- **[phase3-provenance-status.md](phase3-provenance-status.md)**: Detailed assessment of Phase 3.1 and 3.2 parameters, validation evidence, and gate criteria
- **[renal-validation-evidence.md](renal-validation-evidence.md)**: Evidence ledger for renal datasets; none admitted yet
- **[biology-realism.md](biology-realism.md)**: General biological realism guidance
- **[ADR-0014](../adr/0014-fidelity-envelope-and-validation-evidence.md)**: Fidelity envelope and provenance requirements

## Terminology Guidance

### ✅ Permitted Terms

- "Causally verifiable"
- "Dimensionally consistent"
- "Synthetic baseline"
- "Expert estimate envelope"
- "Coarse mechanism"

### ❌ Prohibited Terms (without measured validation)

- "Realistic physiology"
- "Validated model"
- "Calibrated parameters"
- "Biological accuracy"
- "Clinical applicability"

## Next Steps for Gate Closure

1. **Find compatible datasets**: renal-only boundary inputs, healthy cohort, time-resolved outputs, individual traces
2. **Declare protocol**: question of interest, population, thresholds fixed before fitting
3. **Split calibration/holdout**: before parameter tuning
4. **Execute validation**: uncertainty-bounded comparison through public seams
5. **Implement resolution upgrade**: executable fine mechanisms through commit
6. **Complete dependency manifest**: scheduler/circadian/digestion/gas/metabolism archived contracts

See [phase3-provenance-status.md](phase3-provenance-status.md#6-следующие-шаги-для-закрытия-gates) for details.
