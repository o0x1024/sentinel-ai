# V8 HandleScope 重启问题分析

## 问题症状

当尝试在同一线程上创建和销毁多个 `JsRuntime` 实例时，V8 会崩溃：

```
# Fatal error in v8::HandleScope::CreateHandle()
# Cannot create a handle without a HandleScope
```

## 根本原因

### V8 Isolate 的生命周期管理

V8 的 `Isolate` 是一个独立的 JavaScript 执行环境：

1. **Isolate 绑定到线程**
   - 每个 V8 Isolate 在创建时会记录当前线程 ID
   - Isolate 的所有操作必须在同一线程上进行

2. **HandleScope 的作用**
   - `HandleScope` 管理 V8 堆对象的句柄
   - 所有 V8 API 调用都需要一个活跃的 HandleScope

3. **清理顺序很关键**
   ```
   创建 Isolate
   ↓
   创建 Context
   ↓
   创建 HandleScope
   ↓
   执行 JS 代码
   ↓
   销毁 HandleScope
   ↓
   销毁 Context
   ↓
   销毁 Isolate
   ```

### 为什么在同一线程上连续创建失败？

```rust
// 第一次创建
let mut engine1 = PluginEngine::new()?; // Isolate A 创建
//使用 engine1...
drop(engine1); // Isolate A 销毁

// 立即第二次创建
let mut engine2 = PluginEngine::new()?; // ❌ Isolate B 创建时崩溃！
```

**问题所在**：
1. V8 的 Isolate 销毁是异步的
2. `drop(engine1)` 触发了销毁流程，但**V8 内部清理可能还未完成**
3. 立即创建新的 Isolate 时，V8 发现线程上还有未清理的状态
4. 新 Isolate 的 HandleScope 创建失败

## 尝试过的解决方案

### ❌ 方案1：等待一段时间

```rust
drop(old_engine);
tokio::time::sleep(Duration::from_millis(50)).await;
let new_engine = PluginEngine::new()?; // 仍然失败！
```

**失败原因**：50ms 可能不够，而且没有明确的信号表明清理完成。

### ❌ 方案2：使用 spawn_blocking

```rust
tokio::task::spawn_blocking(move || {
    drop(old_engine);
});
```

**失败原因**：`PluginEngine` 不是 `Send`，不能跨线程传递。

### ❌ 方案3：显式 drop 后创建

```rust
std::mem::drop(old_engine);
let new_engine = PluginEngine::new()?; // 仍然失败！
```

**失败原因**：Rust 的 drop 是同步的，但 V8 的清理是异步的。

## ✅ 正确的解决方案

### 方案A：每次重启都使用新线程（推荐）

**原理**：完全避免在同一线程上重复创建 Isolate。

```rust
pub struct PluginExecutor {
    current_thread: Option<JoinHandle<()>>,
    command_sender: mpsc::Sender<Command>,
    restart_threshold: usize,
}

impl PluginExecutor {
    fn restart(&mut self) {
        // 1. 发送停止信号给旧线程
        self.command_sender.send(Command::Stop).await;
        
        // 2. 等待旧线程完全退出
        if let Some(handle) = self.current_thread.take() {
            handle.join().unwrap();  // 确保线程完全退出
        }
        
        // 3. 创建新线程
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            // 新线程中创建新的 PluginEngine
            let engine = PluginEngine::new().unwrap();
            // ... 处理命令循环
        });
        
        self.current_thread = Some(handle);
        self.command_sender = tx;
    }
}
```

**优点**：
- ✅ 完全隔离每个 Isolate
- ✅ 线程退出时 V8 完全清理
- ✅ 新线程中创建新 Isolate 没有问题

**缺点**：
- ❌ 线程创建开销（~1ms）
- ❌ 重启期间无法处理请求

### 方案B：使用进程池（最可靠）

**原理**：每个插件在独立进程中运行。

```rust
// 使用子进程运行插件
let child = Command::new("plugin-worker")
    .arg(&plugin_id)
    .spawn()?;

// 通过 IPC 通信
child.stdin.write_all(&request)?;
let result = child.stdout.read_to_end()?;
```

**优点**：
- ✅ 完全隔离
- ✅ 崩溃不影响主进程
- ✅ 可以强制 kill 卡死的插件

**缺点**：
- ❌ 进程创建开销大（~10-50ms）
- ❌ IPC 通信开销
- ❌ 实现复杂度高

### 方案C：池化策略（性能最优）

**原理**：维护一个线程池，重用线程但不重用 Isolate。

```rust
pub struct PluginExecutorPool {
    workers: Vec<Worker>,
    current_index: AtomicUsize,
}

struct Worker {
    thread: JoinHandle<()>,
    sender: mpsc::Sender<WorkItem>,
    execution_count: AtomicUsize,
}

impl PluginExecutorPool {
    async fn execute(&self, txn: HttpTransaction) -> Result<Vec<Finding>> {
        // 1. 选择执行次数最少的 worker
        let worker = self.select_worker();
        
        // 2. 如果该 worker 执行次数超过阈值，替换它
        if worker.execution_count.load() > THRESHOLD {
            self.replace_worker(worker_index);
        }
        
        // 3. 发送任务到 worker
        worker.sender.send(WorkItem::Execute(txn)).await?;
    }
    
    fn replace_worker(&mut self, index: usize) {
        // 停止并等待旧 worker 线程退出
        let old_worker = &self.workers[index];
        old_worker.sender.send(WorkItem::Stop).await;
        old_worker.thread.join().unwrap();
        
        // 创建新 worker（新线程）
        let new_worker = Worker::new();
        self.workers[index] = new_worker;
    }
}
```

**优点**：
- ✅ 性能好（多个 worker 并发）
- ✅ 平滑重启（只替换单个 worker）
- ✅ 请求几乎不中断

**缺点**：
- ❌ 实现复杂
- ❌ 内存占用大（多个 V8 实例）

## 最终推荐

### 对于当前项目

**使用方案A：每次重启创建新线程**

理由：
1. 实现简单，可靠性高
2. 重启频率低（每1000-10000次执行）
3. 1ms 的线程创建开销可以接受
4. 符合V8的设计原则

### 优化后的实现

```rust
pub struct PluginExecutor {
    // 不再持有线程句柄，每次重启都创建新的
    sender: mpsc::Sender<PluginCommand>,
    plugin_id: String,
    metadata: PluginMetadata,
    code: String,
    restart_count: Arc<AtomicUsize>,
    stop_signal: Arc<AtomicBool>,
}

impl PluginExecutor {
    pub async fn restart(&self) -> Result<()> {
        // 1. 发送停止信号
        self.stop_signal.store(true, Ordering::Relaxed);
        
        // 2. 等待旧线程自然退出（通过channel关闭）
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // 3. 创建新线程
        let (tx, rx) = mpsc::channel(100);
        let code = self.code.clone();
        let metadata = self.metadata.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
                
            rt.block_on(async move {
                let mut engine = PluginEngine::new().unwrap();
                engine.load_plugin_with_metadata(&code, metadata).await.unwrap();
                
                // 处理命令循环...
                while let Some(cmd) = rx.recv().await {
                    // ...
                }
            });
        });
        
        // 4. 更新 sender
        // 注意：这需要使用 Arc<Mutex<Sender>> 或类似机制
    }
}
```

## 核心要点

1. **V8 Isolate 必须在创建它的线程上销毁**
2. **销毁后立即在同一线程创建新 Isolate 会失败**
3. **最可靠的重启方式是：旧线程退出 → 创建新线程 → 新线程中创建新 Isolate**
4. **不要试图在同一线程上"重用"或"快速切换" Isolate**

## 参考

- [V8 Embedder's Guide - Isolates](https://v8.dev/docs/embed#isolates)
- [Deno Core - Runtime](https://docs.rs/deno_core/latest/deno_core/struct.JsRuntime.html)
- [Rust std::thread](https://doc.rust-lang.org/std/thread/index.html)

