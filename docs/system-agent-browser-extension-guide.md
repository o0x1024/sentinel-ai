# 浏览器扩展行为采集使用说明

本文说明如何使用 Chrome 扩展，把浏览器行为事件接入当前的 `System Agent` 逻辑漏洞检测链路。

## 1. 适用范围

当前只提供：

- Chrome
- Manifest V3

扩展目录：

- [manifest.json](/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture/manifest.json)
- [background.js](/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture/background.js)
- [content.js](/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture/content.js)
- [popup.html](/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture/popup.html)
- [popup.js](/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture/popup.js)

## 2. Sentinel 侧准备

先启动 Sentinel 应用。

扩展会把行为事件发送到本地 bridge：

```text
http://127.0.0.1:18931
```

该 bridge 由 Sentinel 启动时自动拉起，不需要额外执行命令。

## 3. 安装扩展

1. 打开 Chrome
2. 进入：

```text
chrome://extensions
```

3. 打开“开发者模式”
4. 点击“加载已解压的扩展程序”
5. 选择目录：

```text
/Users/like/code/sentinel/sentinel-ai/extensions/chrome-behavior-capture
```

安装后工具栏会出现：

`Sentinel Behavior Capture`

## 4. 扩展侧配置

点击扩展图标，确认：

- `启用行为采集` 已勾选
- `Bridge 地址` 为：

```text
http://127.0.0.1:18931
```

然后点击：

- `测试连接`

如果弹窗显示：

- `Bridge 已连接`

说明扩展已经能和本地 Sentinel 通信。

## 5. Sentinel 中切换行为来源

进入代理配置页面，在“行为特征来源”中切换到：

- `增强：浏览器扩展行为采集`

这时页面里会显示：

- 扩展已连接 / 扩展未连接
- 本地桥接地址
- 最近连接时间

如果扩展未连接，系统仍会自动回退到默认的：

- `代理侧弱行为推断`

## 6. 扩展会采集哪些事件

当前最小版本会采集：

- 页面加载
- 路由切换
- 点击按钮 / 链接
- 表单提交
- 输入项变化
- 心跳事件

这些事件会以“增强行为上下文”的形式进入：

- `behaviorSession`
- `traffic_logic_triage`

用于提高复杂业务流程的语义理解能力。

## 7. 如何验证是否生效

推荐步骤：

1. 启动 Sentinel
2. 安装并启用 Chrome 扩展
3. 在代理配置中切到“浏览器扩展行为采集”
4. 打开测试业务服务：

```text
http://127.0.0.1:7788/
```

5. 在页面里进行真实点击操作
6. 到 Security Center 查看 `System Agent` finding
7. 在 finding 详情中查看：
   - 过程图
   - 命中 skills
   - triage 上下文

## 8. 常见问题

### 8.1 为什么扩展显示未连接

常见原因：

- Sentinel 应用未启动
- 本地 bridge 端口 `18931` 未监听
- popup 里的 bridge 地址被改错了

### 8.2 为什么切到扩展模式后仍然像默认模式

这是当前设计的正常行为：

- 只有扩展连接成功时，`effectiveMode` 才会是 `browser_extension`
- 否则系统会自动回退到 `proxy_inferred`

### 8.3 为什么没有看到明显效果

扩展行为采集是“增强输入”，不是单独的检测器。

要让效果更明显，建议：

- 让流量经过代理
- 在真实业务页面按顺序点击
- 制造和逻辑漏洞相关的多步操作

## 9. 备注

当前扩展是最小可用版本，重点是把浏览器行为事件接入现有 System Agent 逻辑分析链。

后续如果继续增强，优先级建议是：

1. 增加 popup 中的更多状态信息
2. 增加更细的交互事件筛选
3. 增加 tab / frame / route 关联展示
4. 增加扩展连接状态在 Agent 管理页的可视化
