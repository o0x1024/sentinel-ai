<template>
  <div class="editable-select relative" ref="containerRef">
    <!-- 输入框 - 支持手动输入和下拉选择 -->
    <div class="relative">
      <input
        ref="inputRef"
        type="text"
        class="input input-bordered w-full pr-8"
        :class="{ 'opacity-50 cursor-not-allowed': disabled }"
        :placeholder="placeholder"
        :disabled="disabled"
        v-model="inputValue"
        @focus="handleFocus"
        @input="handleInput"
        @keydown.down.prevent="navigateDown"
        @keydown.up.prevent="navigateUp"
        @keydown.enter.prevent="handleEnter"
        @keydown.escape="closeDropdown"
        @blur="handleBlur"
      />
      <button
        type="button"
        class="absolute right-2 top-1/2 -translate-y-1/2 text-base-content/60 hover:text-base-content transition-colors"
        :class="{ 'opacity-50 cursor-not-allowed': disabled }"
        :disabled="disabled"
        @click="toggleDropdown"
      >
        <i class="fas fa-chevron-down text-xs transition-transform" :class="{ 'rotate-180': isOpen }"></i>
      </button>
    </div>
    
    <!-- 下拉面板 -->
    <div 
      v-if="isOpen && !disabled && (filteredOptions.length > 0 || allowCustom)"
      class="absolute z-50 top-full left-0 right-0 mt-1 bg-base-100 border border-base-300 rounded-box shadow-lg max-h-64 overflow-hidden flex flex-col"
    >
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
          @mousedown.prevent="selectOption(option)"
          @mouseenter="highlightedIndex = index"
        >
          <span class="flex-1 truncate">{{ option.label }}</span>
          <span v-if="option.description" class="text-xs opacity-70 truncate max-w-32">{{ option.description }}</span>
        </div>
        
        <!-- 自定义输入提示 -->
        <div 
          v-if="allowCustom && inputValue && !filteredOptions.some(o => o.value === inputValue)"
          class="px-3 py-2 cursor-pointer transition-colors bg-base-200/50 hover:bg-base-200 border-t border-base-300"
          :class="{ 'bg-primary text-primary-content': highlightedIndex === filteredOptions.length }"
          @mousedown.prevent="selectCustomValue"
          @mouseenter="highlightedIndex = filteredOptions.length"
        >
          <div class="flex items-center gap-2">
            <i class="fas fa-plus text-xs"></i>
            <span class="flex-1 truncate">{{ customValueText }}: <strong>{{ inputValue }}</strong></span>
          </div>
        </div>
        
        <!-- 无匹配结果 -->
        <div v-if="filteredOptions.length === 0 && !allowCustom" class="px-3 py-4 text-center text-base-content/50">
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
  noResultsText?: string
  customValueText?: string
  disabled?: boolean
  allowCustom?: boolean // 是否允许自定义输入
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '请选择或输入',
  noResultsText: '无匹配结果',
  customValueText: '使用自定义值',
  disabled: false,
  allowCustom: true
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'change': [value: string]
}>()

const containerRef = ref<HTMLElement>()
const inputRef = ref<HTMLInputElement>()
const isOpen = ref(false)
const inputValue = ref('')
const highlightedIndex = ref(0)
const isBlurring = ref(false)

// 初始化输入值
watch(() => props.modelValue, (newValue) => {
  if (newValue !== inputValue.value) {
    // 尝试从选项中找到对应的 label
    const option = props.options.find(o => o.value === newValue)
    inputValue.value = option ? option.label : newValue
  }
}, { immediate: true })

// 过滤选项
const filteredOptions = computed(() => {
  if (!inputValue.value) {
    return props.options
  }
  const query = inputValue.value.toLowerCase()
  return props.options.filter(option => 
    option.label.toLowerCase().includes(query) ||
    option.value.toLowerCase().includes(query) ||
    (option.description && option.description.toLowerCase().includes(query))
  )
})

// 打开下拉
const openDropdown = () => {
  if (props.disabled) return
  isOpen.value = true
  highlightedIndex.value = 0
}

// 关闭下拉
const closeDropdown = () => {
  isOpen.value = false
}

// 切换下拉
const toggleDropdown = () => {
  if (props.disabled) return
  if (isOpen.value) {
    closeDropdown()
  } else {
    openDropdown()
    nextTick(() => {
      inputRef.value?.focus()
    })
  }
}

// 处理焦点
const handleFocus = () => {
  openDropdown()
}

// 处理输入
const handleInput = () => {
  openDropdown()
  highlightedIndex.value = 0
}

// 处理失焦
const handleBlur = () => {
  isBlurring.value = true
  // 延迟关闭，以便点击选项时能够触发
  setTimeout(() => {
    if (isBlurring.value) {
      closeDropdown()
      // 如果允许自定义且输入值不为空，使用输入值
      if (props.allowCustom && inputValue.value) {
        const option = props.options.find(o => 
          o.label.toLowerCase() === inputValue.value.toLowerCase() ||
          o.value.toLowerCase() === inputValue.value.toLowerCase()
        )
        const finalValue = option ? option.value : inputValue.value
        if (finalValue !== props.modelValue) {
          emit('update:modelValue', finalValue)
          emit('change', finalValue)
        }
      }
      isBlurring.value = false
    }
  }, 200)
}

// 选择选项
const selectOption = (option: Option) => {
  isBlurring.value = false
  inputValue.value = option.label
  emit('update:modelValue', option.value)
  emit('change', option.value)
  closeDropdown()
  inputRef.value?.blur()
}

// 选择自定义值
const selectCustomValue = () => {
  isBlurring.value = false
  if (inputValue.value) {
    emit('update:modelValue', inputValue.value)
    emit('change', inputValue.value)
  }
  closeDropdown()
  inputRef.value?.blur()
}

// 键盘导航
const navigateDown = () => {
  const maxIndex = props.allowCustom && inputValue.value && !filteredOptions.value.some(o => o.value === inputValue.value)
    ? filteredOptions.value.length
    : filteredOptions.value.length - 1
  
  if (highlightedIndex.value < maxIndex) {
    highlightedIndex.value++
  }
}

const navigateUp = () => {
  if (highlightedIndex.value > 0) {
    highlightedIndex.value--
  }
}

const handleEnter = () => {
  if (!isOpen.value) {
    openDropdown()
    return
  }

  if (filteredOptions.value.length > 0 && highlightedIndex.value < filteredOptions.value.length) {
    selectOption(filteredOptions.value[highlightedIndex.value])
  } else if (props.allowCustom && inputValue.value) {
    selectCustomValue()
  }
}

// 点击外部关闭
const handleClickOutside = (event: MouseEvent) => {
  if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
    closeDropdown()
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.editable-select .input {
  padding-right: 2rem;
}
</style>

