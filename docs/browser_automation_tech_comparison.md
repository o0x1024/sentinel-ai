# 浏览器自动化技术选型对比

## 📋 概述

为实现AI驱动的自动化安全测试，需要选择合适的浏览器自动化方案。本文档对比了适用于Rust生态的几种方案。

---

## 🔍 候选方案对比

### 方案1: Fantoccini (WebDriver协议)

**简介**: Rust原生的WebDriver客户端，兼容 Selenium 协议

**依赖**: 
```toml
[dependencies]
fantoccini = "0.19"
tokio = { version = "1.40", features = ["full"] }
```

#### 优点
- ✅ 纯Rust实现，无需外部依赖
- ✅ 异步支持（基于tokio）
- ✅ 兼容所有主流浏览器（Chrome/Firefox/Edge）
- ✅ 成熟稳定，社区活跃
- ✅ 支持自定义WebDriver配置（包括代理）
- ✅ 类型安全，编译时错误检查

#### 缺点
- ⚠️ 需要独立运行 ChromeDriver/GeckoDriver 进程
- ⚠️ API相对底层，需要封装常用操作
- ⚠️ 文档相对Playwright/Puppeteer少

#### 代码示例
```rust
use fantoccini::{ClientBuilder, Locator};

async fn test_website(proxy_port: u16) -> Result<()> {
    // 配置 Chrome 使用代理
    let mut caps = serde_json::json!({
        "goog:chromeOptions": {
            "args": [
                format!("--proxy-server=http://127.0.0.1:{}", proxy_port),
                "--disable-web-security",
            ]
        }
    });
    
    // 连接到 WebDriver
    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:9515")
        .await?;
    
    // 导航到目标URL
    client.goto("https://zeus.imgo.tv/").await?;
    
    // 查找元素并交互
    let search = client.find(Locator::Css("input[name='search']")).await?;
    search.send_keys("test' OR 1=1--").await?;
    
    // 截图
    let screenshot = client.screenshot().await?;
    tokio::fs::write("screenshot.png", &screenshot).await?;
    
    client.close().await?;
    Ok(())
}
```

#### 部署要求
1. 安装 ChromeDriver
   ```bash
   # macOS
   brew install chromedriver
   
   # 或下载: https://chromedriver.chromium.org/
   ```

2. 启动 WebDriver 服务
   ```bash
   chromedriver --port=9515
   ```

3. Rust代码连接到 WebDriver

**评分**: ⭐⭐⭐⭐⭐ (推荐)

---

### 方案2: Headless Chrome (Chrome DevTools Protocol)

**简介**: 直接使用Chrome DevTools Protocol控制浏览器

**依赖**: 
```toml
[dependencies]
headless_chrome = "1.0"
```

#### 优点
- ✅ 无需外部 WebDriver 进程
- ✅ 直接控制 Chrome，性能更好
- ✅ 支持拦截网络请求（可用于分析）
- ✅ 集成度高，易于部署

#### 缺点
- ❌ 仅支持 Chrome/Chromium
- ❌ 维护不活跃（最后更新2年前）
- ❌ API不如 Fantoccini 完善
- ⚠️ 需要安装 Chrome/Chromium

#### 代码示例
```rust
use headless_chrome::{Browser, LaunchOptions};
use headless_chrome::protocol::cdp::Network;

async fn test_website(proxy_port: u16) -> Result<()> {
    let options = LaunchOptions {
        headless: true,
        args: vec![
            format!("--proxy-server=127.0.0.1:{}", proxy_port),
        ],
        ..Default::default()
    };
    
    let browser = Browser::new(options)?;
    let tab = browser.new_tab()?;
    
    tab.navigate_to("https://zeus.imgo.tv/")?;
    tab.wait_for_element("input[name='search']")?
        .type_into("test' OR 1=1--")?;
    
    let screenshot = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None, None, true
    )?;
    
    Ok(())
}
```

**评分**: ⭐⭐⭐ (备选方案)

---

### 方案3: Playwright via Node.js

**简介**: 通过子进程调用 Node.js Playwright

**依赖**: 
```toml
[dependencies]
tokio = { version = "1.40", features = ["process"] }
serde_json = "1.0"
```

#### 优点
- ✅ Playwright 功能最强大
- ✅ 支持多浏览器（Chrome/Firefox/Safari）
- ✅ 录制回放、自动等待等高级特性
- ✅ 文档完善，社区活跃

#### 缺点
- ❌ 需要 Node.js 运行时
- ❌ 进程间通信开销
- ❌ 部署复杂度高
- ❌ 类型不安全（JSON通信）

#### 实现方式
1. 创建 Node.js Wrapper 脚本
   ```javascript
   // playwright-wrapper.js
   const { chromium } = require('playwright');
   const readline = require('readline');
   
   const rl = readline.createInterface({
     input: process.stdin,
     output: process.stdout
   });
   
   let browser, page;
   
   rl.on('line', async (line) => {
     const cmd = JSON.parse(line);
     
     switch(cmd.action) {
       case 'launch':
         browser = await chromium.launch({
           proxy: { server: `http://127.0.0.1:${cmd.proxyPort}` }
         });
         page = await browser.newPage();
         console.log(JSON.stringify({ success: true }));
         break;
         
       case 'navigate':
         await page.goto(cmd.url);
         console.log(JSON.stringify({ success: true }));
         break;
         
       // ... 其他命令
     }
   });
   ```

2. Rust代码调用
   ```rust
   use tokio::process::Command;
   use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
   
   struct PlaywrightBrowser {
       process: Child,
       stdin: ChildStdin,
       stdout: BufReader<ChildStdout>,
   }
   
   impl PlaywrightBrowser {
       async fn new(proxy_port: u16) -> Result<Self> {
           let mut process = Command::new("node")
               .arg("playwright-wrapper.js")
               .stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .spawn()?;
           
           let stdin = process.stdin.take().unwrap();
           let stdout = BufReader::new(process.stdout.take().unwrap());
           
           let mut browser = Self { process, stdin, stdout };
           
           browser.send_command(json!({
               "action": "launch",
               "proxyPort": proxy_port
           })).await?;
           
           Ok(browser)
       }
       
       async fn send_command(&mut self, cmd: Value) -> Result<Value> {
           let cmd_str = serde_json::to_string(&cmd)?;
           self.stdin.write_all(cmd_str.as_bytes()).await?;
           self.stdin.write_all(b"\n").await?;
           
           let mut line = String::new();
           self.stdout.read_line(&mut line).await?;
           Ok(serde_json::from_str(&line)?)
       }
   }
   ```

**评分**: ⭐⭐ (不推荐，复杂度高)

---

### 方案4: Thirtyfour (Selenium WebDriver)

**简介**: 另一个 Rust WebDriver 客户端

**依赖**: 
```toml
[dependencies]
thirtyfour = "0.31"
```

#### 优点
- ✅ API设计友好
- ✅ 完整的WebDriver支持
- ✅ 维护活跃

#### 缺点
- ⚠️ 与 Fantoccini 功能类似
- ⚠️ 相对 Fantoccini 社区更小

#### 代码示例
```rust
use thirtyfour::prelude::*;

async fn test_website(proxy_port: u16) -> WebDriverResult<()> {
    let mut caps = DesiredCapabilities::chrome();
    caps.add_chrome_arg(&format!("--proxy-server=http://127.0.0.1:{}", proxy_port))?;
    
    let driver = WebDriver::new("http://localhost:9515", caps).await?;
    
    driver.goto("https://zeus.imgo.tv/").await?;
    
    let search = driver.find(By::Name("search")).await?;
    search.send_keys("test' OR 1=1--").await?;
    
    driver.quit().await?;
    Ok(())
}
```

**评分**: ⭐⭐⭐⭐ (备选)

---

## 🎯 推荐方案

### 首选: Fantoccini

**理由**:
1. ✅ 纯Rust，与项目技术栈一致
2. ✅ 成熟稳定，社区活跃
3. ✅ 兼容所有主流浏览器
4. ✅ 异步支持，性能良好
5. ✅ 易于与现有代码集成

**实施步骤**:

#### Step 1: 添加依赖
```toml
# src-tauri/Cargo.toml
[dependencies]
fantoccini = "0.19"
```

#### Step 2: 创建浏览器自动化模块
```rust
// src-tauri/sentinel-tools/src/browser_automation/
mod.rs              # 模块定义
browser_driver.rs   # WebDriver 管理
browser_tool.rs     # MCP 工具实现
actions.rs          # 高级操作封装
```

#### Step 3: 实现 MCP 工具接口
```rust
pub struct BrowserAutomationTool {
    driver_url: String,
    sessions: Arc<Mutex<HashMap<String, Client>>>,
}

#[async_trait::async_trait]
impl UnifiedTool for BrowserAutomationTool {
    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let action = params.inputs.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing action parameter"))?;
        
        match action {
            "launch" => self.launch(params).await,
            "navigate" => self.navigate(params).await,
            "click" => self.click(params).await,
            "fill" => self.fill(params).await,
            "screenshot" => self.screenshot(params).await,
            "close" => self.close(params).await,
            _ => Err(anyhow!("Unknown action: {}", action))
        }
    }
}
```

#### Step 4: 自动化 WebDriver 部署

**选项A: 嵌入式 ChromeDriver (推荐)**
```rust
// 使用 include_bytes! 嵌入 ChromeDriver 二进制
const CHROMEDRIVER: &[u8] = include_bytes!("../binaries/chromedriver-macos");

fn ensure_chromedriver() -> Result<PathBuf> {
    let path = PathBuf::from("/tmp/sentinel-chromedriver");
    if !path.exists() {
        std::fs::write(&path, CHROMEDRIVER)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(path)
}
```

**选项B: 运行时下载 (备选)**
```rust
async fn download_chromedriver() -> Result<PathBuf> {
    let version = get_chrome_version()?;
    let url = format!(
        "https://chromedriver.storage.googleapis.com/{}/chromedriver_mac64.zip",
        version
    );
    
    // 下载并解压...
}
```

#### Step 5: 集成到工具系统
```rust
// src-tauri/src/tools/mod.rs
pub async fn register_browser_tools(tool_manager: Arc<UnifiedToolManager>) -> Result<()> {
    let browser_tool = Arc::new(BrowserAutomationTool::new().await?);
    tool_manager.register_tool("browser", browser_tool).await?;
    Ok(())
}
```

---

## 🧪 测试计划

### 单元测试
```rust
#[tokio::test]
async fn test_browser_launch() {
    let tool = BrowserAutomationTool::new().await.unwrap();
    let result = tool.execute(ToolExecutionParams {
        inputs: hashmap! {
            "action" => "launch",
            "proxy_port" => 4201,
            "headless" => true,
        },
        ..Default::default()
    }).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_browser_navigate() {
    // ...
}
```

### 集成测试
```rust
#[tokio::test]
async fn test_full_automation_workflow() {
    // 1. 启动被动扫描
    let passive_scan = start_passive_scan(ProxyConfig::default()).await?;
    
    // 2. 启动浏览器
    let browser = BrowserAutomationTool::new().await?;
    let session_id = browser.launch(passive_scan.port).await?;
    
    // 3. 访问网站
    browser.navigate(session_id, "https://example.com").await?;
    
    // 4. 等待流量
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 5. 检查漏洞
    let findings = passive_scan.get_findings().await?;
    assert!(!findings.is_empty());
    
    // 6. 清理
    browser.close(session_id).await?;
    passive_scan.stop().await?;
}
```

---

## 📦 部署方案

### macOS
```bash
# 安装 ChromeDriver
brew install chromedriver

# 信任 ChromeDriver（绕过 Gatekeeper）
xattr -d com.apple.quarantine /opt/homebrew/bin/chromedriver
```

### Linux
```bash
# 下载 ChromeDriver
wget https://chromedriver.storage.googleapis.com/LATEST_RELEASE
VERSION=$(cat LATEST_RELEASE)
wget https://chromedriver.storage.googleapis.com/$VERSION/chromedriver_linux64.zip
unzip chromedriver_linux64.zip
sudo mv chromedriver /usr/local/bin/
sudo chmod +x /usr/local/bin/chromedriver
```

### Windows
```powershell
# 使用 Chocolatey
choco install chromedriver
```

### Docker (开发环境)
```dockerfile
FROM rust:1.75

# 安装 Chrome 和 ChromeDriver
RUN wget -q -O - https://dl-ssl.google.com/linux/linux_signing_key.pub | apt-key add -
RUN echo "deb http://dl.google.com/linux/chrome/deb/ stable main" >> /etc/apt/sources.list.d/google.list
RUN apt-get update && apt-get install -y google-chrome-stable chromium-driver

CMD ["cargo", "test"]
```

---

## 🔧 高级特性

### 1. 智能等待
```rust
impl BrowserAutomationTool {
    async fn wait_for_element(&self, session_id: &str, selector: &str, timeout: Duration) -> Result<()> {
        let client = self.get_session(session_id)?;
        client.wait()
            .for_element(Locator::Css(selector))
            .timeout(timeout)
            .await?;
        Ok(())
    }
}
```

### 2. 自动重试
```rust
async fn navigate_with_retry(&self, session_id: &str, url: &str) -> Result<()> {
    for attempt in 1..=3 {
        match self.navigate(session_id, url).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < 3 => {
                tracing::warn!("Navigate failed (attempt {}): {}", attempt, e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 3. 网络拦截
```rust
// 可通过被动扫描代理实现，无需额外开发
// 浏览器流量 → 代理 → 插件检测
```

---

## 📊 性能基准

| 操作 | 耗时 | 说明 |
|-----|------|------|
| 启动浏览器 | ~1.5s | 首次启动较慢 |
| 页面导航 | ~500ms | 取决于网络 |
| 元素查找 | ~50ms | 单个元素 |
| 点击操作 | ~100ms | 包括动画等待 |
| 截图 | ~200ms | 全屏截图 |

---

## 🎓 学习资源

- [Fantoccini文档](https://docs.rs/fantoccini/)
- [WebDriver协议](https://w3c.github.io/webdriver/)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [Selenium最佳实践](https://www.selenium.dev/documentation/test_practices/)

---

**推荐**: 使用 **Fantoccini**，理由充分，实施成本低，风险可控。

