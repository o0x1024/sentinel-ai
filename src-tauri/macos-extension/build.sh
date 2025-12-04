#!/bin/bash
# Sentinel AI Network Extension 构建脚本

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
BUILD_DIR="${SCRIPT_DIR}/build"
EXTENSION_NAME="SentinelProxy"
MANAGER_LIB="SentinelProxyManager"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Building Sentinel AI Network Extension...${NC}"

# 检查 Xcode
if ! command -v xcodebuild &> /dev/null; then
    echo -e "${RED}Error: Xcode command line tools not found${NC}"
    echo "Please install Xcode and run: xcode-select --install"
    exit 1
fi

# 创建构建目录
mkdir -p "${BUILD_DIR}"

# 构建配置
CONFIGURATION="Release"
ARCHS="x86_64 arm64"

echo -e "${YELLOW}Building for architectures: ${ARCHS}${NC}"

# 使用 Swift Package Manager 构建
cd "${SCRIPT_DIR}"

# 构建管理库
echo -e "${GREEN}Building ${MANAGER_LIB}...${NC}"
swift build -c release --product ${MANAGER_LIB}

# 复制构建产物
echo -e "${GREEN}Copying build artifacts...${NC}"

# 找到构建产物目录
BUILD_OUTPUT=$(swift build -c release --show-bin-path)
echo "Build output: ${BUILD_OUTPUT}"

# 复制动态库
if [ -f "${BUILD_OUTPUT}/lib${MANAGER_LIB}.dylib" ]; then
    cp "${BUILD_OUTPUT}/lib${MANAGER_LIB}.dylib" "${BUILD_DIR}/"
    echo -e "${GREEN}Copied lib${MANAGER_LIB}.dylib${NC}"
fi

# 构建 System Extension（需要 Xcode 项目，这里只是提示）
echo ""
echo -e "${YELLOW}Note: System Extension requires Xcode project for proper signing.${NC}"
echo -e "${YELLOW}To build the System Extension:${NC}"
echo "1. Open the Xcode project (create one if needed)"
echo "2. Configure your Apple Developer Team ID"
echo "3. Enable Network Extension capability"
echo "4. Build and sign the extension"
echo ""

# 生成头文件
echo -e "${GREEN}Generating C header...${NC}"
cat > "${BUILD_DIR}/sentinel_proxy.h" << 'EOF'
#ifndef SENTINEL_PROXY_H
#define SENTINEL_PROXY_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Extension Status
// 0: Unknown, 1: NotInstalled, 2: Installing, 3: Installed, 4: NeedsApproval, 5: Failed
int32_t sentinel_extension_check_status(void);
int32_t sentinel_extension_get_status(void);

// Extension Install/Uninstall
int32_t sentinel_extension_install(char* error_buffer, int32_t buffer_size);
int32_t sentinel_extension_uninstall(char* error_buffer, int32_t buffer_size);

// Proxy Control
int32_t sentinel_proxy_start(
    const char* proxy_host,
    uint16_t proxy_port,
    const char* target_apps_json,
    char* error_buffer,
    int32_t buffer_size
);
int32_t sentinel_proxy_stop(char* error_buffer, int32_t buffer_size);

// VPN Status
// 0: Invalid, 1: Disconnected, 2: Connecting, 3: Connected, 4: Reasserting, 5: Disconnecting
int32_t sentinel_proxy_get_status(void);

#ifdef __cplusplus
}
#endif

#endif // SENTINEL_PROXY_H
EOF

echo -e "${GREEN}Build complete!${NC}"
echo ""
echo "Build artifacts in: ${BUILD_DIR}"
ls -la "${BUILD_DIR}/"

