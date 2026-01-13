<template>
  <div class="todo-panel h-full flex flex-col bg-base-100" v-if="isActive">
    <div class="todo-header flex items-center gap-2 px-4 py-3 border-b border-base-300">
      <i class="fas fa-tasks text-primary"></i>
      <span class="font-semibold text-base-content">{{ $t('agent.todos') }}</span>
      <span v-if="rootTodos.length > 0" class="badge badge-sm badge-primary">{{ rootTodos.length }}</span>
      <span class="ml-auto text-xs text-success" v-if="progress > 0">{{ progress }}%</span>
      <button 
        @click="$emit('close')"
        class="btn btn-ghost btn-sm btn-square"
        :title="$t('common.close')"
      >
        <i class="fas fa-times"></i>
      </button>
    </div>
    
    <!-- Todo List -->
    <div v-if="hasTodos" class="todo-list flex flex-col gap-1 p-4 overflow-y-auto flex-1">
      <!-- Recursive rendering supports nesting -->
      <TodoItem 
        v-for="todo in rootTodos" 
        :key="todo.id"
        :todo="todo"
        :children="getChildren(todo.id)"
        :get-children="getChildren"
      />
    </div>

    <!-- Empty State -->
    <div v-else class="flex-1 flex flex-col items-center justify-center text-base-content/60 p-8">
      <div class="avatar placeholder mb-4">
        <div class="bg-base-200 text-base-content/40 rounded-full w-16 flex items-center justify-center">
          <i class="fas fa-tasks text-2xl"></i>
        </div>
      </div>
      <h3 class="text-base font-semibold mb-2 text-base-content/80">{{ $t('agent.noTodos') }}</h3>
      <p class="text-sm text-center max-w-xs text-base-content/60">{{ $t('agent.todosWillAppearHere') }}</p>
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
