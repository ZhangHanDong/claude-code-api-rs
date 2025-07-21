#!/bin/bash

echo "🔍 Checking Claude Code API configuration..."
echo ""

# 检查环境变量
echo "📋 Environment variables:"
echo "RUN_MODE=$RUN_MODE"
echo "CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=$CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS"
echo "CLAUDE_CODE__PROCESS_POOL__SIZE=$CLAUDE_CODE__PROCESS_POOL__SIZE"
echo ""

# 检查配置文件
echo "📄 Config files:"
if [ -f "config/fast.toml" ]; then
    echo "config/fast.toml exists:"
    grep -E "use_interactive_sessions|size|min_idle" config/fast.toml
else
    echo "config/fast.toml not found"
fi
echo ""

if [ -f "config/optimized.toml" ]; then
    echo "config/optimized.toml exists:"
    grep -E "use_interactive_sessions|size|min_idle" config/optimized.toml
else
    echo "config/optimized.toml not found"
fi
echo ""

# 测试健康检查
echo "🏥 Testing health endpoint:"
curl -s http://localhost:8080/health || echo "Server not running"
echo ""
echo ""

# 测试统计信息
echo "📊 Stats endpoint:"
curl -s http://localhost:8080/stats | jq . 2>/dev/null || echo "Failed to get stats"
echo ""

# 简单的测试请求
echo "🧪 Test request with conversation_id:"
RESPONSE=$(curl -s -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "conversation_id": "debug-test-123",
    "messages": [{"role": "user", "content": "Say hello"}]
  }')

if [ -n "$RESPONSE" ]; then
    echo "Response received:"
    echo "$RESPONSE" | jq '{conversation_id, model, usage}' 2>/dev/null || echo "$RESPONSE"
else
    echo "No response received"
fi