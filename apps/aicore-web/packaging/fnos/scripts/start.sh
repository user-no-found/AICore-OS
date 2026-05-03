#!/usr/bin/env sh
set -eu

exec ./bin/aicore-web --host "${AICORE_WEB_HOST:-0.0.0.0}" --port "${AICORE_WEB_PORT:-8731}"
