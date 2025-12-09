<template>
  <div class="todo-item-wrapper flex flex-col">
    <!-- 当前任务 -->
    <div :class="['todo-item flex items-center gap-2 text-sm py-1 leading-snug', statusClass]">
      <span :class="['todo-indicator w-4 text-center font-bold flex-shrink-0', indicatorClass]">{{ indicator }}</span>
      <span :class="['todo-content flex-1 break-words', contentClass]">{{ displayText }}</span>
      <span v-if="childrenCount > 0" class="todo-children-count text-xs text-base-content/60 flex-shrink-0">
        ({{ completedChildrenCount }}/{{ childrenCount }})
      </span>
    </div>
    
    <!-- 子任务（递归） -->
    <div v-if="childrenCount > 0" class="todo-children ml-5 pl-2 border-l border-base-300">
      <TodoItem 
        v-for="child in children" 
        :key="child.id"
        :todo="child"
        :children="getChildren(child.id)"
        :get-children="getChildren"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Todo, TodoStatus } from '@/types/todo'
import { getTodoIndicator, getTodoDisplayText } from '@/types/todo'

const props = defineProps<{
  todo: Todo
  children: Todo[]
  getChildren: (parentId: string) => Todo[]
}>()

// 状态指示符
const indicator = computed(() => getTodoIndicator(props.todo.status))

// 显示文本（如果正在执行，使用 active_form）
const displayText = computed(() => getTodoDisplayText(props.todo))

// 子任务数量
const childrenCount = computed(() => props.children.length)

// 已完成的子任务数
const completedChildrenCount = computed(() => 
  props.children.filter(c => c.status === 'completed').length
)

// 状态样式类
const statusClass = computed(() => {
  switch (props.todo.status) {
    case 'pending': return 'status-pending'
    case 'in_progress': return 'status-in_progress'
    case 'completed': return 'status-completed'
    default: return ''
  }
})

// 指示符颜色
const indicatorClass = computed(() => {
  switch (props.todo.status) {
    case 'pending': return 'text-base-content/60'
    case 'in_progress': return 'text-primary'
    case 'completed': return 'text-success'
    default: return ''
  }
})

// 内容样式
const contentClass = computed(() => {
  switch (props.todo.status) {
    case 'in_progress': return 'text-base-content'
    case 'completed': return 'line-through text-base-content/60'
    default: return 'text-base-content/80'
  }
})
</script>

<style scoped>
/* No custom styles needed - using Tailwind utilities */
</style>
