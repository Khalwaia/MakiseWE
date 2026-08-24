use makise_causal_kernel::{NeuralPopulation, NeuralPopulationError};

#[test]
fn population_construction_stores_neuron_count() {
    let population = NeuralPopulation::new(10_000);

    assert_eq!(population.neuron_count(), 10_000);
    assert_eq!(population.total_spike_count(), 0);
    assert_eq!(population.cumulative_spike_energy_uj(), 0);
}

#[test]
fn recording_spikes_adds_exact_counts_and_energy() {
    let mut population = NeuralPopulation::new(1_000);

    population
        .record_spikes(50, 1_000_000)
        .expect("valid spike batch");

    assert_eq!(population.total_spike_count(), 50);
    assert_eq!(population.cumulative_spike_energy_uj(), 50 * 1_000_000);

    population
        .record_spikes(25, 1_000_000)
        .expect("second valid batch");

    assert_eq!(population.total_spike_count(), 75);
    assert_eq!(population.cumulative_spike_energy_uj(), 75 * 1_000_000);
}

#[test]
fn negative_or_nonpositive_batch_is_typed_failure_without_mutation() {
    let mut population = NeuralPopulation::new(100);

    let error = population
        .record_spikes(-5, 1_000)
        .expect_err("negative spike count must be typed rejection");
    assert_eq!(error, NeuralPopulationError::InvalidSpikeBatch);

    let error = population
        .record_spikes(5, -1_000)
        .expect_err("negative energy must be typed rejection");
    assert_eq!(error, NeuralPopulationError::InvalidSpikeBatch);

    // State unchanged after rejections.
    assert_eq!(population.total_spike_count(), 0);
    assert_eq!(population.cumulative_spike_energy_uj(), 0);
}

#[test]
fn arithmetic_overflow_is_typed_failure_not_wraparound() {
    let mut population = NeuralPopulation::new(100);
    population
        .record_spikes(i64::MAX / 2, 2)
        .expect("first batch near limit");

    let error = population
        .record_spikes(i64::MAX / 2, 2)
        .expect_err("overflow must be typed failure");
    assert_eq!(error, NeuralPopulationError::Overflow);
}
