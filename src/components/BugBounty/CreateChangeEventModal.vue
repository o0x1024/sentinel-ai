<template>
  <div v-if="visible" class="modal modal-open">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.changeEvents.createEvent') }}</h3>
      
      <div class="space-y-4">
        <!-- Event Type -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.eventType') }} *</span>
          </label>
          <select v-model="form.event_type" class="select select-bordered">
            <option value="">{{ t('bugBounty.changeEvents.selectEventType') }}</option>
            <option value="asset_discovered">{{ t('bugBounty.changeEvents.types.assetDiscovered') }}</option>
            <option value="asset_removed">{{ t('bugBounty.changeEvents.types.assetRemoved') }}</option>
            <option value="asset_modified">{{ t('bugBounty.changeEvents.types.assetModified') }}</option>
            <option value="dns_change">{{ t('bugBounty.changeEvents.types.dnsChange') }}</option>
            <option value="certificate_change">{{ t('bugBounty.changeEvents.types.certificateChange') }}</option>
            <option value="technology_change">{{ t('bugBounty.changeEvents.types.technologyChange') }}</option>
            <option value="port_change">{{ t('bugBounty.changeEvents.types.portChange') }}</option>
            <option value="service_change">{{ t('bugBounty.changeEvents.types.serviceChange') }}</option>
            <option value="content_change">{{ t('bugBounty.changeEvents.types.contentChange') }}</option>
            <option value="api_change">{{ t('bugBounty.changeEvents.types.apiChange') }}</option>
            <option value="configuration_exposed">{{ t('bugBounty.changeEvents.types.configurationExposed') }}</option>
          </select>
        </div>

        <!-- Title -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.table.title') }} *</span>
          </label>
          <input 
            v-model="form.title" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.changeEvents.titlePlaceholder')"
          />
        </div>

        <!-- Asset ID -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.asset') }} *</span>
          </label>
          <input 
            v-model="form.asset_id" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.changeEvents.assetPlaceholder')"
          />
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ t('bugBounty.changeEvents.assetHint') }}
            </span>
          </label>
        </div>

        <!-- Program (Optional) -->
        <div class="form-control" v-if="programs.length > 0">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.program') }}</span>
          </label>
          <select v-model="form.program_id" class="select select-bordered">
            <option value="">{{ t('bugBounty.form.selectProgram') }}</option>
            <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
        </div>

        <!-- Severity -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.table.severity') }} *</span>
          </label>
          <select v-model="form.severity" class="select select-bordered">
            <option value="low">{{ t('bugBounty.severity.low') }}</option>
            <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
            <option value="high">{{ t('bugBounty.severity.high') }}</option>
            <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
          </select>
        </div>

        <!-- Description -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.description') }}</span>
          </label>
          <textarea 
            v-model="form.description" 
            class="textarea textarea-bordered h-24"
            :placeholder="t('bugBounty.changeEvents.descriptionPlaceholder')"
          ></textarea>
        </div>

        <!-- Detection Method -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.detectionMethod') }} *</span>
          </label>
          <select v-model="form.detection_method" class="select select-bordered">
            <option value="manual">{{ t('bugBounty.changeEvents.detectionMethods.manual') }}</option>
            <option value="automated">{{ t('bugBounty.changeEvents.detectionMethods.automated') }}</option>
            <option value="monitor">{{ t('bugBounty.changeEvents.detectionMethods.monitor') }}</option>
            <option value="plugin">{{ t('bugBounty.changeEvents.detectionMethods.plugin') }}</option>
          </select>
        </div>

        <div class="divider">{{ t('bugBounty.changeEvents.changeDetails') }}</div>

        <!-- Old Value -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.oldValue') }}</span>
          </label>
          <textarea 
            v-model="form.old_value" 
            class="textarea textarea-bordered h-20 font-mono text-sm"
            :placeholder="t('bugBounty.changeEvents.oldValuePlaceholder')"
          ></textarea>
        </div>

        <!-- New Value -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.newValue') }}</span>
          </label>
          <textarea 
            v-model="form.new_value" 
            class="textarea textarea-bordered h-20 font-mono text-sm"
            :placeholder="t('bugBounty.changeEvents.newValuePlaceholder')"
          ></textarea>
        </div>

        <!-- Affected Scope -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.changeEvents.affectedScope') }}</span>
          </label>
          <input 
            v-model="form.affected_scope" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.changeEvents.affectedScopePlaceholder')"
          />
        </div>

        <!-- Tags -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.tags') }}</span>
          </label>
          <input 
            v-model="form.tags" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.form.tagsPlaceholder')"
          />
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ t('bugBounty.form.tagsHint') }}
            </span>
          </label>
        </div>

        <!-- Auto Trigger -->
        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text">
              <i class="fas fa-bolt mr-2 text-warning"></i>
              {{ t('bugBounty.changeEvents.autoTriggerWorkflow') }}
            </span>
            <input type="checkbox" v-model="form.auto_trigger_enabled" class="checkbox checkbox-primary" />
          </label>
          <label class="label">
            <span class="label-text-alt text-base-content/60">
              {{ t('bugBounty.changeEvents.autoTriggerHint') }}
            </span>
          </label>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="$emit('close')">{{ t('common.cancel') }}</button>
        <button 
          class="btn btn-primary" 
          @click="submit" 
          :disabled="!isValid || submitting"
        >
          <span v-if="submitting" class="loading loading-spinner loading-sm mr-2"></span>
          {{ t('common.create') }}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" @click="$emit('close')"></div>
  </div>
</template>

<script setup lang="ts">
import { reactive, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  submitting: boolean
  programs: any[]
  selectedProgram?: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'submit', data: any): void
}>()

const form = reactive({
  event_type: '',
  title: '',
  asset_id: '',
  program_id: '',
  severity: 'medium',
  description: '',
  detection_method: 'manual',
  old_value: '',
  new_value: '',
  affected_scope: '',
  tags: '',
  auto_trigger_enabled: true,
})

const isValid = computed(() => {
  return form.event_type && form.title && form.asset_id && form.detection_method
})

const submit = () => {
  if (!isValid.value) return

  const tagsArray = form.tags
    .split(',')
    .map(t => t.trim())
    .filter(t => t.length > 0)

  const data = {
    event_type: form.event_type,
    title: form.title,
    asset_id: form.asset_id,
    program_id: form.program_id || null,
    severity: form.severity,
    description: form.description || null,
    detection_method: form.detection_method,
    old_value: form.old_value || null,
    new_value: form.new_value || null,
    affected_scope: form.affected_scope || null,
    tags: tagsArray.length > 0 ? tagsArray : null,
    auto_trigger_enabled: form.auto_trigger_enabled,
  }

  emit('submit', data)
}

const resetForm = () => {
  form.event_type = ''
  form.title = ''
  form.asset_id = ''
  form.program_id = ''
  form.severity = 'medium'
  form.description = ''
  form.detection_method = 'manual'
  form.old_value = ''
  form.new_value = ''
  form.affected_scope = ''
  form.tags = ''
  form.auto_trigger_enabled = true
}

// Auto-fill program if selected
watch(() => props.selectedProgram, (newVal) => {
  if (newVal && props.visible) {
    form.program_id = newVal.id
  }
}, { immediate: true })

watch(() => props.visible, (val) => {
  if (val) {
    if (props.selectedProgram) {
      form.program_id = props.selectedProgram.id
    }
  } else {
    resetForm()
  }
})
</script>
