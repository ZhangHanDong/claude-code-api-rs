#!/bin/bash

# 测试多模态功能的脚本

echo "=== 测试多模态 API ==="

# 测试1: 使用本地文件路径
echo -e "\n1. 测试本地文件路径:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [
        {
          "role": "user",
          "content": [
            {"type": "text", "text": "描述这张图片"},
            {"type": "image_url", "image_url": {"url": "/tmp/test_image.png"}}
          ]
        }
      ]
    }' | jq .

# 测试2: 使用远程图片 URL
echo -e "\n2. 测试远程图片 URL:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [
        {
          "role": "user",
          "content": [
            {"type": "text", "text": "这是什么标志？"},
            {"type": "image_url", "image_url": {"url": "https://www.google.com/images/branding/googlelogo/2x/googlelogo_color_92x30dp.png"}}
          ]
        }
      ]
    }' | jq .

# 测试3: 使用 base64 编码的图片 (创建一个简单的1x1像素红色图片)
echo -e "\n3. 测试 base64 图片:"
BASE64_IMAGE="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="

curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d "{
      \"model\": \"claude-opus-4-20250514\",
      \"messages\": [
        {
          \"role\": \"user\",
          \"content\": [
            {\"type\": \"text\", \"text\": \"这是什么颜色的像素？\"},
            {\"type\": \"image_url\", \"image_url\": {\"url\": \"$BASE64_IMAGE\"}}
          ]
        }
      ]
    }" | jq .

# 测试4: 多图片测试
echo -e "\n4. 测试多张图片:"
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
      "model": "claude-opus-4-20250514",
      "messages": [
        {
          "role": "user",
          "content": [
            {"type": "text", "text": "比较这两张图片"},
            {"type": "image_url", "image_url": {"url": "/tmp/test1.png"}},
            {"type": "image_url", "image_url": {"url": "/tmp/test2.png"}}
          ]
        }
      ]
    }' | jq .

echo -e "\n=== 测试完成 ==="