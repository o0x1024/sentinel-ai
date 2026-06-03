# Sentinel Plugins - 文档索引

## 📚 核心文档

### 问题分析
- [V8_HANDLESCOPE_ERROR.md](./V8_HANDLESCOPE_ERROR.md) - HandleScope 错误详细分析
- [V8_RESTART_ISSUE.md](./V8_RESTART_ISSUE.md) - 重启问题深入探讨  
- [MEMORY_LEAK_SOLUTION.md](./MEMORY_LEAK_SOLUTION.md) - 内存泄漏解决方案

### 设计文档
- [EXECUTOR_VS_ENGINE.md](./EXECUTOR_VS_ENGINE.md) - Engine 与 Executor 对比
- [EXECUTOR_WITH_RESTART_DESIGN.md](./EXECUTOR_WITH_RESTART_DESIGN.md) - 重启机制设计

### 测试文档
- [ROBUSTNESS_TESTS.md](./ROBUSTNESS_TESTS.md) - 鲁棒性与健壮性测试指南
- [ROBUSTNESS_TEST_REPORT.md](./ROBUSTNESS_TEST_REPORT.md) - 鲁棒性测试完整报告 ⭐

### 完整解决方案
- [V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md](./V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md) ⭐
  - 最全面的文档，包含问题、原因、解决方案、使用模式和性能数据

## 🎯 快速导航

### 我想了解...

| 问题 | 推荐文档 |
|------|---------|
| 为什么会报 HandleScope 错误？ | [V8_HANDLESCOPE_ERROR.md](./V8_HANDLESCOPE_ERROR.md) |
| 如何解决这个错误？ | [V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md](./V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md) |
| PluginEngine 和 PluginExecutor 有什么区别？ | [EXECUTOR_VS_ENGINE.md](./EXECUTOR_VS_ENGINE.md) |
| 如何使用重启功能？ | [EXECUTOR_WITH_RESTART_DESIGN.md](./EXECUTOR_WITH_RESTART_DESIGN.md) |
| 如何解决内存泄漏？ | [MEMORY_LEAK_SOLUTION.md](./MEMORY_LEAK_SOLUTION.md) |
| 完整的技术方案？ | [V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md](./V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md) ⭐ |
| 如何测试系统鲁棒性？ | [ROBUSTNESS_TESTS.md](./ROBUSTNESS_TESTS.md) |

## 🚀 快速开始

### 基本用法

```rust
use sentinel_plugins::PluginExecutor;

// 创建executor
let executor = PluginExecutor::new(metadata, code, 1000)?;

// 使用
let findings = executor.scan_transaction(transaction).await?;

// 重启
executor.restart().await?;
```

### 定期重启模式

```rust
// 后台任务：定期检查并重启
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let stats = executor.get_stats().await.unwrap();
        if stats.current_instance_executions >= 900 {
            executor.restart().await.ok();
        }
    }
});
```

## 📊 关键数据

| 指标 | 数值 |
|------|------|
| 重启开销 | 50-150ms |
| 内存控制 | < 1 MB/s（相比 150 MB/s） |
| 推荐阈值 | 1000-2000 次执行 |
| 稳定性 | ✅ 生产可用 |

## 🧪 测试

### 重启机制测试
```bash
# 运行所有重启测试
cargo test --test executor_restart_tests --release -- --ignored --nocapture

# 手动重启测试
cargo test --test executor_restart_tests --release -- --ignored test_manual_restart
```

### 鲁棒性测试
```bash
# 运行完整鲁棒性测试套件
./run_stress_tests.sh robustness

# 或单独运行
cargo test --test robustness_tests --release -- --ignored --nocapture
```

详见: [ROBUSTNESS_TESTS.md](./ROBUSTNESS_TESTS.md)

## 📖 推荐阅读顺序

1. **新手**: 先读 [V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md](./V8_HANDLESCOPE_ERROR_COMPLETE_SOLUTION.md)
2. **深入了解**: 再读 [V8_HANDLESCOPE_ERROR.md](./V8_HANDLESCOPE_ERROR.md) 和 [V8_RESTART_ISSUE.md](./V8_RESTART_ISSUE.md)
3. **实现细节**: 最后读 [EXECUTOR_WITH_RESTART_DESIGN.md](./EXECUTOR_WITH_RESTART_DESIGN.md)

## 🎯 核心原则

**一个线程，一个 Isolate，一个生命周期**

V8 不支持在同一线程上连续创建多个 Isolate，解决方案是每次重启时创建新线程。

---

**最后更新**: 2025-12-22

