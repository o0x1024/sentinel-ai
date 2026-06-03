# PluginExecutor vs PluginEngine 对比分析

## 核心区别

### PluginEngine（底层引擎）

**定义**: 直接封装 Deno Core 的 `JsRuntime`，提供 V8 引擎的直接访问。

**架构**:
```
PluginEngine
├── JsRuntime (V8 引擎)
│   ├── ModuleLoader
│   ├── Extensions (deno_web, deno_crypto, etc.)
│   └── OpState (插件上下文)
└── PluginMetadata
```

**特点**:
- ✅ **直接访问**: 最底层的 API，性能最高
- ✅ **灵活控制**: 完全控制 V8 引擎的生命周期
- ❌ **不是 Send**: 包含 `Rc<T>`，不能跨线程传递
- ❌ **需要手动管理**: 内存、生命周期都需要开发者控制
- ❌ **长时间运行问题**: V8 内存会持续增长

### PluginExecutor（执行器封装）

**定义**: 在专用线程中运行 `PluginEngine`，通过消息通道通信。

**架构**:
```
PluginExecutor (主线程)
├── mpsc::Sender<PluginCommand>
└── 专用线程
    ├── tokio Runtime
    └── PluginEngine (V8)
        └── 持续运行，处理命令队列
```

**特点**:
- ✅ **线程安全**: 可以跨线程传递（实现了 Send）
- ✅ **隔离执行**: 每个插件有独立的线程和 Runtime
- ✅ **自动管理**: 生命周期由 Executor 管理
- ✅ **并发友好**: 适合高并发场景
- ❌ **额外开销**: 消息传递和线程切换的成本
- ❌ **仍有内存问题**: 底层还是 PluginEngine，长时间运行仍会泄漏

## 详细对比表

| 特性 | PluginEngine | PluginExecutor |
|------|-------------|---------------|
| **线程模型** | 单线程（当前线程） | 独立专用线程 |
| **Send trait** | ❌ 不实现 | ✅ 实现 |
| **创建开销** | 小（~50ms） | 大（~100ms + 线程启动） |
| **执行开销** | 最小（直接调用） | 中等（消息传递） |
| **内存隔离** | 无 | 线程级隔离 |
| **并发支持** | 差（需要 spawn_blocking） | 好（天然支持） |
| **生命周期** | 手动管理 | 自动管理 |
| **适用场景** | 临时执行、测试 | 生产环境、长期运行 |
| **内存管理** | 需要手动处理 | 封装了但仍有问题 |

## 代码示例对比

### 使用 PluginEngine（直接）

```rust
// ❌ 不适合长时间运行
async fn test_with_engine() {
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
    
    // 执行30秒
    for _ in 0..3000 {
        let transaction = create_transaction();
        engine.scan_transaction(&transaction).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // 结果：内存泄漏 5GB+！
}

// ✅ 适合短期使用
async fn test_with_engine_short() {
    let mut engine = PluginEngine::new().unwrap();
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
    
    // 执行几次就销毁
    for _ in 0..10 {
        let transaction = create_transaction();
        engine.scan_transaction(&transaction).await.unwrap();
    }
    drop(engine); // 立即释放
}
```

### 使用 PluginExecutor（封装）

```rust
// ✅ 适合长时间运行（但仍会泄漏）
async fn test_with_executor() {
    let executor = PluginExecutor::new(metadata, code).unwrap();
    
    // 执行30秒
    for _ in 0..3000 {
        let transaction = create_transaction();
        executor.scan_transaction(transaction).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    // 结果：内存仍会泄漏，但隔离更好
}

// ✅ 可以并发调用
async fn test_with_executor_concurrent() {
    let executor = Arc::new(PluginExecutor::new(metadata, code).unwrap());
    
    let mut handles = vec![];
    for _ in 0..100 {
        let exec = executor.clone();
        let handle = tokio::spawn(async move {
            let transaction = create_transaction();
            exec.scan_transaction(transaction).await.unwrap();
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## 内存泄漏问题分析

### 根本原因

两者都使用同一个 `PluginEngine` 底层实现，因此**都有内存泄漏问题**：

```rust
// executor.rs 第52-75行
let mut engine = match PluginEngine::new() {
    Ok(e) => e,
    // ...
};

// 循环处理命令
while let Some(cmd) = rx.recv().await {
    match cmd {
        PluginCommand::ScanTransaction(txn, reply) => {
            let res = engine.scan_transaction(&txn).await;  // ⚠️ 同样会泄漏
            let _ = reply.send(res);
        }
    }
}
```

### 泄漏机制

1. **V8 引擎内部状态累积**
   - 每次执行都创建新的执行上下文
   - 模块缓存持续增长
   - GC 不够激进

2. **Deno Core 的缓存**
   - ModuleLoader 缓存模块代码
   - OpState 累积状态
   - Extension 的内部状态

3. **没有清理机制**
   - 没有显式 GC 调用
   - 没有定期重置机制
   - 没有内存限制

## 修改影响分析

### 场景1: 测试代码改用 PluginExecutor

```rust
// 原代码（PluginEngine）
let mut engine = PluginEngine::new().unwrap();
engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
for _ in 0..1000 {
    engine.scan_transaction(&transaction).await;
}

// 改为 PluginExecutor
let executor = PluginExecutor::new(metadata, code).unwrap();
for _ in 0..1000 {
    executor.scan_transaction(transaction.clone()).await;  // ⚠️ 需要 clone
}
```

**影响**:
- ✅ 代码可以编译
- ✅ 线程安全
- ⚠️ 性能略降（消息传递开销）
- ❌ **内存泄漏问题依然存在**！

### 场景2: 并发测试改用 PluginExecutor

```rust
// 原代码（有编译错误）
tokio::spawn(async move {
    let mut engine = PluginEngine::new().unwrap();  // ❌ 不能跨线程
    engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
    engine.scan_transaction(&transaction).await;
});

// 改为 PluginExecutor
let executor = Arc::new(PluginExecutor::new(metadata, code).unwrap());
tokio::spawn(async move {
    let exec = executor.clone();  // ✅ 可以 clone
    exec.scan_transaction(transaction).await;
});
```

**影响**:
- ✅ 编译通过
- ✅ 并发更高效
- ⚠️ 创建开销更大
- ❌ 内存泄漏仍未解决

### 场景3: 生产环境使用

```rust
// 原方案（PluginManager 使用 spawn_blocking）
pub async fn scan_transaction(&self, plugin_id: &str, transaction: &HttpTransaction) 
    -> Result<Vec<Finding>> 
{
    let (metadata, code) = self.get_plugin(plugin_id).await?;
    
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async move {
            let mut engine = PluginEngine::new()?;
            engine.load_plugin_with_metadata(&code, metadata).await?;
            engine.scan_transaction(&transaction).await
        })
    }).await??
}

// 改为 PluginExecutor（需要重构）
pub struct PluginManager {
    executors: Arc<RwLock<HashMap<String, Arc<PluginExecutor>>>>,
}

pub async fn scan_transaction(&self, plugin_id: &str, transaction: &HttpTransaction) 
    -> Result<Vec<Finding>> 
{
    let executor = self.get_or_create_executor(plugin_id).await?;
    executor.scan_transaction(transaction.clone()).await
}
```

**影响**:
- ✅ 性能更好（复用 Executor）
- ✅ 架构更清晰
- ⚠️ 需要管理 Executor 生命周期
- ⚠️ 需要定期重启 Executor（避免长期泄漏）

## 真正的解决方案

### 方案1: 定期重建引擎（推荐）

```rust
pub struct PluginExecutor {
    current_executor: Arc<RwLock<PluginExecutor>>,
    metadata: PluginMetadata,
    code: String,
    execution_count: Arc<AtomicUsize>,
    max_executions: usize,  // 例如 1000 次后重启
}

impl PluginExecutor {
    pub async fn scan_transaction(&self, transaction: HttpTransaction) 
        -> Result<Vec<Finding>> 
    {
        // 检查是否需要重启
        let count = self.execution_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.max_executions {
            self.restart_executor().await?;
        }
        
        let executor = self.current_executor.read().await;
        executor.scan_transaction(transaction).await
    }
    
    async fn restart_executor(&self) -> Result<()> {
        let mut executor = self.current_executor.write().await;
        *executor = PluginExecutor::new(
            self.metadata.clone(), 
            self.code.clone()
        )?;
        self.execution_count.store(0, Ordering::Relaxed);
        Ok(())
    }
}
```

### 方案2: 添加显式 GC（需要修改引擎）

```rust
// plugin_engine.rs
impl PluginEngine {
    pub async fn force_gc(&mut self) {
        // 触发 V8 垃圾回收
        let script = "if (globalThis.gc) globalThis.gc();";
        let _ = self.runtime.execute_script("<gc>", script.into());
        
        // 运行事件循环确保清理完成
        let _ = self.runtime
            .run_event_loop(deno_core::PollEventLoopOptions::default())
            .await;
    }
}

// executor.rs - 定期调用
while let Some(cmd) = rx.recv().await {
    match cmd {
        PluginCommand::ScanTransaction(txn, reply) => {
            let res = engine.scan_transaction(&txn).await;
            let _ = reply.send(res);
            
            // 每100次执行触发GC
            if execution_count % 100 == 0 {
                engine.force_gc().await;
            }
        }
    }
}
```

### 方案3: 进程隔离（终极方案）

```rust
// 每个插件运行在独立进程中
pub struct ProcessIsolatedExecutor {
    child_process: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl ProcessIsolatedExecutor {
    pub async fn scan_transaction(&mut self, transaction: HttpTransaction) 
        -> Result<Vec<Finding>> 
    {
        // 通过 IPC 发送任务
        self.stdin.write_all(&serialize(&transaction)?).await?;
        
        // 读取结果
        let result = self.stdout.read_to_string(&mut String::new()).await?;
        
        // 进程崩溃或泄漏？重启即可
        if self.should_restart() {
            self.restart_process()?;
        }
        
        Ok(deserialize(&result)?)
    }
}
```

## 测试修改建议

### 内存泄漏测试应该测试什么？

**当前测试**（失败但有价值）:
```rust
// ✅ 证明了长时间运行会泄漏
#[tokio::test]
async fn test_simple_plugin_memory_leak() {
    let mut engine = PluginEngine::new().unwrap();
    // 30秒持续运行 -> 泄漏 5GB
    while elapsed < 30s {
        engine.scan_transaction(&txn).await;
    }
    // 断言：不应该泄漏
    assert!(growth_rate < 0.1 MB/s);  // ❌ 失败：150 MB/s
}
```

**改进后的测试**:
```rust
// ✅ 测试短期使用是否正常
#[tokio::test]
async fn test_short_term_memory_usage() {
    for _ in 0..100 {
        let mut engine = PluginEngine::new().unwrap();
        engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
        
        // 执行10次后销毁
        for _ in 0..10 {
            engine.scan_transaction(&txn).await;
        }
        drop(engine);
    }
    // 应该不会持续增长
}

// ✅ 测试 Executor 重启机制
#[tokio::test]
async fn test_executor_restart_prevents_leak() {
    let executor = PluginExecutor::new(metadata, code, 100).unwrap();
    
    // 运行足够长时间触发多次重启
    for _ in 0..1000 {
        executor.scan_transaction(txn.clone()).await;
    }
    
    // 内存应该保持稳定
    assert!(memory_growth < threshold);
}
```

## 总结与建议

### PluginEngine vs PluginExecutor

| 使用场景 | 推荐方案 | 原因 |
|---------|---------|------|
| **临时执行**（<10次） | PluginEngine | 开销小，直接销毁 |
| **短期批处理**（10-100次） | PluginEngine | 可以接受，及时销毁 |
| **长期运行**（100+次） | ❌ 两者都不推荐 | 都会泄漏 |
| **并发执行** | PluginExecutor | 线程安全，易于管理 |
| **生产环境** | PluginExecutor + 重启机制 | 最佳平衡 |

### 修改测试的影响

1. **改用 PluginExecutor**:
   - ✅ 编译通过
   - ✅ 并发更友好
   - ❌ **内存泄漏问题仍在**

2. **真正需要的**:
   - 实现 Executor 重启机制
   - 或者接受当前设计，调整测试预期
   - 或者使用进程隔离

### 最终建议

**不要简单地改用 PluginExecutor**，因为它只是换了个包装，底层问题仍在。

**应该做的**:
1. ✅ 保留现有测试，证明长期运行有问题
2. ✅ 实现 `PluginExecutor`
3. ✅ 在文档中明确说明使用限制
4. ✅ 生产环境使用重启机制

**测试策略**:
- 短期测试：验证正常功能
- 长期测试：验证重启机制有效
- 极限测试：找到重启阈值

这样才能真正解决问题！🎯

