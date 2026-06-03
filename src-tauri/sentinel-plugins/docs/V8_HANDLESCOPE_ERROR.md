# V8 HandleScope 错误分析与解决

## 🔴 错误现象

```
# Fatal error in v8::HandleScope::CreateHandle()
# Cannot create a handle without a HandleScope
```

## 原因分析

### 1. V8 Runtime 的线程安全限制

V8 的 `JsRuntime` 包含大量非线程安全的类型：

```rust
// PluginEngine 内部结构
pub struct PluginEngine {
    runtime: JsRuntime,           // 包含 Lrc<JsRuntimeState>
    loader: Rc<PluginModuleLoader>, // Rc 不是 Send
    // ...
}
```

**关键类型限制**：
- `Lrc<T>` (Local Reference Counted) - 不能跨线程
- `Rc<T>` (Reference Counted) - 不能跨线程  
- `NonNull<T>` - V8 内部指针，线程绑定
- `*const T` - 原始指针，不能跨线程

### 2. HandleScope 的作用

在 V8 中：
- **HandleScope** 管理 V8 对象的生命周期
- **Handle** 是指向 V8 堆对象的智能指针
- 所有 V8 对象操作必须在 HandleScope 上下文中进行

### 3. 错误触发场景

#### ❌ 错误的做法（之前的代码）

```rust
// 尝试在 spawn_blocking 中销毁引擎
let old_engine = std::mem::replace(&mut engine, new_engine);
tokio::task::spawn_blocking(move || {
    drop(old_engine);  // ❌ 跨线程销毁，没有正确的 HandleScope
});
```

**问题**：
1. `old_engine` 被移动到另一个线程
2. 在新线程中 `drop` 时，V8 尝试清理资源
3. 但当前线程没有 HandleScope 上下文
4. 导致 `Cannot create a handle without a HandleScope` 错误

#### ✅ 正确的做法

```rust
// 在同一线程/上下文中替换引擎
match Self::create_engine(&code, &metadata, &plugin_id).await {
    Ok(new_engine) => {
        engine = new_engine;  // ✅ 旧引擎在当前上下文中自动 drop
        // ...
    }
}
```

**原理**：
1. 赋值操作会自动触发旧 `engine` 的 drop
2. drop 发生在创建它的同一线程上
3. V8 的 HandleScope 上下文正确
4. 资源安全释放

## 为什么之前尝试使用 spawn_blocking？

### 初始想法（错误）
```rust
// 以为可以避免阻塞主线程
tokio::task::spawn_blocking(move || {
    drop(old_engine);  // 想在后台线程销毁
});
```

**误解**：
- 认为 V8 清理是阻塞操作，应该放到 blocking 线程池
- 但忽略了 V8 不是线程安全的

### 正确理解

1. **V8 Runtime 是单线程的**
   - 必须在创建它的线程上销毁
   - 不能跨线程共享或传递

2. **PluginExecutor 的设计**
   - 为每个插件创建专属线程
   - 所有 V8 操作都在该线程上
   - 这样保证了线程安全

3. **Drop 操作很快**
   - V8 的清理操作实际上很快
   - 不需要特意放到 blocking 线程池
   - 在原线程 drop 是最安全的方式

## 完整的解决方案

### 架构设计

```rust
// 1. 主线程创建 Executor
let executor = PluginExecutor::new(metadata, code)?;

// 2. Executor 内部启动专属线程
std::thread::spawn(move || {
    // 3. 在专属线程上创建 PluginEngine
    let mut engine = PluginEngine::new(...);
    
    // 4. 处理命令循环
    loop {
        match rx.recv() {
            PluginCommand::ScanTransaction(...) => {
                // 检查是否需要重启
                if should_restart {
                    // 5. 在同一线程上重新创建引擎
                    let new_engine = PluginEngine::new(...);
                    engine = new_engine;  // 旧引擎在这里 drop
                }
                // 执行任务
                engine.scan_transaction(...);
            }
        }
    }
});
```

### 关键点

1. **一个线程，一个 Runtime**
   - 每个 `PluginEngine` 有自己的线程
   - V8 Runtime 生命周期完全在该线程内

2. **重启操作在原线程**
   - 创建新引擎：`let new_engine = PluginEngine::new(...);`
   - 替换引擎：`engine = new_engine;`
   - 旧引擎自动 drop（在同一线程）

3. **不使用 spawn_blocking**
   - `PluginEngine` 不是 `Send`，不能跨线程
   - drop 操作很快，不需要单独的线程池

## 测试中的注意事项

### 并发测试

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_execution() {
    // ✅ 创建多个 executor，每个有自己的线程
    let executor1 = PluginExecutor::new(...)?;
    let executor2 = PluginExecutor::new(...)?;
    
    // ✅ 并发执行
    tokio::join!(
        executor1.scan_transaction(txn1),
        executor2.scan_transaction(txn2),
    );
}
```

### 避免的模式

```rust
// ❌ 不要尝试克隆或共享 PluginEngine
let engine = PluginEngine::new(...);
tokio::spawn(async move {
    engine.scan_transaction(...);  // ❌ 错误！
});

// ❌ 不要尝试手动控制 drop 的线程
let engine = PluginEngine::new(...);
std::thread::spawn(move || {
    drop(engine);  // ❌ 可能错误！
});
```

## 性能考虑

### 重启开销

- **创建新 Runtime**：~10-50ms
- **加载模块**：~5-20ms
- **销毁旧 Runtime**：~5-10ms
- **总计**：~20-80ms

### 优化策略

1. **设置合理的重启间隔**
   ```rust
   // 每 1000 次执行重启一次
   PluginExecutor::with_restart_interval(
       metadata, 
       code, 
       1000
   )
   ```

2. **监控内存使用**
   ```rust
   // 仅在内存超过阈值时重启
   if current_memory > threshold {
       executor.restart().await?;
   }
   ```

3. **按需重启**
   ```rust
   // 提供手动重启 API
   executor.restart().await?;
   ```

## 总结

| 方面 | 错误做法 | 正确做法 |
|------|---------|---------|
| **引擎销毁** | 跨线程 drop | 原线程 drop |
| **重启方式** | spawn_blocking | 直接赋值 |
| **并发模型** | 共享引擎 | 独立线程 |
| **错误现象** | HandleScope 错误 | 正常运行 |

**核心原则**：V8 Runtime 必须在创建它的线程上使用和销毁，不能跨线程传递。

## 参考

- [V8 Embedder's Guide](https://v8.dev/docs/embed)
- [Deno Core Runtime](https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html)
- [Rust Send and Sync Traits](https://doc.rust-lang.org/nomicon/send-and-sync.html)

