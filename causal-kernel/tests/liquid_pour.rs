//! Phase 2 slice 14 evidence: exact liquid accounting for pours,
//! overflows and floor puddles — the physical substrate of the "spill"
//! gate verb. Independent anchors are fixture geometry in cubic
//! millimetres (1 ml = 1000 mm³) and hand-derived depth quotients,
//! never the production code.

use makise_causal_kernel::{FluidError, LiquidContainer, PourRequest, puddle_depth_nm};

/// 300 ml poured from a full one-litre jug into an empty half-litre
/// bowl transfers completely: no spill, both sides conserved.
#[test]
fn clean_pour_transfers_exactly_without_spill() {
    let jug = LiquidContainer::new(1_000_000, 1_000_000).expect("valid jug");
    let bowl = LiquidContainer::new(500_000, 0).expect("valid bowl");
    let request = PourRequest::new(300_000).expect("positive request");

    let outcome = jug.pour_into(&bowl, &request).expect("valid pour");
    assert_eq!(outcome.transferred_mm3(), 300_000);
    assert_eq!(outcome.spilled_mm3(), 0);
    assert_eq!(outcome.next_source().content_mm3(), 700_000);
    assert_eq!(outcome.next_target().content_mm3(), 300_000);

    // Volume conservation across the outcome boundary.
    assert_eq!(
        jug.content_mm3() + bowl.content_mm3(),
        outcome.next_source().content_mm3()
            + outcome.next_target().content_mm3()
            + outcome.spilled_mm3()
    );
}

/// Pouring past the free space overflows: the bowl accepts exactly its
/// free 200 ml and the remaining 500 ml becomes spill — a first-class
/// outcome, not an error and not a silent clamp.
#[test]
fn overflow_beyond_free_space_spills_the_exact_remainder() {
    let jug = LiquidContainer::new(1_000_000, 800_000).expect("valid jug");
    let bowl = LiquidContainer::new(500_000, 300_000).expect("valid bowl");
    let request = PourRequest::new(700_000).expect("positive request");

    let outcome = jug.pour_into(&bowl, &request).expect("valid pour");
    assert_eq!(outcome.transferred_mm3(), 200_000);
    assert_eq!(outcome.spilled_mm3(), 500_000);
    assert_eq!(outcome.next_source().content_mm3(), 100_000);
    assert_eq!(outcome.next_target().content_mm3(), 500_000);
    assert!(outcome.next_target().is_full());

    // Total liquid is conserved bit-exactly.
    assert_eq!(
        jug.content_mm3() + bowl.content_mm3(),
        outcome.next_source().content_mm3()
            + outcome.next_target().content_mm3()
            + outcome.spilled_mm3()
    );
}

/// Requesting more than the source holds pours everything it has; the
/// request is a bound, never an invented amount.
#[test]
fn request_above_available_pours_only_what_is_there() {
    let cup = LiquidContainer::new(300_000, 50_000).expect("valid cup");
    let pot = LiquidContainer::new(2_000_000, 0).expect("valid pot");
    let request = PourRequest::new(300_000).expect("positive request");

    let outcome = cup.pour_into(&pot, &request).expect("valid pour");
    assert_eq!(outcome.transferred_mm3(), 50_000);
    assert_eq!(outcome.spilled_mm3(), 0);
    assert!(outcome.next_source().is_empty());
}

/// The exact-fill boundary: a request equal to the free space lands
/// the target precisely at capacity with zero spill.
#[test]
fn request_equal_to_free_space_fills_exactly() {
    let jug = LiquidContainer::new(1_000_000, 450_000).expect("valid jug");
    let bottle = LiquidContainer::new(400_000, 150_000).expect("valid bottle");
    let request = PourRequest::new(250_000).expect("positive request");

    let outcome = jug.pour_into(&bottle, &request).expect("valid pour");
    assert_eq!(outcome.transferred_mm3(), 250_000);
    assert_eq!(outcome.spilled_mm3(), 0);
    assert!(outcome.next_target().is_full());
}

/// Spill geometry: 250 ml spread over a 0.25 m² footprint stands
/// exactly one millimetre deep (V/A = 250 000 mm³ / 250 000 mm²).
#[test]
fn puddle_depth_is_the_exact_volume_over_footprint() {
    let depth = puddle_depth_nm(250_000, 250_000).expect("representable quotient");
    assert_eq!(depth, 1_000_000); // 1 mm in nm
}

/// A quotient that leaves whole nanometres is typed-rejected instead
/// of silently rounding the puddle surface.
#[test]
fn fractional_puddle_depth_is_typed() {
    let error =
        puddle_depth_nm(1, 3).expect_err("one cubic millimetre over three cannot stand flat");
    assert!(matches!(error, FluidError::NonRepresentableDepth));
}

/// Degenerate declarations are rejected at the boundary: non-positive
/// capacity, content outside capacity, negative or zero requests,
/// pouring a container into itself.
#[test]
fn degenerate_containers_and_requests_are_typed() {
    assert!(matches!(
        LiquidContainer::new(0, 0),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        LiquidContainer::new(100, 101),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        LiquidContainer::new(-5, 0),
        Err(FluidError::InvalidParameters)
    ));

    let jug = LiquidContainer::new(1_000, 500).expect("valid jug");
    assert!(matches!(
        PourRequest::new(0),
        Err(FluidError::InvalidParameters)
    ));
    assert!(matches!(
        jug.pour_into(&jug, &PourRequest::new(10).expect("valid")),
        Err(FluidError::InvalidParameters)
    ));
}

/// Accounting is a pure function of its inputs: identical pours yield
/// identical outcomes bit for bit.
#[test]
fn pouring_is_deterministic_under_repetition() {
    let jug_a = LiquidContainer::new(1_000_000, 800_000).expect("valid jug");
    let jug_b = LiquidContainer::new(1_000_000, 800_000).expect("valid jug");
    let bowl_a = LiquidContainer::new(500_000, 300_000).expect("valid bowl");
    let bowl_b = LiquidContainer::new(500_000, 300_000).expect("valid bowl");
    let request = PourRequest::new(700_000).expect("valid request");

    let first = jug_a.pour_into(&bowl_a, &request).expect("valid pour");
    let second = jug_b.pour_into(&bowl_b, &request).expect("valid pour");
    assert_eq!(first, second);
}
