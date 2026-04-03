<template>
  <section class="flex h-full min-h-0 flex-col rounded-l-lg border border-base-300 bg-base-100">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex flex-wrap items-center gap-3">
        <label class="flex min-w-0 flex-1 items-center gap-3">
          <span class="w-14 text-sm text-base-content/70">{{ $t('trafficAnalysis.intruder.labels.target') }}</span>
          <input
            :value="targetUrl"
            type="text"
            class="input input-bordered input-sm flex-1"
            :placeholder="$t('trafficAnalysis.intruder.placeholders.targetUrl')"
            @input="$emit('update:targetUrl', ($event.target as HTMLInputElement).value)"
          />
        </label>

        <label class="flex items-center gap-2 text-sm">
          <input
            :checked="updateHostHeader"
            type="checkbox"
            class="checkbox checkbox-sm checkbox-primary"
            @change="$emit('update:updateHostHeader', ($event.target as HTMLInputElement).checked)"
          />
          <span>{{ $t('trafficAnalysis.intruder.labels.updateHostHeader') }}</span>
        </label>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2 border-b border-base-300 bg-base-200 px-4 py-2">
      <span class="text-sm font-medium text-base-content/70">{{ $t('trafficAnalysis.intruder.sections.positions') }}</span>
      <button class="btn btn-sm btn-ghost" type="button" @click="markSelection">
        {{ $t('trafficAnalysis.intruder.actions.addPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost" type="button" @click="$emit('clearMarkers')">
        {{ $t('trafficAnalysis.intruder.actions.clearPositionSymbol') }}
      </button>
      <button class="btn btn-sm btn-ghost" type="button" @click="$emit('autoMark')">
        {{ $t('trafficAnalysis.intruder.actions.autoMark') }}
      </button>
      <div class="ml-auto flex items-center gap-3 text-xs text-base-content/70">
        <span>{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.detectedPositions') }}</span>
        <span>{{ requestLengthLabel }}</span>
      </div>
    </div>

    <div class="min-h-0 flex-1">
      <HttpMessageSurface
        ref="requestEditor"
        :model-value="requestText"
        message-type="request"
        height="100%"
        display-mode="raw"
        :state-key="`intruder:editor:${targetUrl || 'default'}`"
        @update:model-value="$emit('update:requestText', $event)"
      />
    </div>

    <div class="flex items-center gap-3 border-t border-base-300 bg-base-200 px-4 py-2 text-xs text-base-content/70">
      <span>{{ $t('trafficAnalysis.intruder.help.markerHint') }}</span>
      <span class="ml-auto">{{ positions.length }} {{ $t('trafficAnalysis.intruder.labels.positionCount') }}</span>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog'
import HttpMessageSurface from '@/components/http-editor/HttpMessageSurface.vue'
import type { IntruderPosition } from './types'

const { t } = useI18n()

const props = defineProps<{
  requestText: string
  targetUrl: string
  updateHostHeader: boolean
  positions: IntruderPosition[]
}>()

const emit = defineEmits<{
  (e: 'update:requestText', value: string): void
  (e: 'update:targetUrl', value: string): void
  (e: 'update:updateHostHeader', value: boolean): void
  (e: 'markSelection', payload: { start: number; end: number }): void
  (e: 'autoMark'): void
  (e: 'clearMarkers'): void
}>()

const requestEditor = ref<InstanceType<typeof HttpMessageSurface> | null>(null)

const requestLengthLabel = computed(() => {
  const length = props.requestText.length
  return `${t('trafficAnalysis.intruder.labels.length')}: ${length}`
})

function markSelection() {
  const selection = requestEditor.value?.getSelectionRange()
  if (!selection || selection.from === selection.to) {
    dialog.toast.info(t('trafficAnalysis.intruder.messages.selectTextFirst'))
    return
  }

  emit('markSelection', {
    start: selection.from,
    end: selection.to,
  })
}
</script>
