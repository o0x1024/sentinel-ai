# ASM (Attack Surface Management) 使用示例

## 📚 目录

1. [资产发现](#资产发现)
2. [资产查询](#资产查询)
3. [资产Enrichment](#资产enrichment)
4. [攻击面评分](#攻击面评分)
5. [高级查询](#高级查询)

---

## 1. 资产发现

### 1.1 子域名发现

通过前端界面：
```
BugBounty → 资产管理 → 发现资产
- 选择项目："平安"
- 选择插件：plugin__subdomain_enumerator
- 输入域名：pingan.com
- 勾选"自动导入资产"
- 点击"开始发现"
```

自动填充的ASM字段：
- `exposure_level`: "internet"
- `asset_category`: "external"
- `discovery_method`: "active"
- `data_sources_json`: ["plugin__subdomain_enumerator"]
- `confidence_score`: 0.9
- `last_scan_type`: "subdomain_enumeration"

### 1.2 端口扫描

插件输出格式：
```json
{
  "success": true,
  "data": {
    "target": "192.168.1.1",
    "ports": [
      {
        "ip": "192.168.1.1",
        "port": 22,
        "service": "ssh",
        "version": "OpenSSH 8.2",
        "banner": "SSH-2.0-OpenSSH_8.2",
        "protocol": "tcp"
      },
      {
        "ip": "192.168.1.1",
        "port": 80,
        "service": "http",
        "banner": "nginx/1.18.0",
        "protocol": "tcp"
      }
    ]
  }
}
```

自动创建的资产：
- `asset_type`: "port"
- `canonical_url`: "192.168.1.1:22"
- `service_name`: "ssh"
- `service_version`: "OpenSSH 8.2"
- `banner`: "SSH-2.0-OpenSSH_8.2"
- `transport_protocol`: "TCP"
- `attack_surface_score`: 20.0 (SSH端口)
- `labels`: ["monitor-discovered", "port-scan"]

### 1.3 支持的资产类型

| 资产类型 | 描述 | canonical_url示例 | 状态 |
|---------|------|------------------|------|
| `domain` | 域名/子域名 | `api.example.com` | ✅ 完全支持 |
| `port` | IP+端口+服务 | `192.168.1.1:443` | ✅ 完全支持 |
| `ip` | IP地址 | `192.168.1.1` | ⚠️ 结构支持，待实现导入 |
| `url` | 完整URL | `https://api.example.com/v1` | ⚠️ 结构支持，待实现导入 |
| `certificate` | SSL/TLS证书 | `cert:sha256:abc123...` | ⏳ P1待实现 |
| `api_endpoint` | API端点 | `POST /api/v1/users` | ⏳ P1待实现 |

---

## 2. 资产查询

### 2.1 基础查询

```typescript
// 前端调用
const assets = await invoke<BountyAssetRow[]>('bounty_list_assets', {
  filter: {
    program_id: 'xxx-xxx-xxx',
    scope_id: null,
    asset_type: null,
    is_alive: true,
    has_findings: null,
    limit: 100,
    offset: 0
  }
});
```

### 2.2 按资产类型过滤

```typescript
// 只查询域名资产
const domains = await invoke('bounty_list_assets', {
  filter: {
    program_id: 'xxx',
    asset_type: 'domain',
    // ...
  }
});

// 只查询端口资产
const ports = await invoke('bounty_list_assets', {
  filter: {
    program_id: 'xxx',
    asset_type: 'port',
    // ...
  }
});
```

### 2.3 访问ASM字段

```typescript
for (const asset of assets) {
  console.log('Asset:', asset.canonical_url);
  console.log('Attack Surface Score:', asset.attack_surface_score);
  console.log('Exposure:', asset.exposure_level);
  console.log('Country:', asset.country);
  console.log('ASN:', asset.asn);
  console.log('Service:', asset.service_name);
}
```

---

## 3. 资产Enrichment

### 3.1 手动Enrich单个资产

```typescript
// 前端调用
const result = await invoke('enrich_asset', {
  request: {
    asset_id: 'asset-uuid-here'
  }
});

if (result.success) {
  console.log('Asset enriched:', result.message);
}
```

### 3.2 启动自动Enrichment服务

```typescript
// 启动后台enrichment服务
await invoke('start_asset_enrichment');

// 服务会每5分钟自动enrichment待处理的资产

// 停止服务
await invoke('stop_asset_enrichment');
```

### 3.3 Enrichment流程

```
资产创建 → 基础信息
    ↓
Enrichment Pipeline（后台）
    ↓
    ├─ IP资产 → 查询ASN、地理位置、云服务商
    ├─ 域名 → 查询WHOIS、DNS记录
    ├─ 端口 → 服务版本检测、CVE查询
    └─ URL → 技术栈检测、WAF/CDN识别
    ↓
完整的ASM信息
```

---

## 4. 攻击面评分

### 4.1 端口风险评分

算法实现：
```rust
fn calculate_port_risk_score(port: i32, service: Option<&str>) -> f64 {
    let mut score = 0.0;
    
    // 高风险端口
    match port {
        21 => score += 40.0,  // FTP
        23 => score += 50.0,  // Telnet
        445 => score += 45.0, // SMB
        3389 => score += 40.0, // RDP
        // ...
    }
    
    // 服务风险
    if service.contains("telnet") {
        score += 20.0;
    }
    
    score.min(100.0)
}
```

评分范围：
- 0-30: 低风险（绿色）
- 40-69: 中风险（黄色）
- 70-100: 高风险（红色）

### 4.2 综合攻击面评分

```rust
fn calculate_attack_surface_score(asset: &BountyAssetRow) -> f64 {
    let mut score = 0.0;
    
    // 暴露级别 (40%)
    score += match asset.exposure_level {
        "internet" => 40.0,
        "intranet" => 20.0,
        "private" => 5.0,
        _ => 10.0,
    };
    
    // 漏洞数量 (30%)
    score += (asset.vulnerability_count * 5.0).min(30.0);
    
    // 关键性 (20%)
    score += match asset.criticality {
        "critical" => 20.0,
        "high" => 15.0,
        "medium" => 10.0,
        "low" => 5.0,
        _ => 7.0,
    };
    
    // 端口/服务风险 (10%)
    // ...
    
    score.min(100.0)
}
```

---

## 5. 高级查询

### 5.1 SQL直接查询

```sql
-- 查询所有internet暴露的高风险资产
SELECT * FROM bounty_assets
WHERE program_id = 'xxx'
  AND exposure_level = 'internet'
  AND attack_surface_score > 70
ORDER BY attack_surface_score DESC;

-- 按ASN分组统计
SELECT asn, asn_org, COUNT(*) as asset_count
FROM bounty_assets
WHERE program_id = 'xxx'
GROUP BY asn, asn_org
ORDER BY asset_count DESC;

-- 查询特定国家的资产
SELECT * FROM bounty_assets
WHERE program_id = 'xxx'
  AND country = 'US'
ORDER BY attack_surface_score DESC;

-- 查询云服务资产
SELECT * FROM bounty_assets
WHERE program_id = 'xxx'
  AND is_cloud = 1
ORDER BY cloud_provider, canonical_url;

-- 查询高危端口
SELECT * FROM bounty_assets
WHERE asset_type = 'port'
  AND port IN (21, 23, 445, 3389)
  AND is_alive = 1;

-- 父子资产关系查询
SELECT 
  p.canonical_url as parent,
  c.canonical_url as child,
  c.asset_type
FROM bounty_assets c
LEFT JOIN bounty_assets p ON c.parent_asset_id = p.id
WHERE c.program_id = 'xxx';
```

### 5.2 Rust查询示例

```rust
// 获取高风险资产
async fn get_high_risk_assets(
    db: &DatabaseService,
    program_id: &str
) -> Result<Vec<BountyAssetRow>> {
    let assets = db.list_bounty_assets(
        Some(program_id),
        None,
        None,
        Some(true), // is_alive
        None,
        None,
        None
    ).await?;
    
    // 过滤高风险资产
    let high_risk: Vec<_> = assets.into_iter()
        .filter(|a| a.attack_surface_score.unwrap_or(0.0) > 70.0)
        .collect();
    
    Ok(high_risk)
}

// 获取云服务资产
async fn get_cloud_assets(
    db: &DatabaseService,
    program_id: &str
) -> Result<HashMap<String, Vec<BountyAssetRow>>> {
    let assets = db.list_bounty_assets(
        Some(program_id),
        None,
        None,
        None,
        None,
        None,
        None
    ).await?;
    
    // 按云服务商分组
    let mut grouped: HashMap<String, Vec<BountyAssetRow>> = HashMap::new();
    for asset in assets {
        if asset.is_cloud == Some(true) {
            let provider = asset.cloud_provider.clone()
                .unwrap_or_else(|| "unknown".to_string());
            grouped.entry(provider).or_default().push(asset);
        }
    }
    
    Ok(grouped)
}
```

---

## 💡 实战场景

### 场景1: 识别互联网暴露的高危端口

```typescript
// 1. 运行端口扫描插件
await invoke('monitor_discover_and_import_assets', {
  request: {
    program_id: 'xxx',
    plugin_id: 'plugin__port_scan',
    plugin_input: { target: '203.0.113.0/24' },
    auto_import: true
  }
});

// 2. 查询高危端口
const assets = await invoke('bounty_list_assets', {
  filter: { program_id: 'xxx', asset_type: 'port' }
});

const highRisk = assets.filter(a => a.attack_surface_score > 70);
console.log(`Found ${highRisk.length} high-risk ports`);
```

### 场景2: 资产地理分布分析

```sql
-- 统计资产地理分布
SELECT 
  country,
  city,
  COUNT(*) as count,
  AVG(attack_surface_score) as avg_risk
FROM bounty_assets
WHERE program_id = 'xxx'
  AND country IS NOT NULL
GROUP BY country, city
ORDER BY count DESC;
```

### 场景3: 监控关键业务资产

```rust
// 标记关键业务资产
async fn mark_critical_assets(db: &DatabaseService, asset_ids: Vec<String>) -> Result<()> {
    for id in asset_ids {
        let mut asset = db.get_bounty_asset(&id).await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;
        
        asset.criticality = Some("critical".to_string());
        asset.monitoring_enabled = Some(true);
        asset.scan_frequency = Some("hourly".to_string());
        
        db.update_bounty_asset(&asset).await?;
    }
    Ok(())
}
```

---

## 🎯 最佳实践

### 1. 资产分类策略
```
external + internet → 最高优先级监控
external + intranet → 定期扫描
internal + private  → 低频率监控
```

### 2. 关键性等级定义
- `critical`: 支付、认证、核心API
- `high`: 用户数据、管理后台
- `medium`: 公开功能、信息展示
- `low`: 测试环境、静态资源

### 3. Enrichment优先级
1. 高攻击面评分资产优先
2. 新发现资产优先
3. 关键业务资产优先
4. 其他资产按队列处理

### 4. 监控频率建议
```
critical + internet → 每小时
high + internet     → 每6小时
medium + internet   → 每天
low                 → 每周
```

---

## 🔧 故障排查

### 问题1: 资产导入后ASM字段为NULL

**原因**: 只有明确支持的插件输出格式才会填充ASM字段

**解决方案**:
1. 确认插件输出格式符合规范
2. 手动运行enrichment: `invoke('enrich_asset', { asset_id })`
3. 启动自动enrichment服务

### 问题2: 攻击面评分为0

**原因**: 评分算法需要足够的字段数据

**解决方案**:
1. 运行enrichment填充缺失字段
2. 手动设置`criticality`和`exposure_level`
3. 关联漏洞数据更新`vulnerability_count`

### 问题3: 地理位置信息缺失

**原因**: IP enrichment需要外部API

**解决方案**:
1. 配置IP geolocation API密钥
2. 实现`enrich_ip_from_api`函数调用
3. 或手动导入IP数据库（MaxMind GeoLite2）

---

## 📊 数据示例

### 完整的域名资产

```json
{
  "id": "uuid-xxx",
  "program_id": "program-uuid",
  "asset_type": "domain",
  "canonical_url": "api.example.com",
  "hostname": "api.example.com",
  "parent_domain": "example.com",
  "is_wildcard": false,
  "dns_records_json": "{\"A\": [\"203.0.113.1\"], \"AAAA\": [\"2001:db8::1\"]}",
  "nameservers_json": "[\"ns1.example.com\", \"ns2.example.com\"]",
  "mx_records_json": "[\"mx1.example.com\"]",
  "domain_registrar": "GoDaddy",
  "registration_date": "2020-01-01T00:00:00Z",
  "expiration_date": "2025-01-01T00:00:00Z",
  "exposure_level": "internet",
  "attack_surface_score": 45.5,
  "asset_category": "external",
  "criticality": "high",
  "discovery_method": "active",
  "data_sources_json": "[\"plugin__subdomain_enumerator\"]",
  "confidence_score": 0.95,
  "monitoring_enabled": true,
  "scan_frequency": "daily"
}
```

### 完整的端口资产

```json
{
  "id": "uuid-yyy",
  "program_id": "program-uuid",
  "asset_type": "port",
  "canonical_url": "203.0.113.1:443",
  "hostname": "203.0.113.1",
  "port": 443,
  "protocol": "TCP",
  "service_name": "https",
  "service_version": "nginx/1.18.0",
  "service_product": "nginx",
  "banner": "nginx/1.18.0",
  "transport_protocol": "TCP",
  "ssl_enabled": true,
  "certificate_subject": "CN=*.example.com",
  "ip_version": "IPv4",
  "asn": 13335,
  "asn_org": "CLOUDFLARENET",
  "isp": "Cloudflare Inc",
  "country": "US",
  "city": "San Francisco",
  "latitude": 37.7749,
  "longitude": -122.4194,
  "is_cloud": true,
  "cloud_provider": "Cloudflare",
  "exposure_level": "internet",
  "attack_surface_score": 30.0,
  "vulnerability_count": 0,
  "discovery_method": "active",
  "last_scan_type": "port_scan"
}
```

---

## 🚀 性能优化建议

### 1. 批量Enrichment

```rust
// 批量处理而非逐个处理
async fn batch_enrich_assets(
    service: &AssetEnrichmentService,
    asset_ids: Vec<String>
) -> Result<()> {
    let batch_size = 50;
    for chunk in asset_ids.chunks(batch_size) {
        let tasks: Vec<_> = chunk.iter()
            .map(|id| service.enrich_asset(id))
            .collect();
        
        futures::future::join_all(tasks).await;
    }
    Ok(())
}
```

### 2. 缓存Enrichment结果

```rust
// 避免重复查询相同IP的ASN信息
use std::collections::HashMap;

struct EnrichmentCache {
    ip_cache: HashMap<String, IpEnrichment>,
}

impl EnrichmentCache {
    async fn get_or_fetch_ip(&mut self, ip: &str) -> Result<IpEnrichment> {
        if let Some(cached) = self.ip_cache.get(ip) {
            return Ok(cached.clone());
        }
        
        let enriched = fetch_ip_info(ip).await?;
        self.ip_cache.insert(ip.to_string(), enriched.clone());
        Ok(enriched)
    }
}
```

### 3. 使用索引优化查询

```sql
-- 已创建的索引
CREATE INDEX idx_bounty_assets_asset_type ON bounty_assets(asset_type);
CREATE INDEX idx_bounty_assets_exposure_level ON bounty_assets(exposure_level);
CREATE INDEX idx_bounty_assets_attack_surface_score ON bounty_assets(attack_surface_score DESC);
CREATE INDEX idx_bounty_assets_vulnerability_count ON bounty_assets(vulnerability_count DESC);
CREATE INDEX idx_bounty_assets_asn ON bounty_assets(asn);
CREATE INDEX idx_bounty_assets_country ON bounty_assets(country);

-- 利用索引的高效查询
SELECT * FROM bounty_assets 
WHERE asset_type = 'port' 
  AND exposure_level = 'internet'
ORDER BY attack_surface_score DESC
LIMIT 100;
```

---

**文档版本**: v1.0.0  
**最后更新**: 2026-01-23  
**维护者**: Sentinel AI Team
