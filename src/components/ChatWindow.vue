<template>
  <div class="chat-window-container">
    <div 
      v-if="isOpen" 
      class="chat-window"
      :style="windowStyle"
      ref="chatWindowRef"
    >
      <!-- Header -->
      <div class="chat-header" @mousedown="startDrag">
        <div class="flex-1">
          <h1 class="text-lg font-semibold">{{ t('aiChat.title') }}</h1>
          <p class="text-xs opacity-80">{{ t('aiChat.subtitle') }}</p>
        </div>
        <div class="header-actions">
          <button class="btn btn-sm btn-ghost" @click="$emit('open-roles')">
            <i class="fas fa-users-cog mr-1"></i> {{ t('roles.roleManagement') }}
          </button>
          <button class="btn btn-sm btn-ghost">
            <i class="fas fa-history mr-1"></i> {{ t('aiChat.conversationHistory') }}
          </button>
          <button class="btn btn-sm btn-primary" @click="$emit('new-conversation')">
            <i class="fas fa-plus mr-1"></i> {{ t('aiChat.newConversation') }}
          </button>
          <button class="btn btn-sm btn-ghost btn-square" @click="$emit('minimize')">
            <i class="fas fa-window-minimize"></i>
          </button>
          <button class="btn btn-sm btn-ghost btn-square hover:bg-error hover:text-white" @click="$emit('close')">
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>

      <!-- Main Content -->
      <div class="chat-body" :class="{ 'sidebar-open': isSidebarOpen }">
        <!-- Sidebar -->
        <div class="chat-sidebar">
          <slot name="sidebar"></slot>
        </div>

        <!-- Main chat content -->
        <div class="chat-main">
          <slot></slot>
        </div>
      </div>

      <!-- Resize Handles -->
      <div class="resize-handles">
        <div class="resize-handle resize-e" @mousedown="startResize('e', $event)"></div>
        <div class="resize-handle resize-s" @mousedown="startResize('s', $event)"></div>
        <div class="resize-handle resize-se" @mousedown="startResize('se', $event)"></div>
        <div v-if="!isLeftSide" class="resize-handle resize-w" @mousedown="startResize('w', $event)"></div>
        <div v-if="!isLeftSide" class="resize-handle resize-sw" @mousedown="startResize('sw', $event)"></div>
      </div>
    </div>
    
    <!-- 最小化状态下的浮动按钮，点击可重新打开聊天窗口 -->
    <button 
      v-if="!isOpen" 
      class="chat-toggle-btn"
      @click="$emit('open')"
    >
      <i class="fas fa-comments"></i>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import useDraggableResizable from '@/composables/useDraggableResizable';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  isOpen: boolean,
  isSidebarOpen: boolean,
}>();

defineEmits(['open-roles', 'new-conversation', 'close', 'minimize', 'open']);

const chatWindowRef = ref<HTMLElement | null>(null);

const { 
  windowStyle, 
  isLeftSide,
  startDrag, 
  startResize 
} = useDraggableResizable(chatWindowRef);

</script>

<style scoped>
/* Styles for the main chat window and header */
.chat-window {
  position: fixed;
  z-index: 100;
  display: flex;
  flex-direction: column;
  background-color: var(--fallback-b1,oklch(var(--b1)/1));
  border-radius: 1rem;
  box-shadow: 0 10px 25px rgba(0,0,0,0.1);
  overflow: hidden;
  border: 1px solid var(--fallback-b3,oklch(var(--b3)/1));
}

.chat-header {
  background-color: var(--fallback-primary,oklch(var(--p)/1));
  color: var(--fallback-primary-content,oklch(var(--pc)/1));
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  cursor: move;
  user-select: none;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

/* 最小化状态下的浮动按钮 */
.chat-toggle-btn {
  position: fixed;
  bottom: 20px;
  right: 20px;
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background-color: var(--fallback-primary,oklch(var(--p)/1));
  color: var(--fallback-primary-content,oklch(var(--pc)/1));
  font-size: 1.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  cursor: pointer;
  z-index: 100;
  transition: transform 0.3s, box-shadow 0.3s;
}

.chat-toggle-btn:hover {
  transform: scale(1.05);
  box-shadow: 0 6px 16px rgba(0,0,0,0.2);
}

.chat-body {
  flex-grow: 1;
  display: flex;
  overflow: hidden;
}

.chat-sidebar {
  width: 260px;
  background-color: #f8f9fa;
  border-right: 1px solid #e0e0e0;
  padding: 1rem;
  overflow-y: auto;
  transition: width 0.3s ease, margin-left 0.3s ease;
  margin-left: -260px;
}

.chat-body.sidebar-open .chat-sidebar {
  margin-left: 0;
}

.chat-main {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: #ffffff;
  transition: width 0.3s ease;
}

/* Resize Handles Styling */
.resize-handles {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  pointer-events: none;
}
.resize-handle {
  position: absolute;
  pointer-events: auto;
  z-index: 10;
}
.resize-e { top: 0; right: -3px; width: 6px; height: 100%; cursor: ew-resize; }
.resize-w { top: 0; left: -3px; width: 6px; height: 100%; cursor: ew-resize; }
.resize-s { bottom: -3px; left: 0; width: 100%; height: 6px; cursor: ns-resize; }
.resize-se { bottom: -3px; right: -3px; width: 12px; height: 12px; cursor: nw-resize; }
.resize-sw { bottom: -3px; left: -3px; width: 12px; height: 12px; cursor: ne-resize; }
.resize-handle:hover {
  background-color: var(--fallback-primary,oklch(var(--p)/0.3));
}
</style> 