<template>
  <div class="chat-window-container">
    <div 
      v-if="isOpen" 
      class="chat-window"
      :style="windowStyle"

      ref="chatWindowRef"
    >
      <!-- Header -->
    

      <!-- Main Content -->
      <div 
        class="chat-body" 
        :class="{ 'small-screen': isSmallScreen, 'drawer-open': isConversationDrawerOpen }"
        :style="contentAreaStyle"
      >
        <!-- Main chat content -->
        <div 
          class="chat-main"
          :style="{
            fontSize: responsiveFontSize,
            padding: '0'
          }"
        >
          <slot></slot>
        </div>
      </div>

      <!-- Conversation Drawer -->
      <div 
        v-if="isConversationDrawerOpen" 
        class="conversation-drawer"
        :class="{ 'drawer-left': drawerPosition === 'left', 'drawer-right': drawerPosition === 'right' }"
        :style="{
          width: `${responsiveDrawerWidth}px`,
          fontSize: responsiveFontSize
        }"
      >
        <div class="drawer-header">
          <h3 class="font-semibold">{{ t('aiChat.conversationHistory') }}</h3>
          <div class="drawer-controls">
            <button 
              class="btn btn-xs btn-ghost" 
              @click="toggleDrawerPosition"
              :title="t('aiChat.switchSide')"
            >
              <i class="fas fa-exchange-alt"></i>
            </button>
            <button 
              class="btn btn-xs btn-ghost" 
              @click="closeConversationDrawer"
              :title="t('common.close')"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>
        <div 
          class="drawer-content"
          :style="{
            padding: responsivePadding
          }"
        >
          <slot name="sidebar"></slot>
        </div>
        <!-- Drawer resize handle -->
        <div 
          class="drawer-resize-handle"
          :class="drawerPosition === 'left' ? 'resize-right' : 'resize-left'"
          @mousedown="startDrawerResize"
        ></div>
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
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import useDraggableResizable from '@/composables/useDraggableResizable';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  isOpen: boolean,
  isSidebarOpen: boolean,
  isStandalone?: boolean,
}>();

defineEmits(['open-roles', 'new-conversation', 'close', 'minimize', 'open']);

const chatWindowRef = ref<HTMLElement | null>(null);

// 响应式断点
const windowWidth = ref(window.innerWidth);
const windowHeight = ref(window.innerHeight);

// 响应式布局状态
const isSmallScreen = computed(() => windowWidth.value < 768);
const isMediumScreen = computed(() => windowWidth.value >= 768 && windowWidth.value < 1024);
const isLargeScreen = computed(() => windowWidth.value >= 1024);

// 对话抽屉状态
const isConversationDrawerOpen = ref(false);
const drawerPosition = ref<'left' | 'right'>('left');
const drawerWidth = ref(280);

// 响应式抽屉宽度
const responsiveDrawerWidth = computed(() => {
  const maxWidth = maxDrawerWidth.value;
  const currentWidth = drawerWidth.value;
  
  if (isSmallScreen.value) {
    // 小屏幕时，抽屉宽度不超过窗口宽度的80%
    return Math.min(currentWidth, Math.min(windowWidth.value * 0.8, chatWindowWidth.value * 0.8));
  }
  
  if (isMediumScreen.value) {
    // 中等屏幕时，限制最大宽度
    return Math.min(currentWidth, maxWidth);
  }
  
  // 大屏幕时，确保不超过窗口宽度的40%
  return Math.min(currentWidth, maxWidth);
});

// 窗口尺寸状态
const chatWindowWidth = ref(800);
const chatWindowHeight = ref(600);

// 响应式最小尺寸
const minWindowWidth = computed(() => isSmallScreen.value ? 300 : 400);
const minWindowHeight = computed(() => isSmallScreen.value ? 250 : 300);

// 响应式初始尺寸
const initialWidth = computed(() => {
  if (isSmallScreen.value) return Math.min(windowWidth.value * 0.9, 400);
  if (isMediumScreen.value) return Math.min(windowWidth.value * 0.8, 600);
  return 800;
});

const initialHeight = computed(() => {
  if (isSmallScreen.value) return Math.min(windowHeight.value * 0.8, 500);
  if (isMediumScreen.value) return Math.min(windowHeight.value * 0.7, 550);
  return 600;
});

const { 
  windowStyle: baseWindowStyle, 
  isLeftSide,
  startDrag: baseDrag, 
  startResize 
} = useDraggableResizable(chatWindowRef, {
  initialWidth: initialWidth.value,
  initialHeight: initialHeight.value,
  minWidth: minWindowWidth.value,
  minHeight: minWindowHeight.value
});

// 监听窗口样式变化，更新内部尺寸状态
watch(baseWindowStyle, (newStyle) => {
  if (newStyle.width) {
    chatWindowWidth.value = parseInt(newStyle.width.toString().replace('px', ''));
  }
  if (newStyle.height) {
    chatWindowHeight.value = parseInt(newStyle.height.toString().replace('px', ''));
  }
}, { deep: true, immediate: true });

// 响应式窗口样式
const windowStyle = computed(() => {
  const base = baseWindowStyle.value;
  
  // 在小屏幕上确保窗口不超出边界
  if (isSmallScreen.value) {
    return {
      ...base,
      maxWidth: '95vw',
      maxHeight: '90vh'
    };
  }
  
  return base;
});

// 响应式内容区域样式
const contentAreaStyle = computed(() => {
  const baseHeight = chatWindowHeight.value;
  const headerHeight = isSmallScreen.value ? 48 : 56; // 头部高度
  const availableHeight = baseHeight - headerHeight;
  
  return {
    height: `${availableHeight}px`,
    maxHeight: `${availableHeight}px`
  };
});

// 响应式字体大小
const responsiveFontSize = computed(() => {
  if (chatWindowWidth.value < 400) return '12px';
  if (chatWindowWidth.value < 600) return '13px';
  return '14px';
});

// 响应式间距
const responsivePadding = computed(() => {
  if (chatWindowWidth.value < 400) return '8px';
  if (chatWindowWidth.value < 600) return '12px';
  return '16px';
});

// 响应式抽屉宽度限制
const maxDrawerWidth = computed(() => {
  return Math.min(chatWindowWidth.value * 0.4, 320);
});

// 监听窗口尺寸变化，自动调整抽屉宽度
watch([chatWindowWidth, maxDrawerWidth], ([newWindowWidth, newMaxWidth]) => {
  if (drawerWidth.value > newMaxWidth) {
    drawerWidth.value = newMaxWidth;
  }
});

// 对话抽屉控制
const toggleConversationDrawer = () => {
  isConversationDrawerOpen.value = !isConversationDrawerOpen.value;
  
  // 根据窗口位置自动设置抽屉位置
  if (isConversationDrawerOpen.value) {
    drawerPosition.value = isLeftSide.value ? 'right' : 'left';
  }
};

const closeConversationDrawer = () => {
  isConversationDrawerOpen.value = false;
};

const toggleDrawerPosition = () => {
  drawerPosition.value = drawerPosition.value === 'left' ? 'right' : 'left';
};

// 抽屉调整大小
const startDrawerResize = (e: MouseEvent) => {
  e.preventDefault();
  const startX = e.clientX;
  const startWidth = drawerWidth.value;
  
  const onMouseMove = (event: MouseEvent) => {
    const deltaX = event.clientX - startX;
    const newWidth = drawerPosition.value === 'left' 
      ? startWidth + deltaX 
      : startWidth - deltaX;
    
    drawerWidth.value = Math.max(200, Math.min(400, newWidth));
  };
  
  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  };
  
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
};

// 抽屉位置计算
const drawerStyle = computed(() => {
  const width = `${responsiveDrawerWidth.value}px`;
  
  // 响应式抽屉定位
  const windowRect = chatWindowRef.value?.getBoundingClientRect();
  if (!windowRect) return { width };
  
  const isLeft = drawerPosition.value === 'left';
  
  // 在小屏幕上，抽屉覆盖在窗口内部
  if (isSmallScreen.value) {
    return {
      width,
      height: '100%',
      position: 'absolute' as const,
      top: '0',
      [drawerPosition.value]: '0',
      zIndex: 10
    };
  }
  
  // 在大屏幕上，抽屉在窗口外侧
  const left = isLeft 
    ? Math.max(0, windowRect.left - responsiveDrawerWidth.value)
    : Math.min(window.innerWidth - responsiveDrawerWidth.value, windowRect.right);
  
  return {
    width,
    height: `${windowRect.height}px`,
    position: 'fixed' as const,
    left: `${left}px`,
    top: `${windowRect.top}px`,
    zIndex: 1000
  };
});

// 拖拽功能
const startDrag = (e: MouseEvent) => {
  baseDrag(e);
};

// 监听窗口大小变化
const handleResize = () => {
  windowWidth.value = window.innerWidth;
  windowHeight.value = window.innerHeight;
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
  // 初始化响应式状态
  handleResize();
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});

// 监听屏幕尺寸变化，自动调整抽屉行为
watch([isSmallScreen, isConversationDrawerOpen], ([small, drawerOpen]) => {
  if (small && drawerOpen) {
    // 小屏幕时，抽屉打开时禁用主内容交互
    document.body.style.overflow = 'hidden';
  } else {
    document.body.style.overflow = '';
  }
}, { immediate: true });

// 暴露方法给父组件
defineExpose({
  toggleConversationDrawer,
  closeConversationDrawer
});

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
  transition: all 0.3s ease;
  position: relative;
}

.chat-body.small-screen {
  /* 小屏幕样式通过内联样式动态设置 */
}

.chat-body.drawer-open {
  margin-right: 0;
}

@media (max-width: 768px) {
  .chat-body.drawer-open {
    filter: blur(2px);
    pointer-events: none;
  }
}

.chat-main {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: var(--fallback-b1,oklch(var(--b1)/1));
  transition: all 0.3s ease;
  /* 字体大小和内边距通过内联样式动态设置 */
}

/* 对话抽屉样式 */
.conversation-drawer {
  position: fixed;
  top: 0;
  bottom: 0;
  /* 宽度通过内联样式动态设置 */
  background-color: var(--fallback-b1,oklch(var(--b1)/1));
  border: 1px solid var(--fallback-b3,oklch(var(--b3)/1));
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  z-index: 200;
  display: flex;
  flex-direction: column;
  transition: transform 0.3s ease;
  /* 字体大小通过内联样式动态设置 */
}

.conversation-drawer.drawer-left {
  left: 0;
  border-radius: 0 1rem 1rem 0;
  border-left: none;
}

.conversation-drawer.drawer-right {
  right: 0;
  border-radius: 1rem 0 0 1rem;
  border-right: none;
}

.drawer-header {
  padding: 1rem;
  border-bottom: 1px solid var(--fallback-b3,oklch(var(--b3)/1));
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: var(--fallback-b2,oklch(var(--b2)/1));
}

.drawer-controls {
  display: flex;
  gap: 0.5rem;
}

.drawer-content {
  flex: 1;
  overflow-y: auto;
  /* 内边距通过内联样式动态设置 */
  transition: all 0.3s ease;
}

.drawer-resize-handle {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: ew-resize;
  background-color: transparent;
  transition: background-color 0.2s;
}

.drawer-resize-handle:hover {
  background-color: var(--fallback-primary,oklch(var(--p)/0.3));
}

.drawer-resize-handle.resize-right {
  right: -2px;
}

.drawer-resize-handle.resize-left {
  left: -2px;
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

/* Input Area */
.input-area {
  padding: 16px;
  border-top: 1px solid var(--fallback-b3,oklch(var(--b3)/1));
  background: var(--fallback-b1,oklch(var(--b1)/1));
}

.input-container {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.input-container.compact {
  gap: 6px;
}

.input-container textarea {
  flex: 1;
  padding: 12px;
  border: 1px solid var(--fallback-b3,oklch(var(--b3)/1));
  border-radius: 8px;
  background: var(--fallback-b1,oklch(var(--b1)/1));
  color: var(--fallback-bc,oklch(var(--bc)/1));
  resize: none;
  font-family: inherit;
  font-size: 14px;
  line-height: 1.4;
  min-height: 44px;
}

@media (max-width: 768px) {
  .input-container textarea {
    padding: 10px;
    font-size: 16px; /* 防止iOS缩放 */
    min-height: 40px;
  }
  
  .input-area {
    padding: 12px;
  }
}

.input-container textarea:focus {
  outline: none;
  border-color: var(--fallback-primary,oklch(var(--p)/1));
}

.send-button {
  padding: 12px;
  background: var(--fallback-primary,oklch(var(--p)/1));
  color: var(--fallback-primary-content,oklch(var(--pc)/1));
  border: none;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.2s;
  min-width: 44px;
  min-height: 44px;
}

@media (max-width: 768px) {
  .send-button {
    padding: 10px;
    min-width: 40px;
    min-height: 40px;
  }
}

.send-button:hover {
  opacity: 0.9;
}

/* Message Container */
.message-container {
  padding: 8px 16px;
  margin: 4px 0;
  border-radius: 8px;
  max-width: 80%;
  word-wrap: break-word;
}

@media (max-width: 768px) {
  .message-container {
    padding: 6px 12px;
    margin: 3px 0;
    max-width: 90%;
    font-size: 14px;
  }
}

.message-container.user {
  background: var(--fallback-primary,oklch(var(--p)/1));
  color: var(--fallback-primary-content,oklch(var(--pc)/1));
  margin-left: auto;
}

.message-container.assistant {
  background: var(--fallback-b2,oklch(var(--b2)/1));
  color: var(--fallback-bc,oklch(var(--bc)/1));
  margin-right: auto;
}

/* Chat Window Responsive */
@media (max-width: 768px) {
  .chat-window {
    border-radius: 0.5rem;
    margin: 8px;
  }
  
  .chat-header {
    padding: 0.5rem 0.75rem;
  }
  
  .header-actions .btn {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }
}

@media (max-width: 480px) {
  .chat-window {
    margin: 4px;
    border-radius: 0.25rem;
  }
  
  .conversation-drawer {
    width: 100% !important;
    height: 100% !important;
    border-radius: 0;
    border: none;
  }
  
  .drawer-header {
    padding: 0.75rem;
  }
  
  .drawer-content {
    padding: 0.75rem;
  }
}
</style>