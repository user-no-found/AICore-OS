#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    Origin,
    ActiveViews,
    FollowedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    pub target: OutputTarget,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedOutputs {
    pub events: Vec<OutputEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRouter {
    default_target: OutputTarget,
}

impl OutputRouter {
    pub fn new(default_target: OutputTarget) -> Self {
        Self { default_target }
    }

    pub fn route_reply(&self, content: impl Into<String>) -> OutputEvent {
        OutputEvent {
            target: self.default_target.clone(),
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputRouter, OutputTarget};

    #[test]
    fn assistant_output_routes_to_active_views() {
        let router = OutputRouter::new(OutputTarget::ActiveViews);
        let output = router.route_reply("reply");

        assert_eq!(output.target, OutputTarget::ActiveViews);
        assert_eq!(output.content, "reply");
    }
}
