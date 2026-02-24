<template>
  <div class="bg-base-200/30 rounded-lg p-4 space-y-3">
    <!-- Section Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="badge badge-sm" :class="badgeColor">{{ title }}</span>
        <span class="text-xs text-base-content/50">({{ patterns.length }})</span>
      </div>
      <button class="btn btn-xs btn-ghost gap-1" @click="$emit('add')">
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        {{ $t('agent.addPattern') }}
      </button>
    </div>
    <p v-if="hint" class="text-xs text-base-content/50">{{ hint }}</p>

    <!-- Pattern Rows -->
    <div v-if="patterns.length === 0" class="text-xs text-base-content/40 py-2 text-center">
      暂无模式，点击添加
    </div>
    <div
      v-for="(pattern, idx) in patterns"
      :key="idx"
      class="flex items-start gap-2 bg-base-100 rounded-md p-2 border border-base-300"
    >
      <div class="flex-1 grid grid-cols-1 md:grid-cols-3 gap-2">
        <div class="form-control">
          <input
            v-model="pattern.name_pattern"
            type="text"
            class="input input-xs input-bordered font-mono"
            :placeholder="$t('agent.patternNamePlaceholder')"
          />
        </div>
        <div class="form-control">
          <input
            :value="pattern.languages.join(', ')"
            type="text"
            class="input input-xs input-bordered"
            :placeholder="$t('agent.patternLanguagesPlaceholder')"
            @input="updateLanguages(pattern, ($event.target as HTMLInputElement).value)"
          />
        </div>
        <div class="form-control">
          <input
            v-model="pattern.description"
            type="text"
            class="input input-xs input-bordered"
            :placeholder="$t('agent.patternDescription')"
          />
        </div>
      </div>
      <button
        class="btn btn-xs btn-ghost text-error flex-shrink-0"
        @click="$emit('remove', idx)"
        :title="$t('agent.removePattern')"
      >
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
interface PatternSpec {
  name_pattern: string
  arg_pattern?: string
  languages: string[]
  description?: string
}

defineProps<{
  title: string
  hint?: string
  patterns: PatternSpec[]
  badgeColor?: string
  iconClass?: string
}>()

defineEmits<{
  (e: 'add'): void
  (e: 'remove', index: number): void
}>()

function updateLanguages(pattern: PatternSpec, value: string) {
  pattern.languages = value
    ? value.split(',').map((l) => l.trim()).filter(Boolean)
    : []
}
</script>
