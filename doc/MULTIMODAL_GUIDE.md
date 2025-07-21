# 多模态 API 使用指南

Claude Code API 现在支持在对话中包含图片。本指南将介绍如何使用这个功能。

## 支持的图片格式

1. **本地文件路径** - 直接指定本地图片文件的路径
2. **远程 URL** - HTTP/HTTPS 图片链接
3. **Base64 编码** - Data URL 格式的 base64 编码图片

## API 格式

使用 OpenAI 兼容的格式，在 `messages` 中的 `content` 字段使用数组格式：

```json
{
  "model": "claude-opus-4-20250514",
  "messages": [
    {
      "role": "user",
      "content": [
        {"type": "text", "text": "你的文本消息"},
        {"type": "image_url", "image_url": {"url": "图片URL或路径"}}
      ]
    }
  ]
}
```

## 使用示例

### 1. 本地文件

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "这张图片显示了什么？"},
        {"type": "image_url", "image_url": {"url": "/Users/you/pictures/screenshot.png"}}
      ]
    }]
  }'
```

### 2. 远程 URL

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "分析这个网站的 logo"},
        {"type": "image_url", "image_url": {"url": "https://example.com/logo.png"}}
      ]
    }]
  }'
```

### 3. Base64 编码

```bash
# 首先将图片转换为 base64
IMAGE_BASE64=$(base64 -i image.png)

curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d "{
    \"model\": \"claude-opus-4-20250514\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": [
        {\"type\": \"text\", \"text\": \"这是什么图片？\"},
        {\"type\": \"image_url\", \"image_url\": {\"url\": \"data:image/png;base64,$IMAGE_BASE64\"}}
      ]
    }]
  }"
```

### 4. 多张图片

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "比较这两张图片的不同"},
        {"type": "image_url", "image_url": {"url": "/path/to/image1.png"}},
        {"type": "image_url", "image_url": {"url": "/path/to/image2.png"}}
      ]
    }]
  }'
```

### 5. 会话中使用图片

```bash
# 第一次请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "记住这张图片"},
        {"type": "image_url", "image_url": {"url": "/path/to/image.png"}}
      ]
    }]
  }'

# 获取 conversation_id 后，在后续请求中使用
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "conversation_id": "your-conversation-id",
    "messages": [{
      "role": "user",
      "content": "刚才的图片中有什么颜色？"
    }]
  }'
```

## 实现细节

1. **自动下载和缓存**：远程图片会被自动下载到临时目录
2. **Base64 解码**：Base64 编码的图片会被自动解码并保存为临时文件
3. **自动清理**：临时文件会在 15 分钟后自动清理
4. **Claude CLI 集成**：图片路径会被传递给 Claude CLI 进行处理

## 注意事项

1. 确保 Claude CLI 有权限访问图片文件路径
2. 远程图片下载可能需要时间，建议使用本地文件以获得最佳性能
3. Base64 编码的图片会增加请求大小，建议用于小图片
4. 临时文件存储在系统临时目录中（通常是 `/tmp` 或 `/var/folders`）

## 错误处理

如果图片处理失败，API 会返回相应的错误信息：

- `400 Bad Request`: 无效的图片格式或 URL
- `500 Internal Error`: 下载或处理图片时发生错误

## Python 客户端示例

```python
import requests
import base64

# 使用本地文件
def chat_with_image(text, image_path):
    response = requests.post(
        "http://localhost:8080/v1/chat/completions",
        json={
            "model": "claude-opus-4-20250514",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": text},
                    {"type": "image_url", "image_url": {"url": image_path}}
                ]
            }]
        }
    )
    return response.json()

# 使用 base64 编码
def chat_with_base64_image(text, image_path):
    with open(image_path, "rb") as f:
        image_base64 = base64.b64encode(f.read()).decode()
    
    response = requests.post(
        "http://localhost:8080/v1/chat/completions",
        json={
            "model": "claude-opus-4-20250514",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": text},
                    {"type": "image_url", "image_url": {
                        "url": f"data:image/png;base64,{image_base64}"
                    }}
                ]
            }]
        }
    )
    return response.json()
```

## 性能建议

1. 对于本地图片，直接使用文件路径最快
2. 避免重复上传相同的图片，利用会话功能
3. 压缩大图片以减少处理时间
4. 使用适当的图片格式（PNG、JPEG）