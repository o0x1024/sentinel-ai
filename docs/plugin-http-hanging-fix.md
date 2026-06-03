# HTTP 插件执行挂起无响应问题修复

## 问题描述

用户执行 `http.js` 插件时，插件一直没有响应，表现为：
- 日志显示 `Plugin executor started for _`
- 但插件执行一直没有完成
- 界面显示加载状态（转圈圈）

## 问题根本原因

### 🔍 事件监听器注册时序问题

这是一个经典的 **事件触发时机** 问题：

#### Node.js 标准行为

在真实的 Node.js 中，`http.request()` 的执行流程是：

```javascript
const req = http.request(options, (res) => {
    // 1️⃣ 回调被调用时，响应对象已经准备好
    // 2️⃣ 但响应数据还没有开始读取
    res.on('data', chunk => { /* ... */ });  // ✅ 可以注册监听器
    res.on('end', () => { /* ... */ });      // ✅ 可以注册监听器
});
req.end(); // 3️⃣ 触发请求，但不会立即完成
```

流程：
1. `req.end()` 发起网络请求（异步）
2. 网络连接建立后，调用 `callback(res)` 传递响应对象
3. 在回调中注册 `data` 和 `end` 事件监听器
4. 响应数据到达时，触发 `data` 事件
5. 响应结束时，触发 `end` 事件

#### 我们的实现问题

我们之前的实现：

```javascript
end: async (data) => {
    // ...
    const response = await fetch(url, { /* ... */ });  // 1️⃣ 立即完成整个请求
    
    const res = {
        on: (event, handler) => {
            resEventHandlers[event].push(handler)  // 3️⃣ 事件监听器
        },
    }
    
    responseBody = await response.text()  // 2️⃣ 立即读取完所有数据
    
    // ❌ 问题：先触发事件
    resEventHandlers.data.forEach(handler => handler(responseBody))  // 4️⃣ 触发 data 事件
    resEventHandlers.end.forEach(handler => handler())               // 5️⃣ 触发 end 事件
    
    if (callback) callback(res)  // 6️⃣ 最后才调用回调
}
```

**时序错误：**
1. `fetch()` 完成并读取完所有数据
2. 创建 `res` 对象（包含 `on` 方法）
3. **立即触发** `data` 和 `end` 事件（此时 `resEventHandlers` 是空的）
4. **最后才调用** `callback(res)`
5. 插件代码在回调中注册 `res.on('data')` 和 `res.on('end')` **但事件已经触发过了！**

### 💥 导致的问题

在 `http.js` 插件中（第135-154行）：

```javascript
const req = protocol.request(options, (res) => {
    let data = '';
    
    // ❌ 这些监听器注册得太晚了！事件已经触发过了
    res.on('data', (chunk) => { data += chunk; });
    res.on('end', () => {
        // ❌ 这个 resolve 永远不会被调用
        resolve({
            status: res.statusCode,
            statusText: res.statusMessage,
            headers: res.headers,
            body: data,
            url: urlStr
        });
    });
});
```

**结果：**
- `res.on('data')` 和 `res.on('end')` 注册时，事件已经触发过了
- `resolve()` 永远不会被调用
- `performRequest()` 的 Promise 永远不会 resolve
- 整个 `analyze()` 函数挂起
- 插件执行无响应 🔴

## ✅ 解决方案

### 调整事件触发时序

**关键改变：先调用 callback，再触发事件**

```javascript
end: async (data) => {
    // ...
    const response = await fetch(url, { /* ... */ });
    
    const resEventHandlers = {
        data: [],
        end: [],
    }
    
    const res = {
        statusCode: response.status,
        statusMessage: response.statusText,
        headers: Object.fromEntries(response.headers.entries()),
        
        on: (event, handler) => {
            if (event === 'data' || event === 'end') {
                resEventHandlers[event].push(handler)  // ✅ 允许注册监听器
            }
            return res
        },
    }
    
    responseBody = await response.text()
    
    // ✅ 关键修改 1：先调用回调，让插件代码注册监听器
    if (callback) callback(res)
    
    // ✅ 关键修改 2：使用 queueMicrotask 延迟事件触发
    // 这确保回调函数执行完毕后才触发事件
    queueMicrotask(() => {
        // 现在 resEventHandlers 已经有监听器了！
        resEventHandlers.data.forEach(handler => handler(responseBody))
        resEventHandlers.end.forEach(handler => handler())
    })
}
```

### 为什么使用 `queueMicrotask()`？

`queueMicrotask()` 会将回调放入微任务队列，在当前同步代码执行完毕后立即执行：

```
执行流程：
1. callback(res) 被调用（同步）
   └─> 插件代码执行 res.on('data', handler)  ✅ 注册成功
   └─> 插件代码执行 res.on('end', handler)   ✅ 注册成功
2. callback(res) 返回
3. queueMicrotask 中的回调执行（微任务）
   └─> 触发 'data' 事件  ✅ 监听器已注册，能够收到
   └─> 触发 'end' 事件   ✅ 监听器已注册，能够收到
```

这模拟了 Node.js 中事件循环的异步行为。

## 验证

### 正确的执行流程

```javascript
// 插件代码
const req = protocol.request(options, (res) => {
    console.log('1. 回调被调用');
    let data = '';
    
    res.on('data', (chunk) => {
        console.log('3. data 事件触发:', chunk);
        data += chunk;
    });
    
    res.on('end', () => {
        console.log('4. end 事件触发');
        resolve({ body: data, ... });
    });
    
    console.log('2. 监听器注册完毕');
});

req.end();
```

**输出顺序：**
```
1. 回调被调用
2. 监听器注册完毕
3. data 事件触发: <响应内容>
4. end 事件触发
✅ Promise resolved，插件执行完成
```

### 测试场景

1. ✅ 基本 HTTP 请求
2. ✅ 带超时的请求
3. ✅ 错误处理
4. ✅ 重定向处理
5. ✅ POST/PUT 请求

## 技术细节

### 事件循环与微任务

JavaScript 事件循环有多个阶段：
1. **同步代码执行**
2. **微任务队列** (microtasks) - `queueMicrotask`, `Promise.then`
3. **宏任务队列** (macrotasks) - `setTimeout`, `setInterval`

使用 `queueMicrotask()` 而不是 `setTimeout(..., 0)` 的原因：
- ✅ 微任务会在当前任务完成后**立即**执行
- ❌ 宏任务会等到下一个事件循环周期才执行（可能有延迟）

### Node.js 原生实现对比

Node.js 原生的 `http` 模块使用 C++ 实现，通过 libuv 事件循环处理网络 I/O。它的行为是：
1. 网络连接建立 → 触发回调
2. 数据到达 → 触发 `data` 事件
3. 连接关闭 → 触发 `end` 事件

我们的实现使用 `fetch()` 一次性获取所有数据，因此需要人工模拟这个异步流程。

## 相关问题修复

这次修复同时解决了以下问题：
1. ✅ HTTP 请求挂起无响应
2. ✅ Promise 永远不 resolve
3. ✅ 插件执行超时
4. ✅ 事件监听器注册但从未触发

## 最佳实践

### 推荐：使用 `fetch()` API

对于新插件，推荐直接使用 `fetch()` API，避免事件回调的复杂性：

```javascript
// ✅ 简单直接，没有事件时序问题
const response = await fetch('http://example.com', {
    method: 'GET',
    headers: { 'User-Agent': 'Sentinel-AI' },
    timeout: 5000,
});

const data = await response.text();
console.log('Status:', response.status);
console.log('Body:', data);
```

### 可选：使用 `http`/`https` 模块

如果需要 Node.js 兼容性（例如移植现有代码），现在可以正常使用：

```javascript
const http = require('http');

const req = http.request('http://example.com', (res) => {
    let data = '';
    res.on('data', chunk => { data += chunk; });
    res.on('end', () => { console.log('Data:', data); });
});

req.on('error', err => console.error(err));
req.end();
```

## 相关文件

- `/Users/a1024/code/ai/sentinel-ai/src-tauri/sentinel-plugins/src/plugin_bootstrap.js` - 修复位置
- `/Users/a1024/code/ai/sentinel-ai/scripts/http.js` - 触发问题的插件

## 总结

此次修复解决了 HTTP 模块中事件触发时序错误导致的插件挂起问题。关键改进：

1. ✅ **先调用 callback，再触发事件** - 确保监听器有机会注册
2. ✅ **使用 `queueMicrotask()` 延迟事件触发** - 模拟异步 I/O 行为
3. ✅ **保持与 Node.js 行为一致** - 提高兼容性

这使得插件能够正确接收 HTTP 响应数据并完成执行。

---

*文档更新时间：2026-01-09*
