<template>
  <div v-if="open" class="modal modal-open">
    <div class="modal-box max-w-2xl">
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
const showSafeMode = computed(() => props.dictionaryType === 'poc_rule')
const supportsMatchers = computed(() =>
  props.dictionaryType === 'sensitive_file' || props.dictionaryType === 'fingerprint_rule' || props.dictionaryType === 'poc_rule'
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
