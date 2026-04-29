mod commands;
mod config_store;
mod errors;
mod names;
mod terminal;

pub fn run_from_args(args: Vec<String>) -> i32 {
    dispatch(args.as_slice())
}

fn dispatch(args: &[String]) -> i32 {
    match args {
        [cmd] if cmd == "__component-smoke-stdio" => commands::kernel::run_component_smoke_stdio(),
        [cmd] if cmd == "__component-config-validate-stdio" => {
            commands::kernel::run_component_config_validate_stdio()
        }
        [cmd] if cmd == "__component-auth-list-stdio" => {
            commands::kernel::run_component_auth_list_stdio()
        }
        [cmd] if cmd == "__component-model-show-stdio" => {
            commands::kernel::run_component_model_show_stdio()
        }
        [cmd] if cmd == "__component-service-list-stdio" => {
            commands::kernel::run_component_service_list_stdio()
        }
        [cmd] if cmd == "__component-runtime-smoke-stdio" => {
            commands::kernel::run_component_runtime_smoke_stdio()
        }
        [cmd] if cmd == "__component-instance-list-stdio" => {
            commands::kernel::run_component_instance_list_stdio()
        }
        [cmd] if cmd == "__component-status-stdio" => {
            commands::kernel::run_component_status_stdio()
        }
        [cmd] if cmd == "__component-provider-smoke-stdio" => {
            commands::kernel::run_component_provider_smoke_stdio()
        }
        [cmd] if cmd == "__component-agent-smoke-stdio" => {
            commands::kernel::run_component_agent_smoke_stdio()
        }
        [cmd] if cmd == "__component-agent-session-smoke-stdio" => {
            commands::kernel::run_component_agent_session_smoke_stdio()
        }
        [cmd] if cmd == "__component-memory-status-stdio" => {
            commands::kernel::run_component_memory_status_stdio()
        }
        [cmd] if cmd == "__component-memory-search-stdio" => {
            commands::kernel::run_component_memory_search_stdio()
        }
        [cmd] if cmd == "__component-memory-proposals-stdio" => {
            commands::kernel::run_component_memory_proposals_stdio()
        }
        [cmd] if cmd == "__component-memory-audit-stdio" => {
            commands::kernel::run_component_memory_audit_stdio()
        }
        [cmd] if cmd == "__component-memory-wiki-stdio" => {
            commands::kernel::run_component_memory_wiki_stdio()
        }
        [cmd] if cmd == "__component-memory-wiki-page-stdio" => {
            commands::kernel::run_component_memory_wiki_page_stdio()
        }
        [cmd] if cmd == "__component-memory-remember-stdio" => {
            commands::kernel::run_component_memory_remember_stdio()
        }
        [cmd] if cmd == "__component-memory-accept-stdio" => {
            commands::kernel::run_component_memory_accept_stdio()
        }
        [cmd] if cmd == "__component-memory-reject-stdio" => {
            commands::kernel::run_component_memory_reject_stdio()
        }
        [cmd, rest @ ..] if cmd == "status" => commands::status::run_status_command(rest),
        [group, action, rest @ ..] if group == "instance" && action == "list" => {
            commands::status::run_instance_list_command(rest)
        }
        [group, action, rest @ ..] if group == "runtime" && action == "smoke" => {
            commands::runtime::run_runtime_smoke_command(rest)
        }
        [group, action, operation] if group == "kernel" && action == "route" => {
            commands::kernel::print_kernel_route(operation)
        }
        [group, action, operation] if group == "kernel" && action == "invoke-smoke" => {
            commands::kernel::print_kernel_invoke_smoke(operation)
        }
        [group, action, operation, rest @ ..]
            if group == "kernel" && action == "invoke-readonly" =>
        {
            commands::kernel::print_kernel_invoke_readonly(operation, rest)
        }
        [group, action, operation, rest @ ..] if group == "kernel" && action == "invoke-write" => {
            commands::kernel::print_kernel_invoke_write(operation, rest)
        }
        [group, action, operation] if group == "kernel" && action == "invoke-process-smoke" => {
            commands::kernel::print_kernel_invoke_process_smoke(operation)
        }
        [group, action] if group == "config" && action == "smoke" => {
            commands::run_config_command(commands::config::print_config_smoke)
        }
        [group, action] if group == "config" && action == "path" => {
            commands::run_config_command(commands::config::print_config_path)
        }
        [group, action] if group == "config" && action == "init" => {
            commands::run_config_command(commands::config::print_config_init)
        }
        [group, action] if group == "config" && action == "validate" => {
            commands::run_config_command(commands::config::print_config_validate)
        }
        [group, action] if group == "auth" && action == "list" => {
            commands::run_config_command(commands::auth::print_auth_list)
        }
        [group, action] if group == "model" && action == "show" => {
            commands::run_config_command(commands::model::print_model_show)
        }
        [group, action] if group == "service" && action == "list" => {
            commands::run_config_command(commands::service::print_service_list)
        }
        [group, action] if group == "provider" && action == "smoke" => {
            commands::run_config_command(commands::provider::print_provider_smoke)
        }
        [group, action, content] if group == "agent" && action == "smoke" => {
            commands::run_config_command_with_arg(content, commands::agent::print_agent_smoke)
        }
        [group, action, first, second] if group == "agent" && action == "session-smoke" => {
            commands::run_config_command_with_two_args(
                first,
                second,
                commands::agent::print_agent_session_smoke,
            )
        }
        [group, action] if group == "memory" && action == "status" => {
            commands::run_memory_command(commands::memory::print_memory_status)
        }
        [group, action] if group == "memory" && action == "audit" => {
            commands::run_memory_command(commands::memory::print_memory_audit)
        }
        [group, action] if group == "memory" && action == "proposals" => {
            commands::run_memory_command(commands::memory::print_memory_proposals)
        }
        [group, action] if group == "memory" && action == "wiki" => {
            commands::run_memory_command(commands::memory::print_memory_wiki_index)
        }
        [group, action, content] if group == "memory" && action == "remember" => {
            commands::run_memory_command_with_arg(content, commands::memory::print_memory_remember)
        }
        [group, action, page] if group == "memory" && action == "wiki" => {
            commands::run_memory_command_with_arg(page, commands::memory::print_memory_wiki_page)
        }
        [group, action, query, rest @ ..] if group == "memory" && action == "search" => {
            commands::run_memory_search_command(query, rest)
        }
        [group, action, proposal_id] if group == "memory" && action == "accept" => {
            commands::run_memory_command_with_arg(
                proposal_id,
                commands::memory::print_memory_accept,
            )
        }
        [group, action, proposal_id] if group == "memory" && action == "reject" => {
            commands::run_memory_command_with_arg(
                proposal_id,
                commands::memory::print_memory_reject,
            )
        }
        [group, _] if group == "config" => {
            eprintln!("未知 config 命令。");
            eprintln!("可用命令：config smoke | config path | config init | config validate");
            1
        }
        [group, _] if group == "memory" => {
            eprintln!("未知 memory 命令。");
            eprintln!(
                "可用命令：memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
        [group, _] if group == "agent" => {
            eprintln!("未知 agent 命令。");
            eprintln!(
                "可用命令：agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容>"
            );
            1
        }
        [group, _] if group == "kernel" => {
            eprintln!("未知 kernel 命令。");
            eprintln!(
                "可用命令：kernel route <operation> | kernel invoke-smoke <operation> | kernel invoke-readonly <operation> | kernel invoke-write <operation> | kernel invoke-process-smoke <operation>"
            );
            1
        }
        _ => {
            eprintln!("未知命令。");
            eprintln!(
                "可用命令：status | instance list | runtime smoke | kernel route <operation> | kernel invoke-smoke <operation> | kernel invoke-readonly <operation> | kernel invoke-write <operation> | kernel invoke-process-smoke <operation> | config smoke | config path | config init | config validate | auth list | model show | service list | provider smoke | agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容> | memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
    }
}

// runtime_status_handler_for_layout remains owned by aicore-kernel.

#[cfg(test)]
mod tests;
