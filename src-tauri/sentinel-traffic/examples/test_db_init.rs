// 测试流量分析数据库初始化
use sentinel_traffic::TrafficDatabaseService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 流量分析数据库初始化测试 ===\n");

    // 数据库路径
    let db_path = dirs::home_dir()
        .unwrap()
        .join(".sentinel-ai")
        .join("traffic_scan_test.db");

    // 删除旧数据库
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("✓ 删除旧数据库");
    }

    let database_url = format!("sqlite://{}", db_path.display());
    println!("数据库路径: {}\n", db_path.display());

    // 初始化数据库
    println!("正在初始化数据库...");
    let db_service = TrafficDatabaseService::new(&database_url).await?;
    println!("✓ 数据库初始化成功\n");

    // 验证表结构
    println!("验证表结构...");
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    .fetch_all(db_service.pool())
    .await?;

    println!("已创建的表 ({} 个):", tables.len());
    for (name,) in &tables {
        println!("  - {}", name);
    }

    // 检查必需的表
    let required_tables = [
        "traffic_vulnerabilities",
        "traffic_evidence",
        "plugin_registry",
        "traffic_scan_sessions",
        "traffic_dedupe_index",
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
