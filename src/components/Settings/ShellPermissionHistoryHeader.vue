<template>
  <div class="flex items-start justify-between gap-4">
    <div>
      <h3 class="card-title">
        <i class="fas fa-shield-halved"></i>
        {{ t('settings.agent.permissionHistory.title') }}
      </h3>
      <p class="text-sm text-base-content/70 mt-1">
        {{ t('settings.agent.permissionHistory.desc') }}
      </p>
    </div>
    <button class="btn btn-sm btn-ghost" :disabled="loading" @click="$emit('refresh')">
      <span v-if="loading" class="loading loading-spinner loading-xs mr-2"></span>
      <i v-else class="fas fa-rotate mr-2"></i>
      {{ t('settings.agent.permissionHistory.refresh') }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  loading: boolean
}>()

defineEmits<{
  refresh: []
}>()

const { t } = useI18n()
</script>
