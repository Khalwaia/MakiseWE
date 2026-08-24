use crate::circadian::SleepPhase;
use crate::organism::OrganismState;

pub const INITIAL_CHEMICAL_STORE_UJ: i64 = 8_400_000_000_000;

/// Sleep debt in canonical seconds of missed recovery. Physical counter,
/// not a normalized score: one awake second adds exactly one second of
/// debt, one asleep second removes exactly one (floored at zero).
pub fn advance_sleep_debt(current_seconds: i64, phase: SleepPhase) -> i64 {
    match phase {
        SleepPhase::Awake => current_seconds + 1,
        SleepPhase::Asleep => (current_seconds - 1).max(0),
    }
}

/// Non-authoritative projections for cognition/UI. Each observable keeps a
/// link to its source quantity; nothing here mutates state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InteroceptionObservables {
    chemical_store_uj: i64,
    chemical_capacity_uj: i64,
    sleep_debt_seconds: i64,
    sleep_phase: SleepPhase,
}

impl InteroceptionObservables {
    pub fn of(
        chemical_store_uj: i64,
        chemical_capacity_uj: i64,
        sleep_debt_seconds: i64,
        sleep_phase: SleepPhase,
    ) -> Self {
        Self {
            chemical_store_uj,
            chemical_capacity_uj,
            sleep_debt_seconds,
            sleep_phase,
        }
    }

    pub fn chemical_store_uj(&self) -> i64 {
        self.chemical_store_uj
    }

    pub fn sleep_debt_seconds(&self) -> i64 {
        self.sleep_debt_seconds
    }

    /// Deficit fraction against full store, in permille of capacity.
    pub fn hunger_fraction_permille(&self) -> i64 {
        let deficit = (self.chemical_capacity_uj - self.chemical_store_uj).max(0);
        deficit * 1_000 / self.chemical_capacity_uj
    }

    /// Debt fraction against a declared reference day of debt, permille.
    const FATIGUE_REFERENCE_SECONDS: i64 = 57_600; // 16 h of wake debt

    pub fn fatigue_fraction_permille(&self) -> i64 {
        self.sleep_debt_seconds * 1_000 / Self::FATIGUE_REFERENCE_SECONDS
    }

    pub(crate) fn from_state(
        organism: &OrganismState,
        sleep_debt_seconds: i64,
        sleep_phase: SleepPhase,
    ) -> Self {
        Self::of(
            organism.chemical_store_uj(),
            INITIAL_CHEMICAL_STORE_UJ,
            sleep_debt_seconds,
            sleep_phase,
        )
    }
}
