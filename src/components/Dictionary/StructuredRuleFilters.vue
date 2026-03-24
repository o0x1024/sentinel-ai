<template>
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
    <div class="form-control">
      <label class="label">
        <span class="label-text">分类筛选</span>
      </label>
      <select
        :value="category"
        class="select select-bordered"
        @change="emit('update:category', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">全部分类</option>
        <option v-for="option in categoryOptions" :key="option" :value="option">
          {{ option }}
        </option>
      </select>
    </div>

    <div class="form-control">
      <label class="label">
        <span class="label-text">风险等级</span>
      </label>
      <select
        :value="severity"
        class="select select-bordered"
        @change="emit('update:severity', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">全部等级</option>
        <option v-for="option in severityOptions" :key="option" :value="option">
          {{ option }}
        </option>
      </select>
    </div>

    <div class="form-control">
      <label class="label">
        <span class="label-text">匹配器</span>
      </label>
      <select
        :value="matcher"
        class="select select-bordered"
        @change="emit('update:matcher', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">全部</option>
        <option value="with_matchers">仅有匹配器</option>
        <option value="without_matchers">无匹配器</option>
      </select>
    </div>

    <div class="form-control">
      <label class="label">
        <span class="label-text">规则状态</span>
      </label>
      <select
        :value="enabled"
        class="select select-bordered"
        @change="emit('update:enabled', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">全部状态</option>
        <option value="enabled">仅启用</option>
        <option value="disabled">仅禁用</option>
      </select>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  category: string
  severity: string
  matcher: string
  enabled: string
  categoryOptions: string[]
  severityOptions: string[]
}>()

const emit = defineEmits<{
  (e: 'update:category', value: string): void
  (e: 'update:severity', value: string): void
  (e: 'update:matcher', value: string): void
  (e: 'update:enabled', value: string): void
}>()
</script>
