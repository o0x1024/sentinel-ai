# PluginExecutor 设计说明

## 概述

`PluginExecutor` 是一个支持手动重启的插件执行器，通过创建新线程来避免 V8 HandleScope 错误。

**注**: 原名为 `PluginExecutor`，现已合并为统一的 `PluginExecutor` 实现。

## 核心设计原则

### ✅ 可行：手动重启
```rust
let executor = PluginExecutor::new(metadata, code, 1000)?;

// 外部调用重启
executor.restart().await?;
```

**工作原理**：
1. 发送停止信号给当前工作线程
2. 等待线程完全退出
3. 创建新线程 + 新 V8 Isolate
4. 更新 channel sender

### ❌ 不可行：自动重启

**为什么不能自动重启？**

```rust
// 工作线程内部
while let Some(cmd) = rx.recv().await {
    if executions >= threshold {
        // ❌ 无法在这里重启自己！
        // 因为：
        // 1. 不能 join 自己的线程
        // 2. 不能在同一线程上创建新 Isolate
        // 3. 不能修改外部的 sender
    }
}
```

## 推荐使用模式

### 模式 A：定期检查和重启

```rust
pub struct PluginScheduler {
    executor: Arc<RwLock<PluginExecutor>>,
}

impl PluginScheduler {
    async fn maintenance_loop(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            
            let stats = self.executor.read().await.get_stats().await.unwrap();
            
            // 如果接近阈值，触发重启
            if stats.current_instance_executions > 900 {
                self.executor.write().await.restart().await.unwrap();
                info!("Executor restarted by scheduler");
            }
        }
    }
}
```

### 模式 B：响应式重启

```rust
pub async fn execute_with_auto_restart(
    executor: &Arc<RwLock<PluginExecutor>>,
    txn: HttpTransaction,
) -> Result<Vec<Finding>> {
    let stats = executor.read().await.get_stats().await?;
    
    // 执行前检查
    if stats.current_instance_executions >= 1000 {
        executor.write().await.restart().await?;
    }
    
    // 执行任务
    executor.read().await.scan_transaction(txn).await
}
```

### 模式 C：池化多个 Executor

```rust
pub struct PluginExecutorPool {
    executors: Vec<Arc<RwLock<PluginExecutor>>>,
    current_index: AtomicUsize,
}

impl PluginExecutorPool {
    async fn execute(&self, txn: HttpTransaction) -> Result<Vec<Finding>> {
        // 轮询选择 executor
        let idx = self.current_index.fetch_add(1, Ordering::Relaxed) % self.executors.len();
        let executor = &self.executors[idx];
        
        let stats = executor.read().await.get_stats().await?;
        
        // 如果该 executor 需要重启
        if stats.current_instance_executions >= 1000 {
            // 在后台重启，不阻塞当前请求
            let executor_clone = executor.clone();
            tokio::spawn(async move {
                executor_clone.write().await.restart().await.ok();
            });
            
            // 使用下一个 executor
            let next_idx = (idx + 1) % self.executors.len();
            return self.executors[next_idx].read().await.scan_transaction(txn).await;
        }
        
        executor.read().await.scan_transaction(txn).await
    }
}
```

## API 说明

### 构造函数

```rust
pub fn new(
    metadata: PluginMetadata,
    code: String,
    max_executions_before_restart: usize,
) -> Result<Self>
```

- `max_executions_before_restart`: 建议值 1000-10000
- 这个值**不会触发自动重启**，仅用于统计和外部判断

### 核心方法

#### 1. `scan_transaction`
```rust
pub async fn scan_transaction(&self, transaction: HttpTransaction) -> Result<Vec<Finding>>
```
执行插件扫描，线程安全。

#### 2. `restart` (手动重启)
```rust
pub async fn restart(&self) -> Result<()>
```
- 停止当前工作线程
- 等待其完全退出
- 创建新线程 + 新 V8 Isolate
- **耗时**: 约 50-150ms

#### 3. `get_stats`
```rust
pub async fn get_stats(&self) -> Result<ExecutorStats>
```
获取执行统计信息，用于判断是否需要重启。

#### 4. `shutdown`
```rust
pub async fn shutdown(&self) -> Result<()>
```
优雅地关闭executor。

## 性能考虑

### 重启开销

| 操作 | 耗时 |
|------|------|
| 发送停止信号 | ~1ms |
| 等待线程退出 | ~10-50ms |
| 创建新线程 | ~1ms |
| 初始化 V8 Isolate | ~10-50ms |
| 加载插件代码 | ~5-20ms |
| **总计** | **~50-150ms** |

### 优化策略

1. **减少重启频率**
   ```rust
   // 每 10000 次执行重启一次
   PluginExecutor::new(metadata, code, 10000)?
   ```

2. **预热备用实例**
   ```rust
   // 在重启前先创建新实例
   let new_executor = PluginExecutor::new(...)?;
   // 切换到新实例
   // 然后关闭旧实例
   ```

3. **使用多实例池**
   ```rust
   // 3 个实例轮询使用
   let pool = PluginExecutorPool::new(metadata, code, 3)?;
   ```

## 内存泄漏问题

### 测试结果

从 30 秒压测结果看：
- **开始内存**: 56.91 MB
- **结束内存**: 1287.19 MB
- **内存增长**: 1230.28 MB
- **增长率**: 40.9 MB/s
- **执行次数**: 789,196 次
- **重启次数**: 0 次（未触发重启）

### 结论

1. **V8 确实存在内存累积**
   - 即使没有明显泄漏，V8 堆也会增长
   - 这是 V8 GC 策略的正常行为

2. **重启可以缓解但不能根除**
   - 重启会创建新 Isolate，重置内存
   - 但如果执行频率高，仍会快速增长

3. **推荐策略**
   - 定期重启（每 1000-10000 次）
   - 监控内存使用
   - 设置内存告警阈值
   - 必要时重启整个应用

## 与 PluginExecutor 的对比

| 特性 | PluginExecutor | PluginExecutor |
|------|---------------|--------------------------|
| 重启能力 | ❌ 无 | ✅ 手动重启 |
| 自动重启 | ❌ 无 | ❌ 需外部协调 |
| 内存管理 | 依赖 V8 GC | V8 GC + 重启 |
| 复杂度 | 简单 | 中等 |
| 性能 | 高 | 高（重启时有短暂停顿） |
| 适用场景 | 短期运行 | 长期运行 |

## 最佳实践

### 1. 何时使用 PluginExecutor
- 插件生命周期短（< 1小时）
- 执行次数少（< 10,000 次）
- 内存增长可接受

### 2. 何时使用 PluginExecutor
- 插件需要长期运行（> 1小时）
- 执行次数多（> 10,000 次）
- 需要主动管理内存

### 3. 监控指标

```rust
let stats = executor.get_stats().await?;

// 监控这些指标
println!("Executions: {}", stats.current_instance_executions);
println!("Restarts: {}", stats.restart_count);
println!("Last restart: {:?}", stats.last_restart_time);

// 告警规则
if stats.current_instance_executions > 5000 {
    warn!("Executor approaching restart threshold");
}
```

### 4. 优雅重启

```rust
async fn graceful_restart(executor: &PluginExecutor) -> Result<()> {
    info!("Starting graceful restart");
    
    // 1. 停止接收新任务（需要外部控制）
    
    // 2. 等待正在进行的任务完成（通过统计检查）
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 3. 执行重启
    executor.restart().await?;
    
    // 4. 恢复接收任务
    
    info!("Graceful restart completed");
    Ok(())
}
```

## 未来改进方向

### 1. 基于内存的重启策略

```rust
pub struct MemoryBasedRestart {
    executor: PluginExecutor,
    memory_threshold: usize, // MB
}

impl MemoryBasedRestart {
    async fn check_and_restart(&mut self) {
        let current_memory = get_process_memory();
        if current_memory > self.memory_threshold {
            self.executor.restart().await.ok();
        }
    }
}
```

### 2. 自适应重启间隔

```rust
// 根据内存增长速度动态调整重启间隔
let growth_rate = calculate_memory_growth_rate();
let optimal_interval = calculate_optimal_interval(growth_rate);
```

### 3. 零停机重启

```rust
// 使用双缓冲模式
let active = executor_a;
let standby = executor_b;

// 切换到 standby
swap(active, standby);

// 重启旧的 active
old_active.restart().await;
```

## 总结

`PluginExecutor` 提供了**手动重启**能力，是应对 V8 长期内存累积的有效手段。

**关键要点**：
1. ✅ 支持手动重启，创建新线程避免 V8 错误
2. ❌ 不支持自动重启，需要外部协调
3. 🔄 推荐使用定期检查或响应式重启模式
4. 📊 通过 `get_stats()` 监控执行状态
5. ⚡ 重启开销约 50-150ms，可接受

**使用建议**：
- 短期任务：使用 `PluginExecutor`
- 长期任务：使用 `PluginExecutor` + 外部重启调度器

