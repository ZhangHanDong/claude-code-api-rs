# API 测试指南

本指南展示如何测试 Claude Code SDK 的各种 API 模式。

## 快速开始

### 1. 运行简单示例（无需 Claude CLI）

```bash
# 运行模拟 API 示例
cargo run --example simple_api_demo

# 输出将显示各种模式的演示
```

### 2. 启动 REST API 服务器

```bash
# 以模拟模式启动（默认，无需 Claude CLI）
MOCK_MODE=true cargo run --example rest_api_server

# 以真实模式启动（需要 Claude CLI）
MOCK_MODE=false cargo run --example rest_api_server
```

服务器将在 `http://localhost:3000` 启动。

## 使用 Curl 测试

### 基础测试

```bash
# 1. 健康检查
curl http://localhost:3000/health

# 2. 简单查询
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{"prompt": "What is 2 + 2?"}'

# 3. 批量处理
curl -X POST http://localhost:3000/batch \
  -H "Content-Type: application/json" \
  -d '{
    "prompts": ["What is 1 + 1?", "What is 2 + 2?", "What is 3 + 3?"],
    "max_concurrent": 3
  }'

# 4. 查看指标
curl http://localhost:3000/metrics
```

### 使用测试脚本

```bash
# 运行完整的 curl 测试套件
./test_with_curl.sh
```

## API 端点详解

### 1. **GET /health**
健康检查端点

**响应示例：**
```json
{
  "status": "ok",
  "mode": "mock",
  "mock": true
}
```

### 2. **POST /query**
单次查询端点

**请求体：**
```json
{
  "prompt": "Your question here",
  "mode": "oneshot"  // 可选
}
```

**响应示例：**
```json
{
  "success": true,
  "message": "The answer is 4.",
  "error": null,
  "duration_ms": 100
}
```

### 3. **POST /batch**
批量处理端点

**请求体：**
```json
{
  "prompts": ["Question 1", "Question 2", "Question 3"],
  "max_concurrent": 5  // 可选，默认为 5
}
```

**响应示例：**
```json
{
  "success": true,
  "results": [
    {
      "success": true,
      "message": "Answer 1",
      "error": null,
      "duration_ms": 50
    },
    {
      "success": true,
      "message": "Answer 2",
      "error": null,
      "duration_ms": 45
    }
  ],
  "total_duration_ms": 150
}
```

### 4. **GET /metrics**
性能指标端点

**响应示例：**
```json
{
  "total_requests": 100,
  "successful_requests": 95,
  "failed_requests": 5,
  "success_rate": 0.95,
  "average_latency_ms": 120.5,
  "min_latency_ms": 50,
  "max_latency_ms": 500
}
```

## 高级测试场景

### 1. 性能测试

```bash
# 使用 Apache Bench (ab)
ab -n 100 -c 10 -p query.json -T application/json http://localhost:3000/query

# 其中 query.json 包含：
# {"prompt": "What is 2 + 2?"}
```

### 2. 并发测试

```bash
# 并发发送 10 个请求
for i in {1..10}; do
  curl -X POST http://localhost:3000/query \
    -H "Content-Type: application/json" \
    -d "{\"prompt\": \"What is $i squared?\"}" &
done
wait
```

### 3. 负载测试

```bash
# 使用 wrk 进行负载测试
wrk -t4 -c100 -d30s -s post.lua http://localhost:3000/query

# post.lua 内容：
# wrk.method = "POST"
# wrk.body   = '{"prompt": "What is 2 + 2?"}'
# wrk.headers["Content-Type"] = "application/json"
```

## 使用 Postman 或 Insomnia

### Postman 集合

创建以下请求：

1. **Health Check**
   - Method: GET
   - URL: http://localhost:3000/health

2. **Simple Query**
   - Method: POST
   - URL: http://localhost:3000/query
   - Body (JSON):
     ```json
     {
       "prompt": "What is the capital of France?"
     }
     ```

3. **Batch Query**
   - Method: POST
   - URL: http://localhost:3000/batch
   - Body (JSON):
     ```json
     {
       "prompts": [
         "What is 1 + 1?",
         "What is 2 + 2?",
         "What is 3 + 3?"
       ],
       "max_concurrent": 3
     }
     ```

## 测试不同模式

### OneShot 模式
最适合单次查询：

```bash
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Explain quantum computing"}'
```

### Batch 模式
最适合批量处理：

```bash
curl -X POST http://localhost:3000/batch \
  -H "Content-Type: application/json" \
  -d '{
    "prompts": [
      "Task 1: Write a haiku",
      "Task 2: Solve 10 * 20",
      "Task 3: List 3 colors"
    ],
    "max_concurrent": 2
  }'
```

## 错误处理测试

### 测试空请求
```bash
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{}'
```

### 测试大批量请求
```bash
# 生成 100 个查询
prompts=$(python3 -c "import json; print(json.dumps([f'What is {i} squared?' for i in range(100)]))")

curl -X POST http://localhost:3000/batch \
  -H "Content-Type: application/json" \
  -d "{\"prompts\": $prompts, \"max_concurrent\": 10}"
```

## 监控和调试

### 查看实时日志
```bash
# 启动服务器时启用详细日志
RUST_LOG=debug cargo run --example rest_api_server
```

### 监控指标
```bash
# 持续监控指标
watch -n 1 'curl -s http://localhost:3000/metrics | jq .'
```

## 常见问题

1. **连接被拒绝**
   - 确保服务器正在运行
   - 检查端口 3000 是否被占用

2. **JSON 解析错误**
   - 确保请求体是有效的 JSON
   - 使用 Content-Type: application/json

3. **超时错误**
   - 增加客户端超时时间
   - 检查服务器负载

## 完整测试流程

```bash
# 1. 启动服务器
MOCK_MODE=true cargo run --example rest_api_server &
SERVER_PID=$!

# 2. 等待服务器启动
sleep 2

# 3. 运行测试
./test_with_curl.sh

# 4. 停止服务器
kill $SERVER_PID
```