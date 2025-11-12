<template>
  <div class="page-content-padded safe-top space-y-6">
    <!-- 页面标题 -->
    <div class="flex items-center justify-between">
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
    <div class="tabs tabs-boxed bg-base-100 shadow-sm">
    <a 
        class="tab" 
        :class="{ 'tab-active': activeTab === 'vulnerabilities' }"
        @click="activeTab = 'vulnerabilities'"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
        </svg>
        {{ $t('securityCenter.tabs.vulnerabilities') }}
      </a>
      <a 
        class="tab" 
        :class="{ 'tab-active': activeTab === 'scan' }"
        @click="activeTab = 'scan'"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
        </svg>
        {{ $t('securityCenter.tabs.scanTasks') }}
      </a>

      <a 
        class="tab" 
        :class="{ 'tab-active': activeTab === 'assets' }"
        @click="activeTab = 'assets'"
      >
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"></path>
        </svg>
        {{ $t('securityCenter.tabs.assets') }}
      </a>
    </div>

      <!-- 扫描任务 Tab -->
        <ScanTasksPanel  @stats-updated="updateScanStats"  v-if="activeTab === 'scan'"/>
      <!-- 漏洞管理 Tab -->
        <VulnerabilitiesPanel @stats-updated="updateVulnStats" v-if="activeTab === 'vulnerabilities'"/>
      <!-- 资产管理 Tab -->
        <AssetsPanel @stats-updated="updateAssetStats" v-if="activeTab === 'assets'"/>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import ScanTasksPanel from '../components/SecurityCenter/ScanTasksPanel.vue';
import VulnerabilitiesPanel from '../components/SecurityCenter/VulnerabilitiesPanel.vue';
import AssetsPanel from '../components/SecurityCenter/AssetsPanel.vue';

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

// 当前激活的 Tab
const activeTab = ref<'scan' | 'vulnerabilities' | 'assets'>('vulnerabilities');

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
  if (tab && ['scan', 'vulnerabilities', 'assets'].includes(tab)) {
    activeTab.value = tab as 'scan' | 'vulnerabilities' | 'assets';
  }
});

// 更新 URL 参数
const updateUrlTab = (tab: string) => {
  router.replace({ query: { tab } });
};

// 监听 Tab 切换
const switchTab = (tab: 'scan' | 'vulnerabilities' | 'assets') => {
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
  window.dispatchEvent(new CustomEvent('security-center-refresh'));
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
