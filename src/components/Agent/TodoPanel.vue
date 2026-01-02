<template>
  <div class="todo-panel border border-base-300 rounded-lg p-3 my-4 bg-base-200" v-if="isActive && hasTodos">
    <div class="todo-header flex items-center gap-2 text-sm font-semibold text-base-content/70 mb-2">
      <span class="todo-title font-bold">📋 To-dos</span>
      <span class="todo-count bg-base-300 px-1.5 py-0.5 rounded text-xs">{{ rootTodos.length }}</span>
      <span class="todo-progress ml-auto text-xs text-success" v-if="progress > 0">{{ progress }}%</span>
      <button 
        @click="$emit('close')"
        class="btn btn-ghost btn-xs ml-2"
        title="Close"
      >
        <i class="fas fa-times"></i>
      </button>
    </div>
    
    <div class="todo-list flex flex-col gap-1">
      <!-- Recursive rendering supports nesting -->
      <TodoItem 
        v-for="todo in rootTodos" 
        :key="todo.id"
        :todo="todo"
        :children="getChildren(todo.id)"
        :get-children="getChildren"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, toRef } from 'vue'
import type { Todo } from '@/types/todo'
import { getRootTodos, getChildTodos, calculateProgress } from '@/types/todo'
import TodoItem from './TodoItem.vue'

const props = defineProps<{
  todos: Todo[]
  isActive?: boolean
}>()

defineEmits<{
  close: []
}>()

// Top-level tasks (no parent_id)
const rootTodos = computed(() => getRootTodos(props.todos))

// Whether there are todos
const hasTodos = computed(() => props.todos.length > 0)

// 完成进度
const progress = computed(() => calculateProgress(props.todos))

// 获取某个任务的子任务
const getChildren = (parentId: string): Todo[] => {
  return getChildTodos(props.todos, parentId)
}
</script>

<style scoped>
/* No custom styles needed - using Tailwind utilities */
</style>
