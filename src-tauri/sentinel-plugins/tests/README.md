# Plugin System Stress Tests

完整的插件系统压力测试套件，用于检测内存泄漏、CPU瓶颈、并发限制和V8引擎极限。

## 测试分类

### 1. 基础压力测试 (`stress_tests.rs`)
- ✅ 单引擎长时间运行（10,000次迭代）
- ✅ 并发引擎创建和销毁（100个引擎，并发度10）
- ✅ CPU密集型插件压力测试
- ✅ 内存密集型插件压力测试
- ✅ 最大并发线程数测试（10-1000并发）
- ✅ PluginExecutor长期运行测试（5,000次迭代）
- ✅ 大数据量事务处理（1KB-5MB）
- ✅ PluginManager并发扫描（1,000次，50并发）

### 2. 内存泄漏测试 (`memory_leak_tests.rs`)
- ✅ 简单插件长时间运行（30秒持续测试）
- ✅ 大对象分配内存泄漏检测
- ✅ 闭包和循环引用检测
- ✅ 字符串拼接内存泄漏
- ✅ 异步操作内存泄漏
- ✅ 多引擎实例内存隔离

**检测方法**: 使用线性回归分析内存增长趋势，斜率>0.1 MB/s判定为泄漏

### 3. CPU密集型测试 (`cpu_stress_tests.rs`)
- ✅ 正则表达式回溯爆炸
- ✅ 大数据排序和过滤（50,000条数据）
- ✅ 递归算法（斐波那契、阶乘、Ackermann）
- ✅ 密集数学计算（质数、矩阵乘法、三角函数）
- ✅ 字符串处理密集型
- ✅ 并发CPU密集型任务（200次迭代，10并发）

### 4. 并发压力测试 (`concurrency_tests.rs`)
- ✅ 逐步增加并发数找系统极限（10-2000）
- ✅ 持续高并发压力测试（60秒，100并发）
- ✅ PluginExecutor并发测试（1,000次，100并发）
- ✅ PluginManager多插件并发（10个插件，500次迭代）
- ✅ 竞态条件测试（共享状态）
- ✅ 死锁检测测试
- ✅ 线程池耗尽测试

### 5. V8引擎限制测试 (`v8_limits_tests.rs`)
- ✅ 堆内存限制（尝试分配1000个1MB数组）
- ✅ 栈溢出测试（深度递归）
- ✅ 无限循环检测（10,000,000次迭代）
- ✅ 大对象分配（1KB-10MB）
- ✅ 字符串长度限制（1KB-100MB）
- ✅ 对象属性数量限制（100-1,000,000个属性）
- ✅ 多引擎隔离测试

### 6. 网络操作压力测试 (`network_stress_tests.rs`)
- ✅ 10,000个并发网络请求
- ✅ 逐步增加并发数找网络极限（10-2000）
- ✅ 每个插件并发多个HTTP请求
- ✅ 网络超时处理测试
- ✅ 持续网络压力测试（60秒）
- ✅ 不同网络条件下的性能测试

## 运行测试

### 运行所有测试
```bash
cd src-tauri/sentinel-plugins
cargo test --tests --release -- --ignored --nocapture
```

### 运行特定分类测试

```bash
# 基础压力测试
cargo test --test stress_tests --release -- --ignored --nocapture

# 内存泄漏测试
cargo test --test memory_leak_tests --release -- --ignored --nocapture

# CPU密集型测试
cargo test --test cpu_stress_tests --release -- --ignored --nocapture

# 并发测试
cargo test --test concurrency_tests --release -- --ignored --nocapture

# V8限制测试
cargo test --test v8_limits_tests --release -- --ignored --nocapture

# 网络压力测试
cargo test --test network_stress_tests --release -- --ignored --nocapture
```

### 运行单个测试

```bash
# 示例：运行最大并发测试
cargo test --test concurrency_tests test_find_max_concurrency --release -- --ignored --nocapture

# 示例：运行内存泄漏检测
cargo test --test memory_leak_tests test_simple_plugin_memory_leak --release -- --ignored --nocapture

# 示例：运行10,000并发网络请求测试
cargo test --test network_stress_tests test_10k_concurrent_requests --release -- --ignored --nocapture
```

## 预期结果

### 内存使用
- **正常范围**: 50-500 MB
- **警告阈值**: > 1 GB
- **危险阈值**: > 2 GB 或持续增长

### CPU使用
- **轻量插件**: 10-30%
- **CPU密集型**: 50-90%
- **异常情况**: 持续100%

### 并发能力
- **推荐并发数**: 50-100
- **最大并发数**: 200-500（取决于系统配置）
- **错误率阈值**: < 5%

### 吞吐量
- **简单插件**: 100-500 ops/sec
- **复杂插件**: 10-50 ops/sec
- **CPU密集型**: 1-10 ops/sec

## 故障场景

### 场景1: 内存泄漏
**症状**: 内存持续增长，不回收
**检测**: `memory_leak_tests.rs` 中的线性回归检测
**原因**:
- 全局变量累积
- 闭包捕获大对象
- 事件监听器未清理

### 场景2: CPU爆炸
**症状**: CPU使用率持续100%
**检测**: `cpu_stress_tests.rs` 监控CPU使用率
**原因**:
- 正则回溯爆炸
- 无限循环
- 大数据处理

### 场景3: 并发死锁
**症状**: 任务卡住，无响应
**检测**: `concurrency_tests.rs` 的超时检测
**原因**:
- 锁顺序不一致
- 资源竞争

### 场景4: V8堆溢出
**症状**: 插件执行失败，OOM错误
**检测**: `v8_limits_tests.rs` 的大对象分配
**原因**:
- 分配超大数组
- 字符串拼接无限增长

## 性能基准

基于 MacBook Pro M1 Max (32GB RAM, 10 cores):

| 测试类型 | 并发数 | 吞吐量 | 内存峰值 | CPU峰值 |
|---------|--------|--------|----------|---------|
| 简单插件 | 100 | 250 ops/s | 150 MB | 45% |
| CPU密集型 | 10 | 8 ops/s | 200 MB | 95% |
| 内存密集型 | 50 | 15 ops/s | 800 MB | 60% |
| 长时间运行 | 1 | 300 ops/s | 120 MB | 25% |

## 故障排查

### 测试失败
1. 检查系统资源（内存、CPU）
2. 查看错误日志
3. 降低并发数重试
4. 使用 `--nocapture` 查看详细输出

### 性能下降
1. 检查是否有其他进程占用资源
2. 确认使用 `--release` 模式
3. 调整测试参数（迭代次数、并发数）

### 内存泄漏误报
1. 延长测试时间（确保GC有机会运行）
2. 检查基线内存是否稳定
3. 多次运行确认趋势

## 添加新测试

1. 在对应的测试文件中添加测试函数
2. 使用 `#[tokio::test]` 和 `#[ignore]` 标记
3. 添加资源监控（内存、CPU）
4. 记录测试结果
5. 更新本README

示例：
```rust
#[tokio::test]
#[ignore]
async fn test_my_scenario() {
    let mut monitor = ResourceMonitor::new();
    let start = Instant::now();
    
    // 测试逻辑
    
    let duration = start.elapsed();
    let (peak_mem, avg_mem, peak_cpu, avg_cpu) = monitor.get_stats();
    
    println!("Test completed in {:?}", duration);
    println!("Memory: peak={:.2}MB, avg={:.2}MB", peak_mem, avg_mem);
    println!("CPU: peak={:.2}%, avg={:.2}%", peak_cpu, avg_cpu);
}
```

## 生成报告

测试完成后，使用 `stress_test_runner` 生成报告：

```rust
use stress_test_runner::{TestSuiteReport, TestResult, TestCategory};

let mut report = TestSuiteReport::new();
report.add_result(result);
report.generate_recommendations();

// Markdown报告
report.save_to_file("stress_test_report.md").unwrap();

// JSON报告
report.save_json_to_file("stress_test_report.json").unwrap();

// 控制台报告
report.print_console_report();
```

## 持续集成

建议在CI中定期运行压力测试：

```yaml
# .github/workflows/stress-test.yml
name: Stress Tests
on:
  schedule:
    - cron: '0 0 * * 0'  # 每周日运行
  workflow_dispatch:

jobs:
  stress-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run stress tests
        run: |
          cd src-tauri/sentinel-plugins
          cargo test --tests --release -- --ignored --nocapture > stress_test_output.log
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: stress-test-results
          path: stress_test_output.log
```

## 注意事项

1. **测试时间**: 完整测试套件需要30-60分钟
2. **系统资源**: 确保有足够的内存和CPU资源
3. **Release模式**: 务必使用 `--release` 获得准确结果
4. **隔离环境**: 避免在生产环境运行
5. **监控**: 使用系统监控工具（htop、Activity Monitor）观察资源使用

## 依赖

测试需要以下依赖（已在 `Cargo.toml` 中配置）：

```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
sysinfo = "0.30"
uuid = { version = "1.0", features = ["v4"] }
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
num_cpus = "1.0"
```

## 联系与反馈

如果发现新的故障场景或有改进建议，请提交Issue或PR。

