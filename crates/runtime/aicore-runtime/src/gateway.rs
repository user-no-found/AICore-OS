#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewaySource {
    Cli,
    Tui,
    Web,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEnvelope {
    pub source: GatewaySource,
    pub platform: Option<String>,
    pub target_id: Option<String>,
    pub sender_id: Option<String>,
    pub is_group: bool,
    pub mentioned_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayInput {
    pub envelope: TransportEnvelope,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceIoGateway {
    instance_id: String,
}

impl InstanceIoGateway {
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
        }
    }

    pub fn normalize_user_input(
        &self,
        envelope: TransportEnvelope,
        content: impl Into<String>,
    ) -> GatewayInput {
        GatewayInput {
            envelope,
            content: content.into(),
        }
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
}
