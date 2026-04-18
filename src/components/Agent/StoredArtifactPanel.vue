<template>
  <div :class="panelClass">
    <div :class="titleClass">
      Stored Artifacts
    </div>
    <div class="mt-2 space-y-3">
      <div
        v-for="artifact in artifacts"
        :key="`${artifact.storage_backend}:${artifact.slot}:${artifact.path}`"
        :class="itemClass"
      >
        <div class="flex flex-wrap items-center gap-2 text-[11px]">
          <span :class="badgeClass">{{ artifact.slot }}</span>
          <span class="opacity-80">{{ artifact.storage_backend }}</span>
          <span class="opacity-70">{{ artifact.sizeLabel }}</span>
          <span class="opacity-70">{{ artifact.lineLabel }}</span>
          <span :class="artifact.fullyRead ? completeClass : progressClass">
            {{ artifact.progressLabel }}
          </span>
        </div>
        <div
          v-if="artifact.progressPercent !== null"
          :class="progressTrackClass"
          :aria-label="artifact.progressLabel"
          role="progressbar"
          :aria-valuemin="0"
          :aria-valuemax="100"
          :aria-valuenow="artifact.progressPercent"
        >
          <div
            :class="artifact.fullyRead ? progressFillCompleteClass : progressFillPendingClass"
            :style="{ width: `${artifact.progressPercent}%` }"
          />
        </div>
        <code :class="pathClass">{{ artifact.path }}</code>
        <div class="mt-2 text-[11px] opacity-80">{{ artifact.nextLabel }}</div>
        <code :class="commandClass">{{ artifact.nextCommand }}</code>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { StoredArtifactView } from './storedArtifactSupport'

const props = defineProps<{
  artifacts: StoredArtifactView[]
  variant?: 'default' | 'terminal'
}>()

const isTerminal = computed(() => props.variant === 'terminal')

const panelClass = computed(() =>
  isTerminal.value
    ? 'mt-3 rounded-md border border-[#3a3a3a] bg-[#181818] px-3 py-2 text-[#d4d4d4]'
    : 'mt-3 rounded-md border border-base-300 bg-base-100 px-3 py-2 text-base-content'
)

const itemClass = computed(() =>
  isTerminal.value
    ? 'rounded-md border border-[#303030] bg-[#202020] px-3 py-2'
    : 'rounded-md border border-base-300 bg-base-200/60 px-3 py-2'
)

const titleClass = computed(() =>
  isTerminal.value ? 'text-xs font-semibold text-[#dcdcaa]' : 'text-xs font-semibold text-base-content/80'
)

const badgeClass = computed(() =>
  isTerminal.value
    ? 'rounded-full bg-[#264f78] px-2 py-0.5 text-[10px] font-semibold text-[#9cdcfe]'
    : 'rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-semibold text-primary'
)

const pathClass = computed(() =>
  isTerminal.value
    ? 'mt-2 block whitespace-pre-wrap break-all rounded bg-[#111111] px-2 py-1 text-[11px] text-[#ce9178]'
    : 'mt-2 block whitespace-pre-wrap break-all rounded bg-base-300 px-2 py-1 text-[11px] text-base-content/80'
)

const commandClass = computed(() =>
  isTerminal.value
    ? 'mt-1 block whitespace-pre-wrap break-all rounded bg-[#111111] px-2 py-1 text-[11px] text-[#b5cea8]'
    : 'mt-1 block whitespace-pre-wrap break-all rounded bg-base-300 px-2 py-1 text-[11px] text-base-content/80'
)

const progressClass = computed(() =>
  isTerminal.value
    ? 'rounded-full bg-[#3a3d41] px-2 py-0.5 text-[10px] font-medium text-[#d7ba7d]'
    : 'rounded-full bg-warning/10 px-2 py-0.5 text-[10px] font-medium text-warning'
)

const completeClass = computed(() =>
  isTerminal.value
    ? 'rounded-full bg-[#1f4721] px-2 py-0.5 text-[10px] font-medium text-[#6a9955]'
    : 'rounded-full bg-success/10 px-2 py-0.5 text-[10px] font-medium text-success'
)

const progressTrackClass = computed(() =>
  isTerminal.value
    ? 'mt-2 h-1.5 overflow-hidden rounded-full bg-[#111111]'
    : 'mt-2 h-1.5 overflow-hidden rounded-full bg-base-300'
)

const progressFillPendingClass = computed(() =>
  isTerminal.value
    ? 'h-full rounded-full bg-[#d7ba7d] transition-all'
    : 'h-full rounded-full bg-warning transition-all'
)

const progressFillCompleteClass = computed(() =>
  isTerminal.value
    ? 'h-full rounded-full bg-[#6a9955] transition-all'
    : 'h-full rounded-full bg-success transition-all'
)
</script>
