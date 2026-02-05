<template>
  <div>
    <!-- 市场视图切换 -->
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

    <!-- 卡片视图 -->
    <div v-if="viewMode === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <div 
        v-for="server in servers" 
        :key="server.name"
        class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
      >
        <div class="card-body">
          <div class="flex items-center gap-3">
            <div class="avatar">
              <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                <i :class="server.icon || 'fas fa-server'" class="text-primary text-xl"></i>
              </div>
            </div>
            <div class="flex-1">
              <h3 class="card-title text-lg">{{ server.name }}</h3>
            </div>
          </div>

          <p class="text-sm mt-2 h-16">{{ server.description }}</p>

          <div class="card-actions justify-end mt-4">
            <button 
              @click="addServer(server)"
              :disabled="server.is_adding || isServerAdded(server)"
              class="btn btn-primary btn-sm"
            >
              <i v-if="server.is_adding" class="fas fa-spinner fa-spin mr-1"></i>
              <i v-else-if="!isServerAdded(server)" class="fas fa-plus mr-1"></i>
              {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('Tools.added') : $t('common.add')) }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 列表视图 -->
    <div v-if="viewMode === 'list'" class="overflow-x-auto">
      <table class="table w-full">
        <thead>
          <tr>
            <th class="w-12"></th>
            <th>{{ $t('common.name') }}</th>
            <th>{{ $t('common.description') }}</th>
            <th class="w-40">{{ $t('common.operations') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="server in servers" :key="server.name">
            <td>
               <div class="avatar">
                  <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                    <i :class="server.icon || 'fas fa-server'" class="text-primary text-lg"></i>
                  </div>
                </div>
            </td>
            <td>{{ server.name }}</td>
            <td>{{ server.description }}</td>
            <td>
               <button 
                @click="addServer(server)"
                :disabled="server.is_adding || isServerAdded(server)"
                class="btn btn-primary btn-sm"
              >
                <i v-if="server.is_adding" class="fas fa-spinner fa-spin mr-1"></i>
                <i v-else-if="!isServerAdded(server)" class="fas fa-plus mr-1"></i>
                {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('Tools.added') : $t('common.add')) }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 空状态 -->
    <div v-if="servers.length === 0" class="text-center p-8">
      <i class="fas fa-store text-4xl text-base-content/30 mb-4"></i>
      <p class="text-lg font-semibold">暂无可用服务器</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { emit as tauriEmit } from '@tauri-apps/api/event'

const { t } = useI18n()

// Props
const props = defineProps<{
  addedServerNames: string[]
}>()

// 类型定义
interface MarketplaceServer {
  name: string
  description: string
  command: string
  args: string[]
  icon: string
  is_adding?: boolean
}

// 状态
const servers = ref<MarketplaceServer[]>([])
const viewMode = ref('list')

// 方法
function isServerAdded(server: MarketplaceServer) {
  return props.addedServerNames.includes(server.name)
}

async function addServer(server: MarketplaceServer) {
  server.is_adding = true
  try {
    const { command, args, name } = server
    await invoke('add_child_process_mcp_server', { name, command, args })
    await tauriEmit('mcp:tools-changed', { action: 'server_added', serverName: name })
  } catch (error) {
    console.error(`Failed to add marketplace server ${server.name}:`, error)
  } finally {
    server.is_adding = false
  }
}

const refresh = async () => {
  // TODO: Add backend call to fetch marketplace servers
}

// 暴露方法供父组件调用
defineExpose({ servers, refresh })
</script>

