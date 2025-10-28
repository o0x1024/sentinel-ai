// 简单的测试脚本来触发LLM调用
const { invoke } = require('@tauri-apps/api/tauri');

async function testLLMLogging() {
    try {
        console.log('Testing LLM logging functionality...');
        
        // 创建一个测试对话
        const conversationId = await invoke('create_ai_conversation', {
            title: 'Test LLM Logging'
        });
        
        console.log('Created conversation:', conversationId);
        
        // 发送一个简单的消息来触发LLM调用
        const response = await invoke('send_ai_message_stream', {
            conversationId: conversationId,
            message: 'Hello, this is a test message to check LLM logging.',
            stream: false
        });
        
        console.log('LLM Response:', response);
        console.log('Test completed. Check logs/llm-http-requests-*.log for logging output.');
        
    } catch (error) {
        console.error('Test failed:', error);
    }
}

testLLMLogging();
