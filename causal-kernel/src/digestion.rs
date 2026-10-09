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

/// Fasting plasma glucose target maintained by hepatic glucose output.
/// Provenance: tier_3, ADA fasting reference band midpoint (5.0 mmol/L).
const FASTING_GLUCOSE_TARGET_MMOL_PER_L: i64 = 5_000;

/// Liver glycogen buffer capacity: ~100g = ~555 mmol glucose equivalents
const LIVER_GLYCOGEN_CAPACITY_MMOL: i64 = 555_000;

/// Glycogen synthesis threshold: plasma glucose > 6 mmol/L
const GLYCOGEN_SYNTHESIS_THRESHOLD_MMOL_PER_L: i64 = 6_000;

/// Maximum hepatic glucose output per second toward the fasting target.
/// Provenance: tier_3_derived, ~0.5 mmol/min baseline hepatic output.
const HEPATIC_OUTPUT_MMOL_PER_SECOND: i64 = 1_000;

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

/// Liver glucose buffering: store surplus as glycogen above the synthesis
/// threshold, and release glycogen to hold plasma glucose near the fasting
/// target when it falls below. Combined with basal uptake this reproduces
/// the contract's fasting steady state and postprandial normalization.
pub fn liver_buffer_one_second(organism: &mut OrganismState) {
    let plasma_mm3 = organism.renal().plasma_mm3();
    let glucose_concentration = if plasma_mm3 > 0 {
        organism.plasma_glucose_mmol().saturating_mul(1_000_000) / plasma_mm3
    } else {
        FASTING_GLUCOSE_TARGET_MMOL_PER_L
    };

    if glucose_concentration > GLYCOGEN_SYNTHESIS_THRESHOLD_MMOL_PER_L {
        // Store surplus glucose as glycogen.
        let available = organism.plasma_glucose_mmol();
        let capacity_headroom =
            LIVER_GLYCOGEN_CAPACITY_MMOL.saturating_sub(organism.liver_glycogen_mmol());
        let flux = HEPATIC_OUTPUT_MMOL_PER_SECOND
            .min(available)
            .min(capacity_headroom);
        if flux > 0 {
            organism.consume_plasma_glucose(flux);
            organism.synthesize_glycogen(flux);
        }
    } else if glucose_concentration < FASTING_GLUCOSE_TARGET_MMOL_PER_L {
        // Hepatic glucose output restores plasma glucose toward the fasting
        // target from the glycogen reserve. Exact integer accounting.
        let available = organism.liver_glycogen_mmol();
        let deficit = target_glucose_mmol(plasma_mm3)
            .saturating_sub(organism.plasma_glucose_mmol())
            .max(0);
        let flux = HEPATIC_OUTPUT_MMOL_PER_SECOND.min(deficit).min(available);
        if flux > 0 {
            organism.breakdown_glycogen(flux);
            organism.absorb_glucose(flux);
        }
    }
}

/// Plasma glucose quantity that realizes the fasting target concentration at
/// the current plasma volume.
fn target_glucose_mmol(plasma_mm3: i64) -> i64 {
    FASTING_GLUCOSE_TARGET_MMOL_PER_L
        .saturating_mul(plasma_mm3)
        .saturating_div(1_000_000)
}

/// Basal peripheral glucose uptake rate from contract: ~0.5 mmol/min baseline.
/// Provenance: tier_3_derived, matches hepatic release for fasting steady state.
const BASAL_GLUCOSE_UPTAKE_MMOL_PER_SECOND: i64 = 8; // 0.008 mmol/s in millimolar precision

/// Metabolic consumption: burn basal glucose uptake from plasma, rest from chemical store.
/// Returns actual energy consumed from glucose (caller must burn remainder from store).
/// Contract specifies only ~0.0083 mmol/s glucose uptake; remainder is fat oxidation proxy.
pub fn consume_glucose_for_metabolism(organism: &mut OrganismState, _demand_uj: i64) -> i64 {
    let glucose_available = organism.plasma_glucose_mmol();
    let glucose_consumed = BASAL_GLUCOSE_UPTAKE_MMOL_PER_SECOND.min(glucose_available);

    if glucose_consumed > 0 {
        organism.consume_plasma_glucose(glucose_consumed);
        glucose_consumed.saturating_mul(GLUCOSE_ENERGY_UJ_PER_MMOL)
    } else {
        0
    }
}
