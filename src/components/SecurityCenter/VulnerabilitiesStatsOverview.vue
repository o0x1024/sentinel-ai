<template>
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
    <div class="stat bg-error/10 rounded-lg shadow-sm">
      <div class="stat-title text-xs">{{ t('vulnerabilities.severity.critical') }}</div>
      <div class="stat-value text-error text-lg">{{ stats.critical }}</div>
    </div>
    <div class="stat bg-warning/10 rounded-lg shadow-sm">
      <div class="stat-title text-xs">{{ t('vulnerabilities.severity.high') }}</div>
      <div class="stat-value text-warning text-lg">{{ stats.high }}</div>
    </div>
    <div class="stat bg-info/10 rounded-lg shadow-sm">
      <div class="stat-title text-xs">{{ t('vulnerabilities.severity.medium') }}</div>
      <div class="stat-value text-info text-lg">{{ stats.medium }}</div>
    </div>
    <div class="stat bg-success/10 rounded-lg shadow-sm">
      <div class="stat-title text-xs">{{ t('vulnerabilities.severity.low') }}</div>
      <div class="stat-value text-success text-lg">{{ stats.low }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  stats: {
    critical: number
    high: number
    medium: number
    low: number
  }
}>()

const { t } = useI18n()
</script>
