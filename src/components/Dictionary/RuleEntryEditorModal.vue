<template>
  <Teleport to="body">
  <div v-if="open" class="modal modal-open dictionary-modal" @click.self="$emit('cancel')">
    <div class="modal-box max-w-2xl dictionary-modal-box">
      <h3 class="font-bold text-lg mb-4">
        {{ editing ? '编辑规则' : '新增规则' }}
      </h3>

      <form class="space-y-4" @submit.prevent="submit">
        <div v-if="!editing && starterTemplates.length > 0" class="rounded-lg border border-base-300 bg-base-200/50 p-3">
          <div class="flex flex-col gap-3">
            <div>
              <div class="font-medium">快速模板</div>
              <div class="text-sm text-base-content/70">先填一组常见字段和 matcher，再按你的资产场景微调。</div>
            </div>
            <div class="flex gap-2 flex-wrap">
              <button
                v-for="template in starterTemplates"
                :key="template.key"
                type="button"
                class="btn btn-sm btn-outline"
                @click="applyStarterTemplate(template.key)"
              >
                {{ template.label }}
              </button>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control md:col-span-2">
            <label class="label">
              <span class="label-text">规则标识</span>
            </label>
            <input
              v-model.trim="form.word"
              type="text"
              class="input input-bordered"
              placeholder="例如: swagger_ui / actuator_env / sensitive_path"
              required
            >
          </div>

          <div class="form-control md:col-span-2">
            <label class="label">
              <span class="label-text">分类</span>
            </label>
            <input
              v-model.trim="form.category"
              type="text"
              class="input input-bordered"
              placeholder="例如: api / exposure / ci"
            >
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">权重</span>
            </label>
            <input
              v-model.number="form.weight"
              type="number"
              min="0"
              step="0.1"
              class="input input-bordered"
            >
          </div>

          <div v-if="isSensitiveFile || isPocRule" class="form-control">
            <label class="label">
              <span class="label-text">风险等级</span>
            </label>
            <select v-model="form.severity" class="select select-bordered">
              <option value="critical">critical</option>
              <option value="high">high</option>
              <option value="medium">medium</option>
              <option value="low">low</option>
              <option value="info">info</option>
            </select>
          </div>

          <div v-if="isFingerprintRule" class="form-control">
            <label class="label">
              <span class="label-text">置信度</span>
            </label>
            <input
              v-model.number="form.confidence"
              type="number"
              min="0"
              max="1"
              step="0.05"
              class="input input-bordered"
            >
          </div>

          <div v-if="isFingerprintRule" class="form-control">
            <label class="label">
              <span class="label-text">匹配关系</span>
            </label>
            <select v-model="form.operator" class="select select-bordered">
              <option value="or">or</option>
              <option value="and">and</option>
            </select>
          </div>
        </div>

        <template v-if="isSensitiveFile">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">访问路径</span>
              </label>
              <input
                v-model.trim="form.path"
                type="text"
                class="input input-bordered"
                placeholder="例如: swagger-ui.html"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">标签</span>
              </label>
              <input
                v-model.trim="form.tagsText"
                type="text"
                class="input input-bordered"
                placeholder="用逗号分隔，例如 swagger,openapi"
              >
            </div>
          </div>
        </template>

        <template v-if="isFingerprintRule">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">展示名称</span>
              </label>
              <input
                v-model.trim="form.name"
                type="text"
                class="input input-bordered"
                placeholder="例如: Swagger UI"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">产品名</span>
              </label>
              <input
                v-model.trim="form.product"
                type="text"
                class="input input-bordered"
                placeholder="例如: Jenkins"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">服务名</span>
              </label>
              <input
                v-model.trim="form.service"
                type="text"
                class="input input-bordered"
                placeholder="例如: http / ssh / redis"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">协议</span>
              </label>
              <input
                v-model.trim="form.protocol"
                type="text"
                class="input input-bordered"
                placeholder="例如: tcp / http / https"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">Probe 名称</span>
              </label>
              <input
                v-model.trim="form.probeName"
                type="text"
                class="input input-bordered"
                placeholder="例如: tcp_banner / http_head"
              >
            </div>

            <div class="form-control">
              <label class="label cursor-pointer justify-start gap-3">
                <input v-model="form.softmatch" type="checkbox" class="checkbox checkbox-primary">
                <span class="label-text">Softmatch</span>
              </label>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">资产分类</span>
              </label>
              <input
                v-model.trim="form.assetCategory"
                type="text"
                class="input input-bordered"
                placeholder="例如: web_server / router / middleware"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">资产家族</span>
              </label>
              <input
                v-model.trim="form.assetFamily"
                type="text"
                class="input input-bordered"
                placeholder="例如: network_device / observability"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">厂商</span>
              </label>
              <input
                v-model.trim="form.vendor"
                type="text"
                class="input input-bordered"
                placeholder="例如: Cisco / VMware / HashiCorp"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">别名</span>
              </label>
              <input
                v-model.trim="form.aliasesText"
                type="text"
                class="input input-bordered"
                placeholder="用逗号分隔，例如 nginx,openresty"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">版本</span>
              </label>
              <input
                v-model.trim="form.version"
                type="text"
                class="input input-bordered"
                placeholder="例如: 1.24.0"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">优先级</span>
              </label>
              <input
                v-model.number="form.priority"
                type="number"
                min="0"
                step="1"
                class="input input-bordered"
                placeholder="数字越大越优先"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">规则 ID</span>
              </label>
              <input
                v-model.trim="form.ruleId"
                type="text"
                class="input input-bordered"
                placeholder="留空则按词条标识生成"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">端口列表</span>
              </label>
              <input
                v-model.trim="form.portsText"
                type="text"
                class="input input-bordered"
                placeholder="例如: 80,443,8080 或 8000-8005"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">SSL 端口列表</span>
              </label>
              <input
                v-model.trim="form.sslPortsText"
                type="text"
                class="input input-bordered"
                placeholder="例如: 443,8443"
              >
            </div>
          </div>
        </template>

        <template v-if="isPocRule">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">规则名称</span>
              </label>
              <input
                v-model.trim="form.name"
                type="text"
                class="input input-bordered"
                placeholder="例如: Spring Actuator Env Exposure"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">发现类型</span>
              </label>
              <input
                v-model.trim="form.findingType"
                type="text"
                class="input input-bordered"
                placeholder="例如: config_exposure"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">请求方法</span>
              </label>
              <select v-model="form.requestMethod" class="select select-bordered">
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="DELETE">DELETE</option>
              </select>
            </div>

            <div class="form-control md:col-span-2">
              <label class="label">
                <span class="label-text">请求路径</span>
              </label>
              <input
                v-model.trim="form.requestPath"
                type="text"
                class="input input-bordered"
                placeholder="例如: /actuator/env"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">超时(ms)</span>
              </label>
              <input
                v-model.number="form.timeoutMs"
                type="number"
                min="1000"
                step="500"
                class="input input-bordered"
              >
            </div>

            <div class="form-control md:col-span-2">
              <label class="label cursor-pointer justify-start gap-3">
                <input v-model="form.safeMode" type="checkbox" class="checkbox checkbox-primary">
                <span class="label-text">仅安全验证模式</span>
              </label>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">目标类型</span>
              </label>
              <input
                v-model.trim="form.targetTypesText"
                type="text"
                class="input input-bordered"
                placeholder="例如: web,service"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">关联指纹</span>
              </label>
              <input
                v-model.trim="form.fingerprintScopeText"
                type="text"
                class="input input-bordered"
                placeholder="例如: spring_boot,jenkins"
              >
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">产品范围</span>
              </label>
              <input
                v-model.trim="form.productScopeText"
                type="text"
                class="input input-bordered"
                placeholder="例如: Jenkins,Grafana"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">厂商范围</span>
              </label>
              <input
                v-model.trim="form.vendorScopeText"
                type="text"
                class="input input-bordered"
                placeholder="例如: Atlassian,HashiCorp"
              >
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">端口范围</span>
              </label>
              <input
                v-model.trim="form.portScopeText"
                type="text"
                class="input input-bordered"
                placeholder="例如: 80,443,8080"
              >
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">标签</span>
            </label>
            <input
              v-model.trim="form.tagsText"
              type="text"
              class="input input-bordered"
              placeholder="用逗号分隔，例如 exposure,spring,actuator"
            >
          </div>
        </template>

        <div class="form-control">
          <label class="label">
            <span class="label-text">描述</span>
          </label>
          <textarea
            v-model="form.description"
            class="textarea textarea-bordered min-h-24"
            placeholder="规则描述"
          />
        </div>

        <div v-if="isPocRule || isSensitiveFile" class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">CWE</span>
            </label>
            <input
              v-model.trim="form.cwe"
              type="text"
              class="input input-bordered"
              placeholder="例如: CWE-200"
            >
          </div>

          <div v-if="isPocRule" class="form-control">
            <label class="label">
              <span class="label-text">影响</span>
            </label>
            <input
              v-model.trim="form.impact"
              type="text"
              class="input input-bordered"
              placeholder="例如: Environment variables exposed"
            >
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">修复建议</span>
          </label>
          <textarea
            v-model="form.remediation"
            class="textarea textarea-bordered min-h-24"
            placeholder="修复建议"
          />
        </div>

        <div v-if="showMatchers" class="space-y-3">
          <div class="flex items-center justify-between">
            <h4 class="font-semibold">匹配器</h4>
            <button type="button" class="btn btn-sm btn-outline" @click="addMatcher">
              <i class="fas fa-plus mr-2"></i>
              添加匹配器
            </button>
          </div>

          <div
            v-for="(matcher, index) in form.matchers"
            :key="index"
            class="rounded-lg border border-base-300 p-3 space-y-3"
          >
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">Part</span>
                </label>
                <select v-model="matcher.part" class="select select-bordered">
                  <option value="body">body</option>
                  <option value="title">title</option>
                  <option value="header">header</option>
                  <option value="status">status</option>
                  <option value="banner">banner</option>
                  <option value="product">product</option>
                  <option value="service">service</option>
                </select>
              </div>

              <div class="form-control">
                <label class="label">
                  <span class="label-text">Type</span>
                </label>
                <select v-model="matcher.type" class="select select-bordered">
                  <option value="contains">contains</option>
                  <option value="equals">equals</option>
                  <option value="regex">regex</option>
                  <option value="exists">exists</option>
                  <option value="in">in</option>
                </select>
              </div>

              <div v-if="matcher.part === 'header'" class="form-control">
                <label class="label">
                  <span class="label-text">Header Key</span>
                </label>
                <input v-model.trim="matcher.key" type="text" class="input input-bordered">
              </div>

              <div class="form-control" :class="matcher.part === 'header' ? '' : 'md:col-span-2'">
                <label class="label">
                  <span class="label-text">Value</span>
                </label>
                <input
                  v-model="matcher.valueText"
                  type="text"
                  class="input input-bordered"
                  :placeholder="matcher.type === 'in' ? '例如: 200,302' : '匹配值'"
                >
              </div>
            </div>

            <div class="flex justify-end">
              <button type="button" class="btn btn-xs btn-ghost text-error" @click="removeMatcher(index)">
                删除
              </button>
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3">
          <div class="flex items-center justify-between">
            <h4 class="font-semibold">规则预览</h4>
            <div class="text-xs text-base-content/60">
              {{ previewSummary }}
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
            <div class="rounded-md bg-base-200 px-3 py-2">
              <div class="text-xs text-base-content/60">标识</div>
              <div class="font-medium break-all">{{ form.word || '-' }}</div>
            </div>
            <div class="rounded-md bg-base-200 px-3 py-2">
              <div class="text-xs text-base-content/60">分类</div>
              <div class="font-medium">{{ form.category || '-' }}</div>
            </div>
            <div class="rounded-md bg-base-200 px-3 py-2">
              <div class="text-xs text-base-content/60">权重</div>
              <div class="font-medium">{{ form.weight || 1 }}</div>
            </div>
          </div>

          <div v-if="previewError" class="alert alert-warning">
            <span>{{ previewError }}</span>
          </div>

          <pre class="rounded-md bg-neutral text-neutral-content p-3 text-xs overflow-x-auto">{{ previewJson }}</pre>
        </div>

        <div class="collapse collapse-arrow border border-base-300 bg-base-100">
          <input type="checkbox">
          <div class="collapse-title text-sm font-medium">
            高级元数据
          </div>
          <div class="collapse-content">
            <textarea
              v-model="form.extraMetadataText"
              class="textarea textarea-bordered w-full min-h-40 font-mono text-xs"
              placeholder="{&quot;custom&quot;: true}"
            />
          </div>
        </div>

        <div v-if="errorMessage" class="alert alert-error">
          <span>{{ errorMessage }}</span>
        </div>

        <div class="modal-action">
          <button type="button" class="btn" @click="$emit('cancel')">取消</button>
          <button type="submit" class="btn btn-primary">保存规则</button>
        </div>
      </form>
    </div>
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import {
  getRuleStarterTemplate,
  getRuleStarterTemplates,
} from '@/components/Dictionary/ruleStarterTemplates'

interface RuleMatcherForm {
  part: string
  type: string
  key: string
  valueText: string
}

interface RuleEntryValue {
  id?: string
  word: string
  category?: string | null
  weight?: number | null
  metadata?: Record<string, any> | null
}

const props = defineProps<{
  open: boolean
  dictionaryType: string
  dictionarySubtype?: string
  value: RuleEntryValue | null
}>()

const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'save', value: RuleEntryValue): void
}>()

const handledMetadataKeysByType: Record<string, string[]> = {
  sensitive_file: ['path', 'severity', 'tags', 'description', 'cwe', 'remediation', 'matchers'],
  fingerprint_rule: [
    'name',
    'service',
    'product',
    'vendor',
    'version',
    'asset_category',
    'asset_family',
    'rule_id',
    'protocol',
    'probeName',
    'ports',
    'sslPorts',
    'softmatch',
    'aliases',
    'priority',
    'confidence',
    'operator',
    'matchers',
  ],
  service_probe_rule: [
    'name',
    'service',
    'product',
    'vendor',
    'version',
    'asset_category',
    'asset_family',
    'rule_id',
    'protocol',
    'probeName',
    'ports',
    'sslPorts',
    'softmatch',
    'aliases',
    'priority',
    'confidence',
    'operator',
    'matchers',
  ],
  poc_rule: [
    'name',
    'finding_type',
    'severity',
    'target_types',
    'tags',
    'match_scope',
    'request',
    'matchers',
    'impact',
    'remediation',
    'safe_mode',
    'description',
    'cwe',
  ],
}

const form = reactive({
  id: '',
  word: '',
  category: '',
  weight: 1,
  path: '',
  severity: 'medium',
  tagsText: '',
  description: '',
  cwe: '',
  remediation: '',
  name: '',
  service: '',
  product: '',
  vendor: '',
  version: '',
  assetCategory: '',
  assetFamily: '',
  ruleId: '',
  protocol: '',
  probeName: '',
  portsText: '',
  sslPortsText: '',
  softmatch: false,
  aliasesText: '',
  priority: 100,
  confidence: 0.8,
  operator: 'or',
  findingType: 'risk_verification',
  requestMethod: 'GET',
  requestPath: '/',
  timeoutMs: 8000,
  safeMode: true,
  targetTypesText: 'web',
  fingerprintScopeText: '',
  productScopeText: '',
  vendorScopeText: '',
  portScopeText: '',
  impact: '',
  extraMetadataText: '',
  matchers: [] as RuleMatcherForm[],
})

const validationError = ref('')
const errorMessage = computed(() => validationError.value)

const isSensitiveFile = computed(() => props.dictionaryType === 'sensitive_file')
const isFingerprintRule = computed(() =>
  props.dictionaryType === 'fingerprint_rule' || props.dictionaryType === 'service_probe_rule'
)
const isPocRule = computed(() => props.dictionaryType === 'poc_rule')
const showMatchers = computed(() => isSensitiveFile.value || isFingerprintRule.value || isPocRule.value)
const editing = computed(() => Boolean(props.value?.id))
const starterTemplates = computed(() =>
  getRuleStarterTemplates(props.dictionaryType, props.dictionarySubtype)
)
const previewResult = computed(() => {
  try {
    return {
      value: buildRulePayload(),
      error: '',
    }
  } catch (error) {
    return {
      value: null,
      error: `预览生成失败: ${String(error)}`,
    }
  }
})
const previewError = computed(() => previewResult.value.error)
const previewJson = computed(() =>
  previewResult.value.value
    ? JSON.stringify(previewResult.value.value, null, 2)
    : '{}'
)
const previewSummary = computed(() => {
  if (previewResult.value.error) return '存在预览错误'
  const matchers = Array.isArray(previewResult.value.value?.metadata?.matchers)
    ? previewResult.value.value.metadata.matchers.length
    : 0
  if (isPocRule.value) {
    const request = previewResult.value.value?.metadata?.request || {}
    return `${request.method || 'GET'} ${request.path || '/'} · ${matchers} matcher`
  }
  if (isFingerprintRule.value) return `${matchers} matcher`
  if (isSensitiveFile.value) return `${previewResult.value.value?.metadata?.path || form.word || '-'}`
  return ''
})

watch(
  () => [props.open, props.value, props.dictionaryType],
  () => {
    resetForm()
  },
  { immediate: true, deep: true }
)

function resetForm() {
  const metadata = props.value?.metadata || {}
  const extraMetadata = { ...metadata }
  for (const key of handledMetadataKeysByType[props.dictionaryType] || []) {
    delete extraMetadata[key]
  }

  form.id = props.value?.id || ''
  form.word = props.value?.word || ''
  form.category = props.value?.category || ''
  form.weight = Number(props.value?.weight ?? 1)
  form.path = metadata.path || props.value?.word || ''
  form.severity = metadata.severity || 'medium'
  form.tagsText = Array.isArray(metadata.tags) ? metadata.tags.join(',') : ''
  form.description = metadata.description || ''
  form.cwe = metadata.cwe || ''
  form.remediation = metadata.remediation || ''
  form.name = metadata.name || ''
  form.service = metadata.service || ''
  form.product = metadata.product || ''
  form.vendor = metadata.vendor || ''
  form.version = metadata.version || ''
  form.assetCategory = metadata.asset_category || ''
  form.assetFamily = metadata.asset_family || ''
  form.ruleId = metadata.rule_id || ''
  form.protocol = metadata.protocol || ''
  form.probeName = metadata.probeName || metadata.probe_name || ''
  form.portsText = Array.isArray(metadata.ports) ? metadata.ports.join(',') : ''
  form.sslPortsText = Array.isArray(metadata.sslPorts)
    ? metadata.sslPorts.join(',')
    : Array.isArray(metadata.ssl_ports)
      ? metadata.ssl_ports.join(',')
      : ''
  form.softmatch = metadata.softmatch === true
  form.aliasesText = Array.isArray(metadata.aliases) ? metadata.aliases.join(',') : ''
  form.priority = Number(metadata.priority ?? 100)
  form.confidence = Number(metadata.confidence ?? 0.8)
  form.operator = metadata.operator || 'or'
  form.findingType = metadata.finding_type || 'risk_verification'
  form.requestMethod = metadata.request?.method || 'GET'
  form.requestPath = metadata.request?.path || '/'
  form.timeoutMs = Number(metadata.request?.timeout_ms ?? 8000)
  form.safeMode = metadata.safe_mode !== false
  form.targetTypesText = Array.isArray(metadata.target_types) ? metadata.target_types.join(',') : 'web'
  form.fingerprintScopeText = Array.isArray(metadata.match_scope?.fingerprints)
    ? metadata.match_scope.fingerprints.join(',')
    : ''
  form.productScopeText = Array.isArray(metadata.match_scope?.products)
    ? metadata.match_scope.products.join(',')
    : ''
  form.vendorScopeText = Array.isArray(metadata.match_scope?.vendors)
    ? metadata.match_scope.vendors.join(',')
    : ''
  form.portScopeText = Array.isArray(metadata.match_scope?.ports)
    ? metadata.match_scope.ports.join(',')
    : ''
  form.impact = metadata.impact || ''
  form.matchers = normalizeMatchers(metadata.matchers)
  form.extraMetadataText = Object.keys(extraMetadata).length > 0 ? JSON.stringify(extraMetadata, null, 2) : ''
  validationError.value = ''
}

function normalizeMatchers(matchers: any): RuleMatcherForm[] {
  if (!Array.isArray(matchers) || matchers.length === 0) return []
  return matchers.map((matcher: any) => ({
    part: matcher.part || 'body',
    type: matcher.type || 'contains',
    key: matcher.key || '',
    valueText: Array.isArray(matcher.value) ? matcher.value.join(',') : String(matcher.value ?? ''),
  }))
}

function addMatcher() {
  form.matchers.push({
    part: 'body',
    type: 'contains',
    key: '',
    valueText: '',
  })
}

function applyStarterTemplate(key: string) {
  const template = getRuleStarterTemplate(props.dictionaryType, props.dictionarySubtype, key)
  if (!template) return
  const payload = template.payload

  form.word = payload.word ?? form.word
  form.category = payload.category ?? form.category
  form.severity = payload.severity ?? form.severity
  form.name = payload.name ?? form.name
  form.service = payload.service ?? form.service
  form.product = payload.product ?? form.product
  form.vendor = payload.vendor ?? form.vendor
  form.assetCategory = payload.assetCategory ?? form.assetCategory
  form.assetFamily = payload.assetFamily ?? form.assetFamily
  form.protocol = payload.protocol ?? form.protocol
  form.probeName = payload.probeName ?? form.probeName
  form.portsText = payload.portsText ?? form.portsText
  form.sslPortsText = payload.sslPortsText ?? form.sslPortsText
  form.softmatch = payload.softmatch ?? form.softmatch
  form.priority = payload.priority ?? form.priority
  form.confidence = payload.confidence ?? form.confidence
  form.operator = payload.operator ?? form.operator
  form.findingType = payload.findingType ?? form.findingType
  form.requestMethod = payload.requestMethod ?? form.requestMethod
  form.requestPath = payload.requestPath ?? form.requestPath
  form.timeoutMs = payload.timeoutMs ?? form.timeoutMs
  form.safeMode = payload.safeMode ?? form.safeMode
  form.targetTypesText = payload.targetTypesText ?? form.targetTypesText
  form.fingerprintScopeText = payload.fingerprintScopeText ?? form.fingerprintScopeText
  form.productScopeText = payload.productScopeText ?? form.productScopeText
  form.vendorScopeText = payload.vendorScopeText ?? form.vendorScopeText
  form.portScopeText = payload.portScopeText ?? form.portScopeText
  form.matchers = (payload.matchers || []).map(matcher => ({
    part: matcher.part,
    type: matcher.type,
    key: matcher.key || '',
    valueText: matcher.valueText || '',
  }))
}

function removeMatcher(index: number) {
  form.matchers.splice(index, 1)
}

function parseCommaSeparated(value: string): string[] {
  return value
    .split(',')
    .map(item => item.trim())
    .filter(Boolean)
}

function parsePortList(value: string): number[] {
  const ports = new Set<number>()

  for (const item of parseCommaSeparated(value)) {
    const range = item.split('-').map(part => Number(part.trim()))
    if (range.length === 2 && Number.isInteger(range[0]) && Number.isInteger(range[1])) {
      const [start, end] = range[0] <= range[1] ? range : [range[1], range[0]]
      for (let port = start; port <= end; port += 1) {
        if (port > 0 && port <= 65535) {
          ports.add(port)
        }
      }
      continue
    }

    const port = Number(item)
    if (Number.isInteger(port) && port > 0 && port <= 65535) {
      ports.add(port)
    }
  }

  return Array.from(ports).sort((left, right) => left - right)
}

function buildMatchers() {
  return form.matchers
    .map(matcher => {
      const value = matcher.type === 'in'
        ? parseCommaSeparated(matcher.valueText).map(item => {
            const parsed = Number(item)
            return Number.isNaN(parsed) ? item : parsed
          })
        : matcher.valueText

      const result: Record<string, any> = {
        part: matcher.part,
        type: matcher.type,
      }

      if (matcher.part === 'header' && matcher.key.trim()) {
        result.key = matcher.key.trim()
      }

      if (matcher.type !== 'exists') {
        result.value = value
      }

      return result
    })
    .filter(matcher => matcher.type === 'exists' || matcher.value !== '' || (Array.isArray(matcher.value) && matcher.value.length > 0))
}

function submit() {
  try {
    emit('save', buildRulePayload())
    validationError.value = ''
  } catch (error) {
    validationError.value = `高级元数据 JSON 无法解析: ${String(error)}`
  }
}

function buildRulePayload(): RuleEntryValue {
  const extraMetadata = form.extraMetadataText.trim() ? JSON.parse(form.extraMetadataText) : {}
  const metadata: Record<string, any> = { ...extraMetadata }

  if (isSensitiveFile.value) {
    metadata.path = form.path.trim() || form.word.trim()
    metadata.severity = form.severity
    metadata.description = form.description.trim() || undefined
    metadata.cwe = form.cwe.trim() || undefined
    metadata.remediation = form.remediation.trim() || undefined
    const tags = parseCommaSeparated(form.tagsText)
    if (tags.length > 0) metadata.tags = tags
    const matchers = buildMatchers()
    if (matchers.length > 0) metadata.matchers = matchers
  }

  if (isFingerprintRule.value) {
    metadata.name = form.name.trim() || form.word.trim()
    metadata.service = form.service.trim() || undefined
    metadata.product = form.product.trim() || undefined
    metadata.vendor = form.vendor.trim() || undefined
    metadata.version = form.version.trim() || undefined
    metadata.asset_category = form.assetCategory.trim() || undefined
    metadata.asset_family = form.assetFamily.trim() || undefined
    metadata.rule_id = form.ruleId.trim() || undefined
    metadata.protocol = form.protocol.trim() || undefined
    metadata.probeName = form.probeName.trim() || undefined
    metadata.ports = parsePortList(form.portsText)
    metadata.sslPorts = parsePortList(form.sslPortsText)
    metadata.softmatch = form.softmatch
    const aliases = parseCommaSeparated(form.aliasesText)
    if (aliases.length > 0) metadata.aliases = aliases
    metadata.priority = Number.isFinite(Number(form.priority)) ? Number(form.priority) : 100
    metadata.confidence = Number(form.confidence)
    metadata.operator = form.operator
    metadata.matchers = buildMatchers()
  }

  if (isPocRule.value) {
    metadata.name = form.name.trim() || form.word.trim()
    metadata.finding_type = form.findingType.trim() || 'risk_verification'
    metadata.severity = form.severity
    metadata.target_types = parseCommaSeparated(form.targetTypesText)
    const tags = parseCommaSeparated(form.tagsText)
    if (tags.length > 0) metadata.tags = tags
    metadata.match_scope = {
      ...(metadata.match_scope || {}),
      fingerprints: parseCommaSeparated(form.fingerprintScopeText),
      products: parseCommaSeparated(form.productScopeText),
      vendors: parseCommaSeparated(form.vendorScopeText),
      ports: parseCommaSeparated(form.portScopeText)
        .map(item => Number(item))
        .filter(item => Number.isFinite(item)),
    }
    metadata.request = {
      method: form.requestMethod,
      path: form.requestPath.trim() || '/',
      timeout_ms: Number(form.timeoutMs) || 8000,
    }
    metadata.matchers = buildMatchers()
    metadata.impact = form.impact.trim() || undefined
    metadata.remediation = form.remediation.trim() || undefined
    metadata.description = form.description.trim() || undefined
    metadata.cwe = form.cwe.trim() || undefined
    metadata.safe_mode = form.safeMode
  }

  return {
    id: form.id || undefined,
    word: form.word.trim(),
    category: form.category.trim() || null,
    weight: Number(form.weight) || 1,
    metadata,
  }
}
</script>

<style scoped>
.dictionary-modal {
  z-index: 70;
  align-items: flex-start;
  padding: 5rem 1rem 1.5rem;
}

.dictionary-modal-box {
  max-height: calc(100vh - 6.5rem);
  overflow-y: auto;
}
</style>
