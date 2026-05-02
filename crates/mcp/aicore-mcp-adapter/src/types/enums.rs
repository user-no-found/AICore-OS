use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerStatus {
    Configured,
    Discovered,
    Enabled,
    Disabled,
    Unavailable,
    Broken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCandidateStatus {
    Discovered,
    Mapped,
    Rejected,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpMappingStatus {
    Candidate,
    RegisteredToolModule,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTrustLevel {
    Untrusted,
    UserConfirmed,
    SystemManaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRedactionMode {
    SummaryOnly,
    ArtifactRefOnly,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpMappingValidationOutcome {
    AllowedMappingCandidate,
    RejectedUnknownTool,
    RejectedUntrustedServer,
    RejectedCandidateNotDiscovered,
}
