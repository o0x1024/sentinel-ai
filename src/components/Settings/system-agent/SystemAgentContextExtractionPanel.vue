<template>
  <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-4">
    <div class="flex items-start justify-between gap-3">
      <div>
        <div class="text-sm font-semibold">上下文抽取词典</div>
        <div class="text-xs text-base-content/60 mt-1">
          给逻辑漏洞检测补充自定义身份字段、资源主键、凭证字段和动作别名。
        </div>
      </div>
      <div class="flex items-center gap-2">
        <label class="form-control">
          <select
            class="select select-bordered select-sm min-w-24"
            :value="recentHistoryLimit"
            :disabled="saving || recommending"
            @change="updateRecentHistoryLimit"
          >
            <option
              v-for="option in recentHistoryLimitOptions"
              :key="option"
              :value="option"
            >
              最近 {{ option }} 条
            </option>
          </select>
        </label>
        <button class="btn btn-sm btn-ghost" :disabled="saving || recommending" @click="emitRecommend">
          {{ recommending ? '分析中' : '从最近历史推荐' }}
        </button>
        <button class="btn btn-sm btn-primary" :disabled="saving" @click="emitSave">
          {{ saving ? '保存中' : '保存设置' }}
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-2 gap-4">
      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">主体字段</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="principalKeysText"
          placeholder="operatorCode&#10;memberNo&#10;staffLevel"
          @input="updateListField('principalKeys', $event)"
        />
      </label>

      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">资源主键字段</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="resourceKeyHintsText"
          placeholder="caseRef&#10;bizNo&#10;ledgerKey"
          @input="updateListField('resourceKeyHints', $event)"
        />
      </label>

      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">认证头</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="authHeaderKeysText"
          placeholder="x-tenant-token&#10;x-member-auth"
          @input="updateListField('authHeaderKeys', $event)"
        />
      </label>

      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">Token 参数</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="authTokenKeysText"
          placeholder="tenantToken&#10;accessKey"
          @input="updateListField('authTokenKeys', $event)"
        />
      </label>

      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">Cookie 提示词</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="cookieHintKeysText"
          placeholder="member&#10;tenant&#10;auth"
          @input="updateListField('cookieHintKeys', $event)"
        />
      </label>

      <label class="form-control">
        <div class="label">
          <span class="label-text text-xs font-medium">动作别名</span>
        </div>
        <textarea
          class="textarea textarea-bordered min-h-28 font-mono text-xs"
          :value="actionAliasesText"
          placeholder="complete=finalize,writeoff&#10;grant=issueQuota,grantQuota"
          @input="updateActionAliases($event)"
        />
      </label>
    </div>

    <div
      class="rounded-lg bg-base-200/60 border border-base-300 p-3 text-xs text-base-content/70 space-y-1"
    >
      <div>每行一个值，留空表示只使用系统默认词典。</div>
      <div>动作别名格式：`动作=别名1,别名2`。保存后会自动去重和规范化。</div>
      <div>命中结果会出现在 triage payload 的 `contextExtraction` 字段里，便于调试。</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TrafficContextExtractionSettings } from '@/components/traffic/proxyConfigurationTypes'
import { computed } from 'vue'

const props = defineProps<{
  settings: TrafficContextExtractionSettings
  saving?: boolean
  recommending?: boolean
  recentHistoryLimit: number
  recentHistoryLimitOptions: number[]
}>()

const emit = defineEmits<{
  (event: 'update:settings', value: TrafficContextExtractionSettings): void
  (event: 'update:recentHistoryLimit', value: number): void
  (event: 'save'): void
  (event: 'recommend-from-history'): void
}>()

const principalKeysText = computed(() => props.settings.principalKeys.join('\n'))
const resourceKeyHintsText = computed(() => props.settings.resourceKeyHints.join('\n'))
const authHeaderKeysText = computed(() => props.settings.authHeaderKeys.join('\n'))
const authTokenKeysText = computed(() => props.settings.authTokenKeys.join('\n'))
const cookieHintKeysText = computed(() => props.settings.cookieHintKeys.join('\n'))
const actionAliasesText = computed(() =>
  Object.entries(props.settings.actionAliases)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([action, aliases]) => `${action}=${aliases.join(',')}`)
    .join('\n')
)

function emitSave() {
  emit('save')
}

function emitRecommend() {
  emit('recommend-from-history')
}

function updateRecentHistoryLimit(event: Event) {
  const target = event.target as HTMLSelectElement
  const nextValue = Number.parseInt(target.value, 10)
  if (Number.isFinite(nextValue) && nextValue > 0) {
    emit('update:recentHistoryLimit', nextValue)
  }
}

function updateListField(
  field:
    | 'principalKeys'
    | 'resourceKeyHints'
    | 'authHeaderKeys'
    | 'authTokenKeys'
    | 'cookieHintKeys',
  event: Event
) {
  const target = event.target as HTMLTextAreaElement
  emit('update:settings', {
    ...props.settings,
    [field]: parseLineList(target.value),
  })
}

function updateActionAliases(event: Event) {
  const target = event.target as HTMLTextAreaElement
  emit('update:settings', {
    ...props.settings,
    actionAliases: parseActionAliases(target.value),
  })
}

function parseLineList(raw: string) {
  return raw
    .split('\n')
    .map(item => item.trim())
    .filter(Boolean)
}

function parseActionAliases(raw: string) {
  const entries = raw
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
  const next: Record<string, string[]> = {}
  for (const entry of entries) {
    const [actionPart, aliasPart = ''] = entry.split('=')
    const action = actionPart?.trim()
    if (!action) continue
    const aliases = aliasPart
      .split(',')
      .map(item => item.trim())
      .filter(Boolean)
    if (aliases.length > 0) {
      next[action] = aliases
    }
  }
  return next
}
</script>
