<template>
  <div class="json-node" :style="{ paddingLeft: depth > 0 ? '16px' : '0' }">
    <!-- 对象/数组 -->
    <div v-if="isObject || isArray" class="json-complex">
      <div class="json-line flex items-start gap-1 hover:bg-base-300/50 rounded px-1 cursor-pointer" @click="toggleExpand">
        <span class="json-toggle text-base-content/60">
          <i :class="['fas', isExpanded ? 'fa-chevron-down' : 'fa-chevron-right']" class="text-xs"></i>
        </span>
        <span v-if="name" class="json-key text-info font-semibold">{{ name }}:</span>
        <span class="json-bracket text-base-content/80">{{ isArray ? '[' : '{' }}</span>
        <span v-if="!isExpanded" class="json-preview text-base-content/60 text-xs ml-1">
          {{ isArray ? `${arrayLength} items` : `${objectKeys.length} keys` }}
        </span>
        <span v-if="!isExpanded" class="json-bracket text-base-content/80">{{ isArray ? ']' : '}' }}</span>
      </div>
      
      <div v-if="isExpanded" class="json-children">
        <template v-if="isArray">
          <JsonNode 
            v-for="(item, index) in data" 
            :key="index"
            :data="item"
            :name="`[${index}]`"
            :expanded="depth < 2"
            :depth="depth + 1"
          />
        </template>
        <template v-else>
          <JsonNode 
            v-for="key in objectKeys" 
            :key="key"
            :data="data[key]"
            :name="key"
            :expanded="depth < 2"
            :depth="depth + 1"
          />
        </template>
        <div class="json-line px-1">
          <span class="json-bracket text-base-content/80">{{ isArray ? ']' : '}' }}</span>
        </div>
      </div>
    </div>
    
    <!-- 基本类型 -->
    <div v-else class="json-primitive json-line flex items-start gap-1 px-1">
      <span v-if="name" class="json-key text-info">{{ name }}:</span>
      <span :class="valueClass">{{ formattedValue }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface Props {
  data: any
  name?: string
  expanded?: boolean
  depth?: number
}

const props = withDefaults(defineProps<Props>(), {
  name: '',
  expanded: true,
  depth: 0
})

const isExpanded = ref(props.expanded)

const isObject = computed(() => {
  return props.data !== null && typeof props.data === 'object' && !Array.isArray(props.data)
})

const isArray = computed(() => {
  return Array.isArray(props.data)
})

const objectKeys = computed(() => {
  if (!isObject.value) return []
  return Object.keys(props.data)
})

const arrayLength = computed(() => {
  if (!isArray.value) return 0
  return props.data.length
})

const valueClass = computed(() => {
  const type = typeof props.data
  if (props.data === null) return 'json-null text-base-content/60'
  if (type === 'string') return 'json-string text-success'
  if (type === 'number') return 'json-number text-warning'
  if (type === 'boolean') return 'json-boolean text-error'
  return 'json-value text-base-content'
})

const formattedValue = computed(() => {
  if (props.data === null) return 'null'
  if (props.data === undefined) return 'undefined'
  if (typeof props.data === 'string') return `"${props.data}"`
  return String(props.data)
})

function toggleExpand() {
  isExpanded.value = !isExpanded.value
}
</script>

<style scoped>
.json-node {
  user-select: text;
}

.json-line {
  min-height: 20px;
}

.json-toggle {
  width: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.json-key {
  flex-shrink: 0;
  margin-right: 4px;
}

.json-string {
  word-break: break-all;
}

.json-preview {
  font-style: italic;
  margin-left: 4px;
}
</style>
