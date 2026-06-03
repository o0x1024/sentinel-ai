// 测试流量分析数据库初始化
// 注意：流量分析数据库表现在由 sentinel-db 统一管理
use sentinel_db::DatabaseService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 流量分析数据库初始化测试 ===\n");
    println!("注意：流量分析数据库表现在由 sentinel-db 统一管理");
    println!("请使用主数据库服务进行测试\n");

    // 数据库路径
    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sentinel-ai")
        .join("database.db");

    println!("数据库路径: {}\n", db_path.display());

    // 初始化数据库
    println!("正在初始化数据库...");
    let mut db_service = DatabaseService::new();
    db_service.initialize().await?;
    println!("✓ 数据库初始化成功\n");

    // 验证表结构
    println!("验证流量分析相关表...");
    let pool = db_service.get_pool()?;
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'traffic_%' OR name IN ('plugin_registry', 'proxy_config', 'proxy_requests') ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    println!("流量分析相关表 ({} 个):", tables.len());
    for (name,) in &tables {
        println!("  - {}", name);
    }

    // 检查必需的表
    let required_tables = [
        "traffic_vulnerabilities",
        "traffic_evidence",
        "traffic_dedupe_index",
        "plugin_registry",
        "proxy_requests",
        "proxy_config",
    ];

    println!("\n检查必需的表:");
    let mut all_ok = true;
    for required in &required_tables {
        let exists = tables.iter().any(|(name,)| name == required);
        if exists {
            println!("  ✓ {}", required);
        } else {
            println!("  ✗ {} (缺失)", required);
            all_ok = false;
        }
    }

    println!("\n");
    if all_ok {
        println!("=== ✓ 所有测试通过！ ===");
        Ok(())
    } else {
        println!("=== ✗ 部分测试失败 ===");
        std::process::exit(1);
    }
}
