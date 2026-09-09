#!/usr/bin/env bash
# Purpose: fail if versioned or untracked release files contain machine-local paths.
# Run from: anywhere inside this git repo.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"
cd "$repo_root"

patterns=(
  '/Users/'
  'file:///(Users|home)/'
  '/home/[^/[:space:]]+/'
  '[A-Za-z]:\\\\Users\\\\'
)

failed=0
while IFS= read -r -d '' file; do
  [[ "$file" == "scripts/check-no-local-paths.sh" ]] && continue
  [[ -f "$file" ]] || continue
  for pattern in "${patterns[@]}"; do
    if LC_ALL=C grep -nI -E "$pattern" "$file"; then
      printf 'Local path marker in %s (pattern: %s)\n' "$file" "$pattern"
      failed=1
    fi
  done
done < <(git ls-files --cached --others --exclude-standard -z)

if [[ "$failed" -ne 0 ]]; then
  echo "Replace machine-local paths with repository-relative or public links."
  exit 1
fi

echo "No local filesystem paths found in release files."
