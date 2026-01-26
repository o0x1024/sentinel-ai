<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal modal-open">
        <div class="modal-box max-w-3xl">
      <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.createSubmission') }}</h3>
      
      <div class="space-y-4">
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.program') }} *</span>
            </label>
            <select v-model="form.program_id" class="select select-bordered" @change="$emit('program-change', form.program_id)">
              <option value="">{{ t('bugBounty.form.selectProgram') }}</option>
              <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.finding') }} *</span>
            </label>
            <select v-model="form.finding_id" class="select select-bordered" :disabled="!form.program_id">
              <option value="">{{ t('bugBounty.form.selectFinding') }}</option>
              <option v-for="f in programFindings" :key="f.id" :value="f.id">{{ f.title }}</option>
            </select>
          </div>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.title') }} *</span>
          </label>
          <input 
            v-model="form.title" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.form.submissionTitlePlaceholder')"
          />
        </div>
        
        <div class="grid grid-cols-3 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.type') }} *</span>
            </label>
            <select v-model="form.vulnerability_type" class="select select-bordered">
              <option value="xss">XSS</option>
              <option value="sqli">SQL Injection</option>
              <option value="ssrf">SSRF</option>
              <option value="idor">IDOR</option>
              <option value="rce">RCE</option>
              <option value="auth_bypass">Auth Bypass</option>
              <option value="info_disclosure">Info Disclosure</option>
              <option value="other">Other</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.severity') }}</span>
            </label>
            <select v-model="form.severity" class="select select-bordered">
              <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
              <option value="high">{{ t('bugBounty.severity.high') }}</option>
              <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
              <option value="low">{{ t('bugBounty.severity.low') }}</option>
              <option value="info">{{ t('bugBounty.severity.info') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.cvssScore') }}</span>
            </label>
            <input 
              v-model.number="form.cvss_score" 
              type="number" 
              step="0.1"
              min="0"
              max="10"
              class="input input-bordered"
              placeholder="0.0 - 10.0"
            />
          </div>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.description') }} *</span>
          </label>
          <textarea 
            v-model="form.description" 
            class="textarea textarea-bordered"
            rows="4"
            :placeholder="t('bugBounty.form.submissionDescriptionPlaceholder')"
          ></textarea>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.impact') }} *</span>
          </label>
          <textarea 
            v-model="form.impact" 
            class="textarea textarea-bordered"
            rows="2"
            :placeholder="t('bugBounty.form.impactPlaceholder')"
          ></textarea>
        </div>
      </div>
      
      <div class="modal-action">
        <button @click="$emit('close')" class="btn">
          {{ t('common.cancel') }}
        </button>
        <button 
          @click="submit" 
          class="btn btn-primary" 
          :disabled="!isValid || submitting"
        >
          <span v-if="submitting" class="loading loading-spinner loading-sm mr-2"></span>
          {{ t('common.create') }}
        </button>
      </div>
    </div>
  </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  submitting: boolean
  programs: any[]
  programFindings: any[]
  initialData?: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'submit', data: any): void
  (e: 'program-change', programId: string): void
}>()

const form = reactive({
  program_id: '',
  finding_id: '',
  title: '',
  vulnerability_type: 'xss',
  severity: 'medium',
  cvss_score: null as number | null,
  description: '',
  impact: '',
})

const isValid = computed(() => 
  form.program_id && form.finding_id && form.title && 
  form.vulnerability_type && form.description && form.impact
)

const submit = () => {
  if (!isValid.value) return
  emit('submit', { ...form })
}

watch(() => props.visible, (val) => {
  if (!val) {
    form.program_id = ''
    form.finding_id = ''
    form.title = ''
    form.vulnerability_type = 'xss'
    form.severity = 'medium'
    form.cvss_score = null
    form.description = ''
    form.impact = ''
  }
})

watch(() => props.initialData, (data) => {
  if (data) {
    form.program_id = data.program_id || ''
    form.finding_id = data.finding_id || ''
    form.title = data.title || ''
    form.vulnerability_type = data.vulnerability_type || 'xss'
    form.severity = data.severity || 'medium'
    form.cvss_score = data.cvss_score || null
    form.description = data.description || ''
    form.impact = data.impact || ''
  }
}, { immediate: true })
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .modal-box,
.modal-leave-active .modal-box {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.modal-enter-from .modal-box,
.modal-leave-to .modal-box {
  transform: scale(0.95);
  opacity: 0;
}
</style>
