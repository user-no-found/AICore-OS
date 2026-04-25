#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workflow {
    Foundation,
    Kernel,
    Core,
}

impl Workflow {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "foundation" => Some(Self::Foundation),
            "kernel" => Some(Self::Kernel),
            "core" => Some(Self::Core),
            _ => None,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Foundation => "底层",
            Self::Kernel => "内核层",
            Self::Core => "底层与内核层",
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
        }
    }
}
