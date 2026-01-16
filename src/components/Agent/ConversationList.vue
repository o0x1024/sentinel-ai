<template>
  <div class="conversation-list h-full flex flex-col bg-base-200">
    <!-- Header -->
    <div class="p-3 border-b border-base-300 flex items-center justify-between">
      <h3 class="font-semibold text-sm flex items-center gap-2">
        <i class="fas fa-comments text-primary"></i>
        {{ t('agent.conversationList') }}
      </h3>
      <div class="flex items-center gap-2">
        <button 
          @click="createNewConversation" 
          class="btn btn-xs btn-primary gap-1"
          :title="t('agent.newConversation')"
        >
          <i class="fas fa-plus"></i>
          {{ t('agent.newConversation') }}
        </button>
        <button 
          @click="$emit('close')"
          class="btn btn-xs btn-ghost"
          :title="t('agent.close')"
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
        :placeholder="t('agent.searchConversations')" 
        class="input input-sm input-bordered w-full"
        @input="handleSearch"
      />
    </div>

    <!-- Conversation List -->
    <div 
      ref="scrollContainer"
      class="flex-1 overflow-y-auto"
      @scroll="handleScroll"
    >
      <div v-if="isLoading && conversations.length === 0" class="flex items-center justify-center py-8">
        <span class="loading loading-spinner loading-md text-primary"></span>
      </div>

      <div v-else-if="conversations.length === 0" class="text-center py-8 text-base-content/50 text-sm">
        <i class="fas fa-inbox text-3xl mb-2 opacity-50"></i>
        <p>{{ searchQuery ? t('agent.noMatchingConversations') : t('agent.noConversations') }}</p>
      </div>

      <div v-else class="space-y-1 p-2">
        <div
          v-for="conv in conversations"
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
              <h4 class="font-medium text-sm truncate" :title="conv.title || t('agent.unnamedConversation')">
                {{ conv.title || t('agent.unnamedConversation') }}
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
                :title="t('agent.rename')"
              >
                <i class="fas fa-edit"></i>
              </button>
              <button
                @click.stop="deleteConversation(conv)"
                class="btn btn-xs btn-ghost text-error"
                :title="t('agent.delete')"
              >
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
        </div>

        <!-- Loading more indicator -->
        <div v-if="isLoadingMore" class="flex items-center justify-center py-4">
          <span class="loading loading-spinner loading-sm text-primary"></span>
        </div>

        <!-- End of list indicator -->
        <div v-else-if="hasMore" class="text-center py-2 text-xs text-base-content/50">
          {{ t('agent.scrollToLoadMore') }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

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

const { t } = useI18n()

const conversations = ref<Conversation[]>([])
const isLoading = ref(false)
const isLoadingMore = ref(false)
const searchQuery = ref('')
const scrollContainer = ref<HTMLElement | null>(null)

// Pagination state
const PAGE_SIZE = 20
let currentOffset = 0
let totalCount = 0
const hasMore = ref(true)

// Search debounce
let searchTimeout: ReturnType<typeof setTimeout> | null = null

const loadConversations = async (reset = false) => {
  if (reset) {
    currentOffset = 0
    conversations.value = []
    hasMore.value = true
  }

  if (isLoading.value || isLoadingMore.value) return
  if (!reset && !hasMore.value) return

  const loading = reset ? 'isLoading' : 'isLoadingMore'
  if (reset) {
    isLoading.value = true
  } else {
    isLoadingMore.value = true
  }

  try {
    // Get total count on first load
    if (currentOffset === 0) {
      totalCount = await invoke<number>('get_ai_conversations_count')
    }

    const result = await invoke<Conversation[]>('get_ai_conversations_paginated', {
      limit: PAGE_SIZE,
      offset: currentOffset
    })

    if (reset) {
      conversations.value = result
    } else {
      conversations.value = [...conversations.value, ...result]
    }

    currentOffset += result.length
    hasMore.value = currentOffset < totalCount && result.length === PAGE_SIZE
  } catch (error) {
    console.error('Failed to load conversations:', error)
  } finally {
    isLoading.value = false
    isLoadingMore.value = false
  }
}

const handleScroll = async () => {
  if (!scrollContainer.value || !hasMore.value || isLoadingMore.value) return

  const { scrollTop, scrollHeight, clientHeight } = scrollContainer.value
  const scrollPercentage = (scrollTop + clientHeight) / scrollHeight

  // Load more when scrolled to 80%
  if (scrollPercentage > 0.8) {
    await loadConversations(false)
  }
}

const handleSearch = () => {
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }

  searchTimeout = setTimeout(async () => {
    if (searchQuery.value.trim()) {
      // For search, load all conversations and filter locally
      // In production, you might want to implement server-side search
      isLoading.value = true
      try {
        const allConversations = await invoke<Conversation[]>('get_ai_conversations')
        const query = searchQuery.value.toLowerCase()
        conversations.value = allConversations.filter(conv => 
          (conv.title || '').toLowerCase().includes(query) ||
          conv.model_name.toLowerCase().includes(query)
        )
        hasMore.value = false
      } catch (error) {
        console.error('Failed to search conversations:', error)
      } finally {
        isLoading.value = false
      }
    } else {
      // Reset to paginated mode
      await loadConversations(true)
    }
  }, 300)
}

const createNewConversation = async () => {
  try {
    const conversationId = await invoke<string>('create_ai_conversation', {
      request: {
        title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
        service_name: 'default'
      }
    })
    await loadConversations(true)
    emit('create', conversationId)
  } catch (error) {
    console.error('Failed to create conversation:', error)
  }
}

const selectConversation = (conv: Conversation) => {
  emit('select', conv.id)
}

const renameConversation = async (conv: Conversation) => {
  const newTitle = prompt(t('agent.enterNewConversationName'), conv.title || '')
  if (newTitle && newTitle.trim()) {
    try {
      await invoke('update_ai_conversation_title', {
        conversationId: conv.id,
        title: newTitle.trim(),
        serviceName: 'default'
      })
      // Update local conversation instead of reloading all
      const index = conversations.value.findIndex(c => c.id === conv.id)
      if (index !== -1) {
        conversations.value[index].title = newTitle.trim()
      }
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
    // Remove from local list instead of reloading
    conversations.value = conversations.value.filter(c => c.id !== conv.id)
    totalCount--
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
  
  if (minutes < 1) return t('agent.justNow')
  if (minutes < 60) return `${minutes} ${t('agent.minutesAgo')}`
  if (hours < 24) return `${hours} ${t('agent.hoursAgo')}`
  if (days < 7) return `${days} ${t('agent.daysAgo')}`
  
  return date.toLocaleDateString()
}

onMounted(() => {
  loadConversations(true)
})

defineExpose({
  loadConversations: () => loadConversations(true)
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

