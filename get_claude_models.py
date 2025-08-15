#!/usr/bin/env python3
"""
获取 Claude Code 当前支持的模型列表
"""

import subprocess
import json
import asyncio
from typing import List, Dict, Any

async def get_available_models_from_cli() -> List[str]:
    """通过 Claude CLI 获取可用模型"""
    # 常见的模型别名（2025年）
    model_aliases = [
        "opus",      # Opus 4.1
        "sonnet",    # Sonnet 4
        "haiku",     # Haiku
        "opus-4",
        "opus-4.1", 
        "sonnet-4",
        "claude-opus-4-1-20250805",  # Opus 4.1 完整名称
        "claude-sonnet-4-20250514",  # Sonnet 4 完整名称
    ]
    
    available_models = []
    
    for model in model_aliases:
        try:
            # 尝试使用该模型运行一个简单的查询
            result = subprocess.run(
                ["claude", "--model", model, "--print", "Say 'ok' if this model works"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0 and "ok" in result.stdout.lower():
                available_models.append(model)
                print(f"✓ Model available: {model}")
            else:
                print(f"✗ Model not available: {model}")
        except subprocess.TimeoutExpired:
            print(f"⚠ Timeout testing model: {model}")
        except Exception as e:
            print(f"⚠ Error testing model {model}: {e}")
    
    return available_models

async def get_models_via_sdk():
    """通过 Claude SDK 获取模型信息"""
    try:
        from claude_code_sdk import query, ClaudeCodeOptions
        
        # 测试不同的模型
        test_models = [
            "opus-4.1",
            "opus-4",
            "sonnet-4", 
            "sonnet",
            "claude-opus-4-1-20250805",
            "claude-sonnet-4-20250514",
        ]
        
        working_models = []
        
        for model in test_models:
            try:
                options = ClaudeCodeOptions(
                    model=model,
                    max_turns=1
                )
                
                # 尝试一个简单查询
                response_received = False
                async for message in query(prompt="Reply with 'ok'", options=options):
                    response_received = True
                    break  # 只要能收到响应就说明模型可用
                
                if response_received:
                    working_models.append(model)
                    print(f"✓ SDK: Model {model} is working")
                else:
                    print(f"✗ SDK: Model {model} did not respond")
                    
            except Exception as e:
                print(f"✗ SDK: Model {model} failed: {str(e)[:100]}")
        
        return working_models
        
    except ImportError:
        print("Claude Code SDK not installed. Install with: pip install claude-code-sdk")
        return []

async def check_api_models():
    """检查 API 端点返回的模型"""
    import requests
    
    try:
        response = requests.get("http://localhost:8080/v1/models")
        if response.status_code == 200:
            models = response.json()
            print("\n=== API Models ===")
            for model in models.get("data", []):
                print(f"  - {model['id']}")
            return [m['id'] for m in models.get("data", [])]
    except Exception as e:
        print(f"Could not connect to API: {e}")
    
    return []

async def main():
    """主函数"""
    print("=== Checking Claude Code Models (2025) ===\n")
    
    # 1. 通过 CLI 检查
    print("1. Checking via Claude CLI...")
    cli_models = await get_available_models_from_cli()
    
    # 2. 通过 SDK 检查
    print("\n2. Checking via Python SDK...")
    sdk_models = await get_models_via_sdk()
    
    # 3. 通过 API 检查
    print("\n3. Checking via API endpoint...")
    api_models = await check_api_models()
    
    # 汇总结果
    print("\n=== Summary ===")
    print(f"CLI available models: {cli_models}")
    print(f"SDK working models: {sdk_models}")
    print(f"API listed models: {api_models}")
    
    # 推荐使用的模型名称（2025年）
    print("\n=== Recommended Model Names for 2025 ===")
    print("• Opus 4.1: 'opus-4.1' or 'claude-opus-4-1-20250805'")
    print("• Sonnet 4: 'sonnet-4' or 'claude-sonnet-4-20250514'")
    print("• Latest: Use 'opus' or 'sonnet' for latest versions")
    
    return {
        "cli": cli_models,
        "sdk": sdk_models,
        "api": api_models
    }

if __name__ == "__main__":
    models = asyncio.run(main())