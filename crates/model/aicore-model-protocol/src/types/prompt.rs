use super::{
    PromptAssemblyId, PromptModuleId, PromptModuleKind, PromptModuleSource, PromptModuleVisibility,
};
use aicore_foundation::{InstanceId, SessionId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptAssemblyRequest {
    pub assembly_id: PromptAssemblyId,
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<String>,
    pub is_global_main: bool,
    pub modules: Vec<PromptModule>,
    pub max_budget_units: Option<u64>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptAssembly {
    pub assembly_id: PromptAssemblyId,
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<String>,
    pub modules: Vec<PromptModule>,
    pub module_digests: Vec<String>,
    pub total_content_unit_estimate: u64,
    pub redaction_summary: Option<String>,
    pub omitted_context_summary: Option<String>,
    pub visibility_scope_summary: Option<String>,
    pub model_request_ready: bool,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptModule {
    pub module_id: PromptModuleId,
    pub kind: PromptModuleKind,
    pub source: PromptModuleSource,
    pub visibility: PromptModuleVisibility,
    pub content: String,
    pub content_digest: Option<String>,
    pub content_unit_estimate: u64,
    pub redaction_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptAssemblyError {
    MissingRequiredModule(PromptModuleKind),
    ModuleOrderViolation,
    WorkspaceCannotUseGlobalMainSoul,
    WorkspaceCannotUseGlobalMainUserProfile,
}

impl PromptAssembly {
    pub fn build(request: PromptAssemblyRequest) -> Result<Self, PromptAssemblyError> {
        validate_required(&request.modules, PromptModuleKind::InstanceSoul)?;
        validate_required(&request.modules, PromptModuleKind::UserMessage)?;
        validate_order(&request.modules)?;
        validate_visibility(&request)?;

        let module_digests = request
            .modules
            .iter()
            .filter_map(|module| module.content_digest.clone())
            .collect();
        let total_content_unit_estimate = request
            .modules
            .iter()
            .map(|module| module.content_unit_estimate)
            .sum();

        Ok(Self {
            assembly_id: request.assembly_id,
            instance_id: request.instance_id,
            session_id: request.session_id,
            turn_id: request.turn_id,
            modules: request.modules,
            module_digests,
            total_content_unit_estimate,
            redaction_summary: None,
            omitted_context_summary: None,
            visibility_scope_summary: Some("current_instance_only".to_string()),
            model_request_ready: true,
            created_at: request.created_at,
        })
    }
}

fn validate_required(
    modules: &[PromptModule],
    kind: PromptModuleKind,
) -> Result<(), PromptAssemblyError> {
    if modules.iter().any(|module| module.kind == kind) {
        Ok(())
    } else {
        Err(PromptAssemblyError::MissingRequiredModule(kind))
    }
}

fn validate_order(modules: &[PromptModule]) -> Result<(), PromptAssemblyError> {
    let order = PromptModuleKind::fixed_order();
    let mut last_index = None;
    for module in modules {
        let Some(index) = order.iter().position(|kind| *kind == module.kind) else {
            return Err(PromptAssemblyError::ModuleOrderViolation);
        };
        if let Some(last) = last_index
            && index < last
        {
            return Err(PromptAssemblyError::ModuleOrderViolation);
        }
        last_index = Some(index);
    }
    Ok(())
}

fn validate_visibility(request: &PromptAssemblyRequest) -> Result<(), PromptAssemblyError> {
    if request.is_global_main {
        return Ok(());
    }

    for module in &request.modules {
        match module.source {
            PromptModuleSource::GlobalMainSoul => {
                return Err(PromptAssemblyError::WorkspaceCannotUseGlobalMainSoul);
            }
            PromptModuleSource::GlobalMainUserProfile => {
                return Err(PromptAssemblyError::WorkspaceCannotUseGlobalMainUserProfile);
            }
            _ => {}
        }
    }
    Ok(())
}
