<template>
  <div class="page-content-full h-full min-h-0">
    <AIAssistantWorkspace presentation-target="page" :active="true" />
  </div>
</template>

<script setup lang="ts">
import { onActivated, onDeactivated, onMounted, onUnmounted } from 'vue'
import AIAssistantWorkspace from '@/components/assistant/AIAssistantWorkspace.vue'
import {
  claimAssistantPresentationTarget,
  releaseAssistantPresentationTarget,
} from '@/services/assistantPresentation'

defineOptions({
  name: 'AIAssistant',
})

function claimPageAssistant() {
  claimAssistantPresentationTarget('page')
}

function releasePageAssistant() {
  releaseAssistantPresentationTarget('page')
}

onMounted(claimPageAssistant)
onActivated(claimPageAssistant)
onDeactivated(releasePageAssistant)
onUnmounted(releasePageAssistant)
</script>
