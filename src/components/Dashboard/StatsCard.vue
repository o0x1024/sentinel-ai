<template>
  <div :class="cardClass" class="card shadow-lg">
    <div class="card-body">
      <div class="flex items-center justify-between">
        <div>
          <h3 :class="valueClass" class="text-2xl font-bold">{{ formattedValue }}</h3>
          <p :class="labelClass" class="opacity-80">{{ label }}</p>
          <p v-if="subtitle" class="text-xs opacity-60 mt-1">{{ subtitle }}</p>
        </div>
        <div :class="iconContainerClass" class="w-12 h-12 rounded-lg flex items-center justify-center">
          <i :class="iconClass" class="text-3xl opacity-80"></i>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  value: number | string
  label: string
  subtitle?: string
  icon: string
  theme?: 'success' | 'primary' | 'secondary' | 'error' | 'warning' | 'info'
  prefix?: string
  suffix?: string
}

const props = withDefaults(defineProps<Props>(), {
  theme: 'primary',
  prefix: '',
  suffix: ''
})

// 计算属性
const formattedValue = computed(() => {
  if (typeof props.value === 'number') {
    // 格式化数字显示
    if (props.prefix === '$' && props.value >= 1000) {
      return `${props.prefix}${(props.value / 1000).toFixed(1)}k`
    }
    return `${props.prefix}${props.value.toLocaleString()}${props.suffix}`
  }
  return `${props.prefix}${props.value}${props.suffix}`
})

const cardClass = computed(() => {
  const baseClass = 'bg-'
  const textClass = ' text-'
  switch (props.theme) {
    case 'success':
      return `${baseClass}success${textClass}success-content`
    case 'primary':
      return `${baseClass}primary${textClass}primary-content`
    case 'secondary':
      return `${baseClass}secondary${textClass}secondary-content`
    case 'error':
      return `${baseClass}error${textClass}error-content`
    case 'warning':
      return `${baseClass}warning${textClass}warning-content`
    case 'info':
      return `${baseClass}info${textClass}info-content`
    default:
      return `${baseClass}primary${textClass}primary-content`
  }
})

const valueClass = computed(() => {
  switch (props.theme) {
    case 'success':
      return 'text-success-content'
    case 'primary':
      return 'text-primary-content'
    case 'secondary':
      return 'text-secondary-content'
    case 'error':
      return 'text-error-content'
    case 'warning':
      return 'text-warning-content'
    case 'info':
      return 'text-info-content'
    default:
      return 'text-primary-content'
  }
})

const labelClass = computed(() => {
  switch (props.theme) {
    case 'success':
      return 'text-success-content'
    case 'primary':
      return 'text-primary-content'
    case 'secondary':
      return 'text-secondary-content'
    case 'error':
      return 'text-error-content'
    case 'warning':
      return 'text-warning-content'
    case 'info':
      return 'text-info-content'
    default:
      return 'text-primary-content'
  }
})

const iconContainerClass = computed(() => {
  const opacity = ' opacity-20'
  switch (props.theme) {
    case 'success':
      return `bg-success-content${opacity}`
    case 'primary':
      return `bg-primary-content${opacity}`
    case 'secondary':
      return `bg-secondary-content${opacity}`
    case 'error':
      return `bg-error-content${opacity}`
    case 'warning':
      return `bg-warning-content${opacity}`
    case 'info':
      return `bg-info-content${opacity}`
    default:
      return `bg-primary-content${opacity}`
  }
})

const iconClass = computed(() => {
  switch (props.theme) {
    case 'success':
      return `${props.icon} text-success-content`
    case 'primary':
      return `${props.icon} text-primary-content`
    case 'secondary':
      return `${props.icon} text-secondary-content`
    case 'error':
      return `${props.icon} text-error-content`
    case 'warning':
      return `${props.icon} text-warning-content`
    case 'info':
      return `${props.icon} text-info-content`
    default:
      return `${props.icon} text-primary-content`
  }
})
</script>

<style scoped>
.card {
  transition: transform 0.2s ease-in-out;
}

.card:hover {
  transform: translateY(-2px);
}
</style> 