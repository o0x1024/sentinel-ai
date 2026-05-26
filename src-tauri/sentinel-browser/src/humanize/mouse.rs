use rand::Rng;
use rand_distr::{LogNormal, Normal, Distribution};

use crate::adapter::traits::Point;
use super::profile::HumanProfileConfig;

#[derive(Debug, Clone)]
pub struct MouseStep {
    pub x: f64,
    pub y: f64,
    pub delay_ms: u64,
}

pub struct MouseHumanizer {
    config: HumanProfileConfig,
}

impl MouseHumanizer {
    pub fn new(config: HumanProfileConfig) -> Self {
        Self { config }
    }

    /// Generate a human-like mouse path from `from` to `to` using Bezier curves + Fitts' Law
    pub fn generate_path(&self, from: Point, to: Point, target_size: f64) -> Vec<MouseStep> {
        let mut rng = rand::thread_rng();
        let distance = from.distance_to(&to);

        if distance < 1.0 {
            return vec![MouseStep { x: to.x, y: to.y, delay_ms: 0 }];
        }

        let movement_time_ms = self.fitts_time(distance, target_size);
        let num_steps = (movement_time_ms / 8.0).max(5.0) as usize; // ~8ms per step (125Hz)

        let control_points = self.random_control_points(&from, &to, distance, &mut rng);
        let mut steps = self.sample_cubic_bezier(&from, &to, &control_points, num_steps);

        self.add_jitter(&mut steps, &mut rng);
        self.apply_speed_curve(&mut steps, movement_time_ms);

        if rng.gen::<f64>() < self.config.overshoot_probability {
            self.add_overshoot(&mut steps, &to, target_size, &mut rng);
        }

        steps
    }

    /// Fitts' Law: MT = a + b * log2(2D/W)
    fn fitts_time(&self, distance: f64, target_width: f64) -> f64 {
        let a = 50.0; // base time ms
        let b = 150.0; // scaling factor
        let id = (2.0 * distance / target_width.max(1.0) + 1.0).log2(); // Index of Difficulty
        let speed_factor = rand::thread_rng()
            .gen_range(self.config.mouse_speed_factor.clone());
        (a + b * id) / speed_factor
    }

    fn random_control_points(
        &self, from: &Point, to: &Point, distance: f64, rng: &mut impl Rng,
    ) -> Vec<Point> {
        let mid = Point {
            x: (from.x + to.x) / 2.0,
            y: (from.y + to.y) / 2.0,
        };
        let spread = distance * 0.3;

        let cp1 = Point {
            x: mid.x + rng.gen_range(-spread..spread) * 0.5,
            y: mid.y + rng.gen_range(-spread..spread) * 0.5,
        };
        let cp2 = Point {
            x: mid.x + rng.gen_range(-spread..spread) * 0.3,
            y: mid.y + rng.gen_range(-spread..spread) * 0.3,
        };

        vec![cp1, cp2]
    }

    fn sample_cubic_bezier(
        &self, from: &Point, to: &Point, control_points: &[Point], num_steps: usize,
    ) -> Vec<MouseStep> {
        let cp1 = &control_points[0];
        let cp2 = &control_points[1];
        let mut steps = Vec::with_capacity(num_steps);

        for i in 0..=num_steps {
            let t = i as f64 / num_steps as f64;
            let mt = 1.0 - t;

            let x = mt.powi(3) * from.x
                + 3.0 * mt.powi(2) * t * cp1.x
                + 3.0 * mt * t.powi(2) * cp2.x
                + t.powi(3) * to.x;
            let y = mt.powi(3) * from.y
                + 3.0 * mt.powi(2) * t * cp1.y
                + 3.0 * mt * t.powi(2) * cp2.y
                + t.powi(3) * to.y;

            steps.push(MouseStep { x, y, delay_ms: 0 });
        }

        steps
    }

    fn add_jitter(&self, steps: &mut [MouseStep], rng: &mut impl Rng) {
        let normal = Normal::new(0.0, 1.5).unwrap();
        for step in steps.iter_mut() {
            step.x += normal.sample(rng);
            step.y += normal.sample(rng);
        }
    }

    /// Slow start → fast middle → slow end (ease-in-out)
    fn apply_speed_curve(&self, steps: &mut [MouseStep], total_time_ms: f64) {
        let n = steps.len();
        if n < 2 {
            return;
        }

        let mut cumulative_ease = Vec::with_capacity(n);
        let mut sum = 0.0;
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            // Sine ease-in-out: slower at edges
            let ease = (1.0 - (std::f64::consts::PI * t).cos()) / 2.0;
            let delta = if i == 0 { 0.0 } else { ease - cumulative_ease.last().unwrap_or(&0.0) };
            sum += delta.max(0.001);
            cumulative_ease.push(ease);
        }

        let mut elapsed = 0.0;
        for i in 0..n {
            let fraction = if i == 0 {
                0.0
            } else {
                (cumulative_ease[i] - cumulative_ease[i - 1]).max(0.001) / sum
            };
            let delay = (total_time_ms * fraction) as u64;
            steps[i].delay_ms = delay.max(4); // minimum 4ms between events
            elapsed += delay as f64;
        }
        steps[0].delay_ms = 0;
    }

    fn add_overshoot(
        &self, steps: &mut Vec<MouseStep>, target: &Point, target_size: f64, rng: &mut impl Rng,
    ) {
        let overshoot_dist = target_size * rng.gen_range(0.3..0.8);
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);

        let overshoot_point = MouseStep {
            x: target.x + overshoot_dist * angle.cos(),
            y: target.y + overshoot_dist * angle.sin(),
            delay_ms: rng.gen_range(30..80),
        };

        let correction = MouseStep {
            x: target.x,
            y: target.y,
            delay_ms: rng.gen_range(50..120),
        };

        steps.push(overshoot_point);
        steps.push(correction);
    }
}
