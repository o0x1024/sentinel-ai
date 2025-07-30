#!/bin/bash
echo "正在获取AI聊天记录..."
./sentinel-ai invoke print_ai_conversations > ai_chats.txt
echo "聊天记录已保存到 ai_chats.txt" 