<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
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
  activateWithLicenseCard,
  refreshEntitlementTokenFromServer,
} from '../services/entitlementRefresh'
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
  trial_active: boolean
  trial_started_at: number | null
  trial_expires_at: number | null
  trial_remaining_seconds: number | null
  trial_days_remaining: number | null
}

const emit = defineEmits<{
  (e: 'activated'): void
}>()

const entitlements = useFeatureEntitlementsState()
const featureAccessStatus = useFeatureAccessStatusState()
const entitlementRefreshRuntime = useEntitlementRefreshRuntimeState()
const licenseInfo = ref<LicenseInfo | null>(null)
const loading = ref(false)
const accessSyncLoading = ref(false)
const error = ref('')
const accessSyncError = ref('')
const accessSyncMessage = ref('')
const copied = ref(false)
const activationUsername = ref('')
const activationKey = ref('')
const dialogOpen = ref(false)
const featureAccessToolsOpen = ref(false)
const upgradeToolbarRef = ref<HTMLElement | null>(null)
const upgradeToolbarPosition = ref<{ x: number; y: number } | null>(null)
const upgradeToolbarDrag = ref<{
  pointerId: number
  startClientX: number
  startClientY: number
  startX: number
  startY: number
} | null>(null)

const UPGRADE_TOOLBAR_POSITION_KEY = 'sentinel.license-upgrade-toolbar-position'
const UPGRADE_TOOLBAR_MARGIN = 12

const isTrialAccess = computed(() => entitlements.value.access_source === 'trial' || entitlements.value.trial_active || Boolean(licenseInfo.value?.trial_active))
const hasLocalLicense = computed(() => (
  entitlements.value.has_local_license
  || (Boolean(licenseInfo.value?.is_licensed) && !isTrialAccess.value)
))
const hasFullAccess = computed(() => entitlements.value.is_licensed)
const isDebugAccess = computed(() => entitlements.value.access_source === 'debug')
const showUpgradeEntry = computed(() => !hasFullAccess.value)
const refreshServiceConfigured = computed(() => true)
const activationView = computed(() => buildLicenseActivationViewState({
  hasLocalLicense: hasLocalLicense.value,
  isDebugAccess: isDebugAccess.value,
  isTrialAccess: isTrialAccess.value,
  trialExpiresAt: entitlements.value.trial_expires_at ?? licenseInfo.value?.trial_expires_at ?? null,
  trialDaysRemaining: entitlements.value.trial_days_remaining ?? licenseInfo.value?.trial_days_remaining ?? null,
  featureAccessStatus: featureAccessStatus.value,
  refreshServiceConfigured: refreshServiceConfigured.value,
  refreshRuntime: entitlementRefreshRuntime.value,
  refreshCooldownSeconds: getEntitlementRefreshCooldownSeconds(),
  formatTimestamp,
  formatDuration: formatDurationLabel,
}))
const activateButtonLabel = computed(() => (loading.value ? '激活中...' : '激活'))
const upgradeToolbarStyle = computed(() => {
  if (!upgradeToolbarPosition.value) {
    return {
      right: '1rem',
      bottom: '1rem',
    }
  }

  return {
    left: `${upgradeToolbarPosition.value.x}px`,
    top: `${upgradeToolbarPosition.value.y}px`,
  }
})

onMounted(async () => {
  loadUpgradeToolbarPosition()
  window.addEventListener('pointermove', handleUpgradeToolbarPointerMove)
  window.addEventListener('pointerup', handleUpgradeToolbarPointerUp)
  window.addEventListener('resize', clampStoredUpgradeToolbarPosition)
  await refreshAllStatus()
})

onUnmounted(() => {
  window.removeEventListener('pointermove', handleUpgradeToolbarPointerMove)
  window.removeEventListener('pointerup', handleUpgradeToolbarPointerUp)
  window.removeEventListener('resize', clampStoredUpgradeToolbarPosition)
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
  await Promise.all([checkLicenseStatus(), checkFeatureAccessStatus()])
}

async function activateLicense() {
  if (!activationUsername.value.trim() || !activationKey.value.trim()) {
    error.value = '请输入用户名和激活密钥'
    return
  }

  loading.value = true
  error.value = ''

  try {
    const result = await activateWithLicenseCard({
      username: activationUsername.value.trim(),
      activation_key: activationKey.value.trim(),
    })

    if (result.success) {
      error.value = ''
      accessSyncError.value = ''
      accessSyncMessage.value = result.message
      markEntitlementRefreshSuccess()
      await refreshAllStatus()
      emit('activated')
      dialogOpen.value = false
    } else {
      if (result.configured) {
        markEntitlementRefreshFailure({
          message: result.message,
          errorCode: result.error_code,
          retryAfterSecs: result.retry_after_secs,
        })
      }
      error.value = result.message
      await refreshAllStatus()
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
  featureAccessToolsOpen.value = hasLocalLicense.value
  dialogOpen.value = true
}

function startUpgradeToolbarDrag(event: PointerEvent) {
  if (event.button !== 0) {
    return
  }

  const rect = upgradeToolbarRef.value?.getBoundingClientRect()
  if (!rect) {
    return
  }

  event.preventDefault()
  upgradeToolbarDrag.value = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    startX: rect.left,
    startY: rect.top,
  }
}

function handleUpgradeToolbarPointerMove(event: PointerEvent) {
  const drag = upgradeToolbarDrag.value
  if (!drag || drag.pointerId !== event.pointerId) {
    return
  }

  upgradeToolbarPosition.value = clampUpgradeToolbarPosition({
    x: drag.startX + event.clientX - drag.startClientX,
    y: drag.startY + event.clientY - drag.startClientY,
  })
}

function handleUpgradeToolbarPointerUp(event: PointerEvent) {
  const drag = upgradeToolbarDrag.value
  if (!drag || drag.pointerId !== event.pointerId) {
    return
  }

  upgradeToolbarDrag.value = null
  persistUpgradeToolbarPosition()
}

function loadUpgradeToolbarPosition() {
  const raw = window.localStorage.getItem(UPGRADE_TOOLBAR_POSITION_KEY)
  if (!raw) {
    return
  }

  try {
    const parsed = JSON.parse(raw)
    if (typeof parsed?.x === 'number' && typeof parsed?.y === 'number') {
      upgradeToolbarPosition.value = clampUpgradeToolbarPosition(parsed)
    }
  } catch (error) {
    window.localStorage.removeItem(UPGRADE_TOOLBAR_POSITION_KEY)
  }
}

function persistUpgradeToolbarPosition() {
  if (!upgradeToolbarPosition.value) {
    return
  }

  window.localStorage.setItem(
    UPGRADE_TOOLBAR_POSITION_KEY,
    JSON.stringify(upgradeToolbarPosition.value),
  )
}

function clampStoredUpgradeToolbarPosition() {
  if (!upgradeToolbarPosition.value) {
    return
  }

  upgradeToolbarPosition.value = clampUpgradeToolbarPosition(upgradeToolbarPosition.value)
  persistUpgradeToolbarPosition()
}

function clampUpgradeToolbarPosition(position: { x: number; y: number }) {
  const toolbar = upgradeToolbarRef.value
  const width = toolbar?.offsetWidth || 220
  const height = toolbar?.offsetHeight || 44
  const maxX = Math.max(UPGRADE_TOOLBAR_MARGIN, window.innerWidth - width - UPGRADE_TOOLBAR_MARGIN)
  const maxY = Math.max(UPGRADE_TOOLBAR_MARGIN, window.innerHeight - height - UPGRADE_TOOLBAR_MARGIN)

  return {
    x: Math.min(Math.max(position.x, UPGRADE_TOOLBAR_MARGIN), maxX),
    y: Math.min(Math.max(position.y, UPGRADE_TOOLBAR_MARGIN), maxY),
  }
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
      ref="upgradeToolbarRef"
      class="fixed z-[9998] flex items-center gap-2 rounded-2xl border border-warning/30 bg-base-100/95 px-2 py-2 shadow-xl backdrop-blur"
      :style="upgradeToolbarStyle"
    >
      <button
        type="button"
        class="btn btn-ghost btn-xs cursor-grab px-2 active:cursor-grabbing"
        aria-label="拖动激活入口"
        @pointerdown="startUpgradeToolbarDrag"
      >
        <i class="fas fa-grip-vertical"></i>
      </button>
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
              <span class="label-text font-medium">用户名</span>
            </label>
            <input
              v-model="activationUsername"
              type="text"
              class="input input-bordered"
              placeholder="输入管理员分配的用户名"
              autocomplete="username"
            />
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">激活密钥</span>
            </label>
            <input
              v-model="activationKey"
              type="password"
              class="input input-bordered font-mono"
              placeholder="输入管理员分配的卡密"
              autocomplete="one-time-code"
              @keyup.enter="activateLicense"
            />
            <label class="label">
              <span class="label-text-alt text-base-content/50">激活成功后会自动绑定当前设备，后续无需重复输入。</span>
            </label>
          </div>

          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">当前设备</span>
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
              <span class="label-text-alt text-base-content/50">仅用于设备绑定和设备数限制。</span>
            </label>
          </div>

          <div v-if="error" class="alert alert-error mb-4">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ error }}</span>
          </div>

          <div class="card-actions justify-center">
            <button
              class="btn btn-primary btn-wide"
              :class="{ 'loading': loading }"
              :disabled="loading || !activationUsername.trim() || !activationKey.trim()"
              @click="activateLicense"
            >
              <i v-if="!loading" class="fas fa-unlock mr-2"></i>
              {{ activateButtonLabel }}
            </button>
          </div>

          <template v-if="hasLocalLicense">
            <div class="divider my-6">服务端授权</div>

            <div class="rounded-2xl border border-base-300 bg-base-200/50 p-4">
              <div class="flex items-start justify-between gap-4">
                <div class="space-y-2">
                  <div class="flex items-center gap-2">
                    <h3 class="text-lg font-semibold">当前授权</h3>
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
                      : '服务端激活成功后会开放全部功能，不再按单个功能拆分授权。'
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
                    :disabled="accessSyncLoading"
                    @click="refreshFeatureAccessFromServer"
                  >
                    <i v-if="!accessSyncLoading" class="fas fa-rotate-right mr-2"></i>
                    立即刷新授权
                  </button>
                </div>

                <div class="divider my-2">自动续期</div>

                <div class="alert alert-info">
                  <i class="fas fa-rotate"></i>
                  <span>{{ activationView.refreshRuntimeText }}</span>
                </div>
                <p class="text-xs text-base-content/50 text-center">
                  自动续期使用当前设备保存的服务端凭证，不需要用户维护服务地址或 refresh key。
                </p>
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
