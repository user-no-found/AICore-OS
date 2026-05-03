#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Foundation,
    Kernel,
    Core,
    AppAicore,
    AppCli,
    AppTui,
    AppWeb,
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
            "app-web" => Some(Self::AppWeb),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Kernel => "kernel",
            Self::Core => "core",
            Self::AppAicore => "app-aicore",
            Self::AppCli => "app-cli",
            Self::AppTui => "app-tui",
            Self::AppWeb => "app-web",
        }
    }

    pub fn target_label(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Kernel => "foundation + kernel",
            Self::Core => "foundation + kernel",
            Self::AppAicore => "aicore",
            Self::AppCli => "aicore-cli",
            Self::AppTui => "aicore-tui",
            Self::AppWeb => "aicore-web",
        }
    }

    pub fn crates(self) -> &'static [&'static str] {
        match self {
            Self::Foundation => &["aicore-foundation", "aicore-terminal"],
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
            Self::AppWeb => &["aicore-web"],
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
    fn parses_app_web_workflow() {
        assert_eq!(Workflow::parse("app-web"), Some(Workflow::AppWeb));
    }

    #[test]
    fn app_web_workflow_maps_to_aicore_web_package() {
        assert_eq!(Workflow::AppWeb.crates(), &["aicore-web"]);
    }

    #[test]
    fn kernel_workflow_includes_foundation_and_kernel() {
        assert_eq!(
            Workflow::Kernel.crates(),
            &["aicore-foundation", "aicore-kernel"]
        );
    }

    #[test]
    fn foundation_workflow_includes_terminal_kit() {
        assert_eq!(
            Workflow::Foundation.crates(),
            &["aicore-foundation", "aicore-terminal"]
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

    #[test]
    fn provider_workflow_includes_aicore_provider() {
        assert!(Workflow::AppCli.crates().contains(&"aicore-provider"));
    }
}
