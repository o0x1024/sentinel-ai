use crate::adapter::traits::{BrowserBackend, BrowserError, Result, TabId};
use crate::humanize::keyboard::KeyboardHumanizer;
use crate::humanize::mouse::MouseHumanizer;
use crate::humanize::profile::{HumanProfile, HumanizationLevel};
use crate::humanize::scroll::ScrollHumanizer;
use crate::humanize::timing::TimingController;
use crate::page::element::{ElementSelector, ResolvedElement};

use std::sync::Arc;
use tokio::sync::RwLock;

/// High-level browser actions with built-in humanization
pub struct BrowserActions {
    backend: Arc<RwLock<dyn BrowserBackend>>,
    mouse: MouseHumanizer,
    keyboard: KeyboardHumanizer,
    scroll: ScrollHumanizer,
    timing: TimingController,
    level: HumanizationLevel,
}

impl BrowserActions {
    pub fn new(
        backend: Arc<RwLock<dyn BrowserBackend>>,
        profile: HumanProfile,
        level: HumanizationLevel,
    ) -> Self {
        let config = profile.config();
        Self {
            backend,
            mouse: MouseHumanizer::new(config.clone()),
            keyboard: KeyboardHumanizer::new(config.clone()),
            scroll: ScrollHumanizer::new(config.clone()),
            timing: TimingController::new(config),
            level,
        }
    }

    /// Navigate to URL and wait for page load
    pub async fn navigate(&mut self, tab: &TabId, url: &str) -> Result<()> {
        let backend = self.backend.read().await;
        backend.navigate(tab, url).await?;

        if self.level != HumanizationLevel::Raw {
            let delay = self.timing.comprehension_delay(1.0);
            tokio::time::sleep(delay).await;
        }

        Ok(())
    }

    /// Click on an element with human-like mouse movement
    pub async fn click(&mut self, tab: &TabId, element: &ResolvedElement) -> Result<()> {
        use crate::adapter::traits::{MouseButton, MouseEvent, MouseEventKind};

        if !element.is_visible || !element.is_interactable {
            return Err(BrowserError::ElementNotFound(
                "Element is not visible or interactable".to_string(),
            ));
        }

        let target = element.random_click_point();
        let backend = self.backend.read().await;

        match self.level {
            HumanizationLevel::Raw => {
                backend.dispatch_mouse_event(tab, MouseEvent {
                    kind: MouseEventKind::Pressed,
                    x: target.x,
                    y: target.y,
                    button: MouseButton::Left,
                }).await?;
                backend.dispatch_mouse_event(tab, MouseEvent {
                    kind: MouseEventKind::Released,
                    x: target.x,
                    y: target.y,
                    button: MouseButton::Left,
                }).await?;
            }
            _ => {
                // TODO: get current mouse position from state
                let from = crate::adapter::traits::Point { x: 0.0, y: 0.0 };
                let steps = self.mouse.generate_path(from, target.clone(), element.target_size());

                for step in &steps {
                    backend.dispatch_mouse_event(tab, MouseEvent {
                        kind: MouseEventKind::Moved,
                        x: step.x,
                        y: step.y,
                        button: MouseButton::Left,
                    }).await?;

                    if step.delay_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(step.delay_ms)).await;
                    }
                }

                // Click at final position
                let delay = self.timing.micro_delay();
                tokio::time::sleep(delay).await;

                backend.dispatch_mouse_event(tab, MouseEvent {
                    kind: MouseEventKind::Pressed,
                    x: target.x,
                    y: target.y,
                    button: MouseButton::Left,
                }).await?;

                tokio::time::sleep(tokio::time::Duration::from_millis(
                    rand::Rng::gen_range(&mut rand::thread_rng(), 50..120),
                )).await;

                backend.dispatch_mouse_event(tab, MouseEvent {
                    kind: MouseEventKind::Released,
                    x: target.x,
                    y: target.y,
                    button: MouseButton::Left,
                }).await?;
            }
        }

        // Post-click think delay
        if self.level != HumanizationLevel::Raw {
            let delay = self.timing.think_delay();
            tokio::time::sleep(delay).await;
        }

        Ok(())
    }

    /// Type text with human-like keystroke timing
    pub async fn type_text(&mut self, tab: &TabId, text: &str) -> Result<()> {
        use crate::adapter::traits::{KeyEvent, KeyEventKind, KeyModifiers};

        let backend = self.backend.read().await;

        match self.level {
            HumanizationLevel::Raw => {
                for ch in text.chars() {
                    backend.dispatch_key_event(tab, KeyEvent {
                        kind: KeyEventKind::Char,
                        key: ch.to_string(),
                        code: String::new(),
                        modifiers: KeyModifiers::default(),
                    }).await?;
                }
            }
            _ => {
                let keystrokes = self.keyboard.generate_keystrokes(text);

                for ks in &keystrokes {
                    // Handle typo corrections first
                    for correction in &ks.corrections {
                        // Type wrong key
                        backend.dispatch_key_event(tab, KeyEvent {
                            kind: KeyEventKind::Down,
                            key: correction.wrong_key.clone(),
                            code: String::new(),
                            modifiers: KeyModifiers::default(),
                        }).await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(correction.hold_ms)).await;
                        backend.dispatch_key_event(tab, KeyEvent {
                            kind: KeyEventKind::Up,
                            key: correction.wrong_key.clone(),
                            code: String::new(),
                            modifiers: KeyModifiers::default(),
                        }).await?;

                        // Pause, then backspace
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            correction.pause_before_backspace_ms,
                        )).await;
                        backend.dispatch_key_event(tab, KeyEvent {
                            kind: KeyEventKind::Down,
                            key: "Backspace".to_string(),
                            code: "Backspace".to_string(),
                            modifiers: KeyModifiers::default(),
                        }).await?;
                        backend.dispatch_key_event(tab, KeyEvent {
                            kind: KeyEventKind::Up,
                            key: "Backspace".to_string(),
                            code: "Backspace".to_string(),
                            modifiers: KeyModifiers::default(),
                        }).await?;
                    }

                    // Pre-keystroke delay
                    if ks.delay_before_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(ks.delay_before_ms)).await;
                    }

                    // Type correct key
                    backend.dispatch_key_event(tab, KeyEvent {
                        kind: KeyEventKind::Down,
                        key: ks.key.clone(),
                        code: ks.code.clone(),
                        modifiers: KeyModifiers::default(),
                    }).await?;
                    tokio::time::sleep(tokio::time::Duration::from_millis(ks.hold_duration_ms)).await;
                    backend.dispatch_key_event(tab, KeyEvent {
                        kind: KeyEventKind::Up,
                        key: ks.key.clone(),
                        code: ks.code.clone(),
                        modifiers: KeyModifiers::default(),
                    }).await?;
                }
            }
        }

        Ok(())
    }

    /// Scroll the page with human-like physics
    pub async fn scroll(&mut self, tab: &TabId, delta_y: f64) -> Result<()> {
        let backend = self.backend.read().await;

        match self.level {
            HumanizationLevel::Raw => {
                backend.dispatch_scroll_event(tab, crate::adapter::traits::ScrollEvent {
                    x: 0.0,
                    y: 0.0,
                    delta_x: 0.0,
                    delta_y,
                }).await?;
            }
            _ => {
                let steps = self.scroll.generate_scroll(delta_y);
                for step in &steps {
                    if step.delta_y.abs() > 0.1 || step.delta_x.abs() > 0.1 {
                        backend.dispatch_scroll_event(tab, crate::adapter::traits::ScrollEvent {
                            x: 0.0,
                            y: 0.0,
                            delta_x: step.delta_x,
                            delta_y: step.delta_y,
                        }).await?;
                    }
                    if step.delay_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(step.delay_ms)).await;
                    }
                }
            }
        }

        Ok(())
    }
}
