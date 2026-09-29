use crate::organism::OrganismState;

/// Absorption flux from the digestive buffer into the chemical store, in
/// microjoules per canonical second.
///
/// Provenance: `expert_estimate`. A mixed meal completes absorption over
/// roughly 3–5 hours; this rate moves a standard 478 kcal meal in about
/// four hours. Upgradeable via mechanism artifacts carrying measured
/// time-series calibration.
pub const ABSORPTION_RATE_UJ_PER_SECOND: i64 = 140_000_000;

/// Fraction of absorbed energy that becomes glucose (remainder to generic store).
/// Provenance: `expert_estimate`. Mixed meal ~40% carbohydrate energy.
const GLUCOSE_FRACTION: i64 = 400; // numerator for /1000

/// Glucose energy content: 15.6 kJ/g × 180 mg/mmol = 2808 µJ/µmol = 2_808_000 µJ/mmol
const GLUCOSE_ENERGY_UJ_PER_MMOL: i64 = 2_808_000;

/// Liver glycogen buffer capacity: ~100g = ~555 mmol glucose equivalents
const LIVER_GLYCOGEN_CAPACITY_MMOL: i64 = 555_000;

/// Baseline plasma glucose: 5.0 mmol/L × 3L = 15 mmol
const BASELINE_PLASMA_GLUCOSE_MMOL: i64 = 15_000;

/// Glycogen synthesis threshold: plasma glucose > 6 mmol/L
const GLYCOGEN_SYNTHESIS_THRESHOLD_MMOL_PER_L: i64 = 6_000;

/// Glycogen breakdown threshold: plasma glucose < 4 mmol/L
const GLYCOGEN_BREAKDOWN_THRESHOLD_MMOL_PER_L: i64 = 4_000;

/// Liver glucose buffering flux: 1 mmol/s when outside thresholds
const LIVER_FLUX_MMOL_PER_SECOND: i64 = 1_000;

/// Fecal dry mass fraction of ingested energy (as mass proxy)
/// Provenance: `expert_estimate`. ~10% of intake mass becomes feces.
const FECAL_FRACTION_MILLI: i64 = 100; // /1000

/// Moves at most one canonical second of declared absorption flux from
/// the digestive buffer into the chemical store. Exact integer
/// accounting; the transfer never exceeds the buffered amount.
pub fn absorb_one_second(organism: &mut OrganismState) {
    let flux = organism
        .digestion_buffer_uj()
        .min(ABSORPTION_RATE_UJ_PER_SECOND);
    if flux > 0 {
        // All energy goes to chemical store for now (substrate partitioning
        // is a coarse approximation that doesn't change total energy balance)
        organism.absorb_chemical_energy(flux);
        organism.consume_digestion_buffer(flux);

        // Glucose tracking (parallel accounting, doesn't affect energy totals)
        let glucose_energy = flux.saturating_mul(GLUCOSE_FRACTION) / 1000;
        let glucose_mmol = glucose_energy / GLUCOSE_ENERGY_UJ_PER_MMOL;
        organism.absorb_glucose(glucose_mmol);

        // Fecal accumulation (coarse mass proxy)
        let fecal_mg = (flux / 1000).saturating_mul(FECAL_FRACTION_MILLI) / 1000;
        organism.accumulate_fecal_mass(fecal_mg);
    }
}

/// Liver glucose buffering: glycogen synthesis when glucose high, breakdown when low.
pub fn liver_buffer_one_second(organism: &mut OrganismState) {
    let plasma_mm3 = organism.renal().plasma_mm3();
    let glucose_concentration = if plasma_mm3 > 0 {
        organism.plasma_glucose_mmol().saturating_mul(1_000_000) / plasma_mm3
    } else {
        BASELINE_PLASMA_GLUCOSE_MMOL.saturating_mul(1_000_000) / 3_000_000 // fallback 3L
    };

    if glucose_concentration > GLYCOGEN_SYNTHESIS_THRESHOLD_MMOL_PER_L {
        // Store glucose as glycogen
        let available = organism.plasma_glucose_mmol();
        let capacity_headroom = LIVER_GLYCOGEN_CAPACITY_MMOL.saturating_sub(organism.liver_glycogen_mmol());
        let flux = LIVER_FLUX_MMOL_PER_SECOND.min(available).min(capacity_headroom);
        if flux > 0 {
            organism.consume_plasma_glucose(flux);
            organism.synthesize_glycogen(flux);
        }
    } else if glucose_concentration < GLYCOGEN_BREAKDOWN_THRESHOLD_MMOL_PER_L {
        // Release glucose from glycogen
        let available = organism.liver_glycogen_mmol();
        let flux = LIVER_FLUX_MMOL_PER_SECOND.min(available);
        if flux > 0 {
            organism.breakdown_glycogen(flux);
            organism.absorb_glucose(flux);
        }
    }
}

/// Metabolic consumption: burn plasma glucose first, then generic chemical store.
/// Returns actual energy consumed from glucose (caller must burn remainder from store).
pub fn consume_glucose_for_metabolism(organism: &mut OrganismState, demand_uj: i64) -> i64 {
    let glucose_available = organism.plasma_glucose_mmol();
    let glucose_energy = glucose_available.saturating_mul(GLUCOSE_ENERGY_UJ_PER_MMOL);

    if glucose_energy >= demand_uj {
        // Sufficient glucose to cover full demand
        let glucose_needed = (demand_uj + GLUCOSE_ENERGY_UJ_PER_MMOL - 1) / GLUCOSE_ENERGY_UJ_PER_MMOL;
        organism.consume_plasma_glucose(glucose_needed);
        demand_uj
    } else {
        // Burn all available glucose, caller burns remainder from chemical store
        organism.consume_plasma_glucose(glucose_available);
        glucose_energy
    }
}
