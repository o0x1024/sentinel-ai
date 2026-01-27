#!/usr/bin/env bash
# Generate license keys for Sentinel AI (wraps sentinel-license license_generator).
# Run from project root. Keys: src-tauri/license_keys.json
#
# Usage:
#   ./scripts/gen_license.sh gen              # Generate key pair (first time)
#   ./scripts/gen_license.sh sign <machine_id> [metadata]  # Sign license
#   ./scripts/gen_license.sh verify <key>     # Verify license
#   ./scripts/gen_license.sh pubkey           # Show public key for crypto.rs
#   ./scripts/gen_license.sh help             # Full help
#
# Machine ID: 16-char hex (XXXX-XXXX-XXXX-XXXX) or 64-char full hash.
# Get it from app: Settings > License > Machine ID.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TAURI_DIR="$PROJECT_ROOT/src-tauri"

cd "$TAURI_DIR"
exec cargo run -p sentinel-license --bin license_generator -- "$@"
