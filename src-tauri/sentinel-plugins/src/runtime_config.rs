use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

const FETCH_QUEUE_DEPTH_MIN: u64 = 1;
const FETCH_QUEUE_DEPTH_MAX: u64 = 5_000;
const FETCH_PENDING_PER_RUN_MIN: u64 = 1;
const FETCH_PENDING_PER_RUN_MAX: u64 = 2_000;
const FETCH_PENDING_PER_PLUGIN_MIN: u64 = 1;
const FETCH_PENDING_PER_PLUGIN_MAX: u64 = 5_000;
const FETCH_GLOBAL_CONCURRENT_MIN: u64 = 1;
const FETCH_GLOBAL_CONCURRENT_MAX: u64 = 128;
const FETCH_CONCURRENT_PER_HOST_MIN: u64 = 1;
const FETCH_CONCURRENT_PER_HOST_MAX: u64 = 32;
const FETCH_CONCURRENT_PER_RUN_MIN: u64 = 1;
const FETCH_CONCURRENT_PER_RUN_MAX: u64 = 128;
const FETCH_CONCURRENT_PER_PLUGIN_MIN: u64 = 1;
const FETCH_CONCURRENT_PER_PLUGIN_MAX: u64 = 128;
const FETCH_JITTER_MIN_MS: u64 = 0;
const FETCH_JITTER_MAX_MS: u64 = 30_000;
const FETCH_DELAY_MIN_MS: u64 = 0;
const FETCH_DELAY_MAX_MS: u64 = 60_000;
const FETCH_TIMEOUT_MIN_MS: u64 = 3_000;
const FETCH_TIMEOUT_MAX_MS: u64 = 3_000;

const ACTIVE_PROBE_JITTER_MIN_MS: u64 = 0;
const ACTIVE_PROBE_JITTER_MAX_MS: u64 = 30_000;
const ACTIVE_PROBE_COOLDOWN_MIN_MS: u64 = 0;
const ACTIVE_PROBE_COOLDOWN_MAX_MS: u64 = 60_000;
const ACTIVE_PROBE_TIMEOUT_MIN_MS: u64 = 3_000;
const ACTIVE_PROBE_TIMEOUT_MAX_MS: u64 = 3_000;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFetchRuntimeSettings {
    pub max_queue_depth: u64,
    pub max_pending_per_run: u64,
    pub max_pending_per_plugin: u64,
    pub max_global_concurrent: u64,
    pub max_concurrent_per_host: u64,
    pub max_concurrent_per_run: u64,
    pub max_concurrent_per_plugin: u64,
    pub min_host_delay_ms: u64,
    pub jitter_range: [u64; 2],
    pub timeout_ms: u64,
}

impl PluginFetchRuntimeSettings {
    pub fn sanitized(&self) -> Self {
        let jitter_start = self.jitter_range[0].clamp(FETCH_JITTER_MIN_MS, FETCH_JITTER_MAX_MS);
        let jitter_end = self.jitter_range[1].clamp(FETCH_JITTER_MIN_MS, FETCH_JITTER_MAX_MS);
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
            max_concurrent_per_host: self
                .max_concurrent_per_host
                .clamp(FETCH_CONCURRENT_PER_HOST_MIN, FETCH_CONCURRENT_PER_HOST_MAX),
            max_concurrent_per_run: self
                .max_concurrent_per_run
                .clamp(FETCH_CONCURRENT_PER_RUN_MIN, FETCH_CONCURRENT_PER_RUN_MAX),
            max_concurrent_per_plugin: self.max_concurrent_per_plugin.clamp(
                FETCH_CONCURRENT_PER_PLUGIN_MIN,
                FETCH_CONCURRENT_PER_PLUGIN_MAX,
            ),
            min_host_delay_ms: self
                .min_host_delay_ms
                .clamp(FETCH_DELAY_MIN_MS, FETCH_DELAY_MAX_MS),
            jitter_range,
            timeout_ms: self
                .timeout_ms
                .clamp(FETCH_TIMEOUT_MIN_MS, FETCH_TIMEOUT_MAX_MS),
        }
    }
}

impl Default for PluginFetchRuntimeSettings {
    fn default() -> Self {
        Self {
            max_queue_depth: 1000,
            max_pending_per_run: 250,
            max_pending_per_plugin: 500,
            max_global_concurrent: 16,
            max_concurrent_per_host: 2,
            max_concurrent_per_run: 16,
            max_concurrent_per_plugin: 16,
            min_host_delay_ms: 1000,
            jitter_range: [300, 1000],
            timeout_ms: 3_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSettings {
    #[serde(default)]
    pub active_probe: ActiveProbeRuntimeSettings,
    #[serde(default = "default_bounty_fetch_runtime_settings")]
    pub bounty_fetch: PluginFetchRuntimeSettings,
    #[serde(default = "default_monitor_fetch_runtime_settings")]
    pub monitor_fetch: PluginFetchRuntimeSettings,
    #[serde(default = "default_agent_fetch_runtime_settings")]
    pub agent_fetch: PluginFetchRuntimeSettings,
    #[serde(default = "default_plugin_test_fetch_runtime_settings")]
    pub plugin_test_fetch: PluginFetchRuntimeSettings,
}

impl Default for PluginRuntimeSettings {
    fn default() -> Self {
        Self {
            active_probe: ActiveProbeRuntimeSettings::default(),
            bounty_fetch: default_bounty_fetch_runtime_settings(),
            monitor_fetch: default_monitor_fetch_runtime_settings(),
            agent_fetch: default_agent_fetch_runtime_settings(),
            plugin_test_fetch: default_plugin_test_fetch_runtime_settings(),
        }
    }
}

impl PluginRuntimeSettings {
    pub fn sanitized(&self) -> Self {
        Self {
            active_probe: self.active_probe.sanitized(),
            bounty_fetch: self.bounty_fetch.sanitized(),
            monitor_fetch: self.monitor_fetch.sanitized(),
            agent_fetch: self.agent_fetch.sanitized(),
            plugin_test_fetch: self.plugin_test_fetch.sanitized(),
        }
    }
}

fn default_bounty_fetch_runtime_settings() -> PluginFetchRuntimeSettings {
    PluginFetchRuntimeSettings::default()
}

fn default_monitor_fetch_runtime_settings() -> PluginFetchRuntimeSettings {
    PluginFetchRuntimeSettings {
        max_queue_depth: 1_000,
        max_pending_per_run: 250,
        max_pending_per_plugin: 500,
        max_global_concurrent: 16,
        max_concurrent_per_host: 2,
        max_concurrent_per_run: 16,
        max_concurrent_per_plugin: 16,
        min_host_delay_ms: 500,
        jitter_range: [50, 250],
        timeout_ms: 3_000,
    }
}

fn default_agent_fetch_runtime_settings() -> PluginFetchRuntimeSettings {
    PluginFetchRuntimeSettings {
        max_queue_depth: 300,
        max_pending_per_run: 75,
        max_pending_per_plugin: 150,
        max_global_concurrent: 16,
        max_concurrent_per_host: 2,
        max_concurrent_per_run: 16,
        max_concurrent_per_plugin: 16,
        min_host_delay_ms: 500,
        jitter_range: [100, 500],
        timeout_ms: 3_000,
    }
}

fn default_plugin_test_fetch_runtime_settings() -> PluginFetchRuntimeSettings {
    PluginFetchRuntimeSettings {
        max_queue_depth: 50,
        max_pending_per_run: 20,
        max_pending_per_plugin: 30,
        max_global_concurrent: 16,
        max_concurrent_per_host: 1,
        max_concurrent_per_run: 16,
        max_concurrent_per_plugin: 16,
        min_host_delay_ms: 100,
        jitter_range: [0, 100],
        timeout_ms: 3_000,
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
