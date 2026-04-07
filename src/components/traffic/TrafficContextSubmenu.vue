<template>
  <div class="relative group">
    <button class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2 justify-between">
      <span class="flex items-center gap-2">
        <i :class="submenu.triggerIconClass"></i>
        {{ t(`${labelPrefix}.${submenu.triggerLabelKey}`) }}
      </span>
      <i class="fas fa-chevron-right text-xs"></i>
    </button>
    <div
      :class="submenu.submenuClass ?? 'absolute left-full top-0 ml-1 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-40 z-50 hidden group-hover:block'"
    >
      <button
        v-for="item in submenu.items"
        :key="item.key"
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        :disabled="item.disabled"
        :class="{ 'opacity-50 cursor-not-allowed': item.disabled }"
        @click="item.onClick"
      >
        <i v-if="item.iconClass" :class="item.iconClass"></i>
        <span>{{ t(`${labelPrefix}.${item.labelKey}`) }}</span>
        <span v-if="item.suffixText" class="truncate text-base-content/70">: {{ item.suffixText }}</span>
      </button>
      <div v-if="submenu.footerItems?.length" class="divider my-1 h-px"></div>
      <button
        v-for="item in submenu.footerItems"
        :key="item.key"
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        :disabled="item.disabled"
        :class="{ 'opacity-50 cursor-not-allowed': item.disabled }"
        @click="item.onClick"
      >
        <i v-if="item.iconClass" :class="item.iconClass"></i>
        {{ t(`${labelPrefix}.${item.labelKey}`) }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { TrafficContextSubmenu } from './trafficContextSubmenuSupport'

defineProps<{
  submenu: TrafficContextSubmenu
  labelPrefix: string
}>()

const { t } = useI18n()
</script>
