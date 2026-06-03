<template>
  <div class="tenth-man-settings space-y-4">
    <div class="form-control">
      <label class="label cursor-pointer">
        <span class="label-text">{{ t('settings.enableTenthMan') }}</span>
        <input 
          type="checkbox" 
          v-model="localSettings.enabled" 
          class="toggle toggle-primary"
          @change="saveSettings"
        />
      </label>
    </div>

    <div v-if="localSettings.enabled" class="space-y-4 pl-4 border-l-2 border-base-300">
      <div class="form-control">
        <label class="label">
          <span class="label-text">{{ t('settings.interventionMode') }}</span>
        </label>
        <select 
          v-model="localSettings.mode" 
          class="select select-bordered w-full"
          @change="saveSettings"
        >
          <option value="final_only">{{ t('settings.finalOnly') }}</option>
          <option value="proactive">{{ t('settings.proactive') }}</option>
        </select>
      </div>

      <div v-if="localSettings.mode === 'proactive'" class="space-y-3">
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.toolCallInterval') }}</span>
            <span class="label-text-alt">{{ t('settings.toolCallIntervalHint') }}</span>
          </label>
          <input 
            type="number" 
            v-model.number="localSettings.toolCallInterval" 
            min="1" 
            max="10"
            class="input input-bordered w-full"
            @change="saveSettings"
          />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.dangerousKeywords') }}</span>
            <span class="label-text-alt">{{ t('settings.dangerousKeywordsHint') }}</span>
          </label>
          <textarea 
            v-model="keywordsText"
            class="textarea textarea-bordered w-full"
            rows="3"
            placeholder="rm -rf, DROP TABLE, DELETE FROM"
            @blur="updateKeywords"
          ></textarea>
        </div>
      </div>

      <div class="form-control">
        <label class="label cursor-pointer">
          <span class="label-text">{{ t('settings.requireConfirmation') }}</span>
          <input 
            type="checkbox" 
            v-model="localSettings.requireConfirmation" 
            class="toggle toggle-warning"
            @change="saveSettings"
          />
        </label>
        <label class="label">
          <span class="label-text-alt">{{ t('settings.requireConfirmationHint') }}</span>
        </label>
      </div>
    </div>

    <div v-if="saveStatus" class="alert" :class="saveStatus.type === 'success' ? 'alert-success' : 'alert-error'">
      <span>{{ saveStatus.message }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()

interface TenthManSettings {
  enabled: boolean
  mode: 'final_only' | 'proactive'
  toolCallInterval: number
  dangerousKeywords: string[]
  requireConfirmation: boolean
}

const localSettings = ref<TenthManSettings>({
  enabled: false,
  mode: 'final_only',
  toolCallInterval: 3,
  dangerousKeywords: ['rm -rf', 'DROP TABLE', 'DELETE FROM', '格式化', '删除所有'],
  requireConfirmation: false,
})

const keywordsText = ref('')
const saveStatus = ref<{ type: 'success' | 'error', message: string } | null>(null)

const updateKeywords = () => {
  localSettings.value.dangerousKeywords = keywordsText.value
    .split(',')
    .map(k => k.trim())
    .filter(k => k.length > 0)
  saveSettings()
}

const loadSettings = async () => {
  try {
    const enabled = await invoke<string>('get_config', {
      category: 'agent',
      key: 'tenth_man_enabled'
    })
    localSettings.value.enabled = enabled === 'true'

    const mode = await invoke<string>('get_config', {
      category: 'agent',
      key: 'tenth_man_mode'
    })
    if (mode) {
      localSettings.value.mode = mode as 'final_only' | 'proactive'
    }

    const interval = await invoke<string>('get_config', {
      category: 'agent',
      key: 'tenth_man_tool_interval'
    })
    if (interval) {
      localSettings.value.toolCallInterval = parseInt(interval)
    }

    const keywords = await invoke<string>('get_config', {
      category: 'agent',
      key: 'tenth_man_dangerous_keywords'
    })
    if (keywords) {
      localSettings.value.dangerousKeywords = JSON.parse(keywords)
      keywordsText.value = localSettings.value.dangerousKeywords.join(', ')
    }

    const requireConfirm = await invoke<string>('get_config', {
      category: 'agent',
      key: 'tenth_man_require_confirmation'
    })
    localSettings.value.requireConfirmation = requireConfirm === 'true'
  } catch (error) {
    console.error('Failed to load tenth man settings:', error)
  }
}

const saveSettings = async () => {
  try {
    await invoke('set_config', {
      category: 'agent',
      key: 'tenth_man_enabled',
      value: localSettings.value.enabled.toString()
    })

    await invoke('set_config', {
      category: 'agent',
      key: 'tenth_man_mode',
      value: localSettings.value.mode
    })

    await invoke('set_config', {
      category: 'agent',
      key: 'tenth_man_tool_interval',
      value: localSettings.value.toolCallInterval.toString()
    })

    await invoke('set_config', {
      category: 'agent',
      key: 'tenth_man_dangerous_keywords',
      value: JSON.stringify(localSettings.value.dangerousKeywords)
    })

    await invoke('set_config', {
      category: 'agent',
      key: 'tenth_man_require_confirmation',
      value: localSettings.value.requireConfirmation.toString()
    })

    saveStatus.value = { type: 'success', message: t('settings.saveSuccess') }
    setTimeout(() => {
      saveStatus.value = null
    }, 3000)
  } catch (error) {
    console.error('Failed to save tenth man settings:', error)
    saveStatus.value = { type: 'error', message: t('settings.saveFailed') }
  }
}

onMounted(() => {
  loadSettings()
  keywordsText.value = localSettings.value.dangerousKeywords.join(', ')
})
</script>

<style scoped>
.tenth-man-settings {
  max-width: 600px;
}
</style>
