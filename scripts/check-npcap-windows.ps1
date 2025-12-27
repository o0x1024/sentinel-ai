# Check Npcap Installation Status for Sentinel AI
# This script verifies if Npcap is properly installed and configured

Write-Host "=== Sentinel AI - Npcap Installation Check ===" -ForegroundColor Cyan
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($isAdmin) {
    Write-Host "[OK] Running as Administrator" -ForegroundColor Green
} else {
    Write-Host "[WARNING] Not running as Administrator - some checks may fail" -ForegroundColor Yellow
    Write-Host "         For best results, run PowerShell as Administrator" -ForegroundColor Yellow
}
Write-Host ""

# Check Npcap Service
Write-Host "Checking Npcap Service..." -ForegroundColor Cyan
try {
    $npcapService = Get-Service -Name "npcap" -ErrorAction Stop
    if ($npcapService.Status -eq "Running") {
        Write-Host "[OK] Npcap service is running" -ForegroundColor Green
    } else {
        Write-Host "[ERROR] Npcap service is not running (Status: $($npcapService.Status))" -ForegroundColor Red
        Write-Host "        Try: Start-Service npcap" -ForegroundColor Yellow
    }
} catch {
    Write-Host "[ERROR] Npcap service not found - Npcap is not installed" -ForegroundColor Red
    Write-Host "        Download from: https://nmap.org/npcap/" -ForegroundColor Yellow
}
Write-Host ""

# Check Npcap DLL (WinPcap compatibility)
Write-Host "Checking Npcap DLL files..." -ForegroundColor Cyan
$npcapDllPaths = @(
    "C:\Windows\System32\Npcap\wpcap.dll",
    "C:\Windows\System32\Npcap\Packet.dll"
)

$dllFound = $false
foreach ($path in $npcapDllPaths) {
    if (Test-Path $path) {
        Write-Host "[OK] Found: $path" -ForegroundColor Green
        $dllFound = $true
    } else {
        Write-Host "[WARNING] Not found: $path" -ForegroundColor Yellow
    }
}

if (-not $dllFound) {
    Write-Host "[ERROR] Npcap DLL files not found" -ForegroundColor Red
    Write-Host "        Reinstall Npcap with 'WinPcap API-compatible Mode' enabled" -ForegroundColor Yellow
}
Write-Host ""

# Check Npcap SDK (for development)
Write-Host "Checking Npcap SDK (optional for development)..." -ForegroundColor Cyan
$sdkPath = "C:\Program Files\Npcap\SDK"
if (Test-Path $sdkPath) {
    Write-Host "[OK] Npcap SDK found at: $sdkPath" -ForegroundColor Green
    
    $packetLib = Join-Path $sdkPath "Lib\x64\Packet.lib"
    if (Test-Path $packetLib) {
        Write-Host "[OK] Packet.lib found: $packetLib" -ForegroundColor Green
    } else {
        Write-Host "[WARNING] Packet.lib not found in SDK" -ForegroundColor Yellow
    }
} else {
    Write-Host "[INFO] Npcap SDK not installed (not required for runtime)" -ForegroundColor Gray
}
Write-Host ""

# Check Network Adapters
Write-Host "Checking Network Adapters..." -ForegroundColor Cyan
try {
    $adapters = Get-NetAdapter | Where-Object {$_.Status -eq "Up"}
    if ($adapters.Count -gt 0) {
        Write-Host "[OK] Found $($adapters.Count) active network adapter(s):" -ForegroundColor Green
        foreach ($adapter in $adapters) {
            Write-Host "     - $($adapter.Name) ($($adapter.InterfaceDescription))" -ForegroundColor Gray
        }
    } else {
        Write-Host "[WARNING] No active network adapters found" -ForegroundColor Yellow
    }
} catch {
    Write-Host "[ERROR] Failed to enumerate network adapters: $_" -ForegroundColor Red
}
Write-Host ""

# Check Npcap Loopback Adapter
Write-Host "Checking Npcap Loopback Adapter..." -ForegroundColor Cyan
try {
    $loopback = Get-NetAdapter | Where-Object {$_.InterfaceDescription -like "*Npcap*Loopback*"}
    if ($loopback) {
        Write-Host "[OK] Npcap Loopback Adapter found: $($loopback.Name)" -ForegroundColor Green
        if ($loopback.Status -eq "Up") {
            Write-Host "     Status: Enabled" -ForegroundColor Green
        } else {
            Write-Host "     Status: Disabled (Status: $($loopback.Status))" -ForegroundColor Yellow
            Write-Host "     To enable: Enable-NetAdapter -Name '$($loopback.Name)'" -ForegroundColor Yellow
        }
    } else {
        Write-Host "[INFO] Npcap Loopback Adapter not installed (optional)" -ForegroundColor Gray
    }
} catch {
    Write-Host "[WARNING] Could not check for loopback adapter: $_" -ForegroundColor Yellow
}
Write-Host ""

# Summary
Write-Host "=== Summary ===" -ForegroundColor Cyan
$allGood = $true

if (-not (Get-Service -Name "npcap" -ErrorAction SilentlyContinue)) {
    Write-Host "[ACTION REQUIRED] Install Npcap from: https://nmap.org/npcap/" -ForegroundColor Red
    $allGood = $false
} elseif ((Get-Service -Name "npcap").Status -ne "Running") {
    Write-Host "[ACTION REQUIRED] Start Npcap service: Start-Service npcap" -ForegroundColor Red
    $allGood = $false
}

if (-not $dllFound) {
    Write-Host "[ACTION REQUIRED] Reinstall Npcap with 'WinPcap API-compatible Mode'" -ForegroundColor Red
    $allGood = $false
}

if ($allGood) {
    Write-Host ""
    Write-Host "All checks passed! Packet capture should work." -ForegroundColor Green
    Write-Host ""
    Write-Host "Note: You may need to run Sentinel AI as Administrator for packet capture." -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "Some issues detected. Please follow the actions above." -ForegroundColor Red
}

Write-Host ""
Write-Host "For detailed setup instructions, see:" -ForegroundColor Cyan
Write-Host "src-tauri/docs/windows_packet_capture_setup.md" -ForegroundColor Gray
Write-Host ""

# Pause if not running in automated mode
if ($Host.Name -eq "ConsoleHost") {
    Write-Host "Press any key to exit..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}

