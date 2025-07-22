#!/usr/bin/env python3
"""测试并发请求处理"""

import asyncio
import aiohttp
import json
import time
from typing import List, Dict, Any


async def send_request(session: aiohttp.ClientSession, request_id: int, conversation_id: str = None) -> Dict[str, Any]:
    """发送单个请求"""
    url = "http://localhost:8080/v1/chat/completions"
    
    payload = {
        "model": "claude-opus-4-20250514",
        "messages": [{
            "role": "user",
            "content": f"请回答：{request_id} + {request_id} = ?"
        }],
        "stream": False
    }
    
    if conversation_id:
        payload["conversation_id"] = conversation_id
    
    start_time = time.time()
    
    try:
        async with session.post(url, json=payload) as response:
            result = await response.json()
            elapsed = time.time() - start_time
            
            if response.status == 200:
                content = result['choices'][0]['message']['content']
                conv_id = result.get('conversation_id', 'N/A')
                print(f"[请求 {request_id}] 成功 (耗时: {elapsed:.2f}s) - 会话ID: {conv_id}")
                print(f"  响应: {content[:100]}...")
                
                # 检查响应是否正确
                expected = str(request_id * 2)
                if expected in content:
                    print(f"  ✓ 响应正确包含预期答案: {expected}")
                else:
                    print(f"  ✗ 响应错误！预期包含: {expected}")
                    
                return {
                    "request_id": request_id,
                    "success": True,
                    "elapsed": elapsed,
                    "conversation_id": conv_id,
                    "correct": expected in content
                }
            else:
                print(f"[请求 {request_id}] 失败 (状态码: {response.status})")
                print(f"  错误: {result}")
                return {
                    "request_id": request_id,
                    "success": False,
                    "elapsed": elapsed,
                    "error": result
                }
    except Exception as e:
        elapsed = time.time() - start_time
        print(f"[请求 {request_id}] 异常: {e} (耗时: {elapsed:.2f}s)")
        return {
            "request_id": request_id,
            "success": False,
            "elapsed": elapsed,
            "error": str(e)
        }


async def test_concurrent_new_conversations(num_requests: int):
    """测试并发创建新会话"""
    print(f"\n=== 测试1: 并发创建 {num_requests} 个新会话 ===")
    
    async with aiohttp.ClientSession() as session:
        tasks = [send_request(session, i) for i in range(1, num_requests + 1)]
        results = await asyncio.gather(*tasks)
    
    successful = sum(1 for r in results if r['success'])
    correct = sum(1 for r in results if r.get('correct', False))
    avg_time = sum(r['elapsed'] for r in results) / len(results)
    
    print(f"\n统计:")
    print(f"  成功请求: {successful}/{num_requests}")
    print(f"  正确响应: {correct}/{num_requests}")
    print(f"  平均耗时: {avg_time:.2f}s")
    
    return results


async def test_concurrent_same_conversation(num_requests: int):
    """测试同一会话的并发请求"""
    print(f"\n=== 测试2: 同一会话的 {num_requests} 个并发请求 ===")
    
    # 先创建一个会话
    async with aiohttp.ClientSession() as session:
        initial_result = await send_request(session, 0)
        if not initial_result['success']:
            print("创建初始会话失败！")
            return
        
        conversation_id = initial_result['conversation_id']
        print(f"使用会话ID: {conversation_id}")
        
        # 并发发送多个请求到同一会话
        tasks = [send_request(session, i, conversation_id) for i in range(1, num_requests + 1)]
        results = await asyncio.gather(*tasks)
    
    successful = sum(1 for r in results if r['success'])
    correct = sum(1 for r in results if r.get('correct', False))
    avg_time = sum(r['elapsed'] for r in results) / len(results)
    
    print(f"\n统计:")
    print(f"  成功请求: {successful}/{num_requests}")
    print(f"  正确响应: {correct}/{num_requests}")
    print(f"  平均耗时: {avg_time:.2f}s")
    
    return results


async def test_mixed_concurrent(num_new: int, num_same: int):
    """测试混合并发：新会话和已有会话"""
    print(f"\n=== 测试3: 混合并发 ({num_new} 个新会话 + {num_same} 个同会话请求) ===")
    
    async with aiohttp.ClientSession() as session:
        # 创建一个初始会话
        initial_result = await send_request(session, 0)
        if not initial_result['success']:
            print("创建初始会话失败！")
            return
        
        conversation_id = initial_result['conversation_id']
        print(f"已有会话ID: {conversation_id}")
        
        # 混合请求
        new_tasks = [send_request(session, i) for i in range(1, num_new + 1)]
        same_tasks = [send_request(session, i + num_new, conversation_id) for i in range(1, num_same + 1)]
        
        all_tasks = new_tasks + same_tasks
        results = await asyncio.gather(*all_tasks)
    
    new_results = results[:num_new]
    same_results = results[num_new:]
    
    print(f"\n新会话统计:")
    successful = sum(1 for r in new_results if r['success'])
    correct = sum(1 for r in new_results if r.get('correct', False))
    avg_time = sum(r['elapsed'] for r in new_results) / len(new_results) if new_results else 0
    print(f"  成功请求: {successful}/{num_new}")
    print(f"  正确响应: {correct}/{num_new}")
    print(f"  平均耗时: {avg_time:.2f}s")
    
    print(f"\n同会话统计:")
    successful = sum(1 for r in same_results if r['success'])
    correct = sum(1 for r in same_results if r.get('correct', False))
    avg_time = sum(r['elapsed'] for r in same_results) / len(same_results) if same_results else 0
    print(f"  成功请求: {successful}/{num_same}")
    print(f"  正确响应: {correct}/{num_same}")
    print(f"  平均耗时: {avg_time:.2f}s")
    
    return results


async def main():
    """主测试函数"""
    print("开始并发测试...")
    print("确保服务器已启动并启用交互式会话模式")
    print("建议设置: CLAUDE_CODE__CLAUDE__USE_INTERACTIVE_SESSIONS=true")
    
    # 测试1: 并发新会话
    await test_concurrent_new_conversations(5)
    
    # 等待一下
    await asyncio.sleep(2)
    
    # 测试2: 同一会话并发
    await test_concurrent_same_conversation(5)
    
    # 等待一下
    await asyncio.sleep(2)
    
    # 测试3: 混合并发
    await test_mixed_concurrent(3, 3)
    
    print("\n测试完成！")


if __name__ == "__main__":
    asyncio.run(main())