<template>
  <div class="border-t border-base-300/50 bg-base-100 flex-shrink-0">
    <!-- Configuration toolbar -->
    <div class="bg-base-200/50 border-b border-base-300 p-2 mx-2 mt-1">
      <div class="flex items-center justify-between gap-2 text-xs">
        <!-- Left: Conversation management -->
        <div class="flex items-center gap-2">
          <div class="flex items-center gap-1">
            <button @click="$emit('create-conversation')" class="btn btn-xs btn-ghost" :disabled="isLoadingConversations" title="新建会话">
              <i class="fas fa-plus"></i>
            </button>
            <button @click="$emit('update:show-conversations-list', true)" class="btn btn-xs btn-ghost" :disabled="conversations.length === 0" title="会话列表">
              <i class="fas fa-list"></i>
            </button>
            <button v-if="currentConversationId" @click="$emit('clear-conversation')" class="btn btn-xs btn-ghost text-warning" title="清空">
              <i class="fas fa-broom"></i>
            </button>
          </div>
          <!-- Current conversation info -->
          <div v-if="currentConversationId" class="flex items-center gap-1 text-xs text-base-content/60">
            <span>会话 ({{ conversations.length > 0 ? conversations.find(c => c.id === currentConversationId)?.title || '无标题' : currentConversationId.slice(0, 8) }})</span>
          </div>
        </div>

        <!-- Right: Configuration settings -->
        <div class="flex items-center gap-2">
          <!-- Debug toggle -->
          <button @click="$emit('toggle-debug')" 
                  class="btn btn-xs btn-ghost gap-1"
                  :class="{ 'text-warning': showDebugInfo }"
                  title="切换调试信息">
            <i class="fas fa-bug text-xs"></i>
            <span class="hidden sm:inline">调试</span>
          </button>
          
          <!-- Architecture selector -->
          <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-xs btn-ghost gap-1">
              <i class="fas fa-layer-group text-xs"></i>
              <span class="hidden sm:inline">{{ selectedArchitecture }}</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 max-h-96 overflow-y-auto">
              <li v-for="arch in availableArchitectures" :key="arch.id">
                <a @click="$emit('select-architecture', arch)" 
                   class="hover:bg-primary hover:text-primary-content py-2 px-3"
                   :class="{ 'active bg-primary text-primary-content': selectedArchitecture === arch.name }">
                  <div class="flex flex-col items-start w-full">
                    <div class="flex items-center justify-between w-full mb-1">
                      <span class="font-medium text-sm">{{ arch.name }}</span>
                      <div class="badge badge-xs" :class="getArchBadgeClass(arch.status)">
                        {{ getArchBadgeText(arch.status) }}
                      </div>
                    </div>
                    <span class="text-xs opacity-70 text-left" v-if="arch.description">
                      {{ arch.description }}
                    </span>
                  </div>
                </a>
              </li>
              <li v-if="availableArchitectures.length === 0" class="opacity-60">
                <span class="text-sm px-3 py-2">暂无可用架构</span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>

    <!-- Conversation list drawer -->
    <div class="fixed inset-0 z-50 mt-16" v-if="showConversationsList">
      <div class="absolute inset-0 bg-black bg-opacity-50" @click="$emit('update:show-conversations-list', false)"></div>
      <div class="absolute right-0 top-0 h-[calc(100vh-4rem)] w-80 bg-base-200 shadow-xl transform transition-transform duration-300 ease-in-out">
        <div class="h-full p-4 overflow-y-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-lg flex items-center gap-2">
              <i class="fas fa-comments text-primary"></i>
              会话历史
            </h3>
            <button @click="$emit('update:show-conversations-list', false)" class="btn btn-ghost btn-sm btn-circle">
              <i class="fas fa-times"></i>
            </button>
          </div>
          
          <!-- Operations -->
          <div class="flex gap-2 mb-4">
            <button @click="$emit('load-conversations')" class="btn btn-outline btn-sm flex-1" :disabled="isLoadingConversations">
              <i class="fas fa-sync" :class="{ 'animate-spin': isLoadingConversations }"></i>
              刷新
            </button>
            <button @click="$emit('create-conversation')" class="btn btn-primary btn-sm" :disabled="isLoadingConversations">
              <i class="fas fa-plus"></i>
              新建
            </button>
          </div>
          
          <!-- Conversation list -->
          <div v-if="conversations.length === 0" class="text-center text-base-content/60 py-8">
            <i class="fas fa-comments text-4xl opacity-30 mb-4"></i>
            <p>暂无会话记录</p>
          </div>
          <div v-else class="space-y-3 max-h-[calc(100vh-200px)] overflow-y-auto">
            <div 
              v-for="conv in conversations" 
              :key="conv.id"
              class="card bg-base-100 shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
              :class="{ 'ring-2 ring-primary': conv.id === currentConversationId }"
              @click="$emit('switch-conversation', conv.id)"
            >
              <div class="card-body p-3">
                <div class="flex items-start justify-between">
                  <div class="flex-1 min-w-0">
                    <h4 class="font-medium text-sm truncate mb-1">
                      {{ conv.title || '无标题会话' }}
                    </h4>
                    <div class="text-xs text-base-content/60 space-y-1">
                      <div class="flex items-center gap-2">
                        <i class="fas fa-clock"></i>
                        {{ new Date(conv.created_at).toLocaleString() }}
                      </div>
                      <div class="flex items-center gap-2">
                        <i class="fas fa-comment-dots"></i>
                        {{ conv.total_messages }} 条消息
                      </div>
                    </div>
                  </div>
                  <div class="flex flex-col gap-1">
                    <button 
                      v-if="conv.id === currentConversationId" 
                      class="badge badge-primary badge-xs"
                    >
                      当前
                    </button>
                    <button 
                      @click.stop="$emit('delete-conversation', conv.id)" 
                      class="btn btn-ghost btn-xs text-error hover:bg-error hover:text-error-content"
                      title="删除会话"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Input box -->
    <div class="flex gap-2 mx-2 mb-2">
      <div class="flex-1 relative">
        <textarea 
          :value="inputMessage"
          @input="$emit('update:input-message', ($event.target as HTMLTextAreaElement).value)"
          @keydown.enter.ctrl="$emit('send-message')"
          :disabled="isLoading"
          placeholder="描述您需要执行的安全任务... (Ctrl+Enter发送)"
          class="textarea textarea-bordered w-full resize-none border border-base-300 focus:border-primary transition-colors"
          rows="2"
        ></textarea>
      </div>
      <div class="flex items-end">
        <button 
          v-if="!isLoading" 
          @click="$emit('send-message')" 
          :disabled="!inputMessage.trim()" 
          class="btn btn-primary btn-sm"
          :class="{ 'btn-disabled': !inputMessage.trim() }"
        >
          <i class="fas fa-paper-plane"></i>
        </button>
        <button 
          v-else 
          @click="$emit('stop-execution')" 
          class="btn btn-error btn-sm"
        >
          <i class="fas fa-stop"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useMessageUtils } from '../composables/useMessageUtils'

const props = defineProps<{
  inputMessage: string
  isLoading: boolean
  showDebugInfo: boolean
  selectedArchitecture: string
  availableArchitectures: any[]
  conversations: any[]
  currentConversationId: string | null
  isLoadingConversations: boolean
  showConversationsList: boolean
}>()

defineEmits([
  'update:input-message',
  'send-message',
  'stop-execution',
  'toggle-debug',
  'select-architecture',
  'create-conversation',
  'load-conversations',
  'switch-conversation',
  'delete-conversation',
  'clear-conversation',
  'update:show-conversations-list'
])

const { getArchBadgeClass, getArchBadgeText } = useMessageUtils()
</script>