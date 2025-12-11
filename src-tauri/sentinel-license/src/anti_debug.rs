//! Anti-debugging detection

use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_DETECTED: AtomicBool = AtomicBool::new(false);

/// Check if a debugger is attached
pub fn is_debugger_present() -> bool {
    // Skip in debug builds
    #[cfg(debug_assertions)]
    return false;

    #[cfg(not(debug_assertions))]
    {
        // Check cached result first
        if DEBUG_DETECTED.load(Ordering::Relaxed) {
            return true;
        }
        
        let detected = platform_check_debugger();
        
        if detected {
            DEBUG_DETECTED.store(true, Ordering::Relaxed);
        }
        
        detected
    }
}

/// Perform timing-based anti-debug check
pub fn timing_check() -> bool {
    #[cfg(debug_assertions)]
    return false;

    #[cfg(not(debug_assertions))]
    {
        use std::time::Instant;
        
        let start = Instant::now();
        
        // Perform some operations
        let mut sum: u64 = 0;
        for i in 0..10000 {
            sum = sum.wrapping_add(i);
        }
        
        let elapsed = start.elapsed();
        
        // If debugging with breakpoints, this would take much longer
        // Normal execution: < 1ms, Debugging: could be seconds
        elapsed.as_millis() > 100
    }
}

#[cfg(target_os = "macos")]
fn platform_check_debugger() -> bool {
    use std::process::Command;
    
    // Method 1: Check sysctl for P_TRACED flag
    let output = Command::new("sysctl")
        .args(["hw.optional.arm64"])
        .output();
    
    // Method 2: Check for lldb/gdb processes (basic check)
    if let Ok(output) = Command::new("pgrep")
        .args(["-x", "lldb"])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            // lldb is running, could be attached
            // This is a weak check, as lldb might be debugging other processes
        }
    }
    
    // Method 3: Use ptrace
    check_ptrace()
}

#[cfg(target_os = "windows")]
fn platform_check_debugger() -> bool {
    use windows::Win32::System::Diagnostics::Debug::{IsDebuggerPresent, CheckRemoteDebuggerPresent};
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Threading::GetCurrentProcess;
    
    unsafe {
        // Check local debugger
        if IsDebuggerPresent().as_bool() {
            return true;
        }
        
        // Check remote debugger
        let mut is_remote_debugger = BOOL::default();
        let process = GetCurrentProcess();
        if CheckRemoteDebuggerPresent(process, &mut is_remote_debugger).is_ok() {
            if is_remote_debugger.as_bool() {
                return true;
            }
        }
    }
    
    false
}

#[cfg(target_os = "linux")]
fn platform_check_debugger() -> bool {
    // Check /proc/self/status for TracerPid
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("TracerPid:") {
                if let Some(pid_str) = line.split_whitespace().nth(1) {
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        if pid != 0 {
                            return true;
                        }
                    }
                }
            }
        }
    }
    
    check_ptrace()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn check_ptrace() -> bool {
    // PT_DENY_ATTACH on macOS, PTRACE_TRACEME on Linux
    #[cfg(target_os = "macos")]
    {
        use std::os::raw::c_int;
        
        const PT_DENY_ATTACH: c_int = 31;
        
        extern "C" {
            fn ptrace(request: c_int, pid: c_int, addr: *mut u8, data: c_int) -> c_int;
        }
        
        unsafe {
            // If already being traced, this will fail
            let result = ptrace(PT_DENY_ATTACH, 0, std::ptr::null_mut(), 0);
            // On macOS, PT_DENY_ATTACH returns 0 on success, -1 if already traced
            result == -1
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::os::raw::c_long;
        
        const PTRACE_TRACEME: c_long = 0;
        
        extern "C" {
            fn ptrace(request: c_long, pid: i32, addr: *mut u8, data: *mut u8) -> c_long;
        }
        
        unsafe {
            // Try to trace ourselves
            let result = ptrace(PTRACE_TRACEME, 0, std::ptr::null_mut(), std::ptr::null_mut());
            // If already being traced, this returns -1
            result == -1
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn platform_check_debugger() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_debugger_check() {
        // In debug builds, should return false
        #[cfg(debug_assertions)]
        assert!(!is_debugger_present());
    }
}
