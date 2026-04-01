#!/usr/bin/env bash
# Clean up E2E test screenshots safely.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="$SCRIPT_DIR/e2e_screenshots"

if [ ! -d "$TARGET" ]; then
    echo "Nothing to clean — $TARGET does not exist"
    exit 0
fi

# Only delete known screenshot files, not arbitrary content
find "$TARGET" -maxdepth 1 -name '*.png' -delete
echo "Cleaned screenshots from $TARGET"
