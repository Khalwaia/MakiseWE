use thiserror::Error;

/// Explicit representation transition record. It is an authoritative intent,
/// not a hidden level-of-detail switch. Physical state is never mutated by
/// this record; the transition only changes the declared representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolutionChanged {
    from_resolution_id: [u8; 48],
    from_len: u8,
    to_resolution_id: [u8; 48],
    to_len: u8,
    deterministic_seed: u64,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ResolutionError {
    #[error("resolution identifier cannot be empty")]
    EmptyResolutionId,
    #[error("resolution identifier exceeds canonical fixed-width limit")]
    ResolutionIdTooLong,
    #[error("resolution change requires a different target representation")]
    NoRepresentationChange,
}

impl ResolutionChanged {
    pub fn new(
        from_resolution_id: impl AsRef<str>,
        to_resolution_id: impl AsRef<str>,
        deterministic_seed: u64,
    ) -> Result<Self, ResolutionError> {
        let from = CanonicalId::new(from_resolution_id)?;
        let to = CanonicalId::new(to_resolution_id)?;
        if from == to {
            return Err(ResolutionError::NoRepresentationChange);
        }
        Ok(Self {
            from_resolution_id: from.bytes,
            from_len: from.len,
            to_resolution_id: to.bytes,
            to_len: to.len,
            deterministic_seed,
        })
    }

    pub fn from_resolution_id(&self) -> &str {
        CanonicalId::slice(&self.from_resolution_id, self.from_len)
    }

    pub fn to_resolution_id(&self) -> &str {
        CanonicalId::slice(&self.to_resolution_id, self.to_len)
    }

    pub fn deterministic_seed(&self) -> u64 {
        self.deterministic_seed
    }

    pub fn validate(&self) -> Result<(), ResolutionError> {
        if self.from_resolution_id == self.to_resolution_id {
            return Err(ResolutionError::NoRepresentationChange);
        }
        Ok(())
    }

    pub(crate) fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"resolution-changed-v1");
        bytes.extend_from_slice(self.from_resolution_id().as_bytes());
        bytes.push(0xff);
        bytes.extend_from_slice(self.to_resolution_id().as_bytes());
        bytes.push(0xff);
        bytes.extend_from_slice(&self.deterministic_seed.to_be_bytes());
        bytes
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct CanonicalId {
    bytes: [u8; 48],
    len: u8,
}

impl CanonicalId {
    const MAX_LEN: usize = 48;

    fn new(value: impl AsRef<str>) -> Result<Self, ResolutionError> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(ResolutionError::EmptyResolutionId);
        }
        let bytes = value.as_bytes();
        if bytes.len() > Self::MAX_LEN {
            return Err(ResolutionError::ResolutionIdTooLong);
        }
        let mut fixed = [0; Self::MAX_LEN];
        fixed[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            bytes: fixed,
            len: bytes.len() as u8,
        })
    }

    fn slice(fixed: &[u8; 48], length: u8) -> &str {
        std::str::from_utf8(&fixed[..usize::from(length)]).expect("validated UTF-8 identifier")
    }
}
