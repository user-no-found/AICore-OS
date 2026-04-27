import json
import os


def _event(request, kind, content=None, payload_json=None, user_message_zh=None, machine_code=None):
    return {
        "protocol_version": request.get("protocol_version", "provider.engine.v1"),
        "invocation_id": request.get("invocation_id", ""),
        "kind": kind,
        "content": content,
        "payload_json": payload_json,
        "user_message_zh": user_message_zh,
        "machine_code": machine_code,
    }


def _credential(request):
    lease = request.get("credential_lease_ref")
    if not lease:
        return None
    if lease.startswith("env:"):
        return os.environ.get(lease[4:])
    return None


def _split_messages(messages):
    system_parts = []
    conversation = []
    for message in messages:
        role = message.get("role", "user")
        content = message.get("content", "")
        if role == "system":
            system_parts.append(content)
        else:
            conversation.append({"role": role, "content": content})
    return "\n".join(system_parts), conversation


def invoke(request):
    if os.environ.get("AICORE_PROVIDER_FORCE_MISSING_ANTHROPIC") == "1":
        yield _event(
            request,
            "Error",
            user_message_zh="Provider 请求失败",
            machine_code="anthropic_sdk_missing",
        )
        return

    try:
        from anthropic import Anthropic
    except Exception:
        yield _event(
            request,
            "Error",
            user_message_zh="Provider 请求失败",
            machine_code="anthropic_sdk_missing",
        )
        return

    api_key = _credential(request)
    if not api_key:
        yield _event(
            request,
            "Error",
            user_message_zh="Provider 凭证不可用",
            machine_code="credential_unavailable",
        )
        return

    client_args = {"api_key": api_key}
    if request.get("base_url"):
        client_args["base_url"] = request["base_url"]
    client = Anthropic(**client_args)

    try:
        yield _event(request, "Started")
        system, messages = _split_messages(request.get("messages", []))
        parameters = json.loads(request.get("parameters_json") or "{}")
        tools = json.loads(request.get("tools_json") or "null")
        kwargs = {
            "model": request["model"],
            "messages": messages,
            "max_tokens": parameters.pop("max_tokens", 1024),
            **parameters,
        }
        if system:
            kwargs["system"] = system
        if tools is not None:
            kwargs["tools"] = tools

        response = client.messages.create(**kwargs)
        content = "".join(
            getattr(block, "text", "")
            for block in getattr(response, "content", [])
            if getattr(block, "type", None) == "text"
        )
        if content:
            yield _event(request, "MessageDelta", content=content)
        yield _event(request, "Finished")
    except Exception:
        yield _event(
            request,
            "Error",
            user_message_zh="Provider 请求失败",
            machine_code="anthropic_request_failed",
        )
