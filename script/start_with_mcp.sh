#!/bin/bash

# 启动带 MCP 支持的 Claude Code API

echo "Starting Claude Code API with MCP support..."

# 启用 MCP
export CLAUDE_CODE__MCP__ENABLED=true

# 使用配置文件（推荐）
export CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"

# 或者使用 JSON 字符串
# export CLAUDE_CODE__MCP__CONFIG_JSON='{"mcpServers":{"filesystem":{"command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","/tmp"]}}}'

# 严格模式：只使用 MCP 配置中的服务器
# export CLAUDE_CODE__MCP__STRICT=true

# 调试模式：显示 MCP 服务器错误
# export CLAUDE_CODE__MCP__DEBUG=true

# 同时启用文件权限
export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true

# 启动服务
./target/release/claude-code-api