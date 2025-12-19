use anyhow::Result;
use async_trait::async_trait;
use sentinel_ai_lib::engines::vision_explorer_v2::core::{Agent, Event, PageContext};
use sentinel_ai_lib::engines::vision_explorer_v2::driver::BrowserActions;
use sentinel_ai_lib::engines::vision_explorer_v2::driver::NavigatorAgent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MockBrowserDriver {
    pub goto_called: Arc<AtomicBool>,
}

#[async_trait]
impl BrowserActions for MockBrowserDriver {
    fn goto(&self, _url: &str) -> Result<()> {
        self.goto_called.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn type_text(&self, _selector: &str, _text: &str) -> Result<()> {
        Ok(())
    }
    fn click(&self, _selector: &str) -> Result<()> {
        Ok(())
    }
    fn capture_context(&self) -> Result<PageContext> {
        Ok(PageContext {
            url: "http://mock.com".to_string(),
            title: "Mock Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body>Mock</body></html>".to_string(),
            accessibility_tree: None,
        })
    }
}

#[tokio::test]
async fn test_navigator_agent_execution() {
    let goto_called = Arc::new(AtomicBool::new(false));
    let mock_driver = MockBrowserDriver {
        goto_called: goto_called.clone(),
    };

    // Create Box<dyn BrowserActions> wrapped in Arc<Mutex>
    let driver: Arc<Mutex<Box<dyn BrowserActions>>> = Arc::new(Mutex::new(Box::new(mock_driver)));

    let (tx, _rx) = tokio::sync::mpsc::channel(10);

    let agent = NavigatorAgent::new("nav_test".to_string(), driver, tx);

    let event = Event::TaskAssigned {
        agent_id: "nav_test".to_string(),
        task_id: "task_1".to_string(),
        target_node_id: "http://example.com".to_string(),
        payload: None,
    };

    // Handle the event
    let result = agent.handle_event(&event).await;
    assert!(result.is_ok());

    // Verify goto was called
    assert!(goto_called.load(Ordering::SeqCst));
}
