use crate::service_probe::ServiceProbeEngineCapability;

#[derive(Debug, Clone)]
pub struct ServiceProbeEngineResolution {
    pub requested: String,
    pub used: String,
    pub experimental: bool,
    pub fallback_reason: Option<String>,
}

pub fn native_engine_capability() -> ServiceProbeEngineCapability {
    ServiceProbeEngineCapability {
        id: "native".to_string(),
        name: "Native Service Probe".to_string(),
        experimental: false,
        available: true,
        implemented: true,
        default_engine: true,
    }
}

pub fn resolve_service_probe_engine(requested: Option<&str>) -> ServiceProbeEngineResolution {
    let normalized = requested.unwrap_or("native").trim().to_lowercase();

    match normalized.as_str() {
        "" | "native" => ServiceProbeEngineResolution {
            requested: "native".to_string(),
            used: "native".to_string(),
            experimental: false,
            fallback_reason: None,
        },
        "builtin" | "pistol" => ServiceProbeEngineResolution {
            requested: normalized,
            used: "native".to_string(),
            experimental: false,
            fallback_reason: Some(
                "Legacy service probe engines are migrated to native".to_string(),
            ),
        },
        other => ServiceProbeEngineResolution {
            requested: other.to_string(),
            used: "native".to_string(),
            experimental: false,
            fallback_reason: Some(format!(
                "Unknown service probe engine: {other}; using native"
            )),
        },
    }
}
