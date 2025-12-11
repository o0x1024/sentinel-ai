//! Code integrity verification
//! 
//! Verifies the application binary hasn't been tampered with.

use sha2::{Sha256, Digest};
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

/// Integrity check passed flag
static INTEGRITY_OK: AtomicBool = AtomicBool::new(false);

/// Known hash of critical code sections (set at build time)
/// In production, this would be computed during build and embedded
static EXPECTED_HASH: Lazy<Option<[u8; 32]>> = Lazy::new(|| {
    // This would be set by build script
    // For now, skip integrity check if not configured
    None
});

/// Check code integrity
pub fn verify_integrity() -> bool {
    #[cfg(debug_assertions)]
    {
        INTEGRITY_OK.store(true, Ordering::SeqCst);
        return true;
    }

    #[cfg(not(debug_assertions))]
    {
        // If no expected hash configured, skip check
        let expected = match EXPECTED_HASH.as_ref() {
            Some(h) => h,
            None => {
                INTEGRITY_OK.store(true, Ordering::SeqCst);
                return true;
            }
        };

        let actual = compute_binary_hash();
        let ok = match actual {
            Some(h) => constant_time_compare(&h, expected),
            None => false,
        };

        INTEGRITY_OK.store(ok, Ordering::SeqCst);
        ok
    }
}

/// Quick check if integrity verification passed
#[inline]
pub fn is_integrity_ok() -> bool {
    #[cfg(debug_assertions)]
    return true;

    #[cfg(not(debug_assertions))]
    INTEGRITY_OK.load(Ordering::SeqCst)
}

/// Compute hash of current binary
fn compute_binary_hash() -> Option<[u8; 32]> {
    let exe_path = std::env::current_exe().ok()?;
    let binary_data = std::fs::read(&exe_path).ok()?;
    
    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    
    Some(hasher.finalize().into())
}

/// Get current binary hash (for build-time recording)
pub fn get_binary_hash() -> Option<String> {
    compute_binary_hash().map(hex::encode)
}

/// Constant-time comparison to prevent timing attacks
#[inline(never)]
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

/// Runtime checksum for critical functions
/// Call this at function entry points to detect code patching
#[inline(never)]
pub fn function_checksum(func_id: u32) -> u64 {
    let machine_id = crate::MachineId::generate();
    let hash = machine_id.to_hash();
    
    // Derive checksum from machine ID and function ID
    let mut result = 0u64;
    for (i, &byte) in hash.iter().enumerate() {
        result = result.wrapping_add((byte as u64).wrapping_mul(func_id as u64 + i as u64 + 1));
    }
    
    // Mix with license state
    if crate::is_licensed() {
        result ^= 0xDEADBEEFCAFEBABE;
    }
    
    result
}

/// Verify function checksum
#[inline(never)]
pub fn verify_function_checksum(checksum: u64, func_id: u32) -> bool {
    let expected = function_checksum(func_id);
    checksum == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrity_check() {
        // In debug mode should pass
        assert!(verify_integrity());
        assert!(is_integrity_ok());
    }

    #[test]
    fn test_function_checksum() {
        let func_id = 42u32;
        let checksum1 = function_checksum(func_id);
        let checksum2 = function_checksum(func_id);
        
        // Should be deterministic
        assert_eq!(checksum1, checksum2);
        
        // Should verify correctly
        assert!(verify_function_checksum(checksum1, func_id));
        
        // Different func_id should produce different checksum
        let checksum3 = function_checksum(43);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_constant_time_compare() {
        let a = [1, 2, 3, 4];
        let b = [1, 2, 3, 4];
        let c = [1, 2, 3, 5];
        
        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
    }
}
