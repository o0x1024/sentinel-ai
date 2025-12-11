<template>
  <div class="ai-assistant-view page-content-full h-full flex flex-col bg-base-100 overflow-hidden">
    <!-- 头部控制栏 -->
    <div class="navbar bg-base-200 shadow-sm border-b border-base-300 flex-shrink-0">
      <div class="navbar-start">
        <h1 class="text-xl font-bold flex items-center gap-2">
          <i class="fas fa-robot text-primary"></i>
          {{ t('aiAssistant.title', 'AI智能助手') }}
        </h1>
      </div>
      <div class="navbar-end">
        <div class="flex items-center gap-2">
          <!-- 角色选择器 -->
          <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-sm btn-outline gap-2">
              <i class="fas fa-user-tie"></i>
              {{ selectedRole ? selectedRole.title : t('aiAssistant.selectRole', '选择角色') }}
              <i class="fas fa-chevron-down text-xs"></i>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 md:w-80">
              <li><span class="menu-title">{{ t('aiAssistant.availableRoles', '可用角色') }}</span></li>
              <li @click="handleSelectRole(null)">
                <a class="flex items-center justify-between gap-3" :class="{ 'active': !selectedRole }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-ghost">默认</div>
                    <span>{{ t('aiAssistant.defaultRole', '默认助手') }}</span>
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li v-for="role in roles" :key="role.id" @click="handleSelectRole(role)">
                <a class="flex items-center justify-between gap-3" :class="{ 'active': selectedRole?.id === role.id }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-primary">角色</div>
                    <span class="truncate">{{ role.title }}</span>
                  </div>
                  <div class="text-xs text-base-content/60 truncate max-w-20" :title="role.description">
                    {{ role.description }}
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li @click="showRoleManagement = true">
                <a class="flex items-center gap-2 text-primary">
                  <i class="fas fa-cog"></i>
                  <span>{{ t('aiAssistant.manageRoles', '管理角色') }}</span>
                </a>
              </li>
              <li v-if="roles.length === 0 && !isLoadingRoles">
                <span class="text-base-content/50 text-sm">{{ t('aiAssistant.noRoles', '暂无自定义角色') }}</span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>

    <!-- 主内容区 - Agent 执行模式 -->
    <div class="flex-1 overflow-hidden min-h-0">
      <AgentView 
        ref="agentViewRef"
        :show-todos="true"
        :selected-role="selectedRole"
        @submit="handleAgentSubmit"
        @complete="handleAgentComplete"
        @error="handleAgentError"
      />
    </div>

    <!-- 角色管理弹窗 -->
    <RoleManagement v-if="showRoleManagement" @close="showRoleManagement = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import RoleManagement from '@/components/RoleManagement.vue'
import { AgentView } from '@/components/Agent'
import { useRoleManagement } from '@/composables/useRoleManagement'

// 流量引用类型
interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
}

defineOptions({
  name: 'AIAssistant'
});

const { t } = useI18n()

// 角色管理
const {
  roles,
  selectedRole,
  isLoading: isLoadingRoles,
  loadRoles,
  selectRole,
} = useRoleManagement()

// 角色管理状态
const showRoleManagement = ref(false)

// --- AgentView 相关 ---
const agentViewRef = ref<any>(null)

// 流量事件监听器
let unlistenTraffic: UnlistenFn | null = null

const handleSelectRole = async (role: any) => {
  try {
    await selectRole(role)
  } catch (error) {
    console.error('Failed to select role:', error)
  }
}

// --- AgentView 事件处理 ---
const handleAgentSubmit = (task: string) => {
  console.log('Agent task submitted:', task)
}

const handleAgentComplete = async (result: any) => {
  console.log('Agent task completed:', result)
}

const handleAgentError = (error: string) => {
  console.error('Agent task error:', error)
}

// 初始化
onMounted(async () => {
  try {
    // 加载角色列表
    await loadRoles()

    // 监听流量发送到助手事件
    unlistenTraffic = await listen<{ requests: ReferencedTraffic[], type?: 'request' | 'response' | 'both' }>('traffic:send-to-assistant', (event) => {
      console.log('AIAssistant: Received traffic data:', event.payload)
      if (event.payload?.requests && agentViewRef.value?.addReferencedTraffic) {
        const type = event.payload.type || 'both'
        agentViewRef.value.addReferencedTraffic(event.payload.requests, type)
      }
    })
  } catch (error) {
    console.error('Failed to initialize AI Assistant:', error)
  }
})

// 清理事件监听
onUnmounted(() => {
  if (unlistenTraffic) {
    unlistenTraffic()
    unlistenTraffic = null
  }
})
</script>

<style scoped>
.ai-assistant-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* 自定义滚动条 */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}

/* 状态指示器动画 */
.badge-success {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}
</style>
