<template>
  <div
    class="safe-top"
    :class="immersiveDrillModeEnabled ? 'px-3 py-3 space-y-3' : 'page-content-padded space-y-6'"
  >
    <!-- 页面标题 -->
    <div v-if="!immersiveDrillModeEnabled" class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">{{ $t('securityCenter.title') }}</h1>
      <!-- <div class="flex space-x-2">
        <button @click="refreshAll" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div> -->
    </div>


    <!-- Tab 导航 -->
    <div
      class="tabs tabs-boxed bg-base-100 shadow-sm"
      :class="immersiveDrillModeEnabled ? 'tabs-sm rounded-2xl px-1 py-1' : ''"
    >
      <a
        v-if="isTabVisible('workbench')"
        class="tab"
        :class="{ 'tab-active': activeTab === 'workbench' }"
        @click="switchTab('workbench')"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 6.75V6a3 3 0 016 0v.75m-7.5 0h9A1.5 1.5 0 0118 8.25v9A1.5 1.5 0 0116.5 18.75h-9A1.5 1.5 0 016 17.25v-9A1.5 1.5 0 017.5 6.75zM10.5 11.25h3"
          ></path>
        </svg>
        {{ $t('securityCenter.tabs.workbench') }}
      </a>

      <a 
        v-if="isTabVisible('vulnerabilities')"
        class="tab" 
        :class="{ 'tab-active': activeTab === 'vulnerabilities' }"
        @click="switchTab('vulnerabilities')"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
        </svg>
        {{ $t('securityCenter.tabs.vulnerabilities') }}
      </a>

      <a
        v-if="isTabVisible('llmSecurity')"
        class="tab"
        :class="{ 'tab-active': activeTab === 'llmSecurity' }"
        @click="switchTab('llmSecurity')"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 3a.75.75 0 01.75.75V5h3V3.75a.75.75 0 011.5 0V5H16a2 2 0 012 2v2.25a.75.75 0 01-1.5 0V7a.5.5 0 00-.5-.5H8a.5.5 0 00-.5.5v2.25a.75.75 0 01-1.5 0V7a2 2 0 012-2h1V3.75A.75.75 0 019.75 3zM6.75 12A1.75 1.75 0 005 13.75v4.5C5 19.216 5.784 20 6.75 20h10.5A1.75 1.75 0 0019 18.25v-4.5A1.75 1.75 0 0017.25 12H6.75z"></path>
        </svg>
        {{ $t('securityCenter.tabs.llmSecurity') }}
      </a>
    </div>

    <KeepAlive>
      <component
        :is="activeTabComponent"
        @stats-updated="updateVulnStats"
      />
    </KeepAlive>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch, markRaw } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import VulnerabilitiesPanel from '../components/SecurityCenter/VulnerabilitiesPanel.vue';
import LlmSecurityPanel from '../components/SecurityCenter/LlmSecurityPanel.vue';
import SecurityWorkbenchPage from '../components/SecurityCenter/SecurityWorkbenchPage.vue';
import { immersiveDrillModeEnabled } from '../services/immersiveDrillMode'
import {
  immersiveDrillSecurityTabs,
  isImmersiveDrillSecurityTab,
} from '../services/immersiveDrillPreset'


defineOptions({
  name: 'SecurityCenter'
});

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

type SecurityCenterTab = 'scan' | 'vulnerabilities' | 'llmSecurity' | 'workbench' | 'assets'

const allSecurityTabs: SecurityCenterTab[] = ['workbench', 'vulnerabilities', 'llmSecurity']

// 当前激活的 Tab
const activeTab = ref<SecurityCenterTab>('workbench');
const lastWorkbenchLocation = ref('/security-center/workbench')

const visibleSecurityTabs = computed<SecurityCenterTab[]>(() =>
  immersiveDrillModeEnabled.value ? [...immersiveDrillSecurityTabs] : allSecurityTabs,
)

const isTabVisible = (tab: SecurityCenterTab) => visibleSecurityTabs.value.includes(tab)

const normalizeRequestedTab = (tab: unknown, findingId: unknown): SecurityCenterTab => {
  if (typeof findingId === 'string' && findingId.trim()) {
    return 'vulnerabilities'
  }

  if (typeof tab === 'string' && ['scan', 'vulnerabilities', 'llmSecurity', 'workbench'].includes(tab)) {
    if (immersiveDrillModeEnabled.value && !isImmersiveDrillSecurityTab(tab)) {
      return 'workbench'
    }

    return tab as SecurityCenterTab
  }

  return 'workbench'
}

const tabComponents = {
  workbench: markRaw(SecurityWorkbenchPage),
  vulnerabilities: markRaw(VulnerabilitiesPanel),
  llmSecurity: markRaw(LlmSecurityPanel),
} as const

const activeTabComponent = computed(() => {
  switch (activeTab.value) {
    case 'vulnerabilities':
      return tabComponents.vulnerabilities
    case 'llmSecurity':
      return tabComponents.llmSecurity
    case 'workbench':
    default:
      return tabComponents.workbench
  }
})

// 统计数据
const overviewStats = ref({
  scanTasks: 0,
  runningTasks: 0,
  vulnerabilities: 0,
  criticalVulns: 0,
  assets: 0,
  activeAssets: 0,
  riskScore: 0
});

// 从 URL 参数读取初始 Tab
onMounted(() => {
  const tab = route.query.tab as string;
  const findingId = typeof route.query.findingId === 'string' ? route.query.findingId : ''
  const caseId =
    typeof route.params.caseId === 'string'
      ? route.params.caseId
      : typeof route.query.caseId === 'string'
        ? route.query.caseId
        : ''
  if (shouldNormalizeToWorkbenchRoute(route.name, route.path, tab, findingId, route.params.caseId, route.query.caseId)) {
    activeTab.value = 'workbench'
    normalizeToWorkbenchRoute()
    return
  }
  if (route.name === 'SecurityWorkbench' || caseId) {
    rememberWorkbenchLocation()
    activeTab.value = 'workbench'
    return
  }
  activeTab.value = normalizeRequestedTab(tab, findingId)
});

const isWorkbenchRoute = (routePath: unknown, routeName: unknown, routeCaseId: unknown, queryCaseId: unknown) => (
  routeName === 'SecurityWorkbench'
  || (typeof routePath === 'string' && routePath.startsWith('/security-center/workbench'))
  || (typeof routeCaseId === 'string' && routeCaseId.trim())
  || (typeof queryCaseId === 'string' && queryCaseId.trim())
)

const isSecurityCenterContainerRoute = (routePath: unknown, routeName: unknown) => (
  routeName === 'SecurityCenter'
  || routeName === 'SecurityWorkbench'
  || routeName === 'ScanTasks'
  || routeName === 'Vulnerabilities'
  || (typeof routePath === 'string' && routePath.startsWith('/security-center'))
)

const hasWorkbenchCase = (routeCaseId: unknown, queryCaseId: unknown) => (
  (typeof routeCaseId === 'string' && routeCaseId.trim())
  || (typeof queryCaseId === 'string' && queryCaseId.trim())
)

const shouldNormalizeToWorkbenchRoute = (
  routeName: unknown,
  routePath: unknown,
  tab: unknown,
  findingId: unknown,
  routeCaseId: unknown,
  queryCaseId: unknown,
) => {
  if (routeName !== 'SecurityCenter' || routePath !== '/security-center') {
    return false
  }

  if (typeof findingId === 'string' && findingId.trim()) {
    return false
  }

  if (hasWorkbenchCase(routeCaseId, queryCaseId)) {
    return false
  }

  if (typeof tab === 'string' && tab.trim() && tab !== 'workbench') {
    return false
  }

  return true
}

const rememberWorkbenchLocation = () => {
  lastWorkbenchLocation.value = route.fullPath
}

const normalizeToWorkbenchRoute = () => {
  const nextQuery = { ...route.query }
  delete nextQuery.tab
  void router.replace({
    path: '/security-center/workbench',
    query: nextQuery,
  })
}

watch(
  () => [route.name, route.path, route.query.tab, route.query.findingId, route.params.caseId, route.query.caseId],
  ([routeName, routePath, tab, findingId, routeCaseId, queryCaseId]) => {
    if (!isSecurityCenterContainerRoute(routePath, routeName)) {
      return
    }

    if (shouldNormalizeToWorkbenchRoute(routeName, routePath, tab, findingId, routeCaseId, queryCaseId)) {
      activeTab.value = 'workbench'
      normalizeToWorkbenchRoute()
      return
    }

    if (isWorkbenchRoute(routePath, routeName, routeCaseId, queryCaseId)) {
      rememberWorkbenchLocation()
      activeTab.value = 'workbench'
      return
    }

    activeTab.value = normalizeRequestedTab(tab, findingId)
  },
)

watch(visibleSecurityTabs, tabs => {
  if (!tabs.includes(activeTab.value)) {
    activeTab.value = tabs[0] ?? 'workbench'
    updateUrlTab(activeTab.value)
  }
})

// 更新 URL 参数
const updateUrlTab = (tab: SecurityCenterTab) => {
  if (tab === 'workbench') {
    router.replace(lastWorkbenchLocation.value)
    return
  }
  router.replace({ path: '/security-center', query: { tab } });
};

const refreshTab = (tab: SecurityCenterTab) => {
  window.dispatchEvent(new CustomEvent('security-center-refresh', {
    detail: { tab },
  }))
}

// 监听 Tab 切换
const switchTab = (tab: SecurityCenterTab) => {
  if (activeTab.value === tab) {
    refreshTab(tab)
    return
  }
  if (activeTab.value === 'workbench') {
    rememberWorkbenchLocation()
  }
  activeTab.value = tab;
  updateUrlTab(tab);
};

// 更新各模块统计数据
const updateScanStats = (stats: any) => {
  overviewStats.value.scanTasks = stats.total || 0;
  overviewStats.value.runningTasks = stats.running || 0;
  calculateRiskScore();
};

const updateVulnStats = (stats: any) => {
  overviewStats.value.vulnerabilities = stats.total || 0;
  overviewStats.value.criticalVulns = stats.critical || 0;
  calculateRiskScore();
};

const updateAssetStats = (stats: any) => {
  overviewStats.value.assets = stats.total || 0;
  overviewStats.value.activeAssets = stats.active || 0;
  calculateRiskScore();
};

// 计算风险评分
const calculateRiskScore = () => {
  // 简单的风险评分算法
  const criticalWeight = 10;
  const vulnWeight = 1;
  const assetWeight = 0.1;
  
  const score = 
    overviewStats.value.criticalVulns * criticalWeight +
    overviewStats.value.vulnerabilities * vulnWeight +
    overviewStats.value.assets * assetWeight;
  
  overviewStats.value.riskScore = Math.min(100, Math.round(score));
};

// 获取风险等级描述
const getRiskLevel = (score: number): string => {
  if (score >= 80) return t('securityCenter.riskLevel.critical');
  if (score >= 60) return t('securityCenter.riskLevel.high');
  if (score >= 40) return t('securityCenter.riskLevel.medium');
  if (score >= 20) return t('securityCenter.riskLevel.low');
  return t('securityCenter.riskLevel.safe');
};

// 刷新所有数据
const refreshAll = () => {
  // 触发所有子组件刷新
  refreshTab(activeTab.value);
};
</script>

<style scoped>
.tab-content {
  min-height: 400px;
}

.tabs-boxed .tab {
  transition: all 0.2s;
}

.tabs-boxed .tab:hover {
  background-color: hsl(var(--b2));
}

.stat {
  border: 1px solid hsl(var(--bc) / 0.2);
}
</style>
