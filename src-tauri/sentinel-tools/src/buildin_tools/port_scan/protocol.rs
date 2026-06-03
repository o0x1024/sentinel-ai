use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ScanProtocol {
    Tcp,
    Udp,
    Sctp,
}

impl ScanProtocol {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Sctp => "sctp",
        }
    }

    pub(crate) fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.trim().to_ascii_lowercase().as_str() {
            "t" | "tcp" => Some(Self::Tcp),
            "u" | "udp" => Some(Self::Udp),
            "s" | "sctp" => Some(Self::Sctp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RequestedScanPort {
    pub port: u16,
    pub protocol: ScanProtocol,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortScanSocketRef {
    pub port: u16,
    pub protocol: String,
}

impl From<RequestedScanPort> for PortScanSocketRef {
    fn from(value: RequestedScanPort) -> Self {
        Self {
            port: value.port,
            protocol: value.protocol.label().to_string(),
        }
    }
}
