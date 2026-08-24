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
///
/// Provenance: `expert_estimate` anchored to published human physiology
/// (Weir 1949 gas-exchange method; typical adult female daily energy
/// expenditure of roughly 1500–2200 kcal/day), as summarized in
/// docs/research/biology-realism.md. The causal claim under test is the
/// ordering sleep < night-awake < day-awake; exact magnitudes are
/// upgradeable via mechanism artifacts carrying measured provenance.
pub const AWAKE_METABOLISM_UJ_PER_SECOND: i64 = 95_000_000;
pub const ASLEEP_METABOLISM_UJ_PER_SECOND: i64 = 75_000_000;

/// Canonical circadian modulation of awake demand: night seconds 0..21600
/// (00:00–06:00) get a lower rate than daytime. Sleep phase still dominates.
pub const NIGHT_AWAKE_METABOLISM_UJ_PER_SECOND: i64 = 88_000_000;

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
