use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanProfileConfig {
    pub wpm: Range<u32>,
    pub mouse_speed_factor: Range<f64>,
    pub typo_rate: f64,
    pub think_pause_ms: Range<u64>,
    pub micro_delay_ms: Range<u64>,
    pub overshoot_probability: f64,
    pub fatigue_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HumanProfile {
    Casual,
    Expert,
    Cautious,
    Custom(HumanProfileConfig),
}

impl HumanProfile {
    pub fn config(&self) -> HumanProfileConfig {
        match self {
            HumanProfile::Casual => HumanProfileConfig {
                wpm: 40..60,
                mouse_speed_factor: 0.8..1.2,
                typo_rate: 0.02,
                think_pause_ms: 500..2000,
                micro_delay_ms: 50..150,
                overshoot_probability: 0.10,
                fatigue_enabled: true,
            },
            HumanProfile::Expert => HumanProfileConfig {
                wpm: 80..120,
                mouse_speed_factor: 1.2..1.8,
                typo_rate: 0.005,
                think_pause_ms: 200..800,
                micro_delay_ms: 20..80,
                overshoot_probability: 0.05,
                fatigue_enabled: false,
            },
            HumanProfile::Cautious => HumanProfileConfig {
                wpm: 25..40,
                mouse_speed_factor: 0.5..0.8,
                typo_rate: 0.0,
                think_pause_ms: 1000..5000,
                micro_delay_ms: 100..300,
                overshoot_probability: 0.15,
                fatigue_enabled: true,
            },
            HumanProfile::Custom(config) => config.clone(),
        }
    }
}

impl Default for HumanProfile {
    fn default() -> Self {
        HumanProfile::Casual
    }
}

/// Humanization level controls how much effort is spent on appearing human
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanizationLevel {
    /// No humanization — raw protocol commands, fastest execution
    Raw,
    /// Basic random delays between operations
    Basic,
    /// Full human simulation: Bezier mouse, typing rhythm, scroll physics
    Human,
    /// Human + fingerprint management + auto-recovery (for adversarial targets)
    Stealth,
}

impl Default for HumanizationLevel {
    fn default() -> Self {
        HumanizationLevel::Human
    }
}
