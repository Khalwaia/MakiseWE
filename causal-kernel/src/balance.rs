//! Exact balance feedback over a declared support base.
//!
//! The walker's stability is assessed against the floor-plane (xz)
//! projection of its centre of mass and the support contacts produced
//! by the contact slice. Validity range: bipedal stances only — zero,
//! one, or two simultaneous support contacts. One contact is a point
//! base; two contacts form the segment between them. A projection on
//! the base means `Stable`, anything else means `Tipping`; with no
//! contacts the body is `Airborne`. Every verdict is an exact integer
//! comparison — no epsilon, no heuristic margin (INVARIANTS §18).

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum BalanceError {
    #[error(
        "more than two simultaneous support contacts exceed this slice's declared bipedal validity range"
    )]
    OutsideValidityRange,
}

/// Closed-world verdict of one assessment: airborne is a legitimate
/// physical state distinct from tipping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceState {
    Airborne,
    Stable,
    Tipping,
}

/// Observable outcome of one balance assessment: the state plus the
/// exact signed area anchor that decided the segment cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalanceAssessment {
    state: BalanceState,
    signed_area_doubled_nm2: i128,
}

impl BalanceAssessment {
    pub fn state(&self) -> BalanceState {
        self.state
    }

    /// Twice the signed area of the triangle (foot A, foot B, COM
    /// projection) in nm². Zero exactly when the projection lies on the
    /// foot line; singleton and airborne bases report 0.
    pub fn signed_area_doubled_nm2(&self) -> i128 {
        self.signed_area_doubled_nm2
    }
}

/// Assesses balance from the declared floor contacts and the centre-of
/// mass projection on the same plane, both in nanometres.
///
/// Structural geometry only: no material parameters, no provenance
/// beyond the fixture coordinates themselves. Pure function of inputs;
/// repeated evaluation over identical states yields identical verdicts.
pub fn balance_assessment(
    support_contacts_xz_nm: &[[i64; 2]],
    centre_of_mass_projection_xz_nm: [i64; 2],
) -> Result<BalanceAssessment, BalanceError> {
    if support_contacts_xz_nm.len() > 2 {
        return Err(BalanceError::OutsideValidityRange);
    }
    let [px, pz] = centre_of_mass_projection_xz_nm;
    match support_contacts_xz_nm {
        [] => Ok(BalanceAssessment {
            state: BalanceState::Airborne,
            signed_area_doubled_nm2: 0,
        }),
        [only] => {
            let stable = only[0] == px && only[1] == pz;
            Ok(BalanceAssessment {
                state: if stable {
                    BalanceState::Stable
                } else {
                    BalanceState::Tipping
                },
                signed_area_doubled_nm2: 0,
            })
        }
        [a, b] => {
            let ax = i128::from(a[0]);
            let az = i128::from(a[1]);
            let bx = i128::from(b[0]);
            let bz = i128::from(b[1]);
            let px = i128::from(px);
            let pz = i128::from(pz);
            // Cross product (B−A)×(P−A): zero iff P lies on line AB.
            // Only astronomically out-of-range floor coordinates could
            // overflow here; that input is outside the declared
            // validity range and rejected instead of wrapping.
            let along = (bx - ax)
                .checked_mul(pz - az)
                .ok_or(BalanceError::OutsideValidityRange)?;
            let across = (bz - az)
                .checked_mul(px - ax)
                .ok_or(BalanceError::OutsideValidityRange)?;
            let cross = along
                .checked_sub(across)
                .ok_or(BalanceError::OutsideValidityRange)?;
            let within_x = px >= ax.min(bx) && px <= ax.max(bx);
            let within_z = pz >= az.min(bz) && pz <= az.max(bz);
            let stable = cross == 0 && within_x && within_z;
            Ok(BalanceAssessment {
                state: if stable {
                    BalanceState::Stable
                } else {
                    BalanceState::Tipping
                },
                signed_area_doubled_nm2: cross,
            })
        }
        // Lengths above two were rejected before the match; the arm
        // stays so the match over slices is statically exhaustive.
        [_, _, _, ..] => Err(BalanceError::OutsideValidityRange),
    }
}
