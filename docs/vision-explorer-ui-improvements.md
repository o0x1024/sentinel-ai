# Vision Explorer UI/UX 改进

## 问题分析

### 问题1：用户无法直观看到LLM操作内容

**现象**：
- Vision Explorer界面只显示 "Iteration 9: click ###"
- 缺少LLM的思考过程和决策理由

**根本原因**：
前端UI (`VisionExplorerPanel.vue`) 只显示了：
- `action_type`（动作类型，如"click"）
- `element_index`（元素索引）
- `reason`（决策理由）

但**缺少了 `thought` 字段**（LLM的完整思考过程）

### 问题2：API发现显示第三方追踪域名

**现象**：
- 探索目标网站 `testphp.vulnweb.com` 
- 但API列表显示大量 `google-analytics.com`、`linkedin.com` 等请求

**根本原因**：
网络拦截捕获了**所有**网络请求，包括：
- Google Analytics
- Google Tag Manager  
- LinkedIn Ads
- Facebook Pixel
- 等第三方追踪代码

这些请求与目标网站功能无关，干扰了真实API的发现。

### 问题3：点击停止执行按钮时Vision Explorer未停止

**现象**：
- 用户点击InputArea的"停止执行"按钮
- Agent执行停止了
- 但Vision Explorer仍在后台运行

**根本原因**：
`AgentView.handleStop()` 只调用了：
- `agentEvents.stopExecution()` - 停止Agent执行
- **但没有调用** `visionEvents.stop()` - 停止Vision Explorer

## 解决方案

### 1. 显示LLM思考过程

#### 前端改进 (`useVisionEvents.ts`)
- 在 `VisionStep` 接口添加 `thought?: string` 字段
- 在 `step` 事件处理中捕获并保存 `thought` 数据

#### UI改进 (`VisionExplorerPanel.vue`)
在每个迭代步骤中，添加独立的"LLM思考"区块：

```vue
<!-- Thought Process (LLM Reasoning) -->
<div v-if="step.thought" class="bg-accent/10 p-2 rounded border border-accent/30 mb-2">
   <div class="text-[10px] text-accent font-semibold mb-1 flex items-center gap-1">
      <i class="fas fa-brain"></i>
      <span>LLM Thinking</span>
   </div>
   <p class="text-[11px] italic leading-relaxed">{{ step.thought }}</p>
</div>
```

**视觉效果**：
- 淡蓝色背景高亮显示
- 脑图标 + "LLM Thinking" 标签
- 斜体文字展示思考内容

### 2. 同源API过滤

#### 智能域名过滤 (`service.rs`)

**策略**：只保留与目标网站**同源**的API请求

**实现**：
1. 获取当前页面URL
2. 提取目标域名（如 `testphp.vulnweb.com`）
3. 过滤网络请求：
   ```rust
   // 只保留同域名或子域名的请求
   if req_domain.eq_ignore_ascii_case(domain) || 
      req_domain.ends_with(&format!(".{}", domain)) {
       // 保留
   }
   ```

**效果**：
- ✅ 保留：`testphp.vulnweb.com/api/users`
- ✅ 保留：`api.testphp.vulnweb.com/v1/data`
- ❌ 过滤：`google-analytics.com/collect`
- ❌ 过滤：`ads.linkedin.com/tracking`

### 3. 停止执行联动

#### 前端改进 (`useVisionEvents.ts`)
添加 `stop()` 方法用于停止Vision Explorer：

```typescript
const stop = () => {
  isVisionActive.value = false
  if (currentExecutionId.value) {
    console.log('[useVisionEvents] Stopping vision explorer:', currentExecutionId.value)
  }
}
```

#### 联动逻辑 (`AgentView.vue`)
在 `handleStop()` 中同时停止Vision Explorer：

```typescript
// Notify useAgentEvents to stop execution status
agentEvents.stopExecution()

// Also stop Vision Explorer if it's running
if (visionEvents.isVisionActive.value) {
  console.log('[AgentView] Stopping Vision Explorer')
  visionEvents.stop()
}
```

**效果**：
- 用户点击停止按钮 → Agent停止 + Vision Explorer停止
- 确保所有后台任务都被终止

## 修改文件清单

### 前端
1. `src/composables/useVisionEvents.ts`
   - 添加 `thought` 字段到 `VisionStep` 接口
   - 更新 `step` 事件处理逻辑
   - 添加 `stop()` 方法用于停止Vision Explorer

2. `src/components/Agent/VisionExplorerPanel.vue`
   - 添加LLM思考过程显示区块

3. `src/components/Agent/AgentView.vue`
   - 在 `handleStop()` 中添加Vision Explorer停止逻辑

4. `src/i18n/locales/agent/en.ts` & `zh.ts`
   - 添加 `llmThinking` 国际化文本

### 后端
1. `src-tauri/sentinel-tools/src/agent_browser/service.rs`
   - 添加 `get_current_url()` 方法
   - 添加 `extract_domain()` 辅助函数
   - 重构 `get_discovered_apis()` 使用同源过滤

## 用户体验改进

### 改进前
```
Iteration 9: ACTION
click
###
```

### 改进后
```
Iteration 9: THINK
🧠 LLM Thinking
"I've been exploring the homepage links. I'll now try the 'Signup' 
link to see if it leads to a registration form, which is a key part 
of exploring application functionality."

Iteration 9: ACTION
⚡ click
Target: Index [19]
"Exploring the signup page to discover registration forms and 
user-related functionality."
```

## 技术要点

### 域名提取算法
```rust
fn extract_domain(url: &str) -> Option<String> {
    // 1. 去除协议（http://、https://）
    // 2. 提取域名部分（到第一个'/'）
    // 3. 移除端口号（如果有）
    // 4. 返回纯域名
}
```

### 同源判断逻辑
- **完全匹配**：`testphp.vulnweb.com == testphp.vulnweb.com`
- **子域名匹配**：`api.testphp.vulnweb.com` 匹配 `testphp.vulnweb.com`
- **不区分大小写**：`TestPhp.VulnWeb.COM` == `testphp.vulnweb.com`

## 测试验证

### 验证点1：LLM思考过程显示
1. 启动Vision Explorer探索任务
2. 检查每个迭代步骤
3. 确认显示LLM的 `thought` 内容

### 验证点2：API过滤效果
1. 探索包含第三方追踪的网站（如 testphp.vulnweb.com）
2. 查看发现的API列表
3. 确认只包含目标域名的请求
4. 确认Google Analytics等第三方请求被过滤

## 总结

通过这三个改进：
1. **可视化增强**：用户能清晰看到LLM的决策过程
2. **信息过滤**：只显示与目标网站相关的API，提高信息质量
3. **执行控制**：停止按钮能同时停止Agent和Vision Explorer，确保完全停止

这些改进显著提升了Vision Explorer的用户体验和实用性。
