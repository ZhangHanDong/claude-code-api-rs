#!/bin/bash

# 高性能启动脚本 - 禁用所有不必要的功能

# 环境优化
export CLAUDE_CLI_NO_UPDATE_CHECK=1
export CLAUDE_CLI_NO_TELEMETRY=1
export CLAUDE_NO_TOOLS=1  # 如果支持的话

# 使用快速配置
export RUN_MODE=fast

# 设置日志级别
export RUST_LOG=info

# 禁用 MCP
export CLAUDE_CODE__MCP__ENABLED=false

# 跳过文件权限检查
export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true

# 增加缓存
export CLAUDE_CODE__CACHE__ENABLED=true
export CLAUDE_CODE__CACHE__MAX_ENTRIES=10000
export CLAUDE_CODE__CACHE__TTL_SECONDS=14400  # 4小时

# 禁用进程池预热
export CLAUDE_CODE__PROCESS_POOL__SIZE=0
export CLAUDE_CODE__PROCESS_POOL__MIN_IDLE=0

# 启用交互式会话
export CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true

echo "🚀 Starting Claude Code API in FAST mode"
echo "✅ Interactive sessions enabled"
echo "✅ Cache enabled (4-hour TTL)"
echo "✅ File permissions check disabled"
echo "❌ MCP disabled"
echo "❌ Process pool pre-warming disabled"

./target/release/ccapi