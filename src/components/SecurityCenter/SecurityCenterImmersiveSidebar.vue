<template>
  <aside class="security-center-immersive-sidebar rounded-[28px] border border-base-300/80 bg-base-100/92 p-3 shadow-[0_24px_64px_rgba(15,23,42,0.12)] backdrop-blur-sm">
    <div class="mb-3 px-2">
      <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
        {{ t('securityCenter.title') }}
      </p>
      <h2 class="mt-1 text-lg font-semibold text-base-content">
        {{ t('securityCenter.immersiveSidebar.title', '安全中心侧栏') }}
      </h2>
      <p class="mt-1 text-xs text-base-content/60">
        {{ t('securityCenter.immersiveSidebar.description', '在沉浸模式下集中切换安全工作台和漏洞视图。') }}
      </p>
    </div>

    <nav class="flex flex-col gap-2">
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="flex items-center gap-3 rounded-2xl border px-3 py-3 text-left transition-all duration-200"
        :class="item.id === activeTab
          ? 'border-primary bg-primary text-primary-content shadow-lg shadow-primary/15'
          : 'border-base-300/70 bg-base-200/70 text-base-content/75 hover:border-primary/35 hover:bg-base-200'"
        @click="$emit('select-tab', item.id)"
      >
        <span class="flex h-10 w-10 items-center justify-center rounded-xl bg-black/10">
          <i :class="`${item.icon} text-sm`"></i>
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-semibold">{{ item.label }}</span>
          <span class="mt-0.5 block truncate text-xs opacity-75">{{ item.description }}</span>
        </span>
      </button>
    </nav>
  </aside>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

defineOptions({
  name: 'SecurityCenterImmersiveSidebar',
})

const props = defineProps<{
  activeTab: 'workbench' | 'vulnerabilities'
}>()

defineEmits<{
  (e: 'select-tab', tab: 'workbench' | 'vulnerabilities'): void
}>()

const { t } = useI18n()

const items = computed(() => [
  {
    id: 'workbench' as const,
    label: t('securityCenter.tabs.workbench'),
    description: t('securityCenter.immersiveSidebar.workbenchDescription', '查看 case、证据链和验证流程。'),
    icon: 'fas fa-shield-alt',
  },
  {
    id: 'vulnerabilities' as const,
    label: t('securityCenter.tabs.vulnerabilities'),
    description: t('securityCenter.immersiveSidebar.vulnerabilitiesDescription', '查看漏洞列表、详情和相关证据。'),
    icon: 'fas fa-bug',
  },
])
</script>

<style scoped>
.security-center-immersive-sidebar {
  width: 17rem;
  flex-shrink: 0;
}
</style>
