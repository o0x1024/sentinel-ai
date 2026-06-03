<template>
  <AppDialog ref="dialogRef" class="modal">
    <div class="modal-box flex max-h-[90vh] w-[92vw] max-w-3xl flex-col overflow-hidden p-0">
      <div class="border-b border-base-300 px-5 py-4">
        <h3 class="text-center text-lg font-semibold">
          {{ isEditing ? '编辑 Codec 规则' : '新建 Codec 规则' }}
        </h3>
      </div>

      <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4">
        <label class="form-control">
          <span class="label-text mb-1">规则名称</span>
          <input
            v-model="form.name"
            type="text"
            class="input input-bordered"
            placeholder="输入规则名称"
          />
        </label>

        <details class="rounded-lg border border-base-300 bg-base-100" open>
          <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">匹配条件</summary>
          <div class="space-y-3 border-t border-base-300 px-4 py-3">
            <div class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center">
              <span class="text-sm text-base-content/70">Host</span>
              <input v-model="form.host" type="text" class="input input-bordered input-sm" placeholder="例如 api.example.com 或 *.example.com" />
            </div>
            <div class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center">
              <span class="text-sm text-base-content/70">Path</span>
              <input v-model="form.path" type="text" class="input input-bordered input-sm" placeholder="例如 /api/*" />
            </div>
            <div>
              <span class="mb-2 block text-sm text-base-content/70">Method</span>
              <div class="flex flex-wrap gap-3">
                <label
                  v-for="method in httpMethods"
                  :key="method"
                  class="flex items-center gap-2 text-sm"
                >
                  <input
                    v-model="form.methods"
                    type="checkbox"
                    class="checkbox checkbox-sm"
                    :value="method"
                  />
                  <span>{{ method }}</span>
                </label>
              </div>
            </div>
          </div>
        </details>

        <details class="rounded-lg border border-base-300 bg-base-100" open>
          <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">作用范围</summary>
          <div class="space-y-3 border-t border-base-300 px-4 py-3">
            <div class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center">
              <span class="text-sm text-base-content/70">类型</span>
              <select v-model="form.scopeTarget" class="select select-bordered select-sm">
                <option v-for="option in scopeTargetOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div
              v-if="showScopeFields"
              class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center"
            >
              <span class="text-sm text-base-content/70">字段</span>
              <input
                v-model="form.scopeFields"
                type="text"
                class="input input-bordered input-sm"
                placeholder="逗号分隔，例如 data, payload.token"
              />
            </div>
            <div
              v-if="form.scopeTarget === 'header-value'"
              class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center"
            >
              <span class="text-sm text-base-content/70">Header</span>
              <input
                v-model="form.scopeHeaderName"
                type="text"
                class="input input-bordered input-sm"
                placeholder="例如 Authorization"
              />
            </div>
            <div
              v-if="form.scopeTarget === 'regex-match'"
              class="grid gap-2 sm:grid-cols-[6rem_minmax(0,1fr)] sm:items-center"
            >
              <span class="text-sm text-base-content/70">Pattern</span>
              <input
                v-model="form.scopePattern"
                type="text"
                class="input input-bordered input-sm font-mono"
                placeholder="正则表达式"
              />
            </div>
          </div>
        </details>

        <details class="rounded-lg border border-base-300 bg-base-100" open>
          <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">解密管道</summary>
          <div class="border-t border-base-300 px-4 py-3">
            <TrafficCodecPipelineEditor v-model="form.steps" />
          </div>
        </details>

        <details class="rounded-lg border border-base-300 bg-base-100" open>
          <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">预览</summary>
          <div class="space-y-3 border-t border-base-300 px-4 py-3">
            <label class="form-control">
              <span class="label-text mb-1">输入</span>
              <textarea
                v-model="previewInput"
                class="textarea textarea-bordered min-h-24 font-mono text-sm"
                placeholder="输入测试内容"
              />
            </label>
            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                class="btn btn-sm btn-outline"
                :disabled="previewLoading"
                @click="runPreview('decode')"
              >
                测试解密
              </button>
              <button
                type="button"
                class="btn btn-sm btn-outline"
                :disabled="previewLoading"
                @click="runPreview('encode')"
              >
                测试加密
              </button>
            </div>
            <label class="form-control">
              <span class="label-text mb-1">输出</span>
              <textarea
                :value="previewOutput"
                readonly
                class="textarea textarea-bordered min-h-24 font-mono text-sm"
                placeholder="测试结果将显示在这里"
              />
            </label>
            <p v-if="previewError" class="text-sm text-error">{{ previewError }}</p>
          </div>
        </details>

        <p v-if="validationError" class="text-sm text-error">{{ validationError }}</p>
      </div>

      <div class="flex items-center justify-end gap-3 border-t border-base-300 bg-base-100 px-5 py-4">
        <button class="btn btn-ghost" type="button" @click="close">取消</button>
        <button class="btn btn-primary" type="button" :disabled="saving" @click="save">
          {{ saving ? '保存中...' : '保存' }}
        </button>
      </div>
    </div>

    <form method="dialog" class="modal-backdrop">
      <button type="button" @click="close">关闭</button>
    </form>
  </AppDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import AppDialog from '@/components/AppDialog.vue'
import TrafficCodecPipelineEditor from './TrafficCodecPipelineEditor.vue'
import { useTrafficCodec } from './useTrafficCodec'
import { useTrafficCodecRuleStore } from './trafficCodecRuleStore'
import type {
  CodecRequestMeta,
  CodecScopeTarget,
  CodecStep,
  TrafficCodecRule,
} from './trafficCodecTypes'

const props = defineProps<{
  initialMeta?: CodecRequestMeta
  editRule?: TrafficCodecRule
}>()

const emit = defineEmits<{
  saved: [rule: TrafficCodecRule]
  closed: []
}>()

const httpMethods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as const

const scopeTargetOptions: Array<{ value: CodecScopeTarget; label: string }> = [
  { value: 'json-field', label: 'JSON 字段' },
  { value: 'query-param', label: 'Query 参数' },
  { value: 'form-field', label: 'Form 字段' },
  { value: 'full-body', label: '完整 Body' },
  { value: 'header-value', label: 'Header 值' },
  { value: 'regex-match', label: '正则匹配' },
]

const dialogRef = ref<InstanceType<typeof AppDialog> | null>(null)
const { testPipeline } = useTrafficCodec()
const ruleStore = useTrafficCodecRuleStore()

const editingRuleId = ref<string | null>(null)
const editingCreatedAt = ref('')
const saving = ref(false)
const validationError = ref('')
const previewInput = ref('')
const previewOutput = ref('')
const previewError = ref('')
const previewLoading = ref(false)

const form = reactive({
  name: '',
  host: '',
  path: '',
  methods: [] as string[],
  scopeTarget: 'json-field' as CodecScopeTarget,
  scopeFields: '',
  scopeHeaderName: '',
  scopePattern: '',
  steps: [] as CodecStep[],
})

const isEditing = computed(() => editingRuleId.value !== null)

const showScopeFields = computed(() =>
  ['json-field', 'query-param', 'form-field'].includes(form.scopeTarget),
)

function createDefaultStep(): CodecStep {
  return {
    id: crypto.randomUUID(),
    type: 'builtin',
    codec: 'base64',
    config: { variant: 'standard' },
    enabled: true,
  }
}

function resetForm() {
  form.name = ''
  form.host = ''
  form.path = ''
  form.methods = []
  form.scopeTarget = 'json-field'
  form.scopeFields = ''
  form.scopeHeaderName = ''
  form.scopePattern = ''
  form.steps = [createDefaultStep()]
  editingRuleId.value = null
  editingCreatedAt.value = ''
  validationError.value = ''
  previewInput.value = ''
  previewOutput.value = ''
  previewError.value = ''
}

function populateFromProps() {
  if (props.editRule) {
    const rule = props.editRule
    editingRuleId.value = rule.id
    editingCreatedAt.value = rule.createdAt
    form.name = rule.name
    form.host = rule.matchRule.hosts[0] ?? ''
    form.path = rule.matchRule.paths[0] ?? ''
    form.methods = [...rule.matchRule.methods]
    form.scopeTarget = rule.scope.target
    form.scopeFields = rule.scope.fields.join(', ')
    form.scopeHeaderName = rule.scope.headerName ?? ''
    form.scopePattern = rule.scope.pattern ?? ''
    form.steps = rule.pipeline.steps.map(step => ({
      ...step,
      config: { ...step.config },
    }))
    return
  }

  if (props.initialMeta) {
    form.host = props.initialMeta.host
    form.path = props.initialMeta.path
    if (props.initialMeta.method) {
      form.methods = [props.initialMeta.method.toUpperCase()]
    }
  }
}

function splitCsv(value: string): string[] {
  return value
    .split(',')
    .map(item => item.trim())
    .filter(Boolean)
}

function buildRule(): TrafficCodecRule {
  const now = new Date().toISOString()
  return {
    id: editingRuleId.value ?? crypto.randomUUID(),
    name: form.name.trim(),
    enabled: props.editRule?.enabled ?? true,
    order: props.editRule?.order ?? ruleStore.rules.value.length,
    matchRule: {
      hosts: form.host.trim() ? [form.host.trim()] : [],
      paths: form.path.trim() ? [form.path.trim()] : [],
      methods: [...form.methods],
      contentTypes: props.editRule?.matchRule.contentTypes ?? [],
    },
    scope: {
      target: form.scopeTarget,
      fields: showScopeFields.value ? splitCsv(form.scopeFields) : [],
      headerName: form.scopeTarget === 'header-value' ? form.scopeHeaderName.trim() || undefined : undefined,
      pattern: form.scopeTarget === 'regex-match' ? form.scopePattern.trim() || undefined : undefined,
    },
    pipeline: {
      steps: form.steps.map(step => ({
        ...step,
        config: { ...step.config },
      })),
    },
    reversible: props.editRule?.reversible ?? true,
    createdAt: editingCreatedAt.value || now,
    updatedAt: now,
  }
}

function validate(): boolean {
  validationError.value = ''
  if (!form.name.trim()) {
    validationError.value = '请填写规则名称'
    return false
  }
  if (form.steps.length === 0) {
    validationError.value = '至少需要一个管道步骤'
    return false
  }
  return true
}

async function save() {
  if (!validate()) return

  saving.value = true
  try {
    const rule = buildRule()
    await ruleStore.saveRule(rule)
    emit('saved', rule)
    close()
  } catch (error) {
    validationError.value = error instanceof Error ? error.message : '保存失败'
  } finally {
    saving.value = false
  }
}

async function runPreview(direction: 'decode' | 'encode') {
  previewLoading.value = true
  previewError.value = ''
  previewOutput.value = ''
  try {
    const result = await testPipeline(previewInput.value, form.steps, direction)
    if (result.success) {
      previewOutput.value = result.content
    } else {
      previewError.value = result.error ?? '测试失败'
    }
  } catch (error) {
    previewError.value = error instanceof Error ? error.message : '测试失败'
  } finally {
    previewLoading.value = false
  }
}

function showModal() {
  resetForm()
  populateFromProps()
  dialogRef.value?.showModal()
}

function close() {
  dialogRef.value?.close()
  emit('closed')
}

defineExpose({
  showModal,
  close,
})
</script>
