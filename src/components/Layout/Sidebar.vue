<template>
  <div class="h-screen bg-base-200 overflow-y-auto overflow-x-hidden transition-all duration-300 ease-in-out"
       :class="{'p-4': !collapsed, 'p-2': collapsed}">
    
    <!-- 折叠模式显示图标导航 -->
    <div v-if="collapsed" class="flex flex-col items-center space-y-4 py-4">
      <router-link to="/dashboard" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="仪表盘">
        <i class="fas fa-home text-xl"></i>
      </router-link>
      
      <router-link to="/scan-tasks" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="扫描任务">
        <i class="fas fa-search text-xl"></i>
      </router-link>
      
      <router-link to="/scan-sessions" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="智能扫描会话">
        <i class="fas fa-brain text-xl"></i>
      </router-link>
      
      <router-link to="/vulnerabilities" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="漏洞管理">
        <i class="fas fa-bug text-xl"></i>
      </router-link>
      
      <router-link to="/dictionary" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="字典管理">
        <i class="fas fa-book text-xl"></i>
      </router-link>
      
      <router-link to="/projects" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="赏金项目">
        <i class="fas fa-trophy text-xl"></i>
      </router-link>
      
      <router-link to="/submissions" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="提交记录">
        <i class="fas fa-paper-plane text-xl"></i>
      </router-link>
      
      <router-link to="/earnings" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="收益统计">
        <i class="fas fa-dollar-sign text-xl"></i>
      </router-link>
      
      <router-link to="/mcp-tools" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="MCP工具">
        <i class="fas fa-tools text-xl"></i>
      </router-link>
      
      <div class="divider divider-neutral my-2"></div>
      
      <button @click="toggleAIChat" class="btn btn-ghost btn-circle tooltip tooltip-right" data-tip="AI助手">
        <i class="fas fa-robot text-xl"></i>
      </button>
    </div>
    
    <!-- 展开模式显示完整侧边栏 -->
    <div v-else class="space-y-4">
      <!-- 任务状态 -->
      <div class="card bg-base-100 shadow">
        <div class="card-body p-4">
          <h3 class="card-title text-base font-medium flex items-center">
            <i class="fas fa-tasks mr-2 text-primary"></i>{{ t('scanTasks.taskDetails') }}
          </h3>
          
          <div class="space-y-3 mt-2">
            <div v-for="step in currentTaskSteps" :key="step.name" class="flex items-center justify-between">
              <span class="text-sm">{{ step.name }}</span>
              <div :class="getStepBadgeClass(step.status)" class="badge badge-sm">
                {{ step.status }}
              </div>
            </div>
            
            <div class="mt-4">
              <progress 
                class="progress progress-primary w-full" 
                :value="taskProgress" 
                max="100">
              </progress>
              <div class="text-xs text-center mt-1 opacity-70">
                {{ t('common.time') }}: {{ remainingTime }}
              </div>
            </div>
          </div>
        </div>
      </div>
      <!-- 快速操作 -->
      <div class="card bg-base-100 shadow">
        <div class="card-body p-4">
          <h3 class="card-title text-base font-medium flex items-center">
            <i class="fas fa-bolt mr-2 text-accent"></i>{{ t('dashboard.quickActions') }}
          </h3>
          
          <div class="grid grid-cols-2 gap-2 mt-2">
            <button 
              @click="toggleAIChat" 
              class="btn btn-sm btn-primary">
              <i class="fas fa-robot"></i>{{ t('aiChat.title') }}
            </button>
            <button 
              @click="pauseScan" 
              class="btn btn-sm btn-outline"
              :disabled="!hasRunningTasks">
              <i class="fas fa-pause"></i>{{ t('scanTasks.pause') }}
            </button>
            <button 
              @click="exportResults" 
              class="btn btn-sm btn-outline">
              <i class="fas fa-download"></i>{{ t('scanTasks.export') }}
            </button>
            <button 
              @click="createNewTask" 
              class="btn btn-sm btn-outline">
              <i class="fas fa-plus"></i>{{ t('scanTasks.newScan') }}
            </button>
          </div>
        </div>
      </div>

      <!-- 实时统计 -->
      <div class="card bg-base-100 shadow">
        <div class="card-body p-4">
          <h3 class="card-title text-base font-medium flex items-center">
            <i class="fas fa-chart-bar mr-2 text-info"></i>{{ t('dashboard.todayStats') }}
          </h3>
          
          <div class="stats stats-vertical shadow-sm bg-base-200 mt-2">
            <div class="stat py-1 px-2">
              <div class="stat-title text-xs">{{ t('dashboard.vulnerabilitiesFound') }}</div>
              <div class="stat-value text-lg text-error">{{ todayVulns }}</div>
            </div>
            <div class="stat py-1 px-2">
              <div class="stat-title text-xs">{{ t('dashboard.scanProgress') }}</div>
              <div class="stat-value text-lg text-primary">{{ taskProgress }}%</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

// 接收折叠状态
const props = defineProps({
  collapsed: {
    type: Boolean,
    default: false
  }
});

// 初始化i18n
const { t } = useI18n()

// 任务状态相关
const currentTaskSteps = ref([
  { name: t('scanTasks.targetUrl'), status: t('common.completed') },
  { name: t('vulnerabilities.title'), status: t('common.inProgress') },
  { name: t('common.statistics'), status: t('common.pending') },
  { name: t('scanTasks.viewReport'), status: t('common.pending') }
])

const taskProgress = ref(65)
const remainingTime = ref('2小时15分钟')
const hasRunningTasks = ref(true)

// 统计数据
const todayEarnings = ref(1250)
const todayVulns = ref(3)

// 方法
const toggleAIChat = () => {
  // 触发AI助手聊天框显示
  window.dispatchEvent(new CustomEvent('toggle-ai-chat'))
}

const pauseScan = () => {
  console.log('暂停扫描')
  hasRunningTasks.value = false
}

const exportResults = () => {
  console.log('导出结果')
}

const createNewTask = () => {
  console.log('创建新任务')
}

// 获取步骤状态的样式类
const getStepBadgeClass = (status: string) => {
  switch (status) {
    case t('common.completed'):
      return 'badge-success'
    case t('common.inProgress'):
      return 'badge-primary'
    case t('common.pending'):
      return 'badge-neutral'
    default:
      return 'badge-neutral'
  }
}

// 模拟数据更新
onMounted(() => {
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