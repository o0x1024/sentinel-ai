use rand::Rng;
use rand_distr::{LogNormal, Distribution};
use std::time::Duration;

use super::profile::HumanProfileConfig;

pub struct TimingController {
    config: HumanProfileConfig,
    operation_count: u64,
}

impl TimingController {
    pub fn new(config: HumanProfileConfig) -> Self {
        Self {
            config,
            operation_count: 0,
        }
    }

    /// Delay between high-level operations (simulates thinking)
    pub fn think_delay(&mut self) -> Duration {
        let mut rng = rand::thread_rng();
        let base = rng.gen_range(self.config.think_pause_ms.clone()) as f64;

        let fatigue_factor = if self.config.fatigue_enabled {
            self.fatigue_factor()
        } else {
            1.0
        };

        self.operation_count += 1;
        Duration::from_millis((base * fatigue_factor) as u64)
    }

    /// Delay after page load (simulates reading/comprehending the page)
    pub fn comprehension_delay(&self, page_complexity: f64) -> Duration {
        let mut rng = rand::thread_rng();
        let base_ms = 500.0 + page_complexity * 200.0;
        let variance = LogNormal::new(0.0, 0.3).unwrap().sample(&mut rng);
        Duration::from_millis((base_ms * variance) as u64)
    }

    /// Short delay between rapid sub-operations
    pub fn micro_delay(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let ms = rng.gen_range(self.config.micro_delay_ms.clone());
        Duration::from_millis(ms)
    }

    /// Occasional distraction pause (checking phone, looking away)
    pub fn maybe_distraction(&self) -> Option<Duration> {
        let mut rng = rand::thread_rng();
        if self.config.fatigue_enabled && rng.gen::<f64>() < 0.02 {
            Some(Duration::from_millis(rng.gen_range(2000..8000)))
        } else {
            None
        }
    }

    /// Fatigue increases delays over time (logarithmic growth)
    fn fatigue_factor(&self) -> f64 {
        1.0 + (self.operation_count as f64 / 50.0).ln().max(0.0) * 0.15
    }

    pub fn reset(&mut self) {
        self.operation_count = 0;
    }
}
