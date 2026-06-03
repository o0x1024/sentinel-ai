# Robustness & Resilience Tests (鲁棒性测试)

## 概述

本测试套件专注于验证插件系统在**异常输入、极端条件、资源压力**下的健壮性与恢复能力。

### 测试文件

- **位置**: `tests/robustness_tests.rs`
- **分类**: `robustness`
- **运行方式**: `./run_stress_tests.sh robustness`

---

## 测试分类（8 大类场景）

### 1. 异常输入与边界测试 (Fuzzing & Edge Cases)

验证插件对畸形、极端、非法输入的容错能力。

#### 测试用例

- **`test_malformed_url_handling`**: URL 格式异常
  - 空 URL、超长 URL (10KB+)、非法字符、协议缺失
  - **预期**: 插件不 panic，返回可控错误或正常处理

- **`test_extreme_header_values`**: HTTP Header 边界
  - 单个 header 超长 (1MB)、header 数量极多 (10,000+)、重复 key
  - **预期**: 解析成功或优雅失败，不触发 V8 OOM

- **`test_invalid_content_types`**: Content-Type 异常
  - 空值、超长、非法字符、多重分号、缺失 charset
  - **预期**: 插件能处理或回退到默认类型

- **`test_body_size_extremes`**: Body 大小极端
  - 空 body、超大 body (100MB)、声明与实际不符
  - **预期**: 大 body 不触发内存爆炸，能正确处理或拒绝

- **`test_unicode_and_encoding_edge_cases`**: 编码边界
  - 非 UTF-8、混合编码、emoji、零宽字符、双向控制字符
  - **预期**: 不出现乱码崩溃或安全问题

---

### 2. 超时与卡死控制 (Timeout & Hang Detection)

验证插件执行超时控制与隔离机制。

#### 测试用例

- **`test_infinite_loop_timeout`**: 无限循环
  - 插件内 `while(true)` 或递归爆栈
  - **预期**: 外部超时机制生效（需配合 `tokio::time::timeout`）

- **`test_promise_never_resolves`**: Promise 永不 resolve
  - `new Promise(() => {})` 卡住
  - **预期**: 超时后任务能被取消或标记失败

- **`test_sync_blocking_operations`**: 同步阻塞
  - 大量同步 CPU 密集计算（无 await）
  - **预期**: 不阻塞整个系统，其他插件可继续执行

- **`test_recursive_stack_overflow`**: 递归栈溢出
  - 深度递归超过 V8 栈限制
  - **预期**: V8 抛出 RangeError，Rust 捕获并返回错误

**⚠️ 注意**: 
- V8 没有内置执行时间限制，`tokio::time::timeout` 只能中断等待，不能强制终止 V8 正在执行的 JS
- 若需真正的"杀死卡住的 V8"，需使用 `PluginExecutor.restart()` 或设置 V8 isolate 级别的 interrupt

---

### 3. 内存压力模式 (Memory Stress Patterns)

验证不同内存使用模式下的表现与泄漏检测。

#### 测试用例

- **`test_peak_memory_allocation_and_release`**: 峰值分配后释放
  - 一次性分配大对象 (256MB)，触发 GC 后观察内存回落
  - **预期**: 内存能回到基线 ±20%

- **`test_gradual_memory_leak_detection`**: 渐进式泄漏
  - 每次执行泄漏 1KB 闭包/全局数组，执行 1000 次
  - **预期**: 检测到线性增长趋势，建议重启

- **`test_memory_fragmentation_pattern`**: 碎片化模式
  - 交替分配/释放不同大小对象 (1KB, 10KB, 100KB)
  - **预期**: 内存使用波动但不持续上涨

- **`test_large_string_concatenation`**: 大字符串拼接
  - 循环拼接生成 100MB 字符串
  - **预期**: V8 优化处理或触发 OOM（需优雅回退）

---

### 4. 插件热更新与一致性 (Hot Reload & Consistency)

验证插件动态加载、更新、缓存的正确性。

#### 测试用例

- **`test_rapid_plugin_reload`**: 快速重载
  - 同一 plugin_id 在 1 秒内重载 10 次
  - **预期**: 最新版本生效，无缓存污染

- **`test_concurrent_load_and_execute`**: 并发加载+执行
  - 10 个线程同时加载插件 A，10 个线程执行插件 A
  - **预期**: 无竞态条件，执行使用正确版本

- **`test_reload_with_syntax_error`**: 新代码有语法错误
  - 先加载正确版本，重载时提供错误代码
  - **预期**: 旧版本继续可用 OR 明确标记插件失败

- **`test_code_cache_invalidation`**: 缓存失效
  - 修改 `PluginManager.code_cache` 后重新加载
  - **预期**: 使用新代码，不读取旧缓存

---

### 5. 并发与资源枯竭 (Concurrency & Resource Exhaustion)

验证高并发、资源紧张时的行为。

#### 测试用例

- **`test_channel_backpressure`**: 通道积压
  - 向 `PluginExecutor` 快速提交 10,000 个扫描请求
  - **预期**: `mpsc` 通道容量 (100) 触发 backpressure，调用方能感知并等待

- **`test_rapid_executor_creation_destruction`**: 快速创建/销毁
  - 1 秒内创建+销毁 100 个 `PluginExecutor`
  - **预期**: 无 V8 HandleScope 错误，无线程泄漏

- **`test_executor_restart_under_load`**: 高负载重启
  - 执行 500 次后立即调用 `restart()`，同时有 50 个请求排队
  - **预期**: 旧实例优雅退出，新实例接管，无请求丢失

- **`test_concurrent_manager_operations`**: PluginManager 并发操作
  - 100 个线程同时 register/unregister/scan 不同插件
  - **预期**: 操作串行化，状态一致

---

### 6. 跨平台差异 (Cross-Platform Edge Cases)

验证 macOS/Windows/Linux 行为一致性。

#### 测试用例

- **`test_path_separator_handling`**: 路径分隔符
  - 插件引用路径使用 `/` 或 `\`
  - **预期**: 统一处理，不因平台而异

- **`test_line_ending_normalization`**: 换行符
  - 插件代码混合 `\n` / `\r\n` / `\r`
  - **预期**: 转译/解析成功

- **`test_non_utf8_filename_handling`**: 非 UTF-8 文件名
  - 模拟 Windows GBK 路径或 Linux ISO-8859-1
  - **预期**: 能识别或明确报错，不 panic

**⚠️ CI 限制**: 本套件运行于 macOS，Windows/Linux 差异需在对应平台手动验证

---

### 7. 安全与沙箱边界 (Security & Sandbox)

验证插件权限边界与能力限制。

#### 测试用例

- **`test_filesystem_access_denial`**: 文件系统访问
  - 插件尝试 `Deno.readTextFile()` / `Deno.writeTextFile()`
  - **预期**: 抛出 Permission Denied 错误（当前沙箱配置为禁止）

- **`test_network_access_boundary`**: 网络访问
  - 插件尝试 `fetch()` 外部 URL
  - **预期**: 根据权限配置允许或拒绝

- **`test_process_spawn_prevention`**: 进程启动
  - 插件尝试 `Deno.run()` 启动子进程
  - **预期**: 明确拒绝

- **`test_eval_and_function_constructor`**: 动态代码执行
  - 插件使用 `eval()` / `new Function()`
  - **预期**: V8 允许但需审计（CSP 策略）

**⚠️ 配置依赖**: 沙箱行为取决于 `PluginEngine` 的 Deno permissions 配置

---

### 8. 日志与可观测性 (Logging & Observability)

验证日志系统鲁棒性。

#### 测试用例

- **`test_massive_console_output`**: 大量日志
  - 插件循环 `console.log()` 10,000 次
  - **预期**: 不拖垮执行速度，日志可截断/限流

- **`test_log_with_circular_references`**: 循环引用对象
  - `console.log(obj)` 其中 obj 自引用
  - **预期**: V8 或序列化层处理，不死循环

- **`test_log_non_serializable_types`**: 不可序列化类型
  - 打印 Symbol / BigInt / Function
  - **预期**: 转为字符串或占位符

---

## 运行指南

### 1. 运行所有鲁棒性测试

```bash
cd /path/to/sentinel-ai/src-tauri/sentinel-plugins
./run_stress_tests.sh robustness
```

### 2. 运行单个测试

```bash
cargo test --test robustness_tests --release -- --ignored test_malformed_url_handling --nocapture
```

### 3. 运行特定分类（手动过滤）

```bash
# 只运行"异常输入"类测试（名称包含 malformed/invalid/extreme）
cargo test --test robustness_tests --release -- --ignored malformed invalid extreme --nocapture
```

---

## 关键注意事项

### ⚠️ Timeout 机制局限

**问题**: `tokio::time::timeout` **不能中断正在执行的 V8 JavaScript**

```rust
// ❌ 这不会真正"杀死"无限循环的 JS
let result = tokio::time::timeout(
    Duration::from_secs(5),
    executor.scan_transaction(txn)
).await;

// 无限循环的 JS 仍在 V8 线程中运行，只是外部不再等待结果
```

**解决方案**:
1. **使用 `PluginExecutor.restart()`**: 新建线程+Isolate 强制重启
2. **V8 Isolate Interrupt**: 调用 `v8::Isolate::TerminateExecution()`（需底层接口暴露）
3. **预防为主**: 对不受信任的插件设置执行次数阈值，达到后自动重启

### ⚠️ V8 Fatal OOM

**问题**: 某些内存测试（如 `test_body_size_extremes`）可能触发 V8 的 `Fatal JavaScript out of memory`，这会直接杀死进程，无法在 Rust 层捕获。

**建议**:
- 将极端内存测试独立运行（避免影响其他测试）
- 使用 `--test-threads=1` 防止并行测试叠加内存
- 考虑使用子进程隔离（`std::process::Command`）

### ⚠️ 并发测试的非确定性

部分测试（如 `test_channel_backpressure`、`test_concurrent_load_and_execute`）涉及时序竞争，可能在某些运行中通过，某些运行中失败（flaky tests）。

**缓解措施**:
- 增加重试次数（如运行 3 次取最优）
- 使用更宽松的断言阈值
- 记录失败时的详细日志用于分析

---

## 性能基准参考

| 测试类别 | 典型耗时 | 内存峰值 | CPU 峰值 |
|---------|---------|---------|---------|
| 异常输入 | 2-5s | <500MB | <50% |
| 超时控制 | 5-30s (含 timeout) | <200MB | 100% (busy loop) |
| 内存压力 | 10-60s | 1-2GB | <30% |
| 热更新 | 3-10s | <300MB | <40% |
| 并发资源 | 5-20s | 500MB-1GB | 80-100% |
| 跨平台 | 1-3s | <100MB | <20% |
| 沙箱安全 | 1-2s | <100MB | <10% |
| 日志洪泛 | 3-10s | <200MB | <30% |

---

## 与其他测试的区别

| 测试类型 | 关注点 | 示例 |
|---------|--------|------|
| **stress_tests** | 正常负载下的性能极限 | 1000 次连续扫描 |
| **memory_leak_tests** | 长期运行内存增长趋势 | 10,000 次重复操作 |
| **v8_limits_tests** | V8 引擎技术限制 | 堆大小、栈深度 |
| **concurrency_tests** | 多线程正确性 | 100 并发扫描 |
| **robustness_tests** ✅ | **异常输入+边界条件+恢复能力** | 畸形 URL、卡死、权限越界 |

---

## 故障排查

### 1. 测试卡住不退出

**可能原因**: 
- 插件内无限循环且未触发超时
- `PluginExecutor` 的 channel 满了且没有消费者

**排查**:
```bash
# 查看卡住的线程
ps aux | grep robustness_tests
kill -QUIT <pid>  # 触发线程 dump (Unix)
```

### 2. 内存测试触发 OOM Killer

**可能原因**: 
- 测试并行运行，内存叠加超过物理内存
- V8 堆限制超过系统可用内存

**解决**:
```bash
# 单线程运行
cargo test --test robustness_tests --release -- --ignored --test-threads=1

# 或只运行非内存类测试
cargo test --test robustness_tests --release -- --ignored --skip memory
```

### 3. 跨平台测试在 Windows 失败

**可能原因**: 
- 路径分隔符硬编码 `/`
- 换行符 `\n` vs `\r\n`
- 编码假设 UTF-8

**验证**:
```bash
# Windows 上手动运行
cargo test --test robustness_tests --release -- --ignored test_path_separator_handling --nocapture
```

---

## 扩展建议

### 1. 增加属性测试 (Property-based Testing)

使用 `proptest` 或 `quickcheck` 自动生成测试输入：

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_arbitrary_url_parsing(url in ".*") {
        let txn = create_transaction_with_url(&url);
        let result = executor.scan_transaction(txn).await;
        // 断言：不 panic，返回 Ok 或明确的 Err
        assert!(result.is_ok() || result.is_err());
    }
}
```

### 2. 增加 Fuzz 持续测试

集成 `cargo-fuzz` 或 AFL，对 `scan_transaction` / `execute_agent` 输入进行持续 fuzzing。

### 3. 监控与告警

在 CI 中运行鲁棒性测试，失败时自动通知：
- 测试超时 > 5 分钟
- 内存峰值 > 3GB
- 错误率 > 10%

---

## 参考文档

- [V8 Isolate & HandleScope](./V8_HANDLESCOPE_ERROR.md)
- [PluginExecutor Restart Design](./EXECUTOR_WITH_RESTART_DESIGN.md)
- [Stress Test Summary](../STRESS_TEST_SUMMARY.md)
- [Plugin System Architecture](./PLUGIN_ARCHITECTURE.md)

---

**最后更新**: 2025-12-22  
**维护者**: Sentinel AI Team

