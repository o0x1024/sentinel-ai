<template>
  <div 
    ref="container" 
    class="virtual-list-container" 
    :style="{ height: `${height}px`, overflow: 'auto' }"
    @scroll="handleScroll"
  >
    <div 
      class="virtual-list-wrapper"
      :style="{ height: `${totalHeight}px`, position: 'relative' }"
    >
      <div 
        class="virtual-list-content"
        :style="{ 
          transform: `translateY(${offsetY}px)`,
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0
        }"
      >
        <div
          v-for="(item, index) in visibleItems"
          :key="getItemKey(item, startIndex + index)"
          class="virtual-list-item"
          :style="{ height: `${itemHeight}px` }"
        >
          <slot :item="item" :index="startIndex + index" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';

// Props定义
interface Props {
  items: any[];
  itemHeight: number;
  height: number;
  buffer?: number;
  keyField?: string;
}

const props = withDefaults(defineProps<Props>(), {
  buffer: 5,
  keyField: 'id'
});

// 响应式变量
const container = ref<HTMLElement>();
const scrollTop = ref(0);

// 计算属性
const totalHeight = computed(() => props.items.length * props.itemHeight);

const visibleCount = computed(() => Math.ceil(props.height / props.itemHeight));

const startIndex = computed(() => {
  const start = Math.floor(scrollTop.value / props.itemHeight) - props.buffer;
  return Math.max(0, start);
});

const endIndex = computed(() => {
  const end = startIndex.value + visibleCount.value + props.buffer * 2;
  return Math.min(props.items.length - 1, end);
});

const visibleItems = computed(() => {
  return props.items.slice(startIndex.value, endIndex.value + 1);
});

const offsetY = computed(() => startIndex.value * props.itemHeight);

// 方法
const emit = defineEmits<{
  (e: 'scroll', payload: { scrollTop: number; clientHeight: number; scrollHeight: number }): void
}>()

const handleScroll = (event: Event) => {
  const target = event.target as HTMLElement;
  scrollTop.value = target.scrollTop;
  emit('scroll', {
    scrollTop: target.scrollTop,
    clientHeight: target.clientHeight,
    scrollHeight: target.scrollHeight,
  })
};

const getItemKey = (item: any, index: number) => {
  return item[props.keyField] || index;
};

const scrollToIndex = (index: number) => {
  if (container.value) {
    const scrollPosition = index * props.itemHeight;
    container.value.scrollTop = scrollPosition;
  }
};

const scrollToTop = () => {
  if (container.value) {
    container.value.scrollTop = 0;
  }
};

// 监听items变化，重置滚动位置
watch(() => props.items.length, () => {
  scrollTop.value = 0;
  if (container.value) {
    container.value.scrollTop = 0;
  }
});

// 暴露方法给父组件
defineExpose({
  scrollToIndex,
  scrollToTop
});
</script>

<style scoped>
.virtual-list-container {
  position: relative;
}

.virtual-list-container::-webkit-scrollbar {
  width: 6px;
}

.virtual-list-container::-webkit-scrollbar-track {
  background: theme('colors.base-300');
  border-radius: 3px;
}

.virtual-list-container::-webkit-scrollbar-thumb {
  background: theme('colors.base-content');
  border-radius: 3px;
  opacity: 0.5;
}

.virtual-list-container::-webkit-scrollbar-thumb:hover {
  opacity: 0.8;
}

.virtual-list-item {
  display: block;
  align-items: center;
  border-bottom: 1px solid theme('colors.base-300');
}
</style> 