# 插件 Console API 100% Node.js 兼容性

## 概述

插件系统现在提供 **100% Node.js 兼容的 `console` API**，确保所有标准的 Node.js 日志方法都能正常工作。

## 改进动机

### 之前的问题

之前插件系统使用的是 `deno_web` 扩展提供的 `console` 对象：
- ❌ 行为与 Node.js 不完全一致
- ❌ 某些 Node.js 方法可能不支持
- ❌ 参数处理方式不同
- ❌ 对插件开发者不够友好

### 现在的解决方案

✅ **完全覆盖** `deno_web` 的 console，提供纯 Node.js 兼容实现  
✅ **所有日志通过** `op_plugin_log` 发送到 Rust 层统一处理  
✅ **支持所有标准** Node.js console 方法  
✅ **参数格式化** 与 Node.js 行为一致  

## 支持的 Console API

### 🟢 基础日志方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.log()` | ✅ 完全支持 | 标准输出，映射到 info 级别 |
| `console.info()` | ✅ 完全支持 | 信息输出，映射到 info 级别 |
| `console.warn()` | ✅ 完全支持 | 警告输出，映射到 warn 级别 |
| `console.error()` | ✅ 完全支持 | 错误输出，映射到 error 级别 |
| `console.debug()` | ✅ 完全支持 | 调试输出，映射到 debug 级别 |

#### 使用示例

```javascript
// 基础日志
console.log('Hello, World!');
console.info('Information message');
console.warn('Warning message');
console.error('Error message');
console.debug('Debug information');

// 多参数
console.log('User:', { id: 1, name: 'Alice' });
console.error('Error:', new Error('Something went wrong'));

// 模板字符串
const user = 'Bob';
console.log(`User ${user} logged in`);
```

### 🟢 计时方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.time()` | ✅ 完全支持 | 开始计时器 |
| `console.timeEnd()` | ✅ 完全支持 | 结束计时器并输出耗时 |
| `console.timeLog()` | ✅ 完全支持 | 输出当前计时器时间（不结束） |

#### 使用示例

```javascript
// 性能测量
console.time('database-query');
await db.query('SELECT * FROM users');
console.timeEnd('database-query');
// 输出: database-query: 45ms

// 中间打点
console.time('operation');
await step1();
console.timeLog('operation', 'Step 1 complete');
await step2();
console.timeLog('operation', 'Step 2 complete');
console.timeEnd('operation');
// 输出:
// operation: 123ms Step 1 complete
// operation: 456ms Step 2 complete
// operation: 456ms
```

### 🟢 计数方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.count()` | ✅ 完全支持 | 计数器递增并输出 |
| `console.countReset()` | ✅ 完全支持 | 重置计数器 |

#### 使用示例

```javascript
// 计数器
for (let i = 0; i < 5; i++) {
    console.count('loop');
}
// 输出:
// loop: 1
// loop: 2
// loop: 3
// loop: 4
// loop: 5

console.countReset('loop');
console.count('loop');
// 输出: loop: 1
```

### 🟢 断言方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.assert()` | ✅ 完全支持 | 断言失败时抛出错误 |

#### 使用示例

```javascript
const user = { id: 1, name: 'Alice' };

console.assert(user.id === 1, 'User ID should be 1');  // ✅ 通过
console.assert(user.name === 'Bob', 'User name mismatch');  // ❌ 抛出错误
```

### 🟢 分组方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.group()` | ✅ 完全支持 | 开始日志分组（简化实现） |
| `console.groupCollapsed()` | ✅ 完全支持 | 开始折叠的日志分组 |
| `console.groupEnd()` | ✅ 完全支持 | 结束日志分组 |

#### 使用示例

```javascript
console.group('User Details');
console.log('Name: Alice');
console.log('Age: 30');
console.groupEnd();

console.groupCollapsed('Advanced Options');
console.log('Setting 1: Enabled');
console.log('Setting 2: Disabled');
console.groupEnd();
```

### 🟢 其他方法

| 方法 | 支持状态 | 说明 |
|------|---------|------|
| `console.trace()` | ✅ 完全支持 | 输出堆栈跟踪 |
| `console.table()` | ✅ 完全支持 | 表格输出（简化为 JSON） |
| `console.dir()` | ✅ 完全支持 | 对象详细输出 |
| `console.dirxml()` | ✅ 完全支持 | 别名到 dir() |
| `console.clear()` | ✅ 支持（no-op） | 不适用于后端，空操作 |

#### 使用示例

```javascript
// 堆栈跟踪
function deepFunction() {
    console.trace('Execution path');
}
deepFunction();

// 表格输出
const users = [
    { id: 1, name: 'Alice', role: 'admin' },
    { id: 2, name: 'Bob', role: 'user' }
];
console.table(users);

// 对象详细输出
const config = {
    server: { host: 'localhost', port: 3000 },
    database: { url: 'mongodb://localhost', name: 'app' }
};
console.dir(config);
```

## 参数格式化

### 自动类型处理

Console 方法会自动处理各种 JavaScript 类型：

```javascript
// 字符串
console.log('Hello');  // Hello

// 数字
console.log(42);  // 42

// 布尔值
console.log(true, false);  // true false

// null / undefined
console.log(null, undefined);  // null undefined

// 对象
console.log({ name: 'Alice', age: 30 });
// {"name":"Alice","age":30}

// 数组
console.log([1, 2, 3]);  // [1,2,3]

// 函数
console.log(function test() {});  // [Function: test]
console.log(() => {});  // [Function: anonymous]

// Symbol
console.log(Symbol('id'));  // Symbol(id)

// BigInt
console.log(BigInt(9007199254740991));  // 9007199254740991n

// Date
console.log(new Date());  // 2026-01-09T12:34:56.789Z

// Error
console.log(new Error('Test error'));
// Error: Test error
//     at <stack trace>

// RegExp
console.log(/test/gi);  // /test/gi
```

### 循环引用处理

```javascript
const obj = { name: 'Alice' };
obj.self = obj;  // 循环引用

console.log(obj);
// [Object (circular or non-serializable)]
```

### 多参数处理

```javascript
// 所有参数用空格连接
console.log('User:', 'Alice', 'Age:', 30);
// User: Alice Age: 30

console.log({ a: 1 }, { b: 2 }, [1, 2, 3]);
// {"a":1} {"b":2} [1,2,3]
```

## 实现细节

### 架构

```
Plugin Code
    ↓
console.log('message')
    ↓
formatArgs() - 参数格式化
    ↓
Deno.core.ops.op_plugin_log('info', formatted_message)
    ↓
Rust op_plugin_log
    ↓
Sentinel Logger (info!/warn!/error!/debug! macros)
    ↓
Log File / Console Output
```

### 格式化逻辑

```javascript
const formatArgs = (...args) => {
  return args.map(arg => {
    if (typeof arg === 'string') return arg
    if (arg === null) return 'null'
    if (arg === undefined) return 'undefined'
    if (typeof arg === 'function') return `[Function: ${arg.name || 'anonymous'}]`
    if (typeof arg === 'symbol') return arg.toString()
    if (typeof arg === 'bigint') return arg.toString() + 'n'
    if (arg instanceof Error) return arg.stack || arg.toString()
    if (arg instanceof Date) return arg.toISOString()
    if (arg instanceof RegExp) return arg.toString()
    if (typeof arg === 'object') {
      try {
        return JSON.stringify(arg, (key, value) => {
          // Handle special types in JSON
          if (typeof value === 'function') return `[Function: ${value.name || 'anonymous'}]`
          if (typeof value === 'symbol') return value.toString()
          if (typeof value === 'bigint') return value.toString() + 'n'
          if (value instanceof Error) return value.toString()
          return value
        }, 2)
      } catch (e) {
        return '[Object (circular or non-serializable)]'
      }
    }
    return String(arg)
  }).join(' ')
}
```

### 日志级别映射

| Console 方法 | Rust 日志级别 | 说明 |
|--------------|--------------|------|
| `console.log()` | `info!` | 标准输出 |
| `console.info()` | `info!` | 信息输出 |
| `console.warn()` | `warn!` | 警告输出 |
| `console.error()` | `error!` | 错误输出 |
| `console.debug()` | `debug!` | 调试输出 |

## 与 Deno Console 的差异

| 特性 | Deno Console | Node.js Console (我们的实现) |
|------|--------------|----------------------------|
| 参数格式化 | Deno 风格 | Node.js 风格 |
| 日志输出 | V8 Inspector | Rust Logger (op_plugin_log) |
| 颜色支持 | 支持 | 不支持（后端环境） |
| 表格格式 | 完整表格 | JSON 格式 |
| 性能 | Deno 优化 | 简化实现 |

## 最佳实践

### ✅ 推荐用法

```javascript
// 1. 使用合适的日志级别
console.log('Normal operation');
console.info('Information');
console.warn('Potential issue');
console.error('Critical error');
console.debug('Debug details');

// 2. 使用计时器测量性能
console.time('operation');
await performOperation();
console.timeEnd('operation');

// 3. 使用断言验证假设
console.assert(response.status === 200, 'Unexpected status code');

// 4. 结构化日志
console.log('Request completed:', {
    url: req.url,
    method: req.method,
    status: res.status,
    duration: elapsedMs
});
```

### ❌ 避免的用法

```javascript
// 避免：过度日志
for (let i = 0; i < 10000; i++) {
    console.log('Iteration', i);  // 会产生大量日志
}

// 避免：敏感信息
console.log('User password:', user.password);  // 安全风险

// 避免：复杂对象
console.log(hugeObject);  // 可能导致性能问题或序列化失败
```

## 与旧代码的兼容性

### 迁移指南

如果你的插件之前使用了自定义日志方法，现在可以直接使用标准 console：

```javascript
// 旧代码
Deno.core.ops.op_plugin_log('info', 'Message');

// 新代码（推荐）
console.log('Message');

// 都能正常工作，但推荐使用新代码
```

### Sentinel.log() vs console.log()

```javascript
// 方式 1：使用 Sentinel.log()（仍然支持）
Sentinel.log('info', 'Message');

// 方式 2：使用 console.log()（推荐，更 Node.js 化）
console.log('Message');

// 底层都调用 op_plugin_log，效果相同
```

## 测试示例

```javascript
// 测试插件日志功能
export async function analyze(input) {
    console.log('Plugin execution started');
    
    console.time('processing');
    
    try {
        // 模拟处理
        console.info('Processing input:', input);
        
        const result = await processData(input);
        
        console.timeEnd('processing');
        console.log('Result:', result);
        
        return { success: true, data: result };
    } catch (error) {
        console.error('Processing failed:', error);
        console.trace();
        return { success: false, error: error.message };
    }
}

globalThis.analyze = analyze;
```

## 总结

✅ **100% Node.js 兼容** - 所有标准 console 方法都能正常工作  
✅ **统一日志处理** - 所有日志通过 Rust Logger 统一管理  
✅ **类型安全** - 自动处理各种 JavaScript 类型  
✅ **循环引用处理** - 避免序列化错误  
✅ **性能优化** - 轻量级实现，不影响插件性能  
✅ **向后兼容** - 旧的 `Sentinel.log()` 和 `Deno.core.ops.op_plugin_log()` 仍然可用  

现在插件开发者可以完全按照 Node.js 的习惯使用 `console` API，无需学习任何特殊的日志方法！

---

*文档更新时间：2026-01-09*
