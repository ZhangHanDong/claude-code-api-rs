# InteractiveSessionManager 并发问题分析

## 1. 问题概述

当 `use_interactive_sessions` 设置为 `true` 时，`InteractiveSessionManager` 的当前实现无法正确处理对**同一个会话（Conversation）的并发请求**，会导致响应内容混淆、数据格式错误和请求超时。

## 2. 根本原因：错误的并发模型

该模块的核心缺陷在于，它允许多个并行的请求在没有任何同步或锁机制的情况下，同时读写同一个 `claude-code` 子进程。这导致了输入和输出的严重混乱。

### 2.1. 输入混乱 (Input Corruption)

- **问题点**: 在 `get_or_create_session_and_send` 函数中，每个并发请求都会获得一个指向同一个 `stdin` 发送器 (`stdin_tx`) 的克隆。
- **后果**: 多个请求的输入消息（例如 "用户问题A" 和 "用户问题B"）会被同时写入子进程的标准输入。`claude-code` 进程会收到一个混杂的输入流，如 `"问题A
问题B
"`，而不是预期的、一次一个的串行输入。

### 2.2. 输出广播混乱 (Output Corruption)

- **问题点**: `claude-code` 进程在处理完混乱的输入后，会产生一连串混乱的输出。这个输出流被一个 `broadcast` channel (`output_tx`) 不加区分地广播给**所有**订阅了该会话的并发请求。
- **后果**: 发起请求A的客户端，会同时收到发给请求A和请求B的混合响应。同理，请求B的客户端也会收到混合的响应。这导致任何一个客户端都无法获得一个完整、正确的返回结果。

## 3. 代码定位

问题的根源在于 `src/core/interactive_session.rs` 中 `get_or_create_session_and_send` 函数的设计，它没有对会话进程的访问进行互斥控制。

```rust
// src/core/interactive_session.rs

pub async fn get_or_create_session_and_send(...) {
    // ...
    if let Some(session) = sessions.get(&conversation_id) {
        // 缺陷1：每个并发请求都会订阅同一个广播通道，导致收到混合响应
        let mut output_rx = session.output_tx.subscribe(); 
        
        // 缺陷2：每个并发请求都会拿到同一个 stdin 发送器的克隆
        let stdin_tx = session.stdin_tx.clone(); 
        // ...
    }
    // ...
    if let Some((_conv_id, stdin_tx)) = existing_session {
        // 缺陷3：没有锁，多个请求会同时向 stdin 发送数据
        match stdin_tx.send(message.clone()).await {
            // ...
        }
    }
    // ...
}
```

## 4. 解决方案方向

要彻底修复此问题，必须重构 `InteractiveSessionManager` 的并发模型，**强制串行化**对每个 `claude-code` 进程的访问。

- **核心思路**: 引入**互斥锁 (Mutex)** 来保护对每个 `InteractiveSession` 实例的访问。
- **具体实现**:
  1. 在 `InteractiveSession` 结构体中增加一个 `tokio::sync::Mutex`。
  2. 当一个请求需要与会话交互时，它必须首先获取这个锁。
  3. 在持有锁的期间，执行完整的“发送消息 -> 循环读取直到获得完整响应 -> 返回结果”的串行操作。
  4. 操作完成后，释放锁，允许下一个等待的请求进入。

通过这种方式，可以保证在任何时候只有一个请求在与 `claude-code` 进程进行交互，从而彻底解决输入输出的混乱问题。
