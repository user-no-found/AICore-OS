#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Foundation,
    Kernel,
    Core,
    AppAicore,
    AppCli,
    AppTui,
}

impl Workflow {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "foundation" => Some(Self::Foundation),
            "kernel" => Some(Self::Kernel),
            "core" => Some(Self::Core),
            "app-aicore" => Some(Self::AppAicore),
            "app-cli" => Some(Self::AppCli),
            "app-tui" => Some(Self::AppTui),
            _ => None,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Foundation => "底层",
            Self::Kernel => "内核层",
            Self::Core => "底层与内核层",
            Self::AppAicore => "应用层 aicore",
            Self::AppCli => "应用层 aicore-cli",
            Self::AppTui => "应用层 aicore-tui",
        }
    }

    pub fn crates(self) -> &'static [&'static str] {
        match self {
            Self::Foundation => &["aicore-foundation"],
            Self::Kernel => &["aicore-foundation", "aicore-kernel"],
            Self::Core => &[],
            Self::AppAicore => &["aicore"],
            Self::AppCli => &[
                "aicore-cli",
                "aicore-agent",
                "aicore-provider",
                "aicore-memory",
                "aicore-surface",
            ],
            Self::AppTui => &["aicore-tui"],
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

    #[test]
    fn parses_app_cli_workflow() {
        assert_eq!(Workflow::parse("app-cli"), Some(Workflow::AppCli));
    }

    #[test]
    fn app_cli_workflow_maps_to_aicore_cli_package() {
        assert!(Workflow::AppCli.crates().contains(&"aicore-cli"));
    }

    #[test]
    fn parses_app_tui_workflow() {
        assert_eq!(Workflow::parse("app-tui"), Some(Workflow::AppTui));
    }

    #[test]
    fn app_tui_workflow_maps_to_aicore_tui_package() {
        assert_eq!(Workflow::AppTui.crates(), &["aicore-tui"]);
    }

    #[test]
    fn kernel_workflow_includes_foundation_and_kernel() {
        assert_eq!(
            Workflow::Kernel.crates(),
            &["aicore-foundation", "aicore-kernel"]
        );
    }

    #[test]
    fn kernel_workflow_excludes_removed_internal_kernel_crates() {
        assert!(!Workflow::Kernel.crates().contains(&"aicore-contracts"));
        assert!(!Workflow::Kernel.crates().contains(&"aicore-control"));
        assert!(!Workflow::Kernel.crates().contains(&"aicore-runtime"));
    }

    #[test]
    fn application_workflow_still_includes_cli_provider_agent_memory_surface() {
        assert_eq!(
            Workflow::AppCli.crates(),
            &[
                "aicore-cli",
                "aicore-agent",
                "aicore-provider",
                "aicore-memory",
                "aicore-surface"
            ]
        );
    }
}
