use std::path::Path;

use aicore_foundation::{
    AicoreResult, InstanceKind, ensure_instance_layout, resolve_instance_for_cwd,
};
use aicore_surface::{KernelSurface, default_kernel_surface};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiBlockKind {
    Prompt,
    Agent,
    Tool,
    Approval,
    Terminal,
    Assistant,
    Code,
    Diff,
    Media,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiBlock {
    pub kind: TuiBlockKind,
    pub title: String,
    pub body: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TuiModel {
    pub instance_id: String,
    pub instance_kind: String,
    pub workspace_root: String,
    pub state_root: String,
    pub conversation_id: String,
    pub event_count: usize,
    pub queue_len: usize,
    pub active_turn: String,
    pub tools_count: usize,
    pub memories_count: usize,
    pub skills_count: usize,
    pub proposals_count: usize,
    pub blocks: Vec<TuiBlock>,
    pub terminal_capabilities: TerminalCapabilities,
    pub copy_button_label: String,
    pub media_status: String,
}

impl TuiModel {
    pub fn instance_kind_label(&self) -> &'static str {
        match self.instance_kind.as_str() {
            "global-main" => "主实例",
            "workspace" => "工作区实例",
            _ => "未知实例",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalCapabilities {
    pub mouse: bool,
    pub clipboard: String,
    pub inline_image: String,
    pub multiplexer: String,
}

pub fn build_tui_model(cwd: &Path, home: &Path) -> AicoreResult<TuiModel> {
    let binding = resolve_instance_for_cwd(cwd, home)?;
    let paths = ensure_instance_layout(&binding)?;
    let surface = default_kernel_surface();

    Ok(TuiModel {
        instance_id: binding.instance_id.as_str().to_string(),
        instance_kind: match binding.kind {
            InstanceKind::GlobalMain => "global-main".to_string(),
            InstanceKind::Workspace => "workspace".to_string(),
        },
        workspace_root: binding
            .workspace_root
            .as_deref()
            .unwrap_or(cwd)
            .display()
            .to_string(),
        state_root: paths.root.display().to_string(),
        conversation_id: "preview".to_string(),
        event_count: 0,
        queue_len: 0,
        active_turn: "idle".to_string(),
        tools_count: surface.tools.len(),
        memories_count: surface.memories.len(),
        skills_count: surface.skills.len(),
        proposals_count: surface.evolution_proposals.len(),
        terminal_capabilities: detect_terminal_capabilities(),
        copy_button_label: "复制".to_string(),
        media_status: "终端图形协议按能力探测，当前仅显示安全预览卡片。".to_string(),
        blocks: default_blocks(&surface),
    })
}

fn detect_terminal_capabilities() -> TerminalCapabilities {
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let multiplexer = if std::env::var_os("ZELLIJ").is_some() {
        "zellij"
    } else if std::env::var_os("TMUX").is_some() {
        "tmux"
    } else {
        "direct"
    };
    let inline_image = if term_program.contains("WezTerm") {
        "wezterm-inline"
    } else if term_program.contains("iTerm") {
        "iterm2-inline"
    } else if term.contains("kitty") {
        "kitty-graphics"
    } else if term.contains("sixel") || term.contains("xterm") {
        "sixel-or-halfblock"
    } else {
        "metadata-fallback"
    };

    TerminalCapabilities {
        mouse: true,
        clipboard: clipboard_status(),
        inline_image: inline_image.to_string(),
        multiplexer: multiplexer.to_string(),
    }
}

fn clipboard_status() -> String {
    if std::env::var_os("TMUX").is_some() || std::env::var_os("ZELLIJ").is_some() {
        "osc52-limited".to_string()
    } else if std::env::var_os("SSH_CONNECTION").is_some() {
        "osc52-remote".to_string()
    } else {
        "osc52-request".to_string()
    }
}

fn default_blocks(surface: &KernelSurface) -> Vec<TuiBlock> {
    vec![
        TuiBlock {
            kind: TuiBlockKind::Assistant,
            title: "实例已绑定".to_string(),
            body: vec!["当前目录已绑定到 AICore instance，TUI 正在等待输入。".to_string()],
        },
        TuiBlock {
            kind: TuiBlockKind::Terminal,
            title: "运行边界".to_string(),
            body: vec![
                "当前版本是本地显示与输入界面，不启动智能体运行时。".to_string(),
                "统一 I/O 广播和真实会话接入在后续阶段打开。".to_string(),
            ],
        },
        TuiBlock {
            kind: TuiBlockKind::Tool,
            title: "能力快照".to_string(),
            body: vec![format!(
                "工具 {}，记忆 {}，技能 {}，提案 {}。",
                surface.tools.len(),
                surface.memories.len(),
                surface.skills.len(),
                surface.evolution_proposals.len()
            )],
        },
        TuiBlock {
            kind: TuiBlockKind::Approval,
            title: "审批".to_string(),
            body: vec!["当前没有待处理审批。".to_string()],
        },
        TuiBlock {
            kind: TuiBlockKind::Code,
            title: "Rust 代码块 · 可点击复制".to_string(),
            body: vec![
                "pub struct KernelInvocationResultEnvelope {".to_string(),
                "    pub write_applied: bool,".to_string(),
                "    pub audit_closed: bool,".to_string(),
                "}".to_string(),
            ],
        },
        TuiBlock {
            kind: TuiBlockKind::Diff,
            title: "Diff 预览".to_string(),
            body: vec![
                "+ write_applied: true".to_string(),
                "+ audit_closed: true".to_string(),
                "- raw_payload: hidden".to_string(),
            ],
        },
        TuiBlock {
            kind: TuiBlockKind::Media,
            title: "媒体预览".to_string(),
            body: vec![
                "图片：后续按 Kitty / iTerm2 / WezTerm / Sixel / half-block 能力分级渲染。"
                    .to_string(),
                "视频：当前只显示封面、元数据和外部打开 action，不假装所有终端都能播放。"
                    .to_string(),
            ],
        },
        TuiBlock {
            kind: TuiBlockKind::Terminal,
            title: "快捷键".to_string(),
            body: vec![
                "Esc / Ctrl+C 退出；输入内容只进入本地显示，不启动智能体运行时。".to_string(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::build_tui_model;

    #[test]
    fn workspace_model_uses_project_aicore_root() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();

        assert_eq!(model.instance_kind, "workspace");
        assert!(model.instance_id.starts_with("workspace."));
        assert!(model.state_root.ends_with(".aicore"));
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be available")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "aicore-tui-state-{name}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("test dir should create");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
