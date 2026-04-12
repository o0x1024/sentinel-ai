<template>
  <Teleport to="body">
    <div
      v-if="pendingRequest"
      class="fixed inset-0 z-[90] flex items-center justify-center bg-black/50 px-4"
    >
      <div class="w-full max-w-3xl rounded-2xl border border-base-300 bg-base-100 shadow-2xl">
        <div class="flex items-center justify-between border-b border-base-300 px-6 py-4">
          <div>
            <h3 class="text-lg font-semibold text-base-content">需要你的选择</h3>
            <p class="mt-1 text-sm text-base-content/60">
              助手正在等待结构化回答后继续执行。
            </p>
          </div>
          <button class="btn btn-sm btn-ghost" @click="handleReject" :disabled="submitting">
            拒绝
          </button>
        </div>

        <div class="max-h-[70vh] space-y-5 overflow-y-auto px-6 py-5">
          <section
            v-for="question in pendingRequest.questions"
            :key="question.question"
            class="rounded-xl border border-base-300 bg-base-200/40 p-4"
          >
            <div class="mb-3 flex items-center gap-2">
              <span class="badge badge-outline">{{ question.header }}</span>
              <span class="text-sm font-medium text-base-content">{{ question.question }}</span>
            </div>

            <div class="space-y-2">
              <label
                v-for="option in question.options"
                :key="`${question.question}:${option.label}`"
                class="flex cursor-pointer items-start gap-3 rounded-lg border border-base-300 bg-base-100 px-3 py-3 transition-colors hover:border-primary/40"
                @mouseenter="focusPreview(question.question, option.label)"
              >
                <input
                  :name="question.question"
                  type="radio"
                  class="radio radio-sm mt-0.5"
                  :checked="selectedAnswers[question.question] === option.label && !customAnswers[question.question]?.trim()"
                  @change="selectOption(question.question, option.label)"
                >
                <div class="min-w-0">
                  <div class="text-sm font-medium text-base-content">{{ option.label }}</div>
                  <div class="mt-1 text-xs leading-5 text-base-content/65">
                    {{ option.description }}
                  </div>
                </div>
              </label>
            </div>

            <div
              v-if="resolvePreview(question.question)"
              class="mt-3 rounded-lg border border-info/30 bg-info/5 p-3"
            >
              <div class="mb-2 text-xs font-medium uppercase tracking-wide text-info">
                预览
              </div>
              <MarkdownRenderer :content="resolvePreview(question.question) || ''" />
            </div>

            <div class="mt-3">
              <label class="mb-1 block text-xs font-medium text-base-content/60">
                或输入自定义回答
              </label>
              <input
                :value="customAnswers[question.question] || ''"
                type="text"
                class="input input-bordered w-full"
                placeholder="输入自定义回答"
                @input="handleCustomAnswerInput(question.question, $event)"
              >
            </div>
          </section>
        </div>

        <div class="flex items-center justify-between border-t border-base-300 px-6 py-4">
          <p class="text-xs text-base-content/55">
            每个问题都需要给出一个回答后才能继续。
          </p>
          <div class="flex items-center gap-3">
            <button class="btn btn-ghost" @click="handleReject" :disabled="submitting">
              取消
            </button>
            <button
              class="btn btn-primary"
              :disabled="!canSubmit || submitting"
              @click="handleSubmit"
            >
              提交回答
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import MarkdownRenderer from './MarkdownRenderer.vue'

interface AskUserQuestionOption {
  label: string
  description: string
  preview?: string | null
}

interface AskUserQuestionItem {
  header: string
  question: string
  options: AskUserQuestionOption[]
}

interface PendingAskUserQuestionRequest {
  id: string
  execution_id?: string | null
  questions: AskUserQuestionItem[]
  timestamp: number
}

const props = defineProps<{
  executionId?: string | null
}>()

const pendingRequest = ref<PendingAskUserQuestionRequest | null>(null)
const selectedAnswers = ref<Record<string, string>>({})
const customAnswers = ref<Record<string, string>>({})
const previewFocus = ref<Record<string, string>>({})
const submitting = ref(false)

let unlisten: UnlistenFn | null = null
let pollTimer: ReturnType<typeof setInterval> | null = null

const matchesExecution = (request: PendingAskUserQuestionRequest) => {
  const requestExecutionId = String(request.execution_id || '').trim()
  const currentExecutionId = String(props.executionId || '').trim()
  if (!requestExecutionId || !currentExecutionId) return false
  return requestExecutionId === currentExecutionId
}

const normalizeRequest = (value: unknown): PendingAskUserQuestionRequest | null => {
  if (!value || typeof value !== 'object') return null
  const request = value as PendingAskUserQuestionRequest
  if (!request.id || !Array.isArray(request.questions)) return null
  return request
}

const clearDraft = () => {
  selectedAnswers.value = {}
  customAnswers.value = {}
  previewFocus.value = {}
}

const setPendingRequest = (request: PendingAskUserQuestionRequest | null) => {
  if (pendingRequest.value?.id === request?.id) return
  pendingRequest.value = request
  clearDraft()
}

const pickLatestRelevantRequest = (requests: PendingAskUserQuestionRequest[]) => {
  const relevant = requests.filter(matchesExecution)
  return relevant.length ? relevant[relevant.length - 1] : null
}

const loadPendingQuestions = async () => {
  if (!props.executionId) {
    setPendingRequest(null)
    return
  }

  try {
    const requests = await invoke<PendingAskUserQuestionRequest[]>('get_pending_ask_user_questions')
    setPendingRequest(pickLatestRelevantRequest(requests))
  } catch (error) {
    console.error('[AskUserQuestionModal] Failed to load pending questions:', error)
  }
}

const getAnswerForQuestion = (questionText: string) => {
  const custom = (customAnswers.value[questionText] || '').trim()
  if (custom) return custom
  return (selectedAnswers.value[questionText] || '').trim()
}

const canSubmit = computed(() => {
  if (!pendingRequest.value) return false
  return pendingRequest.value.questions.every((question) => getAnswerForQuestion(question.question))
})

const selectOption = (questionText: string, label: string) => {
  selectedAnswers.value = {
    ...selectedAnswers.value,
    [questionText]: label,
  }
  previewFocus.value = {
    ...previewFocus.value,
    [questionText]: label,
  }
  customAnswers.value = {
    ...customAnswers.value,
    [questionText]: '',
  }
}

const updateCustomAnswer = (questionText: string, value: string) => {
  customAnswers.value = {
    ...customAnswers.value,
    [questionText]: value,
  }
}

const handleCustomAnswerInput = (questionText: string, event: Event) => {
  const target = event.target as HTMLInputElement | null
  updateCustomAnswer(questionText, target?.value || '')
}

const focusPreview = (questionText: string, label: string) => {
  previewFocus.value = {
    ...previewFocus.value,
    [questionText]: label,
  }
}

const resolvePreview = (questionText: string) => {
  const request = pendingRequest.value
  if (!request) return ''
  const question = request.questions.find((item) => item.question === questionText)
  if (!question) return ''
  const preferredLabel =
    previewFocus.value[questionText] ||
    selectedAnswers.value[questionText] ||
    question.options.find((option) => !!option.preview)?.label ||
    ''
  const option = question.options.find((item) => item.label === preferredLabel)
  return option?.preview || ''
}

const handleSubmit = async () => {
  if (!pendingRequest.value || !canSubmit.value || submitting.value) return
  submitting.value = true
  try {
    const answers = Object.fromEntries(
      pendingRequest.value.questions.map((question) => [
        question.question,
        getAnswerForQuestion(question.question),
      ]),
    )
    await invoke('respond_ask_user_question', {
      id: pendingRequest.value.id,
      answers,
    })
    setPendingRequest(null)
  } catch (error) {
    console.error('[AskUserQuestionModal] Failed to submit answers:', error)
  } finally {
    submitting.value = false
  }
}

const handleReject = async () => {
  if (!pendingRequest.value || submitting.value) return
  submitting.value = true
  try {
    await invoke('reject_ask_user_question', {
      id: pendingRequest.value.id,
    })
    setPendingRequest(null)
  } catch (error) {
    console.error('[AskUserQuestionModal] Failed to reject question request:', error)
  } finally {
    submitting.value = false
  }
}

watch(() => props.executionId, () => {
  void loadPendingQuestions()
}, { immediate: true })

onMounted(async () => {
  unlisten = await listen('ask-user-question-request', (event) => {
    const request = normalizeRequest(event.payload)
    if (!request || !matchesExecution(request)) return
    setPendingRequest(request)
  })

  pollTimer = setInterval(() => {
    void loadPendingQuestions()
  }, 2000)
})

onUnmounted(() => {
  if (unlisten) {
    unlisten()
    unlisten = null
  }
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
})
</script>
