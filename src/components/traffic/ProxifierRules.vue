<template>
  <div class="flex flex-col h-full">
    <div class="p-3 border-b border-base-300">
      <h3 class="font-semibold text-sm flex items-center gap-2">
        <i class="fas fa-filter text-primary"></i>
        {{ $t('proxifierPanel.rules.title') }}
      </h3>
    </div>

    <!-- 规则列表 -->
    <div class="flex-1 overflow-auto">
      <table class="table table-xs table-pin-rows">
        <thead>
          <tr class="bg-base-200">
            <th class="w-8"></th>
            <th class="w-24">{{ $t('proxifierPanel.rules.table.name') }}</th>
            <th class="w-20">{{ $t('proxifierPanel.rules.table.applications') }}</th>
            <th class="w-28">{{ $t('proxifierPanel.rules.table.targetHosts') }}</th>
            <th class="w-16">{{ $t('proxifierPanel.rules.table.targetPorts') }}</th>
            <th class="w-24">{{ $t('proxifierPanel.rules.table.action') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="localRules.length === 0">
            <td colspan="6" class="text-center text-base-content/50 py-4">
              {{ $t('proxifierPanel.rules.noRules') }}
            </td>
          </tr>
          <tr 
            v-for="(rule, index) in localRules" 
            :key="rule.id"
            class="hover:bg-base-200/50 cursor-pointer"
            :class="{ 'bg-primary/10': selectedIndex === index }"
            @click="selectedIndex = index"
            @dblclick="editRule(index)"
          >
            <td>
              <input 
                type="checkbox" 
                class="checkbox checkbox-xs"
                v-model="rule.enabled"
                @change="onRuleToggle(index)"
              />
            </td>
            <td class="text-xs">{{ rule.name }}</td>
            <td class="text-xs truncate max-w-20" :title="rule.applications">
              {{ rule.applications }}
            </td>
            <td class="text-xs truncate max-w-28" :title="rule.targetHosts">
              {{ rule.targetHosts }}
            </td>
            <td class="text-xs">{{ rule.targetPorts }}</td>
            <td class="text-xs">
              <span :class="getActionClass(rule.action)">{{ formatAction(rule.action) }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 操作按钮 -->
    <div class="p-2 border-t border-base-300 flex gap-2 flex-wrap">
      <button class="btn btn-xs btn-primary" @click="addRule">
        {{ $t('proxifierPanel.buttons.add') }}
      </button>
      <button class="btn btn-xs btn-ghost" @click="cloneRule" :disabled="selectedIndex < 0">
        {{ $t('proxifierPanel.buttons.clone') }}
      </button>
      <button 
        class="btn btn-xs btn-ghost" 
        @click="editRule(selectedIndex)"
        :disabled="selectedIndex < 0"
      >
        {{ $t('proxifierPanel.buttons.edit') }}
      </button>
      <button 
        class="btn btn-xs btn-ghost text-error" 
        @click="removeRule"
        :disabled="selectedIndex < 0"
      >
        {{ $t('proxifierPanel.buttons.remove') }}
      </button>
    </div>

    <!-- 排序按钮 -->
    <div class="p-2 border-t border-base-300 flex gap-2">
      <button 
        class="btn btn-xs btn-ghost" 
        @click="moveRule('up')"
        :disabled="selectedIndex <= 0"
      >
        <i class="fas fa-arrow-up"></i>
      </button>
      <button 
        class="btn btn-xs btn-ghost" 
        @click="moveRule('down')"
        :disabled="selectedIndex < 0 || selectedIndex >= localRules.length - 1"
      >
        <i class="fas fa-arrow-down"></i>
      </button>
    </div>

    <!-- 编辑对话框 -->
    <dialog ref="editDialog" class="modal">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-4">
          {{ editingRule.id ? $t('proxifierPanel.buttons.edit') : $t('proxifierPanel.buttons.add') }}
        </h3>
        
        <div class="space-y-4">
          <!-- 规则名称 -->
          <div class="flex items-center gap-4">
            <div class="form-control flex-1">
              <label class="label py-1">
                <span class="label-text text-sm">{{ $t('proxifierPanel.rules.table.name') }}:</span>
              </label>
              <input 
                type="text" 
                v-model="editingRule.name"
                class="input input-bordered input-sm"
                placeholder="{{ $t('proxifierPanel.rules.table.name') }}"
              />
            </div>
            <label class="label cursor-pointer gap-2">
              <input 
                type="checkbox" 
                v-model="editingRule.enabled"
                class="checkbox checkbox-sm"
              />
              <span class="label-text">{{ $t('proxifierPanel.buttons.enabled') }}</span>
            </label>
          </div>

          <!-- 应用程序 -->
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text text-sm">{{ $t('proxifierPanel.rules.table.applications') }}</span>
            </label>
            <textarea 
              v-model="editingRule.applications"
              class="textarea textarea-bordered textarea-sm h-20"
              placeholder="Any"
            ></textarea>
            <label class="label py-1">
              <span class="label-text-alt text-xs">{{ $t('proxifierPanel.rules.table.applicationsExample') }}</span>
            </label>
          </div>

          <!-- 目标主机 -->
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text text-sm">{{ $t('proxifierPanel.rules.table.targetHosts') }}</span>
            </label>
            <textarea 
              v-model="editingRule.targetHosts"
              class="textarea textarea-bordered textarea-sm h-20"
              placeholder="Any"
            ></textarea>
            <label class="label py-1">
              <span class="label-text-alt text-xs">{{ $t('proxifierPanel.rules.table.targetHostsExample') }}</span>
            </label>
          </div>

          <!-- 目标端口 -->
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text text-sm">{{ $t('proxifierPanel.rules.table.targetPorts') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingRule.targetPorts"
              class="input input-bordered input-sm"
              placeholder="Any"
            />
            <label class="label py-1">
              <span class="label-text-alt text-xs">{{ $t('proxifierPanel.rules.table.targetPortsExample') }}</span>
            </label>
          </div>

          <!-- 动作 -->
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text text-sm">{{ $t('proxifierPanel.rules.table.action') }}:</span>
            </label>
            <select v-model="editingRule.action" class="select select-bordered select-sm">
              <option value="Direct">{{ $t('proxifierPanel.rules.table.direct') }}</option>
              <option value="Block">{{ $t('proxifierPanel.rules.table.block') }}</option>
              <optgroup label="{{ $t('proxifierPanel.rules.table.viaProxy') }}">
                <option 
                  v-for="proxy in availableProxies" 
                  :key="proxy.id"
                  :value="`Proxy ${proxy.type} ${proxy.host}:${proxy.port}`"
                >
                  {{ $t('proxifierPanel.rules.table.proxyFormat', { type: proxy.type, host: proxy.host, port: proxy.port }) }}
                </option>
              </optgroup>
            </select>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost btn-sm" @click="cancelEdit">{{ $t('proxifierPanel.buttons.cancel') }}</button>
          <button class="btn btn-primary btn-sm" @click="saveRule">{{ $t('proxifierPanel.buttons.save') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('proxifierPanel.buttons.close') }}</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'

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

interface ProxifierRule {
  id: string
  name: string
  enabled: boolean
  applications: string
  targetHosts: string
  targetPorts: string
  action: string
}

const props = defineProps<{
  rules: ProxifierRule[]
  proxies: ProxyServer[]
}>()

const emit = defineEmits<{
  'update:rules': [rules: ProxifierRule[]]
}>()

// Local state
const localRules = ref<ProxifierRule[]>([...props.rules])
const selectedIndex = ref(-1)
const editDialog = ref<HTMLDialogElement | null>(null)
const editingRule = ref<Partial<ProxifierRule>>({})
const editingIndex = ref(-1)

// Computed
const availableProxies = computed(() => props.proxies.filter(p => p.enabled))

// Watch for prop changes
watch(() => props.rules, (newRules) => {
  localRules.value = [...newRules]
}, { deep: true })

// Methods
function getActionClass(action: string): string {
  if (action === 'Direct') return 'text-success'
  if (action === 'Block') return 'text-error'
  return 'text-warning'
}

function formatAction(action: string): string {
  if (action === 'Direct') return 'Direct'
  if (action === 'Block') return 'Block'
  return action
}

function addRule() {
  editingIndex.value = -1
  editingRule.value = {
    name: 'New',
    enabled: true,
    applications: 'Any',
    targetHosts: 'Any',
    targetPorts: 'Any',
    action: 'Direct'
  }
  editDialog.value?.showModal()
}

function cloneRule() {
  if (selectedIndex.value < 0) return
  
  const source = localRules.value[selectedIndex.value]
  editingIndex.value = -1
  editingRule.value = {
    ...source,
    id: '',
    name: `Copy of ${source.name}`
  }
  editDialog.value?.showModal()
}

function editRule(index: number) {
  if (index < 0 || index >= localRules.value.length) return
  
  editingIndex.value = index
  editingRule.value = { ...localRules.value[index] }
  editDialog.value?.showModal()
}

function removeRule() {
  if (selectedIndex.value < 0) return
  
  localRules.value.splice(selectedIndex.value, 1)
  selectedIndex.value = -1
  emit('update:rules', localRules.value)
}

function moveRule(direction: 'up' | 'down') {
  if (selectedIndex.value < 0) return
  
  const newIndex = direction === 'up' 
    ? selectedIndex.value - 1 
    : selectedIndex.value + 1
  
  if (newIndex < 0 || newIndex >= localRules.value.length) return
  
  const temp = localRules.value[selectedIndex.value]
  localRules.value[selectedIndex.value] = localRules.value[newIndex]
  localRules.value[newIndex] = temp
  selectedIndex.value = newIndex
  
  emit('update:rules', localRules.value)
}

function onRuleToggle(index: number) {
  emit('update:rules', localRules.value)
}

function saveRule() {
  if (!editingRule.value.name) {
    return
  }

  const rule: ProxifierRule = {
    id: editingRule.value.id || Date.now().toString(),
    name: editingRule.value.name || 'New',
    enabled: editingRule.value.enabled ?? true,
    applications: editingRule.value.applications || 'Any',
    targetHosts: editingRule.value.targetHosts || 'Any',
    targetPorts: editingRule.value.targetPorts || 'Any',
    action: editingRule.value.action || 'Direct'
  }

  if (editingIndex.value >= 0) {
    localRules.value[editingIndex.value] = rule
  } else {
    localRules.value.push(rule)
  }

  editDialog.value?.close()
  emit('update:rules', localRules.value)
}

function cancelEdit() {
  editDialog.value?.close()
}
</script>

<style scoped>
.table-xs th,
.table-xs td {
  padding: 0.4rem 0.5rem;
}
</style>

