#!/bin/bash
# Build Docker sandbox image for Sentinel AI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOLS_DIR="$PROJECT_ROOT/src-tauri/sentinel-tools"

# Parse arguments
VARIANT="${1:-kali}"  # kali, minimal, kali-full
IMAGE_NAME="sentinel-sandbox:latest"

case "$VARIANT" in
    minimal)
        DOCKERFILE="Dockerfile.sandbox.minimal"
        echo "Building minimal Docker sandbox image (Kali Linux based)..."
        ;;
    kali-full)
        DOCKERFILE="Dockerfile.sandbox.kali-full"
        echo "Building full Kali Linux sandbox image (with all security tools)..."
        ;;
    kali|full)
        DOCKERFILE="Dockerfile.sandbox"
        echo "Building standard Kali Linux sandbox image..."
        ;;
    *)
        echo "Unknown variant: $VARIANT"
        echo "Usage: $0 [kali|minimal|kali-full]"
        echo "  kali       - Standard Kali with top tools (default)"
        echo "  minimal    - Minimal Kali with essential tools only"
        echo "  kali-full  - Full Kali with comprehensive security tools"
        exit 1
        ;;
esac

echo "Dockerfile: $DOCKERFILE"
echo "Image name: $IMAGE_NAME"

if [ ! -f "$TOOLS_DIR/$DOCKERFILE" ]; then
    echo "Error: Dockerfile not found at $TOOLS_DIR/$DOCKERFILE"
    exit 1
fi

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

# Build the image
cd "$TOOLS_DIR"
echo ""
echo "Building image (this may take a few minutes)..."
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE" .

if [ $? -eq 0 ]; then
    echo ""
    echo "✓ Docker sandbox image built successfully: $IMAGE_NAME"
    
    # Show image size
    IMAGE_SIZE=$(docker images $IMAGE_NAME --format "{{.Size}}" | head -1)
    echo "  Image size: $IMAGE_SIZE"
    
    echo ""
    echo "You can now use the shell tool with Docker execution mode."
    echo "To test the image, run:"
    echo "  docker run --rm -it $IMAGE_NAME /bin/bash"
    echo ""
    echo "Available tools in this image:"
    case "$VARIANT" in
        minimal)
            echo "  - Basic: curl, wget, git, nmap, python3"
            ;;
        kali-full)
            echo "  - Comprehensive Kali security tools"
            echo "  - Web testing, exploitation, password attacks, wireless"
            ;;
        *)
            echo "  - Kali top 10 tools + common security utilities"
            ;;
    esac
else
    echo ""
    echo "✗ Build failed."
    echo ""
    echo "Try these alternatives:"
    echo "  $0 minimal     - Fastest build, essential tools only"
    echo "  $0 kali        - Standard Kali with top tools"
    echo "  $0 kali-full   - Complete Kali security suite"
    exit 1
fi
