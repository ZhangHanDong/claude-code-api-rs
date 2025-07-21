# MCP (Model Context Protocol) 使用指南

Claude Code API 现在支持 MCP，允许 Claude 通过标准化协议访问外部工具和服务。

## 什么是 MCP？

MCP (Model Context Protocol) 是一个标准化协议，让 AI 助手能够安全地与外部系统交互。通过 MCP，Claude 可以：

- 访问文件系统（更细粒度的权限控制）
- 与 GitHub 仓库交互
- 查询数据库
- 访问云存储服务
- 调用自定义工具

## 配置方式

### 1. 使用配置文件（推荐）

创建 `mcp_config.json` 文件：

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/allowed/path"],
      "env": {}
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "your-token"
      }
    }
  }
}
```

然后启动服务：

```bash
export CLAUDE_CODE__MCP__ENABLED=true
export CLAUDE_CODE__MCP__CONFIG_FILE="./mcp_config.json"
./target/release/claude-code-api
```

### 2. 使用环境变量

```bash
export CLAUDE_CODE__MCP__ENABLED=true
export CLAUDE_CODE__MCP__CONFIG_JSON='{"mcpServers":{"filesystem":{"command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","/tmp"]}}}'
./target/release/claude-code-api
```

### 3. 使用启动脚本

```bash
./start_with_mcp.sh
```

## 可用的 MCP 服务器

### 1. 文件系统服务器

提供对指定目录的读写访问：

```json
{
  "filesystem": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path1", "/path2"],
    "env": {}
  }
}
```

使用示例：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "使用 MCP 文件系统服务器列出 /tmp 目录的内容"
    }]
  }'
```

### 2. GitHub 服务器

访问 GitHub 仓库和 API：

```json
{
  "github": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-github"],
    "env": {
      "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_your_token"
    }
  }
}
```

使用示例：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "使用 GitHub MCP 获取 anthropics/claude-code 的最新提交"
    }]
  }'
```

### 3. SQLite 服务器

查询 SQLite 数据库：

```json
{
  "sqlite": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-sqlite", "/path/to/database.db"],
    "env": {}
  }
}
```

使用示例：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "查询数据库中的用户表"
    }]
  }'
```

### 4. Google Drive 服务器

访问 Google Drive 文件：

```json
{
  "google-drive": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-gdrive"],
    "env": {
      "GOOGLE_CLIENT_ID": "your-client-id",
      "GOOGLE_CLIENT_SECRET": "your-client-secret"
    }
  }
}
```

## 高级配置

### 严格模式

只使用 MCP 配置中定义的服务器，忽略其他配置：

```bash
export CLAUDE_CODE__MCP__STRICT=true
```

### 调试模式

显示 MCP 服务器的详细错误信息：

```bash
export CLAUDE_CODE__MCP__DEBUG=true
```

## 自定义 MCP 服务器

你可以创建自定义的 MCP 服务器来扩展 Claude 的能力。MCP 服务器需要实现以下接口：

1. **列出工具** - 返回可用的工具列表
2. **执行工具** - 执行特定的工具调用
3. **列出资源** - 可选，返回可用资源
4. **读取资源** - 可选，读取特定资源

示例自定义服务器配置：

```json
{
  "my-custom-server": {
    "command": "python",
    "args": ["/path/to/my_mcp_server.py"],
    "env": {
      "API_KEY": "your-api-key"
    }
  }
}
```

## 安全考虑

1. **最小权限原则**：只授予必要的访问权限
2. **令牌安全**：使用环境变量存储敏感令牌
3. **路径限制**：文件系统服务器应限制在特定目录
4. **审计日志**：所有 MCP 调用都会被记录

## 故障排查

### MCP 服务器未响应

1. 检查 MCP 服务器是否正确安装：
   ```bash
   npx -y @modelcontextprotocol/server-filesystem --version
   ```

2. 启用调试模式查看详细错误：
   ```bash
   export CLAUDE_CODE__MCP__DEBUG=true
   ```

3. 检查配置文件格式是否正确

### 权限被拒绝

1. 确保 MCP 服务器有必要的权限
2. 检查环境变量和令牌是否正确设置
3. 对于文件系统，确保路径在允许列表中

## 实际使用案例

### 1. 代码审查助手

```bash
# 配置 GitHub MCP 服务器后
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "审查 owner/repo PR #123 的代码变更"
    }]
  }'
```

### 2. 数据分析

```bash
# 配置 SQLite MCP 服务器后
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "分析销售数据库中上个月的销售趋势"
    }]
  }'
```

### 3. 文档管理

```bash
# 配置 Google Drive MCP 服务器后
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "整理我的项目文档并创建索引"
    }]
  }'
```

## Python 客户端示例

```python
import requests

def claude_mcp_request(prompt):
    response = requests.post(
        "http://localhost:8080/v1/chat/completions",
        json={
            "model": "claude-opus-4-20250514",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }
    )
    return response.json()

# 使用 MCP 文件系统
result = claude_mcp_request("使用 MCP 创建一个项目结构模板")

# 使用 MCP GitHub
result = claude_mcp_request("获取我的 GitHub 仓库列表")

# 使用 MCP 数据库
result = claude_mcp_request("创建一个用户管理数据库架构")
```

## 性能优化

1. **服务器复用**：MCP 服务器会被复用，避免重复启动
2. **连接池**：保持与 MCP 服务器的持久连接
3. **缓存**：常用查询结果会被缓存

## 更多资源

- [MCP 协议规范](https://modelcontextprotocol.io)
- [MCP 服务器列表](https://github.com/modelcontextprotocol)
- [创建自定义 MCP 服务器](https://modelcontextprotocol.io/docs/server)