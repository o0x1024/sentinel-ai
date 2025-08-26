<template>
  <div :class="[
    'chat',
    message.role === 'user' ? 'chat-end' : 'chat-start',
    'mb-4'
  ]">
    <div class="chat-image">
      <div class="w-10 h-8 rounded-full shadow-lg border-2 border-base-300 bg-base-100 flex items-center justify-center">
        <!-- User SVG Avatar -->
        <svg v-if="message.role === 'user'" class="w-6 h-6 text-primary flex-shrink-0" fill="currentColor" viewBox="0 0 24 24">
          <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
        </svg>
        <!-- AI SVG Avatar -->
        <svg v-else class="w-6 h-6 text-secondary flex-shrink-0" fill="currentColor" viewBox="0 0 24 24">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.94-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
        </svg>
      </div>
    </div>
    
    <div class="chat-header mb-2">
      <span class="font-medium text-sm text-base-content/80">
        {{ message.role === 'user' ? t('common.you', '您') : t('common.assistant', 'AI助手') }}
      </span>
      <time class="text-xs text-base-content/60 ml-2 px-2 py-0.5 bg-base-200 rounded-full">
        {{ formatTime(message.timestamp) }}
      </time>
    </div>
    
    <div :class="[
      'chat-bubble max-w-[85%] shadow-sm border transition-all duration-200',
      message.role === 'user' 
        ? 'bg-base-100 text-primary-content border-primary/20' 
        : 'bg-base-100 text-base-content border-base-300 hover:border-base-400'
    ]">
      <!-- Message Content -->
      <MessageContent 
        :message="message"
        :displayed-content="displayedContent"
        :is-typing="isTyping"
        @skip-typewriter="$emit('skip-typewriter', message.id)"
      />
      
      <!-- Execution Plan -->
      <ExecutionPlan 
        v-if="message.executionPlan"
        :execution-plan="message.executionPlan"
        :execution-progress="message.executionProgress"
        :current-step="message.currentStep"
        :is-streaming="message.isStreaming"
      />
      
      <!-- Tool Executions -->
      <ToolExecutions 
        v-if="message.toolExecutions?.length"
        :tool-executions="message.toolExecutions"
      />
      
      <!-- Execution Result -->
      <ExecutionResult 
        v-if="message.executionResult"
        :execution-result="message.executionResult"
        :message="message"
      />
      
      <!-- Error Actions -->
      <MessageActions 
        v-if="message.hasError && message.role === 'assistant'"
        :message="message"
        @retry="$emit('retry')"
        @clear-error="$emit('clear-error', message)"
        @open-settings="$emit('open-settings')"
      />
      
      <!-- Debug Panel -->
      <DebugPanel 
        v-if="showDebugInfo && message.role === 'assistant'"
        :message="message"
        :typewriter-mode="typewriterMode"
        :typewriter-progress="typewriterProgress"
        :enable-typewriter="enableTypewriter"
        :typewriter-speed="typewriterSpeed"
        @update:enable-typewriter="$emit('update:enable-typewriter', $event)"
        @update:typewriter-speed="$emit('update:typewriter-speed', $event)"
        @skip-typewriter="$emit('skip-typewriter', message.id)"
        @test-typewriter="$emit('test-typewriter', message.id)"
        @debug-display="$emit('debug-display', message.id)"
        @debug-typewriter="$emit('debug-typewriter', message.id)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessageUtils } from '../composables/useMessageUtils'
import MessageContent from './MessageParts/MessageContent.vue'
import ExecutionPlan from './MessageParts/ExecutionPlan.vue'
import ToolExecutions from './MessageParts/ToolExecutions.vue'
import ExecutionResult from './MessageParts/ExecutionResult.vue'
import MessageActions from './MessageParts/MessageActions.vue'
import DebugPanel from './MessageParts/DebugPanel.vue'

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  isStreaming?: boolean
  hasError?: boolean
  executionPlan?: any
  toolExecutions?: any[]
  executionResult?: any
  executionProgress?: number
  currentStep?: string
  totalSteps?: number
  completedSteps?: number
}

const props = defineProps<{
  message: ChatMessage
  displayedContent: string
  isTyping: boolean
  typewriterMode: string
  typewriterProgress: string
  showDebugInfo: boolean
  enableTypewriter: boolean
  typewriterSpeed: number
}>()

defineEmits([
  'skip-typewriter',
  'retry',
  'clear-error',
  'open-settings',
  'update:enable-typewriter',
  'update:typewriter-speed',
  'test-typewriter',
  'debug-display',
  'debug-typewriter'
])

const { t } = useI18n()
const { formatTime } = useMessageUtils()
</script>