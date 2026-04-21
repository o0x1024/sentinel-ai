import { computed, ref, watch, type MaybeRefOrGetter, toValue } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { AgentMessage } from '@/types/agent'
import { clearFocusLocationQuery } from './focusLocationSupport'
import { buildFocusedMemoryToolsRoute, deriveFocusBannerState } from './focusBannerSupport'
import { resolveFocusedMemoryMessageId } from './memoryFocusSupport'

const normalizeText = (value: unknown): string => String(value || '').trim()

export const useAgentMessageFocus = (params: {
  focusedMemoryId: MaybeRefOrGetter<string | null | undefined>
  focusedMessageId: MaybeRefOrGetter<string | null | undefined>
  visibleMessages: MaybeRefOrGetter<AgentMessage[]>
  emitMemoryMessageFocused: (payload: { memoryId: string; messageId: string }) => void
  focusTeamTaskInWorkspace: (taskId: string) => void
}) => {
  const route = useRoute()
  const router = useRouter()

  const focusedMemoryMessageId = ref<string | null>(null)
  const normalizedFocusedMemoryId = computed(() => normalizeText(toValue(params.focusedMemoryId)))
  const normalizedFocusedMessageId = computed(() => normalizeText(toValue(params.focusedMessageId)))
  const lastFocusedMemoryId = ref<string | null>(normalizedFocusedMemoryId.value || null)

  const focusBannerState = computed(() => deriveFocusBannerState({
    focusedMemoryId: normalizedFocusedMemoryId.value,
    focusedMessageId: normalizedFocusedMessageId.value,
    resolvedMessageId: focusedMemoryMessageId.value,
    lastFocusedMemoryId: lastFocusedMemoryId.value,
  }))
  const focusBannerMemoryId = computed(() => focusBannerState.value.memoryId)
  const isFocusBannerVisible = computed(() => focusBannerState.value.visible)

  watch(
    () => ({
      messageId: normalizedFocusedMessageId.value,
      memoryId: normalizedFocusedMemoryId.value,
      messages: toValue(params.visibleMessages),
    }),
    ({ messageId, memoryId, messages }) => {
      if (messageId) {
        focusedMemoryMessageId.value = messageId
        return
      }
      focusedMemoryMessageId.value = memoryId
        ? resolveFocusedMemoryMessageId(messages, memoryId)
        : null
    },
    { immediate: true },
  )

  watch(focusBannerState, (state) => {
    lastFocusedMemoryId.value = state.nextLastFocusedMemoryId
  }, { immediate: true })

  const handleFocusedMessage = (messageId: string) => {
    const memoryId = normalizedFocusedMemoryId.value
    if (!memoryId) return
    params.emitMemoryMessageFocused({ memoryId, messageId })
  }

  const handleFocusTeamTask = (taskId: string) => {
    const normalizedTaskId = normalizeText(taskId)
    if (!normalizedTaskId) return
    params.focusTeamTaskInWorkspace(normalizedTaskId)
  }

  const clearFocusedLocation = () => {
    const nextQuery = clearFocusLocationQuery(route.query)
    void router.replace({ query: nextQuery })
  }

  const openFocusedMemoryInTools = () => {
    const target = buildFocusedMemoryToolsRoute(focusBannerMemoryId.value)
    if (!target) return
    void router.push(target)
  }

  return {
    clearFocusedLocation,
    focusBannerMemoryId,
    focusedMemoryMessageId,
    handleFocusedMessage,
    handleFocusTeamTask,
    isFocusBannerVisible,
    openFocusedMemoryInTools,
  }
}
