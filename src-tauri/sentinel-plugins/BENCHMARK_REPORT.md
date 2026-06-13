# Sentinel 插件系统 QuickJS 引擎 Benchmark 报告

> 测试时间: 2026-06-03  
> 平台: macOS (darwin 25.3.0)  
> 引擎: QuickJS-NG via rquickjs 0.11 + sentinel-js  
> 框架: criterion 0.5 (统计学微基准测试)

---

## 1. 运行时生命周期 (`runtime_lifecycle`)

测量 JS 运行时从创建到可执行状态的全链路耗时。

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `sentinel_js_runtime_create` | **948 µs** | 创建 SentinelJsRuntime（含 Web/Node API 支持） |
| `sentinel_js_runtime_create + register_full_perms` | **1.225 ms** | 创建 + 注册全部宿主函数（网络/文件/字典/TLS/监控/AST） |
| `sentinel_js_runtime_create + register_sandboxed` | **1.032 ms** | 创建 + 仅注册 fetch 权限（沙箱模式） |
| `sentinel_js_runtime_full_bootstrap` | **1.262 ms** | 完整初始化：创建 + 注册 + Bootstrap shim |
| `plugin_engine_new` | **1.314 ms** | PluginEngine 端到端创建（生产路径） |
| `qjs_runtime_create` | **105 µs** | 创建裸 QjsPluginRuntime（直接 rquickjs 封装） |
| `qjs_runtime_create + register_full_perms` | **110 µs** | 创建 + 注册全部宿主函数 |
| `qjs_runtime_full_bootstrap` | **616 µs** | 完整初始化：创建 + 注册 + Bootstrap |

### 分析

- **SentinelJsRuntime vs QjsPluginRuntime 创建速度差距约 9 倍**（948µs vs 105µs），SentinelJsRuntime 内置了完整的 Web API（fetch polyfill、URL、crypto.subtle、TextEncoder 等）和 Node.js 兼容层（require、Buffer、process、path），这些额外的 JS 环境初始化是主要开销来源。
- **宿主函数注册开销极低**：QjsPluginRuntime 注册全部函数仅增加 ~5µs，SentinelJsRuntime 注册增加 ~277µs（因为 HostBindingsExt 需要通过 JSON 序列化桥接层注册）。
- **Bootstrap 是最大的固定开销**：包含 Sentinel 全局对象、Deno 兼容 shim、fetch bridge 等 JS 代码注入，占总初始化时间的 ~30%。
- **PluginEngine::new() ≈ 1.3ms** — 这是生产环境中每次创建新引擎实例的基准成本。

### 权限注册对比

| 权限级别 | SentinelJsRuntime | QjsPluginRuntime |
|----------|-------------------|------------------|
| 无权限 | 919 µs | 104 µs |
| 仅 Fetch | 961 µs | 105 µs |
| 全部权限 | 1.103 ms | 110 µs |

权限越多，注册的宿主函数越多。SentinelJsRuntime 的边际成本更高（~90µs/级），QjsPluginRuntime 几乎可忽略（~3µs/级）。

---

## 2. TypeScript 编译 (`ts_compilation`)

测量 oxc 对 TypeScript 源码的类型剥离性能。

| 测试项 | 中位数 | 源码规模 |
|--------|--------|----------|
| `ts_strip/small` | **2.23 µs** | ~5 行（interface + type + const） |
| `ts_strip/medium` | **13.2 µs** | ~50 行（完整安全头检查插件） |
| `ts_strip/large` | **194 µs** | ~500 行（50 个 interface + function） |
| `strip_for_script/medium` | **15.6 µs** | strip + export 关键字移除 |
| `strip_for_script/large` | **217 µs** | strip + export 关键字移除 |

### 编译缓存

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `compile_cache/cold_miss` | **18.0 µs** | 缓存未命中：strip + 存入缓存 |
| `compile_cache/warm_hit` | **2.34 µs** | 缓存命中：仅 hash 比对 + 返回 |
| `hash_plugin_source` | **2.30 µs** | FNV-1a hash 计算（中等源码） |

### 分析

- **oxc 的 TypeScript 剥离性能极优**：500 行 TS 代码仅需 ~194µs（含 parse → semantic → transform → codegen 全流程）。
- **编译缓存命中时快 7.7 倍**（2.34µs vs 18.0µs），缓存命中路径几乎等于一次 hash 计算的成本。
- **strip_for_script 额外开销约 12%**（逐行扫描移除 `export` 关键字），对于大文件这个开销是值得的，因为它避免了 QuickJS 的 ESM 模块加载开销。

---

## 3. 插件加载 (`plugin_loading`)

测量从 PluginEngine 创建到插件代码可执行的端到端耗时。

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `load_simple_js` | **1.303 ms** | 简单 JS 插件（~20 行） |
| `load_medium_js` | **1.554 ms** | 中等 JS 插件（~80 行，正则/遍历） |
| `load_typescript_plugin` | **1.354 ms** | TypeScript 插件（需 oxc strip） |
| `load_agent_plugin` | **1.306 ms** | Agent 工具插件（analyze/get_input_schema） |
| `load_heavy_computation` | **1.371 ms** | 重计算插件（多正则/token 分析） |

### 热重载

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `reload_same_code_skip` | **2.73 µs** | 同代码重载跳过（hash 比对） |
| `reload_changed_code` | **14.5 µs** | 代码变更后重新加载 |

### 分析

- **插件加载耗时主要由引擎创建主导**：1.3ms 基础开销中，~1.26ms 来自 PluginEngine::new()，实际 JS 代码 eval 仅占 40~250µs。
- **TypeScript 插件并不显著慢于 JS 插件**（1.354ms vs 1.303ms），因为 oxc strip 仅增加 ~15µs，且编译缓存会消除后续加载的 strip 开销。
- **热重载路径极快**：当 plugin_id + code_hash 都匹配时，仅需 2.73µs（跳过 strip + eval），比完整加载快 **477 倍**。这对引擎池的引擎复用至关重要。
- **代码变更重载 14.5µs** — 这是 TS strip(cached) + eval 的成本，因为引擎已存在，不需要重建。

---

## 4. 插件执行 (`plugin_execution`)

测量已加载插件的执行性能（不含加载时间）。

### scan_transaction（流量扫描）

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `simple_plugin × small_txn` | **32.1 µs** | 简单 URL 检查 + 小请求 |
| `medium_plugin × small_txn` | **25.8 µs** | 头部/正则检查 + 小请求 |
| `medium_plugin × large_txn` | **950 µs** | 头部/正则检查 + 500 条用户记录 |
| `heavy_computation × large_txn` | **931 µs** | hash + tokenize + 7 种正则 + 大请求 |
| `typescript_plugin × small_txn` | **53.6 µs** | TS 安全头检查 + 小请求 |

### execute_agent（Agent 工具调用）

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `agent_small_input` | **26.2 µs** | 1 个 target 的 analyze 调用 |
| `agent_many_targets` | **170 µs** | 100 个 targets 的 analyze 调用 |
| `get_input_schema` | **15.2 µs** | 获取插件输入 schema |

### 端到端（创建 + 加载 + 执行）

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `create_load_scan_simple` | **1.229 ms** | 全流程：简单插件扫描小请求 |
| `create_load_scan_medium` | **2.499 ms** | 全流程：中等插件扫描大请求 |
| `create_load_agent_call` | **1.229 ms** | 全流程：Agent 插件执行 |

### 分析

- **小请求扫描非常快**：25~53µs，即使是 TypeScript 插件也在 54µs 内完成。这意味着 QuickJS 单线程每秒可处理约 **20,000~40,000 次**小请求扫描。
- **大请求扫描的瓶颈是 JSON 序列化**：large_txn 包含 500 条用户记录（~100KB JSON），序列化 + 注入 JS 全局变量占了 ~800µs（参见 JSON Bridge 报告），实际 JS 执行仅 ~130µs。
- **medium_plugin 反而比 simple_plugin 更快**（25.8µs vs 32.1µs）：因为 simple_plugin 的 URL indexOf 检查命中了（URL 包含 "admin"），触发了 `Sentinel.emit` 宿主函数调用（~8µs 开销），而 medium_plugin 在 small_txn 上没有匹配项。
- **Agent 调用和 Scan 调用性能基本一致**（~26µs），说明 `call_plugin_function` 的 async IIFE 包装和 Promise.resolve 开销很小。
- **端到端耗时中，引擎创建 + 加载占 97%+**，实际执行仅占 2~3%。这强化了引擎池复用策略的重要性。

---

## 5. JSON 桥接 (`json_bridge`)

测量 Rust ↔ JS 之间的 JSON 数据传递性能。

### set_json_global（Rust → JS）

| 测试项 | SentinelJsRuntime | QjsPluginRuntime | JSON 大小 |
|--------|--------------------|------------------|-----------|
| small | **370 ns** | 495 ns | ~60 bytes |
| medium | **2.09 µs** | 3.04 µs | ~400 bytes |
| large | **833 µs** | 1.146 ms | ~100 KB |

### serde 序列化开销

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `serialize_transaction` (to Value) | **221 µs** | HttpTransaction → serde_json::Value |
| `serialize_to_string` | **589 µs** | Value → JSON String |
| `deserialize_from_string` | **1.092 ms** | JSON String → Value |

### JS 内 JSON.parse vs Host set_global

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `eval_json_parse_in_js` | **7.83 µs** | JS 内 `JSON.parse()` 中等 JSON |
| `set_global_vs_json_parse` | **2.04 µs** | Host `set_json_global()` 中等 JSON |

### 分析

- **SentinelJsRuntime 的 JSON 注入比 QjsPluginRuntime 快约 25-30%**：SentinelJsRuntime 通过 `sentinel-js` 的 `set_global` API 直接操作，而 QjsPluginRuntime 需要逐层递归构建 JS 对象。
- **大 JSON 传递是性能热点**：100KB 的 HttpTransaction 需要 ~833µs 注入 JS，其中 serde 序列化（Value→内部结构）和 JS 对象构建各占约一半。
- **Host set_global 比 JS JSON.parse 快 3.8 倍**（2.04µs vs 7.83µs），因为 Host 路径跳过了 JSON 字符串中间表示，直接从 Rust Value 构建 JS 对象。
- **对于大流量扫描，JSON 序列化是单次扫描耗时的主要瓶颈**（占 scan_transaction large_txn 总耗时的 ~88%）。优化建议：考虑增量/流式 JSON 注入，或只注入插件实际需要的字段子集。

---

## 6. 引擎池 (`engine_pool`)

测量引擎池的调度、缓存和并发性能。

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `scan_cold_start` | **1.408 ms** | 冷启动：创建新引擎 + 加载 + 扫描 |
| `scan_warm_cached` | **46.5 µs** | 热缓存：复用已有引擎扫描 |
| `agent_cached` | **37.2 µs** | 热缓存：复用引擎执行 Agent |
| `pool_evict_and_recreate` | **1.399 ms** | 驱逐 + 重建引擎 |
| `4_concurrent_scans_same_plugin` | **1.576 ms** | 4 并发扫描（同一插件） |
| `4_concurrent_scans_different_plugins` | **1.787 ms** | 4 并发扫描（不同插件） |
| `get_stats` | **12.6 µs** | 查询池统计信息 |

### 分析

- **引擎缓存的效果极其显著**：冷启动 1.408ms vs 热缓存 46.5µs，**快 30 倍**。这验证了 EnginePool 的 cache affinity 设计（基于 plugin_id 的一致性 hash 分发到固定 worker）是高效的。
- **Agent 缓存比 Scan 缓存略快**（37.2µs vs 46.5µs），因为 Agent 输入 JSON 比 HttpTransaction 小得多。
- **evict + recreate ≈ cold_start**，说明驱逐操作本身几乎零开销（仅从 HashMap 移除），重建成本完全来自新引擎创建。
- **4 并发扫描**：同一插件 1.576ms，不同插件 1.787ms。不同插件时每个 worker 都需要冷启动创建引擎（因为缓存中没有），所以整体慢约 13%。在稳态运行时，所有常用插件都会被缓存。
- **池统计查询 12.6µs**：轻量级跨 worker channel 通信，不会成为瓶颈。

---

## 7. 宿主函数开销 (`host_functions`)

测量 JS 调用 Rust 宿主函数的单次开销。

### 宿主函数调用

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `sentinel_log` | **2.62 µs** | `__sentinel_log("debug", "msg")` |
| `sentinel_return` | **5.13 µs** | `__sentinel_return({...})` 小 payload |
| `sentinel_emit_finding` | **8.31 µs** | `__sentinel_emit_finding({...})` 完整 Finding |
| `sentinel_return_large_payload` | **181 µs** | return 含 100 个对象的大结果 |

### JS 执行基准

| 测试项 | 中位数 | 说明 |
|--------|--------|------|
| `eval_empty` | **792 ns** | 空字符串 eval（解释器基础开销） |
| `eval_simple_expression` | **2.22 µs** | `var x = 1 + 2 * 3` |
| `eval_string_ops` | **6.02 µs** | split + map + toUpperCase + join |
| `eval_regex_match` | **5.48 µs** | SQL 注入正则匹配 |
| `eval_loop_1000` | **47.3 µs** | 1000 次循环累加 |
| `eval_object_creation` | **30.6 µs** | 创建 100 个 `{id, name}` 对象 |
| `eval_json_stringify_parse` | **5.73 µs** | JSON.stringify + JSON.parse roundtrip |
| `eval_promise_resolve` | **5.53 µs** | async IIFE + Promise.resolve |

### 两种 Runtime 对比

| 测试项 | SentinelJsRuntime | QjsPluginRuntime |
|--------|-------------------|------------------|
| loop_1000 | **45.7 µs** | **46.2 µs** |

### 分析

- **空 eval 开销仅 792ns**：QuickJS 的解释器 dispatch 非常轻量，作为 baseline 这是优秀的。
- **宿主函数调用开销合理**：log 2.6µs、return 5.1µs、emit_finding 8.3µs。开销来源是 JS→Rust 的值序列化（JSON bridge）和 PluginContext 的 Mutex 锁操作。
- **大 payload return 开销线性增长**：100 个对象的结果需要 181µs，这是 JS 对象→JSON→serde_json::Value 的序列化成本。
- **Promise.resolve 仅 5.53µs**：async IIFE 包装的额外开销可以忽略，不会影响插件执行性能。
- **两种 Runtime 纯计算性能完全一致**（45.7µs vs 46.2µs），因为底层都是同一个 QuickJS-NG 引擎。差异仅在初始化成本和 API 层抽象上。

---

## 8. 内存开销 (`memory_overhead`)

### 运行时内存占用

| 测试项 | 每实例内存 | 批量创建 10 个耗时 |
|--------|-----------|-------------------|
| SentinelJsRuntime（全部 API） | **~98 KB** | 11.7 ms |
| QjsPluginRuntime（直接封装） | **~10 KB** | 5.86 ms |
| PluginEngine（已加载中等插件） | **~3 KB**（增量） | 1.4 ms/个 |

### 编译缓存

| 测试项 | 中位数 |
|--------|--------|
| 填充 50 条缓存条目 | **99.7 µs** |

### 分析

- **SentinelJsRuntime 内存约 98KB/实例**：包含 QuickJS runtime + context + Web API polyfills + Node compat 层。对于桌面应用，同时运行 100 个 runtime 约占 10MB，完全可接受。
- **QjsPluginRuntime 仅 10KB/实例**：极其轻量，适合需要大量并发引擎的场景。
- **PluginEngine 加载后增量仅 ~3KB**：插件代码编译后的 bytecode 很紧凑，说明 QuickJS 的内存效率优秀。
- **编译缓存的内存开销可忽略**：50 条缓存条目总操作时间仅 100µs，每条目包含 stripped JS 源码 + metadata。

---

## 总结与关键指标

| 指标 | 值 | 评价 |
|------|-----|------|
| 引擎创建（生产路径） | **1.3 ms** | 良好 — 池化后可忽略 |
| 小请求单次扫描 | **26~54 µs** | 优秀 — 理论 QPS ~20K/线程 |
| 大请求单次扫描 | **930~950 µs** | 良好 — 瓶颈在 JSON 序列化 |
| Agent 工具调用 | **26 µs** | 优秀 |
| 引擎池缓存命中 | **46 µs** | 优秀 — 比冷启动快 30x |
| TS 编译（中等文件） | **13 µs** | 极优 — oxc 性能出色 |
| 编译缓存命中 | **2.3 µs** | 极优 |
| 热重载（代码未变） | **2.7 µs** | 极优 — hash 比对即返回 |
| 内存/引擎（SentinelJs） | **~98 KB** | 良好 |
| 内存/引擎（QjsDirect） | **~10 KB** | 优秀 |

### 性能瓶颈排序（单次 scan_transaction 大请求）

```
JSON 序列化 (Rust→JS)    ~833 µs  ███████████████████████████████████  88%
JS 插件执行逻辑           ~100 µs  ████                                11%
宿主函数调用开销            ~8 µs  ▏                                    1%
```

### 优化建议

1. **JSON 传递优化**：对大型 HttpTransaction，考虑 lazy 字段注入（只在 JS 首次访问时才序列化该字段），可减少 80%+ 的序列化开销。
2. **引擎池预热**：在应用启动时预创建常用插件的引擎实例，避免首次请求的 1.3ms 冷启动延迟。
3. **编译缓存持久化**：当前 CompileCache 是内存 HashMap，重启后丢失。考虑持久化到磁盘（~2µs/hit 已经足够快，瓶颈不在这里，但可避免重启后的首次 strip 开销）。
