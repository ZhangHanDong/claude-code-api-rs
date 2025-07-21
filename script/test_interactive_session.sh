#!/bin/bash

# 测试交互式会话功能
# 同一个会话应该复用相同的进程

echo "🧪 Testing interactive session functionality..."

# 生成会话ID
CONVERSATION_ID="test-session-$(date +%s)"
API_URL="http://localhost:8080/v1/chat/completions"

echo "📝 Using conversation ID: $CONVERSATION_ID"

# 第一个请求
echo "📤 Sending first request..."
START_TIME1=$(date +%s)
RESPONSE1=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"conversation_id\": \"$CONVERSATION_ID\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"Hello! Please remember the number 42. I'll ask about it later.\"
        }]
    }")
END_TIME1=$(date +%s)
TIME1=$((END_TIME1 - START_TIME1))

echo "⏱️  First request took: ${TIME1}s"
echo "Response excerpt: $(echo "$RESPONSE1" | jq -r '.choices[0].message.content' | head -c 100)..."

# 等待一秒
sleep 1

# 第二个请求 - 应该使用相同的会话
echo "📤 Sending second request to same session..."
START_TIME2=$(date +%s)
RESPONSE2=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"conversation_id\": \"$CONVERSATION_ID\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"What number did I ask you to remember?\"
        }]
    }")
END_TIME2=$(date +%s)
TIME2=$((END_TIME2 - START_TIME2))

echo "⏱️  Second request took: ${TIME2}s"
echo "Response excerpt: $(echo "$RESPONSE2" | jq -r '.choices[0].message.content' | head -c 100)..."

# 分析结果
echo ""
echo "📊 Analysis:"
if [ $TIME2 -lt $TIME1 ]; then
    echo "✅ Second request was faster ($TIME2s vs $TIME1s) - session reuse likely working!"
else
    echo "⚠️  Second request was not faster ($TIME2s vs $TIME1s)"
fi

# 检查是否记住了数字
if echo "$RESPONSE2" | grep -q "42"; then
    echo "✅ Claude remembered the number - session context maintained!"
else
    echo "❌ Claude didn't remember the number - session context might be lost"
fi

# 测试不同会话
echo ""
echo "📤 Testing different conversation ID..."
CONVERSATION_ID2="test-session-$(date +%s)-2"
START_TIME3=$(date +%s)
RESPONSE3=$(curl -s -X POST $API_URL \
    -H "Content-Type: application/json" \
    -d "{
        \"model\": \"claude-3-5-sonnet-20241022\",
        \"conversation_id\": \"$CONVERSATION_ID2\",
        \"messages\": [{
            \"role\": \"user\",
            \"content\": \"What number did I ask you to remember?\"
        }]
    }")
END_TIME3=$(date +%s)
TIME3=$((END_TIME3 - START_TIME3))

echo "⏱️  New session request took: ${TIME3}s"

if echo "$RESPONSE3" | grep -q "42"; then
    echo "❌ New session knew the number - sessions might be mixing!"
else
    echo "✅ New session didn't know the number - sessions are properly isolated!"
fi

echo ""
echo "🎉 Test complete!"