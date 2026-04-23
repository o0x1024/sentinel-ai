<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import {
  refreshFeatureEntitlements,
  useFeatureEntitlementsState,
} from '../services/featureEntitlements'
import {
  refreshEntitlementTokenStatus,
  useEntitlementTokenStatusState,
  getEntitlementTokenIssueMessage,
} from '../services/entitlementToken'
import {
  getEntitlementRefreshConfig,
  refreshEntitlementTokenFromServer,
  saveEntitlementRefreshConfig,
} from '../services/entitlementRefresh'
import {
  formatDurationLabel,
  getEntitlementRefreshCooldownSeconds,
  markEntitlementRefreshFailure,
  markEntitlementRefreshSuccess,
  useEntitlementRefreshRuntimeState,
} from '../services/entitlementRefreshState'

const { t } = useI18n()

interface LicenseInfo {
  machine_id: string
  is_licensed: boolean
  needs_activation: boolean
}

interface ActivationResult {
  success: boolean
  message: string
}

const emit = defineEmits<{
  (e: 'activated'): void
}>()

const entitlements = useFeatureEntitlementsState()
const entitlementTokenStatus = useEntitlementTokenStatusState()
const entitlementRefreshRuntime = useEntitlementRefreshRuntimeState()
const licenseInfo = ref<LicenseInfo | null>(null)
const licenseKey = ref('')
const entitlementToken = ref('')
const refreshConfig = ref({
  enabled: false,
  endpoint: '',
  api_key: '',
  customer_id: '',
  timeout_secs: 15,
  api_key_configured: false,
})
const loading = ref(false)
const tokenLoading = ref(false)
const refreshConfigLoading = ref(false)
const error = ref('')
const tokenError = ref('')
const tokenMessage = ref('')
const copied = ref(false)
const dialogOpen = ref(false)

const needsActivation = computed(() => licenseInfo.value?.needs_activation ?? false)
const hasLocalLicense = computed(() => entitlements.value.has_local_license || Boolean(licenseInfo.value?.is_licensed))
const hasFullAccess = computed(() => entitlements.value.is_licensed)
const isDebugAccess = computed(() => entitlements.value.access_source === 'debug')
const showUpgradeEntry = computed(() => !hasFullAccess.value)
const upgradeEntryLabel = computed(() => {
  if (!hasLocalLicense.value) {
    return '升级 Pro'
  }

  if (!entitlementTokenStatus.value.valid) {
    return '同步授权'
  }

  return '查看授权'
})
const statusBadgeLabel = computed(() => {
  if (isDebugAccess.value) {
    return 'Debug'
  }

  if (!hasLocalLicense.value) {
    return 'Free'
  }

  if (entitlementTokenStatus.value.valid) {
    return 'Token OK'
  }

  return 'License Only'
})
const tokenStatusTone = computed(() => {
  if (isDebugAccess.value) {
    return 'alert-success'
  }

  if (!hasLocalLicense.value) {
    return 'alert-warning'
  }

  if (entitlementTokenStatus.value.valid) {
    return 'alert-success'
  }

  return 'alert-info'
})
const tokenStatusText = computed(() => {
  if (isDebugAccess.value) {
    return '当前为 debug 模式，已绕过 release 环境下的 license 和 entitlement token 限制。'
  }

  if (!hasLocalLicense.value) {
    return '当前未激活本地 license。release 环境下高级功能不可用。'
  }

  if (entitlementTokenStatus.value.valid) {
    const expiresAt = formatTimestamp(entitlementTokenStatus.value.expires_at)
    return expiresAt
      ? `已同步 entitlement token，过期时间：${expiresAt}`
      : '已同步 entitlement token'
  }

  return `已激活本地 license，但${getEntitlementTokenIssueMessage(entitlementTokenStatus.value)}。高价值功能仍会受限。`
})
const refreshRuntimeText = computed(() => {
  if (isDebugAccess.value) {
    return 'debug 模式不会要求 entitlement token。需要验证 release 限制时，可手工写入或从服务端刷新 token。'
  }

  if (entitlementTokenStatus.value.valid) {
    if (entitlementRefreshRuntime.value.last_success_at) {
      return `最近一次自动同步成功时间：${formatTimestamp(entitlementRefreshRuntime.value.last_success_at)}`
    }
    return '当前 entitlement token 有效。'
  }

  if (entitlementRefreshRuntime.value.next_retry_at) {
    const cooldown = getEntitlementRefreshCooldownSeconds()
    return `自动刷新冷却中，下次重试${formatDurationLabel(cooldown)}。`
  }

  if (entitlementRefreshRuntime.value.last_error) {
    return `最近一次自动刷新失败：${entitlementRefreshRuntime.value.last_error}`
  }

  return '当前还没有自动刷新记录。'
})

onMounted(async () => {
  await refreshAllStatus()
})

async function checkLicenseStatus() {
  try {
    licenseInfo.value = await invoke<LicenseInfo>('get_license_info')
  } catch (e) {
    console.error('Failed to check license status:', e)
  }
}

async function checkEntitlementTokenStatus() {
  await refreshEntitlementTokenStatus()
}

async function refreshAllStatus() {
  await refreshFeatureEntitlements()
  await Promise.all([checkLicenseStatus(), checkEntitlementTokenStatus(), loadRefreshConfig()])

  if (hasFullAccess.value) {
    emit('activated')
  }
}

async function activateLicense() {
  if (!licenseKey.value.trim()) {
    error.value = t('license.enterKey')
    return
  }

  loading.value = true
  error.value = ''

  try {
    const result = await invoke<ActivationResult>('activate_license', {
      licenseKey: licenseKey.value.trim()
    })

    if (result.success) {
      dialogOpen.value = false
      licenseKey.value = ''
      error.value = ''
      await refreshAllStatus()
    } else {
      error.value = result.message
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function copyMachineId() {
  if (licenseInfo.value?.machine_id) {
    try {
      await navigator.clipboard.writeText(licenseInfo.value.machine_id)
      copied.value = true
      setTimeout(() => {
        copied.value = false
      }, 2000)
    } catch (e) {
      console.error('Failed to copy:', e)
    }
  }
}

async function storeEntitlementToken() {
  if (!entitlementToken.value.trim()) {
    tokenError.value = '请输入 entitlement token'
    return
  }

  tokenLoading.value = true
  tokenError.value = ''
  tokenMessage.value = ''

  try {
    const result = await invoke<ActivationResult>('store_entitlement_token', {
      token: entitlementToken.value.trim()
    })

    if (result.success) {
      entitlementToken.value = ''
      tokenMessage.value = result.message
      await refreshAllStatus()
    } else {
      tokenError.value = result.message
    }
  } catch (e) {
    tokenError.value = String(e)
  } finally {
    tokenLoading.value = false
  }
}

async function clearEntitlementToken() {
  tokenLoading.value = true
  tokenError.value = ''
  tokenMessage.value = ''

  try {
    const result = await invoke<ActivationResult>('clear_entitlement_token')
    if (result.success) {
      entitlementToken.value = ''
      tokenMessage.value = result.message
      await refreshAllStatus()
    } else {
      tokenError.value = result.message
    }
  } catch (e) {
    tokenError.value = String(e)
  } finally {
    tokenLoading.value = false
  }
}

async function loadRefreshConfig() {
  if (!hasLocalLicense.value && !licenseInfo.value?.is_licensed) {
    return
  }

  try {
    refreshConfig.value = await getEntitlementRefreshConfig()
  } catch (e) {
    console.error('Failed to load entitlement refresh config:', e)
  }
}

async function saveRefreshConfig() {
  refreshConfigLoading.value = true
  tokenError.value = ''
  tokenMessage.value = ''

  try {
    refreshConfig.value = await saveEntitlementRefreshConfig({
      enabled: refreshConfig.value.enabled,
      endpoint: refreshConfig.value.endpoint,
      api_key: refreshConfig.value.api_key,
      customer_id: refreshConfig.value.customer_id,
      timeout_secs: refreshConfig.value.timeout_secs,
    })
    tokenMessage.value = '自动同步配置已保存'
  } catch (e) {
    tokenError.value = String(e)
  } finally {
    refreshConfigLoading.value = false
  }
}

async function refreshTokenFromServer() {
  tokenLoading.value = true
  tokenError.value = ''
  tokenMessage.value = ''

  try {
    const result = await refreshEntitlementTokenFromServer()
    if (result.success) {
      markEntitlementRefreshSuccess()
      tokenMessage.value = result.message
      await refreshAllStatus()
    } else if (!result.configured) {
      tokenError.value = '尚未配置 entitlement 刷新服务'
    } else {
      markEntitlementRefreshFailure({
        message: result.message,
        errorCode: result.error_code,
        retryAfterSecs: result.retry_after_secs,
      })
      tokenError.value = result.message
      await refreshAllStatus()
    }
  } catch (e) {
    markEntitlementRefreshFailure({
      message: String(e),
      errorCode: 'manual_refresh_exception',
      retryAfterSecs: 300,
    })
    tokenError.value = String(e)
  } finally {
    tokenLoading.value = false
  }
}

function openDialog() {
  dialogOpen.value = true
}

function formatTimestamp(timestamp: number | null) {
  if (!timestamp) {
    return ''
  }

  return new Date(timestamp * 1000).toLocaleString()
}

defineExpose({
  openDialog,
  refreshAllStatus,
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="showUpgradeEntry"
      class="fixed bottom-4 right-4 z-[9998] flex items-center gap-2 rounded-2xl border border-warning/30 bg-base-100/95 px-3 py-2 shadow-xl backdrop-blur"
    >
      <span class="badge badge-ghost">{{ statusBadgeLabel }}</span>
      <button class="btn btn-warning btn-sm" @click="dialogOpen = true">
        <i class="fas fa-crown mr-2"></i>
        {{ upgradeEntryLabel }}
      </button>
    </div>

    <div v-if="dialogOpen" class="fixed inset-0 z-[9999] flex items-center justify-center bg-base-300/95 backdrop-blur-sm">
      <div class="card bg-base-100 shadow-2xl w-full max-w-md mx-4">
        <div class="card-body">
          <div class="flex justify-end">
            <button class="btn btn-ghost btn-sm btn-circle" @click="dialogOpen = false">
              <i class="fas fa-times"></i>
            </button>
          </div>

          <div class="text-center mb-6">
            <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
              <i class="fas fa-key text-3xl text-primary"></i>
            </div>
            <h2 class="card-title justify-center text-2xl">{{ t('license.title') }}</h2>
            <p class="text-base-content/60 mt-2">{{ t('license.subtitle') }}</p>
          </div>

          <div class="alert mb-4" :class="tokenStatusTone">
            <i class="fas fa-shield-alt"></i>
            <span>{{ tokenStatusText }}</span>
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">{{ t('license.machineId') }}</span>
            </label>
            <div class="join w-full">
              <input
                type="text"
                :value="licenseInfo?.machine_id || ''"
                readonly
                class="input input-bordered join-item flex-1 font-mono text-sm bg-base-200"
              />
              <button
                class="btn join-item"
                :class="{ 'btn-success': copied }"
                @click="copyMachineId"
              >
                <i :class="copied ? 'fas fa-check' : 'fas fa-copy'"></i>
              </button>
            </div>
            <label class="label">
              <span class="label-text-alt text-base-content/50">{{ t('license.machineIdHint') }}</span>
            </label>
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">{{ t('license.licenseKey') }}</span>
            </label>
            <textarea
              v-model="licenseKey"
              :placeholder="t('license.enterKeyPlaceholder')"
              class="textarea textarea-bordered font-mono text-sm h-24"
              :disabled="loading"
            ></textarea>
          </div>

          <div v-if="error" class="alert alert-error mb-4">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ error }}</span>
          </div>

          <div class="card-actions justify-center">
            <button
              class="btn btn-primary btn-wide"
              :class="{ 'loading': loading }"
              :disabled="loading || !licenseKey.trim()"
              @click="activateLicense"
            >
              <i v-if="!loading" class="fas fa-unlock mr-2"></i>
              {{ loading ? t('license.activating') : t('license.activate') }}
            </button>
          </div>

          <div v-if="hasLocalLicense" class="divider my-6">Entitlement Token</div>

          <template v-if="hasLocalLicense">
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text font-medium">Entitlement Token</span>
              </label>
              <textarea
                v-model="entitlementToken"
                placeholder="粘贴服务端下发的短期 entitlement token"
                class="textarea textarea-bordered font-mono text-sm h-24"
                :disabled="tokenLoading"
              ></textarea>
              <label class="label">
                <span class="label-text-alt text-base-content/50">
                  {{ isDebugAccess
                    ? '当前为 debug 模式，这里的 token 仅用于手工验证 release 授权链路，不影响本地开发放行。'
                    : 'token 用于 release 环境下同步高价值功能权限，如漏洞赏金、插件目录写入和非白名单插件访问。'
                  }}
                </span>
              </label>
            </div>

            <div v-if="tokenError" class="alert alert-error mb-4">
              <i class="fas fa-exclamation-circle"></i>
              <span>{{ tokenError }}</span>
            </div>

            <div v-else-if="tokenMessage" class="alert alert-success mb-4">
              <i class="fas fa-check-circle"></i>
              <span>{{ tokenMessage }}</span>
            </div>

            <div v-if="entitlementTokenStatus?.exists" class="rounded-xl border border-base-300 bg-base-200/70 px-4 py-3 text-sm mb-4">
              <div class="flex items-center justify-between gap-3">
                <span class="font-medium">当前 Token</span>
                <span class="badge" :class="entitlementTokenStatus.valid ? 'badge-success' : 'badge-warning'">
                  {{ entitlementTokenStatus.valid ? '有效' : '无效' }}
                </span>
              </div>
              <div class="mt-2 space-y-1 text-base-content/70">
                <p v-if="entitlementTokenStatus.tier">Tier: {{ entitlementTokenStatus.tier }}</p>
                <p v-if="entitlementTokenStatus.expires_at">Expires: {{ formatTimestamp(entitlementTokenStatus.expires_at) }}</p>
                <p v-if="entitlementTokenStatus.feature_ids.length">Features: {{ entitlementTokenStatus.feature_ids.join(', ') }}</p>
                <p v-if="!entitlementTokenStatus.valid">{{ getEntitlementTokenIssueMessage(entitlementTokenStatus) }}</p>
              </div>
            </div>

            <div class="card-actions justify-center">
              <button
                class="btn btn-secondary"
                :class="{ 'loading': tokenLoading }"
                :disabled="tokenLoading || !entitlementToken.trim()"
                @click="storeEntitlementToken"
              >
                <i v-if="!tokenLoading" class="fas fa-shield-alt mr-2"></i>
                写入 Token
              </button>
              <button
                class="btn btn-ghost"
                :disabled="tokenLoading || !entitlementTokenStatus?.exists"
                @click="clearEntitlementToken"
              >
                <i class="fas fa-trash-alt mr-2"></i>
                清除 Token
              </button>
              <button
                class="btn btn-outline"
                :class="{ 'loading': tokenLoading }"
                :disabled="tokenLoading || !refreshConfig.enabled || !refreshConfig.endpoint.trim()"
                @click="refreshTokenFromServer"
              >
                <i v-if="!tokenLoading" class="fas fa-rotate-right mr-2"></i>
                从服务端刷新
              </button>
            </div>

            <div class="divider my-6">自动刷新</div>

            <div class="alert alert-info mb-4">
              <i class="fas fa-rotate"></i>
              <span>{{ refreshRuntimeText }}</span>
            </div>

            <div class="form-control mb-3">
              <label class="label cursor-pointer justify-start gap-3">
                <input v-model="refreshConfig.enabled" type="checkbox" class="toggle toggle-primary" />
                <span class="label-text">启用自动同步 entitlement token</span>
              </label>
            </div>

            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text font-medium">Refresh Endpoint</span>
              </label>
              <input
                v-model="refreshConfig.endpoint"
                type="text"
                class="input input-bordered"
                placeholder="https://license.example.com/api/entitlements/refresh"
                :disabled="refreshConfigLoading"
              />
            </div>

            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text font-medium">Customer ID</span>
              </label>
              <input
                v-model="refreshConfig.customer_id"
                type="text"
                class="input input-bordered"
                placeholder="可选，用于多租户/客户侧识别"
                :disabled="refreshConfigLoading"
              />
            </div>

            <div class="form-control mb-3">
              <label class="label">
                <span class="label-text font-medium">Refresh API Key</span>
              </label>
              <input
                v-model="refreshConfig.api_key"
                type="password"
                class="input input-bordered"
                placeholder="服务端签发接口的 API Key / Bearer Token"
                :disabled="refreshConfigLoading"
              />
            </div>

            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text font-medium">Timeout (seconds)</span>
              </label>
              <input
                v-model.number="refreshConfig.timeout_secs"
                type="number"
                min="5"
                max="120"
                class="input input-bordered"
                :disabled="refreshConfigLoading"
              />
            </div>

            <div class="card-actions justify-center">
              <button
                class="btn btn-outline btn-primary"
                :class="{ 'loading': refreshConfigLoading }"
                :disabled="refreshConfigLoading"
                @click="saveRefreshConfig"
              >
                <i v-if="!refreshConfigLoading" class="fas fa-floppy-disk mr-2"></i>
                保存自动刷新配置
              </button>
            </div>
          </template>

          <div class="text-center mt-4">
            <a href="mailto:support@example.com" class="link link-hover text-sm text-base-content/60">
              <i class="fas fa-question-circle mr-1"></i>
              {{ t('license.needHelp') }}
            </a>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop-blur-sm {
  backdrop-filter: blur(4px);
}
</style>
