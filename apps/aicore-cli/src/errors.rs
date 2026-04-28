use aicore_provider::ProviderError;

pub(crate) fn config_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) => format!("I/O 错误：{message}"),
        aicore_config::ConfigError::Parse(message) => format!("配置解析错误：{message}"),
        aicore_config::ConfigError::Validation(message) => {
            format!("配置校验错误：{message}")
        }
    }
}

pub(crate) fn memory_error(error: aicore_memory::MemoryError) -> String {
    error.0
}

pub(crate) fn provider_error(error: ProviderError) -> String {
    match error {
        ProviderError::Resolve(message) => format!("provider 解析错误：{message}"),
        ProviderError::Invoke(message) => format!("provider 调用错误：{message}"),
    }
}

pub(crate) fn map_runtime_load_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) if message.contains("missing runtime config") => {
            "缺少 global-main runtime 配置，请先运行 config init 或配置模型。".to_string()
        }
        other => config_error(other),
    }
}
