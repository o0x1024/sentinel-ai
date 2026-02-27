<template>
  <div class="searchable-select relative" ref="containerRef">
    <div 
      :class="[triggerClass, controlClass, { 'opacity-50 cursor-not-allowed': disabled }]"
      @click="toggleDropdown"
    >
      <i v-if="props.variant === 'toolbar'" class="fas fa-sparkles text-primary mr-2 mb-[1px]"></i>
      <span class="flex-1 truncate" :title="displayValue || placeholder">
        {{ displayValue || placeholder }}
      </span>
      <i class="fas fa-chevron-down text-[10px] ml-1.5 opacity-60 transition-transform" :class="{ 'rotate-180': isOpen }"></i>
    </div>
    
    <!-- 下拉面板 -->
    <div 
      v-if="isOpen && !disabled"
      class="absolute z-50 border rounded-xl shadow-2xl max-h-80 overflow-hidden flex flex-col"
      :class="[panelPositionClass, panelVariantClass]"
      :style="panelStyle"
    >
      <!-- 搜索框 (无边框样式) -->
      <div class="px-3 py-2 border-b border-base-300/50 flex items-center gap-2">
        <i class="fas fa-search text-base-content/40 text-xs"></i>
        <input
          ref="searchInputRef"
          type="text"
          class="bg-transparent border-none outline-none w-full text-sm placeholder:text-base-content/40 focus:ring-0 p-0 h-7"
          :placeholder="searchPlaceholder"
          v-model="searchQuery"
          @keydown.down.prevent="navigateDown"
          @keydown.up.prevent="navigateUp"
          @keydown.enter.prevent="selectHighlighted"
          @keydown.escape="closeDropdown"
        />
      </div>
      
      <!-- 选项列表 -->
      <div class="overflow-y-auto flex-1 p-1">
        <template v-for="(option, index) in filteredOptions" :key="option.value">
          <!-- 分组标题 -->
          <div v-if="option.isGroupHeader" class="px-2 pt-2 pb-1 mt-1 mb-1 text-[11px] font-semibold text-base-content/40 uppercase tracking-wider">
            {{ option.label }}
          </div>
          
          <!-- 实际选项 -->
          <div v-else
            class="px-2 py-1.5 my-0.5 rounded-lg cursor-pointer transition-colors flex flex-nowrap items-center gap-2 text-sm"
            :class="getOptionClass(option.value, index)"
            @click="selectOption(option)"
            @mouseenter="highlightedIndex = index"
          >
            <span class="flex-1 truncate" :title="option.label">
              {{ option.label }}
            </span>
            <span v-if="option.description && props.groupBy !== 'description'" class="text-xs opacity-50 whitespace-nowrap ml-2">
              {{ option.description }}
            </span>
            <!-- Checked Indicator -->
            <i v-if="option.value === props.modelValue" class="fas fa-check text-xs ml-auto pl-2" :class="props.variant === 'toolbar' ? 'text-primary' : 'text-primary-content'"></i>
            <span v-else class="w-3 ml-auto pl-2"></span>
          </div>
        </template>
        
        <!-- 无匹配结果 -->
        <div v-if="filteredOptions.length === 0" class="px-3 py-6 text-center text-base-content/40 text-sm">
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
  isGroupHeader?: boolean
}

interface Props {
  modelValue: string
  options: Option[]
  placeholder?: string
  searchPlaceholder?: string
  noResultsText?: string
  disabled?: boolean
  size?: 'md' | 'sm'
  direction?: 'up' | 'down'
  variant?: 'default' | 'toolbar'
  autoWidth?: boolean
  align?: 'left' | 'right' | 'justify'
  groupBy?: 'description' | false
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '请选择',
  searchPlaceholder: '搜索...',
  noResultsText: '无匹配结果',
  disabled: false,
  size: 'md',
  direction: 'down',
  variant: 'default',
  autoWidth: false,
  align: 'justify',
  groupBy: false,
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
const controlClass = computed(() => {
  if (props.variant === 'toolbar') {
    return props.size === 'sm' ? 'h-8 text-sm font-medium' : 'h-10 text-sm'
  }
  return props.size === 'sm' ? 'h-9 text-sm' : 'h-12'
})
const triggerClass = computed(() => {
  if (props.variant === 'toolbar') {
    return 'w-full flex flex-nowrap items-center cursor-pointer rounded-lg px-2 text-base-content/70 hover:text-base-content hover:bg-base-300/70 transition-colors'
  }
  return 'input input-bordered w-full flex items-center cursor-pointer'
})
const panelPositionClass = computed(() => {
  const yClass = props.direction === 'up' ? 'bottom-full mb-1' : 'top-full mt-1'
  let xClass = ''
  if (props.align === 'justify') {
    xClass = 'left-0 right-0'
  } else if (props.align === 'left') {
    xClass = 'left-0'
  } else if (props.align === 'right') {
    xClass = 'right-0'
  }
  return `${yClass} ${xClass}`
})
const panelVariantClass = computed(() =>
  props.variant === 'toolbar'
    ? 'bg-base-200/95 border-base-300 backdrop-blur-md shadow-2xl drop-shadow-2xl'
    : 'bg-base-100 border-base-300',
)
const triggerWidth = ref(0)
const updateTriggerWidth = () => {
  triggerWidth.value = containerRef.value?.offsetWidth || 0
}

const panelStyle = computed<Record<string, string> | undefined>(() => {
  if (!props.autoWidth) return undefined
  const minPx = triggerWidth.value || 0
  return {
    minWidth: props.variant === 'toolbar' ? '220px' : `${minPx}px`,
    width: props.variant === 'toolbar' ? 'max-content' : 'auto',
    maxWidth: '24rem', // limits to ~384px so it won't overflow screen horizontally
  }
})

// 计算显示值
const displayValue = computed(() => {
  const option = props.options.find(o => o.value === props.modelValue)
  return option ? option.label : ''
})

// 过滤选项
const filteredOptions = computed(() => {
  let list = props.options
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    list = props.options.filter(option => 
      option.label.toLowerCase().includes(query) ||
      option.value.toLowerCase().includes(query) ||
      (option.description && option.description.toLowerCase().includes(query))
    )
  }
  
  if (props.groupBy === 'description') {
    const groups = new Map<string, Option[]>()
    const noGroup: Option[] = []
    
    list.forEach(opt => {
      if (opt.description) {
        if (!groups.has(opt.description)) groups.set(opt.description, [])
        groups.get(opt.description)!.push(opt)
      } else {
        noGroup.push(opt)
      }
    })

    if (groups.size > 0) {
      const result: Option[] = []
      if (noGroup.length > 0) result.push(...noGroup)
      
      const sortedKeys = Array.from(groups.keys()).sort()
      for (const groupName of sortedKeys) {
        result.push({ value: `__group_${groupName}`, label: groupName, isGroupHeader: true })
        result.push(...groups.get(groupName)!)
      }
      return result
    }
  }
  
  return list
})

// 能够被选中的下一个索引
const getNextSelectableIndex = (startIndex: number, direction: 1 | -1) => {
  let next = startIndex
  while (next >= 0 && next < filteredOptions.value.length) {
    if (!filteredOptions.value[next].isGroupHeader) {
      return next
    }
    next += direction
  }
  return -1
}

const initializeHighlight = () => {
  const next = getNextSelectableIndex(0, 1)
  highlightedIndex.value = next !== -1 ? next : 0
}

// 打开/关闭下拉
const toggleDropdown = () => {
  if (props.disabled) return
  isOpen.value = !isOpen.value
  if (isOpen.value) {
    updateTriggerWidth()
    searchQuery.value = ''
    initializeHighlight()
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
  if (option.isGroupHeader) return
  emit('update:modelValue', option.value)
  emit('change', option.value)
  closeDropdown()
}

// 键盘导航
const navigateDown = () => {
  const next = getNextSelectableIndex(highlightedIndex.value + 1, 1)
  if (next !== -1) {
    highlightedIndex.value = next
  }
}

const navigateUp = () => {
  const next = getNextSelectableIndex(highlightedIndex.value - 1, -1)
  if (next !== -1) {
    highlightedIndex.value = next
  }
}

const selectHighlighted = () => {
  if (filteredOptions.value.length > 0) {
    const option = filteredOptions.value[highlightedIndex.value]
    if (option && !option.isGroupHeader) {
      selectOption(option)
    }
  }
}

const getOptionClass = (value: string, index: number) => {
  const isSelected = value === props.modelValue
  const isHighlighted = highlightedIndex.value === index

  let base = 'hover:bg-base-200'
  
  if (props.variant === 'toolbar') {
    base = 'hover:bg-base-300/70'
    if (isHighlighted) return 'bg-base-300/70 text-base-content'
    if (isSelected) return 'text-base-content font-medium'
    return `${base} text-base-content/80`
  }

  if (isHighlighted) return 'bg-base-200 text-base-content'
  if (isSelected) return 'text-base-content font-medium'
  return `${base} text-base-content/80`
}

// 点击外部关闭
const handleClickOutside = (event: MouseEvent) => {
  if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
    closeDropdown()
  }
}

// 搜索变化时重置高亮索引
watch(searchQuery, () => {
  initializeHighlight()
})

onMounted(() => {
  updateTriggerWidth()
  document.addEventListener('click', handleClickOutside)
  window.addEventListener('resize', updateTriggerWidth)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  window.removeEventListener('resize', updateTriggerWidth)
})
</script>

<style scoped>
.searchable-select .input {
  display: flex;
  align-items: center;
}
</style>
