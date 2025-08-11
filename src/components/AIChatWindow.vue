<template>
  <div class="ai-chat-window h-screen w-full bg-base-100 flex flex-col overflow-hidden">
    <!-- 简洁的标题栏 -->
    <div class="navbar bg-base-200 shadow-sm border-b border-base-300 min-h-12 h-12" data-tauri-drag-region>
      <div class="navbar-start">
        <div class="flex items-center gap-2 px-2">
          <div class="avatar placeholder">
            <div class="bg-primary text-primary-content rounded-full w-6">
              <i class="fas fa-robot text-xs"></i>
            </div>
          </div>
          <h1 class="text-sm font-semibold">{{ t('aiChat.title', 'AI 助手') }}</h1>
        </div>
      </div>
      
      <div class="navbar-end">
        <div class="flex items-center gap-1 pr-2">
          <!-- 最小化按钮 -->
          <button @click="minimizeWindow" class="btn btn-ghost btn-xs btn-circle" :title="t('common.minimize', '最小化')">
            <i class="fas fa-minus text-xs"></i>
          </button>
          
          <!-- 关闭按钮 -->
          <button @click="closeWindow" class="btn btn-ghost btn-xs btn-circle hover:btn-error" :title="t('common.close', '关闭')">
            <i class="fas fa-times text-xs"></i>
          </button>
        </div>
      </div>
    </div>

    <!-- 聊天区域 - 完全填充剩余空间 -->
    <div class="flex-1 overflow-hidden">
      <AIChat :is-standalone="true" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import AIChat from './AIChat.vue';

const { t } = useI18n();

// 监听窗口事件
let unlistenShowAiChat: (() => void) | null = null;

onMounted(async () => {
  // 监听显示AI聊天的事件
  unlistenShowAiChat = await listen('show-ai-chat-only', (event) => {
    console.log('Received show-ai-chat-only event:', event.payload);
  });
  
  // 自动吸附到主窗口右侧
  try {
    await invoke('attach_ai_window_to_main', { side: 'right' });
    console.log('AI window attached to main window right side');
  } catch (error) {
    console.error('Failed to attach AI window:', error);
  }
  
  // 启动窗口位置同步监听器
  try {
    await invoke('start_window_sync');
    console.log('Window sync started successfully');
  } catch (error) {
    console.error('Failed to start window sync:', error);
  }
});

onUnmounted(() => {
  if (unlistenShowAiChat) {
    unlistenShowAiChat();
  }
});

// 最小化窗口
const minimizeWindow = async () => {
  try {
    await invoke('set_window_size', {
      label: 'ai-chat',
      width: 60,
      height: 60
    });
  } catch (error) {
    console.error('Failed to minimize window:', error);
  }
};

// 关闭窗口
const closeWindow = async () => {
  try {
    await invoke('close_ai_chat_window');
  } catch (error) {
    console.error('Failed to close window:', error);
  }
};
</script>

<style scoped>
.ai-chat-window {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* 确保标题栏可拖拽 */
[data-tauri-drag-region] {
  -webkit-app-region: drag;
}

[data-tauri-drag-region] button {
  -webkit-app-region: no-drag;
}

/* 自定义滚动条样式 */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}

/* 侧边栏动画效果 */
.transition-colors {
  transition: background-color 0.2s ease;
}

/* 状态指示器动画 */
.bg-success {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>