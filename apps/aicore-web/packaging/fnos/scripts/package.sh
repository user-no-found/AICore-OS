#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APP_DIR=$(CDPATH= cd -- "${SCRIPT_DIR}/../../.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "${APP_DIR}/../.." && pwd)
PACKAGE_TEMPLATE="${APP_DIR}/packaging/fnos/package"
STAGE_ROOT="${REPO_ROOT}/target/fnos/aicore-web-package"
OUTPUT_DIR="${REPO_ROOT}/target/fnos/dist"
FPK_NAME="aicore-web.fpk"

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "缺少命令：$1" >&2
        exit 1
    fi
}

copy_tree() {
    src="$1"
    dst="$2"
    mkdir -p "$dst"
    (cd "$src" && tar -cf - .) | (cd "$dst" && tar -xf -)
}

generate_icon() {
    size="$1"
    output="$2"
    font_size=$((size / 4))
    border=$((size / 12))
    ffmpeg -hide_banner -loglevel error \
        -f lavfi \
        -i "color=c=0x0a0f1f:s=${size}x${size}" \
        -frames:v 1 \
        -vf "drawbox=x=${border}:y=${border}:w=$((size - border * 2)):h=$((size - border * 2)):color=0x42e8c3:t=$((border / 2 + 1)),drawtext=text='AC':fontcolor=0xf8fafc:fontsize=${font_size}:x=(w-text_w)/2:y=(h-text_h)/2" \
        -y "$output"
}

require_command cargo
require_command npm
require_command fnpack
require_command ffmpeg

cd "${APP_DIR}/web"
npm run build

cd "${REPO_ROOT}"
cargo build -p aicore-web --release

rm -rf "${STAGE_ROOT}" "${OUTPUT_DIR}"
mkdir -p "${STAGE_ROOT}" "${OUTPUT_DIR}"
copy_tree "${PACKAGE_TEMPLATE}" "${STAGE_ROOT}"

mkdir -p "${STAGE_ROOT}/app/server"
cp "${REPO_ROOT}/target/release/aicore-web" "${STAGE_ROOT}/app/server/aicore-web"
chmod 755 "${STAGE_ROOT}/app/server/aicore-web"

mkdir -p "${STAGE_ROOT}/app/www/assets"
cp "${APP_DIR}/web/dist/index.html" "${STAGE_ROOT}/app/www/index.html"
cp "${APP_DIR}/web/dist/assets/app.js" "${STAGE_ROOT}/app/www/assets/app.js"
cp "${APP_DIR}/web/dist/assets/app.css" "${STAGE_ROOT}/app/www/assets/app.css"

mkdir -p "${STAGE_ROOT}/app/ui/images"
generate_icon 64 "${STAGE_ROOT}/ICON.PNG"
generate_icon 256 "${STAGE_ROOT}/ICON_256.PNG"
cp "${STAGE_ROOT}/ICON.PNG" "${STAGE_ROOT}/app/ui/images/icon_64.png"
cp "${STAGE_ROOT}/ICON_256.PNG" "${STAGE_ROOT}/app/ui/images/icon_256.png"

chmod 755 "${STAGE_ROOT}/cmd/main" "${STAGE_ROOT}"/cmd/*_init "${STAGE_ROOT}"/cmd/*_callback

cd "${OUTPUT_DIR}"
fnpack build --directory "${STAGE_ROOT}"

echo "fnOS 原生 FPK 已生成：${OUTPUT_DIR}/${FPK_NAME}"
