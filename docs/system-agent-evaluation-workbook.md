# System Agent 逻辑漏洞评测工作簿

这份工作簿用于沉淀每轮最小评测基线的结果，避免继续靠肉眼判断：

- 真阳性
- 假阳性
- 漏报
- 候选待验证

## 1. 建议流程

1. 启动业务服务

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:biz-service
```

2. 运行最小评测基线，并把输出保存到文件

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval > .tmp/system-agent-eval-report.json
```

3. 从 Sentinel 导出或保存当前 findings 快照  
建议至少保存为：

- `id`
- `title`
- `url`
- `status`
- `analysisStage`
- `severity`
- `vulnType`

临时命名建议：

```text
.tmp/system-agent-findings-snapshot.json
```

4. 运行对照脚本

```bash
cd /Users/like/code/sentinel/sentinel-ai
npm run demo:eval:compare -- --eval .tmp/system-agent-eval-report.json --findings .tmp/system-agent-findings-snapshot.json
```

5. 把输出里的 `markdownTable` 复制到本工作簿对应轮次

## 2. 轮次模板

### 轮次信息

- 时间：
- 行为来源模式：
  - `proxy_inferred`
  - `browser_extension`
- scope：
- 当前 triage prompt patch：
- 当前 verifier 配置：

### 对照结果

把 `system-agent-eval-compare` 输出中的 `markdownTable` 粘贴到这里。

| 场景 | 预期 | 判定 | 说明 | 命中结果 |
| --- | --- | --- | --- | --- |
| 示例 | negative_control | TN | 负样本未命中候选或正式漏洞。 | - |

### 人工复核补充

- 是否存在“路径没对上，但其实是同一业务问题”的情况：
- 是否存在“候选很多，但都无法验证”的情况：
- 是否存在“浏览器行为增强比弱行为模式更好/更差”的明显现象：
- 是否需要记录新的真阳性/假阳性/漏报样本：

## 3. 建议判定口径

- `TN`
  - 负样本未命中候选或正式漏洞
- `FP-CANDIDATE`
  - 负样本触发了候选待验证
- `FP`
  - 负样本直接命中了正式漏洞或已验证结果
- `TP-CANDIDATE`
  - 候选场景形成了候选待验证
- `TP-VERIFIED`
  - 候选场景命中了正式漏洞或已验证结果
- `FN`
  - 候选场景没有对应结果

## 4. 下一步扩展建议

工作簿稳定后，再继续加：

- 浏览器行为增强 vs 弱行为模式对照列
- 多轮对比趋势
- 误报来源分类
  - 动作识别失真
  - 状态前置误判
  - ownership 误判
  - verifier 未命中
