<template>
  <div class="modal modal-open">
    <div class="modal-box max-w-4xl w-11/12 h-[78vh] p-0 flex flex-col overflow-hidden">
      <div class="p-4 border-b flex items-center justify-between bg-base-200/50">
        <h3 class="font-bold text-lg flex items-center gap-2">
          <i class="fas fa-file-signature text-primary"></i>
          {{ t('aiAssistant.forcedRulesTitle', '用户强制规则') }}
        </h3>
        <button class="btn btn-ghost btn-sm btn-circle" @click="$emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <div class="alert alert-info py-3">
          <i class="fas fa-circle-info"></i>
          <span>
            {{ t('aiAssistant.forcedRulesDescription', '该内容会作为 [User Forced Rules] 消息块拼接到 System Prompt。') }}
            <span class="opacity-70">({{ t('aiAssistant.forcedRulesConfigKey', '配置键') }}: `agent.user_forced_rules`)</span>
          </span>
        </div>

        <div v-if="loadError" class="alert alert-error py-3">
          <i class="fas fa-triangle-exclamation"></i>
          <span>{{ loadError }}</span>
        </div>

        <div class="form-control">
          <textarea
            v-model="rulesText"
            class="textarea textarea-bordered w-full min-h-[380px] font-mono text-sm leading-relaxed"
            :placeholder="t('aiAssistant.forcedRulesPlaceholder', '例如：\\n1. 先给结论再给步骤。\\n2. 回答必须包含可执行命令。\\n3. 若信息不足，先列出缺失项。')"
            :disabled="isLoading || isSaving"
          ></textarea>
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ t('aiAssistant.forcedRulesLengthHint', { max: maxRuleLength }) }}
            </span>
            <span class="label-text-alt" :class="remainingChars < 0 ? 'text-error' : 'text-base-content/60'">
              {{ t('aiAssistant.forcedRulesRemaining', { count: remainingChars }) }}
            </span>
          </label>
        </div>
      </div>

      <div class="p-4 border-t flex items-center justify-between bg-base-100">
        <button
          class="btn btn-ghost btn-sm"
          :disabled="isLoading || isSaving"
          @click="rulesText = ''"
        >
          <i class="fas fa-eraser mr-1"></i>
          {{ t('aiAssistant.clearForcedRules', '清空') }}
        </button>

        <div class="flex items-center gap-2">
          <button class="btn btn-ghost btn-sm" :disabled="isSaving" @click="$emit('close')">
            {{ t('common.cancel', '取消') }}
          </button>
          <button class="btn btn-primary btn-sm min-w-24" :disabled="!canSave" @click="handleSave">
            <span v-if="isSaving" class="loading loading-spinner loading-xs mr-1"></span>
            <i v-else class="fas fa-save mr-1"></i>
            {{ t('common.save', '保存') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

interface ConfigItem {
  key: string
  value: string
}

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'saved', value: string): void
}>()

const { t } = useI18n()
const maxRuleLength = 20000

const isLoading = ref(false)
const isSaving = ref(false)
const loadError = ref('')
const rulesText = ref('')

const remainingChars = computed(() => maxRuleLength - rulesText.value.length)
const canSave = computed(() =>
  !isLoading.value &&
  !isSaving.value &&
  rulesText.value.length <= maxRuleLength
)

const loadRules = async () => {
  isLoading.value = true
  loadError.value = ''
  try {
    const items = await invoke<ConfigItem[]>('get_config', {
      request: {
        category: 'agent',
        key: 'user_forced_rules',
      },
    })
    rulesText.value = Array.isArray(items) && items.length > 0 ? String(items[0].value || '') : ''
  } catch (error) {
    console.error('Failed to load user forced rules:', error)
    loadError.value = t('aiAssistant.loadForcedRulesFailed', '加载强制规则失败')
  } finally {
    isLoading.value = false
  }
}

const handleSave = async () => {
  if (!canSave.value) return
  isSaving.value = true
  try {
    await invoke('set_config', {
      category: 'agent',
      key: 'user_forced_rules',
      value: rulesText.value,
    })
    emit('saved', rulesText.value)
    emit('close')
  } catch (error) {
    console.error('Failed to save user forced rules:', error)
    loadError.value = t('aiAssistant.saveForcedRulesFailed', '保存强制规则失败')
  } finally {
    isSaving.value = false
  }
}

onMounted(() => {
  void loadRules()
})
</script>
