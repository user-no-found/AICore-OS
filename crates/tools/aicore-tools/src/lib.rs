#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub id: String,
    pub toolset: String,
    pub display_name_zh: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBroker {
    revision: u64,
    tools: Vec<ToolDescriptor>,
}

impl ToolBroker {
    pub fn new() -> Self {
        Self {
            revision: 0,
            tools: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        id: impl Into<String>,
        toolset: impl Into<String>,
        display_name_zh: impl Into<String>,
    ) -> Result<(), String> {
        let id = id.into();
        if self.tools.iter().any(|tool| tool.id == id) {
            return Err(format!("duplicate tool id: {id}"));
        }

        self.revision += 1;
        self.tools.push(ToolDescriptor {
            id,
            toolset: toolset.into(),
            display_name_zh: display_name_zh.into(),
            revision: self.revision,
        });
        Ok(())
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn list_by_toolset(&self, toolset: &str) -> Vec<&ToolDescriptor> {
        self.tools
            .iter()
            .filter(|tool| tool.toolset == toolset)
            .collect()
    }

    pub fn list(&self) -> &[ToolDescriptor] {
        &self.tools
    }
}

pub fn default_tool_broker() -> ToolBroker {
    let mut broker = ToolBroker::new();
    broker
        .register("tool.git.status", "toolset.git", "Git 状态")
        .expect("default git status tool should register");
    broker
        .register("tool.fs.read_file", "toolset.filesystem", "读取文件")
        .expect("default fs read_file tool should register");
    broker
}

#[cfg(test)]
mod tests {
    use super::{default_tool_broker, ToolBroker};

    #[test]
    fn registration_increases_revision() {
        let mut broker = ToolBroker::new();
        assert_eq!(broker.revision(), 0);

        broker
            .register("tool.git.status", "toolset.git", "Git 状态")
            .expect("registration should pass");

        assert_eq!(broker.revision(), 1);
        assert_eq!(broker.list()[0].revision, 1);
    }

    #[test]
    fn rejects_duplicate_tool_ids() {
        let mut broker = ToolBroker::new();
        broker
            .register("tool.git.status", "toolset.git", "Git 状态")
            .expect("first registration should pass");

        let error = broker
            .register("tool.git.status", "toolset.git", "重复工具")
            .expect_err("duplicate tool id should fail");

        assert_eq!(error, "duplicate tool id: tool.git.status");
    }

    #[test]
    fn lists_tools_by_toolset() {
        let broker = default_tool_broker();
        let git_tools = broker.list_by_toolset("toolset.git");
        let fs_tools = broker.list_by_toolset("toolset.filesystem");

        assert_eq!(git_tools.len(), 1);
        assert_eq!(git_tools[0].id, "tool.git.status");
        assert_eq!(fs_tools.len(), 1);
        assert_eq!(fs_tools[0].id, "tool.fs.read_file");
    }
}
