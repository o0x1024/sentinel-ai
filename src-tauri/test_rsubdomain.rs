// 测试rsubdomain API的简单程序
use rsubdomain::{SubdomainBruteEngine, SubdomainBruteConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing rsubdomain API...");
    
    // 创建默认配置
    let mut config = SubdomainBruteConfig::default();
    println!("Created config: {:?}", config);
    
    // 设置域名
    config.domains = vec!["example.com".to_string()];
    
    // 创建引擎
    let engine = SubdomainBruteEngine::new(config);
    println!("Created engine successfully");
    
    // 尝试运行扫描
    match engine.await {
        Ok(results) => {
            println!("Scan completed, results type: {:?}", std::any::type_name_of_val(&results));
            println!("Scan completed successfully");
        }
        Err(e) => {
            println!("Scan failed: {}", e);
        }
    }
    
    println!("Test completed");
    Ok(())
}