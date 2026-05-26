use rand::Rng;

use super::profile::HumanProfileConfig;

#[derive(Debug, Clone)]
pub struct ScrollStep {
    pub delta_x: f64,
    pub delta_y: f64,
    pub delay_ms: u64,
}

pub struct ScrollHumanizer {
    config: HumanProfileConfig,
}

impl ScrollHumanizer {
    pub fn new(config: HumanProfileConfig) -> Self {
        Self { config }
    }

    /// Generate smooth scroll steps simulating trackpad/mouse wheel physics
    pub fn generate_scroll(&self, total_delta_y: f64) -> Vec<ScrollStep> {
        let mut rng = rand::thread_rng();
        let direction = total_delta_y.signum();
        let abs_delta = total_delta_y.abs();

        if abs_delta < 10.0 {
            return vec![ScrollStep {
                delta_x: 0.0,
                delta_y: total_delta_y,
                delay_ms: 0,
            }];
        }

        let mut steps = Vec::new();
        let mut remaining = abs_delta;

        // Simulate inertial scrolling: accelerate → cruise → decelerate
        let num_chunks = (abs_delta / 100.0).max(3.0).min(20.0) as usize;

        for i in 0..num_chunks {
            let progress = i as f64 / num_chunks as f64;

            // Bell curve: faster in the middle
            let speed_factor = (std::f64::consts::PI * progress).sin().max(0.2);
            let chunk_size = (abs_delta / num_chunks as f64) * speed_factor * 2.0;
            let actual_chunk = chunk_size.min(remaining);

            if actual_chunk < 1.0 {
                break;
            }

            let delay = if i == 0 {
                0
            } else {
                let base = 16; // ~60fps
                let jitter: i64 = rng.gen_range(-4..8);
                (base + jitter).max(8) as u64
            };

            steps.push(ScrollStep {
                delta_x: rng.gen_range(-0.5..0.5), // slight horizontal wobble
                delta_y: actual_chunk * direction,
                delay_ms: delay,
            });

            remaining -= actual_chunk;
        }

        // Add reading pause after scroll (simulates user reading content)
        if abs_delta > 300.0 && rng.gen::<f64>() < 0.6 {
            steps.push(ScrollStep {
                delta_x: 0.0,
                delta_y: 0.0,
                delay_ms: rng.gen_range(800..2500),
            });
        }

        steps
    }
}
