<template>
  <div v-if="hasFields" class="space-y-2">
    <div class="flex flex-wrap gap-2">
      <span v-if="location" class="badge badge-ghost badge-sm"> Location: {{ location }} </span>
      <span v-if="technique" class="badge badge-primary badge-sm">
        Technique: {{ technique }}
      </span>
      <span
        v-if="referenceStatus"
        :class="['badge badge-sm', getStatusBadgeClass(referenceStatus)]"
      >
        Reference: {{ referenceStatus }}
      </span>
      <span v-if="probeStatus" :class="['badge badge-sm', getStatusBadgeClass(probeStatus)]">
        Probe: {{ probeStatus }}
      </span>
    </div>

    <div v-if="targetPath" class="space-y-2">
      <span class="badge badge-outline badge-sm">Target Path</span>
      <span
        v-if="isCompactTargetPath"
        class="badge badge-outline badge-sm font-mono max-w-full break-all"
      >
        {{ targetPath }}
      </span>
      <pre
        v-else
        class="m-0 overflow-x-auto rounded-lg border border-base-300 bg-base-200 px-3 py-2 font-mono text-xs whitespace-pre-wrap break-all text-base-content"
        >{{ targetPath }}</pre
      >
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const TARGET_PATH_COMPACT_THRESHOLD = 56

const props = defineProps<{
  location?: string
  technique?: string
  targetPath?: string
  referenceStatus?: string
  probeStatus?: string
}>()

const hasFields = computed(() =>
  Boolean(
    props.location ||
      props.technique ||
      props.targetPath ||
      props.referenceStatus ||
      props.probeStatus
  )
)

const isCompactTargetPath = computed(() => {
  const normalized = props.targetPath?.trim() || ''
  if (!normalized) return false
  return normalized.length <= TARGET_PATH_COMPACT_THRESHOLD
})

function getStatusBadgeClass(status: string) {
  const numericStatus = Number(status)
  if (Number.isFinite(numericStatus)) {
    return numericStatus >= 400 ? 'badge-error' : 'badge-success'
  }
  return 'badge-outline'
}
</script>
