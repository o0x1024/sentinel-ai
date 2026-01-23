# ASM 快速参考卡片

## 🚀 快速开始

### 1. 发现资产
```typescript
// 前端操作
BugBounty → 资产管理 → 发现资产
- 选择项目
- 选择插件
- 输入参数
- 勾选"自动导入"
- 开始发现
```

### 2. 查看资产
```typescript
// 自动显示的ASM信息
- 攻击面评分（进度条）
- 暴露级别（标签）
- 资产类型
- 最后发现时间
```

### 3. Enrichment
```typescript
// 手动enrichment
await invoke('enrich_asset', {
  request: { asset_id: 'uuid' }
});

// 自动enrichment
await invoke('start_asset_enrichment');
```

---

## 📋 支持的资产类型

| 类型 | 导入 | Enrichment | 评分 |
|-----|------|-----------|------|
| domain | ✅ | ✅ DNS | ✅ |
| ip | ✅ | ✅ Cloud | ✅ |
| port | ✅ | ✅ Service | ✅ |
| url | ✅ | ✅ WAF/CDN | ✅ |
| certificate | ✅ | ✅ Expiry | ✅ |

---

## 🎯 插件输出格式

### 子域名
```json
{
  "data": {
    "subdomains": ["api.example.com"]
  }
}
```

### 端口
```json
{
  "data": {
    "ports": [{
      "ip": "1.2.3.4",
      "port": 443,
      "service": "https"
    }]
  }
}
```

### URL
```json
{
  "data": {
    "urls": [{
      "url": "https://example.com",
      "status_code": 200
    }]
  }
}
```

### IP
```json
{
  "data": {
    "ips": [{
      "ip": "1.2.3.4",
      "asn": 13335,
      "country": "US"
    }]
  }
}
```

### 证书
```json
{
  "data": {
    "certificates": [{
      "hostname": "example.com",
      "subject": "CN=*.example.com",
      "valid_to": "2025-12-31T23:59:59Z"
    }]
  }
}
```

---

## 🔍 常用查询

### 高危端口
```sql
SELECT * FROM bounty_assets
WHERE asset_type = 'port'
  AND attack_surface_score > 70
  AND is_alive = 1;
```

### 云资产
```sql
SELECT cloud_provider, COUNT(*) 
FROM bounty_assets
WHERE is_cloud = 1
GROUP BY cloud_provider;
```

### 暴露在互联网的资产
```sql
SELECT * FROM bounty_assets
WHERE exposure_level = 'internet'
ORDER BY attack_surface_score DESC;
```

### 近期发现的资产
```sql
SELECT * FROM bounty_assets
WHERE datetime(first_seen_at) > datetime('now', '-7 days')
ORDER BY first_seen_at DESC;
```

---

## ⚡ 风险评分参考

### 端口风险
```
极高危 (50分): Telnet (23)
高危 (40-45): FTP (21), SMB (445), RDP (3389)
中危 (20-35): SSH (22), 数据库端口
低危 (10-15): HTTP (80), HTTPS (443)
```

### 暴露级别
```
internet  → 40分 (公网暴露)
intranet  → 20分 (内网)
private   → 5分  (私有)
```

### 关键性
```
critical → 20分
high     → 15分
medium   → 10分
low      → 5分
```

---

## 🛠️ 命令速查

### Tauri 命令
```typescript
// 资产管理
invoke('bounty_list_assets', { filter })
invoke('bounty_create_asset', { asset })
invoke('bounty_update_asset', { asset })
invoke('bounty_delete_asset', { asset_id })

// Enrichment
invoke('enrich_asset', { request })
invoke('start_asset_enrichment')
invoke('stop_asset_enrichment')

// 发现资产
invoke('monitor_discover_and_import_assets', { request })
```

### SQL 快捷查询
```sql
-- 统计资产类型
SELECT asset_type, COUNT(*) FROM bounty_assets GROUP BY asset_type;

-- 按ASN分组
SELECT asn, asn_org, COUNT(*) FROM bounty_assets WHERE asn IS NOT NULL GROUP BY asn;

-- 即将过期的证书
SELECT * FROM bounty_assets 
WHERE asset_type = 'certificate'
  AND datetime(certificate_valid_to) < datetime('now', '+30 days');
```

---

## 🎨 UI 组件

### AssetsPanel 显示字段
- ✅ 名称 (hostname/url)
- ✅ 资产类型
- ✅ 攻击面评分（进度条）
- ✅ 暴露级别（标签）
- ✅ 风险级别（High/Medium/Low）
- ✅ 状态（Active/Inactive）
- ✅ 最后发现时间

---

## 📊 ASM 核心字段

### 必填字段
```rust
id: String
program_id: String
asset_type: String  // domain, ip, port, url, certificate
canonical_url: String
is_alive: bool
```

### 重要可选字段
```rust
// 风险评估
exposure_level: Option<String>
attack_surface_score: Option<f64>
vulnerability_count: Option<i32>
criticality: Option<String>

// 网络信息
hostname: Option<String>
port: Option<i32>
ip_addresses_json: Option<String>

// 云/地理
asn: Option<i32>
country: Option<String>
is_cloud: Option<bool>
cloud_provider: Option<String>

// 服务信息
service_name: Option<String>
service_version: Option<String>
banner: Option<String>

// 发现信息
discovery_method: Option<String>
confidence_score: Option<f64>
data_sources_json: Option<String>
```

---

## 🔐 安全最佳实践

### 1. 关键资产标记
```sql
UPDATE bounty_assets 
SET criticality = 'critical',
    monitoring_enabled = 1,
    scan_frequency = 'hourly'
WHERE canonical_url IN ('payment.example.com', 'auth.example.com');
```

### 2. 自动化监控
```typescript
// 为高危资产启用自动监控
const highRiskAssets = assets.filter(a => a.attack_surface_score > 70);
for (const asset of highRiskAssets) {
  await invoke('create_monitor_task', {
    asset_id: asset.id,
    interval: '1hour'
  });
}
```

### 3. 优先级队列
```
1. internet + critical + vulnerability_count > 0
2. internet + high + attack_surface_score > 70
3. internet + medium
4. intranet + critical
5. 其他
```

---

## 📈 性能提示

### 1. 使用索引
```sql
-- 好：使用索引
WHERE asset_type = 'port' AND is_alive = 1

-- 差：没有索引
WHERE LOWER(canonical_url) LIKE '%example%'
```

### 2. 批量操作
```rust
// 批量enrichment而非逐个
for chunk in asset_ids.chunks(50) {
    let tasks = chunk.iter().map(|id| enrich_asset(id));
    join_all(tasks).await;
}
```

### 3. 分页查询
```typescript
// 使用 limit + offset
invoke('bounty_list_assets', {
  filter: {
    limit: 100,
    offset: page * 100
  }
});
```

---

## 🐛 故障排查

### 资产没有导入？
1. 检查插件输出格式
2. 查看日志：`logs/sentinel-ai.log`
3. 确认 `auto_import: true`

### ASM字段为空？
1. 运行 enrichment: `invoke('enrich_asset')`
2. 启动自动服务: `invoke('start_asset_enrichment')`
3. 手动填充关键字段

### 评分为0？
1. 设置 `criticality` 和 `exposure_level`
2. 添加 `vulnerability_count`
3. 对端口资产确保有 `port` 值

---

## 📚 延伸阅读

- [完整实施指南](./ASM-IMPLEMENTATION-GUIDE.md)
- [使用示例](./ASM-USAGE-EXAMPLES.md)
- [完成总结](../ASM-COMPLETE-SUMMARY.md)
- [实施状态](../ASM-IMPLEMENTATION-STATUS.md)

---

**版本**: v2.0.0  
**最后更新**: 2026-01-23
