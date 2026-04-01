<template>
  <Teleport to="body">
  <div v-if="open" class="modal modal-open dictionary-modal">
    <div class="modal-box max-w-2xl dictionary-modal-box">
      <h3 class="font-bold text-lg mb-4">批量编辑规则</h3>

      <form class="space-y-4" @submit.prevent="submit">
        <div class="alert alert-info">
          <span>将对 {{ selectedCount }} 条规则应用批量变更，未填写的字段保持不变。</span>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">分类</span>
          </label>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <select v-model="form.categoryMode" class="select select-bordered">
              <option value="keep">保持不变</option>
              <option value="set">统一设置</option>
              <option value="clear">清空分类</option>
            </select>
            <input
              v-if="form.categoryMode === 'set'"
              v-model.trim="form.category"
              type="text"
              class="input input-bordered md:col-span-2"
              placeholder="例如: exposure / middleware"
            >
          </div>
        </div>

        <div v-if="showSeverity" class="form-control">
          <label class="label">
            <span class="label-text">风险等级</span>
          </label>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <select v-model="form.severityMode" class="select select-bordered">
              <option value="keep">保持不变</option>
              <option value="set">统一设置</option>
              <option value="clear">清空等级</option>
            </select>
            <select
              v-if="form.severityMode === 'set'"
              v-model="form.severity"
              class="select select-bordered md:col-span-2"
            >
              <option value="critical">critical</option>
              <option value="high">high</option>
              <option value="medium">medium</option>
              <option value="low">low</option>
              <option value="info">info</option>
            </select>
          </div>
        </div>

        <div v-if="showFingerprintFields" class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">服务名</span>
            </label>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select v-model="form.serviceMode" class="select select-bordered">
                <option value="keep">保持不变</option>
                <option value="set">统一设置</option>
                <option value="clear">清空</option>
              </select>
              <input
                v-if="form.serviceMode === 'set'"
                v-model.trim="form.service"
                type="text"
                class="input input-bordered md:col-span-2"
                placeholder="例如: http / ssh / redis"
              >
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">协议</span>
            </label>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select v-model="form.protocolMode" class="select select-bordered">
                <option value="keep">保持不变</option>
                <option value="set">统一设置</option>
                <option value="clear">清空</option>
              </select>
              <input
                v-if="form.protocolMode === 'set'"
                v-model.trim="form.protocol"
                type="text"
                class="input input-bordered md:col-span-2"
                placeholder="例如: tcp / http / https"
              >
            </div>
          </div>
        </div>

        <div v-if="showFingerprintFields" class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">Probe 名称</span>
            </label>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select v-model="form.probeNameMode" class="select select-bordered">
                <option value="keep">保持不变</option>
                <option value="set">统一设置</option>
                <option value="clear">清空</option>
              </select>
              <input
                v-if="form.probeNameMode === 'set'"
                v-model.trim="form.probeName"
                type="text"
                class="input input-bordered md:col-span-2"
                placeholder="例如: tcp_banner / http_head"
              >
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">Softmatch</span>
            </label>
            <select v-model="form.softmatchAction" class="select select-bordered">
              <option value="keep">保持不变</option>
              <option value="enable">统一启用</option>
              <option value="disable">统一关闭</option>
            </select>
          </div>
        </div>

        <div v-if="showFingerprintFields" class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">端口列表</span>
            </label>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select v-model="form.portsMode" class="select select-bordered">
                <option value="keep">保持不变</option>
                <option value="set">统一设置</option>
                <option value="clear">清空</option>
              </select>
              <input
                v-if="form.portsMode === 'set'"
                v-model.trim="form.portsText"
                type="text"
                class="input input-bordered md:col-span-2"
                placeholder="例如: 80,443,8080 或 8000-8005"
              >
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">SSL 端口列表</span>
            </label>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <select v-model="form.sslPortsMode" class="select select-bordered">
                <option value="keep">保持不变</option>
                <option value="set">统一设置</option>
                <option value="clear">清空</option>
              </select>
              <input
                v-if="form.sslPortsMode === 'set'"
                v-model.trim="form.sslPortsText"
                type="text"
                class="input input-bordered md:col-span-2"
                placeholder="例如: 443,8443"
              >
            </div>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">追加标签</span>
            </label>
            <input
              v-model.trim="form.addTagsText"
              type="text"
              class="input input-bordered"
              placeholder="逗号分隔，例如 swagger,openapi"
            >
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">移除标签</span>
            </label>
            <input
              v-model.trim="form.removeTagsText"
              type="text"
              class="input input-bordered"
              placeholder="逗号分隔，例如 deprecated,test"
            >
          </div>
        </div>

        <div v-if="showSafeMode" class="form-control">
          <label class="label">
            <span class="label-text">Safe Mode</span>
          </label>
          <select v-model="form.safeModeAction" class="select select-bordered">
            <option value="keep">保持不变</option>
            <option value="enable">统一启用</option>
            <option value="disable">统一关闭</option>
          </select>
        </div>

        <div v-if="supportsMatchers" class="form-control">
          <label class="label">
            <span class="label-text">匹配器</span>
          </label>
          <div class="space-y-3">
            <select v-model="form.matcherMode" class="select select-bordered">
              <option value="keep">保持不变</option>
              <option value="append">为选中规则追加一个 matcher</option>
            </select>

            <div v-if="form.matcherMode === 'append'" class="rounded-lg border border-base-300 p-3 space-y-3">
              <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">Part</span>
                  </label>
                  <select v-model="form.matcherPart" class="select select-bordered">
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
                  <select v-model="form.matcherType" class="select select-bordered">
                    <option value="contains">contains</option>
                    <option value="equals">equals</option>
                    <option value="regex">regex</option>
                    <option value="exists">exists</option>
                    <option value="in">in</option>
                  </select>
                </div>

                <div v-if="form.matcherPart === 'header'" class="form-control">
                  <label class="label">
                    <span class="label-text">Header Key</span>
                  </label>
                  <input v-model.trim="form.matcherKey" type="text" class="input input-bordered">
                </div>

                <div class="form-control" :class="form.matcherPart === 'header' ? '' : 'md:col-span-2'">
                  <label class="label">
                    <span class="label-text">Value</span>
                  </label>
                  <input
                    v-model="form.matcherValueText"
                    type="text"
                    class="input input-bordered"
                    :placeholder="form.matcherType === 'in' ? '例如: 200,302' : '匹配值'"
                  >
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">规则状态</span>
          </label>
          <select v-model="form.enabledAction" class="select select-bordered">
            <option value="keep">保持不变</option>
            <option value="enable">统一启用</option>
            <option value="disable">统一禁用</option>
          </select>
        </div>

        <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-2 text-sm">
          <div class="font-medium">本次变更</div>
          <div class="text-base-content/70">{{ summaryText }}</div>
        </div>

        <div v-if="errorMessage" class="alert alert-error">
          <span>{{ errorMessage }}</span>
        </div>

        <div class="modal-action">
          <button type="button" class="btn" @click="$emit('cancel')">取消</button>
          <button type="submit" class="btn btn-primary">应用到选中规则</button>
        </div>
      </form>
    </div>
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'

type ChangeMode = 'keep' | 'set' | 'clear'
type SafeModeAction = 'keep' | 'enable' | 'disable'
type EnabledAction = 'keep' | 'enable' | 'disable'
type MatcherMode = 'keep' | 'append'

interface RuleMatcherPayload {
  part: string
  type: string
  key?: string
  value?: string | number | Array<string | number>
}

interface BatchRuleEditPayload {
  categoryMode: ChangeMode
  category: string
  severityMode: ChangeMode
  severity: string
  serviceMode: ChangeMode
  service: string
  protocolMode: ChangeMode
  protocol: string
  probeNameMode: ChangeMode
  probeName: string
  portsMode: ChangeMode
  ports: number[]
  sslPortsMode: ChangeMode
  sslPorts: number[]
  softmatchAction: EnabledAction
  addTags: string[]
  removeTags: string[]
  safeModeAction: SafeModeAction
  enabledAction: EnabledAction
  matcherMode: MatcherMode
  appendMatcher?: RuleMatcherPayload
}

const props = defineProps<{
  open: boolean
  dictionaryType: string
  selectedCount: number
}>()

const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'save', value: BatchRuleEditPayload): void
}>()

const form = reactive({
  categoryMode: 'keep' as ChangeMode,
  category: '',
  severityMode: 'keep' as ChangeMode,
  severity: 'medium',
  serviceMode: 'keep' as ChangeMode,
  service: '',
  protocolMode: 'keep' as ChangeMode,
  protocol: '',
  probeNameMode: 'keep' as ChangeMode,
  probeName: '',
  portsMode: 'keep' as ChangeMode,
  portsText: '',
  sslPortsMode: 'keep' as ChangeMode,
  sslPortsText: '',
  softmatchAction: 'keep' as EnabledAction,
  addTagsText: '',
  removeTagsText: '',
  safeModeAction: 'keep' as SafeModeAction,
  enabledAction: 'keep' as EnabledAction,
  matcherMode: 'keep' as MatcherMode,
  matcherPart: 'body',
  matcherType: 'contains',
  matcherKey: '',
  matcherValueText: '',
})

const errorMessage = ref('')

const showSeverity = computed(() =>
  props.dictionaryType === 'sensitive_file' || props.dictionaryType === 'poc_rule'
)
const showFingerprintFields = computed(() =>
  props.dictionaryType === 'fingerprint_rule' || props.dictionaryType === 'service_probe_rule'
)
const showSafeMode = computed(() => props.dictionaryType === 'poc_rule')
const supportsMatchers = computed(() =>
  props.dictionaryType === 'sensitive_file'
  || props.dictionaryType === 'fingerprint_rule'
  || props.dictionaryType === 'service_probe_rule'
  || props.dictionaryType === 'poc_rule'
)

const summaryText = computed(() => {
  const actions: string[] = []

  if (form.categoryMode === 'set' && form.category.trim()) {
    actions.push(`分类 -> ${form.category.trim()}`)
  } else if (form.categoryMode === 'clear') {
    actions.push('清空分类')
  }

  if (showSeverity.value) {
    if (form.severityMode === 'set') {
      actions.push(`风险等级 -> ${form.severity}`)
    } else if (form.severityMode === 'clear') {
      actions.push('清空风险等级')
    }
  }

  if (showFingerprintFields.value) {
    if (form.serviceMode === 'set' && form.service.trim()) {
      actions.push(`服务名 -> ${form.service.trim()}`)
    } else if (form.serviceMode === 'clear') {
      actions.push('清空服务名')
    }

    if (form.protocolMode === 'set' && form.protocol.trim()) {
      actions.push(`协议 -> ${form.protocol.trim()}`)
    } else if (form.protocolMode === 'clear') {
      actions.push('清空协议')
    }

    if (form.probeNameMode === 'set' && form.probeName.trim()) {
      actions.push(`Probe -> ${form.probeName.trim()}`)
    } else if (form.probeNameMode === 'clear') {
      actions.push('清空 Probe')
    }

    if (form.portsMode === 'set' && form.portsText.trim()) {
      actions.push(`端口 -> ${form.portsText.trim()}`)
    } else if (form.portsMode === 'clear') {
      actions.push('清空端口列表')
    }

    if (form.sslPortsMode === 'set' && form.sslPortsText.trim()) {
      actions.push(`SSL 端口 -> ${form.sslPortsText.trim()}`)
    } else if (form.sslPortsMode === 'clear') {
      actions.push('清空 SSL 端口列表')
    }

    if (form.softmatchAction === 'enable') actions.push('Softmatch -> enabled')
    if (form.softmatchAction === 'disable') actions.push('Softmatch -> disabled')
  }

  const addTags = parseCommaSeparated(form.addTagsText)
  if (addTags.length > 0) {
    actions.push(`追加标签: ${addTags.join(', ')}`)
  }

  const removeTags = parseCommaSeparated(form.removeTagsText)
  if (removeTags.length > 0) {
    actions.push(`移除标签: ${removeTags.join(', ')}`)
  }

  if (showSafeMode.value) {
    if (form.safeModeAction === 'enable') actions.push('Safe Mode -> enabled')
    if (form.safeModeAction === 'disable') actions.push('Safe Mode -> disabled')
  }

  if (form.enabledAction === 'enable') actions.push('规则状态 -> enabled')
  if (form.enabledAction === 'disable') actions.push('规则状态 -> disabled')
  if (form.matcherMode === 'append') {
    actions.push(`追加 matcher: ${form.matcherPart}/${form.matcherType}`)
  }

  return actions.length > 0 ? actions.join('；') : '尚未选择任何批量变更'
})

watch(
  () => [props.open, props.dictionaryType],
  () => {
    if (!props.open) return
    resetForm()
  },
  { immediate: true }
)

function resetForm() {
  form.categoryMode = 'keep'
  form.category = ''
  form.severityMode = 'keep'
  form.severity = 'medium'
  form.serviceMode = 'keep'
  form.service = ''
  form.protocolMode = 'keep'
  form.protocol = ''
  form.probeNameMode = 'keep'
  form.probeName = ''
  form.portsMode = 'keep'
  form.portsText = ''
  form.sslPortsMode = 'keep'
  form.sslPortsText = ''
  form.softmatchAction = 'keep'
  form.addTagsText = ''
  form.removeTagsText = ''
  form.safeModeAction = 'keep'
  form.enabledAction = 'keep'
  form.matcherMode = 'keep'
  form.matcherPart = 'body'
  form.matcherType = 'contains'
  form.matcherKey = ''
  form.matcherValueText = ''
  errorMessage.value = ''
}

function parseCommaSeparated(value: string): string[] {
  return Array.from(
    new Set(
      value
        .split(',')
        .map(item => item.trim())
        .filter(Boolean)
    )
  )
}

function parsePortList(value: string): number[] {
  const ports = new Set<number>()
  for (const item of parseCommaSeparated(value)) {
    const range = item.split('-').map(part => Number(part.trim()))
    if (range.length === 2 && Number.isInteger(range[0]) && Number.isInteger(range[1])) {
      const [start, end] = range[0] <= range[1] ? range : [range[1], range[0]]
      for (let port = start; port <= end; port += 1) {
        if (port > 0 && port <= 65535) ports.add(port)
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

function buildMatcher(): RuleMatcherPayload {
  const matcher: RuleMatcherPayload = {
    part: form.matcherPart,
    type: form.matcherType,
  }

  if (form.matcherPart === 'header' && form.matcherKey.trim()) {
    matcher.key = form.matcherKey.trim()
  }

  if (form.matcherType !== 'exists') {
    matcher.value = form.matcherType === 'in'
      ? parseCommaSeparated(form.matcherValueText).map(item => {
          const parsed = Number(item)
          return Number.isNaN(parsed) ? item : parsed
        })
      : form.matcherValueText
  }

  return matcher
}

function submit() {
  if (form.categoryMode === 'set' && !form.category.trim()) {
    errorMessage.value = '分类设置不能为空'
    return
  }

  if (form.matcherMode === 'append') {
    if (form.matcherPart === 'header' && !form.matcherKey.trim()) {
      errorMessage.value = 'Header matcher 需要填写 Header Key'
      return
    }
    if (form.matcherType !== 'exists' && !form.matcherValueText.trim()) {
      errorMessage.value = '追加 matcher 时需要填写匹配值'
      return
    }
  }

  const payload: BatchRuleEditPayload = {
    categoryMode: form.categoryMode,
    category: form.category.trim(),
    severityMode: showSeverity.value ? form.severityMode : 'keep',
    severity: form.severity,
    serviceMode: showFingerprintFields.value ? form.serviceMode : 'keep',
    service: form.service.trim(),
    protocolMode: showFingerprintFields.value ? form.protocolMode : 'keep',
    protocol: form.protocol.trim(),
    probeNameMode: showFingerprintFields.value ? form.probeNameMode : 'keep',
    probeName: form.probeName.trim(),
    portsMode: showFingerprintFields.value ? form.portsMode : 'keep',
    ports: parsePortList(form.portsText),
    sslPortsMode: showFingerprintFields.value ? form.sslPortsMode : 'keep',
    sslPorts: parsePortList(form.sslPortsText),
    softmatchAction: showFingerprintFields.value ? form.softmatchAction : 'keep',
    addTags: parseCommaSeparated(form.addTagsText),
    removeTags: parseCommaSeparated(form.removeTagsText),
    safeModeAction: showSafeMode.value ? form.safeModeAction : 'keep',
    enabledAction: form.enabledAction,
    matcherMode: supportsMatchers.value ? form.matcherMode : 'keep',
    appendMatcher: supportsMatchers.value && form.matcherMode === 'append' ? buildMatcher() : undefined,
  }

  const hasChanges =
    payload.categoryMode !== 'keep' ||
    payload.severityMode !== 'keep' ||
    payload.serviceMode !== 'keep' ||
    payload.protocolMode !== 'keep' ||
    payload.probeNameMode !== 'keep' ||
    payload.portsMode !== 'keep' ||
    payload.sslPortsMode !== 'keep' ||
    payload.softmatchAction !== 'keep' ||
    payload.safeModeAction !== 'keep' ||
    payload.enabledAction !== 'keep' ||
    payload.matcherMode !== 'keep' ||
    payload.addTags.length > 0 ||
    payload.removeTags.length > 0

  if (!hasChanges) {
    errorMessage.value = '至少选择一个批量变更项'
    return
  }

  errorMessage.value = ''
  emit('save', payload)
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
