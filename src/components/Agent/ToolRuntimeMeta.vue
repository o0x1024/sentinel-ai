<template>
  <div
    v-if="runtime"
    class="rounded-lg border border-info/20 bg-info/5 px-3 py-2 text-xs text-base-content/75"
  >
    <div class="flex flex-wrap items-center gap-2">
      <span class="font-medium text-info">{{ t('agent.toolRuntime') }}</span>
      <span :class="['badge badge-sm whitespace-nowrap', environmentBadgeClass]">
        {{ environmentLabel }}
      </span>
      <span
        v-if="runtime.containerRef"
        class="inline-flex items-center gap-1 rounded-md bg-base-100/70 px-2 py-1 text-[10px]"
      >
        <span class="text-base-content/50">{{ t('agent.toolRuntimeContainer') }}</span>
        <span class="font-mono text-[10px] leading-4 text-base-content/70">{{ runtime.containerRef }}</span>
      </span>
    </div>

    <div class="mt-2 flex flex-wrap items-start gap-2">
      <span class="text-base-content/50">{{ t('agent.toolRuntimeWorkingDir') }}</span>
      <span class="break-all rounded-md bg-base-100/70 px-2 py-1 font-mono text-[10px] leading-4 text-base-content/70">
        {{ runtime.workingDir }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { extractToolRuntimeMetadata } from './toolRuntimeSupport'

const props = defineProps<{
  result?: unknown
}>()

const { t } = useI18n()

const runtime = computed(() => extractToolRuntimeMetadata(props.result))

const environmentLabel = computed(() => {
  if (runtime.value?.executionEnvironment === 'docker') {
    return t('agent.toolRuntimeDocker')
  }
  return t('agent.toolRuntimeHost')
})

const environmentBadgeClass = computed(() => {
  if (runtime.value?.executionEnvironment === 'docker') {
    return 'badge-info'
  }
  return 'badge-ghost'
})
</script>
