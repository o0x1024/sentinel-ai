<template>
  <div class="flex flex-col h-full">
    <div class="p-3 border-b border-base-300">
      <h3 class="font-semibold text-sm flex items-center gap-2">
        <i class="fas fa-server text-primary"></i>
        {{ $t('trafficAnalysis.proxifierProxies.title') }}
      </h3>
    </div>

    <!-- 代理列表 -->
    <div class="flex-1 overflow-auto">
      <table class="table table-xs table-pin-rows">
        <thead>
          <tr class="bg-base-200">
            <th class="w-40">{{ $t('trafficAnalysis.proxifierProxies.table.name') }}</th>
            <th class="w-20">{{ $t('trafficAnalysis.proxifierProxies.table.port') }}</th>
            <th class="w-20">{{ $t('trafficAnalysis.proxifierProxies.table.type') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="localProxies.length === 0">
            <td colspan="3" class="text-center text-base-content/50 py-4">
              {{ $t('trafficAnalysis.proxifierProxies.emptyState.noProxies') }}
            </td>
          </tr>
          <tr 
            v-for="(proxy, index) in localProxies" 
            :key="proxy.id"
            class="hover:bg-base-200/50 cursor-pointer"
            :class="{ 'bg-primary/10': selectedIndex === index }"
            @click="selectedIndex = index"
            @dblclick="editProxy(index)"
          >
            <td class="font-mono text-xs">{{ proxy.host }}</td>
            <td class="font-mono text-xs">{{ proxy.port }}</td>
            <td class="text-xs">
              <span :class="getTypeClass(proxy.type)">{{ proxy.type }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 操作按钮 -->
    <div class="p-2 border-t border-base-300 flex gap-2">
      <button class="btn btn-xs btn-primary" @click="addProxy">
        {{ $t('trafficAnalysis.proxifierProxies.buttons.add') }}
      </button>
      <button 
        class="btn btn-xs btn-ghost" 
        @click="editProxy(selectedIndex)"
        :disabled="selectedIndex < 0"
      >
        {{ $t('trafficAnalysis.proxifierProxies.buttons.edit') }}
      </button>
      <button 
        class="btn btn-xs btn-ghost text-error" 
        @click="removeProxy"
        :disabled="selectedIndex < 0"
      >
        {{ $t('trafficAnalysis.proxifierProxies.buttons.remove') }}
      </button>
    </div>

    <!-- 代理链按钮 -->
    <div class="p-2 border-t border-base-300">
      <p class="text-xs text-base-content/60 mb-2">{{ $t('trafficAnalysis.proxifierProxies.description') }}</p>
      <button class="btn btn-xs btn-outline w-full" @click="showProxyChains">
        {{ $t('trafficAnalysis.proxifierProxies.buttons.proxyChains') }}
      </button>
    </div>

    <!-- 编辑对话框 -->
    <dialog ref="editDialog" class="modal">
      <div class="modal-box max-w-md">
        <h3 class="font-bold text-lg mb-4">
          {{ editingProxy.id ? $t('trafficAnalysis.proxifierProxies.dialog.editTitle') : $t('trafficAnalysis.proxifierProxies.dialog.addTitle') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxifierProxies.dialog.host.label') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingProxy.host"
              class="input input-bordered input-sm"
              :placeholder="$t('trafficAnalysis.proxifierProxies.dialog.host.placeholder')"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxifierProxies.dialog.port.label') }}</span>
            </label>
            <input 
              type="number" 
              v-model.number="editingProxy.port"
              class="input input-bordered input-sm"
              :placeholder="$t('trafficAnalysis.proxifierProxies.dialog.port.placeholder')"
              min="1"
              max="65535"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxifierProxies.dialog.type.label') }}</span>
            </label>
            <select v-model="editingProxy.type" class="select select-bordered select-sm">
              <option value="HTTP">{{ $t('trafficAnalysis.proxifierProxies.dialog.type.http') }}</option>
              <option value="HTTPS">{{ $t('trafficAnalysis.proxifierProxies.dialog.type.https') }}</option>
              <option value="SOCKS5">{{ $t('trafficAnalysis.proxifierProxies.dialog.type.socks5') }}</option>
            </select>
          </div>

          <div class="divider my-2">{{ $t('trafficAnalysis.proxifierProxies.dialog.auth') }}</div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxifierProxies.dialog.username.label') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingProxy.username"
              class="input input-bordered input-sm"
              :placeholder="$t('trafficAnalysis.proxifierProxies.dialog.username.placeholder')"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxifierProxies.dialog.password.label') }}</span>
            </label>
            <input 
              type="password" 
              v-model="editingProxy.password"
              class="input input-bordered input-sm"
              :placeholder="$t('trafficAnalysis.proxifierProxies.dialog.password.placeholder')"
            />
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost btn-sm" @click="cancelEdit">{{ $t('trafficAnalysis.proxifierProxies.dialog.buttons.cancel') }}</button>
          <button class="btn btn-primary btn-sm" @click="saveProxy">{{ $t('trafficAnalysis.proxifierProxies.dialog.buttons.save') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.history.detailsPanel.close') }}</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface ProxyServer {
  id: string
  name: string
  host: string
  port: number
  type: 'HTTP' | 'HTTPS' | 'SOCKS5'
  username?: string
  password?: string
  enabled: boolean
}

const props = defineProps<{
  proxies: ProxyServer[]
}>()

const emit = defineEmits<{
  'update:proxies': [proxies: ProxyServer[]]
}>()

// Local state
const localProxies = ref<ProxyServer[]>([...props.proxies])
const selectedIndex = ref(-1)
const editDialog = ref<HTMLDialogElement | null>(null)
const editingProxy = ref<Partial<ProxyServer>>({})
const editingIndex = ref(-1)

// Watch for prop changes
watch(() => props.proxies, (newProxies) => {
  localProxies.value = [...newProxies]
}, { deep: true })

// Methods
function getTypeClass(type: string): string {
  switch (type) {
    case 'HTTPS': return 'badge badge-xs badge-success'
    case 'HTTP': return 'badge badge-xs badge-info'
    case 'SOCKS5': return 'badge badge-xs badge-warning'
    default: return 'badge badge-xs'
  }
}

function addProxy() {
  editingIndex.value = -1
  editingProxy.value = {
    host: '',
    port: 8080,
    type: 'HTTP',
    enabled: true
  }
  editDialog.value?.showModal()
}

function editProxy(index: number) {
  if (index < 0 || index >= localProxies.value.length) return
  
  editingIndex.value = index
  editingProxy.value = { ...localProxies.value[index] }
  editDialog.value?.showModal()
}

function removeProxy() {
  if (selectedIndex.value < 0) return
  
  localProxies.value.splice(selectedIndex.value, 1)
  selectedIndex.value = -1
  emit('update:proxies', localProxies.value)
}

function saveProxy() {
  if (!editingProxy.value.host || !editingProxy.value.port) {
    return
  }

  if (editingIndex.value >= 0) {
    // 编辑现有
    localProxies.value[editingIndex.value] = {
      ...localProxies.value[editingIndex.value],
      ...editingProxy.value
    } as ProxyServer
  } else {
    // 添加新的
    localProxies.value.push({
      id: Date.now().toString(),
      name: editingProxy.value.host || '',
      host: editingProxy.value.host || '',
      port: editingProxy.value.port || 8080,
      type: editingProxy.value.type || 'HTTP',
      username: editingProxy.value.username,
      password: editingProxy.value.password,
      enabled: true
    })
  }

  editDialog.value?.close()
  emit('update:proxies', localProxies.value)
}

function cancelEdit() {
  editDialog.value?.close()
}

function showProxyChains() {
  // TODO: 实现代理链功能
  console.log('Proxy chains feature coming soon')
}
</script>

<style scoped>
.table-xs th,
.table-xs td {
  padding: 0.5rem 0.75rem;
}
</style>

