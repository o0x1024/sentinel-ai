# browser_open 工具使用说明

## 功能概述

`browser_open` 工具用于在浏览器中打开 URL 并获取页面快照。支持自动补全 URL 协议前缀和可选的浏览器显示模式。

## 参数说明

### url (必需)
- **类型**: string
- **描述**: 要打开的 URL
- **支持格式**:
  - 完整 URL: `https://example.com`
  - 简短域名: `example.com` (自动添加 `https://` 前缀)
  - 带路径: `example.com/path/to/page`
  - 本地文件: `file:///path/to/file.html`

### wait_until (可选)
- **类型**: string
- **默认值**: `"load"`
- **可选值**:
  - `"load"`: 等待页面完全加载
  - `"domcontentloaded"`: 等待 DOM 内容加载完成
  - `"networkidle"`: 等待网络空闲（500ms 内无网络活动）

### show_browser (可选)
- **类型**: boolean
- **默认值**: `false`
- **描述**: 
  - `true`: 显示浏览器窗口（有界面模式）
  - `false`: 使用无头模式（headless，后台运行）

## 使用示例

### 示例 1: 基本使用（简短域名）

```json
{
  "url": "zeus.imgo.tv"
}
```

**自动转换为**: `https://zeus.imgo.tv`

### 示例 2: 显示浏览器窗口

```json
{
  "url": "baidu.com",
  "show_browser": true
}
```

**效果**: 
- URL 自动补全为 `https://baidu.com`
- 浏览器窗口可见，可以观察页面加载过程

### 示例 3: 等待网络空闲

```json
{
  "url": "https://example.com/dynamic-page",
  "wait_until": "networkidle"
}
```

**适用场景**: 页面有大量异步加载内容

### 示例 4: 完整参数

```json
{
  "url": "github.com",
  "wait_until": "domcontentloaded",
  "show_browser": true
}
```

## URL 自动补全规则

### 1. 已有协议前缀
输入: `https://example.com`
输出: `https://example.com` (不变)

输入: `http://example.com`
输出: `http://example.com` (不变)

### 2. 无协议前缀
输入: `example.com`
输出: `https://example.com`

输入: `www.example.com`
输出: `https://www.example.com`

输入: `example.com/path?query=value`
输出: `https://example.com/path?query=value`

### 3. 本地文件
输入: `file:///Users/name/page.html`
输出: `file:///Users/name/page.html` (不变)

## 返回值说明

```json
{
  "success": true,
  "url": "https://example.com",
  "title": "Example Domain",
  "snapshot": "...",  // 页面结构快照
  "refs_count": 15,   // 可交互元素数量
  "hint": "Use @e1, @e2 etc. refs from snapshot with browser_click/browser_fill"
}
```

### 字段说明

- **success**: 操作是否成功
- **url**: 实际访问的 URL（可能与输入不同，如重定向）
- **title**: 页面标题
- **snapshot**: ARIA 树结构，包含页面元素和引用标记
- **refs_count**: 可交互元素的数量（带 @e1, @e2 等标记）
- **hint**: 使用提示

## 使用场景

### 1. 调试模式（显示浏览器）

当需要观察页面加载过程或调试交互问题时：

```json
{
  "url": "target-website.com",
  "show_browser": true
}
```

### 2. 生产模式（无头模式）

自动化任务，不需要可视化界面：

```json
{
  "url": "target-website.com",
  "show_browser": false
}
```

或省略参数（默认为 false）：

```json
{
  "url": "target-website.com"
}
```

### 3. 快速测试

测试网站是否可访问：

```json
{
  "url": "example.com"
}
```

## 注意事项

### 1. Headless 模式切换

- 设置 `show_browser` 会关闭当前浏览器并重新初始化
- 建议在开始任务时就确定使用哪种模式
- 频繁切换会影响性能

### 2. URL 格式

- 确保域名格式正确
- 对于 IP 地址，建议显式添加协议：`http://192.168.1.1`
- 对于端口号，需要完整 URL：`http://localhost:8080`

### 3. 等待条件选择

| 场景 | 推荐 wait_until |
|------|----------------|
| 静态页面 | `load` |
| SPA 应用 | `networkidle` |
| 快速响应 | `domcontentloaded` |

### 4. 性能考虑

- Headless 模式消耗更少资源
- `networkidle` 等待时间最长，但确保内容完全加载
- 对于简单页面，`domcontentloaded` 足够

## 错误处理

### 常见错误

#### 1. "Cannot navigate to invalid URL"

**原因**: URL 格式错误

**解决方案**: 
- 检查域名拼写
- 确保 URL 不包含非法字符
- 对于特殊字符，使用 URL 编码

#### 2. "Browser not launched"

**原因**: 浏览器未启动

**解决方案**: 
- 确保 Playwright 浏览器已安装
- 运行: `npx playwright install chromium`

#### 3. "Navigation timeout"

**原因**: 页面加载超时（默认 10 秒）

**解决方案**: 
- 检查网络连接
- 尝试使用 `domcontentloaded` 而不是 `load`
- 确认目标网站可访问

## 与其他工具配合使用

### 1. 打开页面后点击元素

```javascript
// 1. 打开页面
browser_open({
  url: "example.com",
  show_browser: true
})

// 2. 从返回的 snapshot 中找到元素引用（如 @e5）

// 3. 点击元素
browser_click({
  target: "@e5"
})
```

### 2. 打开页面后填写表单

```javascript
// 1. 打开页面
browser_open({
  url: "example.com/login"
})

// 2. 填写用户名
browser_fill({
  target: "@e1",  // 用户名输入框
  value: "username"
})

// 3. 填写密码
browser_fill({
  target: "@e2",  // 密码输入框
  value: "password"
})

// 4. 点击登录
browser_click({
  target: "@e3"  // 登录按钮
})
```

## 最佳实践

1. **开发时使用 show_browser=true**
   - 便于观察和调试
   - 可以看到实际的页面交互

2. **生产时使用 show_browser=false**
   - 提高性能
   - 减少资源消耗

3. **根据页面特性选择 wait_until**
   - 静态页面: `load`
   - 动态页面: `networkidle`
   - 快速预览: `domcontentloaded`

4. **URL 简化**
   - 可以省略 `https://` 前缀
   - 让 AI 更容易理解和使用

5. **错误重试**
   - 网络问题时可以重试
   - 考虑使用更宽松的 wait_until

---

**更新日期**: 2026-01-15
**版本**: v1.1
