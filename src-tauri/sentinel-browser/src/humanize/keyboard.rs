use rand::Rng;
use rand_distr::{LogNormal, Distribution};

use super::profile::HumanProfileConfig;

#[derive(Debug, Clone)]
pub struct KeyStroke {
    pub key: String,
    pub code: String,
    pub delay_before_ms: u64,
    pub hold_duration_ms: u64,
    pub corrections: Vec<KeyCorrection>,
}

#[derive(Debug, Clone)]
pub struct KeyCorrection {
    pub wrong_key: String,
    pub hold_ms: u64,
    pub pause_before_backspace_ms: u64,
}

pub struct KeyboardHumanizer {
    config: HumanProfileConfig,
}

impl KeyboardHumanizer {
    pub fn new(config: HumanProfileConfig) -> Self {
        Self { config }
    }

    pub fn generate_keystrokes(&self, text: &str) -> Vec<KeyStroke> {
        let mut rng = rand::thread_rng();
        let chars: Vec<char> = text.chars().collect();
        let mut keystrokes = Vec::with_capacity(chars.len());

        let base_interval_ms = 60_000.0 / (self.avg_wpm(&mut rng) as f64 * 5.0);

        for i in 0..chars.len() {
            let bigram_factor = if i > 0 {
                self.bigram_speed_factor(chars[i - 1], chars[i])
            } else {
                1.2 // first char slightly slower
            };

            let think_pause = self.think_pause(i, &chars, &mut rng);
            let variance = LogNormal::new(0.0, 0.25).unwrap().sample(&mut rng);

            let delay = (base_interval_ms * bigram_factor * variance) as u64 + think_pause;
            let hold = rng.gen_range(40..100);

            let corrections = if rng.gen::<f64>() < self.config.typo_rate {
                self.generate_typo(chars[i], &mut rng)
            } else {
                vec![]
            };

            keystrokes.push(KeyStroke {
                key: chars[i].to_string(),
                code: char_to_code(chars[i]),
                delay_before_ms: delay,
                hold_duration_ms: hold,
                corrections,
            });
        }

        keystrokes
    }

    fn avg_wpm(&self, rng: &mut impl Rng) -> u32 {
        rng.gen_range(self.config.wpm.clone())
    }

    /// Common bigrams (th, he, in, er) are typed faster; uncommon ones slower
    fn bigram_speed_factor(&self, prev: char, curr: char) -> f64 {
        let fast_bigrams = [
            "th", "he", "in", "er", "an", "re", "on", "at", "en", "nd",
            "ti", "es", "or", "te", "of", "ed", "is", "it", "al", "ar",
        ];
        let pair = format!("{}{}", prev.to_lowercase(), curr.to_lowercase());

        if fast_bigrams.contains(&pair.as_str()) {
            0.7 // faster for common pairs
        } else if prev.is_alphabetic() && curr.is_alphabetic() {
            1.0
        } else {
            1.3 // slower for punctuation transitions
        }
    }

    /// Insert thinking pauses at natural break points
    fn think_pause(&self, idx: usize, chars: &[char], rng: &mut impl Rng) -> u64 {
        if idx == 0 {
            return rng.gen_range(self.config.think_pause_ms.clone());
        }

        let prev = chars[idx - 1];
        // Pause after sentence-ending punctuation
        if matches!(prev, '.' | '!' | '?' | '\n') {
            return rng.gen_range(300..1200);
        }
        // Slight pause after comma/semicolon
        if matches!(prev, ',' | ';' | ':') {
            return rng.gen_range(100..400);
        }
        // Occasional random pause (thinking mid-word)
        if rng.gen::<f64>() < 0.03 {
            return rng.gen_range(200..600);
        }

        0
    }

    fn generate_typo(&self, intended: char, rng: &mut impl Rng) -> Vec<KeyCorrection> {
        let neighbors = keyboard_neighbors(intended);
        if neighbors.is_empty() {
            return vec![];
        }
        let wrong = neighbors[rng.gen_range(0..neighbors.len())];

        vec![KeyCorrection {
            wrong_key: wrong.to_string(),
            hold_ms: rng.gen_range(40..80),
            pause_before_backspace_ms: rng.gen_range(100..400),
        }]
    }
}

fn char_to_code(c: char) -> String {
    match c {
        'a'..='z' => format!("Key{}", c.to_uppercase()),
        'A'..='Z' => format!("Key{}", c),
        '0'..='9' => format!("Digit{}", c),
        ' ' => "Space".to_string(),
        '\n' => "Enter".to_string(),
        '\t' => "Tab".to_string(),
        '.' => "Period".to_string(),
        ',' => "Comma".to_string(),
        ';' => "Semicolon".to_string(),
        '/' => "Slash".to_string(),
        '-' => "Minus".to_string(),
        '=' => "Equal".to_string(),
        '[' => "BracketLeft".to_string(),
        ']' => "BracketRight".to_string(),
        '\\' => "Backslash".to_string(),
        '\'' => "Quote".to_string(),
        _ => format!("Key{}", c),
    }
}

/// Returns adjacent keys on a QWERTY keyboard
fn keyboard_neighbors(c: char) -> Vec<char> {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'q' => vec!['w', 'a'],
        'w' => vec!['q', 'e', 'a', 's'],
        'e' => vec!['w', 'r', 's', 'd'],
        'r' => vec!['e', 't', 'd', 'f'],
        't' => vec!['r', 'y', 'f', 'g'],
        'y' => vec!['t', 'u', 'g', 'h'],
        'u' => vec!['y', 'i', 'h', 'j'],
        'i' => vec!['u', 'o', 'j', 'k'],
        'o' => vec!['i', 'p', 'k', 'l'],
        'p' => vec!['o', 'l'],
        'a' => vec!['q', 'w', 's', 'z'],
        's' => vec!['w', 'e', 'a', 'd', 'z', 'x'],
        'd' => vec!['e', 'r', 's', 'f', 'x', 'c'],
        'f' => vec!['r', 't', 'd', 'g', 'c', 'v'],
        'g' => vec!['t', 'y', 'f', 'h', 'v', 'b'],
        'h' => vec!['y', 'u', 'g', 'j', 'b', 'n'],
        'j' => vec!['u', 'i', 'h', 'k', 'n', 'm'],
        'k' => vec!['i', 'o', 'j', 'l', 'm'],
        'l' => vec!['o', 'p', 'k'],
        'z' => vec!['a', 's', 'x'],
        'x' => vec!['z', 's', 'd', 'c'],
        'c' => vec!['x', 'd', 'f', 'v'],
        'v' => vec!['c', 'f', 'g', 'b'],
        'b' => vec!['v', 'g', 'h', 'n'],
        'n' => vec!['b', 'h', 'j', 'm'],
        'm' => vec!['n', 'j', 'k'],
        _ => vec![],
    }
}
