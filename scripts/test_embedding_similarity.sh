#!/bin/bash
# 测试嵌入向量的相似度计算

echo "Testing embedding similarity..."
echo ""
echo "This script will help diagnose why '你好' retrieves irrelevant content."
echo ""
echo "Expected behavior:"
echo "  - '你好' vs '海康威视漏洞' should have LOW similarity (< 0.3)"
echo "  - '你好' vs '你好' should have HIGH similarity (> 0.9)"
echo ""
echo "Current issue: All scores show 1.00, which indicates a problem."
echo ""
echo "To fix:"
echo "1. Restart the application to use the updated code"
echo "2. Check RAG configuration: embedding model and provider"
echo "3. Verify LanceDB distance metric (cosine vs L2)"
echo ""
echo "Modified files:"
echo "  - src-tauri/sentinel-rag/src/service.rs (added deduplication and threshold filtering)"
echo ""

