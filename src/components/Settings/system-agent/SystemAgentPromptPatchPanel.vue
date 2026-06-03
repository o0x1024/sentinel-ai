<template>
  <div class="space-y-4">
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <label class="form-control">
        <span class="label-text">名称</span>
        <input :value="name" class="input input-bordered" disabled />
        <span class="label-text-alt text-base-content/60 mt-1">
          内置智能体名称固定，不建议修改。
        </span>
      </label>
    </div>

    <label class="form-control">
      <span class="label-text">Prompt Patch</span>
      <textarea
        :value="promptPatch"
        class="textarea textarea-bordered h-28 font-mono"
        :placeholder="placeholder"
        @input="handleInput"
      ></textarea>
      <span class="label-text-alt text-base-content/60 mt-1 whitespace-pre-line">
        {{ guidance }}
      </span>
    </label>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  name: string
  promptPatch: string
  placeholder: string
  guidance: string
}>()

const emit = defineEmits<{
  'update:promptPatch': [value: string]
}>()

function handleInput(event: Event) {
  const target = event.target as HTMLTextAreaElement | null
  emit('update:promptPatch', target?.value || '')
}
</script>
