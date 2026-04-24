<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  refreshFeatureEntitlements,
  useFeatureEntitlementsState,
} from '../services/featureEntitlements'
import {
  refreshFeatureAccessStatus,
  useFeatureAccessStatusState,
} from '../services/featureAccessStatus'
import {
  getEntitlementRefreshConfig,
  refreshEntitlementTokenFromServer,
} from '../services/entitlementRefresh'
import { attemptEntitlementAutoRefresh } from '../services/entitlementAutoRefresh'
import { getFeatureAccessIssueMessage } from '../services/featureAccessMessaging'
import { buildLicenseActivationViewState } from '../services/licenseActivationViewState'
import {
  formatDurationLabel,
  getEntitlementRefreshCooldownSeconds,
  markEntitlementRefreshFailure,
  markEntitlementRefreshSuccess,
  useEntitlementRefreshRuntimeState,
} from '../services/entitlementRefreshState'

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
const featureAccessStatus = useFeatureAccessStatusState()
const entitlementRefreshRuntime = useEntitlementRefreshRuntimeState()
const licenseInfo = ref<LicenseInfo | null>(null)
const licenseKey = ref('')
const refreshConfig = ref({
  enabled: false,
  endpoint: '',
  api_key_configured: false,
})
const loading = ref(false)
const accessSyncLoading = ref(false)
const error = ref('')
const accessSyncError = ref('')
const accessSyncMessage = ref('')
const copied = ref(false)
const dialogOpen = ref(false)
const featureAccessToolsOpen = ref(false)

const hasLocalLicense = computed(() => entitlements.value.has_local_license || Boolean(licenseInfo.value?.is_licensed))
const hasFullAccess = computed(() => entitlements.value.is_licensed)
const isDebugAccess = computed(() => entitlements.value.access_source === 'debug')
const showUpgradeEntry = computed(() => !hasFullAccess.value)
const refreshServiceConfigured = computed(
  () => refreshConfig.value.enabled && refreshConfig.value.endpoint.trim().length > 0,
)
const activationView = computed(() => buildLicenseActivationViewState({
  hasLocalLicense: hasLocalLicense.value,
  isDebugAccess: isDebugAccess.value,
  featureAccessStatus: featureAccessStatus.value,
  refreshServiceConfigured: refreshServiceConfigured.value,
  refreshRuntime: entitlementRefreshRuntime.value,
  refreshCooldownSeconds: getEntitlementRefreshCooldownSeconds(),
  formatTimestamp,
  formatDuration: formatDurationLabel,
}))
const activateButtonLabel = computed(() => (loading.value ? '激活中...' : '完成本机激活'))

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

async function checkFeatureAccessStatus() {
  await refreshFeatureAccessStatus()
}

async function refreshAllStatus() {
  await refreshFeatureEntitlements()
  await Promise.all([checkLicenseStatus(), checkFeatureAccessStatus(), loadRefreshConfig()])
}

async function activateLicense() {
  if (!licenseKey.value.trim()) {
    error.value = '请输入许可证密钥'
    return
  }

  loading.value = true
  error.value = ''

  try {
    const result = await invoke<ActivationResult>('activate_license', {
      licenseKey: licenseKey.value.trim()
    })

    if (result.success) {
      licenseKey.value = ''
      error.value = ''
      accessSyncError.value = ''
      accessSyncMessage.value = ''
      await refreshAllStatus()

      const autoRefreshOutcome = await attemptEntitlementAutoRefresh({
        force: true,
        expiringSoonThresholdSeconds: 24 * 60 * 60,
      })
      await refreshAllStatus()
      emit('activated')

      if (autoRefreshOutcome.status === 'failure' && autoRefreshOutcome.configured) {
        featureAccessToolsOpen.value = true
        accessSyncError.value = autoRefreshOutcome.message
        return
      }

      dialogOpen.value = false
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

async function refreshFeatureAccessFromServer() {
  accessSyncLoading.value = true
  accessSyncError.value = ''
  accessSyncMessage.value = ''

  try {
    const result = await refreshEntitlementTokenFromServer()
    if (result.success) {
      markEntitlementRefreshSuccess()
      accessSyncMessage.value = result.message
      await refreshAllStatus()
    } else if (!result.configured) {
      accessSyncError.value = '尚未配置高级功能权限刷新服务'
    } else {
      markEntitlementRefreshFailure({
        message: result.message,
        errorCode: result.error_code,
        retryAfterSecs: result.retry_after_secs,
      })
      accessSyncError.value = result.message
      await refreshAllStatus()
    }
  } catch (e) {
    markEntitlementRefreshFailure({
      message: String(e),
      errorCode: 'manual_refresh_exception',
      retryAfterSecs: 300,
    })
    accessSyncError.value = String(e)
  } finally {
    accessSyncLoading.value = false
  }
}

function openDialog() {
  featureAccessToolsOpen.value = hasLocalLicense.value && !featureAccessStatus.value.ready
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
      <span class="badge badge-ghost">{{ activationView.statusBadgeLabel }}</span>
      <button class="btn btn-warning btn-sm" @click="openDialog">
        <i class="fas fa-crown mr-2"></i>
        {{ activationView.upgradeEntryLabel }}
      </button>
    </div>

    <div
      v-if="dialogOpen"
      class="fixed inset-0 z-[9999] flex items-center justify-center bg-base-300/95 px-4 py-4 backdrop-blur-sm"
    >
      <div class="card bg-base-100 shadow-2xl w-full max-w-xl max-h-full overflow-hidden">
        <div class="card-body overflow-y-auto">
          <div class="flex justify-end">
            <button class="btn btn-ghost btn-sm btn-circle" @click="dialogOpen = false">
              <i class="fas fa-times"></i>
            </button>
          </div>

            <div class="text-center mb-6">
              <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
                <i class="fas fa-key text-3xl text-primary"></i>
              </div>
            <h2 class="card-title justify-center text-2xl">{{ activationView.dialogTitle }}</h2>
            <p class="text-base-content/60 mt-2">{{ activationView.dialogSubtitle }}</p>
          </div>

          <div class="alert mb-4" :class="activationView.featureAccessTone">
            <i class="fas fa-shield-alt"></i>
            <span>{{ activationView.featureAccessText }}</span>
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">设备标识</span>
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
              <span class="label-text-alt text-base-content/50">将这串标识发送给许可证签发方，用于生成当前设备的许可证密钥。</span>
            </label>
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">许可证密钥</span>
            </label>
            <textarea
              v-model="licenseKey"
              placeholder="粘贴许可证签发方提供的许可证密钥"
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
              {{ activateButtonLabel }}
            </button>
          </div>

          <template v-if="hasLocalLicense">
            <div class="divider my-6">高级功能同步</div>

            <div class="rounded-2xl border border-base-300 bg-base-200/50 p-4">
              <div class="flex items-start justify-between gap-4">
                <div class="space-y-2">
                  <div class="flex items-center gap-2">
                    <h3 class="text-lg font-semibold">高级功能权限</h3>
                    <span
                      v-if="featureAccessStatus?.exists"
                      class="badge"
                      :class="featureAccessStatus.ready ? 'badge-success' : 'badge-warning'"
                    >
                      {{ featureAccessStatus.ready ? '有效' : '无效' }}
                    </span>
                  </div>
                  <p class="text-sm text-base-content/70">
                    {{ activationView.featureAccessSummary }}
                  </p>
                  <p class="text-xs text-base-content/50">
                    {{ isDebugAccess
                      ? '当前为 debug 模式，这里的权限状态仅用于手工验证 release 授权链路，不影响本地开发放行。'
                      : '它不是第二次基础激活，而是服务端对高价值功能的短期权限同步。普通本地授权完成后，不需要再重复输入许可密钥。'
                    }}
                  </p>
                </div>
                <button class="btn btn-sm btn-outline" @click="featureAccessToolsOpen = !featureAccessToolsOpen">
                  <i :class="featureAccessToolsOpen ? 'fas fa-chevron-up mr-2' : 'fas fa-chevron-down mr-2'"></i>
                  {{ featureAccessToolsOpen ? '收起工具' : '展开工具' }}
                </button>
              </div>

              <div v-if="featureAccessStatus?.exists" class="rounded-xl border border-base-300 bg-base-100 px-4 py-3 text-sm mt-4">
                <div class="flex items-center justify-between gap-3">
                  <span class="font-medium">当前权限状态</span>
                  <span class="badge" :class="featureAccessStatus.ready ? 'badge-success' : 'badge-warning'">
                    {{ featureAccessStatus.ready ? '有效' : '无效' }}
                  </span>
                </div>
                <div class="mt-2 space-y-1 text-base-content/70">
                  <p v-if="featureAccessStatus.tier">等级：{{ featureAccessStatus.tier }}</p>
                  <p v-if="featureAccessStatus.expiresAt">过期时间：{{ formatTimestamp(featureAccessStatus.expiresAt) }}</p>
                  <p v-if="featureAccessStatus.scopeIds.length">权限范围：{{ featureAccessStatus.scopeIds.join(', ') }}</p>
                  <p v-if="!featureAccessStatus.ready">{{ getFeatureAccessIssueMessage(featureAccessStatus) }}</p>
                </div>
              </div>

              <div v-if="featureAccessToolsOpen" class="mt-4 space-y-4">
                <div v-if="accessSyncError" class="alert alert-error">
                  <i class="fas fa-exclamation-circle"></i>
                  <span>{{ accessSyncError }}</span>
                </div>

                <div v-else-if="accessSyncMessage" class="alert alert-success">
                  <i class="fas fa-check-circle"></i>
                  <span>{{ accessSyncMessage }}</span>
                </div>

                <div class="card-actions justify-center">
                  <button
                    class="btn btn-outline"
                    :class="{ 'loading': accessSyncLoading }"
                    :disabled="accessSyncLoading || !refreshConfig.enabled || !refreshConfig.endpoint.trim()"
                    @click="refreshFeatureAccessFromServer"
                  >
                    <i v-if="!accessSyncLoading" class="fas fa-rotate-right mr-2"></i>
                    立即补齐权限
                  </button>
                </div>

                <div class="divider my-2">自动刷新</div>

                <div class="alert alert-info">
                  <i class="fas fa-rotate"></i>
                  <span>{{ activationView.refreshRuntimeText }}</span>
                </div>

                <div class="rounded-xl border border-base-300 bg-base-100 px-4 py-3 text-sm text-base-content/70">
                  <div class="flex items-center justify-between gap-3">
                    <span class="font-medium text-base-content">服务端同步配置</span>
                    <span class="badge" :class="refreshServiceConfigured ? 'badge-success' : 'badge-ghost'">
                      {{ refreshServiceConfigured ? '已配置' : '未配置' }}
                    </span>
                  </div>
                  <p class="mt-2">{{ activationView.refreshServiceHint }}</p>
                  <p v-if="refreshServiceConfigured" class="mt-1 text-xs text-base-content/50 break-all">
                    服务地址：{{ refreshConfig.endpoint }}
                  </p>
                  <p class="mt-2 text-xs text-base-content/50">
                    手工写入或清除高级功能权限的管理员工具已移至 设置 &gt; 安全 &gt; 高级功能权限同步管理。
                  </p>
                </div>
              </div>
            </div>
          </template>

          <div class="text-center mt-4">
            <a href="mailto:support@example.com" class="link link-hover text-sm text-base-content/60">
              <i class="fas fa-question-circle mr-1"></i>
              需要帮助
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
