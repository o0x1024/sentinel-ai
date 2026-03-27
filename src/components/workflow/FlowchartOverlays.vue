<template>
  <div v-if="contextMenu.visible" class="fixed bg-base-100 shadow-xl rounded-lg border border-base-300 py-1 min-w-[160px]" style="z-index: 9999;" :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }">
    <div v-for="(item, index) in contextMenu.items" :key="index" class="px-4 py-2 hover:bg-base-200 cursor-pointer text-sm transition-colors" :class="{ 'text-error': item.danger }" @click="onContextMenuClick(item)">{{ item.label }}</div>
  </div>
  <dialog :class="['modal', { 'modal-open': showAiGenerateModal }]">
    <div class="modal-box max-w-2xl">
      <div class="flex justify-between items-center mb-3">
        <h3 class="font-bold text-lg">{{ title }}</h3>
        <button class="btn btn-sm btn-ghost" @click="onCloseAiGenerateModal">✕</button>
      </div>
      <div class="space-y-3">
        <div class="alert alert-info"><i class="fas fa-info-circle"></i><span>{{ helpText }}</span></div>
        <textarea :value="aiGenerateText" class="textarea textarea-bordered w-full font-mono text-sm" rows="6" :placeholder="placeholder" spellcheck="false" @input="$emit('update:aiGenerateText', ($event.target as HTMLTextAreaElement).value)"></textarea>
        <div v-if="aiGenerateError" class="alert alert-error text-sm"><i class="fas fa-exclamation-triangle"></i><span>{{ aiGenerateError }}</span></div>
      </div>
      <div class="modal-action">
        <button class="btn" @click="onCloseAiGenerateModal" :disabled="isAiGenerating">{{ cancelLabel }}</button>
        <button class="btn btn-primary" @click="onGenerateWorkflowFromNl" :disabled="isAiGenerating || !aiGenerateText.trim()">
          <i v-if="isAiGenerating" class="fas fa-spinner fa-spin mr-1"></i>
          <i v-else class="fas fa-magic mr-1"></i>
          {{ generateLabel }}
        </button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
defineProps<{
  contextMenu: { visible: boolean; x: number; y: number; items: Array<{ label: string; action: () => void; danger?: boolean }> }
  showAiGenerateModal: boolean
  aiGenerateText: string
  isAiGenerating: boolean
  aiGenerateError: string
  title: string
  helpText: string
  placeholder: string
  cancelLabel: string
  generateLabel: string
  onContextMenuClick: (item: { label: string; action: () => void; danger?: boolean }) => void
  onCloseAiGenerateModal: () => void
  onGenerateWorkflowFromNl: () => void
}>()

defineEmits<{ 'update:aiGenerateText': [value: string] }>()
</script>
