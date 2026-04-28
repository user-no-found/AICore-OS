pub(crate) mod adoption;
mod invoke;
mod payload;
mod process;
mod route;

pub(crate) use invoke::{print_kernel_invoke_readonly, print_kernel_invoke_smoke};
pub(crate) use process::{
    print_kernel_invoke_process_smoke, run_component_auth_list_stdio,
    run_component_config_validate_stdio, run_component_model_show_stdio,
    run_component_service_list_stdio, run_component_smoke_stdio,
};
pub(crate) use route::print_kernel_route;

#[cfg(test)]
pub(crate) use adoption::{KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix};
