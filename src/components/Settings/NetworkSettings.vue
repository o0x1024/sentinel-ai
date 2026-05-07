<template>
    <!-- 网络(代理)设置 -->
    <div  class="card bg-base-100 shadow-md mb-6">
        <div class="card-body gap-4">
            <div class="flex items-start justify-between gap-4">
                <div>
                    <div class="font-semibold">{{ t('settings.network.globalProxyTitle') }}</div>
                    <div class="text-sm text-base-content/70 mt-1">{{ t('settings.network.globalProxyDescription') }}</div>
                </div>
                <div class="flex items-center gap-3 shrink-0">
                    <input type="checkbox" class="toggle toggle-primary" v-model="network.proxy.enabled" :disabled="props.saving"
                        @change="saveProxy" />
                    <span class="font-medium">{{ t('settings.network.enableGlobalProxy') }}</span>
                </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                    <label class="label">
                        <span class="label-text">{{ t('settings.network.scheme') }}</span>
                    </label>
                    <select v-model="network.proxy.scheme" class="select select-bordered w-full" :disabled="props.saving" @change="saveProxy">
                        <option value="http">{{ t('settings.network.proxySchemes.http') }}</option>
                        <option value="https">{{ t('settings.network.proxySchemes.https') }}</option>
                        <option value="socks5">{{ t('settings.network.proxySchemes.socks5') }}</option>
                        <option value="socks5h">{{ t('settings.network.proxySchemes.socks5h') }}</option>
                    </select>
                    <label class="label">
                        <span class="label-text-alt text-gray-500">
                            {{ t('settings.network.proxySchemeHint') }}
                        </span>
                    </label>
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.host') }}</span></label>
                    <input v-model.trim="network.proxy.host" class="input input-bordered w-full" :disabled="props.saving" :placeholder="t('settings.network.placeholders.host')"
                        @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.port') }}</span></label>
                    <input v-model.number="network.proxy.port" class="input input-bordered w-full" type="number"
                        :disabled="props.saving" :placeholder="t('settings.network.placeholders.port')" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.noProxy') }}</span></label>
                    <input v-model.trim="network.proxy.no_proxy" class="input input-bordered w-full"
                        :disabled="props.saving" :placeholder="t('settings.network.placeholders.noProxy')" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.username') }}</span></label>
                    <input v-model.trim="network.proxy.username" class="input input-bordered w-full"
                        :disabled="props.saving" @blur="saveProxy" />
                </div>
                <div>
                    <label class="label"><span class="label-text">{{ t('settings.network.password') }}</span></label>
                    <input v-model.trim="network.proxy.password" class="input input-bordered w-full" type="password"
                        :disabled="props.saving" @blur="saveProxy" />
                </div>
            </div>
        </div>
    </div>

    <div class="card bg-base-100 shadow-md mb-6">
      <div class="card-body gap-4">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="card-title">{{ t('settings.network.runtime.title') }}</h2>
            <p class="text-sm text-base-content/70">{{ t('settings.network.runtime.description') }}</p>
          </div>
          <button class="btn btn-outline btn-sm" :disabled="runtimeProxyBusy" @click="refreshRuntimeProxyStatus">
            <span v-if="runtimeProxyBusy" class="loading loading-spinner loading-xs"></span>
            {{ t('settings.network.runtime.refresh') }}
          </button>
        </div>

        <div class="alert alert-warning text-sm">
          <span>{{ t('settings.network.runtime.confusionHint') }}</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="rounded-xl border border-base-300 p-4 bg-base-200/30">
            <div class="flex items-center justify-between gap-3">
              <div class="font-medium">{{ t('settings.network.runtime.globalProxy') }}</div>
              <div class="badge" :class="network.proxy.enabled ? 'badge-success' : 'badge-ghost'">
                {{ network.proxy.enabled ? t('settings.network.runtime.active') : t('settings.network.runtime.inactive') }}
              </div>
            </div>
            <div class="text-sm text-base-content/70 mt-3">
              {{ network.proxy.enabled ? t('settings.network.runtime.globalProxyActiveDetail') : t('settings.network.runtime.globalProxyInactiveDetail') }}
            </div>
            <div v-if="network.proxy.enabled && globalProxyEndpoint" class="text-xs text-base-content/60 mt-2 break-all">
              {{ globalProxyEndpoint }}
            </div>
          </div>

          <div class="rounded-xl border border-base-300 p-4 bg-base-200/30">
            <div class="flex items-center justify-between gap-3">
              <div class="font-medium">{{ t('settings.network.runtime.localProxy') }}</div>
              <div class="badge" :class="runtimeProxyStatus.running ? 'badge-success' : 'badge-ghost'">
                {{ runtimeProxyStatus.running ? t('settings.network.runtime.active') : t('settings.network.runtime.inactive') }}
              </div>
            </div>
            <div class="text-sm text-base-content/70 mt-3">
              {{ runtimeProxyStatus.running
                ? t('settings.network.runtime.localProxyActiveDetail', { port: runtimeProxyStatus.port || 0 })
                : t('settings.network.runtime.localProxyInactiveDetail') }}
            </div>
            <div class="text-xs text-base-content/60 mt-2">
              {{ runtimeProxyAutoStart
                ? t('settings.network.runtime.localProxyAutoStartOn')
                : t('settings.network.runtime.localProxyAutoStartOff') }}
            </div>
          </div>

        </div>
      </div>
    </div>

    <div class="card bg-base-100 shadow-md mb-6">
      <div class="card-body gap-4">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="card-title">{{ t('settings.network.gateway.title') }}</h2>
            <p class="text-sm text-base-content/70">{{ t('settings.network.gateway.description') }}</p>
          </div>
          <input
            type="checkbox"
            class="toggle toggle-primary"
            v-model="gatewayConfig.enabled"
            :disabled="props.saving || gatewayBusy"
            @change="handleGatewayToggle"
          />
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.gateway.host') }}</span></label>
            <input
              v-model.trim="gatewayConfig.host"
              class="input input-bordered w-full"
              :disabled="props.saving || gatewayBusy"
              placeholder="127.0.0.1"
            />
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.gateway.port') }}</span></label>
            <input
              v-model.number="gatewayConfig.port"
              class="input input-bordered w-full"
              type="number"
              :disabled="props.saving || gatewayBusy"
              placeholder="18765"
            />
          </div>
          <div class="flex items-end">
            <label class="label cursor-pointer gap-3">
              <input
                type="checkbox"
                class="checkbox checkbox-primary"
                v-model="gatewayConfig.allow_lan"
                :disabled="props.saving || gatewayBusy"
              />
              <span class="label-text">{{ t('settings.network.gateway.allowLan') }}</span>
            </label>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
          <button class="btn btn-primary" :disabled="gatewayBusy || props.saving" @click="saveGatewayOnly">
            <span v-if="gatewayBusy" class="loading loading-spinner loading-xs"></span>
            {{ t('settings.network.gateway.save') }}
          </button>
          <button class="btn btn-accent" :disabled="gatewayBusy || props.saving" @click="generateGatewayApiKey">
            {{ t('settings.network.gateway.generateApiKey') }}
          </button>
          <button class="btn btn-secondary" :disabled="gatewayBusy || props.saving" @click="refreshGatewayStatus">
            {{ t('settings.network.gateway.refreshStatus') }}
          </button>
          <div class="rounded-lg border border-base-300 px-3 py-2 text-sm">
            <div>
              {{ t('settings.network.gateway.status') }}:
              <span :class="gatewayStatus.running ? 'text-success' : 'text-error'">
                {{ gatewayStatus.running ? t('settings.enabled') : t('settings.disabled') }}
              </span>
            </div>
            <div v-if="gatewayDisplayBindAddr">{{ gatewayDisplayBindAddr }}</div>
          </div>
        </div>

        <div v-if="gatewayStatus.last_error" class="alert alert-error">
          <span>{{ gatewayStatus.last_error }}</span>
        </div>
        <div v-if="lastGeneratedGatewayKey" class="alert alert-warning">
          <div>
            <div class="font-semibold">{{ t('settings.network.gateway.generatedKeyNotice') }}</div>
            <div class="text-xs mt-1 break-all">{{ lastGeneratedGatewayKey }}</div>
          </div>
        </div>

        <div v-if="isGatewayWebMode" class="rounded-xl border border-base-300 p-4 bg-base-200/30">
          <div class="font-semibold mb-1">浏览器网关 API Key</div>
          <div class="text-xs text-base-content/70 mb-3">
            仅用于当前浏览器访问 HTTP 网关，保存在 localStorage（key: `sentinel:http-gateway:api-key`）。
          </div>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div class="md:col-span-2">
              <input
                v-model.trim="browserGatewayKey"
                class="input input-bordered w-full"
                type="password"
                placeholder="输入网关 API Key"
              />
              <div class="text-xs text-base-content/60 mt-1">
                当前状态: {{ hasBrowserGatewayKey ? '已设置' : '未设置' }}
              </div>
            </div>
            <div class="flex items-end gap-2">
              <button class="btn btn-primary btn-sm flex-1" @click="saveBrowserGatewayKey">
                保存
              </button>
              <button class="btn btn-outline btn-sm" @click="clearBrowserGatewayKey">
                清除
              </button>
              <button class="btn btn-ghost btn-sm" :disabled="!hasBrowserGatewayKey" @click="copyBrowserGatewayKey">
                复制
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 shadow-md mb-6">
      <div class="card-body gap-4">
        <div class="flex items-center justify-between gap-4">
          <div>
            <h2 class="card-title">{{ t('settings.network.weixin.title') }}</h2>
            <p class="text-sm text-base-content/70">{{ t('settings.network.weixin.description') }}</p>
          </div>
          <input
            type="checkbox"
            class="toggle toggle-primary"
            v-model="weixinConfig.enabled"
            :disabled="props.saving || weixinBusy"
            @change="handleWeixinToggle"
          />
        </div>

        <div class="alert alert-warning text-sm">
          <span>{{ t('settings.network.weixin.securityHint') }}</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.accountId') }}</span></label>
            <input v-model.trim="weixinConfig.account_id" class="input input-bordered w-full" :disabled="props.saving || weixinBusy" />
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.token') }}</span></label>
            <input v-model.trim="weixinConfig.token" class="input input-bordered w-full" type="password" :disabled="props.saving || weixinBusy" />
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.baseUrl') }}</span></label>
            <input v-model.trim="weixinConfig.base_url" class="input input-bordered w-full" :disabled="props.saving || weixinBusy" />
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.assistantProfile') }}</span></label>
            <select v-model="weixinConfig.assistant_profile_id" class="select select-bordered w-full" :disabled="props.saving || weixinBusy">
              <option :value="null">{{ t('settings.network.weixin.defaultAssistantProfile') }}</option>
              <option v-for="profile in weixinAssistantProfiles" :key="profile.id" :value="profile.id">
                {{ profile.label }}
              </option>
            </select>
            <div class="text-xs text-base-content/60 mt-1">{{ t('settings.network.weixin.assistantProfileHint') }}</div>
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.allowedUsers') }}</span></label>
            <input
              v-model.trim="weixinAllowedUsersText"
              class="input input-bordered w-full"
              :disabled="props.saving || weixinBusy"
              placeholder="user_id_1,user_id_2"
            />
            <div class="text-xs text-base-content/60 mt-1">{{ t('settings.network.weixin.allowedUsersHint') }}</div>
          </div>
          <div class="flex items-end">
            <label class="label cursor-pointer gap-3">
              <input
                type="checkbox"
                class="checkbox checkbox-primary"
                v-model="weixinGroupEnabled"
                :disabled="props.saving || weixinBusy"
              />
              <span class="label-text">{{ t('settings.network.weixin.enableGroups') }}</span>
            </label>
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.maxIterations') }}</span></label>
            <input v-model.number="weixinConfig.max_iterations" class="input input-bordered w-full" type="number" :disabled="props.saving || weixinBusy" />
          </div>
          <div>
            <label class="label"><span class="label-text">{{ t('settings.network.weixin.timeoutSecs') }}</span></label>
            <input v-model.number="weixinConfig.timeout_secs" class="input input-bordered w-full" type="number" :disabled="props.saving || weixinBusy" />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
          <button class="btn btn-primary" :disabled="weixinBusy || props.saving" @click="saveWeixinOnly">
            <span v-if="weixinBusy" class="loading loading-spinner loading-xs"></span>
            {{ t('settings.network.weixin.save') }}
          </button>
          <button class="btn btn-accent" :disabled="weixinBusy || props.saving" @click="startWeixinQrLoginFlow">
            {{ t('settings.network.weixin.qrLogin') }}
          </button>
          <button class="btn btn-secondary" :disabled="weixinBusy || props.saving" @click="refreshWeixinStatus">
            {{ t('settings.network.weixin.refreshStatus') }}
          </button>
          <div class="rounded-lg border border-base-300 px-3 py-2 text-sm">
            <div>
              {{ t('settings.network.weixin.status') }}:
              <span :class="weixinStatus.running ? 'text-success' : 'text-error'">
                {{ weixinStatus.running ? t('settings.enabled') : t('settings.disabled') }}
              </span>
            </div>
            <div v-if="weixinStatus.account_id" class="break-all">{{ weixinStatus.account_id }}</div>
            <div v-if="weixinStatus.last_message_at" class="text-xs text-base-content/60">
              {{ t('settings.network.weixin.lastMessageAt') }}: {{ weixinStatus.last_message_at }}
            </div>
          </div>
        </div>

        <div v-if="weixinQr.scan_data" class="rounded-xl border border-base-300 p-4 bg-base-200/30">
          <div class="font-semibold mb-2">{{ t('settings.network.weixin.qrPending') }}</div>
          <img
            v-if="weixinQrImageSrc"
            :src="weixinQrImageSrc"
            class="h-48 w-48 object-contain rounded bg-white p-2"
            alt="Weixin QR"
          />
          <div class="text-xs break-all mt-2">{{ weixinQr.scan_data }}</div>
          <div class="text-xs text-base-content/60 mt-2">{{ weixinQrStatusText }}</div>
        </div>

        <div v-if="weixinStatus.last_error" class="alert alert-error">
          <span>{{ weixinStatus.last_error }}</span>
        </div>
      </div>
    </div>

</template>


<script setup lang="ts">
import {
  getHttpGatewayConfig,
  getHttpGatewayStatus,
  rotateHttpGatewayApiKey,
  saveHttpGatewayConfig,
  startHttpGateway,
  stopHttpGateway,
  type HttpGatewayConfig,
  type HttpGatewayStatus,
} from '@/api/httpGateway'
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
import { dialog } from '@/composables/useDialog';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n'
import { createQrCodeSvgDataUrl } from '@/services/qrCode'
import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'


const { t } = useI18n()


const props = defineProps({
  saving: {
    type: Boolean,
    default: false
  }
})


const network = reactive({ proxy: { enabled: false, scheme: 'http', host: '', port: 0, username: '', password: '', no_proxy: '' } })
const runtimeProxyBusy = ref(false)
const runtimeProxyStatus = reactive({
  running: false,
  port: 0,
})
const runtimeProxyAutoStart = ref(false)
let unlistenProxyStatus: null | (() => void) = null

const gatewayConfig = reactive<HttpGatewayConfig>({
  enabled: false,
  host: '127.0.0.1',
  port: 18765,
  allow_lan: false,
  cors: { enabled: false, origins: [] },
  auth: { required: false, api_keys: [], header_name: 'X-API-Key' },
  remote: { enabled: false, mode: 'reverse_proxy', public_base_url: '' },
  limits: { max_body_bytes: 1024 * 1024, requests_per_minute: 600, max_concurrent_requests: 32 },
  audit: { enabled: true, log_auth_failures: true },
})

const gatewayStatus = reactive<HttpGatewayStatus>({
  running: false,
  bind_addr: null,
  started_at: null,
  last_error: null,
})
const gatewayBusy = ref(false)
const lastGeneratedGatewayKey = ref('')
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
const BROWSER_GATEWAY_KEY_STORAGE = 'sentinel:http-gateway:api-key'
const isGatewayWebMode = ref(false)
const browserGatewayKey = ref('')
const ethernetIpv4 = ref('')
const hasBrowserGatewayKey = computed(() => !!browserGatewayKey.value.trim())
const globalProxyEndpoint = computed(() => {
  if (!network.proxy.enabled || !network.proxy.host || !network.proxy.port) return ''
  return `${network.proxy.scheme || 'http'}://${network.proxy.host}:${network.proxy.port}`
})
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

const looksLikeBase64Image = (value: string) => {
  if (value.length < 120 || value.startsWith('http')) return false
  return /^[A-Za-z0-9+/=\s]+$/.test(value)
}

interface NetworkInterface {
  name: string
  description?: string
  mac?: string
  ipv4?: string
}

const selectEthernetIpv4 = (interfaces: NetworkInterface[]) => {
  const withIpv4 = interfaces.filter((iface) => iface.ipv4 && iface.ipv4 !== '127.0.0.1')
  if (withIpv4.length === 0) return ''

  const isWireless = (iface: NetworkInterface) => {
    const text = `${iface.name} ${iface.description ?? ''}`.toLowerCase()
    return /wi-?fi|wireless|wlan/.test(text)
  }

  const preferred = withIpv4.find((iface) => {
    const text = `${iface.name} ${iface.description ?? ''}`.toLowerCase()
    return /ethernet|以太网|^eth\d*|^enp\d+|^ens\d+|^eno\d+|^enx/.test(text)
  })
  if (preferred?.ipv4) return preferred.ipv4

  const nonWireless = withIpv4.find((iface) => !isWireless(iface))
  if (nonWireless?.ipv4) return nonWireless.ipv4

  return withIpv4[0].ipv4 ?? ''
}

const gatewayDisplayBindAddr = computed(() => {
  const bindAddr = gatewayStatus.bind_addr
  if (!bindAddr) return ''
  if (!gatewayConfig.allow_lan || !ethernetIpv4.value) return bindAddr

  const idx = bindAddr.lastIndexOf(':')
  if (idx <= 0 || idx >= bindAddr.length - 1) return bindAddr

  return `${ethernetIpv4.value}:${bindAddr.slice(idx + 1)}`
})


const loadGatewayConfig = async () => {
  try {
    const cfg = await getHttpGatewayConfig()
    Object.assign(gatewayConfig, cfg)
  } catch (error) {
    console.error('Failed to load gateway config:', error)
  }
}

const refreshGatewayStatus = async () => {
  try {
    const status = await getHttpGatewayStatus()
    Object.assign(gatewayStatus, status)
  } catch (error) {
    console.error('Failed to load gateway status:', error)
  }
}

const loadEthernetIpv4 = async () => {
  try {
    const interfaces = await invoke<NetworkInterface[]>('get_network_interfaces')
    ethernetIpv4.value = selectEthernetIpv4(interfaces)
  } catch (error) {
    console.error('Failed to load network interfaces:', error)
    ethernetIpv4.value = ''
  }
}

const saveGatewayOnly = async () => {
  gatewayBusy.value = true
  try {
    await saveHttpGatewayConfig({ ...gatewayConfig })
    dialog.toast.success(t('settings.network.gateway.saved'))
    await refreshGatewayStatus()
  } catch (error) {
    dialog.toast.error(t('settings.network.gateway.saveFailed', { error: String(error) }))
  } finally {
    gatewayBusy.value = false
  }
}

const handleGatewayToggle = async () => {
  gatewayBusy.value = true
  try {
    await saveHttpGatewayConfig({ ...gatewayConfig })

    if (gatewayConfig.enabled) {
      await startHttpGateway({ ...gatewayConfig })
      dialog.toast.success(t('settings.network.gateway.started'))
    } else {
      await stopHttpGateway()
      dialog.toast.success(t('settings.network.gateway.stopped'))
    }

    await refreshGatewayStatus()
  } catch (error) {
    gatewayConfig.enabled = !gatewayConfig.enabled
    dialog.toast.error(t('settings.network.gateway.toggleFailed', { error: String(error) }))
  } finally {
    gatewayBusy.value = false
  }
}

const generateGatewayApiKey = async () => {
  gatewayBusy.value = true
  try {
    const key = await rotateHttpGatewayApiKey()
    lastGeneratedGatewayKey.value = key
    await loadGatewayConfig()
    dialog.toast.success(t('settings.network.gateway.generated'))
  } catch (error) {
    dialog.toast.error(t('settings.network.gateway.generateFailed', { error: String(error) }))
  } finally {
    gatewayBusy.value = false
  }
}

const syncWeixinAllowedUsersFromText = () => {
  weixinConfig.allowed_users = weixinAllowedUsersText.value
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
  weixinConfig.dm_policy = weixinConfig.allowed_users.length > 0 ? 'allowlist' : 'open'
}

const loadWeixinConfig = async () => {
  try {
    const cfg = await getWeixinGatewayConfig()
    Object.assign(weixinConfig, cfg)
    weixinAllowedUsersText.value = cfg.allowed_users.join(',')
  } catch (error) {
    console.error('Failed to load Weixin config:', error)
  }
}

const loadWeixinAssistantProfiles = async () => {
  try {
    const profiles = await invoke<AssistantProfileOption[]>('list_assistant_profiles')
    weixinAssistantProfiles.value = profiles.filter((profile) => profile.runMode !== 'team')
  } catch (error) {
    console.error('Failed to load Weixin assistant profiles:', error)
    weixinAssistantProfiles.value = []
  }
}

const refreshWeixinStatus = async () => {
  try {
    const status = await getWeixinGatewayStatus()
    Object.assign(weixinStatus, status)
  } catch (error) {
    console.error('Failed to load Weixin status:', error)
  }
}

const saveWeixinOnly = async () => {
  weixinBusy.value = true
  try {
    syncWeixinAllowedUsersFromText()
    await saveWeixinGatewayConfig({ ...weixinConfig })
    dialog.toast.success(t('settings.network.weixin.saved'))
    await refreshWeixinStatus()
  } catch (error) {
    dialog.toast.error(t('settings.network.weixin.saveFailed', { error: String(error) }))
  } finally {
    weixinBusy.value = false
  }
}

const handleWeixinToggle = async () => {
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
    await refreshWeixinStatus()
  } catch (error) {
    weixinConfig.enabled = !weixinConfig.enabled
    dialog.toast.error(t('settings.network.weixin.toggleFailed', { error: String(error) }))
  } finally {
    weixinBusy.value = false
  }
}

const clearWeixinQrTimer = () => {
  if (weixinQrTimer !== null) {
    window.clearInterval(weixinQrTimer)
    weixinQrTimer = null
  }
}

const startWeixinQrLoginFlow = async () => {
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


// 网络代理
const loadProxy = async () => {
  try {
    const cfg = await invoke('get_global_proxy_config') as any
    network.proxy.enabled = !!cfg.enabled
    network.proxy.scheme = cfg.scheme || 'http'
    network.proxy.host = cfg.host || ''
    network.proxy.port = cfg.port || 0
    network.proxy.username = cfg.username || ''
    network.proxy.password = cfg.password || ''
    network.proxy.no_proxy = cfg.no_proxy || ''
  } catch (e) {
    console.warn('loadProxy failed', e)
  }
}

const refreshRuntimeProxyStatus = async () => {
  runtimeProxyBusy.value = true
  try {
    const [proxyStatusResponse, autoStartResponse] = await Promise.all([
      invoke<any>('get_proxy_status'),
      invoke<any>('get_proxy_auto_start'),
    ])

    runtimeProxyStatus.running = !!proxyStatusResponse?.data?.running
    runtimeProxyStatus.port = Number(proxyStatusResponse?.data?.port || 0)
    runtimeProxyAutoStart.value = !!autoStartResponse?.data
  } catch (e) {
    console.error('refreshRuntimeProxyStatus failed', e)
    dialog.toast.error(t('settings.network.toast.runtimeStatusFailed'))
  } finally {
    runtimeProxyBusy.value = false
  }
}

const handleWindowFocus = () => {
  refreshRuntimeProxyStatus()
}

const handleVisibilityChange = () => {
  if (document.visibilityState === 'visible') {
    refreshRuntimeProxyStatus()
  }
}



// 生命周期
onMounted(() => {
  try {
    isGatewayWebMode.value = !!(window as any).__SENTINEL_GATEWAY__?.enabled
    browserGatewayKey.value = localStorage.getItem(BROWSER_GATEWAY_KEY_STORAGE) || ''
  } catch {
    isGatewayWebMode.value = false
    browserGatewayKey.value = ''
  }
  loadProxy()
  refreshRuntimeProxyStatus()
  loadGatewayConfig()
  refreshGatewayStatus()
  loadWeixinConfig()
  loadWeixinAssistantProfiles()
  refreshWeixinStatus()
  loadEthernetIpv4()

  listen('proxy:status', (event: any) => {
    const payload = event.payload || {}
    runtimeProxyStatus.running = !!payload.running
    runtimeProxyStatus.port = Number(payload.port || 0)
  }).then((unlisten) => {
    unlistenProxyStatus = unlisten
  }).catch((error) => {
    console.error('Failed to listen proxy:status in NetworkSettings:', error)
  })

  window.addEventListener('focus', handleWindowFocus)
  document.addEventListener('visibilitychange', handleVisibilityChange)
})

onUnmounted(() => {
  if (unlistenProxyStatus) {
    unlistenProxyStatus()
    unlistenProxyStatus = null
  }
  clearWeixinQrTimer()
  window.removeEventListener('focus', handleWindowFocus)
  document.removeEventListener('visibilitychange', handleVisibilityChange)
})

const saveProxy = async () => {
  try {
    const cfg = {
      enabled: network.proxy.enabled,
      scheme: network.proxy.scheme,
      host: network.proxy.host,
      port: Number(network.proxy.port) || null,
      username: network.proxy.username || null,
      password: network.proxy.password || null,
      no_proxy: network.proxy.no_proxy || null,
    }
    await invoke('set_global_proxy_config', { cfg })
    await refreshRuntimeProxyStatus()
    dialog.toast.success(t('settings.network.toast.proxySaved'))
  } catch (e) {
    dialog.toast.error(t('settings.network.toast.proxySaveFailed'))
  }
}

const saveBrowserGatewayKey = () => {
  const key = browserGatewayKey.value.trim()
  if (!key) {
    dialog.toast.error('请输入 API Key')
    return
  }
  try {
    localStorage.setItem(BROWSER_GATEWAY_KEY_STORAGE, key)
    dialog.toast.success('浏览器网关 API Key 已保存')
  } catch (e) {
    dialog.toast.error(`保存失败: ${String(e)}`)
  }
}

const clearBrowserGatewayKey = () => {
  try {
    localStorage.removeItem(BROWSER_GATEWAY_KEY_STORAGE)
    browserGatewayKey.value = ''
    dialog.toast.success('浏览器网关 API Key 已清除')
  } catch (e) {
    dialog.toast.error(`清除失败: ${String(e)}`)
  }
}

const copyBrowserGatewayKey = async () => {
  const key = browserGatewayKey.value.trim()
  if (!key) return
  try {
    await navigator.clipboard.writeText(key)
    dialog.toast.success('API Key 已复制')
  } catch (e) {
    dialog.toast.error(`复制失败: ${String(e)}`)
  }
}


</script>
