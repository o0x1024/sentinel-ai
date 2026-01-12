# Build Docker sandbox image for Sentinel AI (Windows PowerShell)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$Dockerfile = Join-Path $ProjectRoot "src-tauri\sentinel-tools\Dockerfile.sandbox"
$ImageName = "sentinel-sandbox:latest"

Write-Host "Building Docker sandbox image..." -ForegroundColor Cyan
Write-Host "Dockerfile: $Dockerfile"
Write-Host "Image name: $ImageName"

if (-not (Test-Path $Dockerfile)) {
    Write-Host "Error: Dockerfile not found at $Dockerfile" -ForegroundColor Red
    exit 1
}

# Check if Docker is available
try {
    docker --version | Out-Null
} catch {
    Write-Host "Error: Docker is not installed or not in PATH" -ForegroundColor Red
    exit 1
}

# Build the image
Set-Location (Join-Path $ProjectRoot "src-tauri\sentinel-tools")
docker build -t $ImageName -f Dockerfile.sandbox .

Write-Host "`n✓ Docker sandbox image built successfully: $ImageName" -ForegroundColor Green
Write-Host ""
Write-Host "You can now use the shell tool with Docker execution mode."
Write-Host "To test the image, run:"
Write-Host "  docker run --rm -it $ImageName /bin/bash"
