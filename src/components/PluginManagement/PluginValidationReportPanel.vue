<template>
  <div v-if="report" class="space-y-3">
    <div class="alert" :class="hasErrors ? 'alert-warning' : 'alert-info'">
      <i :class="hasErrors ? 'fas fa-triangle-exclamation' : 'fas fa-circle-info'"></i>
      <div class="flex-1">
        <div class="font-semibold">{{ report.title }}</div>
        <div class="text-xs opacity-80">{{ summary }}</div>
      </div>
    </div>

    <div class="space-y-3">
      <div
        v-for="section in report.sections"
        :key="section.key"
        class="rounded-lg border border-base-300 bg-base-200 p-3"
      >
        <div class="mb-2 flex items-center justify-between gap-2">
          <div class="font-semibold text-sm">{{ section.title }}</div>
          <div class="flex gap-2 text-xs">
            <span v-if="section.errors.length > 0" class="badge badge-error badge-sm">
              {{ section.errors.length }} {{ $t('common.error', '错误') }}
            </span>
            <span v-if="section.warnings.length > 0" class="badge badge-warning badge-sm">
              {{ section.warnings.length }} {{ $t('plugins.warnings', '警告') }}
            </span>
          </div>
        </div>

        <div v-if="section.errors.length > 0" class="space-y-1">
          <button
            v-for="issue in section.errors"
            :key="`${section.key}-error-${issue.code}-${issue.message}`"
            type="button"
            class="block w-full rounded bg-error/10 px-2 py-1 text-left text-sm text-error transition-colors hover:bg-error/15"
            @click="$emit('focusIssue', section.key, issue.code, issue.message)"
          >
            {{ issue.message }}
          </button>
        </div>

        <div v-if="section.warnings.length > 0" class="mt-2 space-y-1">
          <button
            v-for="issue in section.warnings"
            :key="`${section.key}-warning-${issue.code}-${issue.message}`"
            type="button"
            class="block w-full rounded bg-warning/10 px-2 py-1 text-left text-sm text-warning-content transition-colors hover:bg-warning/15"
            @click="$emit('focusIssue', section.key, issue.code, issue.message)"
          >
            {{ issue.message }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { summarizeAiValidationReport, type AiValidationReport } from './aiGeneratedPluginGate'

const { t } = useI18n()

const props = defineProps<{
  report: AiValidationReport | null
}>()

defineEmits<{
  focusIssue: [sectionKey: string, issueCode: string, message: string]
}>()

const hasErrors = computed(() => props.report?.sections.some(section => section.errors.length > 0) ?? false)
const summary = computed(() => summarizeAiValidationReport(props.report))
</script>
