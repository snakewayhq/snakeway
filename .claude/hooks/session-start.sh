#!/bin/bash
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

# Install Rust toolchain components for linting and formatting
rustup component add rustfmt clippy 2>/dev/null || true
