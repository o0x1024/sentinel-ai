//! Context budget analysis and pressure thresholds.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPressure {
    Low,
    Warning,
    AutoCompact,
    Blocking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudgetAnalysis {
    pub used_tokens: usize,
    pub remaining_tokens: usize,
    pub usage_percentage: u8,
    pub pressure: ContextPressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudgetAnalyzer {
    pub max_context_tokens: usize,
    pub safe_limit_tokens: usize,
    pub output_reserve_tokens: usize,
    pub effective_context_tokens: usize,
    pub warning_threshold_tokens: usize,
    pub auto_compact_threshold_tokens: usize,
    pub blocking_threshold_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextBudgetStrategy {
    pub safe_context_ratio: f64,
    pub output_reserve_ratio: f64,
    pub warning_buffer_ratio: f64,
    pub auto_compact_buffer_ratio: f64,
    pub blocking_buffer_ratio: f64,
}

impl Default for ContextBudgetStrategy {
    fn default() -> Self {
        Self {
            safe_context_ratio: 0.85,
            output_reserve_ratio: 0.15,
            warning_buffer_ratio: 0.08,
            auto_compact_buffer_ratio: 0.10,
            blocking_buffer_ratio: 0.03,
        }
    }
}

pub const MAX_OUTPUT_RESERVE_TOKENS: usize = 20_000;
pub const MAX_WARNING_BUFFER_TOKENS: usize = 20_000;
pub const MAX_AUTO_COMPACT_BUFFER_TOKENS: usize = 13_000;
pub const MAX_BLOCKING_BUFFER_TOKENS: usize = 3_000;

impl ContextBudgetAnalyzer {
    pub fn new(max_context_tokens: usize) -> Self {
        Self::with_strategy(max_context_tokens, ContextBudgetStrategy::default())
    }

    pub fn with_strategy(max_context_tokens: usize, strategy: ContextBudgetStrategy) -> Self {
        let strategy = strategy.normalized();
        let safe_limit_tokens = ratio_tokens(max_context_tokens, strategy.safe_context_ratio);
        let output_reserve_tokens = ratio_tokens(max_context_tokens, strategy.output_reserve_ratio)
            .min(MAX_OUTPUT_RESERVE_TOKENS);
        let effective_context_tokens = safe_limit_tokens.saturating_sub(output_reserve_tokens);

        let auto_buffer =
            ratio_tokens(effective_context_tokens, strategy.auto_compact_buffer_ratio)
                .min(MAX_AUTO_COMPACT_BUFFER_TOKENS)
                .max(1);
        let blocking_buffer =
            ratio_tokens(effective_context_tokens, strategy.blocking_buffer_ratio)
                .min(MAX_BLOCKING_BUFFER_TOKENS)
                .max(1);
        let warning_buffer = ratio_tokens(effective_context_tokens, strategy.warning_buffer_ratio)
            .min(MAX_WARNING_BUFFER_TOKENS)
            .max(1);

        let auto_compact_threshold_tokens = effective_context_tokens.saturating_sub(auto_buffer);
        let blocking_threshold_tokens = effective_context_tokens.saturating_sub(blocking_buffer);
        let warning_threshold_tokens = auto_compact_threshold_tokens.saturating_sub(warning_buffer);

        Self {
            max_context_tokens,
            safe_limit_tokens,
            output_reserve_tokens,
            effective_context_tokens,
            warning_threshold_tokens,
            auto_compact_threshold_tokens,
            blocking_threshold_tokens,
        }
    }

    pub fn analyze(&self, used_tokens: usize) -> ContextBudgetAnalysis {
        let remaining_tokens = self.effective_context_tokens.saturating_sub(used_tokens);
        let usage_percentage = if self.effective_context_tokens == 0 {
            0
        } else {
            ((used_tokens as f64 / self.effective_context_tokens as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u8
        };
        let pressure = if used_tokens >= self.blocking_threshold_tokens {
            ContextPressure::Blocking
        } else if used_tokens >= self.auto_compact_threshold_tokens {
            ContextPressure::AutoCompact
        } else if used_tokens >= self.warning_threshold_tokens {
            ContextPressure::Warning
        } else {
            ContextPressure::Low
        };

        ContextBudgetAnalysis {
            used_tokens,
            remaining_tokens,
            usage_percentage,
            pressure,
        }
    }

    pub fn history_segment_threshold(
        &self,
        global_summary_ratio: f64,
        segment_summary_ratio: f64,
    ) -> usize {
        let global_summary_budget = ratio_tokens(
            self.max_context_tokens,
            global_summary_ratio.clamp(0.0, 0.55),
        );
        let segment_summary_budget = ratio_tokens(
            self.max_context_tokens,
            segment_summary_ratio.clamp(0.0, 0.55),
        );
        let summary_budget = (global_summary_budget + segment_summary_budget)
            .min(ratio_tokens(self.max_context_tokens, 0.55));
        let threshold = self.safe_limit_tokens.saturating_sub(summary_budget);
        threshold.max(ratio_tokens(self.max_context_tokens, 0.30))
    }
}

impl ContextBudgetStrategy {
    fn normalized(self) -> Self {
        Self {
            safe_context_ratio: self.safe_context_ratio.clamp(0.40, 0.95),
            output_reserve_ratio: self.output_reserve_ratio.clamp(0.05, 0.35),
            warning_buffer_ratio: self.warning_buffer_ratio.clamp(0.01, 0.25),
            auto_compact_buffer_ratio: self.auto_compact_buffer_ratio.clamp(0.01, 0.25),
            blocking_buffer_ratio: self.blocking_buffer_ratio.clamp(0.005, 0.15),
        }
    }
}

fn ratio_tokens(tokens: usize, ratio: f64) -> usize {
    ((tokens as f64) * ratio).floor() as usize
}

#[cfg(test)]
mod tests {
    use super::{ContextBudgetAnalyzer, ContextBudgetStrategy, ContextPressure};

    #[test]
    fn computes_effective_thresholds_with_output_reserve() {
        let analyzer = ContextBudgetAnalyzer::new(128_000);

        assert_eq!(analyzer.safe_limit_tokens, 108_800);
        assert_eq!(analyzer.output_reserve_tokens, 19_200);
        assert_eq!(analyzer.effective_context_tokens, 89_600);
        assert_eq!(analyzer.auto_compact_threshold_tokens, 80_640);
        assert_eq!(analyzer.blocking_threshold_tokens, 86_912);
    }

    #[test]
    fn reports_pressure_levels() {
        let analyzer = ContextBudgetAnalyzer::new(128_000);

        assert_eq!(analyzer.analyze(10_000).pressure, ContextPressure::Low);
        assert_eq!(
            analyzer.analyze(analyzer.warning_threshold_tokens).pressure,
            ContextPressure::Warning
        );
        assert_eq!(
            analyzer
                .analyze(analyzer.auto_compact_threshold_tokens)
                .pressure,
            ContextPressure::AutoCompact
        );
        assert_eq!(
            analyzer
                .analyze(analyzer.blocking_threshold_tokens)
                .pressure,
            ContextPressure::Blocking
        );
    }

    #[test]
    fn history_segment_threshold_matches_summary_reservation() {
        let analyzer = ContextBudgetAnalyzer::new(128_000);

        assert_eq!(analyzer.history_segment_threshold(0.08, 0.15), 79_360);
    }

    #[test]
    fn small_context_keeps_positive_thresholds() {
        let analyzer = ContextBudgetAnalyzer::new(8_192);

        assert!(analyzer.effective_context_tokens > 0);
        assert!(analyzer.warning_threshold_tokens < analyzer.auto_compact_threshold_tokens);
        assert!(analyzer.auto_compact_threshold_tokens < analyzer.blocking_threshold_tokens);
    }

    #[test]
    fn custom_strategy_changes_thresholds_without_call_site_constants() {
        let analyzer = ContextBudgetAnalyzer::with_strategy(
            128_000,
            ContextBudgetStrategy {
                safe_context_ratio: 0.90,
                output_reserve_ratio: 0.10,
                warning_buffer_ratio: 0.05,
                auto_compact_buffer_ratio: 0.08,
                blocking_buffer_ratio: 0.02,
            },
        );

        assert_eq!(analyzer.safe_limit_tokens, 115_200);
        assert_eq!(analyzer.output_reserve_tokens, 12_800);
        assert_eq!(analyzer.effective_context_tokens, 102_400);
        assert!(analyzer.warning_threshold_tokens < analyzer.auto_compact_threshold_tokens);
        assert!(analyzer.auto_compact_threshold_tokens < analyzer.blocking_threshold_tokens);
    }
}
