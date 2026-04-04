<template>
  <div class="border border-base-300 rounded-lg p-4 space-y-4">
    <div>
      <div class="font-semibold text-sm">安全策略</div>
      <p class="text-xs text-base-content/60 mt-1">
        只显示当前 Agent 实际相关的安全控制项。
      </p>
    </div>

    <template v-if="isVerifierAgent">
      <label class="label cursor-pointer justify-start gap-3">
        <input v-model="localValue.allowActiveReplay" type="checkbox" class="toggle toggle-primary" />
        <div>
          <div class="label-text font-medium">允许主动重放</div>
          <div class="text-xs text-base-content/60">仅建议在明确授权范围内开启。</div>
        </div>
      </label>
      <label class="label cursor-pointer justify-start gap-3">
        <input v-model="localValue.autoMode" type="checkbox" class="toggle toggle-primary" />
        <div>
          <div class="label-text font-medium">允许自动模式</div>
          <div class="text-xs text-base-content/60">开启后，可在被动分诊后自动衔接验证。</div>
        </div>
      </label>
      <label class="form-control">
        <span class="label-text">作用域主机</span>
        <textarea
          v-model="localValue.scopeHostsText"
          class="textarea textarea-bordered h-24 font-mono text-xs"
          placeholder="示例：&#10;example.com&#10;*.example.com"
        ></textarea>
        <span class="label-text-alt text-base-content/60 mt-1">
          留空表示不额外限制；填写后仅允许对这些主机执行主动验证。
        </span>
      </label>
    </template>

    <template v-else-if="isTriageAgent">
      <label class="label cursor-pointer justify-start gap-3">
        <input v-model="localValue.autoMode" type="checkbox" class="toggle toggle-primary" />
        <div>
          <div class="label-text font-medium">允许自动模式</div>
          <div class="text-xs text-base-content/60">开启后，系统可在后台自动衔接后续处理链路。</div>
        </div>
      </label>
      <label class="label cursor-pointer justify-start gap-3">
        <input v-model="localValue.shadowMode" type="checkbox" class="toggle toggle-primary" />
        <div>
          <div class="label-text font-medium">仅记录影子结果</div>
          <div class="text-xs text-base-content/60">开启后只保留运行痕迹，不会把分诊结果写入安全中心。</div>
        </div>
      </label>
      <div class="rounded-lg bg-base-200/50 border border-base-300 p-3 text-sm text-base-content/70">
        被动分诊的处理范围现在由“流量分析 -> 配置”中的全局 Scope 控制。
      </div>
    </template>

    <template v-else>
      <div class="rounded-lg bg-base-200/50 border border-base-300 p-3 text-sm text-base-content/70">
        当前 Agent 没有额外的专属安全策略项，使用默认安全边界即可。
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { SystemAgentSafetyPolicyForm } from '../systemAgentSettingsSupport'
import type { SystemAgentSafetyMode } from './systemAgentRegistry'

const props = defineProps<{
  modelValue: SystemAgentSafetyPolicyForm
  safetyMode: SystemAgentSafetyMode
}>()

const emit = defineEmits<{
  'update:modelValue': [value: SystemAgentSafetyPolicyForm]
}>()

const localValue = computed({
  get: () => props.modelValue,
  set: value => emit('update:modelValue', value),
})

const isVerifierAgent = computed(() => props.safetyMode === 'verifier')
const isTriageAgent = computed(() => props.safetyMode === 'triage')
</script>
