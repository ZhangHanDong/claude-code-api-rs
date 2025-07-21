#!/bin/bash

# 测试 MCP 功能的脚本

echo "=== 测试 MCP 功能 ==="

# 测试 1: 基本测试（不使用 MCP）
echo -e "\n1. 基本测试（不使用 MCP）:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [{
        "role": "user",
        "content": "列出可用的 MCP 服务器"
      }]
    }' | jq .

# 测试 2: 使用 MCP 文件系统服务器
echo -e "\n2. 测试 MCP 文件系统功能:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [{
        "role": "user",
        "content": "使用 MCP filesystem 服务器列出 /tmp 目录的内容"
      }]
    }' | jq .

# 测试 3: 使用 MCP GitHub 服务器
echo -e "\n3. 测试 MCP GitHub 功能:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [{
        "role": "user",
        "content": "使用 MCP github 服务器获取 anthropics/claude-code 仓库信息"
      }]
    }' | jq .

echo -e "\n=== 测试完成 ==="