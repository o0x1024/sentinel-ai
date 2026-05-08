<template>
  <section class="rounded-lg border border-base-300 bg-base-100 overflow-hidden">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <div class="text-sm font-semibold">{{ t('settings.network.weixin.title') }}</div>
          <div class="mt-1 text-xs text-base-content/70">{{ t('settings.network.weixin.description') }}</div>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <div class="rounded-lg border border-base-300 px-3 py-2 text-xs">
            <div>
              {{ t('settings.network.weixin.status') }}:
              <span :class="weixinStatus.running ? 'text-success' : 'text-error'">
                {{ weixinStatus.running ? t('settings.enabled') : t('settings.disabled') }}
              </span>
            </div>
            <div v-if="weixinStatus.account_id" class="break-all">{{ weixinStatus.account_id }}</div>
            <div v-if="weixinStatus.last_message_at" class="text-base-content/60">
              {{ t('settings.network.weixin.lastMessageAt') }}: {{ weixinStatus.last_message_at }}
            </div>
          </div>
          <input
            v-model="weixinConfig.enabled"
            type="checkbox"
            class="toggle toggle-primary"
            :disabled="weixinBusy"
            @change="handleWeixinToggle"
          />
        </div>
      </div>
    </div>

    <div class="p-4 space-y-4">
      <div class="alert alert-warning text-sm">
        <span>{{ t('settings.network.weixin.securityHint') }}</span>
      </div>

      <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.accountId') }}</span></label>
          <input v-model.trim="weixinConfig.account_id" class="input input-bordered w-full" :disabled="weixinBusy" />
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.token') }}</span></label>
          <input v-model.trim="weixinConfig.token" class="input input-bordered w-full" type="password" :disabled="weixinBusy" />
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.baseUrl') }}</span></label>
          <input v-model.trim="weixinConfig.base_url" class="input input-bordered w-full" :disabled="weixinBusy" />
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.assistantProfile') }}</span></label>
          <select v-model="weixinConfig.assistant_profile_id" class="select select-bordered w-full" :disabled="weixinBusy">
            <option :value="null">{{ t('settings.network.weixin.defaultAssistantProfile') }}</option>
            <option v-for="profile in weixinAssistantProfiles" :key="profile.id" :value="profile.id">
              {{ profile.label }}
            </option>
          </select>
          <div class="mt-1 text-xs text-base-content/60">{{ t('settings.network.weixin.assistantProfileHint') }}</div>
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.allowedUsers') }}</span></label>
          <input
            v-model.trim="weixinAllowedUsersText"
            class="input input-bordered w-full"
            :disabled="weixinBusy"
            placeholder="user_id_1,user_id_2"
          />
          <div class="mt-1 text-xs text-base-content/60">{{ t('settings.network.weixin.allowedUsersHint') }}</div>
        </div>
        <div class="flex items-end">
          <label class="label cursor-pointer gap-3">
            <input
              v-model="weixinGroupEnabled"
              type="checkbox"
              class="checkbox checkbox-primary"
              :disabled="weixinBusy"
            />
            <span class="label-text">{{ t('settings.network.weixin.enableGroups') }}</span>
          </label>
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.maxIterations') }}</span></label>
          <input
            v-model.number="weixinConfig.max_iterations"
            class="input input-bordered w-full"
            type="number"
            :disabled="weixinBusy"
          />
        </div>
        <div>
          <label class="label"><span class="label-text">{{ t('settings.network.weixin.timeoutSecs') }}</span></label>
          <input
            v-model.number="weixinConfig.timeout_secs"
            class="input input-bordered w-full"
            type="number"
            :disabled="weixinBusy"
          />
        </div>
      </div>

      <div class="grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-4">
        <button class="btn btn-primary" :disabled="weixinBusy" @click="saveWeixinOnly">
          <span v-if="weixinBusy" class="loading loading-spinner loading-xs"></span>
          {{ t('settings.network.weixin.save') }}
        </button>
        <button class="btn btn-accent" :disabled="weixinBusy" @click="startWeixinQrLoginFlow">
          {{ t('settings.network.weixin.qrLogin') }}
        </button>
        <button class="btn btn-secondary" :disabled="weixinBusy" @click="handleRefreshWeixinStatus">
          {{ t('settings.network.weixin.refreshStatus') }}
        </button>
      </div>

      <div v-if="weixinQr.scan_data" class="rounded-xl border border-base-300 bg-base-200/30 p-4">
        <div class="mb-2 font-semibold">{{ t('settings.network.weixin.qrPending') }}</div>
        <img
          v-if="weixinQrImageSrc"
          :src="weixinQrImageSrc"
          class="h-48 w-48 rounded bg-white p-2 object-contain"
          alt="Weixin QR"
        />
        <div class="mt-2 break-all text-xs">{{ weixinQr.scan_data }}</div>
        <div class="mt-2 text-xs text-base-content/60">{{ weixinQrStatusText }}</div>
      </div>

      <div v-if="weixinStatus.last_error" class="alert alert-error">
        <span>{{ weixinStatus.last_error }}</span>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import { createQrCodeSvgDataUrl } from '@/services/qrCode'
import {
  createWeixinQrLogin,
  getWeixinGatewayConfig,
  getWeixinGatewayStatus,
  pollWeixinQrLogin,
  saveWeixinGatewayConfig,
  startWeixinGateway,
  stopWeixinGateway,
  type WeixinGatewayConfig,
  type WeixinGatewayStatus,
  type WeixinQrLoginResponse,
} from '@/api/weixinGateway'
import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'

const emit = defineEmits<{
  updated: [payload: { accountId: string }]
}>()

const { t } = useI18n()

const weixinConfig = reactive<WeixinGatewayConfig>({
  enabled: false,
  account_id: '',
  token: '',
  base_url: 'https://ilinkai.weixin.qq.com',
  assistant_profile_id: null,
  dm_policy: 'open',
  allowed_users: [],
  group_policy: 'open',
  group_allowed_users: [],
  default_service_name: 'default',
  max_iterations: 50,
  timeout_secs: 300,
})

const weixinStatus = reactive<WeixinGatewayStatus>({
  running: false,
  account_id: null,
  started_at: null,
  last_error: null,
  last_message_at: null,
})

const weixinQr = reactive<WeixinQrLoginResponse>({
  qrcode: '',
  qrcode_img_content: '',
  scan_data: '',
})

const weixinAllowedUsersText = ref('')
const weixinAssistantProfiles = ref<AssistantProfileOption[]>([])
const weixinBusy = ref(false)
const weixinQrStatusText = ref('')
let weixinQrTimer: number | null = null

const weixinQrImageSrc = computed(() => {
  const imageContent = weixinQr.qrcode_img_content.trim()
  if (imageContent.startsWith('data:image')) return imageContent
  if (looksLikeBase64Image(imageContent)) return `data:image/png;base64,${imageContent}`

  const scanContent = (weixinQr.scan_data || weixinQr.qrcode_img_content || weixinQr.qrcode).trim()
  if (!scanContent) return ''
  try {
    return createQrCodeSvgDataUrl(scanContent)
  } catch (error) {
    console.error('Failed to render Weixin QR code:', error)
    return ''
  }
})

const weixinGroupEnabled = computed({
  get: () => weixinConfig.group_policy !== 'disabled',
  set: (enabled: boolean) => {
    weixinConfig.group_policy = enabled ? 'open' : 'disabled'
  },
})

function looksLikeBase64Image(value: string) {
  if (value.length < 120 || value.startsWith('http')) return false
  return /^[A-Za-z0-9+/=\s]+$/.test(value)
}

function syncWeixinAllowedUsersFromText() {
  weixinConfig.allowed_users = weixinAllowedUsersText.value
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
  weixinConfig.dm_policy = weixinConfig.allowed_users.length > 0 ? 'allowlist' : 'open'
}

async function loadWeixinConfig() {
  try {
    const cfg = await getWeixinGatewayConfig()
    Object.assign(weixinConfig, cfg)
    weixinAllowedUsersText.value = cfg.allowed_users.join(',')
  } catch (error) {
    console.error('Failed to load Weixin config:', error)
  }
}

async function loadWeixinAssistantProfiles() {
  try {
    const profiles = await invoke<AssistantProfileOption[]>('list_assistant_profiles')
    weixinAssistantProfiles.value = profiles.filter((profile) => profile.runMode !== 'team')
  } catch (error) {
    console.error('Failed to load Weixin assistant profiles:', error)
    weixinAssistantProfiles.value = []
  }
}

async function refreshWeixinStatus(emitUpdate = false) {
  try {
    const status = await getWeixinGatewayStatus()
    Object.assign(weixinStatus, status)
    if (emitUpdate) {
      emit('updated', { accountId: weixinConfig.account_id.trim() })
    }
  } catch (error) {
    console.error('Failed to load Weixin status:', error)
  }
}

async function handleRefreshWeixinStatus() {
  await refreshWeixinStatus(true)
}

async function saveWeixinOnly() {
  weixinBusy.value = true
  try {
    syncWeixinAllowedUsersFromText()
    await saveWeixinGatewayConfig({ ...weixinConfig })
    dialog.toast.success(t('settings.network.weixin.saved'))
    await refreshWeixinStatus(true)
  } catch (error) {
    dialog.toast.error(t('settings.network.weixin.saveFailed', { error: String(error) }))
  } finally {
    weixinBusy.value = false
  }
}

async function handleWeixinToggle() {
  weixinBusy.value = true
  try {
    syncWeixinAllowedUsersFromText()
    await saveWeixinGatewayConfig({ ...weixinConfig })
    if (weixinConfig.enabled) {
      await startWeixinGateway({ ...weixinConfig })
      dialog.toast.success(t('settings.network.weixin.started'))
    } else {
      await stopWeixinGateway()
      dialog.toast.success(t('settings.network.weixin.stopped'))
    }
    await refreshWeixinStatus(true)
  } catch (error) {
    weixinConfig.enabled = !weixinConfig.enabled
    dialog.toast.error(t('settings.network.weixin.toggleFailed', { error: String(error) }))
  } finally {
    weixinBusy.value = false
  }
}

function clearWeixinQrTimer() {
  if (weixinQrTimer !== null) {
    window.clearInterval(weixinQrTimer)
    weixinQrTimer = null
  }
}

async function startWeixinQrLoginFlow() {
  weixinBusy.value = true
  clearWeixinQrTimer()
  try {
    const qr = await createWeixinQrLogin()
    Object.assign(weixinQr, qr)
    weixinQrStatusText.value = t('settings.network.weixin.qrWaiting')
    weixinQrTimer = window.setInterval(async () => {
      try {
        const status = await pollWeixinQrLogin(weixinQr.qrcode)
        weixinQrStatusText.value = status.message || status.status
        if (status.status === 'confirmed') {
          clearWeixinQrTimer()
          Object.assign(weixinQr, { qrcode: '', qrcode_img_content: '', scan_data: '' })
          await loadWeixinConfig()
          await refreshWeixinStatus(true)
          dialog.toast.success(t('settings.network.weixin.qrConfirmed'))
        }
      } catch (error) {
        weixinQrStatusText.value = String(error)
      }
    }, 1500)
  } catch (error) {
    dialog.toast.error(t('settings.network.weixin.qrFailed', { error: String(error) }))
  } finally {
    weixinBusy.value = false
  }
}

onMounted(() => {
  loadWeixinConfig()
  loadWeixinAssistantProfiles()
  refreshWeixinStatus()
})

onUnmounted(() => {
  clearWeixinQrTimer()
})
</script>
