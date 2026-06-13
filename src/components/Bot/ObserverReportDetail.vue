<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MissionObservation, MissionRun, MissionRuntimeDetail } from '@/api/missions'

const props = defineProps<{
  run: MissionRun
  runtime: MissionRuntimeDetail | null
  loading?: boolean
}>()

const { t } = useI18n()

const observations = computed(() => props.runtime?.observations ?? [])
const actions = computed(() => props.runtime?.actions ?? [])

function formatDate(value?: string | null): string {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
}

function severityBadge(severity: string): string {
  switch (severity) {
    case 'error':
      return 'badge-error'
    case 'warning':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

function triggerLabel(triggerKind: string): string {
  if (triggerKind === 'event') return t('botConsole.observer.eventTriggered')
  if (triggerKind === 'scheduled') return t('botConsole.observer.scheduled')
  if (triggerKind === 'manual') return t('botConsole.observer.manual')
  return triggerKind
}

function parseDataJson(observation: MissionObservation): unknown {
  if (!observation.data_json) return null
  try {
    return JSON.parse(observation.data_json)
  } catch {
    return observation.data_json
  }
}
</script>

<template>
  <section class="rounded-lg border border-base-300 p-4 space-y-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div class="text-base font-semibold">{{ run.result_summary || t('botConsole.observer.noSummary') }}</div>
        <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-base-content/60">
          <span class="badge badge-outline badge-sm">{{ triggerLabel(run.trigger_kind) }}</span>
          <span>{{ formatDate(run.started_at) }}</span>
          <span v-if="run.completed_at">→ {{ formatDate(run.completed_at) }}</span>
        </div>
      </div>
      <span class="badge badge-sm">{{ run.status }}</span>
    </div>

    <div v-if="loading" class="text-sm text-base-content/60">{{ t('botConsole.observer.loadingDetail') }}</div>

    <template v-else>
      <div v-if="run.error_message" class="alert alert-error">
        <span>{{ run.error_message }}</span>
      </div>

      <section v-if="observations.length > 0">
        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.observer.sections.observations') }}</div>
        <div class="space-y-2">
          <article
            v-for="observation in observations"
            :key="observation.id"
            class="rounded border border-base-300 p-3"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span class="badge badge-sm" :class="severityBadge(observation.severity)">
                {{ observation.severity }}
              </span>
              <span class="text-sm font-medium">{{ observation.title }}</span>
            </div>
            <p v-if="observation.summary" class="mt-2 text-sm whitespace-pre-wrap">{{ observation.summary }}</p>
            <pre
              v-if="parseDataJson(observation)"
              class="mt-2 overflow-x-auto rounded bg-base-200 p-2 text-xs"
            >{{ JSON.stringify(parseDataJson(observation), null, 2) }}</pre>
          </article>
        </div>
      </section>

      <section v-if="actions.length > 0">
        <div class="mb-2 text-sm font-semibold">{{ t('botConsole.observer.sections.actions') }}</div>
        <div class="space-y-2">
          <article v-for="action in actions" :key="action.id" class="rounded border border-base-300 p-3 text-sm">
            <div class="font-medium">{{ action.action_type }}</div>
            <div class="text-xs text-base-content/60">{{ action.status }}</div>
          </article>
        </div>
      </section>
    </template>
  </section>
</template>
