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

/// Sleep-debt seconds accumulated per awake canonical second. Physical
/// counter rate, not a score: one awake second adds exactly this many
/// seconds of missed-recovery debt.
pub const SLEEP_DEBT_PER_AWAKE_SECOND: i64 = 1;

/// Sleep-debt seconds cleared per asleep canonical second. The 2:1 ratio
/// is a coarse surrogate of the faster homeostatic decay during sleep:
/// eight asleep hours clear sixteen awake hours. Upgradeable via measured
/// mechanism artifacts (two-process model calibration).
pub const SLEEP_RECOVERY_PER_ASLEEP_SECOND: i64 = 2;

/// Sleep onset may trigger outside the night window once missed recovery
/// reaches this many seconds (12 h).
pub const SLEEP_DEBT_ONSET_THRESHOLD_SECONDS: i64 = 43_200;

/// Outcome of the deterministic circadian transition mechanism for one
/// canonical second.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepTransition {
    None,
    FallAsleep,
    WakeUp,
}

fn second_of_day(canonical_second: i64) -> i64 {
    canonical_second.rem_euclid(86_400)
}

fn in_night_window(second_of_day: i64) -> bool {
    // 22:00–06:00 canonical window.
    !(21_600..79_200).contains(&second_of_day)
}

fn in_morning_window(second_of_day: i64) -> bool {
    // 06:00–12:00 canonical window.
    (21_600..43_200).contains(&second_of_day)
}

/// Deterministic circadian transition rule. An accepted sleep intention
/// creates a condition for onset; the physiological trigger (night window
/// or sufficient sleep debt) decides whether and when the transition
/// actually happens. Waking requires cleared recovery debt inside the
/// morning window.
pub fn evaluate_sleep_transition(
    phase: SleepPhase,
    intention_accepted: bool,
    sleep_debt_seconds: i64,
    canonical_second: i64,
) -> SleepTransition {
    let day_second = second_of_day(canonical_second);
    match phase {
        SleepPhase::Awake
            if intention_accepted
                && (in_night_window(day_second)
                    || sleep_debt_seconds >= SLEEP_DEBT_ONSET_THRESHOLD_SECONDS) =>
        {
            SleepTransition::FallAsleep
        }
        SleepPhase::Asleep if sleep_debt_seconds <= 0 && in_morning_window(day_second) => {
            SleepTransition::WakeUp
        }
        _ => SleepTransition::None,
    }
}
