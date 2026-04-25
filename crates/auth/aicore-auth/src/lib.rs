#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEntry {
    pub auth_ref: String,
    pub provider: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthPool {
    pub entries: Vec<AuthEntry>,
}
