/// Sleep phase as authoritative state, not a timer or normalized score.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepPhase {
    Awake,
    Asleep,
}

impl SleepPhase {
    fn canonical_name(self) -> &'static str {
        match self {
            Self::Awake => "awake",
            Self::Asleep => "asleep",
        }
    }

    pub fn from_canonical_name(name: &str) -> Option<Self> {
        match name {
            "awake" => Some(Self::Awake),
            "asleep" => Some(Self::Asleep),
            _ => None,
        }
    }

    pub fn as_canonical_name(self) -> &'static str {
        self.canonical_name()
    }
}

/// Metabolic chemical demand in microjoules per canonical second.
/// Values are declared constants with provenance `expert_estimate`
/// until calibrated against reference data; the ratio is the causal
/// claim under test (sleep lowers demand), exact magnitudes are
/// upgradeable via mechanism artifacts.
pub const AWAKE_METABOLISM_UJ_PER_SECOND: i64 = 1_200_000;
pub const ASLEEP_METABOLISM_UJ_PER_SECOND: i64 = 800_000;

/// Canonical circadian modulation of awake demand: night seconds 0..21600
/// (00:00–06:00) get a lower rate than daytime. Sleep phase still dominates.
pub const NIGHT_AWAKE_METABOLISM_UJ_PER_SECOND: i64 = 1_000_000;

pub const INITIAL_CHEMICAL_STORE_UJ: i64 = 8_400_000_000_000;

pub fn metabolic_demand_uj_per_second(phase: SleepPhase) -> i64 {
    match phase {
        SleepPhase::Awake => AWAKE_METABOLISM_UJ_PER_SECOND,
        SleepPhase::Asleep => ASLEEP_METABOLISM_UJ_PER_SECOND,
    }
}

pub fn awake_metabolism_for_second(canonical_second: i64) -> i64 {
    let second_of_day = canonical_second.rem_euclid(86_400);
    if second_of_day < 21_600 {
        NIGHT_AWAKE_METABOLISM_UJ_PER_SECOND
    } else {
        AWAKE_METABOLISM_UJ_PER_SECOND
    }
}
