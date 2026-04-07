<template>
  <div class="collapse collapse-arrow border border-base-300 bg-base-100">
    <input type="checkbox" />
    <div class="collapse-title text-sm font-semibold">高级设置</div>
    <div class="collapse-content">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 pt-2">
        <div class="rounded-lg border border-base-300 bg-base-200/40 p-3 lg:col-span-2">
          <div class="text-sm font-medium">内部标识</div>
          <div class="mt-2 grid grid-cols-1 lg:grid-cols-2 gap-3 text-xs text-base-content/70">
            <div>
              <div class="font-medium text-base-content/80">智能体 ID</div>
              <div class="font-mono break-all mt-1">{{ profileId }}</div>
            </div>
            <div>
              <div class="font-medium text-base-content/80">基础模板</div>
              <div class="font-mono break-all mt-1">{{ basePromptId || profileId }}</div>
            </div>
          </div>
        </div>

        <label class="form-control">
          <span class="label-text">Cooldown 秒数</span>
          <input
            :value="cooldownSecs"
            type="number"
            min="0"
            class="input input-bordered"
            @input="handleCooldownInput"
          />
          <span class="label-text-alt text-base-content/60 mt-1">
            控制同一个 Agent 两次运行之间的最短间隔。
          </span>
        </label>

        <label class="form-control">
          <span class="label-text">最大并发</span>
          <input
            :value="maxConcurrency"
            type="number"
            min="1"
            class="input input-bordered"
            @input="handleConcurrencyInput"
          />
          <span class="label-text-alt text-base-content/60 mt-1">
            限制同一个 Agent 同时运行的实例数量。
          </span>
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  profileId: string
  basePromptId?: string | null
  cooldownSecs: number
  maxConcurrency: number
}>()

const emit = defineEmits<{
  'update:cooldownSecs': [value: number]
  'update:maxConcurrency': [value: number]
}>()

function handleCooldownInput(event: Event) {
  const target = event.target as HTMLInputElement | null
  emit('update:cooldownSecs', Number(target?.value || 0))
}

function handleConcurrencyInput(event: Event) {
  const target = event.target as HTMLInputElement | null
  emit('update:maxConcurrency', Number(target?.value || 1))
}
</script>
