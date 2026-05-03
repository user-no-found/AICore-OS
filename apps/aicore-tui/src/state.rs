use std::path::Path;

use aicore_foundation::{
    AicoreResult, InstanceKind, ensure_instance_layout, resolve_instance_for_cwd,
};
use aicore_kernel::{RuntimeSummary, default_runtime};
use aicore_surface::{KernelSurface, default_kernel_surface};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiBlockKind {
    Prompt,
    Agent,
    Tool,
    Approval,
    Terminal,
    Assistant,
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

pub fn build_tui_model(cwd: &Path, home: &Path) -> AicoreResult<TuiModel> {
    let binding = resolve_instance_for_cwd(cwd, home)?;
    let paths = ensure_instance_layout(&binding)?;
    let runtime = default_runtime();
    let runtime_summary = runtime.summary();
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
        conversation_id: runtime_summary.conversation_id.clone(),
        event_count: runtime_summary.event_count,
        queue_len: runtime_summary.queue_len,
        active_turn: active_turn_label(&runtime_summary),
        tools_count: surface.tools.len(),
        memories_count: surface.memories.len(),
        skills_count: surface.skills.len(),
        proposals_count: surface.evolution_proposals.len(),
        blocks: default_blocks(&surface),
    })
}

fn active_turn_label(runtime_summary: &RuntimeSummary) -> String {
    if runtime_summary.queue_len > 0 {
        "queued".to_string()
    } else {
        "idle".to_string()
    }
}

fn default_blocks(surface: &KernelSurface) -> Vec<TuiBlock> {
    vec![
        TuiBlock {
            kind: TuiBlockKind::Assistant,
            title: "系统".to_string(),
            body: vec!["TUI 已绑定当前实例，等待输入。".to_string()],
        },
        TuiBlock {
            kind: TuiBlockKind::Agent,
            title: "智能体".to_string(),
            body: vec![
                "当前版本只负责显示与输入。".to_string(),
                "智能体运行时、统一 I/O 广播和真实会话接入在后续阶段打开。".to_string(),
            ],
        },
        TuiBlock {
            kind: TuiBlockKind::Tool,
            title: "能力摘要".to_string(),
            body: vec![format!(
                "tools={} memory={} skills={} proposals={}",
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
            kind: TuiBlockKind::Terminal,
            title: "终端".to_string(),
            body: vec!["输入 q 退出；普通文本会在本地回显。".to_string()],
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
