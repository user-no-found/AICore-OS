use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelErrorCode {
    MissingCapability,
    VersionMismatch,
    PermissionDenied,
    Unavailable,
    Conflict,
    InvalidRequest,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelErrorStage {
    Resolve,
    Route,
    Invoke,
    Dispatch,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryPolicyHint {
    DoNotRetry,
    RetryLater,
    RetryAfterMillis(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelError {
    pub code: KernelErrorCode,
    pub stage: KernelErrorStage,
    pub message_zh: String,
    pub safe_detail: Option<String>,
    pub retry_hint: RetryPolicyHint,
    pub secret_safe: bool,
}

impl KernelError {
    pub fn new(
        code: KernelErrorCode,
        stage: KernelErrorStage,
        message_zh: impl Into<String>,
    ) -> Self {
        Self {
            code,
            stage,
            message_zh: message_zh.into(),
            safe_detail: None,
            retry_hint: RetryPolicyHint::DoNotRetry,
            secret_safe: true,
        }
    }

    pub fn with_safe_detail(mut self, detail: impl Into<String>) -> Self {
        self.safe_detail = Some(detail.into());
        self
    }

    pub fn mark_secret_safe(mut self) -> Self {
        self.secret_safe = true;
        self
    }
}

impl Display for KernelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.message_zh, self.code)
    }
}

impl Error for KernelError {}

#[cfg(test)]
mod tests {
    use super::{KernelError, KernelErrorCode, KernelErrorStage};

    #[test]
    fn kernel_error_has_user_message_and_machine_code() {
        let error = KernelError::new(
            KernelErrorCode::MissingCapability,
            KernelErrorStage::Route,
            "缺少能力",
        );

        assert_eq!(error.code, KernelErrorCode::MissingCapability);
        assert_eq!(error.message_zh, "缺少能力");
    }

    #[test]
    fn kernel_error_redaction_marks_secret_safe() {
        let error = KernelError::new(
            KernelErrorCode::Unavailable,
            KernelErrorStage::Invoke,
            "服务不可用",
        )
        .with_safe_detail("provider unavailable")
        .mark_secret_safe();

        assert!(error.secret_safe);
        assert!(!error.safe_detail.unwrap().contains("secret"));
    }
}
