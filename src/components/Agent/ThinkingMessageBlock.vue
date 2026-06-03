<template>
  <div class="rounded-md border-l-[3px] border-info bg-info/10">
    <div class="flex w-full items-center gap-2 px-3 py-2 text-left text-sm">
      <button
        type="button"
        class="thinking-header-button min-w-0 flex flex-1 items-center gap-2 text-left"
        :aria-expanded="isExpanded ? 'true' : 'false'"
        @click="toggleExpanded"
      >
        <i class="fas fa-brain shrink-0 text-xs text-info"></i>
        <span class="shrink-0 font-semibold text-base-content/75">
          {{ t('agent.thinkingTitle') }}
        </span>
        <span class="min-w-0 flex-1 truncate text-xs text-base-content/55">
          {{ preview }}
        </span>
        <span class="shrink-0 text-xs tabular-nums text-base-content/45">
          {{ statsText }}
        </span>
        <i
          :class="[
            'fas fa-chevron-down shrink-0 text-[10px] text-base-content/45 transition-transform',
            isExpanded ? 'rotate-180' : '',
          ]"
        ></i>
      </button>

      <span
        v-if="isStreaming"
        class="shrink-0 rounded-full bg-info/15 px-2 py-0.5 text-xs text-info"
      >
        {{ t('agent.statusRunning') }}
      </span>
    </div>

    <pre
      v-show="isExpanded"
      :class="[
        'thinking-content whitespace-pre-wrap break-words border-t border-info/20 px-3 py-2 text-xs leading-relaxed text-base-content/70',
        isStreaming ? 'is-streaming' : 'is-limited-height',
      ]"
    >{{ content }}</pre>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const props = withDefaults(
  defineProps<{
    content: string
    status?: 'streaming' | 'complete'
  }>(),
  {
    content: '',
    status: 'complete',
  }
)

const emit = defineEmits<{
  (e: 'heightChanged'): void
}>()

const { t } = useI18n()

const isStreaming = computed(() => props.status === 'streaming')
const isExpanded = ref(isStreaming.value)

const notifyHeightChanged = async () => {
  await nextTick()
  emit('heightChanged')
}

const toggleExpanded = () => {
  if (isStreaming.value) return
  isExpanded.value = !isExpanded.value
  void notifyHeightChanged()
}

watch(
  isStreaming,
  streaming => {
    if (streaming) {
      isExpanded.value = true
      void notifyHeightChanged()
      return
    }
    isExpanded.value = false
    void notifyHeightChanged()
  }
)

const preview = computed(() => {
  const compact = props.content.replace(/\s+/g, ' ').trim()
  if (!compact) return t('agent.aiIsThinking')
  return compact.length > 140 ? `${compact.slice(0, 137)}...` : compact
})

const lineCount = computed(() => {
  const content = props.content.trim()
  return content ? content.split(/\r?\n/).length : 0
})

const charCount = computed(() => props.content.length)

const statsText = computed(() =>
  t('agent.thinkingStats', {
    lines: lineCount.value,
    chars: charCount.value,
  })
)

</script>

<style scoped>
.thinking-header-button {
  appearance: none;
  border: 0;
  background: transparent;
  padding: 0;
  color: inherit;
}

.thinking-header-button:focus-visible {
  outline: 2px solid rgb(var(--in) / 0.45);
  outline-offset: 3px;
}

.thinking-content {
  margin: 0;
  overflow-wrap: anywhere;
  scrollbar-gutter: stable;
}

.thinking-content.is-streaming {
  max-height: none;
  overflow: visible;
}

.thinking-content.is-limited-height {
  max-height: min(48vh, 560px);
  overflow-y: auto;
  overscroll-behavior: contain;
}
</style>
