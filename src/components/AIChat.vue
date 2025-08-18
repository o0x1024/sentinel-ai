<template>
  <div>
    <ChatWindow 
      ref="chatWindowRef"
      :is-open="isChatOpen"
      :is-sidebar-open="false"
      :is-standalone="isStandalone"
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
              <div v-if="message.isExecutingTool" class="tool-execution-indicator">
                <div class="tool-spinner"></div>
                <span class="tool-text">正在执行工具: {{ message.toolName }}</span>
              </div>
              
              <!-- 主要消息内容 -->
              <div v-if="getMainContent(message.content).trim()" class="prose prose-sm max-w-none mb-3" v-html="renderMarkdown(getMainContent(message.content))"></div>
              
              <!-- 工具调用状态显示 -->
              <div v-if="message.toolCalls && message.toolCalls.length > 0" class="tool-calls-section">
                <div v-for="(toolCall, index) in message.toolCalls" :key="index" class="tool-call-item mb-3">
                  <!-- 工具调用头部 -->
                  <div class="tool-call-header flex items-center gap-2 p-3 bg-base-100 rounded-lg border">
                    <div class="tool-icon">
                      <i class="fas fa-cog" :class="{
                        'text-warning animate-spin': toolCall.status === 'executing',
                        'text-success': toolCall.status === 'success',
                        'text-error': toolCall.status === 'error',
                        'text-info': toolCall.status === 'pending'
                      }"></i>
                    </div>
                    <div class="tool-info flex-1">
                      <div class="tool-name font-medium text-sm">{{ toolCall.name }}</div>
                      <div class="tool-status text-xs opacity-70">
                        <span v-if="toolCall.status === 'pending'">准备执行...</span>
                        <span v-else-if="toolCall.status === 'executing'">正在执行...</span>
                        <span v-else-if="toolCall.status === 'success'">执行成功</span>
                        <span v-else-if="toolCall.status === 'error'">执行失败</span>
                      </div>
                    </div>
                    <div class="tool-badge">
                      <div class="badge badge-sm" :class="{
                        'badge-warning': toolCall.status === 'executing' || toolCall.status === 'pending',
                        'badge-success': toolCall.status === 'success',
                        'badge-error': toolCall.status === 'error'
                      }">
                        {{ toolCall.status === 'success' ? '成功' : toolCall.status === 'error' ? '失败' : '执行中' }}
                      </div>
                    </div>
                  </div>
                  
                  <!-- 工具结果展示（风琴组件） -->
                  <div v-if="toolCall.result && (toolCall.status === 'success' || toolCall.status === 'error')" class="tool-result-accordion mt-2">
                    <div class="collapse collapse-arrow bg-base-200">
                      <input type="checkbox" class="collapse-checkbox" :id="`tool-result-${message.id}-${index}`" />
                      <label :for="`tool-result-${message.id}-${index}`" class="collapse-title text-sm font-medium cursor-pointer">
                        <i class="fas fa-file-alt mr-2"></i>
                        查看详细结果
                      </label>
                      <div class="collapse-content">
                        <div class="pt-2">
                          <div class="text-xs text-base-content/70 mb-2">工具执行结果:</div>
                          <div class="mockup-code text-xs max-h-60 overflow-y-auto">
                            <pre><code>{{ typeof toolCall.result === 'string' ? toolCall.result : JSON.stringify(toolCall.result, null, 2) }}</code></pre>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                  
                  <!-- 错误信息显示 -->
                  <div v-if="toolCall.error && toolCall.status === 'error'" class="tool-error mt-2 p-2 bg-error/10 border border-error/20 rounded text-sm text-error">
                    <i class="fas fa-exclamation-triangle mr-2"></i>
                    {{ toolCall.error }}
                  </div>
                </div>
              </div>
              
              <!-- 继续响应内容 -->
              <div v-if="message.continuedContent" class="continued-content mt-3 pt-3 border-t border-base-300">
                <div class="prose prose-sm max-w-none" v-html="renderMarkdown(message.continuedContent)"></div>
              </div>
              
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
              <button class="btn btn-sm btn-ghost btn-circle" @click="toggleConversationList" 
                     :title="t('aiChat.conversationList')">
                <i class="fas fa-list"></i>
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
            <button v-if="!isLoading" @click="sendMessage" :disabled="!inputMessage.trim()" 
                   class="btn btn-circle btn-primary">
              <i class="fas fa-paper-plane"></i>
            </button>
            <button v-else @click="stopMessage" 
                   class="btn btn-circle btn-error">
              <i class="fas fa-stop"></i>
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
import { dialog } from '@/composables/useDialog';

import ChatWindow from './ChatWindow.vue';
import RoleManagementModal from './RoleManagementModal.vue';

interface Props {
  isStandalone?: boolean;
}

const props = defineProps<Props>();

const { t } = useI18n();
marked.setOptions({ breaks: true, gfm: true });

// 定义emit
const emit = defineEmits(['close']);

// Props
const isStandalone = ref(props.isStandalone || false);

// Component State
const isChatOpen = ref(true); // 默认显示
const isRoleModalOpen = ref(false);
const isSidebarOpen = ref(false);
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

const toggleConversationList = () => {
  // 调用ChatWindow组件的对话抽屉切换功能
  chatWindowRef.value?.toggleConversationDrawer();
};

// ChatWindow组件引用
const chatWindowRef = ref<InstanceType<typeof ChatWindow> | null>(null);

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

// 工具调用接口定义
interface ToolCall {
  id: string;
  name: string;
  status: 'pending' | 'executing' | 'success' | 'error';
  result?: any;
  error?: string;
}

// Message State
const messages = ref<Array<{
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  isStreaming?: boolean;
  isExecutingTool?: boolean;
  toolName?: string;
  toolCalls?: ToolCall[];
  continuedContent?: string;
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
      conversation_id: conversation.id,
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
  const confirmed = await dialog.confirm(t('aiChat.confirmDeleteConversation'));
  if (confirmed) {
    await deleteConversation(conversation);
  }
};

const deleteConversation = async (conversation: {id: string, service_name: string}) => {
  try {
    await invoke('delete_ai_conversation', { 
      conversation_id: conversation.id,
      service_name: conversation.service_name
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

// 添加超时和重试机制的状态
const messageTimeout = ref<NodeJS.Timeout | null>(null);
const currentStreamingMessageId = ref<string | null>(null);
const retryCount = ref(0);
const maxRetries = 3;
const messageTimeoutMs = 30000; // 30秒超时

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
  currentStreamingMessageId.value = assistantMessageId;
  messages.value.push({
    id: assistantMessageId,
    role: 'assistant',
    content: '',
    timestamp: new Date(),
    isStreaming: true,
  });
  scrollToBottom();

  // 设置超时处理 - 工具执行时使用更长的超时时间
  const executingToolMessage = messages.value.find(m => m.isExecutingTool);
  const timeoutMs = executingToolMessage ? messageTimeoutMs * 3 : messageTimeoutMs;
  messageTimeout.value = setTimeout(() => {
    handleMessageTimeout();
  }, timeoutMs);

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
    clearMessageTimeout();
    handleSendError(error, assistantMessageId);
  }
};

// 处理发送错误
const handleSendError = (error: any, messageId: string) => {
  const lastMessage = messages.value.find(m => m.id === messageId);
  if (lastMessage) {
    lastMessage.content = `${t('aiChat.sendError')}: ${error}`;
    lastMessage.isStreaming = false;
  }
  isLoading.value = false;
  currentStreamingMessageId.value = null;
  retryCount.value = 0;
};

// 处理消息超时
const handleMessageTimeout = () => {
  console.warn('Message timeout occurred');
  
  // 检查是否有消息正在执行工具，如果是则不处理超时
  const executingToolMessage = messages.value.find(m => m.isExecutingTool);
  if (executingToolMessage) {
    console.log('Tool is executing, ignoring timeout');
    return;
  }
  
  if (currentStreamingMessageId.value) {
    const streamingMessage = messages.value.find(m => m.id === currentStreamingMessageId.value);
    if (streamingMessage && streamingMessage.isStreaming) {
      if (retryCount.value < maxRetries) {
        retryCount.value++;
        console.log(`Retrying message (attempt ${retryCount.value}/${maxRetries})`);
        // 重试发送
        setTimeout(() => {
          if (streamingMessage.isStreaming) {
            stopMessage();
          }
        }, 1000);
      } else {
        streamingMessage.content = streamingMessage.content || t('aiChat.messageTimeout');
        streamingMessage.isStreaming = false;
        streamingMessage.isExecutingTool = false;
        streamingMessage.toolName = undefined;
        isLoading.value = false;
        currentStreamingMessageId.value = null;
        retryCount.value = 0;
      }
    }
  }
};

// 清除超时定时器
const clearMessageTimeout = () => {
  if (messageTimeout.value) {
    clearTimeout(messageTimeout.value);
    messageTimeout.value = null;
  }
};

const stopMessage = async () => {
  if (!currentConversationId.value) return;
  
  console.log('Stopping AI stream...');
  
  // 清除超时定时器
  clearMessageTimeout();
  
  try {
    await invoke('stop_ai_stream', {
      request: {
        conversation_id: currentConversationId.value
      }
    });
    
    console.log('Stop request sent successfully');
  } catch (error) {
    console.error('Stop message failed:', error);
  } finally {
    // 无论是否成功，都要停止本地状态
    forceStopStreaming();
  }
};

// 强制停止流式状态
const forceStopStreaming = () => {
  console.log('Force stopping streaming state');
  
  // 检查是否有消息正在执行工具，如果是则不强制停止
  const executingToolMessage = messages.value.find(m => m.isExecutingTool);
  if (executingToolMessage) {
    console.log('Tool is executing, not forcing stop');
    return;
  }
  
  // 立即停止加载状态
  isLoading.value = false;
  
  // 停止当前流式消息
  const streamingMessage = messages.value.find(m => m.isStreaming);
  if (streamingMessage) {
    streamingMessage.isStreaming = false;
    streamingMessage.isExecutingTool = false;
    streamingMessage.toolName = undefined;
    if (!streamingMessage.content.trim()) {
      streamingMessage.content = t('aiChat.messageStopped');
    }
  }
  
  // 重置状态
  currentStreamingMessageId.value = null;
  retryCount.value = 0;
  clearMessageTimeout();
};

const handleStreamMessage = (streamData: any) => {
  if (streamData.conversation_id !== currentConversationId.value) return;

  // 清除超时定时器，因为收到了响应
  clearMessageTimeout();

  const assistantMessage = messages.value.find(m => 
    m.id === streamData.message_id || 
    (m.role === 'assistant' && m.isStreaming && m.id === currentStreamingMessageId.value)
  );
  
  if (assistantMessage) {
    // 智能处理内容分离：如果有工具调用，将工具调用后的内容放到continuedContent中
    if (assistantMessage.toolCalls && assistantMessage.toolCalls.length > 0) {
      // 检查是否有工具调用完成，如果有，则将新内容作为继续响应
      const hasCompletedTools = assistantMessage.toolCalls.some(tc => 
        tc.status === 'success' || tc.status === 'error'
      );
      
      if (hasCompletedTools) {
        // 将新的流式内容作为工具调用后的继续响应
        const cleanContent = getMainContent(streamData.content);
        if (cleanContent.trim()) {
          assistantMessage.continuedContent = cleanContent;
        }
      } else {
        // 工具还在执行中，更新主要内容
        assistantMessage.content = streamData.content;
      }
    } else {
      // 没有工具调用，正常更新内容
      assistantMessage.content = streamData.content;
    }
    
    if (streamData.is_complete) {
      console.log('Stream completed for message:', streamData.message_id);
      assistantMessage.isStreaming = false;
      assistantMessage.isExecutingTool = false;
      assistantMessage.toolName = undefined;
      isLoading.value = false;
      currentStreamingMessageId.value = null;
      retryCount.value = 0;
      
      // 如果主要内容为空但有工具调用，显示默认消息
      const mainContent = getMainContent(assistantMessage.content);
      if (!mainContent.trim() && assistantMessage.toolCalls && assistantMessage.toolCalls.length > 0) {
        // 如果有工具调用但没有主要内容，不显示任何默认消息
        // 让工具调用结果作为主要内容
      } else if (!mainContent.trim() && !assistantMessage.continuedContent) {
        // 只有在既没有主要内容也没有继续内容时才显示默认消息
        assistantMessage.content = assistantMessage.content || '消息处理完成';
      }
    } else {
      // 重新设置超时，因为还在流式传输中 - 工具执行时使用更长的超时时间
      const executingToolMessage = messages.value.find(m => m.isExecutingTool);
      const timeoutMs = executingToolMessage ? messageTimeoutMs * 3 : messageTimeoutMs;
      messageTimeout.value = setTimeout(() => {
        handleMessageTimeout();
      }, timeoutMs);
    }
  } else {
    console.warn('Could not find assistant message for stream data:', streamData);
  }
  
  scrollToBottom();
};

// 处理工具执行开始事件
const handleToolExecutionStart = (eventData: any) => {
  if (eventData.conversation_id !== currentConversationId.value) return;
  
  console.log('Tool execution started:', eventData.tool_name);
  
  // 清除超时定时器，因为工具正在执行
  clearMessageTimeout();
  
  // 找到最后一个助手消息
  const assistantMessages = messages.value.filter(m => m.role === 'assistant');
  const lastAssistantMessage = assistantMessages[assistantMessages.length - 1];
  
  if (lastAssistantMessage) {
    // 初始化toolCalls数组（如果不存在）
    if (!lastAssistantMessage.toolCalls) {
      lastAssistantMessage.toolCalls = [];
    }
    
    // 查找是否已存在该工具调用
    let toolCall = lastAssistantMessage.toolCalls.find(tc => tc.name === eventData.tool_name);
    
    if (!toolCall) {
      // 创建新的工具调用记录
      toolCall = {
        id: `tool-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        name: eventData.tool_name,
        status: 'executing'
      };
      lastAssistantMessage.toolCalls.push(toolCall);
    } else {
      // 更新现有工具调用状态
      toolCall.status = 'executing';
    }
    
    // 设置全局执行状态（保持向后兼容）
    lastAssistantMessage.isExecutingTool = true;
    lastAssistantMessage.toolName = eventData.tool_name;
  }
  
  scrollToBottom();
};

// 处理工具执行成功事件
const handleToolExecutionSuccess = (eventData: any) => {
  if (eventData.conversation_id !== currentConversationId.value) return;
  
  console.log('Tool execution succeeded:', eventData.tool_name);
  
  // 找到包含该工具调用的助手消息
  const assistantMessage = messages.value.find(m => 
    m.role === 'assistant' && m.toolCalls?.some(tc => tc.name === eventData.tool_name)
  );
  
  if (assistantMessage && assistantMessage.toolCalls) {
    // 找到对应的工具调用记录
    const toolCall = assistantMessage.toolCalls.find(tc => tc.name === eventData.tool_name);
    
    if (toolCall) {
      // 更新工具调用状态和结果
      toolCall.status = 'success';
      toolCall.result = eventData.result;
      
      // 检查是否所有工具调用都已完成
      const allCompleted = assistantMessage.toolCalls.every(tc => 
        tc.status === 'success' || tc.status === 'error'
      );
      
      if (allCompleted) {
        assistantMessage.isExecutingTool = false;
        assistantMessage.toolName = undefined;
      }
    }
  }
  
  scrollToBottom();
};

// 处理工具执行失败事件
const handleToolExecutionError = (eventData: any) => {
  if (eventData.conversation_id !== currentConversationId.value) return;
  
  console.error('Tool execution failed:', eventData.tool_name, eventData.error);
  
  // 找到包含该工具调用的助手消息
  const assistantMessage = messages.value.find(m => 
    m.role === 'assistant' && m.toolCalls?.some(tc => tc.name === eventData.tool_name)
  );
  
  if (assistantMessage && assistantMessage.toolCalls) {
    // 找到对应的工具调用记录
    const toolCall = assistantMessage.toolCalls.find(tc => tc.name === eventData.tool_name);
    
    if (toolCall) {
      // 更新工具调用状态和错误信息
      toolCall.status = 'error';
      toolCall.error = eventData.error;
      
      // 检查是否所有工具调用都已完成
      const allCompleted = assistantMessage.toolCalls.every(tc => 
        tc.status === 'success' || tc.status === 'error'
      );
      
      if (allCompleted) {
        assistantMessage.isExecutingTool = false;
        assistantMessage.toolName = undefined;
      }
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

// 解析消息内容，分离主要内容和工具调用结果
const getMainContent = (content: string): string => {
  // 移除工具执行相关的内容（保持向后兼容）
  const mainContent = content
    .replace(/🔧 正在执行工具: [^\n]+\.\.\./g, '')
    .replace(/✅ 工具执行完成: [^\n]+/g, '')
    .replace(/❌ 工具执行失败: [^\n]+/g, '')
    .replace(/📋 工具执行结果预览:[\s\S]*?```[\s\S]*?```/g, '')
    .replace(/错误信息: [^\n]+/g, '')
    .replace(/\n\s*\n/g, '\n') // 移除多余的空行
    .trim();
  
  return mainContent;
};

// 提取工具调用结果
const getToolResults = (content: string): Array<{name: string, status: string, result: string}> => {
  const results: Array<{name: string, status: string, result: string}> = [];
  
  // 匹配工具执行完成的模式
  const successMatches = content.match(/✅ 工具执行完成: ([^\n]+)/g);
  if (successMatches) {
    successMatches.forEach(match => {
      const toolName = match.replace('✅ 工具执行完成: ', '');
      
      // 查找对应的结果预览
      const resultPattern = new RegExp(`📋 工具执行结果预览:[\\s\\S]*?\`\`\`([\\s\\S]*?)\`\`\``);
      const resultMatch = content.match(resultPattern);
      const result = resultMatch ? resultMatch[1].trim() : '执行成功，无详细结果';
      
      results.push({
        name: toolName,
        status: 'success',
        result: result
      });
    });
  }
  
  // 匹配工具执行失败的模式
  const errorMatches = content.match(/❌ 工具执行失败: ([^\n]+)/g);
  if (errorMatches) {
    errorMatches.forEach(match => {
      const toolName = match.replace('❌ 工具执行失败: ', '');
      
      // 查找错误信息
      const errorPattern = /错误信息: ([^\n]+)/;
      const errorMatch = content.match(errorPattern);
      const errorMsg = errorMatch ? errorMatch[1] : '未知错误';
      
      results.push({
        name: toolName,
        status: 'error',
        result: errorMsg
      });
    });
  }
  
  // 匹配正在执行的工具
  const runningMatches = content.match(/🔧 正在执行工具: ([^\n]+)\.\.\./g);
  if (runningMatches) {
    runningMatches.forEach(match => {
      const toolName = match.replace('🔧 正在执行工具: ', '').replace('...', '');
      
      // 检查是否已经有完成或失败的记录
      const existingResult = results.find(r => r.name === toolName);
      if (!existingResult) {
        results.push({
          name: toolName,
          status: 'running',
          result: '工具正在执行中...'
        });
      }
    });
  }
  
  return results;
};

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

  // 监听AI配置更新事件，重新加载模型列表
  const unlistenConfigUpdate = await listen('ai_config_updated', async () => {
    console.log('AI configuration updated, reloading models...');
    await loadAvailableModels();
  });

  // 监听AI流式响应停止事件
  const unlistenStreamStopped = await listen('ai_stream_stopped', (event) => {
    const conversationId = event.payload as string;
    if (conversationId === currentConversationId.value) {
      console.log('AI stream stopped for current conversation');
      forceStopStreaming();
    }
  });

  // 监听AI流式响应错误事件
  const unlistenStreamError = await listen('ai_stream_error', (event) => {
    const errorData = event.payload as { conversation_id: string; error: string };
    if (errorData.conversation_id === currentConversationId.value) {
      console.error('AI stream error:', errorData.error);
      
      const streamingMessage = messages.value.find(m => m.isStreaming);
      if (streamingMessage) {
        streamingMessage.content = streamingMessage.content || `${t('aiChat.streamError')}: ${errorData.error}`;
        streamingMessage.isStreaming = false;
      }
      
      forceStopStreaming();
    }
  });

  // 监听工具执行事件
  const unlistenToolStart = await listen('ai_tool_execution_start', (event) => {
    handleToolExecutionStart(event.payload);
  });

  const unlistenToolSuccess = await listen('ai_tool_execution_success', (event) => {
    handleToolExecutionSuccess(event.payload);
  });

  const unlistenToolError = await listen('ai_tool_execution_error', (event) => {
    handleToolExecutionError(event.payload);
  });

  onUnmounted(() => {
    // 清理事件监听器
    unlistenStream();
    unlistenConfigUpdate();
    unlistenStreamStopped();
    unlistenStreamError();
    unlistenToolStart();
    unlistenToolSuccess();
    unlistenToolError();
    
    // 清理定时器和状态
    clearMessageTimeout();
    forceStopStreaming();
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
  min-height: 0; /* 允许flex子元素收缩 */
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
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background-color: #f8f9fa;
  min-height: 0; /* 允许flex子元素收缩 */
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
    background-color: #bbdae8;
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

.tool-execution-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  padding: 8px;
  background: rgba(59, 130, 246, 0.1);
  border-radius: 6px;
  border-left: 3px solid #3b82f6;
}

.tool-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid #e5e7eb;
  border-top: 2px solid #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

.tool-text {
  font-size: 0.875rem;
  color: #374151;
  font-weight: 500;
}

.tool-results-section {
  border-top: 1px solid rgba(0, 0, 0, 0.1);
  padding-top: 12px;
}

.tool-results-section .collapse {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
}

.tool-results-section .collapse-title {
  padding: 12px 16px;
  min-height: auto;
}

.tool-results-section .collapse-content {
  padding: 0 16px 12px 16px;
}

.tool-results-section .mockup-code {
  margin: 0;
  border-radius: 6px;
  max-height: 200px;
  overflow-y: auto;
}

.tool-results-section .mockup-code pre {
  margin: 0;
  padding: 12px;
  white-space: pre-wrap;
  word-break: break-word;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

/* Input Area */
.input-area {
  padding: 1rem;
  border-top: 1px solid #e0e0e0;
  background-color: #ffffff;
  flex-shrink: 0; /* 防止输入区域被压缩 */
  max-height: 40vh; /* 限制最大高度，防止超出屏幕 */
  overflow-y: auto; /* 如果内容过多，允许滚动 */
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

/* 响应式布局优化 */
@media (max-width: 768px) {
  .input-area {
    padding: 0.75rem;
    max-height: 50vh;
  }
  
  .input-actions-top {
    margin-bottom: 0.5rem;
    gap: 0.25rem;
  }
  
  .model-selector {
    max-width: 180px;
  }
  
  .messages-area {
    padding: 1rem;
  }
}

@media (max-width: 480px) {
  .input-area {
    padding: 0.5rem;
    max-height: 60vh;
  }
  
  .input-actions-top {
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  
  .model-selector {
    max-width: 150px;
    order: -1;
    flex-basis: 100%;
  }
  
  .messages-area {
    padding: 0.75rem;
  }
  
  .message-bubble {
    max-width: 85%;
    padding: 0.5rem 0.75rem;
  }
}
</style>