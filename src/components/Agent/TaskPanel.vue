<template>
  <div class="task-panel h-full flex flex-col bg-base-100" v-if="isActive">
    <div class="task-header flex items-center gap-2 px-4 py-3 border-b border-base-300">
      <i class="fas fa-list-check text-primary"></i>
      <span class="font-semibold text-base-content">{{ $t('agent.tasks') }}</span>
      <span v-if="rootTasks.length > 0" class="badge badge-sm badge-primary">{{ rootTasks.length }}</span>
      <select
        v-if="showSourceSelector"
        v-model="selectedSourceModel"
        class="select select-xs select-bordered ml-2 max-w-[180px]"
      >
        <option v-for="option in sourceOptions" :key="option.key" :value="option.key">
          {{ option.label }} ({{ option.count }})
        </option>
      </select>
      <span class="ml-auto text-xs text-success" v-if="progress > 0">{{ progress }}%</span>
      <button
        @click="$emit('close')"
        class="btn btn-ghost btn-sm btn-square"
        :title="$t('common.close')"
      >
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div v-if="hasTasks" class="task-list flex flex-col gap-1 p-4 overflow-y-auto flex-1">
      <TaskItem
        v-for="task in rootTasks"
        :key="task.id"
        :task="task"
        :children="getChildren(task.id)"
        :get-children="getChildren"
      />
    </div>

    <div v-else class="flex-1 flex flex-col items-center justify-center text-base-content/60 p-8">
      <div class="avatar placeholder mb-4">
        <div class="bg-base-200 text-base-content/40 rounded-full w-16 flex items-center justify-center">
          <i class="fas fa-list-check text-2xl"></i>
        </div>
      </div>
      <h3 class="text-base font-semibold mb-2 text-base-content/80">{{ $t('agent.noTasks') }}</h3>
      <p class="text-sm text-center max-w-xs text-base-content/60">{{ $t('agent.tasksWillAppearHere') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AgentTask } from '@/types/agentTask'
import {
  calculateAgentTaskProgress,
  getChildAgentTasks,
  getRootAgentTasks,
} from '@/types/agentTask'
import TaskItem from './TaskItem.vue'

interface TaskSourceOption {
  key: string
  label: string
  count: number
}

const props = defineProps<{
  tasks: AgentTask[]
  isActive?: boolean
  sourceOptions?: TaskSourceOption[]
  selectedSourceKey?: string
}>()

const emit = defineEmits<{
  close: []
  sourceChange: [sourceKey: string]
}>()

const showSourceSelector = computed(() => (props.sourceOptions?.length || 0) > 1)
const sourceOptions = computed(() => props.sourceOptions || [])
const selectedSourceModel = computed({
  get: () => props.selectedSourceKey || sourceOptions.value[0]?.key || '',
  set: (value: string) => emit('sourceChange', value),
})
const rootTasks = computed(() => getRootAgentTasks(props.tasks))
const hasTasks = computed(() => props.tasks.length > 0)
const progress = computed(() => calculateAgentTaskProgress(props.tasks))

const getChildren = (parentId: string): AgentTask[] => {
  return getChildAgentTasks(props.tasks, parentId)
}
</script>
