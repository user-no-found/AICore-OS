#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    ActiveView,
    ExternalReply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    pub target: OutputTarget,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRouter {
    default_target: OutputTarget,
}

impl OutputRouter {
    pub fn new(default_target: OutputTarget) -> Self {
        Self {
            default_target,
        }
    }

    pub fn route_reply(&self, content: impl Into<String>) -> OutputEvent {
        OutputEvent {
            target: self.default_target.clone(),
            content: content.into(),
        }
    }
}
