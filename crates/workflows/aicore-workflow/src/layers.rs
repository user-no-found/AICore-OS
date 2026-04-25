#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Foundation,
    Kernel,
    Core,
    AppAicore,
}

impl Workflow {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "foundation" => Some(Self::Foundation),
            "kernel" => Some(Self::Kernel),
            "core" => Some(Self::Core),
            "app-aicore" => Some(Self::AppAicore),
            _ => None,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Foundation => "底层",
            Self::Kernel => "内核层",
            Self::Core => "底层与内核层",
            Self::AppAicore => "应用层 aicore",
        }
    }

    pub fn crates(self) -> &'static [&'static str] {
        match self {
            Self::Foundation => &["aicore-foundation", "aicore-contracts"],
            Self::Kernel => &[
                "aicore-auth",
                "aicore-config",
                "aicore-control",
                "aicore-runtime",
                "aicore-surface",
                "aicore-tools",
                "aicore-memory",
                "aicore-skills",
                "aicore-evolution",
            ],
            Self::Core => &[],
            Self::AppAicore => &["aicore"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Workflow;

    #[test]
    fn parses_app_aicore_workflow() {
        assert_eq!(Workflow::parse("app-aicore"), Some(Workflow::AppAicore));
    }

    #[test]
    fn app_aicore_workflow_maps_to_aicore_package() {
        assert_eq!(Workflow::AppAicore.crates(), &["aicore"]);
    }
}
