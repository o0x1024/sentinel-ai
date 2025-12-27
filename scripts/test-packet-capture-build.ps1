# Test Packet Capture Build on Windows
# This script tests if the packet capture module compiles correctly

Write-Host "=== Testing Packet Capture Build ===" -ForegroundColor Cyan
Write-Host ""

# Check Rust installation
Write-Host "Checking Rust installation..." -ForegroundColor Cyan
try {
    $rustVersion = cargo --version
    Write-Host "[OK] $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Rust/Cargo not found. Install from: https://rustup.rs/" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Check Npcap
Write-Host "Checking Npcap installation..." -ForegroundColor Cyan
$npcapService = Get-Service -Name "npcap" -ErrorAction SilentlyContinue
if ($npcapService) {
    Write-Host "[OK] Npcap service found (Status: $($npcapService.Status))" -ForegroundColor Green
} else {
    Write-Host "[WARNING] Npcap not installed - build may fail" -ForegroundColor Yellow
    Write-Host "          Download from: https://nmap.org/npcap/" -ForegroundColor Yellow
}
Write-Host ""

# Navigate to project directory
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

# Test build sentinel-traffic crate
Write-Host "Building sentinel-traffic crate (packet capture module)..." -ForegroundColor Cyan
Write-Host "This may take a few minutes on first build..." -ForegroundColor Gray
Write-Host ""

try {
    Set-Location "src-tauri/sentinel-traffic"
    
    # Clean build
    Write-Host "Running: cargo clean" -ForegroundColor Gray
    cargo clean 2>&1 | Out-Null
    
    # Build
    Write-Host "Running: cargo build --release" -ForegroundColor Gray
    $buildOutput = cargo build --release 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "[SUCCESS] sentinel-traffic built successfully!" -ForegroundColor Green
        Write-Host ""
        Write-Host "Packet capture functionality is ready to use." -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "[ERROR] Build failed!" -ForegroundColor Red
        Write-Host ""
        Write-Host "Build output:" -ForegroundColor Yellow
        Write-Host $buildOutput
        Write-Host ""
        
        # Check for common errors
        if ($buildOutput -match "Packet.lib") {
            Write-Host "Possible issue: Packet.lib not found" -ForegroundColor Yellow
            Write-Host "Solution: Install Npcap SDK or ensure Npcap is installed correctly" -ForegroundColor Yellow
        }
        
        if ($buildOutput -match "pnet") {
            Write-Host "Possible issue: pnet crate compilation failed" -ForegroundColor Yellow
            Write-Host "Solution: Ensure Npcap is installed with WinPcap compatibility mode" -ForegroundColor Yellow
        }
        
        exit 1
    }
} catch {
    Write-Host "[ERROR] Build process failed: $_" -ForegroundColor Red
    exit 1
} finally {
    Set-Location $projectRoot
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Build the full application: npm run tauri build" -ForegroundColor Gray
Write-Host "2. Or run in dev mode: npm run tauri dev" -ForegroundColor Gray
Write-Host "3. Test packet capture in the Traffic Analysis tab" -ForegroundColor Gray
Write-Host ""

Write-Host "Note: Run the application as Administrator for full packet capture capabilities" -ForegroundColor Yellow
Write-Host ""

