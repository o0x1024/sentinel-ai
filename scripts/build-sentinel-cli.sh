#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${ROOT_DIR}/dist/sentinel-cli"

mkdir -p "${OUTPUT_DIR}"

pushd "${ROOT_DIR}/src-tauri" >/dev/null
cargo build -p sentinel-cli --release
popd >/dev/null

cp "${ROOT_DIR}/src-tauri/target/release/sentinel-cli" "${OUTPUT_DIR}/sentinel-cli"
cp "${ROOT_DIR}/docs/cli-offline-deployment.md" "${OUTPUT_DIR}/README.md"

tar -C "${OUTPUT_DIR}" -czf "${ROOT_DIR}/dist/sentinel-cli.tar.gz" sentinel-cli README.md

echo "Built:"
echo "  ${OUTPUT_DIR}/sentinel-cli"
echo "  ${ROOT_DIR}/dist/sentinel-cli.tar.gz"
