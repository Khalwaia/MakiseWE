use thiserror::Error;

/// Aggregate of neurons with signal statistics. Spike counts and energy
/// are physical counters, not normalized scores. Every recorded spike
/// batch adds exactly its declared values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NeuralPopulation {
    neuron_count: i64,
    total_spike_count: i64,
    cumulative_spike_energy_uj: i64,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum NeuralPopulationError {
    #[error("spike count and energy must be non-negative")]
    InvalidSpikeBatch,
    #[error("checked arithmetic overflow in neural statistics")]
    Overflow,
}

impl NeuralPopulation {
    pub fn new(neuron_count: i64) -> Self {
        Self {
            neuron_count,
            total_spike_count: 0,
            cumulative_spike_energy_uj: 0,
        }
    }

    pub fn neuron_count(&self) -> i64 {
        self.neuron_count
    }

    pub fn total_spike_count(&self) -> i64 {
        self.total_spike_count
    }

    pub fn cumulative_spike_energy_uj(&self) -> i64 {
        self.cumulative_spike_energy_uj
    }

    /// Records one batch of spikes with their metabolic energy cost.
    /// Exact integer accounting; no tolerance, no clamping.
    pub fn record_spikes(
        &mut self,
        spike_count: i64,
        energy_per_spike_uj: i64,
    ) -> Result<(), NeuralPopulationError> {
        if spike_count < 0 || energy_per_spike_uj < 0 {
            return Err(NeuralPopulationError::InvalidSpikeBatch);
        }
        let batch_energy = spike_count
            .checked_mul(energy_per_spike_uj)
            .ok_or(NeuralPopulationError::Overflow)?;
        self.total_spike_count = self
            .total_spike_count
            .checked_add(spike_count)
            .ok_or(NeuralPopulationError::Overflow)?;
        self.cumulative_spike_energy_uj = self
            .cumulative_spike_energy_uj
            .checked_add(batch_energy)
            .ok_or(NeuralPopulationError::Overflow)?;
        Ok(())
    }
}
