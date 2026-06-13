<template>
  <div :class="['rounded-lg overflow-hidden border-l-4 mb-2', containerClass]">
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
      :class="[headerClass, 'hover:opacity-90 cursor-pointer']"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="toggleExpanded"
    >
      <i
        :class="[
          'fas text-xs transition-transform flex-shrink-0',
          isExpanded ? 'fa-chevron-down' : 'fa-chevron-right',
          titleClass,
        ]"
      ></i>
      <div
        :class="[
          'w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 shadow-sm',
          iconClass,
        ]"
      >
        <i :class="['text-white text-sm', iconName]"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div :class="['font-semibold text-sm', titleClass]">{{ title }}</div>
        <div class="text-xs text-base-content/70 mt-0.5 truncate">
          {{ skillName }} ({{ skillId }})
        </div>
        <div
          v-if="!isExpanded && fileSections.length > 0"
          class="text-xs text-base-content/60 mt-0.5 truncate"
        >
          {{ t('agent.skillHelperFilesLabel') }}: {{ fileSections.length }}
        </div>
      </div>
      <span v-if="modeLabel" :class="['badge badge-sm badge-ghost whitespace-nowrap', modeBadgeClass]">
        {{ modeLabel }}
      </span>
    </button>

    <div
      v-show="isExpanded"
      :class="['px-4 py-3 bg-base-100/60 space-y-3 border-t', detailsBorderClass]"
    >
      <SkillFileMarkdownSections v-if="fileSections.length > 0" :files="fileSections" />

      <div v-else-if="fallbackMarkdown" class="max-h-[480px] overflow-y-auto">
        <MarkdownRenderer :content="fallbackMarkdown" />
      </div>

      <div v-else class="text-sm text-base-content/60">
        {{ t('agent.skillHelperFileEmpty') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import MarkdownRenderer from './MarkdownRenderer.vue'
import SkillFileMarkdownSections from './SkillFileMarkdownSections.vue'
import { resolveSkillFileSections } from './skillsToolSupport'

const props = withDefaults(
  defineProps<{
    mode: 'invoke' | 'fork'
    skillId: string
    skillName: string
    referencedFiles?: string[]
    resultPreview?: string
  }>(),
  {
    referencedFiles: () => [],
    resultPreview: '',
  },
)

const { t } = useI18n()
const isExpanded = ref(false)

const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const referencedFiles = computed(() =>
  props.referencedFiles.filter((file) => String(file || '').trim().length > 0),
)

const fileSections = computed(() =>
  resolveSkillFileSections(props.resultPreview, referencedFiles.value),
)

const fallbackMarkdown = computed(() => {
  const preview = props.resultPreview?.trim()
  if (!preview || fileSections.value.length > 0) return ''
  return preview
})

const isFork = computed(() => props.mode === 'fork')

const title = computed(() =>
  isFork.value ? t('agent.skillForkedTitle') : t('agent.skillLoadedTitle'),
)

const modeLabel = computed(() =>
  isFork.value ? t('agent.skillModeFork') : t('agent.skillModeInvoke'),
)

const containerClass = computed(() =>
  isFork.value ? 'bg-secondary/10 border-secondary' : 'bg-success/10 border-success',
)

const headerClass = computed(() =>
  isFork.value ? 'bg-secondary/20' : 'bg-success/20',
)

const detailsBorderClass = computed(() =>
  isFork.value ? 'border-secondary/20' : 'border-success/20',
)

const iconClass = computed(() => (isFork.value ? 'bg-secondary' : 'bg-success'))

const titleClass = computed(() => (isFork.value ? 'text-secondary' : 'text-success'))

const modeBadgeClass = computed(() => (isFork.value ? 'text-secondary' : 'text-success'))

const iconName = computed(() => (isFork.value ? 'fas fa-code-branch' : 'fas fa-lightbulb'))
</script>
