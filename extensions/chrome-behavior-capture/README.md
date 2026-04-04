# Sentinel Chrome 行为采集扩展

这是给当前 `System Agent` 逻辑漏洞检测链路配套的 Chrome 扩展。

它会采集浏览器中的弱交互事件，例如：

- 页面加载
- 路由切换
- 点击按钮 / 链接
- 表单提交
- 输入项变化

然后通过本地 bridge 发送到 Sentinel：

```text
http://127.0.0.1:18931
```

## 安装方式

1. 打开 Chrome
2. 进入 `chrome://extensions`
3. 打开“开发者模式”
4. 点击“加载已解压的扩展程序”
5. 选择本目录：

```text
sentinel-ai/extensions/chrome-behavior-capture
```

## 使用方式

1. 确保 Sentinel 应用已经启动
2. 在 Sentinel 的代理配置页中，将“行为特征来源”切换为：
   - `浏览器扩展行为采集`
3. 点击扩展图标，确认：
   - 已启用行为采集
   - Bridge 地址为 `http://127.0.0.1:18931`
4. 点击“测试连接”
5. 如果弹窗显示 `Bridge 已连接`，说明扩展已经和本地 Sentinel bridge 连通

## 说明

- 如果 bridge 不可用，扩展不会影响页面功能，只是无法上报增强行为事件
- 如果 Sentinel 侧没有启用浏览器扩展模式，系统仍会自动退回到默认的代理侧弱行为推断
- 当前版本只针对 Chrome Manifest V3
