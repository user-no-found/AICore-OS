#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    pub level: HealthLevel,
    pub summary_zh: String,
}
