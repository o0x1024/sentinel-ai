<template>
  <div v-if="actions.length > 0" class="message-action-bar">
    <button
      v-for="action in actions"
      :key="action.key"
      type="button"
      class="message-action-pill"
      :class="action.variant === 'primary' ? 'message-action-pill-primary' : 'message-action-pill-secondary'"
      :title="action.label"
      @click="emit('action', action.key)"
    >
      <i :class="action.icon"></i>
      <span>{{ action.label }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
export type MessageActionKey = 'copy' | 'edit' | 'resend' | 'toggle-details'

export interface MessageActionItem {
  key: MessageActionKey
  label: string
  icon: string
  variant?: 'primary' | 'secondary'
}

defineProps<{
  actions: MessageActionItem[]
}>()

const emit = defineEmits<{
  (e: 'action', key: MessageActionKey): void
}>()
</script>

<style scoped>
.message-action-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 0.9rem;
  padding-top: 0.8rem;
  border-top: 1px solid rgba(15, 23, 42, 0.08);
}

.message-action-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.38rem;
  border-radius: 999px;
  padding: 0.42rem 0.72rem;
  font-size: 0.74rem;
  font-weight: 600;
  border: 1px solid rgba(15, 23, 42, 0.08);
  transition: background-color 0.16s ease, border-color 0.16s ease, color 0.16s ease;
}

.message-action-pill i {
  font-size: 0.72rem;
}

.message-action-pill-secondary {
  background: rgba(255, 255, 255, 0.62);
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 66%, transparent);
}

.message-action-pill-secondary:hover {
  background: rgba(255, 255, 255, 0.88);
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 84%, transparent);
}

.message-action-pill-primary {
  background: rgba(59, 130, 246, 0.08);
  color: rgb(37, 99, 235);
  border-color: rgba(59, 130, 246, 0.16);
}

.message-action-pill-primary:hover {
  background: rgba(59, 130, 246, 0.14);
}
</style>
