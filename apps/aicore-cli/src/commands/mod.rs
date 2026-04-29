pub(crate) mod agent;
pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod kernel;
pub(crate) mod memory;
pub(crate) mod model;
pub(crate) mod provider;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod status;

pub(crate) fn run_config_command(command: fn() -> Result<(), String>) -> i32 {
    match command() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

pub(crate) fn run_memory_command_with_arg(
    arg: &str,
    command: fn(&str) -> Result<(), String>,
) -> i32 {
    match command(arg) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}
