#!/bin/bash

# 简单的交互式会话测试

echo "🧪 Testing interactive session functionality (simple version)..."

API_URL="http://localhost:8080/v1/chat/completions"

# 先检查服务器是否运行
echo "🔍 Checking if server is running..."
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "❌ Server is not running at http://localhost:8080"
    echo "Please start the server first with: ./start_fast.sh"
    exit 1
fi
echo "✅ Server is running"

# 生成会话ID
CONVERSATION_ID="test-session-$(date +%s)"

echo "📝 Using conversation ID: $CONVERSATION_ID"

# 第一个请求 - 让 Claude 记住一个故事设定
echo "📤 Sending first request..."
START_TIME1=$(date +%s%N)
RESPONSE1=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"conversation_id\": \"$CONVERSATION_ID\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"Let's play a game. I'm a pirate captain and my ship is called 'The Golden Seagull'. What's my ship's name?\"
        }]
    }")
END_TIME1=$(date +%s%N)
TIME1=$(( (END_TIME1 - START_TIME1) / 1000000 ))

# 检查响应
if [ -z "$RESPONSE1" ]; then
    echo "❌ No response from server"
    exit 1
fi

# 检查是否有错误
if echo "$RESPONSE1" | grep -q "error"; then
    echo "❌ Server returned error:"
    echo "$RESPONSE1" | jq .
    exit 1
fi

echo "⏱️  First request took: ${TIME1}ms"
CONTENT1=$(echo "$RESPONSE1" | jq -r '.choices[0].message.content' 2>/dev/null || echo "Failed to parse response")
echo "Response: ${CONTENT1:0:200}..."

# 等待一秒
sleep 1

# 第二个请求 - 询问船的名字
echo ""
echo "📤 Sending second request to same session..."
START_TIME2=$(date +%s%N)
RESPONSE2=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"conversation_id\": \"$CONVERSATION_ID\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"What's the name of my pirate ship again?\"
        }]
    }")
END_TIME2=$(date +%s%N)
TIME2=$(( (END_TIME2 - START_TIME2) / 1000000 ))

echo "⏱️  Second request took: ${TIME2}ms"
CONTENT2=$(echo "$RESPONSE2" | jq -r '.choices[0].message.content' 2>/dev/null || echo "Failed to parse response")
echo "Response: ${CONTENT2:0:200}..."

# 分析结果
echo ""
echo "📊 Analysis:"
if [ $TIME2 -lt $TIME1 ]; then
    SPEEDUP=$(awk "BEGIN {printf \"%.1f\", $TIME1/$TIME2}")
    echo "✅ Second request was faster (${TIME2}ms vs ${TIME1}ms) - ${SPEEDUP}x speedup!"
else
    echo "⚠️  Second request was not faster (${TIME2}ms vs ${TIME1}ms)"
fi

# 检查是否记住了船名
if echo "$CONTENT2" | grep -qi "golden seagull"; then
    echo "✅ Claude remembered the ship name - session context maintained!"
else
    echo "❌ Claude didn't remember the ship name - check if sessions are working"
    echo "Full response: $CONTENT2"
fi

# 测试不使用 conversation_id 的请求
echo ""
echo "📤 Testing request without conversation_id (should be slower)..."
START_TIME3=$(date +%s%N)
RESPONSE3=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"Hello, how are you?\"
        }]
    }")
END_TIME3=$(date +%s%N)
TIME3=$(( (END_TIME3 - START_TIME3) / 1000000 ))

echo "⏱️  No conversation ID request took: ${TIME3}ms"

echo ""
echo "🎉 Test complete!"
echo "Summary: First request: ${TIME1}ms, Second request (same session): ${TIME2}ms, New session: ${TIME3}ms"

# 显示调试信息
echo ""
echo "📊 Debug info:"
echo "First response conversation_id: $(echo "$RESPONSE1" | jq -r '.conversation_id' 2>/dev/null)"
echo "Second response conversation_id: $(echo "$RESPONSE2" | jq -r '.conversation_id' 2>/dev/null)"
echo "Third response conversation_id: $(echo "$RESPONSE3" | jq -r '.conversation_id' 2>/dev/null)"