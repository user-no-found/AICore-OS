use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeliveryIdentity {
    ActiveViews,
    External { platform: String, target_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    Origin,
    ActiveViews,
    FollowedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    pub target: OutputTarget,
    pub identity: DeliveryIdentity,
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
            identity: DeliveryIdentity::ActiveViews,
            content: content.into(),
        }
    }
}

pub fn dedupe_outputs(events: Vec<OutputEvent>) -> RoutedOutputs {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for event in events {
        if seen.insert(event.identity.clone()) {
            deduped.push(event);
        }
    }

    RoutedOutputs { events: deduped }
}

#[cfg(test)]
mod tests {
    use super::{DeliveryIdentity, OutputEvent, OutputRouter, OutputTarget, dedupe_outputs};

    #[test]
    fn active_views_output_has_active_views_identity() {
        let router = OutputRouter::new(OutputTarget::ActiveViews);
        let output = router.route_reply("reply");

        assert_eq!(output.target, OutputTarget::ActiveViews);
        assert_eq!(output.identity, DeliveryIdentity::ActiveViews);
        assert_eq!(output.content, "reply");
    }

    #[test]
    fn dedupes_outputs_by_identity_keeping_first() {
        let outputs = dedupe_outputs(vec![
            OutputEvent {
                target: OutputTarget::Origin,
                identity: DeliveryIdentity::External {
                    platform: "feishu".to_string(),
                    target_id: "chat-1".to_string(),
                },
                content: "reply".to_string(),
            },
            OutputEvent {
                target: OutputTarget::FollowedExternal,
                identity: DeliveryIdentity::External {
                    platform: "feishu".to_string(),
                    target_id: "chat-1".to_string(),
                },
                content: "reply".to_string(),
            },
        ]);

        assert_eq!(outputs.events.len(), 1);
        assert_eq!(outputs.events[0].target, OutputTarget::Origin);
    }
}
