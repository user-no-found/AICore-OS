import json
import os


def _error(request, machine_code, user_message_zh="Provider 请求失败"):
    return {
        "protocol_version": request.get("protocol_version", "provider.engine.v1"),
        "invocation_id": request.get("invocation_id", ""),
        "kind": "Error",
        "content": None,
        "payload_json": None,
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


def _started(request):
    return {
        "protocol_version": request.get("protocol_version", "provider.engine.v1"),
        "invocation_id": request.get("invocation_id", ""),
        "kind": "Started",
        "content": None,
        "payload_json": None,
        "user_message_zh": None,
        "machine_code": None,
    }


def _delta(request, content):
    return {
        "protocol_version": request.get("protocol_version", "provider.engine.v1"),
        "invocation_id": request.get("invocation_id", ""),
        "kind": "MessageDelta",
        "content": content,
        "payload_json": None,
        "user_message_zh": None,
        "machine_code": None,
    }


def _finished(request, payload=None):
    return {
        "protocol_version": request.get("protocol_version", "provider.engine.v1"),
        "invocation_id": request.get("invocation_id", ""),
        "kind": "Finished",
        "content": None,
        "payload_json": json.dumps(payload, ensure_ascii=False) if payload else None,
        "user_message_zh": None,
        "machine_code": None,
    }


def invoke(request):
    if os.environ.get("AICORE_PROVIDER_FORCE_MISSING_OPENAI") == "1":
        yield _error(request, "openai_sdk_missing")
        return

    try:
        from openai import OpenAI
    except Exception:
        yield _error(request, "openai_sdk_missing")
        return

    api_key = _credential(request)
    if not api_key:
        yield _error(request, "credential_unavailable", "Provider 凭证不可用")
        return

    client_args = {"api_key": api_key}
    if request.get("base_url"):
        client_args["base_url"] = request["base_url"]
    client = OpenAI(**client_args)

    try:
        yield _started(request)
        api_mode = request.get("api_mode")
        messages = request.get("messages", [])
        parameters = json.loads(request.get("parameters_json") or "{}")
        tools = json.loads(request.get("tools_json") or "null")

        if api_mode == "openai_responses":
            input_text = "\n".join(
                f"{message.get('role', 'user')}: {message.get('content', '')}"
                for message in messages
            )
            response = client.responses.create(
                model=request["model"],
                input=input_text,
                tools=tools,
                **parameters,
            )
            content = getattr(response, "output_text", "") or ""
        else:
            kwargs = {
                "model": request["model"],
                "messages": messages,
                **parameters,
            }
            if tools is not None:
                kwargs["tools"] = tools
            response = client.chat.completions.create(**kwargs)
            content = response.choices[0].message.content or ""

        if content:
            yield _delta(request, content)
        yield _finished(request)
    except Exception:
        yield _error(request, "openai_request_failed")
