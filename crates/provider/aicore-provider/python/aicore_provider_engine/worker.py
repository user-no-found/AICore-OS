import argparse
import json
import sys

from . import fake_engine


def _load_engine(engine_name):
    if engine_name == "fake":
        return fake_engine
    if engine_name == "openai":
        from . import openai_engine

        return openai_engine
    if engine_name == "anthropic":
        from . import anthropic_engine

        return anthropic_engine
    raise ValueError(f"unknown engine: {engine_name}")


def _emit(event):
    sys.stdout.write(json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", required=True)
    args = parser.parse_args()
    engine = _load_engine(args.engine)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            for event in engine.invoke(request):
                _emit(event)
        except Exception as error:
            sys.stderr.write(f"provider worker error: {type(error).__name__}\n")
            sys.stderr.flush()
            _emit(
                {
                    "protocol_version": "provider.engine.v1",
                    "invocation_id": "",
                    "kind": "Error",
                    "content": None,
                    "payload_json": None,
                    "user_message_zh": "Provider 请求失败",
                    "machine_code": "worker_error",
                }
            )


if __name__ == "__main__":
    main()
