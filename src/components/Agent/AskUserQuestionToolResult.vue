<template>
  <div class="rounded-lg overflow-hidden border border-base-300 bg-base-100">
    <div class="flex items-center gap-3 border-b border-base-300 bg-info/10 px-4 py-3">
      <div class="flex h-8 w-8 items-center justify-center rounded-full bg-info text-white">
        <i class="fas fa-list-check text-sm"></i>
      </div>
      <div>
        <div class="text-sm font-semibold text-info">用户选择</div>
        <div class="text-xs text-base-content/60">{{ resultSummary }}</div>
      </div>
    </div>

    <div v-if="answerEntries.length > 0" class="space-y-3 px-4 py-4">
      <div
        v-for="entry in answerEntries"
        :key="entry.question"
        class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3"
      >
        <div class="text-xs font-medium uppercase tracking-wide text-base-content/50">
          {{ entry.header || 'Question' }}
        </div>
        <div class="mt-1 text-sm text-base-content">{{ entry.question }}</div>
        <div class="mt-2 text-sm font-medium text-primary">{{ entry.answer || '-' }}</div>
      </div>
    </div>

    <div v-else class="px-4 py-4 text-sm">
      <div v-if="isFailed" class="rounded-lg border border-error/30 bg-error/10 px-3 py-3 text-error">
        {{ errorText }}
      </div>
      <div v-else-if="isTimeoutWithoutAnswers" class="rounded-lg border border-warning/30 bg-warning/10 px-3 py-3 text-base-content/75">
        AskUserQuestion 已超时，本轮没有写入答案。
      </div>
      <div v-else class="space-y-3">
        <div
          class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3 text-base-content/70"
        >
          正在等待用户回答这些问题。
        </div>
        <div
          v-for="question in pendingQuestions"
          :key="question.question"
          class="rounded-lg border border-base-300 bg-base-200/30 px-3 py-3"
        >
          <div class="text-xs font-medium uppercase tracking-wide text-base-content/50">
            {{ question.header || 'Question' }}
          </div>
          <div class="mt-1 text-sm text-base-content">{{ question.question }}</div>
          <div class="mt-2 text-xs text-base-content/60">
            {{ question.options.map((option) => option.label).join(' / ') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface QuestionItem {
  header?: string
  question: string
  options: { label: string }[]
}

const props = defineProps<{
  result?: unknown
  args?: unknown
  error?: string
  status?: string
}>()

const parsedResult = computed(() => {
  if (!props.result) return null

  let result = props.result
  if (typeof result === 'string') {
    try {
      result = JSON.parse(result)
    } catch {
      return null
    }
  }

  if (Array.isArray(result)) {
    const textItem = result.find((item: any) => item?.type === 'text' && item?.text)
    if (!textItem) return null
    try {
      return JSON.parse(textItem.text)
    } catch {
      return null
    }
  }

  return typeof result === 'object' ? result : null
})

const answerEntries = computed(() => {
  const result = parsedResult.value as {
    questions?: QuestionItem[]
    answers?: Record<string, string>
    status?: string
    source?: string
  } | null
  if (!result?.questions || !result.answers) return []
  return result.questions.map((question) => ({
    header: question.header || '',
    question: question.question,
    answer: result.answers?.[question.question] || '',
  }))
})

const resultSummary = computed(() => {
  const result = parsedResult.value as { status?: string; source?: string } | null
  if (!result) return 'AskUserQuestion 已收到回答'
  if (result.status === 'timeout_with_default') {
    return 'AskUserQuestion 超时后使用默认值继续'
  }
  if (result.status === 'timeout_without_default') {
    return 'AskUserQuestion 超时，未写入默认答案'
  }
  if (result.source === 'system_default') {
    return 'AskUserQuestion 使用系统默认值'
  }
  return 'AskUserQuestion 已收到回答'
})

const pendingQuestions = computed(() => {
  if (!props.args || typeof props.args !== 'object') return []
  const args = props.args as { questions?: QuestionItem[] }
  return Array.isArray(args.questions) ? args.questions : []
})

const isFailed = computed(() => props.status === 'failed')
const isTimeoutWithoutAnswers = computed(() => {
  const result = parsedResult.value as { status?: string } | null
  return result?.status === 'timeout_without_default'
})

const errorText = computed(() => {
  const raw = String(props.error || '').trim()
  if (!raw) return '用户拒绝回答这些问题。'
  return raw
})
</script>
