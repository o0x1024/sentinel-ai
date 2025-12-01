<template>
  <div class="searchable-select relative" ref="containerRef">
    <div 
      class="input input-bordered w-full flex items-center cursor-pointer h-12"
      :class="{ 'opacity-50 cursor-not-allowed': disabled }"
      @click="toggleDropdown"
    >
      <span class="flex-1 truncate">{{ displayValue || placeholder }}</span>
      <i class="fas fa-chevron-down text-xs ml-2 transition-transform" :class="{ 'rotate-180': isOpen }"></i>
    </div>
    
    <!-- 下拉面板 -->
    <div 
      v-if="isOpen && !disabled"
      class="absolute z-50 top-full left-0 right-0 mt-1 bg-base-100 border border-base-300 rounded-box shadow-lg max-h-64 overflow-hidden flex flex-col"
    >
      <!-- 搜索框 -->
      <div class="p-2 border-b border-base-300">
        <input
          ref="searchInputRef"
          type="text"
          class="input input-sm input-bordered w-full"
          :placeholder="searchPlaceholder"
          v-model="searchQuery"
          @keydown.down.prevent="navigateDown"
          @keydown.up.prevent="navigateUp"
          @keydown.enter.prevent="selectHighlighted"
          @keydown.escape="closeDropdown"
        />
      </div>
      
      <!-- 选项列表 -->
      <div class="overflow-y-auto flex-1">
        <div
          v-for="(option, index) in filteredOptions"
          :key="option.value"
          class="px-3 py-2 cursor-pointer transition-colors flex items-center gap-2"
          :class="{
            'bg-primary text-primary-content': option.value === modelValue,
            'bg-base-200': highlightedIndex === index && option.value !== modelValue,
            'hover:bg-base-200': option.value !== modelValue
          }"
          @click="selectOption(option)"
          @mouseenter="highlightedIndex = index"
        >
          <span class="flex-1 truncate">{{ option.label }}</span>
          <span v-if="option.description" class="text-xs opacity-70 truncate max-w-32">{{ option.description }}</span>
        </div>
        
        <!-- 无匹配结果 -->
        <div v-if="filteredOptions.length === 0" class="px-3 py-4 text-center text-base-content/50">
          <i class="fas fa-search mb-2"></i>
          <p>{{ noResultsText }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'

interface Option {
  value: string
  label: string
  description?: string
}

interface Props {
  modelValue: string
  options: Option[]
  placeholder?: string
  searchPlaceholder?: string
  noResultsText?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '请选择',
  searchPlaceholder: '搜索...',
  noResultsText: '无匹配结果',
  disabled: false
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'change': [value: string]
}>()

const containerRef = ref<HTMLElement>()
const searchInputRef = ref<HTMLInputElement>()
const isOpen = ref(false)
const searchQuery = ref('')
const highlightedIndex = ref(0)

// 计算显示值
const displayValue = computed(() => {
  const option = props.options.find(o => o.value === props.modelValue)
  return option ? option.label : ''
})

// 过滤选项
const filteredOptions = computed(() => {
  if (!searchQuery.value) {
    return props.options
  }
  const query = searchQuery.value.toLowerCase()
  return props.options.filter(option => 
    option.label.toLowerCase().includes(query) ||
    option.value.toLowerCase().includes(query) ||
    (option.description && option.description.toLowerCase().includes(query))
  )
})

// 打开/关闭下拉
const toggleDropdown = () => {
  if (props.disabled) return
  isOpen.value = !isOpen.value
  if (isOpen.value) {
    searchQuery.value = ''
    highlightedIndex.value = 0
    nextTick(() => {
      searchInputRef.value?.focus()
    })
  }
}

const closeDropdown = () => {
  isOpen.value = false
}

// 选择选项
const selectOption = (option: Option) => {
  emit('update:modelValue', option.value)
  emit('change', option.value)
  closeDropdown()
}

// 键盘导航
const navigateDown = () => {
  if (highlightedIndex.value < filteredOptions.value.length - 1) {
    highlightedIndex.value++
  }
}

const navigateUp = () => {
  if (highlightedIndex.value > 0) {
    highlightedIndex.value--
  }
}

const selectHighlighted = () => {
  if (filteredOptions.value.length > 0) {
    selectOption(filteredOptions.value[highlightedIndex.value])
  }
}

// 点击外部关闭
const handleClickOutside = (event: MouseEvent) => {
  if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
    closeDropdown()
  }
}

// 搜索变化时重置高亮索引
watch(searchQuery, () => {
  highlightedIndex.value = 0
})

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.searchable-select .input {
  display: flex;
  align-items: center;
}
</style>

