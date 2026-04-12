<template>
  <div class="border border-base-300 rounded-lg p-4 space-y-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div class="font-semibold text-sm">调试与测试</div>
        <p class="text-xs text-base-content/60 mt-1">
          通过测试事件回放后台 Agent 链路；只有在需要调试时再展开编辑原始 JSON。
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <button
          class="btn btn-xs btn-outline"
          @click="applyRecommendedDispatchTemplate"
          :disabled="!supportsDispatch"
        >
          <i class="fas fa-wand-magic-sparkles mr-1"></i>
          填充事件模板
        </button>
      </div>
    </div>

    <div
      class="rounded-lg border border-base-300 p-3 bg-base-200/40"
      :class="{ 'opacity-60': !supportsDispatch }"
    >
      <div class="text-sm font-medium">事件模板</div>
      <div class="text-xs text-base-content/60 mt-1 whitespace-pre-line">
        {{ dispatchPayloadGuidance }}
      </div>
    </div>

    <div class="flex flex-wrap gap-2">
      <button class="btn btn-outline" @click="$emit('dispatch')" :disabled="dispatching || !supportsDispatch">
        <span v-if="dispatching" class="loading loading-spinner loading-xs mr-2"></span>
        <i v-else class="fas fa-bolt mr-1"></i>
        派发测试事件
      </button>
    </div>

    <div class="collapse collapse-arrow border border-base-300 bg-base-100">
      <input type="checkbox" />
      <div class="collapse-title text-sm font-semibold">
        原始事件载荷 JSON
      </div>
      <div class="collapse-content">
        <label class="form-control pt-2">
          <span class="label-text">测试事件载荷 JSON</span>
          <textarea
            v-model="dispatchPayloadProxy"
            class="textarea textarea-bordered h-36 font-mono text-xs"
            :disabled="!supportsDispatch"
          ></textarea>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { SystemAgentProfilePayload } from '../systemAgentSettingsSupport'
import {
  getSystemAgentDispatchPayloadGuidance,
  getSystemAgentDispatchPayloadTemplate,
  supportsSystemAgentDispatch,
} from './systemAgentRegistry'

const props = defineProps<{
  profile: SystemAgentProfilePayload
  dispatchPayloadText: string
  dispatching: boolean
}>()

const emit = defineEmits<{
  'update:dispatchPayloadText': [value: string]
  dispatch: []
}>()

const dispatchPayloadProxy = computed({
  get: () => props.dispatchPayloadText,
  set: value => emit('update:dispatchPayloadText', value),
})

const supportsDispatch = computed(() => supportsSystemAgentDispatch(props.profile))

const dispatchPayloadGuidance = computed(() => {
  return getSystemAgentDispatchPayloadGuidance(props.profile)
})

function applyRecommendedDispatchTemplate() {
  if (!supportsSystemAgentDispatch(props.profile)) {
    return
  }
  emit('update:dispatchPayloadText', getSystemAgentDispatchPayloadTemplate(props.profile))
}
</script>
