use crate::{AicoreError, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeaseId(String);

impl LeaseId {
    pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
        let value = value.into();
        if !value.is_empty()
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        {
            Ok(Self(value))
        } else {
            Err(AicoreError::InvalidId {
                kind: "lease id".to_string(),
                value,
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseState {
    Active,
    Released,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecord {
    pub id: LeaseId,
    pub owner: String,
    pub state: LeaseState,
    pub acquired_at: Timestamp,
    pub expires_at: Option<Timestamp>,
}

impl LeaseRecord {
    pub fn new(
        id: LeaseId,
        owner: impl Into<String>,
        acquired_at: Timestamp,
        expires_at: Option<Timestamp>,
    ) -> Self {
        Self {
            id,
            owner: owner.into(),
            state: LeaseState::Active,
            acquired_at,
            expires_at,
        }
    }

    pub fn release(&mut self) {
        self.state = LeaseState::Released;
    }

    pub fn expire(&mut self) {
        self.state = LeaseState::Expired;
    }

    pub fn revoke(&mut self) {
        self.state = LeaseState::Revoked;
    }
}

#[cfg(test)]
mod tests {
    use crate::Timestamp;

    use super::{LeaseId, LeaseRecord, LeaseState};

    #[test]
    fn lease_record_tracks_state() {
        let id = LeaseId::new("lease.main").expect("lease id should be valid");
        let mut lease = LeaseRecord::new(id, "worker-1", Timestamp::from_unix_millis(10), None);

        assert_eq!(lease.state, LeaseState::Active);
        lease.release();
        assert_eq!(lease.state, LeaseState::Released);
        lease.expire();
        assert_eq!(lease.state, LeaseState::Expired);
        lease.revoke();
        assert_eq!(lease.state, LeaseState::Revoked);
    }
}
