#!/usr/bin/env bash
# Source this file when using the workspace-local cloud tool installation.
if [ -d /workspace/.tools/bun/bin ]; then export PATH="/workspace/.tools/bun/bin:$PATH"; fi
if [ -d /workspace/.tools/cargo ]; then
  export CARGO_HOME=/workspace/.tools/cargo
  export RUSTUP_HOME=/workspace/.tools/rustup
  export PATH="/workspace/.tools/cargo/bin:$PATH"
fi
