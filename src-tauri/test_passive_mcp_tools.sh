#!/bin/bash
# 测试被动扫描 MCP 工具集成

echo "=== Testing Passive Scan MCP Tools ==="
echo ""

# 1. 启动应用（后台运行，10秒后自动停止）
echo "1. Starting Tauri application..."
timeout 10s cargo run --release 2>&1 | tee /tmp/sentinel_passive_test.log &
APP_PID=$!

# 等待应用启动
sleep 5

# 2. 检查日志中是否有工具注册成功的信息
echo ""
echo "2. Checking passive tools registration..."
if grep -q "Passive scan tools registered successfully" /tmp/sentinel_passive_test.log; then
    echo "✅ Passive scan tools registered"
else
    echo "❌ Failed to register passive scan tools"
    echo "Last 20 log lines:"
    tail -20 /tmp/sentinel_passive_test.log
fi

# 3. 检查全局工具系统是否包含 passive 工具
echo ""
echo "3. Checking for passive tools in global tool system..."
if grep -E "(list_findings|plugin_analysis|passive\.builtin)" /tmp/sentinel_passive_test.log; then
    echo "✅ Found passive tools in logs"
else
    echo "⚠️  No passive tools found in logs (might be normal if not enumerated)"
fi

# 4. 检查是否有错误
echo ""
echo "4. Checking for errors..."
if grep -i "error.*passive" /tmp/sentinel_passive_test.log; then
    echo "❌ Found errors related to passive scan"
else
    echo "✅ No passive scan errors found"
fi

# 等待 timeout 结束
wait $APP_PID
echo ""
echo "=== Test Complete ==="
echo "Full log saved to: /tmp/sentinel_passive_test.log"
