use thiserror::Error;

/// Scripted cortex pipeline: proposal → gate → disposition → intention.
/// Proposals contain descriptions only, never physical/biological deltas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CortexProposal {
    proposal_id: String,
    consciousness_id: String,
    cortex_frame_id: String,
    description: String,
}

impl CortexProposal {
    pub fn new(
        proposal_id: impl Into<String>,
        consciousness_id: impl Into<String>,
        cortex_frame_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            consciousness_id: consciousness_id.into(),
            cortex_frame_id: cortex_frame_id.into(),
            description: description.into(),
        }
    }

    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }

    pub fn consciousness_id(&self) -> &str {
        &self.consciousness_id
    }

    pub fn cortex_frame_id(&self) -> &str {
        &self.cortex_frame_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Gate decision. Rejected/deferred cannot carry applied state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CognitiveDisposition {
    Accepted { reasons: Vec<String> },
    Rejected { reasons: Vec<String> },
    Deferred { reconsideration: String },
}

/// Adopted intention as durable cognitive state. Contains no physical delta;
/// physical action requires separate motor validation and canonical transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Intention {
    proposal_id: String,
    consciousness_id: String,
    description: String,
}

impl Intention {
    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }

    pub fn consciousness_id(&self) -> &str {
        &self.consciousness_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub const fn contains_physical_delta(&self) -> bool {
        false
    }
}

/// Scripted gate deciding disposition. Default policy accepts feasible
/// proposals; test hooks override for rejection/deferral scenarios.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CognitiveGate {
    disposition_override: Option<CognitiveDisposition>,
}

impl CognitiveGate {
    pub fn new() -> Self {
        Self {
            disposition_override: None,
        }
    }

    pub fn reject_reason(&mut self, reason: impl Into<String>) {
        self.disposition_override = Some(CognitiveDisposition::Rejected {
            reasons: vec![reason.into()],
        });
    }

    pub fn defer_with_trigger(&mut self, trigger: impl Into<String>) {
        self.disposition_override = Some(CognitiveDisposition::Deferred {
            reconsideration: trigger.into(),
        });
    }

    pub fn evaluate(&self, _proposal: &CortexProposal) -> CognitiveDisposition {
        self.disposition_override
            .clone()
            .unwrap_or(CognitiveDisposition::Accepted {
                reasons: vec!["feasible".into()],
            })
    }

    /// Only an Accepted disposition may adopt the proposal into Intention.
    pub fn adopt_intention(
        &self,
        proposal: &CortexProposal,
    ) -> Result<Intention, CognitiveGateError> {
        match self.evaluate(proposal) {
            CognitiveDisposition::Accepted { .. } => Ok(Intention {
                proposal_id: proposal.proposal_id().to_owned(),
                consciousness_id: proposal.consciousness_id().to_owned(),
                description: proposal.description().to_owned(),
            }),
            _ => Err(CognitiveGateError::NotAccepted),
        }
    }
}

impl Default for CognitiveGate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum CognitiveGateError {
    #[error("proposal was not accepted by cognitive gate")]
    NotAccepted,
}
