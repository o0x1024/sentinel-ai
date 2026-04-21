<template>
  <div class="task-item-wrapper flex flex-col">
    <div :class="['task-item flex items-center gap-2 text-sm py-1 leading-snug', rowClass]">
      <span :class="['task-indicator w-4 text-center font-bold flex-shrink-0', indicatorClass]">{{ indicator }}</span>
      <span :class="['task-content flex-1 break-words', contentClass]">{{ displayText }}</span>
      <span v-if="childrenCount > 0" class="task-children-count text-xs text-base-content/60 flex-shrink-0">
        {{ childrenCount }}
      </span>
    </div>

    <TaskMetaList v-if="metaItems.length > 0" :items="metaItems" class="ml-6 mt-1" />

    <div v-if="childrenCount > 0" class="task-children ml-5 pl-2 border-l border-base-300">
      <TaskItem
        v-for="child in children"
        :key="child.id"
        :task="child"
        :children="getChildren(child.id)"
        :get-children="getChildren"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AgentTask } from '@/types/agentTask'
import { getAgentTaskDisplayText } from '@/types/agentTask'
import TaskMetaList from './TaskMetaList.vue'
import { buildAgentTaskMetaItems } from './taskMetaPresentation'
import {
  getAgentTaskContentClass,
  getAgentTaskIndicator,
  getAgentTaskIndicatorClass,
  getAgentTaskRowClass,
} from './taskPresentation'

const props = defineProps<{
  task: AgentTask
  children: AgentTask[]
  getChildren: (parentId: string) => AgentTask[]
}>()

const indicator = computed(() => getAgentTaskIndicator(props.task.status))
const indicatorClass = computed(() => getAgentTaskIndicatorClass(props.task.status))
const contentClass = computed(() => getAgentTaskContentClass(props.task.status))
const rowClass = computed(() => getAgentTaskRowClass(props.task.status))
const displayText = computed(() => getAgentTaskDisplayText(props.task))
const childrenCount = computed(() => props.children.length)
const metaItems = computed(() => buildAgentTaskMetaItems(props.task))
</script>
