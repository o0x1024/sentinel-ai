//! Machine ID generation - Cross-platform unique identifier

use sha2::{Sha256, Digest};
use sysinfo::System;

/// Machine identifier based on hardware characteristics
#[derive(Debug, Clone)]
pub struct MachineId {
    #[allow(dead_code)]
    components: Vec<String>,
    hash: [u8; 32],
}

impl MachineId {
    /// Generate machine ID from hardware components
    pub fn generate() -> Self {
        let mut components = Vec::new();
        
        // Get MAC address
        if let Some(mac) = get_mac_address() {
            components.push(mac);
        }
        
        // Get CPU info
        if let Some(cpu) = get_cpu_info() {
            components.push(cpu);
        }
        
        // Get platform-specific identifiers
        #[cfg(target_os = "macos")]
        if let Some(serial) = get_macos_serial() {
            components.push(serial);
        }
        
        #[cfg(target_os = "windows")]
        if let Some(id) = get_windows_machine_guid() {
            components.push(id);
        }
        
        #[cfg(target_os = "linux")]
        if let Some(id) = get_linux_machine_id() {
            components.push(id);
        }
        
        // Fallback: hostname
        if let Some(hostname) = get_hostname() {
            components.push(hostname);
        }
        
        // Compute hash
        let combined = components.join("|");
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        
        Self { components, hash }
    }
    
    /// Get raw hash bytes
    pub fn to_hash(&self) -> [u8; 32] {
        self.hash
    }
    
    /// Get display string (shortened hash for user)
    pub fn to_display_string(&self) -> String {
        let hex = hex::encode(&self.hash[..16]);
        // Format: XXXX-XXXX-XXXX-XXXX
        format!("{}-{}-{}-{}",
            &hex[0..4].to_uppercase(),
            &hex[4..8].to_uppercase(),
            &hex[8..12].to_uppercase(),
            &hex[12..16].to_uppercase(),
        )
    }
    
    /// Get full hash as hex string
    pub fn to_full_hex(&self) -> String {
        hex::encode(self.hash)
    }
}

fn get_mac_address() -> Option<String> {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|mac| mac.to_string())
}

fn get_cpu_info() -> Option<String> {
    let sys = System::new_all();
    let cpus = sys.cpus();
    
    if cpus.is_empty() {
        return None;
    }
    
    let cpu = &cpus[0];
    Some(format!("{}:{}", cpu.brand(), cpus.len()))
}

fn get_hostname() -> Option<String> {
    System::host_name()
}

#[cfg(target_os = "macos")]
fn get_macos_serial() -> Option<String> {
    use std::process::Command;
    
    // Get IOPlatformSerialNumber
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse IOPlatformSerialNumber
    for line in stdout.lines() {
        if line.contains("IOPlatformSerialNumber") {
            if let Some(start) = line.find('"') {
                let rest = &line[start + 1..];
                if let Some(end) = rest.rfind('"') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    
    // Fallback: Hardware UUID
    let output = Command::new("system_profiler")
        .args(["SPHardwareDataType"])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("Hardware UUID") {
            if let Some(uuid) = line.split(':').nth(1) {
                return Some(uuid.trim().to_string());
            }
        }
    }
    
    None
}

#[cfg(target_os = "windows")]
fn get_windows_machine_guid() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(r"SOFTWARE\Microsoft\Cryptography").ok()?;
    let guid: String = key.get_value("MachineGuid").ok()?;
    
    Some(guid)
}

#[cfg(target_os = "linux")]
fn get_linux_machine_id() -> Option<String> {
    // Try /etc/machine-id first
    if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
        return Some(id.trim().to_string());
    }
    
    // Fallback to /var/lib/dbus/machine-id
    if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
        return Some(id.trim().to_string());
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_machine_id_deterministic() {
        let id1 = MachineId::generate();
        let id2 = MachineId::generate();
        
        assert_eq!(id1.to_hash(), id2.to_hash());
        assert_eq!(id1.to_display_string(), id2.to_display_string());
    }
    
    #[test]
    fn test_display_format() {
        let id = MachineId::generate();
        let display = id.to_display_string();
        
        // Should be XXXX-XXXX-XXXX-XXXX format
        let parts: Vec<&str> = display.split('-').collect();
        assert_eq!(parts.len(), 4);
        for part in parts {
            assert_eq!(part.len(), 4);
        }
    }
}
