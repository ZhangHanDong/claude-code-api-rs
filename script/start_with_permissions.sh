#!/bin/bash

# 启动带文件权限的 Claude Code API

echo "Starting Claude Code API with file permissions..."

# 方式1：使用环境变量跳过所有权限检查（不推荐在生产环境使用）
# export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true

# 方式2：添加特定目录的访问权限（推荐）
export CLAUDE_CODE__FILE_ACCESS__ADDITIONAL_DIRS='["/Users/zhangalex/Work", "/Users/zhangalex/Documents", "/tmp"]'

# 方式3：临时使用环境变量（最简单）
export CLAUDE_SKIP_PERMISSIONS=true

# 启动服务
./target/release/claude-code-api