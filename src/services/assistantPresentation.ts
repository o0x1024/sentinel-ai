import { readonly, ref } from 'vue'

export type AssistantPresentationTarget = 'page' | 'traffic'

const activeAssistantPresentationTargetState = ref<AssistantPresentationTarget | null>(null)

export const activeAssistantPresentationTarget = readonly(activeAssistantPresentationTargetState)

export function claimAssistantPresentationTarget(target: AssistantPresentationTarget) {
  activeAssistantPresentationTargetState.value = target
}

export function releaseAssistantPresentationTarget(target: AssistantPresentationTarget) {
  if (activeAssistantPresentationTargetState.value === target) {
    activeAssistantPresentationTargetState.value = null
  }
}

export function isAssistantPresentationTarget(target: AssistantPresentationTarget) {
  return activeAssistantPresentationTargetState.value === target
}
