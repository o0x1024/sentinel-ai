<template>
  <div class="space-y-4">
    <div class="flex justify-between items-center">
      <div class="alert alert-info flex-1 mr-4">
        <i class="fas fa-info-circle"></i>
        <span>管理 Agent 插件工具，可在创建 Agent 时选择启用的插件工具</span>
      </div>
      <div class="join">
        <button @click="viewMode = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'card'}]">
          <i class="fas fa-th-large"></i>
        </button>
        <button @click="viewMode = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': viewMode === 'list'}]">
          <i class="fas fa-list"></i>
        </button>
      </div>
    </div>
    
    <!-- 插件列表 -->
    <div v-if="isLoading" class="text-center p-8">
      <i class="fas fa-spinner fa-spin text-2xl"></i>
      <p class="mt-2">正在加载插件...</p>
    </div>
    
    <div v-else-if="plugins.length > 0" class="space-y-4">
      <div class="flex justify-end mb-4">
        <button @click="$emit('show-upload')" class="btn btn-primary btn-sm">
          <i class="fas fa-upload mr-2"></i>
          上传插件
        </button>
      </div>
      
      <!-- 卡片视图 -->
      <div v-if="viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div 
          v-for="plugin in plugins" 
          :key="plugin.metadata.id"
          class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
        >
          <div class="card-body">
            <div class="flex items-center gap-3">
              <div class="avatar">
                <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                  <i class="fas fa-puzzle-piece text-primary text-xl"></i>
                </div>
              </div>
              <div class="flex-1">
                <h3 class="card-title text-lg">{{ plugin.metadata.name }}</h3>
                <span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary toggle-sm" 
                    :checked="plugin.status === 'Enabled'"
                    @change="togglePlugin(plugin)"
                    :disabled="plugin.is_toggling"
                  />
                </label>
              </div>
            </div>

            <p class="text-sm mt-2 h-16">{{ plugin.metadata.description }}</p>

            <div class="flex flex-wrap gap-2 mt-2">
              <span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span>
              <span 
                v-for="perm in plugin.metadata.permissions" 
                :key="perm"
                class="badge badge-warning badge-xs"
              >
                {{ perm }}
              </span>
            </div>

            <div class="card-actions justify-between items-center mt-4">
              <div class="flex gap-1">
                <span 
                  :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']"
                >
                  {{ plugin.status }}
                </span>
                <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                  <i class="fas fa-exclamation-triangle"></i>
                </span>
              </div>
              <div class="flex gap-1">
                <button 
                  @click="editPlugin(plugin)"
                  class="btn btn-xs btn-outline"
                  title="编辑"
                >
                  <i class="fas fa-edit"></i>
                </button>
                <button 
                  @click="viewPluginInfo(plugin)"
                  class="btn btn-xs btn-outline"
                  title="详情"
                >
                  <i class="fas fa-info"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 列表视图 -->
      <div v-if="viewMode === 'list'" class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-1/12">启用</th>
              <th>名称</th>
              <th>版本</th>
              <th>作者</th>
              <th>描述</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="plugin in plugins" :key="plugin.metadata.id">
              <td>
                <input 
                  type="checkbox" 
                  class="toggle toggle-primary toggle-sm" 
                  :checked="plugin.status === 'Enabled'"
                  @change="togglePlugin(plugin)"
                  :disabled="plugin.is_toggling"
                />
              </td>
              <td>
                <div class="flex items-center gap-2">
                  <i class="fas fa-puzzle-piece text-primary"></i>
                  <span class="font-semibold">{{ plugin.metadata.name }}</span>
                </div>
              </td>
              <td><span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span></td>
              <td><span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span></td>
              <td class="text-sm">{{ plugin.metadata.description }}</td>
              <td>
                <div class="flex flex-col gap-1">
                  <span :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']">
                    {{ plugin.status }}
                  </span>
                  <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                    <i class="fas fa-exclamation-triangle"></i>
                  </span>
                </div>
              </td>
              <td>
                <div class="flex gap-1">
                  <button 
                    @click="editPlugin(plugin)"
                    class="btn btn-xs btn-outline"
                    title="编辑"
                  >
                    <i class="fas fa-edit"></i>
                  </button>
                  <button 
                    @click="viewPluginInfo(plugin)"
                    class="btn btn-xs btn-outline"
                    title="详情"
                  >
                    <i class="fas fa-info"></i>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    
    <div v-else class="text-center p-8">
      <i class="fas fa-plug text-4xl text-base-content/30 mb-4"></i>
      <p class="text-lg font-semibold">暂无插件工具</p>
      <p class="text-base-content/70 mt-2">前往插件管理创建 Agent 工具插件</p>
      <button @click="goToPluginManagement" class="btn btn-primary mt-4">
        <i class="fas fa-plus mr-2"></i>
        创建插件工具
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

// 类型定义
interface PluginMetadata {
  id: string
  name: string
  version: string
  description: string
  author: string
  main_category: string
  category: string
  permissions: string[]
}

interface PluginRecord {
  metadata: PluginMetadata
  path: string
  status: string
  last_error: string | null
  is_toggling?: boolean
}

// 定义事件
defineEmits<{
  (e: 'show-upload'): void
}>()

// 状态
const plugins = ref<PluginRecord[]>([])
const isLoading = ref(false)
const viewMode = ref('list')

// 方法
async function fetchPlugins() {
  isLoading.value = true
  try {
    const response = await invoke<any>('list_plugins')
    if (response.success && response.data) {
      plugins.value = response.data.filter((plugin: PluginRecord) => 
        plugin.metadata.main_category === 'agent'
      )
    }
  } catch (error) {
    console.error('Failed to fetch agent tool plugins:', error)
    plugins.value = []
  } finally {
    isLoading.value = false
  }
}

async function refresh() {
  await fetchPlugins()
}

function goToPluginManagement() {
  window.location.hash = '#/plugin-management'
}

async function togglePlugin(plugin: PluginRecord) {
  plugin.is_toggling = true
  try {
    const isEnabled = plugin.status === 'Enabled'
    if (isEnabled) {
      await invoke('disable_plugin', { pluginId: plugin.metadata.id })
      dialog.toast.success(`已禁用插件: ${plugin.metadata.name}`)
    } else {
      await invoke('enable_plugin', { pluginId: plugin.metadata.id })
      dialog.toast.success(`已启用插件: ${plugin.metadata.name}`)
    }
    await fetchPlugins()
  } catch (error: any) {
    console.error(`Failed to toggle plugin ${plugin.metadata.id}:`, error)
    dialog.toast.error(`切换插件状态失败: ${error}`)
  } finally {
    plugin.is_toggling = false
  }
}

function editPlugin(plugin: PluginRecord) {
  dialog.toast.info('插件编辑功能开发中...')
}

function viewPluginInfo(plugin: PluginRecord) {
  const info = `
插件名称: ${plugin.metadata.name}
版本: ${plugin.metadata.version}
作者: ${plugin.metadata.author}
描述: ${plugin.metadata.description}
权限: ${plugin.metadata.permissions.join(', ')}
状态: ${plugin.status}
${plugin.last_error ? `错误: ${plugin.last_error}` : ''}
  `.trim()
  
  dialog.info(info)
}

// 暴露方法供父组件调用
defineExpose({ refresh })

onMounted(() => {
  fetchPlugins()
})
</script>

