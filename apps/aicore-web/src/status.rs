use serde_json::json;

pub fn health_json() -> String {
    json!({
        "status": "ok",
        "service": "aicore-web",
        "surface": "web",
        "runtime_connected": false,
        "agent_runtime_started": false
    })
    .to_string()
}

pub fn status_json() -> String {
    json!({
        "app": "aicore-web",
        "ui": "vue3",
        "backend": "rust",
        "mode": "preview",
        "lan_ready": true,
        "unified_io": "reserved",
        "agent_runtime": "not_started",
        "provider": "not_called",
        "tools": "not_executed",
        "memory_write": "disabled"
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn status_declares_preview_boundaries() {
        let value: serde_json::Value = serde_json::from_str(&super::status_json()).unwrap();
        assert_eq!(value["ui"], "vue3");
        assert_eq!(value["backend"], "rust");
        assert_eq!(value["agent_runtime"], "not_started");
        assert_eq!(value["memory_write"], "disabled");
    }
}
