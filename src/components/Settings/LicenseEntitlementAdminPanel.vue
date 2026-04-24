<template>
  <div class="card bg-base-100 shadow-sm mb-6">
    <div class="card-body gap-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h3 class="card-title">
            <i class="fas fa-id-card-clip"></i>
            {{ t('settings.security.licenseAdmin.title') }}
          </h3>
          <p class="text-sm text-base-content/70 mt-1">
            {{ t('settings.security.licenseAdmin.description') }}
          </p>
        </div>
        <button
          class="btn btn-outline btn-sm"
          :disabled="loading || saving || refreshing"
          @click="loadPanelState"
        >
          <span
            v-if="loading || saving || refreshing"
            class="loading loading-spinner loading-xs"
          ></span>
          {{ t('settings.security.licenseAdmin.refreshStatus') }}
        </button>
      </div>

      <div class="alert alert-warning text-sm">
        <i class="fas fa-user-shield"></i>
        <span>{{ t('settings.security.licenseAdmin.adminOnlyHint') }}</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
          <div class="text-sm text-base-content/60">{{ t('settings.security.licenseAdmin.localLicense') }}</div>
          <div class="mt-2 flex items-center justify-between gap-3">
            <span class="font-semibold">{{ localLicenseLabel }}</span>
            <span class="badge" :class="hasLocalLicense ? 'badge-success' : 'badge-warning'">
              {{ hasLocalLicense ? t('settings.enabled') : t('settings.disabled') }}
            </span>
          </div>
        </div>

        <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
          <div class="text-sm text-base-content/60">{{ t('settings.security.licenseAdmin.currentToken') }}</div>
          <div class="mt-2 flex items-center justify-between gap-3">
            <span class="font-semibold">{{ tokenStatusLabel }}</span>
            <span class="badge" :class="tokenStatus.valid ? 'badge-success' : 'badge-warning'">
              {{ tokenStatus.valid ? t('settings.enabled') : t('settings.disabled') }}
            </span>
          </div>
          <p class="text-xs text-base-content/60 mt-2">{{ tokenStatusDetail }}</p>
        </div>

        <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
          <div class="text-sm text-base-content/60">{{ t('settings.security.licenseAdmin.refreshService') }}</div>
          <div class="mt-2 flex items-center justify-between gap-3">
            <span class="font-semibold">{{ refreshServiceLabel }}</span>
            <span class="badge" :class="refreshConfigured ? 'badge-success' : 'badge-ghost'">
              {{ refreshConfigured ? t('settings.enabled') : t('settings.disabled') }}
            </span>
          </div>
          <p class="text-xs text-base-content/60 mt-2">
            {{ refreshConfig.api_key_configured
              ? t('settings.security.licenseAdmin.apiKeyConfigured')
              : t('settings.security.licenseAdmin.apiKeyMissing')
            }}
          </p>
        </div>

        <div class="rounded-xl border border-base-300 bg-base-200/30 p-4">
          <div class="text-sm text-base-content/60">{{ t('settings.security.licenseAdmin.lastSync') }}</div>
          <div class="mt-2 font-semibold">{{ lastSyncLabel }}</div>
          <p class="text-xs text-base-content/60 mt-2">{{ refreshRuntimeLabel }}</p>
        </div>
      </div>

      <div v-if="errorMessage" class="alert alert-error">
        <i class="fas fa-circle-exclamation"></i>
        <span>{{ errorMessage }}</span>
      </div>

      <div v-if="successMessage" class="alert alert-success">
        <i class="fas fa-circle-check"></i>
        <span>{{ successMessage }}</span>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-3">
            <input
              v-model="refreshConfig.enabled"
              type="checkbox"
              class="toggle toggle-primary"
              :disabled="saving"
            />
            <span class="label-text">{{ t('settings.security.licenseAdmin.enableAutoRefresh') }}</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.security.licenseAdmin.timeout') }}</span>
          </label>
          <input
            v-model.number="refreshConfig.timeout_secs"
            type="number"
            min="5"
            max="120"
            class="input input-bordered"
            :disabled="saving"
          />
        </div>

        <div class="form-control lg:col-span-2">
          <label class="label">
            <span class="label-text">{{ t('settings.security.licenseAdmin.endpoint') }}</span>
          </label>
          <input
            v-model.trim="refreshConfig.endpoint"
            type="url"
            class="input input-bordered"
            :disabled="saving"
            placeholder="https://license.example.com/api/entitlements/refresh"
          />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.security.licenseAdmin.customerId') }}</span>
          </label>
          <input
            v-model.trim="refreshConfig.customer_id"
            type="text"
            class="input input-bordered"
            :disabled="saving"
            :placeholder="t('settings.security.licenseAdmin.customerIdPlaceholder')"
          />
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.security.licenseAdmin.apiKey') }}</span>
          </label>
          <input
            v-model.trim="refreshConfig.api_key"
            type="password"
            class="input input-bordered"
            :disabled="saving"
            :placeholder="t('settings.security.licenseAdmin.apiKeyPlaceholder')"
          />
        </div>
      </div>

      <div class="divider my-1">{{ t('settings.security.licenseAdmin.manualTokenTitle') }}</div>

      <div class="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_auto] gap-4 items-end">
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('settings.security.licenseAdmin.manualTokenField') }}</span>
          </label>
          <textarea
            v-model.trim="manualToken"
            class="textarea textarea-bordered min-h-28 font-mono text-sm"
            :disabled="manualTokenBusy"
            :placeholder="t('settings.security.licenseAdmin.manualTokenPlaceholder')"
          ></textarea>
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ t('settings.security.licenseAdmin.manualTokenHint') }}
            </span>
          </label>
        </div>

        <div class="flex gap-3 lg:flex-col">
          <button
            class="btn btn-secondary"
            :disabled="manualTokenBusy || !manualToken.length"
            @click="storeManualToken"
          >
            <span v-if="manualTokenBusy" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-shield-alt"></i>
            {{ t('settings.security.licenseAdmin.manualTokenStore') }}
          </button>
          <button
            class="btn btn-ghost"
            :disabled="manualTokenBusy || !tokenStatus.exists"
            @click="clearManualToken"
          >
            <i class="fas fa-trash-alt"></i>
            {{ t('settings.security.licenseAdmin.manualTokenClear') }}
          </button>
        </div>
      </div>

      <div class="card-actions justify-end">
        <button
          class="btn btn-outline"
          :disabled="refreshing || !canRefreshNow"
          @click="refreshNow"
        >
          <span v-if="refreshing" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-rotate-right"></i>
          {{ t('settings.security.licenseAdmin.refreshNow') }}
        </button>
        <button class="btn btn-primary" :disabled="saving" @click="saveConfig">
          <span v-if="saving" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-save"></i>
          {{ t('settings.security.licenseAdmin.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { getFeatureEntitlements, refreshFeatureEntitlements } from '@/services/featureEntitlements'
import {
  getEntitlementRefreshConfig,
  refreshEntitlementTokenFromServer,
  saveEntitlementRefreshConfig,
  type EntitlementRefreshConfig,
} from '@/services/entitlementRefresh'
import {
  clearEntitlementTokenFromAdmin,
  storeEntitlementTokenFromAdmin,
} from '@/services/entitlementTokenAdmin'
import {
  markEntitlementRefreshFailure,
  markEntitlementRefreshSuccess,
  useEntitlementRefreshRuntimeState,
} from '@/services/entitlementRefreshState'
import {
  getEntitlementTokenIssueMessage,
  refreshEntitlementTokenStatus,
  useEntitlementTokenStatusState,
} from '@/services/entitlementToken'

const { t } = useI18n()

const tokenStatus = useEntitlementTokenStatusState()
const refreshRuntime = useEntitlementRefreshRuntimeState()
const refreshConfig = ref<EntitlementRefreshConfig>({
  enabled: false,
  endpoint: '',
  api_key: '',
  customer_id: '',
  timeout_secs: 15,
  api_key_configured: false,
})
const hasLocalLicense = ref(false)
const saving = ref(false)
const refreshing = ref(false)
const loading = ref(false)
const errorMessage = ref('')
const successMessage = ref('')
const manualToken = ref('')
const manualTokenBusy = ref(false)

const refreshConfigured = computed(
  () => refreshConfig.value.enabled && refreshConfig.value.endpoint.trim().length > 0,
)
const canRefreshNow = computed(
  () => hasLocalLicense.value && refreshConfigured.value,
)
const localLicenseLabel = computed(() =>
  hasLocalLicense.value
    ? t('settings.security.licenseAdmin.localLicenseActive')
    : t('settings.security.licenseAdmin.localLicenseMissing'),
)
const tokenStatusLabel = computed(() =>
  tokenStatus.value.valid
    ? t('settings.security.licenseAdmin.tokenValid')
    : t('settings.security.licenseAdmin.tokenUnavailable'),
)
const tokenStatusDetail = computed(() => {
  if (tokenStatus.value.valid) {
    const expiresAt = formatTimestamp(tokenStatus.value.expires_at)
    return expiresAt
      ? t('settings.security.licenseAdmin.tokenExpiresAt', { expiresAt })
      : t('settings.security.licenseAdmin.tokenValidDetail')
  }

  return getEntitlementTokenIssueMessage(tokenStatus.value)
})
const refreshServiceLabel = computed(() =>
  refreshConfigured.value
    ? t('settings.security.licenseAdmin.refreshConfigured')
    : t('settings.security.licenseAdmin.refreshNotConfigured'),
)
const lastSyncLabel = computed(() => {
  if (refreshRuntime.value.last_success_at) {
    return formatTimestamp(refreshRuntime.value.last_success_at)
  }

  return t('settings.security.licenseAdmin.noSyncYet')
})
const refreshRuntimeLabel = computed(() => {
  if (refreshRuntime.value.last_error) {
    return refreshRuntime.value.last_error
  }

  return t('settings.security.licenseAdmin.runtimeIdle')
})

onMounted(async () => {
  await loadPanelState()
})

async function loadPanelState() {
  loading.value = true
  clearMessages()

  try {
    const entitlements = await getFeatureEntitlements()
    hasLocalLicense.value = entitlements.has_local_license

    const [config] = await Promise.all([
      getEntitlementRefreshConfig(),
      refreshEntitlementTokenStatus(),
    ])
    refreshConfig.value = config
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  saving.value = true
  clearMessages()

  try {
    refreshConfig.value = await saveEntitlementRefreshConfig({
      enabled: refreshConfig.value.enabled,
      endpoint: refreshConfig.value.endpoint,
      api_key: refreshConfig.value.api_key,
      customer_id: refreshConfig.value.customer_id,
      timeout_secs: refreshConfig.value.timeout_secs,
    })
    successMessage.value = t('settings.security.licenseAdmin.saveSuccess')
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    saving.value = false
  }
}

async function refreshNow() {
  refreshing.value = true
  clearMessages()

  try {
    const result = await refreshEntitlementTokenFromServer()
    if (result.success) {
      markEntitlementRefreshSuccess()
      successMessage.value = result.message
    } else {
      if (result.configured) {
        markEntitlementRefreshFailure({
          message: result.message,
          errorCode: result.error_code,
          retryAfterSecs: result.retry_after_secs,
        })
      }
      errorMessage.value = result.message
    }
    await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus(), loadConfigOnly()])
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    refreshing.value = false
  }
}

async function storeManualToken() {
  if (!manualToken.value.length) {
    return
  }

  manualTokenBusy.value = true
  clearMessages()

  try {
    const result = await storeEntitlementTokenFromAdmin(manualToken.value)
    if (!result.success) {
      errorMessage.value = result.message
      return
    }

    manualToken.value = ''
    successMessage.value = result.message
    await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus(), loadConfigOnly()])
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    manualTokenBusy.value = false
  }
}

async function clearManualToken() {
  manualTokenBusy.value = true
  clearMessages()

  try {
    const result = await clearEntitlementTokenFromAdmin()
    if (!result.success) {
      errorMessage.value = result.message
      return
    }

    manualToken.value = ''
    successMessage.value = result.message
    await Promise.all([refreshFeatureEntitlements(), refreshEntitlementTokenStatus(), loadConfigOnly()])
  } catch (error) {
    errorMessage.value = String(error)
  } finally {
    manualTokenBusy.value = false
  }
}

async function loadConfigOnly() {
  refreshConfig.value = await getEntitlementRefreshConfig()
}

function clearMessages() {
  errorMessage.value = ''
  successMessage.value = ''
}

function formatTimestamp(timestamp: number | null) {
  if (!timestamp) {
    return t('settings.security.licenseAdmin.notAvailable')
  }

  return new Date(timestamp * 1000).toLocaleString()
}
</script>
