use crate::organism::OrganismState;

/// Absorption flux from the digestive buffer into the chemical store, in
/// microjoules per canonical second.
///
/// Provenance: `expert_estimate`. A mixed meal completes absorption over
/// roughly 3–5 hours; this rate moves a standard 478 kcal meal in about
/// four hours. Upgradeable via mechanism artifacts carrying measured
/// time-series calibration.
pub const ABSORPTION_RATE_UJ_PER_SECOND: i64 = 140_000_000;

/// Moves at most one canonical second of declared absorption flux from
/// the digestive buffer into the chemical store. Exact integer
/// accounting; the transfer never exceeds the buffered amount.
pub fn absorb_one_second(organism: &mut OrganismState) {
    let flux = organism
        .digestion_buffer_uj()
        .min(ABSORPTION_RATE_UJ_PER_SECOND);
    if flux > 0 {
        organism.absorb_chemical_energy(flux);
        organism.consume_digestion_buffer(flux);
    }
}
