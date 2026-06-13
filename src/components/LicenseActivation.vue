<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  refreshFeatureEntitlements,
  useFeatureEntitlementsState,
} from '../services/featureEntitlements'
import { buildLicenseActivationViewState } from '../services/licenseActivationViewState'

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
const licenseInfo = ref<LicenseInfo | null>(null)
const loading = ref(false)
const error = ref('')
const copied = ref(false)
const licenseKey = ref('')
const dialogOpen = ref(false)
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

const hasLocalLicense = computed(() => (
  entitlements.value.has_local_license
  || Boolean(licenseInfo.value?.is_licensed)
))
const hasFullAccess = computed(() => entitlements.value.is_licensed)
const isDebugAccess = computed(() => entitlements.value.access_source === 'debug')
const showUpgradeEntry = computed(() => !hasFullAccess.value)
const activationView = computed(() => buildLicenseActivationViewState({
  hasLocalLicense: hasLocalLicense.value,
  isDebugAccess: isDebugAccess.value,
  formatTimestamp,
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

async function refreshAllStatus() {
  await refreshFeatureEntitlements()
  await checkLicenseStatus()
}

async function activateLicense() {
  if (!licenseKey.value.trim()) {
    error.value = '请输入 License'
    return
  }

  loading.value = true
  error.value = ''

  try {
    const result = await invoke<ActivationResult>('activate_license', {
      licenseKey: licenseKey.value.trim(),
    })

    if (result.success) {
      error.value = ''
      licenseKey.value = ''
      await refreshAllStatus()
      emit('activated')
      dialogOpen.value = false
    } else {
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

function openDialog() {
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
  } catch {
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
              <span class="label-text-alt text-base-content/50">复制设备 ID 给管理员签发 License。</span>
            </label>
          </div>

          <div v-if="!hasLocalLicense" class="form-control mb-4">
            <label class="label">
              <span class="label-text font-medium">License</span>
            </label>
            <textarea
              v-model="licenseKey"
              class="textarea textarea-bordered font-mono min-h-28"
              placeholder="粘贴管理员签发的 License"
            />
            <label class="label">
              <span class="label-text-alt text-base-content/50">License 与当前设备绑定，激活后保存在本地，无需联网续期。</span>
            </label>
          </div>

          <div v-else class="rounded-2xl border border-base-300 bg-base-200/50 p-4 mb-4">
            <h3 class="text-lg font-semibold">当前授权</h3>
            <p class="text-sm text-base-content/70 mt-2">
              {{ activationView.featureAccessSummary }}
            </p>
          </div>

          <div v-if="error" class="alert alert-error mb-4">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ error }}</span>
          </div>

          <div v-if="!hasLocalLicense" class="card-actions justify-center">
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
