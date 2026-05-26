use crate::service_probe_engine::{native_engine_capability, resolve_service_probe_engine};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceProbeEngineCapability {
    pub id: String,
    pub name: String,
    pub experimental: bool,
    pub available: bool,
    pub implemented: bool,
    pub default_engine: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceProbeCapabilitiesResponse {
    pub default_engine: String,
    pub engines: Vec<ServiceProbeEngineCapability>,
}

pub fn service_probe_capabilities() -> ServiceProbeCapabilitiesResponse {
    let engines = vec![native_engine_capability()];

    ServiceProbeCapabilitiesResponse {
        default_engine: resolve_service_probe_engine(None).used,
        engines,
    }
}

pub fn op_get_service_probe_capabilities() -> ServiceProbeCapabilitiesResponse {
    service_probe_capabilities()
}
