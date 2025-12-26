<template>
  <div class="page-content-padded space-y-6 pb-24 relative">
    <!-- 页面标题与配置工具栏 -->
    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 sticky top-0 z-30 bg-base-100/80 backdrop-blur-md py-4 border-b border-base-200 -mx-4 px-4">
      <div>
        <h1 class="text-3xl font-bold flex items-center gap-3">
          <i class="fas fa-chart-line text-primary"></i>
          {{ t('dashboard.title') }}
        </h1>
        <p class="text-base-content/60 mt-1">{{ t('dashboard.welcome') }}</p>
      </div>
      
      <div class="flex flex-wrap items-center gap-2">
        <!-- 布局管理下拉菜单 -->
        <div class="dropdown dropdown-end">
          <label tabindex="0" class="btn btn-ghost btn-sm gap-2">
            <i class="fas fa-th-large"></i>
            {{ t('common.nav.settings') }}
          </label>
          <div tabindex="0" class="dropdown-content z-[50] card card-compact w-64 p-2 shadow-xl bg-base-100 border border-base-200">
            <div class="card-body">
              <h3 class="font-bold text-sm mb-2 border-b pb-2 flex items-center gap-2">
                <i class="fas fa-eye"></i>
                显示/隐藏卡片
              </h3>
              <div class="space-y-1 max-h-64 overflow-y-auto pr-2">
                <div v-for="card in cards" :key="card.id" class="flex items-center justify-between hover:bg-base-200 p-2 rounded-lg transition-colors">
                  <span class="text-xs font-medium truncate flex-1 pr-2">{{ t(card.title) }}</span>
                  <input 
                    type="checkbox" 
                    :checked="card.visible" 
                    @change="toggleCard(card.id)"
                    class="checkbox checkbox-primary checkbox-xs" 
                  />
                </div>
              </div>
              <div class="divider my-2"></div>
              <button @click="resetConfig" class="btn btn-ghost btn-xs text-error w-full">
                {{ t('common.resetDefaults') }}
              </button>
            </div>
          </div>
        </div>

        <button @click="refreshData" :disabled="isLoading" class="btn btn-primary btn-sm gap-2">
          <i class="fas fa-sync" :class="{ 'fa-spin': isLoading }"></i>
          {{ t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 统一卡片网格布局 -->
    <div class="grid grid-cols-12 gap-6 auto-rows-min">
      <template v-for="card in visibleCards" :key="card.id">
        <div 
          draggable="true"
          @dragstart="handleDragStart(card.id, $event)"
          @dragend="handleDragEnd"
          @dragover.prevent="draggedOverCardId = card.id"
          @dragleave="draggedOverCardId = null"
          @drop="handleDrop(card.id)"
          class="transition-all duration-300"
          :class="[
            getCardGridClass(card),
            { 
              'opacity-30 scale-95 dragging': draggedCardId === card.id,
              'drag-over': draggedOverCardId === card.id && draggedCardId !== card.id
            }
          ]"
        >
          <!-- 指标卡片类型 -->
          <template v-if="card.category === 'stats'">
            <StatsCard v-if="card.id === 'assets'" :value="stats.discoveredAssets" :label="t('assetManagement.totalAssets')" :subtitle="t('dashboard.projectsMonitored')" icon="fas fa-server" theme="primary" />
            <StatsCard v-else-if="card.id === 'vulns'" :value="stats.vulnerabilities" :label="t('dashboard.vulnerabilitiesFound')" :subtitle="t('dashboard.recentVulnerabilities')" icon="fas fa-bug" theme="error" />
            <StatsCard v-else-if="card.id === 'traffic'" :value="trafficStats.http_count" :label="t('dashboard.trafficStats')" :subtitle="t('dashboard.httpRequests')" icon="fas fa-exchange-alt" theme="info" />
            <StatsCard v-else-if="card.id === 'ai'" :value="totalTokensFormatted" :label="t('dashboard.aiUsage')" :subtitle="t('dashboard.totalTokens')" icon="fas fa-robot" theme="success" />
          </template>

          <!-- 常规内容卡片类型 -->
          <div v-else class="card bg-base-100 shadow-lg border border-base-200 group h-full">
            <div class="card-body p-5">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-base font-bold flex items-center gap-2">
                  <i :class="getCardIcon(card.id)"></i>
                  {{ t(card.title) }}
                </h3>
                <div class="flex gap-1">
                  <router-link v-if="card.category === 'list'" :to="card.id === 'recent_tasks' ? '/scan-tasks' : '/vulnerabilities'" class="btn btn-ghost btn-xs">
                    {{ t('common.viewAll') }}
                  </router-link>
                  <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button class="btn btn-ghost btn-xs btn-circle cursor-move"><i class="fas fa-grip-lines"></i></button>
                    <button @click="toggleCard(card.id)" class="btn btn-ghost btn-xs btn-circle text-error"><i class="fas fa-times"></i></button>
                  </div>
                </div>
              </div>

              <!-- 图表内容 -->
              <template v-if="card.category === 'chart'">
                <div class="h-64">
                  <template v-if="card.id === 'vuln_severity'">
                    <div class="flex flex-col md:flex-row items-center justify-around h-full">
                      <div class="w-44 h-44">
                        <Doughnut v-if="vulnerabilitySeverityData.datasets[0].data.some(v => v > 0)" :data="vulnerabilitySeverityData" :options="chartOptions" />
                        <div v-else class="flex items-center justify-center h-full opacity-30 italic text-sm">{{ t('common.noData') }}</div>
                      </div>
                      <div class="mt-4 md:mt-0 space-y-1.5 min-w-[140px]">
                        <div v-for="(count, idx) in vulnerabilitySeverityData.datasets[0].data" :key="idx" class="flex items-center justify-between text-xs font-medium">
                          <div class="flex items-center gap-2">
                            <div :style="{ backgroundColor: vulnerabilitySeverityColors[idx] }" class="w-2.5 h-2.5 rounded-full shadow-sm"></div>
                            <span class="opacity-70">{{ vulnerabilitySeverityLabels[idx] }}</span>
                          </div>
                          <span class="badge badge-ghost badge-sm font-mono">{{ count }}</span>
                        </div>
                      </div>
                    </div>
                  </template>
                  <template v-else-if="card.id === 'asset_distribution'">
                    <Bar v-if="assetTypeData.datasets[0].data.length" :data="assetTypeData" :options="barChartOptions" />
                    <div v-else class="flex items-center justify-center h-full opacity-30 italic text-sm">{{ t('common.noData') }}</div>
                  </template>
                </div>
              </template>

              <!-- 详情统计内容 -->
              <template v-else-if="card.category === 'info'">
                <!-- 数据库统计 -->
                <div v-if="card.id === 'db_stats'" class="space-y-3">
                  <div class="flex justify-between items-center p-2.5 bg-base-200/50 rounded-xl">
                    <span class="text-sm opacity-60">{{ t('dashboard.dbSize') }}</span>
                    <span class="font-bold font-mono">{{ dbStats.db_size_formatted || '0 B' }}</span>
                  </div>
                  <div class="grid grid-cols-2 gap-3">
                    <div class="p-3 bg-base-200/50 rounded-xl text-center">
                      <div class="text-xs opacity-50 mb-1">{{ t('dashboard.tasks') }}</div>
                      <div class="text-xl font-black">{{ dbStats.scan_tasks_count || 0 }}</div>
                    </div>
                    <div class="p-3 bg-base-200/50 rounded-xl text-center">
                      <div class="text-xs opacity-50 mb-1">{{ t('dashboard.conversations') }}</div>
                      <div class="text-xl font-black">{{ dbStats.conversations_count || 0 }}</div>
                    </div>
                  </div>
                  <div class="flex items-center gap-2 px-1 text-[10px] opacity-40 uppercase tracking-wider font-bold">
                    <i class="fas fa-history"></i>
                    {{ t('dashboard.lastBackup') }}: {{ dbStats.last_backup || '-' }}
                  </div>
                </div>

                <!-- AI 使用详情 -->
                <div v-else-if="card.id === 'ai_providers'" class="overflow-x-auto">
                  <table class="table table-xs w-full">
                    <thead>
                      <tr class="bg-base-200/50">
                        <th class="rounded-l-lg">{{ t('common.provider') }}</th>
                        <th>{{ t('common.total') }}</th>
                        <th class="rounded-r-lg text-right">{{ t('common.cost') }}</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(data, provider) in aiUsageStats" :key="provider" class="hover:bg-base-200/30 transition-colors">
                        <td class="font-bold py-2.5">{{ provider }}</td>
                        <td class="font-mono">{{ formatNumber(data.total_tokens) }}</td>
                        <td class="text-right font-mono text-success">${{ data.cost.toFixed(4) }}</td>
                      </tr>
                      <tr v-if="Object.keys(aiUsageStats).length === 0">
                        <td colspan="3" class="text-center py-6 opacity-30 italic text-xs">{{ t('common.noData') }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>

                <!-- 应用工具统计 -->
                <div v-else-if="card.id === 'tools_stats'" class="space-y-4">
                  <div class="flex flex-col items-center py-2">
                    <div class="radial-progress text-primary mb-3" :style="`--value:${(toolStats.builtin_tools / toolStats.total_tools) * 100 || 0}; --size:5rem; --thickness: 0.5rem;`" role="progressbar">
                      <span class="text-sm font-black">{{ toolStats.total_tools || 0 }}</span>
                    </div>
                    <span class="text-xs opacity-50 font-bold uppercase tracking-widest">{{ t('sidebar.Tools') }}</span>
                  </div>
                  <div class="flex justify-center gap-4">
                    <div class="flex flex-col items-center p-2 rounded-lg bg-base-200/50 flex-1">
                      <span class="text-[10px] opacity-50 uppercase font-bold">内置 / 插件</span>
                      <span class="text-sm font-black">{{ toolStats.builtin_tools || 0 }} / {{ toolStats.plugin_tools || 0 }}</span>
                    </div>
                    <div class="flex flex-col items-center p-2 rounded-lg bg-base-200/50 flex-1">
                      <span class="text-[10px] opacity-50 uppercase font-bold">MCP / 工作流</span>
                      <span class="text-sm font-black">{{ toolStats.mcp_tools || 0 }} / {{ toolStats.workflow_tools || 0 }}</span>
                    </div>
                  </div>
                  <div class="flex justify-between items-center px-1 text-[10px] opacity-40 font-bold">
                    <span>平均执行耗时</span>
                    <span>{{ toolStats.avg_time?.toFixed(2) || 0 }}ms</span>
                  </div>
                </div>

                <!-- 插件管理统计 -->
                <div v-else-if="card.id === 'plugin_stats'" class="space-y-4">
                  <div class="grid grid-cols-2 gap-3">
                    <div class="stat p-0 bg-base-200/30 rounded-xl border border-base-200">
                      <div class="stat-title text-[10px] uppercase font-bold px-3 pt-2">{{ t('common.total') }}</div>
                      <div class="stat-value text-2xl px-3 pb-2">{{ pluginStats.total || 0 }}</div>
                    </div>
                    <div class="stat p-0 bg-warning/5 rounded-xl border border-warning/20">
                      <div class="stat-title text-[10px] uppercase font-bold text-warning px-3 pt-2">待审核</div>
                      <div class="stat-value text-2xl text-warning px-3 pb-2">{{ pluginStats.pending_review || 0 }}</div>
                    </div>
                  </div>
                  <div class="flex items-center justify-between text-xs px-1">
                    <span class="opacity-50">已审核插件</span>
                    <div class="flex gap-1">
                      <span class="badge badge-success badge-xs">{{ pluginStats.approved || 0 }}</span>
                      <span class="badge badge-error badge-xs">{{ pluginStats.rejected || 0 }}</span>
                    </div>
                  </div>
                  <progress class="progress progress-primary w-full h-1.5" :value="pluginStats.approved" :max="pluginStats.total"></progress>
                </div>

                <!-- 字典管理统计 -->
                <div v-else-if="card.id === 'dictionary_stats'" class="space-y-4">
                  <div class="flex items-center gap-4 p-3 bg-base-200/50 rounded-xl">
                    <div class="w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center text-primary">
                      <i class="fas fa-book text-xl"></i>
                    </div>
                    <div class="flex-1">
                      <div class="text-2xl font-black leading-none">{{ dictionaryStats.total || 0 }}</div>
                      <div class="text-[10px] opacity-50 uppercase font-bold mt-1">总词库数量</div>
                    </div>
                  </div>
                  <div class="grid grid-cols-2 gap-4">
                    <div class="flex flex-col gap-1">
                      <span class="text-[10px] opacity-50 uppercase font-bold">内置字典</span>
                      <div class="flex items-end gap-1">
                        <span class="text-lg font-bold">{{ dictionaryStats.builtin || 0 }}</span>
                        <span class="text-[10px] mb-1 opacity-40">FILES</span>
                      </div>
                    </div>
                    <div class="flex flex-col gap-1">
                      <span class="text-[10px] opacity-50 uppercase font-bold">自定义</span>
                      <div class="flex items-end gap-1">
                        <span class="text-lg font-bold">{{ dictionaryStats.custom || 0 }}</span>
                        <span class="text-[10px] mb-1 opacity-40">USER</span>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- 知识库管理统计 -->
                <div v-else-if="card.id === 'rag_stats'" class="space-y-4">
                  <div class="flex justify-between items-center px-1">
                    <span class="text-xs font-bold uppercase tracking-widest opacity-50">存储规模</span>
                    <span class="badge badge-primary badge-sm font-mono">{{ ragStats.total_chunks || 0 }} CHUNKS</span>
                  </div>
                  <div class="space-y-2">
                    <div class="p-3 bg-base-200/50 rounded-xl flex items-center justify-between">
                      <div class="flex items-center gap-2">
                        <i class="fas fa-folder text-warning/70"></i>
                        <span class="text-sm font-medium">数据集合</span>
                      </div>
                      <span class="font-black">{{ ragStats.collections_count || 0 }}</span>
                    </div>
                    <div class="p-3 bg-base-200/50 rounded-xl flex items-center justify-between">
                      <div class="flex items-center gap-2">
                        <i class="fas fa-file-alt text-info/70"></i>
                        <span class="text-sm font-medium">文档总数</span>
                      </div>
                      <span class="font-black">{{ ragStats.total_documents || 0 }}</span>
                    </div>
                  </div>
                  <div v-if="ragStats.initialized" class="text-[10px] text-success font-bold flex items-center gap-1.5 px-1 uppercase tracking-wider">
                    <div class="w-1.5 h-1.5 rounded-full bg-success animate-pulse"></div>
                    Index Service Ready
                  </div>
                </div>

                <!-- 工作流工作室统计 -->
                <div v-else-if="card.id === 'workflow_stats'" class="space-y-4">
                  <div class="flex items-center justify-around py-2">
                    <div class="text-center">
                      <div class="text-2xl font-black text-primary">{{ workflowStats.total || 0 }}</div>
                      <div class="text-[10px] opacity-40 uppercase font-bold mt-0.5">流程定义</div>
                    </div>
                    <div class="divider divider-horizontal mx-0 opacity-10"></div>
                    <div class="text-center">
                      <div class="text-2xl font-black text-info">{{ workflowStats.tools || 0 }}</div>
                      <div class="text-[10px] opacity-40 uppercase font-bold mt-0.5">集成工具</div>
                    </div>
                  </div>
                  <div class="p-3 bg-primary/5 rounded-xl border border-primary/10 flex items-center justify-between hover:bg-primary/10 transition-colors cursor-pointer group/link">
                    <div class="flex items-center gap-2">
                      <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
                        <i class="fas fa-play text-xs"></i>
                      </div>
                      <div>
                        <div class="text-xs font-bold leading-none">正在运行</div>
                        <div class="text-[10px] opacity-40 mt-1">{{ workflowStats.active_runs || 0 }} Active Executions</div>
                      </div>
                    </div>
                    <i class="fas fa-chevron-right text-[10px] opacity-30 group-hover/link:translate-x-1 transition-transform"></i>
                  </div>
                </div>

                <!-- 流量分析统计 -->
                <div v-else-if="card.id === 'traffic_analysis'" class="space-y-4">
                  <div class="flex items-center justify-between p-3 bg-base-200/50 rounded-xl">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full" :class="trafficStats.running ? 'bg-success animate-pulse' : 'bg-error'"></div>
                      <span class="text-sm font-bold">代理状态</span>
                    </div>
                    <span class="badge badge-sm" :class="trafficStats.running ? 'badge-success' : 'badge-error'">
                      {{ trafficStats.running ? 'RUNNING' : 'STOPPED' }}
                    </span>
                  </div>
                  <div class="grid grid-cols-2 gap-3">
                    <div class="p-3 bg-base-200/50 rounded-xl">
                      <div class="text-[10px] opacity-50 uppercase font-bold mb-1">WS 连接</div>
                      <div class="text-xl font-black">{{ trafficStats.ws_connection_count || 0 }}</div>
                    </div>
                    <div class="p-3 bg-base-200/50 rounded-xl">
                      <div class="text-[10px] opacity-50 uppercase font-bold mb-1">WS 消息</div>
                      <div class="text-xl font-black">{{ trafficStats.ws_message_count || 0 }}</div>
                    </div>
                  </div>
                  <div class="flex items-center gap-2 px-1 text-[10px] opacity-40 font-bold uppercase tracking-wider">
                    <i class="fas fa-network-wired"></i>
                    端口: {{ trafficStats.port || '-' }}
                  </div>
                </div>
              </template>

              <!-- 列表内容类型 -->
              <template v-else-if="card.category === 'list'">
                <div class="space-y-2.5">
                  <!-- 最近活动列表 -->
                  <template v-if="card.id === 'recent_tasks'">
                    <div v-for="task in recentTasks" :key="task.id" class="flex items-center justify-between p-3 bg-base-200/40 rounded-xl hover:bg-base-200/80 transition-all cursor-pointer border border-transparent hover:border-base-300">
                      <div class="flex items-center gap-3">
                        <div :class="getTaskIconClass(task.status)" class="w-9 h-9 rounded-xl flex items-center justify-center shadow-sm">
                          <i :class="getTaskIcon(task.status)" class="text-sm"></i>
                        </div>
                        <div>
                          <p class="font-bold text-sm leading-tight">{{ task.name }}</p>
                          <p class="text-[10px] opacity-40 truncate max-w-[150px] mt-1">{{ task.target }}</p>
                        </div>
                      </div>
                      <div class="text-right">
                        <div :class="getStatusBadgeClass(task.status)" class="badge badge-sm font-bold scale-75 origin-right">
                          {{ formatStatus(task.status) }}
                        </div>
                        <p class="text-[10px] opacity-40 font-mono mt-0.5">{{ formatTime(task.createdAt) }}</p>
                      </div>
                    </div>
                  </template>

                  <!-- 最新发现列表 -->
                  <template v-else-if="card.id === 'recent_vulns'">
                    <div v-for="vuln in recentVulns.slice(0, 5)" :key="vuln.id" class="flex items-center justify-between p-3 bg-base-200/40 rounded-xl hover:bg-base-200/80 transition-all cursor-pointer border border-transparent hover:border-base-300">
                      <div class="flex items-center gap-3">
                        <div :class="getSeverityIconClass(vuln.severity)" class="w-9 h-9 rounded-xl flex items-center justify-center shadow-sm">
                          <i class="fas fa-bug text-sm"></i>
                        </div>
                        <div>
                          <p class="font-bold text-sm leading-tight">{{ vuln.title }}</p>
                          <p class="text-[10px] opacity-40 truncate max-w-[150px] mt-1">{{ vuln.target }}</p>
                        </div>
                      </div>
                      <div class="text-right">
                        <div :class="getSeverityBadgeClass(vuln.severity)" class="badge badge-sm font-bold scale-75 origin-right">
                          {{ formatSeverity(vuln.severity) }}
                        </div>
                        <p class="text-[10px] opacity-40 font-mono mt-0.5">{{ formatTime(vuln.discoveredAt) }}</p>
                      </div>
                    </div>
                  </template>

                  <div v-if="(card.id === 'recent_tasks' ? recentTasks : recentVulns).length === 0" class="text-center py-12 opacity-20 italic text-sm">
                    {{ t('common.noData') }}
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- 配置悬浮按钮 (仅在有卡片被隐藏时显示) -->
    <div v-if="hiddenCardsCount > 0" class="fixed bottom-6 right-6 z-40">
      <div class="indicator">
        <span class="indicator-item badge badge-primary badge-sm">{{ hiddenCardsCount }}</span> 
        <div class="dropdown dropdown-top dropdown-end">
          <button tabindex="0" class="btn btn-primary btn-circle shadow-2xl hover:scale-110 transition-transform">
            <i class="fas fa-plus"></i>
          </button>
          <div tabindex="0" class="dropdown-content z-[50] card card-compact w-64 p-2 shadow-2xl bg-base-100 border border-primary/20 mb-4">
            <div class="card-body">
              <h3 class="font-bold text-sm mb-2 border-b pb-2">恢复已隐藏卡片</h3>
              <div class="space-y-1">
                <template v-for="card in cards" :key="card.id">
                  <button v-if="!card.visible" @click="toggleCard(card.id)" class="btn btn-ghost btn-xs w-full justify-between hover:bg-primary/10">
                    <span class="truncate">{{ t(card.title) }}</span>
                    <i class="fas fa-plus-circle text-primary"></i>
                  </button>
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import StatsCard from '../components/Dashboard/StatsCard.vue'
import { Doughnut, Bar } from 'vue-chartjs'
import { 
  Chart as ChartJS, 
  Title, 
  Tooltip, 
  Legend, 
  ArcElement, 
  CategoryScale, 
  LinearScale, 
  BarElement 
} from 'chart.js'

// 注册 Chart.js 组件
ChartJS.register(Title, Tooltip, Legend, ArcElement, CategoryScale, LinearScale, BarElement)

const { t } = useI18n()

// ==================== 状态与配置 ====================

const isLoading = ref(false)
const draggedCardId = ref<string | null>(null)
const draggedOverCardId = ref<string | null>(null)

interface DashboardCard {
  id: string
  title: string
  visible: boolean
  order: number
  category: 'stats' | 'chart' | 'list' | 'info'
}

const DASHBOARD_CONFIG_KEY = 'sentinel-ai:dashboard:config'

const defaultCards: DashboardCard[] = [
  // Stats
  { id: 'assets', title: 'assetManagement.title', visible: true, order: 0, category: 'stats' },
  { id: 'vulns', title: 'dashboard.vulnerabilitiesFound', visible: true, order: 1, category: 'stats' },
  { id: 'traffic', title: 'dashboard.trafficStats', visible: true, order: 2, category: 'stats' },
  { id: 'ai', title: 'dashboard.aiUsage', visible: true, order: 3, category: 'stats' },
  
  // Charts
  { id: 'vuln_severity', title: 'dashboard.vulnerabilitySeverity', visible: true, order: 4, category: 'chart' },
  { id: 'asset_distribution', title: 'dashboard.vulnerabilityType', visible: true, order: 5, category: 'chart' },
  
  // Info/Stats Details
  { id: 'db_stats', title: 'dashboard.databaseStats', visible: true, order: 6, category: 'info' },
  { id: 'ai_providers', title: 'dashboard.aiUsage', visible: true, order: 7, category: 'info' },
  { id: 'tools_stats', title: 'sidebar.Tools', visible: true, order: 8, category: 'info' },
  { id: 'plugin_stats', title: 'plugins.title', visible: true, order: 9, category: 'info' },
  { id: 'dictionary_stats', title: 'dictionary.title', visible: true, order: 10, category: 'info' },
  { id: 'rag_stats', title: 'rag.title', visible: true, order: 11, category: 'info' },
  { id: 'workflow_stats', title: 'workflow.title', visible: true, order: 12, category: 'info' },
  { id: 'traffic_analysis', title: 'dashboard.trafficStats', visible: true, order: 13, category: 'info' },
  
  // Lists
  { id: 'recent_tasks', title: 'dashboard.recentActivity', visible: true, order: 14, category: 'list' },
  { id: 'recent_vulns', title: 'dashboard.recentVulnerabilities', visible: true, order: 15, category: 'list' }
]

const cards = ref<DashboardCard[]>([])

const sortedCards = computed(() => [...cards.value].sort((a, b) => a.order - b.order))
const visibleCards = computed(() => sortedCards.value.filter(c => c.visible))
const hiddenCardsCount = computed(() => cards.value.filter(c => !c.visible).length)

const getCardGridClass = (card: DashboardCard) => {
  switch (card.category) {
    case 'stats':
      return 'col-span-12 sm:col-span-6 lg:col-span-3'
    case 'chart':
      return 'col-span-12 lg:col-span-6'
    case 'info':
      if (card.id === 'ai_providers') return 'col-span-12 md:col-span-12 lg:col-span-6'
      return 'col-span-12 md:col-span-6 lg:col-span-4'
    case 'list':
      return 'col-span-12 lg:col-span-6'
    default:
      return 'col-span-12'
  }
}

// ==================== 业务数据 ====================

const stats = ref({ discoveredAssets: 0, vulnerabilities: 0, criticalVulns: 0, activeScans: 0 })
const trafficStats = ref({ http_count: 0, ws_connection_count: 0, ws_message_count: 0, running: false, port: 0 })
const aiUsageStats = ref<Record<string, any>>({})
const dbStats = ref<any>({})
const assetsByType = ref<Record<string, number>>({})
const findingsBySeverity = ref({ critical: 0, high: 0, medium: 0, low: 0, info: 0 })
const recentTasks = ref<any[]>([])
const recentVulns = ref<any[]>([])

// 新增数据
const pluginStats = ref<any>({})
const dictionaryStats = ref<any>({})
const ragStats = ref<any>({})
const workflowStats = ref<any>({ total: 0, tools: 0 })
const toolStats = ref<any>({ total: 0, enabled: 0 })

// ==================== 图表配置 ====================

const vulnerabilitySeverityColors = ['#f87171', '#fb923c', '#fbbf24', '#60a5fa', '#9ca3af']
const vulnerabilitySeverityLabels = computed(() => [
  t('riskLevels.critical'), t('riskLevels.high'), t('riskLevels.medium'), t('riskLevels.low'), t('riskLevels.info')
])

const vulnerabilitySeverityData = computed(() => ({
  labels: vulnerabilitySeverityLabels.value,
  datasets: [{ backgroundColor: vulnerabilitySeverityColors, data: [findingsBySeverity.value.critical, findingsBySeverity.value.high, findingsBySeverity.value.medium, findingsBySeverity.value.low, findingsBySeverity.value.info], borderOffset: 4, hoverOffset: 8, borderRadius: 4 }]
}))

const assetTypeData = computed(() => {
  const entries = Object.entries(assetsByType.value).filter(([_, count]) => count > 0)
  return {
    labels: entries.map(([type]) => t(`assetTypes.${type}`) || type),
    datasets: [{ label: t('assetManagement.totalAssets'), backgroundColor: 'rgba(99, 102, 241, 0.8)', hoverBackgroundColor: '#6366f1', data: entries.map(([_, count]) => count), borderRadius: 6, barThickness: 20 }]
  }
})

const chartOptions = { responsive: true, maintainAspectRatio: false, plugins: { legend: { display: false }, tooltip: { cornerRadius: 8, padding: 10 } }, cutout: '70%' }
const barChartOptions = { responsive: true, maintainAspectRatio: false, plugins: { legend: { display: false } }, scales: { y: { beginAtZero: true, grid: { color: 'rgba(0,0,0,0.05)', drawTicks: false }, ticks: { font: { size: 10 } } }, x: { grid: { display: false }, ticks: { font: { size: 10 } } } } }

// ==================== 方法 ====================

const loadConfig = () => {
  const saved = localStorage.getItem(DASHBOARD_CONFIG_KEY)
  if (saved) {
    try {
      const parsed = JSON.parse(saved)
      cards.value = defaultCards.map(def => {
        const found = parsed.find((p: any) => p.id === def.id)
        return found ? { ...def, ...found } : def
      })
    } catch (e) { cards.value = [...defaultCards] }
  } else { cards.value = [...defaultCards] }
}

const saveConfig = () => localStorage.setItem(DASHBOARD_CONFIG_KEY, JSON.stringify(cards.value))
const resetConfig = () => { cards.value = [...defaultCards]; saveConfig() }

const handleDragStart = (id: string, event: DragEvent) => {
  draggedCardId.value = id
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.dropEffect = 'move'
  }
}

const handleDragEnd = () => {
  draggedCardId.value = null
  draggedOverCardId.value = null
}

const handleDrop = (targetId: string) => {
  if (!draggedCardId.value || draggedCardId.value === targetId) {
    handleDragEnd()
    return
  }

  // 获取当前排序后的完整列表
  const allCards = [...cards.value].sort((a, b) => a.order - b.order)
  const dIdx = allCards.findIndex(c => c.id === draggedCardId.value)
  const tIdx = allCards.findIndex(c => c.id === targetId)

  if (dIdx !== -1 && tIdx !== -1) {
    // 执行真正的移位操作
    const [draggedCard] = allCards.splice(dIdx, 1)
    allCards.splice(tIdx, 0, draggedCard)
    
    // 重新分配连续的 order 索引
    allCards.forEach((card, index) => {
      card.order = index
    })
    
    // 更新原始响应式数据
    cards.value = allCards
    saveConfig()
  }
  
  handleDragEnd()
}

const toggleCard = (id: string) => {
  const card = cards.value.find(c => c.id === id)
  if (card) { card.visible = !card.visible; saveConfig() }
}

const refreshData = async () => {
  isLoading.value = true
  try {
    await Promise.all([
      fetchAssetStats(), fetchVulnerabilityStats(), fetchTrafficStats(), 
      fetchAiStats(), fetchDbStats(), fetchRecentData(), fetchNewStats()
    ])
  } catch (error) { console.error('Refresh dashboard failed:', error) }
  finally { isLoading.value = false }
}

const fetchAssetStats = async () => {
  try {
    const assetStats = await invoke<any>('get_asset_stats')
    stats.value.discoveredAssets = assetStats.total_assets || 0
    assetsByType.value = assetStats.by_type || {}
  } catch (e) { console.error(e) }
}

const fetchVulnerabilityStats = async () => {
  try {
    const severities = ['critical', 'high', 'medium', 'low', 'info']
    const counts = await Promise.all(severities.map(async (sev) => {
      const resp = await invoke<any>('count_findings', { severityFilter: sev })
      return resp.success ? resp.data : 0
    }))
    findingsBySeverity.value = { critical: counts[0], high: counts[1], medium: counts[2], low: counts[3], info: counts[4] }
    stats.value.vulnerabilities = counts.reduce((a, b) => a + b, 0)
    stats.value.criticalVulns = counts[0]
  } catch (e) { console.error(e) }
}

const fetchTrafficStats = async () => {
  try {
    const [historyResp, proxyResp] = await Promise.all([
      invoke<any>('get_history_stats'),
      invoke<any>('get_proxy_status')
    ])
    if (historyResp.success) {
      trafficStats.value = { 
        ...trafficStats.value,
        ...historyResp.data 
      }
    }
    if (proxyResp.success) {
      trafficStats.value = {
        ...trafficStats.value,
        running: proxyResp.data.running,
        port: proxyResp.data.port
      }
    }
  } catch (e) { console.error(e) }
}

const fetchAiStats = async () => {
  try {
    aiUsageStats.value = await invoke<Record<string, any>>('get_ai_usage_stats')
  } catch (e) { console.error(e) }
}

const fetchDbStats = async () => {
  try { dbStats.value = await invoke<any>('get_database_statistics') } catch (e) { console.error(e) }
}

const fetchRecentData = async () => {
  try {
    const tasks = await invoke<any[]>('get_scan_tasks')
    recentTasks.value = tasks.slice(0, 5).map(t => ({ id: t.id, name: t.name, target: t.target, status: t.status, createdAt: new Date(t.created_at) }))
    const findingsResp = await invoke<any>('list_findings', { limit: 5, offset: 0, severityFilter: null })
    if (findingsResp.success) {
      recentVulns.value = findingsResp.data.map((f: any) => ({ id: f.id, title: f.title, target: f.url || f.target || '-', severity: f.severity, discoveredAt: new Date(f.created_at || f.last_seen_at) }))
    }
  } catch (e) { console.error(e) }
}

const fetchNewStats = async () => {
  try {
    const [pluginRes, dicts, ragRes, workflows, workflowTools, serverStats, usageStats, workflowRuns] = await Promise.all([
      invoke<any>('get_plugin_review_statistics'),
      invoke<any[]>('get_dictionaries', {}),
      invoke<any>('get_rag_status'),
      invoke<any[]>('list_workflow_definitions', { is_template: false }),
      invoke<any[]>('list_workflow_tools'),
      invoke<any>('get_tool_server_stats'),
      invoke<any>('get_tool_usage_stats'),
      invoke<any[]>('list_workflow_runs')
    ])

    if (pluginRes.success) pluginStats.value = pluginRes.data
    dictionaryStats.value = { 
      total: dicts.length, 
      builtin: dicts.filter(d => d.is_builtin).length, 
      custom: dicts.filter(d => !d.is_builtin).length 
    }
    ragStats.value = ragRes
    workflowStats.value = { 
      total: workflows.length, 
      tools: workflowTools.length,
      active_runs: workflowRuns.filter((r: any) => r.status === 'running').length
    }
    toolStats.value = { 
      total_tools: serverStats.total_tools,
      builtin_tools: serverStats.builtin_tools,
      mcp_tools: serverStats.mcp_tools,
      plugin_tools: serverStats.plugin_tools,
      workflow_tools: serverStats.workflow_tools,
      avg_time: usageStats.total_executions > 0 
        ? Object.values(usageStats.by_tool as Record<string, any>).reduce((acc, curr) => acc + curr.avg_execution_time_ms, 0) / Object.keys(usageStats.by_tool).length 
        : 0
    }
  } catch (e) { console.error('Failed to fetch new stats:', e) }
}

const totalTokensFormatted = computed(() => {
  let total = 0
  Object.values(aiUsageStats.value).forEach((s: any) => { total += s.total_tokens })
  if (total >= 1000000) return (total / 1000000).toFixed(1) + 'M'
  if (total >= 1000) return (total / 1000).toFixed(1) + 'k'
  return total.toString()
})

const getCardIcon = (id: string) => {
  switch (id) {
    case 'db_stats': return 'fas fa-database text-warning'
    case 'ai_providers': return 'fas fa-brain text-success'
    case 'tools_stats': return 'fas fa-tools text-primary'
    case 'plugin_stats': return 'fas fa-puzzle-piece text-secondary'
    case 'dictionary_stats': return 'fas fa-book text-accent'
    case 'rag_stats': return 'fas fa-microchip text-info'
    case 'workflow_stats': return 'fas fa-project-diagram text-primary'
    case 'traffic_analysis': return 'fas fa-exchange-alt text-info'
    case 'vuln_severity': return 'fas fa-shield-virus text-error'
    case 'asset_distribution': return 'fas fa-sitemap text-primary'
    case 'recent_tasks': return 'fas fa-history text-primary'
    case 'recent_vulns': return 'fas fa-search text-error'
    default: return 'fas fa-info-circle'
  }
}

const formatNumber = (num: number) => num?.toLocaleString() || '0'
const formatStatus = (s: string) => s ? t(`common.${s.toLowerCase()}`) || s : '-'
const formatSeverity = (s: string) => s ? t(`riskLevels.${s.toLowerCase()}`) || s : '-'
const formatTime = (d: any) => {
  if (!d) return '-'
  const diff = Math.floor((Date.now() - new Date(d).getTime()) / 60000)
  if (diff < 1) return t('common.justNow')
  if (diff < 60) return `${diff}m ago`
  if (diff < 1440) return `${Math.floor(diff / 60)}h ago`
  return `${Math.floor(diff / 1440)}d ago`
}

const getTaskIconClass = (s: string) => {
  const map: any = { Running: 'bg-warning/20 text-warning', Completed: 'bg-success/20 text-success', Failed: 'bg-error/20 text-error', Pending: 'bg-info/20 text-info' }
  return map[s] || 'bg-base-300'
}

const getTaskIcon = (s: string) => {
  const map: any = { Running: 'fas fa-spinner fa-spin', Completed: 'fas fa-check', Failed: 'fas fa-times', Pending: 'fas fa-clock' }
  return map[s] || 'fas fa-question'
}

const getStatusBadgeClass = (s: string) => {
  const map: any = { Running: 'badge-warning', Completed: 'badge-success', Failed: 'badge-error', Pending: 'badge-info' }
  return map[s] || 'badge-ghost'
}

const getSeverityIconClass = (s: string) => {
  const sev = s?.toLowerCase()
  if (sev === 'critical' || sev === 'high') return 'bg-error/20 text-error'
  if (sev === 'medium') return 'bg-warning/20 text-warning'
  return 'bg-info/20 text-info'
}

const getSeverityBadgeClass = (s: string) => {
  const sev = s?.toLowerCase()
  if (sev === 'critical' || sev === 'high') return 'badge-error'
  if (sev === 'medium') return 'badge-warning'
  return 'badge-info'
}

onMounted(() => {
  loadConfig()
  refreshData()
})
</script>

<style scoped>
.card {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  border-radius: 1rem;
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: 0 25px 50px -12px rgb(0 0 0 / 0.25);
  border-color: hsl(var(--p) / 0.3);
}

[draggable="true"] {
  cursor: grab;
  user-select: none;
}

[draggable="true"]:active {
  cursor: grabbing;
}

.dragging {
  opacity: 0.5;
}

.drag-over {
  border: 2px dashed hsl(var(--p));
  background: hsla(var(--p), 0.05);
}

/* 隐藏滚动条但保留功能 */
.overflow-y-auto {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--bc) / 0.1) transparent;
}

.overflow-y-auto::-webkit-scrollbar {
  width: 4px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.1);
  border-radius: 10px;
}
</style>
