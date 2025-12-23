<!-- 
  UnifiedToolTest.vue
  统一工具测试模态框组件
  支持内置工具、工作流工具、插件工具的普通测试和高级测试
-->
<template>
  <dialog :class="['modal', { 'modal-open': modelValue }]">
    <div class="modal-box w-11/12 max-w-4xl" v-show="modelValue">
      <div class="flex justify-between items-center mb-4">
        <h3 class="font-bold text-lg flex items-center gap-2">
          <component :is="toolIcon" class="w-6 h-6" />
          <span>测试工具: {{ toolName }}</span>
          <span v-if="toolVersion" class="badge badge-ghost badge-sm">v{{ toolVersion }}</span>
        </h3>
        <button @click="close" class="btn btn-sm btn-ghost btn-circle">
          <i class="fas fa-times text-lg"></i>
        </button>
      </div>

      <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
        <!-- 工具信息 -->
        <div class="bg-base-200 p-4 rounded-lg">
          <p class="text-sm">{{ toolDescription || '暂无描述' }}</p>
          <div class="flex gap-2 mt-2">
            <span :class="['badge badge-sm', categoryBadgeClass]">{{ toolCategory }}</span>
            <span v-if="toolVersion" class="badge badge-ghost badge-sm">v{{ toolVersion }}</span>
          </div>
        </div>

        <!-- 参数说明（高级模式） -->
        <div v-if="showAdvanced && inputSchema?.properties" class="collapse collapse-arrow border border-base-300 bg-base-100">
          <input type="checkbox" checked />
          <div class="collapse-title text-md font-medium flex items-center gap-2">
            <i class="fas fa-info-circle text-info"></i>
            输入参数说明
          </div>
          <div class="collapse-content">
            <div class="overflow-x-auto">
              <table class="table table-sm w-full">
                <thead>
                  <tr>
                    <th>参数名</th>
                    <th>类型</th>
                    <th>必填</th>
                    <th>描述</th>
                    <th>约束</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="prop in schemaProperties" :key="prop.name">
                    <td class="font-mono text-primary">{{ prop.name }}</td>
                    <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                    <td>
                      <span v-if="prop.required" class="badge badge-error badge-sm">必填</span>
                    </td>
                    <td>{{ prop.description }}</td>
                    <td class="font-mono text-xs">{{ prop.constraints }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- 无参数提示 -->
        <div v-else-if="showAdvanced && !inputSchema?.properties" class="alert alert-warning">
          <i class="fas fa-exclamation-triangle"></i>
          <span>此工具没有输入参数或参数信息暂未提供，可直接运行测试。</span>
        </div>

        <!-- 测试参数输入（高级模式） -->
        <div v-if="showAdvanced" class="form-control">
          <label class="label">
            <span class="label-text">测试参数 (JSON)</span>
            <button @click="resetParams" class="btn btn-xs btn-ghost">
              <i class="fas fa-undo mr-1"></i>
              重置参数
            </button>
          </label>
          <textarea
            v-model="paramsJson"
            class="textarea textarea-bordered font-mono text-sm"
            placeholder='输入 JSON 格式的测试参数，例如: {}'
            rows="6"
            spellcheck="false"
          ></textarea>
        </div>

        <!-- 测试结果 -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">测试结果</span>
            <span v-if="testDuration" class="label-text-alt text-xs">耗时: {{ testDuration }}ms</span>
          </label>
          <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap h-48 bg-base-200 overflow-auto">{{ testResult || '点击"运行测试"查看结果' }}</pre>
        </div>
      </div>

      <div class="modal-action">
        <button @click="close" class="btn btn-ghost">取消</button>
        <button 
          v-if="!showAdvanced"
          @click="showAdvanced = true"
          class="btn btn-outline btn-info"
        >
          <i class="fas fa-cog mr-1"></i>
          高级选项
        </button>
        <button 
          class="btn btn-primary"
          :disabled="isTesting"
          @click="runTest"
        >
          <i v-if="isTesting" class="fas fa-spinner fa-spin mr-1"></i>
          <i v-else class="fas fa-play mr-1"></i>
          运行测试
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop bg-black/50" @click="close">
      <button>close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, defineComponent, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'

// 工具类型图标组件
const BuiltinIcon = defineComponent({
  render() {
    return h('svg', { 
      xmlns: 'http://www.w3.org/2000/svg', 
      class: 'h-6 w-6 text-success',
      fill: 'none', 
      viewBox: '0 0 24 24', 
      stroke: 'currentColor' 
    }, [
      h('path', { 
        'stroke-linecap': 'round', 
        'stroke-linejoin': 'round', 
        'stroke-width': '2',
        d: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z'
      }),
      h('path', { 
        'stroke-linecap': 'round', 
        'stroke-linejoin': 'round', 
        'stroke-width': '2',
        d: 'M15 12a3 3 0 11-6 0 3 3 0 016 0z'
      })
    ])
  }
})

const WorkflowIcon = defineComponent({
  render() {
    return h('svg', { 
      xmlns: 'http://www.w3.org/2000/svg', 
      class: 'h-6 w-6 text-secondary',
      fill: 'none', 
      viewBox: '0 0 24 24', 
      stroke: 'currentColor' 
    }, [
      h('path', { 
        'stroke-linecap': 'round', 
        'stroke-linejoin': 'round', 
        'stroke-width': '2',
        d: 'M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z'
      })
    ])
  }
})

const PluginIcon = defineComponent({
  render() {
    return h('svg', { 
      xmlns: 'http://www.w3.org/2000/svg', 
      class: 'h-6 w-6 text-primary',
      fill: 'none', 
      viewBox: '0 0 24 24', 
      stroke: 'currentColor' 
    }, [
      h('path', { 
        'stroke-linecap': 'round', 
        'stroke-linejoin': 'round', 
        'stroke-width': '2',
        d: 'M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z'
      })
    ])
  }
})

// Props
interface Props {
  modelValue: boolean
  toolName: string
  toolType: 'builtin' | 'workflow' | 'plugin'
  toolDescription?: string
  toolVersion?: string
  toolCategory?: string
  inputSchema?: any
  // 用于执行工具的信息
  executionInfo?: {
    type: 'unified' | 'plugin' | 'workflow'
    id?: string
    name?: string
  }
}

const props = withDefaults(defineProps<Props>(), {
  toolDescription: '',
  toolVersion: '',
  toolCategory: '',
  inputSchema: null,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'test-completed', result: any): void
}>()

// State
const showAdvanced = ref(false)
const paramsJson = ref('{}')
const testResult = ref('')
const testDuration = ref<number | null>(null)
const isTesting = ref(false)

// Computed
const toolIcon = computed(() => {
  switch (props.toolType) {
    case 'builtin': return BuiltinIcon
    case 'workflow': return WorkflowIcon
    case 'plugin': return PluginIcon
    default: return BuiltinIcon
  }
})

const categoryBadgeClass = computed(() => {
  switch (props.toolType) {
    case 'builtin': return 'badge-success'
    case 'workflow': return 'badge-secondary'
    case 'plugin': return 'badge-primary'
    default: return 'badge-ghost'
  }
})

const schemaProperties = computed(() => {
  if (!props.inputSchema?.properties) return []
  const required = new Set(props.inputSchema.required || [])
  const properties = []
  for (const name in props.inputSchema.properties) {
    const details = props.inputSchema.properties[name]
    const constraints = []
    if (details.minimum !== undefined) constraints.push(`min: ${details.minimum}`)
    if (details.maximum !== undefined) constraints.push(`max: ${details.maximum}`)
    if (details.enum) constraints.push(`enum: ${details.enum.join(', ')}`)
    properties.push({
      name,
      type: details.type || 'any',
      required: required.has(name),
      description: details.description || '',
      constraints: constraints.join(', ')
    })
  }
  return properties
})

// Methods
function close() {
  emit('update:modelValue', false)
  // Reset state after animation
  setTimeout(() => {
    showAdvanced.value = false
    testResult.value = ''
    testDuration.value = null
  }, 300)
}

function resetParams() {
  paramsJson.value = generateDefaultParams(props.inputSchema)
}

function generateDefaultParams(schema: any): string {
  if (!schema?.properties) return '{}'
  const params: any = {}
  for (const name in schema.properties) {
    const prop = schema.properties[name]
    if (prop.default !== undefined) {
      params[name] = prop.default
    } else {
      switch (prop.type) {
        case 'string': params[name] = ''; break
        case 'number':
        case 'integer': params[name] = prop.minimum ?? 0; break
        case 'boolean': params[name] = false; break
        case 'array': params[name] = []; break
        case 'object': params[name] = {}; break
        default: params[name] = null
      }
    }
  }
  return JSON.stringify(params, null, 2)
}

async function runTest() {
  let inputs: any = {}
  if (paramsJson.value.trim() && showAdvanced.value) {
    try {
      inputs = JSON.parse(paramsJson.value)
    } catch (e) {
      dialog.toast.error('参数 JSON 格式错误，请检查')
      return
    }
  }

  isTesting.value = true
  testResult.value = '正在执行测试...'
  testDuration.value = null
  const startTime = Date.now()

  try {
    // 统一工具执行
    const toolNameToExecute = props.executionInfo?.type === 'workflow' 
      ? `workflow::${props.executionInfo.id}` 
      : props.toolName

    const result = await invoke<any>('unified_execute_tool', {
      toolName: toolNameToExecute,
      inputs,
      context: null,
      timeout: 120,
    })

    testDuration.value = Date.now() - startTime

    if (result.success) {
      testResult.value = typeof result.output === 'string'
        ? result.output
        : JSON.stringify(result.output, null, 2)
      dialog.toast.success('工具测试完成')
    } else {
      testResult.value = `测试失败: ${result.error || '未知错误'}`
      dialog.toast.error('工具测试失败')
    }

    emit('test-completed', result)
  } catch (error: any) {
    testDuration.value = Date.now() - startTime
    console.error('Failed to test tool:', error)
    testResult.value = `测试失败: ${error?.message || String(error)}`
    dialog.toast.error('工具测试失败')
  } finally {
    isTesting.value = false
  }
}

// Watch for modal open to initialize params
watch(() => props.modelValue, (isOpen) => {
  if (isOpen) {
    paramsJson.value = generateDefaultParams(props.inputSchema)
    testResult.value = ''
    testDuration.value = null
  }
})
</script>

<style scoped>
.modal {
  transition: opacity 0.2s ease-in-out;
}

.modal-box {
  transition: transform 0.2s ease-in-out, opacity 0.2s ease-in-out;
}

.modal-open .modal-box {
  animation: modalSlideIn 0.2s ease-out;
}

@keyframes modalSlideIn {
  from {
    opacity: 0;
    transform: translateY(-20px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>
