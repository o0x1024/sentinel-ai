<template>
  <AppDialog :class="['modal', { 'modal-open': modelValue }]">
    <div
      v-if="modelValue"
      class="modal-box w-11/12 max-w-7xl h-[85vh] flex flex-col p-0"
    >
      <div class="flex items-center justify-between gap-3 border-b border-base-300 bg-base-200 px-4 py-3">
        <div>
          <h3 class="font-bold text-lg flex items-center gap-2">
            <i class="fas fa-terminal text-primary"></i>
            <span>Browser Shell Bridge</span>
          </h3>
          <p class="text-sm text-base-content/70 mt-1">
            管理第三方网页终端会话、绑定当前对话并控制 AI 写入权限。
          </p>
        </div>
        <button class="btn btn-sm btn-ghost btn-circle" @click="close">
          <i class="fas fa-times text-lg"></i>
        </button>
      </div>

      <div class="flex-1 overflow-hidden p-4">
        <div class="h-full overflow-y-auto pr-1">
          <BrowserShellBridgePanel
            :show-tool-test-actions="showToolTestActions"
            @open-tool-test="forwardOpenToolTest"
          />
        </div>
      </div>
    </div>

    <form method="dialog" class="modal-backdrop" @click="close">
      <button>close</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import AppDialog from '@/components/AppDialog.vue'
import BrowserShellBridgePanel from './BrowserShellBridgePanel.vue'

const props = withDefaults(defineProps<{
  modelValue: boolean
  showToolTestActions?: boolean
}>(), {
  showToolTestActions: true,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'open-tool-test', initialParams: Record<string, unknown>): void
}>()

function close() {
  emit('update:modelValue', false)
}

function forwardOpenToolTest(initialParams: Record<string, unknown>) {
  emit('open-tool-test', initialParams)
}
</script>
