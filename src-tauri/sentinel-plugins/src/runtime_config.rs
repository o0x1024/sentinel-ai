use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

const FETCH_QUEUE_DEPTH_MIN: u64 = 1;
const FETCH_QUEUE_DEPTH_MAX: u64 = 10_000;
const FETCH_PENDING_PER_RUN_MIN: u64 = 1;
const FETCH_PENDING_PER_RUN_MAX: u64 = 5_000;
const FETCH_PENDING_PER_PLUGIN_MIN: u64 = 1;
const FETCH_PENDING_PER_PLUGIN_MAX: u64 = 10_000;
const FETCH_GLOBAL_CONCURRENT_MIN: u64 = 1;
const FETCH_GLOBAL_CONCURRENT_MAX: u64 = 1000;
const FETCH_CONCURRENT_PER_RUN_MIN: u64 = 1;
const FETCH_CONCURRENT_PER_RUN_MAX: u64 = 1000;
const FETCH_CONCURRENT_PER_PLUGIN_MIN: u64 = 1;
const FETCH_CONCURRENT_PER_PLUGIN_MAX: u64 = 1000;
const ACTIVE_PROBE_JITTER_MIN_MS: u64 = 0;
const ACTIVE_PROBE_JITTER_MAX_MS: u64 = 30_000;
const ACTIVE_PROBE_COOLDOWN_MIN_MS: u64 = 0;
const ACTIVE_PROBE_COOLDOWN_MAX_MS: u64 = 60_000;
const ACTIVE_PROBE_TIMEOUT_MIN_MS: u64 = 1_000;
const ACTIVE_PROBE_TIMEOUT_MAX_MS: u64 = 120_000;
const ACTIVE_PROBE_MAX_CONCURRENT_PER_HOST_MIN: u64 = 1;
const ACTIVE_PROBE_MAX_CONCURRENT_PER_HOST_MAX: u64 = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveProbeRuntimeSettings {
    pub max_queue_depth: u64,
    pub max_pending_per_run: u64,
    pub max_pending_per_plugin: u64,
    pub max_global_concurrent: u64,
    pub jitter_range: [u64; 2],
    pub min_host_cooldown_ms: u64,
    pub max_concurrent_per_host: u64,
    pub max_concurrent_per_run: u64,
    pub max_concurrent_per_plugin: u64,
    pub timeout_ms: u64,
}

impl Default for ActiveProbeRuntimeSettings {
    fn default() -> Self {
        Self {
            max_queue_depth: 1000,
            max_pending_per_run: 250,
            max_pending_per_plugin: 500,
            max_global_concurrent: 16,
            jitter_range: [300, 1000],
            min_host_cooldown_ms: 1000,
            max_concurrent_per_host: 2,
            max_concurrent_per_run: 16,
            max_concurrent_per_plugin: 16,
            timeout_ms: 8000,
        }
    }
}

impl ActiveProbeRuntimeSettings {
    pub fn sanitized(&self) -> Self {
        let jitter_start =
            self.jitter_range[0].clamp(ACTIVE_PROBE_JITTER_MIN_MS, ACTIVE_PROBE_JITTER_MAX_MS);
        let jitter_end =
            self.jitter_range[1].clamp(ACTIVE_PROBE_JITTER_MIN_MS, ACTIVE_PROBE_JITTER_MAX_MS);
        let jitter_range = if jitter_start <= jitter_end {
            [jitter_start, jitter_end]
        } else {
            [jitter_end, jitter_start]
        };

        Self {
            max_queue_depth: self
                .max_queue_depth
                .clamp(FETCH_QUEUE_DEPTH_MIN, FETCH_QUEUE_DEPTH_MAX),
            max_pending_per_run: self
                .max_pending_per_run
                .clamp(FETCH_PENDING_PER_RUN_MIN, FETCH_PENDING_PER_RUN_MAX),
            max_pending_per_plugin: self
                .max_pending_per_plugin
                .clamp(FETCH_PENDING_PER_PLUGIN_MIN, FETCH_PENDING_PER_PLUGIN_MAX),
            max_global_concurrent: self
                .max_global_concurrent
                .clamp(FETCH_GLOBAL_CONCURRENT_MIN, FETCH_GLOBAL_CONCURRENT_MAX),
            jitter_range,
            min_host_cooldown_ms: self
                .min_host_cooldown_ms
                .clamp(ACTIVE_PROBE_COOLDOWN_MIN_MS, ACTIVE_PROBE_COOLDOWN_MAX_MS),
            max_concurrent_per_host: self.max_concurrent_per_host.clamp(
                ACTIVE_PROBE_MAX_CONCURRENT_PER_HOST_MIN,
                ACTIVE_PROBE_MAX_CONCURRENT_PER_HOST_MAX,
            ),
            max_concurrent_per_run: self
                .max_concurrent_per_run
                .clamp(FETCH_CONCURRENT_PER_RUN_MIN, FETCH_CONCURRENT_PER_RUN_MAX),
            max_concurrent_per_plugin: self.max_concurrent_per_plugin.clamp(
                FETCH_CONCURRENT_PER_PLUGIN_MIN,
                FETCH_CONCURRENT_PER_PLUGIN_MAX,
            ),
            timeout_ms: self
                .timeout_ms
                .clamp(ACTIVE_PROBE_TIMEOUT_MIN_MS, ACTIVE_PROBE_TIMEOUT_MAX_MS),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSettings {
    #[serde(default)]
    pub active_probe: ActiveProbeRuntimeSettings,
}

impl Default for PluginRuntimeSettings {
    fn default() -> Self {
        Self {
            active_probe: ActiveProbeRuntimeSettings::default(),
        }
    }
}

impl PluginRuntimeSettings {
    pub fn sanitized(&self) -> Self {
        Self {
            active_probe: self.active_probe.sanitized(),
        }
    }
}

static PLUGIN_RUNTIME_SETTINGS: OnceLock<RwLock<PluginRuntimeSettings>> = OnceLock::new();

fn plugin_runtime_settings_store() -> &'static RwLock<PluginRuntimeSettings> {
    PLUGIN_RUNTIME_SETTINGS.get_or_init(|| RwLock::new(PluginRuntimeSettings::default()))
}

pub fn get_plugin_runtime_settings() -> PluginRuntimeSettings {
    plugin_runtime_settings_store()
        .read()
        .expect("plugin runtime settings poisoned")
        .clone()
}

pub fn set_plugin_runtime_settings(settings: PluginRuntimeSettings) {
    let sanitized = settings.sanitized();
    let mut current = plugin_runtime_settings_store()
        .write()
        .expect("plugin runtime settings poisoned");
    *current = sanitized;
}

#[cfg(test)]
mod tests {
    use super::ActiveProbeRuntimeSettings;

    #[test]
    fn preserves_configured_timeout_within_runtime_limits() {
        let active_probe = ActiveProbeRuntimeSettings {
            timeout_ms: 5_000,
            ..ActiveProbeRuntimeSettings::default()
        }
        .sanitized();

        assert_eq!(active_probe.timeout_ms, 5_000);
    }

    #[test]
    fn clamps_configured_timeout_to_runtime_limits() {
        let active_probe = ActiveProbeRuntimeSettings {
            timeout_ms: 999_999,
            ..ActiveProbeRuntimeSettings::default()
        }
        .sanitized();

        assert_eq!(active_probe.timeout_ms, 120_000);
    }
}
