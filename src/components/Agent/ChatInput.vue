<template>
  <div class="chat-input-container border-t border-base-300/50 bg-base-100 flex-shrink-0 px-4 pb-3 pt-2">
    <div class="input-wrapper flex items-end gap-2 bg-base-200/60 border border-base-300/60 rounded-2xl px-3 py-2 backdrop-blur-sm focus-within:border-primary transition-colors">
      <textarea
        ref="textareaRef"
        v-model="inputValue"
        :placeholder="placeholder"
        :disabled="disabled"
        @keydown="handleKeydown"
        @input="handleInput"
        rows="1"
        class="chat-textarea flex-1 bg-transparent border-none outline-none resize-none text-sm leading-relaxed text-base-content placeholder:text-base-content/50 min-h-6 max-h-52 font-inherit"
      />
      <button 
        v-if="!isExecuting"
        @click="handleSubmit" 
        :disabled="disabled || !canSubmit"
        class="submit-btn w-8 h-8 rounded-full bg-base-300 text-base-content flex items-center justify-center text-lg cursor-pointer transition-colors hover:bg-primary hover:text-primary-content disabled:opacity-40 disabled:cursor-not-allowed flex-shrink-0"
        :title="submitTitle"
      >
        <i class="fas fa-arrow-up"></i>
      </button>
      <button
        v-else
        @click="handleStop"
        class="submit-btn w-8 h-8 rounded-full bg-error text-error-content flex items-center justify-center text-lg cursor-pointer transition-colors hover:bg-error/90 flex-shrink-0"
        title="Stop execution"
      >
        <i class="fas fa-stop"></i>
      </button>
    </div>
    
    <!-- Character count / hints -->
    <div class="input-footer flex justify-between items-center mt-1.5 px-1 text-xs text-base-content/60">
      <span class="hint">Press Enter to send, Shift+Enter for new line</span>
      <span v-if="inputValue.length > 0" class="char-count">
        {{ inputValue.length }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

const props = withDefaults(defineProps<{
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  isExecuting?: boolean
  maxLength?: number
}>(), {
  modelValue: '',
  placeholder: 'Enter your task or question...',
  disabled: false,
  isExecuting: false,
  maxLength: 10000,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'submit', value: string): void
  (e: 'stop'): void
}>()

const textareaRef = ref<HTMLTextAreaElement | null>(null)
const inputValue = ref(props.modelValue)

// Sync with v-model
watch(() => props.modelValue, (newVal) => {
  inputValue.value = newVal
})

watch(inputValue, (newVal) => {
  emit('update:modelValue', newVal)
})

// Can submit
const canSubmit = computed(() => {
  return inputValue.value.trim().length > 0 && !props.isExecuting
})

// Submit title
const submitTitle = computed(() => {
  if (props.isExecuting) return 'Executing...'
  if (!canSubmit.value) return 'Enter a message'
  return 'Send message'
})

// Handle keydown
const handleKeydown = (event: KeyboardEvent) => {
  // Submit on Enter (without Shift)
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    handleSubmit()
  }
}

// Handle input (auto-resize)
const handleInput = () => {
  autoResize()
}

// Auto-resize textarea
const autoResize = () => {
  const textarea = textareaRef.value
  if (textarea) {
    textarea.style.height = 'auto'
    const maxHeight = 200
    textarea.style.height = `${Math.min(textarea.scrollHeight, maxHeight)}px`
  }
}

// Submit handler
const handleSubmit = () => {
  if (!canSubmit.value) return
  
  const value = inputValue.value.trim()
  emit('submit', value)
  
  // Clear input after submit
  inputValue.value = ''
  nextTick(() => {
    autoResize()
  })
}

// Stop handler
const handleStop = () => {
  emit('stop')
}

// Focus method
const focus = () => {
  textareaRef.value?.focus()
}

// Expose methods
defineExpose({
  focus,
})
</script>

<style scoped>
.chat-textarea {
  font-family: inherit;
}

.chat-textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
