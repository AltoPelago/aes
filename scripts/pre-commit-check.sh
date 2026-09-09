#!/usr/bin/env bash
# Purpose: run fast public-repository safety checks.
# Run from: anywhere inside this git repo.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$script_dir/check-no-local-paths.sh"
node "$script_dir/check-public-surface.mjs"
