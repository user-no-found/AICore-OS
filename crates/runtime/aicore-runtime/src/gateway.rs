#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewaySource {
    Cli,
    Tui,
    Web,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayInput {
    pub source: GatewaySource,
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
        source: GatewaySource,
        content: impl Into<String>,
    ) -> GatewayInput {
        GatewayInput {
            source,
            content: content.into(),
        }
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
}
