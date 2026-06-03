#!/bin/bash

# 证书验证脚本
# 用于检查 Sentinel AI CA 证书是否正确安装

set -e

CERT_PATH="$HOME/Library/Application Support/sentinel-ai/ca/root-ca.pem"
CERT_NAME="Sentinel AI Traffic Analysis CA"

echo "=== Sentinel AI 证书验证工具 ==="
echo ""

# 1. 检查证书文件是否存在
echo "1. 检查证书文件..."
if [ -f "$CERT_PATH" ]; then
    echo "✅ 证书文件存在: $CERT_PATH"
else
    echo "❌ 证书文件不存在: $CERT_PATH"
    echo "   请先启动代理以生成证书"
    exit 1
fi

# 2. 查看证书信息
echo ""
echo "2. 证书信息:"
openssl x509 -in "$CERT_PATH" -text -noout | grep -A 2 "Subject:"
openssl x509 -in "$CERT_PATH" -text -noout | grep -A 2 "Validity"

# 3. 检查证书是否在钥匙串中
echo ""
echo "3. 检查系统钥匙串..."
if security find-certificate -c "$CERT_NAME" -a 2>/dev/null | grep -q "$CERT_NAME"; then
    echo "✅ 证书已安装到系统钥匙串"
    
    # 检查信任设置
    echo ""
    echo "4. 检查信任设置..."
    security dump-trust-settings -d 2>/dev/null | grep -A 5 "$CERT_NAME" || echo "⚠️  无法获取信任设置详情"
else
    echo "❌ 证书未安装到系统钥匙串"
    echo ""
    echo "请执行以下步骤安装证书："
    echo "1. 在应用界面点击 '一键信任证书 (macOS)' 按钮"
    echo "2. 或手动执行命令："
    echo "   sudo security add-trusted-cert -d -r trustRoot \\"
    echo "     -k /Library/Keychains/System.keychain \\"
    echo "     \"$CERT_PATH\""
    exit 1
fi

# 5. 验证证书
echo ""
echo "5. 验证证书有效性..."
if openssl verify "$CERT_PATH" 2>&1 | grep -q "OK"; then
    echo "✅ 证书验证成功（自签名证书）"
else
    echo "⚠️  证书验证警告（自签名证书正常会有此警告）"
fi

# 6. 获取证书指纹
echo ""
echo "6. 证书指纹 (SHA256):"
openssl x509 -in "$CERT_PATH" -noout -fingerprint -sha256

echo ""
echo "=== 验证完成 ==="
echo ""
echo "如果所有检查都通过，请："
echo "1. 完全重启浏览器（关闭所有窗口）"
echo "2. 配置浏览器代理为 127.0.0.1:8080"
echo "3. 访问 HTTPS 网站进行测试"
echo ""
echo "如果仍然有证书错误，请查看文档："
echo "docs/CERTIFICATE_INSTALLATION_GUIDE.md"
