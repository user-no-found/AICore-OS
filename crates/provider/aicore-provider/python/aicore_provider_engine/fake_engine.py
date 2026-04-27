def invoke(request):
    invocation_id = request.get("invocation_id", "")
    protocol_version = request.get("protocol_version", "provider.engine.v1")
    messages = request.get("messages", [])
    content = " ".join(message.get("content", "") for message in messages)

    yield {
        "protocol_version": protocol_version,
        "invocation_id": invocation_id,
        "kind": "Started",
        "content": None,
        "payload_json": None,
        "user_message_zh": None,
        "machine_code": None,
    }

    if "fail" in content:
        yield {
            "protocol_version": protocol_version,
            "invocation_id": invocation_id,
            "kind": "Error",
            "content": None,
            "payload_json": None,
            "user_message_zh": "Provider 请求失败",
            "machine_code": "fake_error",
        }
        return

    yield {
        "protocol_version": protocol_version,
        "invocation_id": invocation_id,
        "kind": "MessageDelta",
        "content": "pong" if "ping" in content else "fake response",
        "payload_json": None,
        "user_message_zh": None,
        "machine_code": None,
    }
    yield {
        "protocol_version": protocol_version,
        "invocation_id": invocation_id,
        "kind": "Finished",
        "content": None,
        "payload_json": None,
        "user_message_zh": None,
        "machine_code": None,
    }
