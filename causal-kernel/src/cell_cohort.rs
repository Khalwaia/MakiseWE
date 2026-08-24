/// Aggregate of compatible cells holding conserved quantities.
/// Coarse state is authoritative; lift produces a deterministic fine
/// representation that conserves every declared total exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellCohort {
    cell_count: i64,
    total_mass_ng: i64,
    total_charge_nano: i64,
}

/// Single cell in the fine (lifted) representation. Quantities use
/// integer units to guarantee exact conservation without rounding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FineCell {
    pub mass_ng: i64,
    pub charge_nano: i64,
}

impl CellCohort {
    pub fn new(cell_count: i64, total_mass_ng: i64, total_charge_nano: i64) -> Self {
        Self {
            cell_count,
            total_mass_ng,
            total_charge_nano,
        }
    }

    pub fn cell_count(&self) -> i64 {
        self.cell_count
    }

    pub fn total_mass_ng(&self) -> i64 {
        self.total_mass_ng
    }

    pub fn total_charge_nano(&self) -> i64 {
        self.total_charge_nano
    }

    /// Deterministic seeded lift from coarse aggregate to per-cell values.
    /// Remainder is distributed one unit at a time to the first cells,
    /// preserving exact totals. Same input always yields same output.
    pub fn lift_to_cells(&self) -> Vec<FineCell> {
        if self.cell_count <= 0 {
            return Vec::new();
        }
        let mass_base = self.total_mass_ng / self.cell_count;
        let mass_remainder = self.total_mass_ng % self.cell_count;
        let charge_base = self.total_charge_nano / self.cell_count;
        let charge_remainder = self.total_charge_nano % self.cell_count;

        (0..self.cell_count)
            .map(|index| FineCell {
                mass_ng: mass_base + i64::from(index < mass_remainder),
                charge_nano: charge_base + i64::from(index < charge_remainder),
            })
            .collect()
    }

    /// Projection back from fine cells into coarse aggregate. Totals are
    /// summed exactly; no information loss for conserved quantities.
    pub fn from_fine_cells(cells: &[FineCell], declared_count: i64) -> Self {
        let total_mass_ng: i64 = cells.iter().map(|cell| cell.mass_ng).sum();
        let total_charge_nano: i64 = cells.iter().map(|cell| cell.charge_nano).sum();
        Self {
            cell_count: declared_count.max(0),
            total_mass_ng,
            total_charge_nano,
        }
    }
}
