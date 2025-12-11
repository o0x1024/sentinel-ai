<template>
  <div class="conversation-list h-full flex flex-col bg-base-200">
    <!-- Header -->
    <div class="p-3 border-b border-base-300 flex items-center justify-between">
      <h3 class="font-semibold text-sm flex items-center gap-2">
        <i class="fas fa-comments text-primary"></i>
        会话列表
      </h3>
      <div class="flex items-center gap-2">
        <button 
          @click="createNewConversation" 
          class="btn btn-xs btn-primary gap-1"
          title="新建会话"
        >
          <i class="fas fa-plus"></i>
          新建
        </button>
        <button 
          @click="$emit('close')"
          class="btn btn-xs btn-ghost"
          title="关闭"
        >
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Search -->
    <div class="p-2">
      <input 
        v-model="searchQuery"
        type="text" 
        placeholder="搜索会话..." 
        class="input input-sm input-bordered w-full"
      />
    </div>

    <!-- Conversation List -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="isLoading" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
      </div>

      <div v-else-if="filteredConversations.length === 0" class="text-center py-8 text-base-content/50 text-sm">
        <i class="fas fa-inbox text-3xl mb-2 opacity-50"></i>
        <p>{{ searchQuery ? '未找到匹配的会话' : '暂无会话' }}</p>
      </div>

      <div v-else class="space-y-1 p-2">
        <div
          v-for="conv in filteredConversations"
          :key="conv.id"
          :class="[
            'conversation-item group relative p-3 rounded-lg cursor-pointer transition-all',
            currentConversationId === conv.id 
              ? 'bg-primary/10 border-l-2 border-primary' 
              : 'hover:bg-base-300/50'
          ]"
          @click="selectConversation(conv)"
        >
          <div class="flex items-start justify-between gap-2">
            <div class="flex-1 min-w-0">
              <h4 class="font-medium text-sm truncate" :title="conv.title || '未命名会话'">
                {{ conv.title || '未命名会话' }}
              </h4>
              <p class="text-xs text-base-content/60 mt-1">
                {{ formatDate(conv.updated_at) }}
              </p>
              <div class="flex items-center gap-2 mt-1 text-xs text-base-content/50">
                <span v-if="conv.total_messages > 0">
                  <i class="fas fa-message"></i> {{ conv.total_messages }}
                </span>
                <span v-if="conv.model_name">
                  <i class="fas fa-robot"></i> {{ conv.model_name }}
                </span>
              </div>
            </div>

            <!-- Actions -->
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                @click.stop="renameConversation(conv)"
                class="btn btn-xs btn-ghost"
                title="重命名"
              >
                <i class="fas fa-edit"></i>
              </button>
              <button
                @click.stop="deleteConversation(conv)"
                class="btn btn-xs btn-ghost text-error"
                title="删除"
              >
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface Conversation {
  id: string
  title: string | null
  model_name: string
  total_messages: number
  created_at: string
  updated_at: string
}

const props = defineProps<{
  currentConversationId?: string | null
}>()

const emit = defineEmits<{
  (e: 'select', conversationId: string): void
  (e: 'create', conversationId: string): void
  (e: 'close'): void
}>()

const conversations = ref<Conversation[]>([])
const isLoading = ref(false)
const searchQuery = ref('')

const filteredConversations = computed(() => {
  if (!searchQuery.value) return conversations.value
  
  const query = searchQuery.value.toLowerCase()
  return conversations.value.filter(conv => 
    (conv.title || '').toLowerCase().includes(query) ||
    conv.model_name.toLowerCase().includes(query)
  )
})

const loadConversations = async () => {
  isLoading.value = true
  try {
    const result = await invoke<Conversation[]>('get_ai_conversations')
    conversations.value = result.sort((a, b) => 
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    )
  } catch (error) {
    console.error('Failed to load conversations:', error)
  } finally {
    isLoading.value = false
  }
}

const createNewConversation = async () => {
  try {
    const conversationId = await invoke<string>('create_ai_conversation', {
      request: {
        title: `新会话 ${new Date().toLocaleString()}`,
        service_name: 'default'
      }
    })
    await loadConversations()
    emit('create', conversationId)
  } catch (error) {
    console.error('Failed to create conversation:', error)
  }
}

const selectConversation = (conv: Conversation) => {
  emit('select', conv.id)
}

const renameConversation = async (conv: Conversation) => {
  const newTitle = prompt('请输入新的会话名称:', conv.title || '')
  if (newTitle && newTitle.trim()) {
    try {
      await invoke('update_ai_conversation_title', {
        conversationId: conv.id,
        title: newTitle.trim(),
        serviceName: 'default'
      })
      await loadConversations()
    } catch (error) {
      console.error('Failed to rename conversation:', error)
    }
  }
}

const deleteConversation = async (conv: Conversation) => {
    try {
      await invoke('delete_ai_conversation', {
        conversationId: conv.id,
        serviceName: 'default'
      })
      await loadConversations()
    } catch (error) {
      console.error('Failed to delete conversation:', error)
    }
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(diff / 86400000)
  
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  if (days < 7) return `${days}天前`
  
  return date.toLocaleDateString()
}

onMounted(() => {
  loadConversations()
})

defineExpose({
  loadConversations
})
</script>

<style scoped>
.conversation-item {
  transition: all 0.2s ease;
}

.conversation-item:hover {
  transform: translateX(2px);
}
</style>

