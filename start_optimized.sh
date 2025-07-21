#!/bin/bash

# 性能优化启动脚本

# 禁用 Claude CLI 更新检查和遥测
export CLAUDE_CLI_NO_UPDATE_CHECK=1
export CLAUDE_CLI_NO_TELEMETRY=1

# 设置运行环境
export RUN_MODE=optimized

# 设置日志级别
export RUST_LOG=info,claude_code_api=info

# 使用优化配置启动
echo "Starting Claude Code API with optimized settings..."
echo "Process pool enabled with min_idle=3, max_idle=8"
echo "Cache enabled with 2-hour TTL"
echo "File permissions check skipped for better performance"

./target/release/ccapi