<template>
  <div class="progress-block">
    <!-- Header -->
    <div class="progress-header">
      <span class="progress-title">Progress</span>
      <span class="progress-stats">{{ currentStep }}/{{ totalSteps }}</span>
      <span class="progress-percent">{{ progressPercent }}%</span>
    </div>
    
    <!-- Progress bar -->
    <div class="progress-bar-container">
      <div class="progress-bar" :style="{ width: `${progressPercent}%` }"></div>
    </div>
    
    <!-- Steps list -->
    <div class="steps-list" v-if="steps.length > 0">
      <div 
        v-for="(step, index) in steps" 
        :key="index"
        :class="['step-item', getStepClass(index)]"
      >
        <span class="step-indicator">{{ getStepIndicator(index) }}</span>
        <span class="step-content">{{ step }}</span>
      </div>
    </div>
    
    <!-- Current action -->
    <div v-if="currentAction" class="current-action">
      <span class="action-icon">→</span>
      <span class="action-text">{{ currentAction }}</span>
      <span class="loading-dots">
        <span>.</span><span>.</span><span>.</span>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  currentStep: number
  totalSteps: number
  steps?: string[]
  currentAction?: string
}>(), {
  currentStep: 0,
  totalSteps: 1,
  steps: () => [],
})

// Progress percentage
const progressPercent = computed(() => {
  if (props.totalSteps === 0) return 0
  return Math.round((props.currentStep / props.totalSteps) * 100)
})

// Get step class based on index
const getStepClass = (index: number): string => {
  if (index < props.currentStep - 1) return 'completed'
  if (index === props.currentStep - 1) return 'current'
  return 'pending'
}

// Get step indicator
const getStepIndicator = (index: number): string => {
  if (index < props.currentStep - 1) return '✓'
  if (index === props.currentStep - 1) return '→'
  return '○'
}
</script>

<style scoped>
.progress-block {
  border-radius: 0.5rem;
  background: var(--color-bg-secondary, #1a1a1a);
  border: 1px solid var(--color-border, #27272a);
  padding: 1rem;
  margin: 0.5rem 0;
}

/* Header */
.progress-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.progress-title {
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--color-text-primary, #fafafa);
}

.progress-stats {
  font-size: 0.8125rem;
  color: var(--color-text-secondary, #a1a1aa);
}

.progress-percent {
  margin-left: auto;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-primary, #6366f1);
}

/* Progress bar */
.progress-bar-container {
  height: 4px;
  background: var(--color-bg-tertiary, #262626);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 1rem;
}

.progress-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--color-primary, #6366f1), var(--color-success, #22c55e));
  border-radius: 2px;
  transition: width 0.3s ease;
}

/* Steps list */
.steps-list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.step-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8125rem;
  padding: 0.25rem 0;
}

.step-indicator {
  width: 1rem;
  text-align: center;
  flex-shrink: 0;
  font-weight: bold;
}

.step-content {
  flex: 1;
}

/* Step states */
.step-item.completed .step-indicator {
  color: var(--color-success, #22c55e);
}

.step-item.completed .step-content {
  color: var(--color-text-secondary, #a1a1aa);
  text-decoration: line-through;
}

.step-item.current .step-indicator {
  color: var(--color-primary, #6366f1);
}

.step-item.current .step-content {
  color: var(--color-text-primary, #fafafa);
  font-weight: 500;
}

.step-item.pending .step-indicator {
  color: var(--color-text-secondary, #a1a1aa);
}

.step-item.pending .step-content {
  color: var(--color-text-secondary, #a1a1aa);
}

/* Current action */
.current-action {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--color-border, #27272a);
  font-size: 0.8125rem;
  color: var(--color-primary, #6366f1);
}

.action-icon {
  animation: bounce 1s infinite;
}

@keyframes bounce {
  0%, 100% { transform: translateX(0); }
  50% { transform: translateX(3px); }
}

.action-text {
  flex: 1;
}

.loading-dots span {
  animation: dots 1.4s infinite;
  opacity: 0;
}

.loading-dots span:nth-child(1) { animation-delay: 0s; }
.loading-dots span:nth-child(2) { animation-delay: 0.2s; }
.loading-dots span:nth-child(3) { animation-delay: 0.4s; }

@keyframes dots {
  0%, 100% { opacity: 0; }
  50% { opacity: 1; }
}
</style>

