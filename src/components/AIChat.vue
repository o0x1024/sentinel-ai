<template>
  <div>
    <ChatWindow 
      :is-open="isChatOpen"
      :is-sidebar-open="isSidebarOpen"
      @open-roles="isRoleModalOpen = true"
      @new-conversation="startNewConversation"
      @close="handleClose"
      @minimize="minimizeChatWindow"
      @open="openChatWindow"
    >
      <template #sidebar>
        <div class="sidebar-content">
          <div class="sidebar-header">
            <h3 class="font-semibold">{{ t('aiChat.conversationHistory') }}</h3>
          </div>
          <div class="conversation-list">
            <div v-for="convo in conversations" :key="convo.id" 
                 class="conversation-item"
                 :class="{ 'active': convo.id === currentConversationId }"
                 @click="loadConversation(convo)">
              <div class="convo-title">{{ convo.title || 'Untitled' }}</div>
              <button @click.stop="confirmDeleteConversation(convo)" class="delete-convo-btn">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
        </div>
      </template>
      <div class="chat-main-content">
        <!-- Message Area -->
        <div ref="messagesContainer" class="messages-area">
          <div v-if="messages.length === 0" class="empty-state">
            <i class="fas fa-comments text-4xl mb-4 opacity-50"></i>
            <p class="font-semibold">AI Assistant</p>
            <p class="text-sm opacity-70 mt-1">Start a conversation or select a role to begin.</p>
          </div>
          <div v-for="message in messages" :key="message.id" class="message-item" :class="`message-${message.role}`">
            <div class="avatar">
              <div class="w-8 h-8 rounded-full flex items-center justify-center" 
                   :class="message.role === 'user' ? 'bg-primary/10' : 'bg-secondary/10'">
                <i :class="[message.role === 'user' ? 'fas fa-user text-primary' : 'fas fa-robot text-secondary']"></i>
              </div>
            </div>
            <div class="message-bubble">
              <div v-if="message.isStreaming" class="streaming-indicator"></div>
              <div class="prose prose-sm max-w-none" v-html="renderMarkdown(message.content)"></div>
              <div class="timestamp">{{ formatTime(message.timestamp) }}</div>
            </div>
          </div>
        </div>

        <!-- Input Area -->
        <div class="input-area">
          <!-- Role Display and Actions -->
          <div v-if="currentRole" class="current-role-display">
            <div class="badge badge-primary gap-1">
              <i class="fas fa-user-tag"></i>
              <span>{{ currentRole.title }}</span>
              <button @click="clearRole" class="btn btn-xs btn-ghost btn-circle">
                <i class="fas fa-times"></i>
              </button>
            </div>
          </div>
          
          <div class="input-actions-top">
            <!-- 模型选择下拉框 -->
            <div class="model-selector">
              <select v-model="selectedModel" class="select select-sm select-bordered w-full max-w-xs">
                <optgroup v-for="(models, provider) in groupedModels" :key="provider" :label="provider">
                  <option v-for="model in models" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </optgroup>
              </select>
            </div>
            
            <div class="flex gap-1">
              <button class="btn btn-sm btn-ghost btn-circle" @click="toggleSidebar" 
                     :title="t('aiChat.toggleSidebar')">
                <i class="fas fa-bars"></i>
              </button>
              <button class="btn btn-sm btn-ghost btn-circle" @click="isRoleModalOpen = true" 
                     :title="currentRole ? currentRole.title : t('aiChat.selectRole')">
                <i class="fas fa-users-cog"></i>
              </button>
              <button class="btn btn-sm btn-ghost btn-circle" @click="startNewConversation" 
                     :title="t('aiChat.newConversation')">
                <i class="fas fa-plus"></i>
              </button>
              <button class="btn btn-sm btn-ghost btn-circle" @click="clearHistory" 
                     :title="t('aiChat.clearConversation')">
                <i class="fas fa-trash"></i>
              </button>
              <button class="btn btn-sm btn-ghost btn-circle" :title="t('aiChat.exportConversation')">
                <i class="fas fa-file-export"></i>
              </button>
            </div>
          </div>
          
          <div class="input-wrapper">
            <textarea
              v-model="inputMessage"
              @keyup.enter="sendMessage"
              :disabled="isLoading"
              :placeholder="t('aiChat.inputPlaceholder')"
              class="textarea textarea-bordered w-full resize-none"
              rows="3"
            ></textarea>
            <button @click="sendMessage" :disabled="isLoading || !inputMessage.trim()" 
                   class="btn btn-circle btn-primary">
              <i v-if="isLoading" class="fas fa-spinner fa-spin"></i>
              <i v-else class="fas fa-paper-plane"></i>
            </button>
          </div>
        </div>
        
        <!-- Role Management Modal -->
        <RoleManagementModal 
          :is-open="isRoleModalOpen"
          @close="isRoleModalOpen = false"
          @select-role="handleRoleSelected"
        />
      </div>
    </ChatWindow>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { marked } from 'marked';
import { useI18n } from 'vue-i18n';

import ChatWindow from './ChatWindow.vue';
import RoleManagementModal from './RoleManagementModal.vue';

const { t } = useI18n();
marked.setOptions({ breaks: true, gfm: true });

// 定义emit
const emit = defineEmits(['close']);

// Component State
const isChatOpen = ref(true); // 默认显示
const isRoleModalOpen = ref(false);
const isSidebarOpen = ref(true);
const isLoading = ref(false);
const inputMessage = ref('');
const messagesContainer = ref<HTMLElement | null>(null);
const currentConversationId = ref<string | null>(null);
const conversations = ref<Array<{ id: string, title: string, service_name: string }>>([]);

// 处理关闭事件
const handleClose = () => {
  emit('close');
};

// 聊天窗口控制
const openChatWindow = () => {
  isChatOpen.value = true;
};

const closeChatWindow = () => {
  isChatOpen.value = false;
};

const minimizeChatWindow = () => {
  isChatOpen.value = false;
};

const toggleSidebar = () => {
  isSidebarOpen.value = !isSidebarOpen.value;
};

// 模型选择相关
const selectedModel = ref('');
const groupedModels = ref<Record<string, { id: string; name: string }[]>>({});

// 加载可用模型
const loadAvailableModels = async () => {
  try {
    const models: Array<{ provider: string; name: string; }> = await invoke('get_ai_chat_models');
    if (Array.isArray(models) && models.length > 0) {
      const grouped: Record<string, { id: string; name: string }[]> = {};
      for (const model of models) {
        if (!grouped[model.provider]) {
          grouped[model.provider] = [];
        }
        grouped[model.provider].push({
          id: `${model.provider}/${model.name}`,
          name: model.name
        });
      }
      groupedModels.value = grouped;

      // 设置默认选中的模型
      const defaultModelInfo: { provider: string, name: string } | null = await invoke('get_default_ai_model', { modelType: 'chat' });
      if (defaultModelInfo) {
          selectedModel.value = `${defaultModelInfo.provider}/${defaultModelInfo.name}`;
      } else if (Object.keys(grouped).length > 0) {
        const firstProvider = Object.keys(grouped)[0];
        if (grouped[firstProvider].length > 0) {
          selectedModel.value = grouped[firstProvider][0].id;
        }
      }
    }
  } catch (error) {
    console.error('Failed to load AI models:', error);
  }
};

// Role State
const currentRole = ref<{ id: string; title: string; description: string; prompt: string; is_system?: boolean; } | null>(null);

// Message State
const messages = ref<Array<{
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  isStreaming?: boolean;
}>>([]);

const handleRoleSelected = (role: { id: string; title: string; description: string; prompt: string; is_system?: boolean; }) => {
  currentRole.value = role;
  isRoleModalOpen.value = false;
  startNewConversation(); // Start a new conversation when a role is selected
};

const clearRole = () => {
  currentRole.value = null;
  startNewConversation(); // Also start a new conversation when role is cleared
};

const startNewConversation = async () => {
    messages.value = [];
    currentConversationId.value = null; // 重置当前对话ID
    if (currentRole.value) {
        messages.value.push({
            id: 'role-info',
            role: 'assistant',
            content: t('aiChat.roleConversationStart', { role: currentRole.value.title }),
            timestamp: new Date()
        });
    } else {
         messages.value.push({
            id: 'welcome',
            role: 'assistant',
            content: t('aiChat.welcome'),
            timestamp: new Date()
        });
    }

    try {
        const [provider, model] = selectedModel.value.split('/');
        if (!provider) {
            console.warn(t('aiChat.noModelSelected'));
            // 不再抛出错误，允许在没有模型的情况下开始一个"空"对话
            return;
        }
        const conversationId = await invoke('create_ai_conversation', {
            request: { 
                title: currentRole.value?.title || t('aiChat.newConversationTitle'),
                service_name: provider
            }
        });
        currentConversationId.value = conversationId as string;
        await loadConversations(); // 创建新对话后刷新列表
    } catch (error) {
        console.error("Failed to create new conversation:", error);
        messages.value.push({
            id: 'error',
            role: 'assistant',
            content: `${t('aiChat.errorCreatingConversation')}: ${error}`,
            timestamp: new Date()
        });
    }
};

const loadConversations = async () => {
  try {
    const convos: any[] = await invoke('get_ai_conversations');
    conversations.value = convos.map(c => ({ id: c.id, title: c.title, service_name: c.service_name }));
  } catch (error) {
    console.error("Failed to load conversations:", error);
  }
};

const loadConversation = async (conversation: {id: string, service_name: string}) => {
  if (currentConversationId.value === conversation.id) return;

  try {
    currentConversationId.value = conversation.id;
    const history: any[] = await invoke('get_ai_conversation_history', { 
      conversationId: conversation.id,
      service_name: conversation.service_name
    });

    messages.value = history.map(msg => ({
      id: msg.id,
      role: msg.role,
      content: msg.content,
      timestamp: new Date(msg.timestamp)
    }));
    scrollToBottom();
  } catch (error) {
    console.error(`Failed to load conversation ${conversation.id}:`, error);
  }
};

const confirmDeleteConversation = async (conversation: {id: string, service_name: string}) => {
  const confirmed = confirm(t('aiChat.confirmDeleteConversation'));
  if (confirmed) {
    await deleteConversation(conversation);
  }
};

const deleteConversation = async (conversation: {id: string, service_name: string}) => {
  try {
    await invoke('delete_ai_conversation', { 
      conversationId: conversation.id,
      serviceName: conversation.service_name
    });
    
    // 从列表中移除
    conversations.value = conversations.value.filter(c => c.id !== conversation.id);
    
    // 如果删除的是当前对话，则开始新对话
    if (currentConversationId.value === conversation.id) {
      await startNewConversation();
    }
  } catch (error) {
    console.error(`Failed to delete conversation ${conversation.id}:`, error);
  }
};

const sendMessage = async () => {
  if (!inputMessage.value.trim() || isLoading.value) return;

  // If there's no active conversation, start one
  if (!currentConversationId.value) {
    await startNewConversation();
    if (!currentConversationId.value) {
      // 如果仍然没有对话ID，说明创建失败了
      return;
    }
  }

  const userMessageContent = inputMessage.value;
  messages.value.push({
    id: Date.now().toString(),
    role: 'user',
    content: userMessageContent,
    timestamp: new Date(),
  });
  inputMessage.value = '';
  isLoading.value = true;

  const assistantMessageId = (Date.now() + 1).toString();
  messages.value.push({
    id: assistantMessageId,
    role: 'assistant',
    content: '',
    timestamp: new Date(),
    isStreaming: true,
  });
  scrollToBottom();

  try {
    const [provider, model] = selectedModel.value.split('/');
    if (!provider || !model) {
      throw new Error(t('aiChat.invalidModelSelected'));
    }
    
    // 确保将角色提示信息传递给AI服务
    const systemPrompt = currentRole.value?.prompt;
    
    console.log("Sending message with role:", currentRole.value?.title || "None");
    if (systemPrompt) {
      console.log("System prompt:", systemPrompt.substring(0, 50) + "...");
    }
    
    await invoke('send_ai_message_stream', {
      request: {
        conversation_id: currentConversationId.value,
        message: userMessageContent,
        system_prompt: systemPrompt, // 传递角色提示
        provider: provider,
        model: model
      }
    });
  } catch (error) {
    console.error('Send message failed:', error);
    const lastMessage = messages.value.find(m => m.id === assistantMessageId);
    if (lastMessage) {
      lastMessage.content = `Error: ${error}`;
      lastMessage.isStreaming = false;
    }
  } finally {
    isLoading.value = false;
  }
};

const handleStreamMessage = (streamData: any) => {
  if (streamData.conversation_id !== currentConversationId.value) return;

  const assistantMessage = messages.value.find(m => m.id === streamData.message_id || (m.role === 'assistant' && m.isStreaming));
  
  if (assistantMessage) {
    assistantMessage.content = streamData.content;
    if (streamData.is_complete) {
      assistantMessage.isStreaming = false;
    }
  }
  scrollToBottom();
};

const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }
  });
};

const renderMarkdown = (content: string) => marked(content);
const formatTime = (timestamp: Date) => timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

const clearHistory = () => {
    startNewConversation();
};

onMounted(async () => {
  await loadAvailableModels();
  await loadConversations();
  if (conversations.value.length > 0) {
    await loadConversation(conversations.value[0]);
  } else {
    await startNewConversation();
  }
  
  const unlistenStream = await listen('ai_stream_message', (event) => {
    handleStreamMessage(event.payload);
  });

  onUnmounted(() => {
    unlistenStream();
  });
});
</script>

<style scoped>
/* Main content layout */
.chat-main-content {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #ffffff;
  position: relative;
  overflow: hidden; /* 确保模态框不会溢出 */
}

.sidebar-content {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sidebar-header {
  padding-bottom: 0.5rem;
  border-bottom: 1px solid #e0e0e0;
  margin-bottom: 0.5rem;
}

.conversation-list {
  flex-grow: 1;
  overflow-y: auto;
}

.conversation-item {
  padding: 0.75rem;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.conversation-item:hover {
  background-color: #e9ecef;
}

.conversation-item.active {
  background-color: #4f46e5;
  color: white;
}

.convo-title {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.delete-convo-btn {
  background: none;
  border: none;
  color: #adb5bd;
  cursor: pointer;
  visibility: hidden;
  opacity: 0;
  transition: opacity 0.2s, visibility 0.2s;
}

.conversation-item:hover .delete-convo-btn,
.conversation-item.active .delete-convo-btn {
  visibility: visible;
  opacity: 1;
  color: inherit;
}

/* New styles for modal container */
.modal-container {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000; /* Ensure it's on top of chat content */
}

/* Messages Area */
.messages-area {
  flex-grow: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background-color: #f8f9fa;
}
.empty-state {
    text-align: center;
    margin-top: 8rem;
    color: #6c757d;
}

.message-item {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}
.message-assistant {
  justify-content: flex-start;
}
.message-user {
  justify-content: flex-end;
}
.message-user .message-bubble {
    background-color: #4f46e5;
    color: white;
}
.message-user .avatar {
    order: 2;
}
.avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background-color: #e9ecef;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}
.message-bubble {
    max-width: 75%;
    padding: 0.75rem 1rem;
    border-radius: 12px;
    background-color: #e9ecef;
    position: relative;
}
.timestamp {
    font-size: 0.75rem;
    color: #adb5bd;
    margin-top: 0.5rem;
}
.message-user .timestamp {
    color: rgba(255,255,255,0.7);
}
.streaming-indicator {
    width: 8px;
    height: 8px;
    background-color: #4f46e5;
    border-radius: 50%;
    display: inline-block;
    animation: bounce 1.4s infinite ease-in-out both;
}
@keyframes bounce {
  0%, 80%, 100% { transform: scale(0); }
  40% { transform: scale(1.0); }
}

/* Input Area */
.input-area {
  padding: 1rem;
  border-top: 1px solid #e0e0e0;
  background-color: #ffffff;
}
.input-actions-top {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
}
.action-btn {
    background: none;
    border: 1px solid #d1d5db;
    padding: 0.4rem;
    border-radius: 6px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
}
.action-btn:hover {
    background-color: #f3f4f6;
    border-color: #4f46e5;
    color: #4f46e5;
}
.input-wrapper {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  background-color: #f8f9fa;
  border-radius: 8px;
  padding: 0.5rem;
  border: 1px solid #e0e0e0;
}
.message-input {
  flex-grow: 1;
  border: none;
  background: none;
  padding: 0.5rem;
  font-size: 1rem;
  resize: none;
  outline: none;
  min-height: 80px;
  max-height: 150px;
}
.send-button {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background-color: #4f46e5;
  color: white;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
  transition: background-color 0.2s;
}
.send-button:disabled {
  background-color: #ccc;
  cursor: not-allowed;
}

/* 模型选择器样式 */
.model-selector {
  flex: 1;
  max-width: 220px;
}
.model-select {
  width: 100%;
  padding: 0.4rem 0.6rem;
  border-radius: 6px;
  border: 1px solid #d1d5db;
  background-color: white;
  font-size: 0.875rem;
  outline: none;
  cursor: pointer;
}
.model-select:focus {
  border-color: #4f46e5;
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.2);
}

.current-role-display {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.25rem 0.75rem;
  background-color: #eef2ff;
  color: #4f46e5;
  border-radius: 6px;
  margin-bottom: 0.5rem;
  font-size: 0.875rem;
}

.clear-role-button {
  background: none;
  border: none;
  font-size: 1.25rem;
  font-weight: bold;
  cursor: pointer;
  color: #4f46e5;
  padding: 0 0.5rem;
  line-height: 1;
}
</style> 