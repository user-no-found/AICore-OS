use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    unix_millis: u128,
}

impl Timestamp {
    pub fn from_unix_millis(unix_millis: u128) -> Self {
        Self { unix_millis }
    }

    pub fn unix_millis(&self) -> u128 {
        self.unix_millis
    }
}

pub trait AicoreClock {
    fn now(&self) -> Timestamp;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemClock;

impl AicoreClock for SystemClock {
    fn now(&self) -> Timestamp {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        Timestamp::from_unix_millis(millis)
    }
}
