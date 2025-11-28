<template>
  <div class="h-full bg-base-200 overflow-y-auto overflow-x-hidden transition-all duration-300 ease-in-out flex flex-col">
    
    <!-- 折叠模式显示图标导航 -->
    <div v-if="collapsed" class="flex flex-col items-center space-y-2 py-4 flex-1">
      <!-- 主要功能菜单 -->
      <router-link 
        v-for="item in mainMenuItems" 
        :key="item.path"
        :to="item.path" 
        class="btn btn-ghost btn-circle tooltip tooltip-right" 
        :data-tip="item.name"
      >
        <i :class="`${item.icon} text-xl`"></i>
      </router-link>
      
      <div class="divider divider-neutral my-2"></div>
      
      <!-- 工具菜单 -->
      <router-link 
        v-for="item in toolMenuItems" 
        :key="item.path"
        :to="item.path" 
        class="btn btn-ghost btn-circle tooltip tooltip-right" 
        :data-tip="item.name"
      >
        <i :class="`${item.icon} text-xl`"></i>
      </router-link>
    </div>
    
    <!-- 展开模式显示完整侧边栏 -->
    <div v-else class="flex flex-col h-full">
      <!-- 主导航菜单 -->
      <div class="flex-1 p-4 space-y-2">
        <!-- 核心功能区 -->
        <div class="mb-6">
          <h3 class="text-xs font-semibold text-base-content/60 uppercase tracking-wider mb-3 px-2">
            {{ t('sidebar.coreFeatures', '核心功能') }}
          </h3>
          <ul class="menu menu-sm space-y-1">
            <li v-for="item in mainMenuItems" :key="item.path">
              <router-link 
                :to="item.path" 
                class="rounded-lg flex items-center gap-3 px-3 py-2 hover:bg-base-300 transition-colors"
                :class="{ 'bg-primary/10 text-primary border-r-2 border-primary': route.path === item.path }"
              >
                <i :class="`${item.icon} text-lg`"></i>
                <span class="font-medium">{{ item.name }}</span>
                <span v-if="item.badge" class="badge badge-sm ml-auto" :class="item.badgeClass">
                  {{ item.badge }}
                </span>
              </router-link>
            </li>
          </ul>
        </div>

        <!-- 工具与管理区 -->
        <div class="mb-6">
          <h3 class="text-xs font-semibold text-base-content/60 uppercase tracking-wider mb-3 px-2">
            {{ t('sidebar.toolsManagement', '工具与管理') }}
          </h3>
          <ul class="menu menu-sm space-y-1">
            <li v-for="item in toolMenuItems" :key="item.path">
              <router-link 
                :to="item.path" 
                class="rounded-lg flex items-center gap-3 px-3 py-2 hover:bg-base-300 transition-colors"
                :class="{ 'bg-primary/10 text-primary border-r-2 border-primary': route.path === item.path }"
              >
                <i :class="`${item.icon} text-lg`"></i>
                <span class="font-medium">{{ item.name }}</span>
                <span v-if="item.badge" class="badge badge-sm ml-auto" :class="item.badgeClass">
                  {{ item.badge }}
                </span>
              </router-link>
            </li>
          </ul>
        </div>

        <!-- 系统设置区 -->
        <div>
          <h3 class="text-xs font-semibold text-base-content/60 uppercase tracking-wider mb-3 px-2">
            {{ t('sidebar.systemSettings', '系统设置') }}
          </h3>
          <ul class="menu menu-sm space-y-1">
            <li v-for="item in systemMenuItems" :key="item.path">
              <router-link 
                :to="item.path" 
                class="rounded-lg flex items-center gap-3 px-3 py-2 hover:bg-base-300 transition-colors"
                :class="{ 'bg-primary/10 text-primary border-r-2 border-primary': route.path === item.path }"
              >
                <i :class="`${item.icon} text-lg`"></i>
                <span class="font-medium">{{ item.name }}</span>
              </router-link>
            </li>
          </ul>
        </div>
      </div>

      <!-- 底部状态信息 -->
      <div class="p-4 border-t border-base-300">
        <!-- 当前任务状态 -->
        <div class="bg-base-100 rounded-lg p-3 mb-3">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium">{{ t('sidebar.currentTask', '当前任务') }}</span>
            <div class="badge badge-primary badge-sm">{{ t('sidebar.running') }}</div>
          </div>
          <div class="text-xs text-base-content/70 mb-2">{{ t('sidebar.scanning') }} example.com</div>
          <progress class="progress progress-primary w-full h-2" :value="taskProgress" max="100"></progress>
          <div class="text-xs text-center mt-1 opacity-70">{{ taskProgress }}% - {{ t('sidebar.remaining') }} {{ remainingTime }}</div>
        </div>

        <!-- 今日统计 -->
        <div class="grid grid-cols-2 gap-2 text-center">
          <div class="bg-base-100 rounded-lg p-2">
            <div class="text-lg font-bold text-error">{{ todayVulns }}</div>
            <div class="text-xs opacity-70">{{ t('sidebar.vulnerabilitiesFound') }}</div>
          </div>
          <div class="bg-base-100 rounded-lg p-2">
            <div class="text-lg font-bold text-success">{{ completedTasks }}</div>
            <div class="text-xs opacity-70">{{ t('sidebar.completedTasks') }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

// 接收折叠状态
const props = defineProps({
  collapsed: {
    type: Boolean,
    default: false
  }
});

// 初始化i18n和路由
const { t } = useI18n()
const route = useRoute()

// 主要功能菜单项
const mainMenuItems = computed(() => [
  {
    path: '/dashboard',
    name: t('sidebar.dashboard', '仪表盘'),
    icon: 'fas fa-home',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/security-center',
    name: t('sidebar.securityCenter', '安全中心'),
    icon: 'fas fa-shield-alt',
    badge: taskStats.value.running > 0 ? taskStats.value.running.toString() : null,
    badgeClass: 'badge-primary'
  },
  {
    path: '/passive-scan',
    name: t('sidebar.passive', '被动扫描'),
    icon: 'fas fa-satellite-dish',
    badge: null,
    badgeClass: 'badge-info'
  },
  {
    path: '/agent-manager',
    name: t('sidebar.agentManager', 'Agent管理'),
    icon: 'fas fa-robot',
    badge: null,
    badgeClass: 'badge-accent'
  },
  {
    path: '/ai-assistant',
    name: t('sidebar.aiAssistant', 'AI助手'),
    icon: 'fas fa-brain',
    badge: null,
    badgeClass: ''
  },

  {
    path: '/workflow-studio',
    name: t('sidebar.workflowStudio', '工作流工作室'),
    icon: 'fas fa-project-diagram',
    badge: null,
    badgeClass: ''
  }
])

// 工具与管理菜单项
const toolMenuItems = computed(() => [
  // {
  //   path: '/plan-execute',
  //   name: 'Plan-and-Execute 演示',
  //   icon: 'fas fa-project-diagram',
  //   badge: 'BETA',
  //   badgeClass: 'badge-warning'
  // },
  // {
  //   path: '/rewoo-test',
  //   name: 'ReWOO 架构测试',
  //   icon: 'fas fa-cogs',
  //   badge: 'TEST',
  //   badgeClass: 'badge-info'
  // },
  // {
  //   path: '/llm-compiler-test',
  //   name: 'LLMCompiler 引擎测试',
  //   icon: 'fas fa-microchip',
  //   badge: 'NEW',
  //   badgeClass: 'badge-success'
  // },
  {
    path: '/workflow-runs',
    name: t('sidebar.workflowRuns', '运行历史'),
    icon: 'fas fa-history',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/rag-management',
    name: t('sidebar.ragManagement', '知识库管理'),
    icon: 'fas fa-database',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/mcp-tools',
    name: t('sidebar.Tools', '应用工具'),
    icon: 'fas fa-tools',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/dictionary',
    name: t('sidebar.dictionary', '字典管理'),
    icon: 'fas fa-book',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/plugins',
    name: t('sidebar.plugins', '插件管理'),
    icon: 'fas fa-puzzle-piece',
    badge: pendingPlugins.value > 0 ? pendingPlugins.value.toString() : null,
    badgeClass: pendingPlugins.value > 0 ? 'badge-warning' : ''
  },
  {
    path: '/prompts',
    name: t('sidebar.promptManagement', 'Prompt管理'),
    icon: 'fas fa-align-left',
    badge: null,
    badgeClass: ''
  }
])

// 系统设置菜单项
const systemMenuItems = computed(() => [
  {
    path: '/settings',
    name: t('sidebar.settings', '系统设置'),
    icon: 'fas fa-cog',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/notifications',
    name: t('sidebar.notifications', '通知管理'),
    icon: 'fas fa-bell',
    badge: null,
    badgeClass: ''
  },
  {
    path: '/performance',
    name: t('sidebar.performance', '性能监控'),
    icon: 'fas fa-chart-line',
    badge: null,
    badgeClass: ''
  }
])

// 任务状态相关
const taskProgress = ref(65)
const remainingTime = ref(t('sidebar.remainingTimeDefault', '2小时15分钟'))
const hasRunningTasks = ref(true)

// 统计数据
const todayVulns = ref(3)
const completedTasks = ref(8)
const pendingPlugins = ref(0)  // 待审核插件数量
const taskStats = ref({
  total: 0,
  running: 0,
  pending: 0,
  completed: 0,
  failed: 0,
  cancelled: 0
})

// 任务统计类型定义
interface TaskStats {
  total: number
  running: number
  pending: number
  completed: number
  failed: number
  cancelled: number
}

// 获取任务统计信息
const loadTaskStats = async () => {
  try {
    const stats = await invoke<TaskStats>('get_scan_task_stats')
    taskStats.value = stats
  } catch (error) {
    console.error('Failed to load task stats:', error)
  }
}

// 加载待审核插件数量
const loadPendingPlugins = async () => {
  try {
    const result = await invoke<any>('get_plugin_statistics')
    pendingPlugins.value = result.pending_review || 0
  } catch (error) {
    console.log('Failed to load pending plugins count:', error)
    // 忽略错误，可能是后端命令还未实现
  }
}

// 模拟数据更新
onMounted(() => {
  // 加载任务统计信息
  loadTaskStats()
  // 加载待审核插件数量
  loadPendingPlugins()
  
  // 定期更新任务统计信息
  // setInterval(() => {
  //   loadTaskStats()
  // }, 10000) // 每10秒更新一次
  
  // 模拟任务进度更新
  setInterval(() => {
    if (hasRunningTasks.value && taskProgress.value < 100) {
      taskProgress.value += Math.random() * 2
      if (taskProgress.value > 100) {
        taskProgress.value = 100
        hasRunningTasks.value = false
      }
    }
  }, 5000)
})
</script>

<style scoped>
/* 自定义滚动条 */
::-webkit-scrollbar {
  width: 4px;
}

::-webkit-scrollbar-track {
  background: var(--fallback-b3,oklch(var(--b3)/1));
  border-radius: 4px;
}

::-webkit-scrollbar-thumb {
  background: var(--fallback-b2,oklch(var(--b2)/1));
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--fallback-n,oklch(var(--n)/1));
}

/* 活动路由样式 */
.router-link-active {
  @apply bg-primary/10 text-primary;
}
</style>
