/// Declared morphotype parameters. Data, not behavior: each value has units,
/// provenance `expert_estimate`, and can be replaced via mechanism artifacts
/// without changing code paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Morphotype {
    awake_metabolism_uj_per_second: i64,
    asleep_metabolism_uj_per_second: i64,
    night_awake_metabolism_uj_per_second: i64,
    core_heat_capacity_uj_per_mk: i64,
    ambient_conductance_uj_per_mk_s: i64,
}

impl Morphotype {
    pub const fn new(
        awake_metabolism_uj_per_second: i64,
        asleep_metabolism_uj_per_second: i64,
        night_awake_metabolism_uj_per_second: i64,
        core_heat_capacity_uj_per_mk: i64,
        ambient_conductance_uj_per_mk_s: i64,
    ) -> Self {
        Self {
            awake_metabolism_uj_per_second,
            asleep_metabolism_uj_per_second,
            night_awake_metabolism_uj_per_second,
            core_heat_capacity_uj_per_mk,
            ambient_conductance_uj_per_mk_s,
        }
    }

    /// Baseline human parameters matching the current hardcoded constants.
    pub fn human() -> Self {
        Self::new(1_200_000, 800_000, 1_000_000, 4_000, 50)
    }

    /// Neko: smaller body mass, fur insulation, slightly different metabolism.
    pub fn neko() -> Self {
        Self::new(960_000, 640_000, 800_000, 2_000, 25)
    }

    pub fn awake_metabolism_uj_per_second(&self) -> i64 {
        self.awake_metabolism_uj_per_second
    }

    pub fn asleep_metabolism_uj_per_second(&self) -> i64 {
        self.asleep_metabolism_uj_per_second
    }

    pub fn night_awake_metabolism_uj_per_second(&self) -> i64 {
        self.night_awake_metabolism_uj_per_second
    }

    pub fn core_heat_capacity_uj_per_mk(&self) -> i64 {
        self.core_heat_capacity_uj_per_mk
    }

    pub fn ambient_conductance_uj_per_mk_s(&self) -> i64 {
        self.ambient_conductance_uj_per_mk_s
    }
}
