<template>
  <dialog :class="['modal', { 'modal-open': modelValue }]">
    <div v-if="modelValue" class="modal-box w-11/12 max-w-4xl">
      <div class="flex justify-between items-center mb-4">
        <h3 class="font-bold text-lg">
          <i class="fas fa-terminal mr-2"></i>
          Shell 工具安全配置
        </h3>
        <button @click="close" class="btn btn-sm btn-ghost">✕</button>
      </div>

      <div v-if="loading" class="flex justify-center p-8">
        <span class="loading loading-spinner loading-lg"></span>
      </div>

      <div v-else class="space-y-6">
        <!-- 默认策略 -->
        <div class="form-control bg-base-200 p-4 rounded-lg">
          <label class="label">
            <span class="label-text font-bold">默认策略</span>
            <span class="label-text-alt">当命令不匹配任何规则时的处理方式</span>
          </label>
          <div class="flex gap-4">
            <label class="label cursor-pointer gap-2">
              <input type="radio" name="default-action" class="radio radio-success" value="Allow" v-model="config.default_action" />
              <span>允许执行</span>
            </label>
            <label class="label cursor-pointer gap-2">
              <input type="radio" name="default-action" class="radio radio-warning" value="Ask" v-model="config.default_action" />
              <span>询问用户</span>
            </label>
            <label class="label cursor-pointer gap-2">
              <input type="radio" name="default-action" class="radio radio-error" value="Deny" v-model="config.default_action" />
              <span>拒绝执行</span>
            </label>
          </div>
        </div>

        <!-- 规则列表 -->
        <div>
          <div class="flex justify-between items-center mb-2">
            <h4 class="font-bold">安全规则</h4>
            <button @click="addRule" class="btn btn-sm btn-primary">
              <i class="fas fa-plus mr-1"></i> 添加规则
            </button>
          </div>
          
          <div class="overflow-x-auto border border-base-300 rounded-lg">
            <table class="table table-zebra w-full">
              <thead>
                <tr>
                  <th class="w-16">顺序</th>
                  <th>命令匹配模式 (包含匹配)</th>
                  <th class="w-32">动作</th>
                  <th class="w-24">操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(rule, index) in config.rules" :key="index">
                  <td class="text-center">{{ index + 1 }}</td>
                  <td>
                    <input type="text" v-model="rule.pattern" class="input input-sm input-bordered w-full font-mono" placeholder="例如: rm -rf" />
                  </td>
                  <td>
                    <select v-model="rule.action" class="select select-sm select-bordered w-full" :class="getActionClass(rule.action)">
                      <option value="Allow">允许</option>
                      <option value="Ask">询问</option>
                      <option value="Deny">拒绝</option>
                    </select>
                  </td>
                  <td>
                    <div class="flex gap-1">
                      <button @click="moveRule(index, -1)" class="btn btn-xs btn-ghost" :disabled="index === 0">
                        <i class="fas fa-arrow-up"></i>
                      </button>
                      <button @click="moveRule(index, 1)" class="btn btn-xs btn-ghost" :disabled="index === config.rules.length - 1">
                        <i class="fas fa-arrow-down"></i>
                      </button>
                      <button @click="removeRule(index)" class="btn btn-xs btn-error btn-ghost">
                        <i class="fas fa-trash"></i>
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="config.rules.length === 0">
                  <td colspan="4" class="text-center py-4 text-base-content/60">
                    暂无规则，将使用默认策略
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <p class="text-xs text-base-content/60 mt-2">
            <i class="fas fa-info-circle mr-1"></i>
            规则按顺序匹配，一旦匹配成功即应用对应动作。建议将具体规则放在前面，通用规则放在后面。
          </p>
        </div>
      </div>

      <div class="modal-action">
        <button @click="close" class="btn">取消</button>
        <button @click="save" class="btn btn-primary" :disabled="loading">
          <i class="fas fa-save mr-1"></i> 保存配置
        </button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

interface ShellRule {
  pattern: string
  action: 'Allow' | 'Deny' | 'Ask'
}

interface ShellConfig {
  rules: ShellRule[]
  default_action: 'Allow' | 'Deny' | 'Ask'
}

const loading = ref(false)
const config = ref<ShellConfig>({
  rules: [],
  default_action: 'Ask'
})

function close() {
  emit('update:modelValue', false)
}

function getActionClass(action: string) {
  switch (action) {
    case 'Allow': return 'text-success'
    case 'Deny': return 'text-error'
    case 'Ask': return 'text-warning'
    default: return ''
  }
}

async function loadConfig() {
  loading.value = true
  try {
    const res = await invoke<ShellConfig>('get_shell_tool_config')
    config.value = res
  } catch (e: any) {
    console.error('Failed to load shell config:', e)
    dialog.toast.error('加载配置失败: ' + e)
  } finally {
    loading.value = false
  }
}

async function save() {
  // Validate
  if (config.value.rules.some(r => !r.pattern.trim())) {
    dialog.toast.error('规则匹配模式不能为空')
    return
  }

  loading.value = true
  try {
    await invoke('set_shell_tool_config', { config: config.value })
    dialog.toast.success('配置已保存')
    close()
  } catch (e: any) {
    console.error('Failed to save shell config:', e)
    dialog.toast.error('保存配置失败: ' + e)
  } finally {
    loading.value = false
  }
}

function addRule() {
  config.value.rules.push({
    pattern: '',
    action: 'Ask'
  })
}

function removeRule(index: number) {
  config.value.rules.splice(index, 1)
}

function moveRule(index: number, direction: number) {
  const newIndex = index + direction
  if (newIndex < 0 || newIndex >= config.value.rules.length) return
  
  const temp = config.value.rules[index]
  config.value.rules[index] = config.value.rules[newIndex]
  config.value.rules[newIndex] = temp
}

watch(() => props.modelValue, (val) => {
  if (val) {
    loadConfig()
  }
})
</script>

