<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'

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

const licenseInfo = ref<LicenseInfo | null>(null)
const licenseKey = ref('')
const loading = ref(false)
const error = ref('')
const copied = ref(false)

const needsActivation = computed(() => licenseInfo.value?.needs_activation ?? false)

onMounted(async () => {
  await checkLicenseStatus()
})

async function checkLicenseStatus() {
  try {
    licenseInfo.value = await invoke<LicenseInfo>('get_license_info')
    if (licenseInfo.value.is_licensed) {
      emit('activated')
    }
  } catch (e) {
    console.error('Failed to check license status:', e)
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
      emit('activated')
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
</script>

<template>
  <div v-if="needsActivation" class="fixed inset-0 z-[9999] flex items-center justify-center bg-base-300/95 backdrop-blur-sm">
    <div class="card bg-base-100 shadow-2xl w-full max-w-md mx-4">
      <div class="card-body">
        <!-- Header -->
        <div class="text-center mb-6">
          <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
            <i class="fas fa-key text-3xl text-primary"></i>
          </div>
          <h2 class="card-title justify-center text-2xl">{{ t('license.title') }}</h2>
          <p class="text-base-content/60 mt-2">{{ t('license.subtitle') }}</p>
        </div>

        <!-- Machine ID -->
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

        <!-- License Key Input -->
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

        <!-- Error Message -->
        <div v-if="error" class="alert alert-error mb-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ error }}</span>
        </div>

        <!-- Activate Button -->
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

        <!-- Help Link -->
        <div class="text-center mt-4">
          <a href="mailto:support@example.com" class="link link-hover text-sm text-base-content/60">
            <i class="fas fa-question-circle mr-1"></i>
            {{ t('license.needHelp') }}
          </a>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backdrop-blur-sm {
  backdrop-filter: blur(4px);
}
</style>
