<template>
  <div class="rounded-lg border border-base-300 bg-base-100" :style="{ '--editor-min-height': minHeight }">
    <div class="flex items-center justify-between border-b border-base-300 px-3 py-2">
      <span class="badge badge-ghost badge-sm">Markdown</span>
      <div class="tabs tabs-boxed tabs-xs">
        <button type="button" class="tab" :class="{ 'tab-active': mode === 'edit' }" @click="mode = 'edit'">
          {{ editLabel }}
        </button>
        <button type="button" class="tab" :class="{ 'tab-active': mode === 'preview' }" @click="mode = 'preview'">
          {{ previewLabel }}
        </button>
      </div>
    </div>

    <textarea
      v-if="mode === 'edit'"
      ref="textareaRef"
      :value="modelValue"
      class="textarea min-h-[var(--editor-min-height)] w-full resize-y rounded-none border-0 font-mono text-sm focus:outline-none"
      :placeholder="placeholder"
      @input="onInput"
      @paste="onPaste"
    ></textarea>
    <BountyMarkdownContent
      v-else
      class="min-h-[var(--editor-min-height)] rounded-none bg-base-100"
      :content="modelValue"
      :empty-text="emptyPreviewText"
    />
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref } from 'vue'
import BountyMarkdownContent from './BountyMarkdownContent.vue'

const props = withDefaults(defineProps<{
  modelValue: string
  placeholder?: string
  minHeight?: string
  editLabel?: string
  previewLabel?: string
  emptyPreviewText?: string
}>(), {
  placeholder: '',
  minHeight: '8rem',
  editLabel: '编辑',
  previewLabel: '预览',
  emptyPreviewText: '-',
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const mode = ref<'edit' | 'preview'>('edit')
const textareaRef = ref<HTMLTextAreaElement | null>(null)

const onInput = (event: Event) => {
  emit('update:modelValue', (event.target as HTMLTextAreaElement).value)
}

const readFileAsDataUrl = (file: File) => new Promise<string>((resolve, reject) => {
  const reader = new FileReader()
  reader.onload = () => resolve(String(reader.result || ''))
  reader.onerror = () => reject(reader.error)
  reader.readAsDataURL(file)
})

const insertAtCursor = async (markdown: string) => {
  if (mode.value !== 'edit') {
    mode.value = 'edit'
    await nextTick()
  }

  const textarea = textareaRef.value
  const currentValue = props.modelValue || ''
  const start = textarea?.selectionStart ?? currentValue.length
  const end = textarea?.selectionEnd ?? currentValue.length
  const prefix = start > 0 && !currentValue.slice(0, start).endsWith('\n') ? '\n' : ''
  const suffix = currentValue.slice(end).startsWith('\n') ? '' : '\n'
  const nextValue = `${currentValue.slice(0, start)}${prefix}${markdown}${suffix}${currentValue.slice(end)}`

  emit('update:modelValue', nextValue)
  await nextTick()
  textareaRef.value?.focus()
  const cursor = start + prefix.length + markdown.length + suffix.length
  textareaRef.value?.setSelectionRange(cursor, cursor)
}

const onPaste = async (event: ClipboardEvent) => {
  const files = Array.from(event.clipboardData?.files || [])
    .filter(file => file.type.startsWith('image/'))

  if (files.length === 0) {
    return
  }

  event.preventDefault()
  const images = await Promise.all(files.map(async (file, index) => {
    const dataUrl = await readFileAsDataUrl(file)
    const extension = file.type.split('/')[1] || 'png'
    return `![pasted-image-${index + 1}.${extension}](${dataUrl})`
  }))
  await insertAtCursor(images.join('\n\n'))
}
</script>
