<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between p-4 border-b border-base-300">
      <div class="flex items-center gap-2">
        <i class="fas fa-shield-halved text-warning"></i>
        <h3 class="text-base font-semibold">{{ t('agent.auditFindings') }}</h3>
      </div>
      <button class="btn btn-sm btn-ghost btn-circle" @click="$emit('close')">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div class="p-4 overflow-y-auto flex-1 space-y-3">
      <div
        v-if="policyGate"
        class="rounded-lg border p-3 text-sm"
        :class="policyGate.passed ? 'border-success/30 bg-success/10 text-success' : 'border-error/30 bg-error/10 text-error'"
      >
        <div class="font-semibold">
          {{ policyGate.passed ? t('agent.policyGatePassed') : t('agent.policyGateBlocked') }}
        </div>
        <div class="text-xs mt-1 opacity-90">
          {{ policyGate.reason || (policyGate.passed ? t('agent.policyGateDefaultPass') : t('agent.policyGateDefaultBlock')) }}
        </div>
      </div>

      <div v-if="findings.length === 0" class="text-sm text-base-content/60">
        {{ t('agent.noAuditFindings') }}
      </div>

      <div
        v-for="item in findings"
        :key="item.id"
        class="border border-base-300 rounded-lg p-3 bg-base-100"
      >
        <div class="flex items-center justify-between gap-2">
          <div class="font-medium text-sm truncate" :title="item.title">{{ item.title || item.id }}</div>
          <span class="badge badge-sm" :class="severityClass(item.severity)">{{ item.severity || 'unknown' }}</span>
        </div>

        <p v-if="item.fix" class="text-xs text-base-content/70 mt-2">
          {{ t('agent.recommendedFix') }}: {{ item.fix }}
        </p>

        <div v-if="item.files && item.files.length > 0" class="mt-2 space-y-1">
          <div class="text-xs text-base-content/60">{{ t('agent.affectedFiles') }}:</div>
          <div v-for="f in item.files.slice(0, 5)" :key="f" class="text-xs font-mono text-base-content/80 break-all">
            {{ f }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

interface AuditFinding {
  id: string
  title?: string
  severity?: string
  fix?: string
  files?: string[]
}

interface PolicyGateResult {
  passed: boolean
  reason?: string
}

defineProps<{
  findings: AuditFinding[]
  policyGate?: PolicyGateResult | null
}>()

defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

const severityClass = (severity?: string) => {
  switch ((severity || '').toLowerCase()) {
    case 'critical':
      return 'badge-error'
    case 'high':
      return 'badge-warning'
    case 'medium':
      return 'badge-info'
    case 'low':
      return 'badge-success'
    default:
      return 'badge-ghost'
  }
}
</script>
