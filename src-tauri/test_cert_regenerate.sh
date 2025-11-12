#!/bin/bash

echo "=== 证书重新生成和信任测试 ==="
echo ""

CA_DIR="/Users/a1024/code/ai/sentinel-ai/src-tauri/AppData/passive-scan/ca"
CA_CERT="$CA_DIR/root-ca.pem"

echo "1. 检查当前证书..."
if [ -f "$CA_CERT" ]; then
    echo "   当前证书存在: $CA_CERT"
    echo "   当前证书指纹:"
    shasum -a 256 "$CA_CERT"
else
    echo "   证书不存在"
fi

echo ""
echo "2. 删除旧证书..."
rm -f "$CA_DIR/root-ca.pem" "$CA_DIR/root-ca.key"
echo "   ✓ 已删除"

echo ""
echo "3. 检查系统钥匙串中的旧证书..."
OLD_CERTS=$(security find-certificate -c "Sentinel AI Passive Scan CA" -a 2>/dev/null | grep -c "keychain")
if [ "$OLD_CERTS" -gt 0 ]; then
    echo "   找到 $OLD_CERTS 个旧证书"
    echo "   需要手动删除: 打开 '钥匙串访问' -> 搜索 'Sentinel AI Passive Scan CA' -> 删除"
else
    echo "   ✓ 没有找到旧证书"
fi

echo ""
echo "4. 运行应用以生成新证书..."
echo "   请在应用中:"
echo "   a) 启动被动扫描"
echo "   b) 下载 CA 证书"
echo "   c) 双击安装并设为'始终信任'"
echo ""
echo "5. 新证书将位于: $CA_CERT"
echo ""
echo "6. 安装后检查指纹:"
echo "   - 文件: shasum -a 256 '$CA_CERT'"
echo "   - 钥匙串: security find-certificate -c 'Sentinel AI Passive Scan CA' -p | openssl x509 -noout -fingerprint -sha256"
echo ""
echo "7. 测试 HTTPS 抓包:"
echo "   curl -x http://127.0.0.1:8080 https://www.baidu.com -v"
echo ""

