#!/bin/bash

# 编译并运行 Claude Code API

echo "🔨 编译项目..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ 编译成功！"
    echo "🚀 启动服务器..."
    ./target/release/claude-code-api
else
    echo "❌ 编译失败"
    exit 1
fi