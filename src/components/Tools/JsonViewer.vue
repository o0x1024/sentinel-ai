<template>
  <div class="json-viewer">
    <JsonNode :data="data" :name="rootName" :expanded="expanded" :depth="0" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import JsonNode from './JsonNode.vue'

interface Props {
  data: any
  rootName?: string
  expanded?: boolean
}

withDefaults(defineProps<Props>(), {
  rootName: 'root',
  expanded: true
})
</script>

<style scoped>
.json-viewer {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.5;
}
</style>
