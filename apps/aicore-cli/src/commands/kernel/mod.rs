pub(crate) mod adoption;
mod component_stdio;
mod invoke;
mod memory_process;
mod memory_write_process;
mod payload;
mod process;
mod route;

pub(crate) use adoption::{adopt_readonly, emit_local_direct_json};
pub(crate) use invoke::{
    print_kernel_invoke_readonly, print_kernel_invoke_smoke, print_kernel_invoke_write,
};
pub(crate) use memory_process::{
    run_component_memory_audit_stdio, run_component_memory_proposals_stdio,
    run_component_memory_search_stdio, run_component_memory_status_stdio,
    run_component_memory_wiki_page_stdio, run_component_memory_wiki_stdio,
};
pub(crate) use memory_write_process::{
    run_component_memory_accept_stdio, run_component_memory_reject_stdio,
    run_component_memory_remember_stdio,
};
pub(crate) use process::{
    print_kernel_invoke_process_smoke, run_component_agent_session_smoke_stdio,
    run_component_agent_smoke_stdio, run_component_auth_list_stdio,
    run_component_config_validate_stdio, run_component_instance_list_stdio,
    run_component_model_show_stdio, run_component_provider_smoke_stdio,
    run_component_runtime_smoke_stdio, run_component_service_list_stdio, run_component_smoke_stdio,
    run_component_status_stdio,
};
pub(crate) use route::print_kernel_route;

#[cfg(test)]
pub(crate) use adoption::{KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix};
