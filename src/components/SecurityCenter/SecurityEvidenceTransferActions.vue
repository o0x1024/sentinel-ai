<template>
  <div v-if="canShow" :class="dropdownClassName" @click.stop>
    <button ref="triggerRef" tabindex="0" type="button" :class="buttonClassName">
      {{ messages.triggerLabel }}
      <i class="fas fa-chevron-down text-[10px] opacity-70"></i>
    </button>
    <ul
      tabindex="0"
      class="dropdown-content menu z-[70] mt-2 w-48 rounded-box border border-base-300 bg-base-100 p-2 shadow-lg"
    >
      <li v-if="canTransfer">
        <button @click.stop="sendRequest('repeater')">
          {{ messages.sendToRepeater }}
        </button>
      </li>
      <li v-if="canTransfer">
        <button @click.stop="sendRequest('intruder')">
          {{ messages.sendToIntruder }}
        </button>
      </li>
      <li v-if="finding">
        <button @click.stop="sendFindingToAssistant">
          {{ messages.sendToAssistant }}
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { dialog } from '@/composables/useDialog'
import {
  buildHttpExchangeRequestFromSecurityEvidence,
  openSecurityEvidenceInTrafficWorkbench,
  type SecurityEvidenceTransferMessages,
  type SecurityEvidenceTransferTarget,
} from './securityEvidenceTransferSupport'
import { queueSecurityFindingsForAssistant } from './securityFindingAssistantTransfer'
import type { Evidence, Finding } from './vulnerabilityFindingTypes'

const props = withDefaults(
  defineProps<{
    evidence?: Evidence | null
    finding?: Finding | null
    size?: 'xs' | 'sm'
    direction?: 'up' | 'down'
    messages: SecurityEvidenceTransferMessages
  }>(),
  {
    evidence: null,
    finding: null,
    size: 'xs',
    direction: 'up',
  }
)

const router = useRouter()
const triggerRef = ref<HTMLButtonElement | null>(null)

const canTransfer = computed(() =>
  props.evidence ? Boolean(buildHttpExchangeRequestFromSecurityEvidence(props.evidence)) : false
)
const canShow = computed(() => canTransfer.value || Boolean(props.finding))

const dropdownClassName = computed(
  () => `dropdown dropdown-end ${props.direction === 'down' ? 'dropdown-bottom' : 'dropdown-top'}`
)

const buttonClassName = computed(() => `btn btn-outline btn-${props.size} gap-2 whitespace-nowrap`)

const formatTransferError = (error: unknown) =>
  props.messages.transferFailed.replace('{error}', String(error))

const closeDropdown = () => {
  triggerRef.value?.blur()
}

const sendRequest = async (target: SecurityEvidenceTransferTarget) => {
  if (!props.evidence) {
    return
  }

  closeDropdown()

  try {
    const handled = await openSecurityEvidenceInTrafficWorkbench(router, props.evidence, target)
    if (!handled) {
      dialog.toast.warning(props.messages.noTransferableRequest)
      return
    }

    dialog.toast.success(
      target === 'repeater' ? props.messages.repeaterOpened : props.messages.intruderOpened
    )
  } catch (error) {
    console.error('Failed to send security evidence request:', error)
    dialog.toast.error(formatTransferError(error))
  }
}

const sendFindingToAssistant = async () => {
  if (!props.finding) {
    return
  }

  closeDropdown()

  try {
    queueSecurityFindingsForAssistant([props.finding])
    await router.push('/ai-assistant')
    dialog.toast.success(props.messages.assistantOpened)
  } catch (error) {
    console.error('Failed to send security finding to AI assistant:', error)
    dialog.toast.error(props.messages.assistantFailed)
  }
}
</script>
