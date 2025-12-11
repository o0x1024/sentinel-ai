//! String obfuscation and validation logic mixing

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// XOR key for string encryption (compile-time constant)
const XOR_KEY: &[u8] = b"S3nt1n3l_A1_L1c3ns3_K3y_2024";

/// Pre-encrypted strings (encrypted at compile time conceptually)
/// In real implementation, use a build script to encrypt these
static ENCRYPTED_STRINGS: Lazy<HashMap<&'static str, Vec<u8>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Encrypt common error messages
    map.insert("license_invalid", encrypt_string("License verification failed"));
    map.insert("machine_mismatch", encrypt_string("Machine ID does not match"));
    map.insert("activation_required", encrypt_string("Activation required"));
    map.insert("debug_detected", encrypt_string("Abnormal environment detected"));
    map.insert("tampered", encrypt_string("Application integrity compromised"));
    
    map
});

/// Encrypt a string with XOR
fn encrypt_string(s: &str) -> Vec<u8> {
    s.as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

/// Decrypt a byte slice with XOR
fn decrypt_bytes(encrypted: &[u8]) -> String {
    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect();
    
    String::from_utf8_lossy(&decrypted).to_string()
}

/// Get decrypted string by key
pub fn decrypt_str(key: &str) -> String {
    ENCRYPTED_STRINGS
        .get(key)
        .map(|encrypted| decrypt_bytes(encrypted))
        .unwrap_or_else(|| key.to_string())
}

/// Obfuscated boolean check - returns value that must be used
/// Makes simple patching ineffective
#[inline(never)]
pub fn obfuscated_check(condition: bool, salt: u64) -> u64 {
    if condition {
        // Return a value derived from the salt
        salt.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0xBB67AE8584CAA73B)
    } else {
        // Return a different pattern
        salt.wrapping_mul(0x6A09E667F3BCC908).wrapping_add(0x3C6EF372FE94F82B)
    }
}

/// Verify the obfuscated result
#[inline(never)]
pub fn verify_obfuscated(result: u64, expected_salt: u64) -> bool {
    let expected = expected_salt.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0xBB67AE8584CAA73B);
    result == expected
}

/// Derive a key from license state (for mixing with business logic)
pub fn derive_feature_key(licensed: bool, feature_id: u32) -> u64 {
    let base = if licensed {
        0xDEADBEEFCAFEBABE_u64
    } else {
        0x0000000000000000_u64
    };
    
    base ^ (feature_id as u64).wrapping_mul(0x517CC1B727220A95)
}

/// Check if derived key is valid
#[inline(never)]
pub fn check_feature_key(key: u64, feature_id: u32) -> bool {
    let expected = 0xDEADBEEFCAFEBABE_u64 ^ (feature_id as u64).wrapping_mul(0x517CC1B727220A95);
    key == expected
}

/// Timing-safe comparison
#[inline(never)]
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_encryption() {
        let original = "Hello, World!";
        let encrypted = encrypt_string(original);
        let decrypted = decrypt_bytes(&encrypted);
        
        assert_eq!(decrypted, original);
        assert_ne!(encrypted, original.as_bytes());
    }
    
    #[test]
    fn test_predefined_strings() {
        let msg = decrypt_str("license_invalid");
        assert_eq!(msg, "License verification failed");
    }
    
    #[test]
    fn test_obfuscated_check() {
        let salt = 12345u64;
        
        let valid = obfuscated_check(true, salt);
        let invalid = obfuscated_check(false, salt);
        
        assert!(verify_obfuscated(valid, salt));
        assert!(!verify_obfuscated(invalid, salt));
    }
    
    #[test]
    fn test_feature_key() {
        let feature_id = 42u32;
        
        let valid_key = derive_feature_key(true, feature_id);
        let invalid_key = derive_feature_key(false, feature_id);
        
        assert!(check_feature_key(valid_key, feature_id));
        assert!(!check_feature_key(invalid_key, feature_id));
    }
}
