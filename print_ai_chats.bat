@echo off
echo 正在获取AI聊天记录...
D:/code/sentinel-ai/src-tauri/target/debug/sentinel-ai.exe invoke print_ai_conversations > ai_chats.txt
echo 聊天记录已保存到 ai_chats.txt
pause 