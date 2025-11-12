#!/bin/bash

# 测试被动扫描数据库初始化脚本

echo "=== 被动扫描数据库初始化测试 ==="
echo ""

# 1. 删除旧数据库
echo "1. 清理旧数据库..."
rm -f ~/.sentinel-ai/passive_scan.db
echo "   ✓ 旧数据库已删除"
echo ""

# 2. 使用 sqlite3 测试数据库创建
DB_PATH="$HOME/.sentinel-ai/passive_scan.db"

echo "2. 测试数据库文件路径: $DB_PATH"
echo ""

echo "3. 等待应用启动并初始化数据库..."
echo "   请在浏览器中打开被动扫描页面，这将触发数据库初始化"
echo ""

# 等待数据库文件创建
for i in {1..30}; do
    if [ -f "$DB_PATH" ]; then
        echo "   ✓ 数据库文件已创建！"
        break
    fi
    sleep 1
    echo -n "."
done

echo ""
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "   ✗ 数据库文件未创建，请检查应用日志"
    exit 1
fi

# 4. 验证表结构
echo "4. 验证表结构..."
TABLES=$(sqlite3 "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;" 2>/dev/null)

if [ -z "$TABLES" ]; then
    echo "   ✗ 未找到任何表"
    exit 1
fi

echo "   已创建的表："
echo "$TABLES" | while read -r table; do
    echo "     - $table"
done

# 5. 检查必需的表
REQUIRED_TABLES=("passive_vulnerabilities" "passive_evidence" "passive_plugin_registry" "passive_scan_sessions" "passive_dedupe_index")

echo ""
echo "5. 检查必需的表..."
ALL_OK=true

for table in "${REQUIRED_TABLES[@]}"; do
    if echo "$TABLES" | grep -q "^$table$"; then
        echo "   ✓ $table"
    else
        echo "   ✗ $table (缺失)"
        ALL_OK=false
    fi
done

echo ""
if [ "$ALL_OK" = true ]; then
    echo "=== ✓ 数据库初始化成功！ ==="
else
    echo "=== ✗ 数据库初始化失败，部分表缺失 ==="
    exit 1
fi
