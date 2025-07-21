# 文件访问权限指南

Claude Code API 现在支持完整的本地文件操作。本指南将说明如何配置和使用文件访问功能。

## 文件操作能力

通过 API，你可以让 Claude 执行以下文件操作：

1. **读取文件** - 读取任何文本文件内容
2. **创建文件** - 创建新文件并写入内容
3. **修改文件** - 编辑现有文件
4. **删除文件** - 删除不需要的文件
5. **搜索文件** - 在目录中搜索特定文件
6. **执行命令** - 运行系统命令

## 配置方式

### 方式1：使用环境变量（推荐用于开发）

最简单的方式是设置环境变量来跳过权限检查：

```bash
export CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS=true
./target/release/claude-code-api
```

### 方式2：添加特定目录权限（推荐用于生产）

通过环境变量指定允许访问的目录：

```bash
export CLAUDE_CODE__FILE_ACCESS__ADDITIONAL_DIRS='["/Users/zhangalex/Work", "/tmp"]'
./target/release/claude-code-api
```

### 方式3：使用配置文件

创建 `config/local.toml`：

```toml
[file_access]
skip_permissions = false
additional_dirs = [
    "/Users/zhangalex/Work",
    "/Users/zhangalex/Documents",
    "/tmp"
]
```

### 方式4：使用启动脚本

使用提供的启动脚本：

```bash
./start_with_permissions.sh
```

## 使用示例

### 读取文件

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "请读取 /path/to/file.txt 并告诉我文件内容"
    }]
  }'
```

### 创建文件

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "请在 /tmp 目录创建一个 hello.txt 文件，内容为 Hello World"
    }]
  }'
```

### 修改文件

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "请修改 /path/to/config.json，将 port 值改为 8080"
    }]
  }'
```

### 搜索文件

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "在 /Users/zhangalex/Work 目录下搜索所有 .md 文件"
    }]
  }'
```

### 执行命令

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "执行 ls -la 命令并告诉我结果"
    }]
  }'
```

## Python 客户端示例

```python
import requests

def claude_file_operation(prompt):
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

# 读取文件
result = claude_file_operation("读取 /etc/hosts 文件内容")
print(result['choices'][0]['message']['content'])

# 创建文件
result = claude_file_operation("在 /tmp 创建 test.py，内容为一个简单的 Hello World 程序")

# 分析代码
result = claude_file_operation("分析 /path/to/project 目录下的所有 Python 文件结构")
```

## 安全考虑

1. **生产环境**：不要使用 `skip_permissions`，而是明确指定允许的目录
2. **敏感文件**：避免授予对系统敏感目录的访问权限
3. **日志记录**：所有文件操作都会被记录在日志中
4. **权限继承**：Claude 使用启动用户的文件系统权限

## 故障排查

### 权限被拒绝

如果看到 "I need permission to write the file" 错误：

1. 确保设置了正确的环境变量
2. 检查目录是否在允许列表中
3. 确认用户有该目录的写权限

### 文件内容被截断

如果文件内容没有完全显示：

1. Claude 可能总结了长文件
2. 可以要求 Claude "显示文件的完整内容"
3. 或要求 "显示文件的前100行"

## 高级用法

### 批量文件操作

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "将 /source 目录下所有 .txt 文件复制到 /destination 目录"
    }]
  }'
```

### 代码重构

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "重构 /path/to/project 中的所有 Python 文件，将 print 语句改为 logging"
    }]
  }'
```

### 项目分析

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-20250514",
    "messages": [{
      "role": "user",
      "content": "分析 /path/to/project 的代码结构并生成架构文档"
    }]
  }'
```

## 环境变量参考

- `CLAUDE_CODE__FILE_ACCESS__SKIP_PERMISSIONS`: 设为 "true" 跳过所有权限检查
- `CLAUDE_CODE__FILE_ACCESS__ADDITIONAL_DIRS`: JSON 数组格式的目录列表
- `CLAUDE_SKIP_PERMISSIONS`: 临时环境变量，设为 "true" 跳过权限（已废弃）