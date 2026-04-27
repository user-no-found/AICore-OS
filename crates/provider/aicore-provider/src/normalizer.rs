use crate::{ModelResponse, ProviderEngineEvent, ProviderEngineEventKind, ProviderError};

pub fn events_to_model_response(
    events: &[ProviderEngineEvent],
) -> Result<ModelResponse, ProviderError> {
    if let Some(error) = events
        .iter()
        .find(|event| event.kind == ProviderEngineEventKind::Error)
    {
        let user_message = error
            .user_message_zh
            .clone()
            .unwrap_or_else(|| "Provider 请求失败".to_string());
        let machine_code = error
            .machine_code
            .clone()
            .unwrap_or_else(|| "provider_error".to_string());
        return Err(ProviderError::Invoke(format!(
            "{user_message}（{machine_code}）"
        )));
    }

    let content = events
        .iter()
        .filter(|event| event.kind == ProviderEngineEventKind::MessageDelta)
        .filter_map(|event| event.content.as_deref())
        .collect::<Vec<_>>()
        .join("");

    if content.is_empty() {
        return Err(ProviderError::Invoke(
            "Provider 未返回可用内容（empty_response）".to_string(),
        ));
    }

    Ok(ModelResponse {
        role: "assistant".to_string(),
        content,
    })
}
