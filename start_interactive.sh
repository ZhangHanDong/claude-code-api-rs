#!/bin/bash

echo "启动交互式会话模式..."
echo "警告：交互式会话模式仍在实验阶段，可能存在稳定性问题"
echo ""

export RUN_MODE=interactive
export RUST_LOG=info
export CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true

cargo run --release --bin ccapi
