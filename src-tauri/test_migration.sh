#!/bin/bash

# 测试数据库迁移脚本

# 设置测试数据库路径
TEST_DB="/tmp/sentinel-ai-test.db"

# 删除旧测试数据库
rm -f "$TEST_DB"

echo "测试数据库迁移功能"
echo "===================="
echo ""
echo "1. 数据库路径: $TEST_DB"
echo "2. 将通过运行应用来触发迁移"
echo ""
echo "请运行应用并创建一个插件来测试迁移是否成功执行"
echo ""
echo "如果看到以下日志，说明迁移成功："
echo "  - Running database migrations..."
echo "  - Executing migration: 20251111_independent_plugin_registry.sql"
echo "  - Migration completed: 20251111_independent_plugin_registry.sql"
echo ""
echo "可以使用以下命令检查数据库结构:"
echo "  sqlite3 $TEST_DB '.schema plugin_registry'"
