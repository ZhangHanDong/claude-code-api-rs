#!/usr/bin/env python3
"""
使用 Claude Code SDK Python 的最新模型（2025年）
"""

import asyncio
from claude_code_sdk import query, ClaudeCodeOptions

async def use_opus_4_1():
    """使用 Opus 4.1 模型"""
    print("=== Using Opus 4.1 ===")
    
    # 方式1：使用别名
    options = ClaudeCodeOptions(model="opus-4.1")
    
    # 方式2：使用完整名称
    # options = ClaudeCodeOptions(model="claude-opus-4-1-20250805")
    
    # 方式3：使用最新的 opus
    # options = ClaudeCodeOptions(model="opus")
    
    async for message in query(
        prompt="What model are you using? Please specify your exact model version.",
        options=options
    ):
        print(message)

async def use_sonnet_4():
    """使用 Sonnet 4 模型"""
    print("\n=== Using Sonnet 4 ===")
    
    # 方式1：使用别名
    options = ClaudeCodeOptions(model="sonnet-4")
    
    # 方式2：使用完整名称
    # options = ClaudeCodeOptions(model="claude-sonnet-4-20250514")
    
    # 方式3：使用最新的 sonnet
    # options = ClaudeCodeOptions(model="sonnet")
    
    async for message in query(
        prompt="What model are you using? Please specify your exact model version.",
        options=options
    ):
        print(message)

async def list_and_test_models():
    """列出并测试可用的模型"""
    
    # 2025年的模型列表
    models_to_test = [
        ("opus-4.1", "Opus 4.1 - 最强大的模型"),
        ("sonnet-4", "Sonnet 4 - 平衡性能和速度"),
        ("opus", "最新的 Opus 模型"),
        ("sonnet", "最新的 Sonnet 模型"),
        ("haiku", "最新的 Haiku 模型 - 快速轻量"),
    ]
    
    print("=== Testing Available Models ===\n")
    
    working_models = []
    
    for model_name, description in models_to_test:
        print(f"Testing {model_name} ({description})...")
        
        try:
            options = ClaudeCodeOptions(
                model=model_name,
                max_turns=1
            )
            
            messages = []
            async for msg in query(
                prompt="Say 'OK' if you're working",
                options=options
            ):
                messages.append(msg)
            
            if messages:
                working_models.append(model_name)
                print(f"  ✓ {model_name} is working!\n")
            else:
                print(f"  ✗ {model_name} - no response\n")
                
        except Exception as e:
            print(f"  ✗ {model_name} - error: {str(e)[:100]}\n")
    
    print(f"\nWorking models: {working_models}")
    return working_models

async def interactive_model_selection():
    """交互式选择模型"""
    print("=== Interactive Model Selection ===\n")
    
    models = {
        "1": ("opus-4.1", "Opus 4.1 - Most powerful (2025)"),
        "2": ("sonnet-4", "Sonnet 4 - Balanced (2025)"),  
        "3": ("opus", "Latest Opus"),
        "4": ("sonnet", "Latest Sonnet"),
        "5": ("haiku", "Latest Haiku - Fast"),
    }
    
    print("Available models:")
    for key, (model, desc) in models.items():
        print(f"  {key}. {desc} ({model})")
    
    choice = input("\nSelect a model (1-5): ").strip()
    
    if choice in models:
        model_name, _ = models[choice]
        print(f"\nUsing model: {model_name}\n")
        
        options = ClaudeCodeOptions(model=model_name)
        
        prompt = input("Enter your prompt: ")
        
        async for message in query(prompt=prompt, options=options):
            print(message)
    else:
        print("Invalid choice")

async def main():
    """主函数"""
    import sys
    
    if len(sys.argv) > 1:
        if sys.argv[1] == "opus":
            await use_opus_4_1()
        elif sys.argv[1] == "sonnet":
            await use_sonnet_4()
        elif sys.argv[1] == "test":
            await list_and_test_models()
        elif sys.argv[1] == "interactive":
            await interactive_model_selection()
        else:
            print(f"Unknown command: {sys.argv[1]}")
            print("Usage: python use_latest_models.py [opus|sonnet|test|interactive]")
    else:
        # 默认测试所有模型
        await list_and_test_models()

if __name__ == "__main__":
    asyncio.run(main())