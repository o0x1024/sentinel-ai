<template>
  <div class="space-y-4">
    <section
      v-for="file in files"
      :key="file.path"
      class="rounded-lg border border-base-300/70 bg-base-100/80 overflow-hidden"
    >
      <div class="px-3 py-2 border-b border-base-300/60 bg-base-200/40">
        <div class="text-[11px] uppercase tracking-wide text-base-content/50">
          {{ t('agent.skillHelperFilesLabel') }}
        </div>
        <div class="text-xs font-mono text-base-content/80 break-all mt-0.5">
          {{ file.path }}
        </div>
      </div>
      <div class="px-3 py-3 max-h-[480px] overflow-y-auto">
        <MarkdownRenderer :content="file.content" />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import MarkdownRenderer from './MarkdownRenderer.vue'
import type { SkillFileBlock } from './skillsToolSupport'

defineProps<{
  files: SkillFileBlock[]
}>()

const { t } = useI18n()
</script>
