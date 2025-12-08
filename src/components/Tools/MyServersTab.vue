<template>
  <div class="space-y-4">
    <div class="flex justify-end mb-4">
      <div class="join">
        <button @click="viewMode = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'card'}]">
          <i class="fas fa-th-large"></i>
        </button>
        <button @click="viewMode = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'list'}]">
          <i class="fas fa-list"></i>
        </button>
      </div>
    </div>
    
    <!-- 列表视图 -->
    <div v-if="viewMode === 'list'" class="overflow-x-auto">
      <table class="table w-full">
        <thead>
          <tr>
            <th class="w-1/12">{{ $t('Tools.addServer.enabled') }}</th>
            <th>{{ $t('common.name') }}</th>
            <th>{{ $t('common.type') }}</th>
            <th>{{ $t('common.status') }}</th>
            <th>{{ $t('Tools.endpoint') }}</th>
            <th>{{ $t('common.operations') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="connection in connections" :key="connection.id || connection.name">
            <td>
              <input type="checkbox" class="toggle toggle-sm toggle-success" :checked="connection.status === 'Connected'" @change="toggleServer(connection)" />
            </td>
            <td>{{ connection.name }}</td>
            <td><span class="badge badge-ghost">{{ connection.transport_type }}</span></td>
            <td>
              <span :class="getStatusBadgeClass(connection.status)" class="flex items-center gap-1">
                <i :class="getStatusIcon(connection.status)"></i>
                {{ connection.status }}
              </span>
            </td>
            <td class="text-xs font-mono">{{ connection.endpoint }}</td>
            <td>
              <div class="flex gap-1">
                <button 
                  v-if="connection.status === 'Connected' && connection.id"
                  @click="disconnect(connection)" 
                  class="btn btn-xs btn-outline btn-warning" 
                  title="断开连接"
                >
                  <i class="fas fa-unlink"></i>
                </button>
                <button 
                  v-if="connection.status === 'Connected' && connection.id"
                  @click="$emit('test-server', connection)" 
                  class="btn btn-xs btn-outline btn-info" 
                  title="测试服务器工具"
                >
                  <i class="fas fa-vial"></i>
                </button>
                <button 
                  v-else-if="connection.status !== 'Connected'"
                  @click="connect(connection)" 
                  class="btn btn-xs btn-outline btn-success" 
                  title="连接"
                >
                  <i class="fas fa-link"></i>
                </button>
                <button 
                  @click="deleteServer(connection)" 
                  class="btn btn-xs btn-outline btn-error" 
                  :title="$t('common.delete')"
                >
                  <i class="fas fa-trash"></i>
                </button>
                <button 
                  @click="$emit('show-details', connection)"
                  class="btn btn-xs btn-outline" 
                  :title="$t('common.details')"
                >
                  <i class="fas fa-info"></i>
                </button>
              </div>
            </td>
          </tr>
          <tr v-if="connections.length === 0">
            <td colspan="6" class="text-center py-4">{{ $t('Tools.noConnections') }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    
    <!-- 卡片视图 -->
    <div v-if="viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <div 
        v-for="connection in connections" 
        :key="connection.id || connection.name"
        class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
      >
        <div class="card-body">
          <div class="flex items-center gap-3">
            <div class="avatar">
              <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                <i class="fas fa-server text-primary text-xl"></i>
              </div>
            </div>
            <div class="flex-1">
              <h3 class="card-title text-lg">{{ connection.name }}</h3>
              <span class="badge badge-ghost badge-sm">{{ connection.transport_type }}</span>
            </div>
            <div class="form-control">
              <label class="label cursor-pointer">
                <input 
                  type="checkbox" 
                  class="toggle toggle-sm toggle-success" 
                  :checked="connection.status === 'Connected'" 
                  @change="toggleServer(connection)" 
                />
              </label>
            </div>
          </div>

          <p class="text-sm mt-2 h-12 overflow-hidden">{{ connection.endpoint }}</p>

          <div class="flex items-center justify-between mt-2">
            <span :class="getStatusBadgeClass(connection.status)" class="flex items-center gap-1">
              <i :class="getStatusIcon(connection.status)"></i>
              {{ connection.status }}
            </span>
          </div>

          <div class="card-actions justify-end mt-4">
            <button 
              v-if="connection.status === 'Connected' && connection.id"
              @click="disconnect(connection)" 
              class="btn btn-xs btn-outline btn-warning" 
              title="断开连接"
            >
              <i class="fas fa-unlink"></i>
            </button>
            <button 
              v-if="connection.status === 'Connected' && connection.id"
              @click="$emit('test-server', connection)" 
              class="btn btn-xs btn-outline btn-info" 
              title="测试服务器工具"
            >
              <i class="fas fa-vial"></i>
            </button>
            <button 
              v-else-if="connection.status !== 'Connected'"
              @click="connect(connection)" 
              class="btn btn-xs btn-outline btn-success" 
              title="连接"
            >
              <i class="fas fa-link"></i>
            </button>
            <button 
              @click="deleteServer(connection)" 
              class="btn btn-xs btn-outline btn-error" 
              :title="$t('common.delete')"
            >
              <i class="fas fa-trash"></i>
            </button>
            <button 
              @click="$emit('show-details', connection)"
              class="btn btn-xs btn-outline" 
              :title="$t('common.details')"
            >
              <i class="fas fa-info"></i>
            </button>
          </div>
        </div>
      </div>
      
      <div v-if="connections.length === 0" class="col-span-full text-center py-8">
        {{ $t('Tools.noConnections') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { emit as tauriEmit } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'

const { t } = useI18n()

// 类型定义
interface McpConnection {
  db_id: number
  id: string | null
  name: string
  description: string | null
  transport_type: string
  endpoint: string
  status: string
  command: string
  args: string[]
}

// 定义事件
defineEmits<{
  (e: 'show-details', connection: McpConnection): void
  (e: 'test-server', connection: McpConnection): void
}>()

// 状态
const connections = ref<McpConnection[]>([])
const viewMode = ref('list')

// 方法
function getStatusBadgeClass(status: string) {
  switch (status) {
    case 'Connected': return 'badge badge-sm badge-success'
    case 'Error': return 'badge badge-sm badge-error'
    case 'Disconnected': return 'badge badge-sm badge-warning'
    case 'Connecting': return 'badge badge-sm badge-info'
    default: return 'badge badge-sm'
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'Connected': return 'fas fa-check-circle'
    case 'Error': return 'fas fa-exclamation-circle'
    case 'Disconnected': return 'fas fa-times-circle'
    case 'Connecting': return 'fas fa-spinner fa-spin'
    default: return 'fas fa-question-circle'
  }
}

async function fetchConnections() {
  try {
    connections.value = await invoke('mcp_get_connections')
    await updateStatus()
  } catch (error) {
    console.error('Failed to fetch MCP connections:', error)
    connections.value = []
  }
}

async function updateStatus() {
  try {
    const statusMap = await invoke('mcp_get_connection_status') as Record<string, string>
    connections.value.forEach(conn => {
      if (conn.name && statusMap[conn.name]) {
        conn.status = statusMap[conn.name]
      }
    })
  } catch (error) {
    console.error('Failed to fetch connection status:', error)
  }
}

async function refresh() {
  await fetchConnections()
}

async function toggleServer(connection: McpConnection) {
  try {
    if (connection.status === 'Connected' && connection.id) {
      await invoke('mcp_disconnect_server', { connectionId: connection.id })
      dialog.toast.success(`已断开服务器 ${connection.name}`)
    } else {
      const existingConnection = connections.value.find(conn => 
        conn.name === connection.name && conn.status === 'Connected'
      )
      if (existingConnection) {
        dialog.toast.warning(`服务器 ${connection.name} 已经连接`)
        return
      }
      await invoke('add_child_process_mcp_server', { 
        name: connection.name, 
        command: connection.command, 
        args: connection.args 
      })
      dialog.toast.success(`已连接服务器 ${connection.name}`)
    }
    await fetchConnections()
    await tauriEmit('mcp:tools-changed', { action: 'server_toggled', serverName: connection.name })
  } catch (error) {
    console.error(`Failed to toggle server ${connection.name} state:`, error)
    dialog.toast.error(`切换服务器 ${connection.name} 状态失败: ${error}`)
  }
}

async function disconnect(connection: McpConnection) {
  if (!connection.id) return
  try {
    await invoke('mcp_disconnect_server', { connectionId: connection.id })
    dialog.toast.success(`已断开服务器 ${connection.name}`)
    await fetchConnections()
    await tauriEmit('mcp:tools-changed', { action: 'server_disconnected', serverName: connection.name })
  } catch (error) {
    console.error('Failed to disconnect MCP server:', error)
    dialog.toast.error(`断开服务器失败: ${error}`)
  }
}

async function connect(connection: McpConnection) {
  try {
    await invoke('add_child_process_mcp_server', { 
      name: connection.name, 
      command: connection.command, 
      args: connection.args 
    })
    dialog.toast.success(`已连接服务器 ${connection.name}`)
    await fetchConnections()
    await tauriEmit('mcp:tools-changed', { action: 'server_connected', serverName: connection.name })
  } catch (error) {
    console.error('Failed to connect MCP server:', error)
    dialog.toast.error(`连接服务器失败: ${error}`)
  }
}

async function deleteServer(connection: McpConnection) {
  try {
    const confirmed = await dialog.confirm(`确定要删除服务器 "${connection.name}" 吗？此操作将删除数据库配置且不可恢复。`)
    if (!confirmed) return
    
    if (connection.status === 'Connected' && connection.id) {
      try {
        await invoke('mcp_disconnect_server', { connectionId: connection.id })
      } catch (e) {
        console.warn('Failed to disconnect before delete:', e)
      }
    }
    
    await invoke('mcp_delete_server_config', { dbId: connection.db_id })
    dialog.toast.success(`已删除服务器 ${connection.name}`)
    await fetchConnections()
    await tauriEmit('mcp:tools-changed', { action: 'server_deleted', serverName: connection.name })
  } catch (error) {
    console.error('Failed to delete MCP server:', error)
    dialog.toast.error(`删除服务器失败: ${error}`)
  }
}

// 暴露方法供父组件调用
defineExpose({ refresh, fetchConnections, connections })

onMounted(() => {
  fetchConnections()
})
</script>

