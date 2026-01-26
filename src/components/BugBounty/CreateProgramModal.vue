<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal modal-open">
        <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg mb-4">
        {{ editMode ? t('bugBounty.editProgram') : t('bugBounty.createProgram') }}
      </h3>
      
      <div class="space-y-4">
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.programName') }} *</span>
          </label>
          <input 
            v-model="form.name" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.form.programNamePlaceholder')"
          />
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.organization') }} *</span>
          </label>
          <input 
            v-model="form.organization" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.form.organizationPlaceholder')"
          />
        </div>
        
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.platform') }}</span>
            </label>
            <select v-model="form.platform" class="select select-bordered">
              <option value="private">{{ t('bugBounty.platforms.private') }}</option>
              <option value="hacker_one">HackerOne</option>
              <option value="bugcrowd">Bugcrowd</option>
              <option value="src">{{ t('bugBounty.platforms.src') }}</option>
              <option value="other">{{ t('bugBounty.platforms.other') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.url') }}</span>
            </label>
            <input 
              v-model="form.url" 
              type="text" 
              class="input input-bordered"
              placeholder="https://..."
            />
          </div>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.description') }}</span>
          </label>
          <textarea 
            v-model="form.description" 
            class="textarea textarea-bordered"
            rows="3"
            :placeholder="t('bugBounty.form.descriptionPlaceholder')"
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
          {{ editMode ? t('common.save') : t('common.create') }}
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
  program?: any // Program to edit (if provided, enters edit mode)
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'submit', data: any): void
}>()

const form = reactive({
  name: '',
  organization: '',
  platform: 'private',
  url: '',
  description: '',
})

const editMode = computed(() => !!props.program)

const isValid = computed(() => form.name && form.organization)

const submit = () => {
  if (!isValid.value) return
  const data = { ...form }
  if (editMode.value && props.program) {
    // Include ID for update
    emit('submit', { id: props.program.id, ...data })
  } else {
    emit('submit', data)
  }
}

const resetForm = () => {
  form.name = ''
  form.organization = ''
  form.platform = 'private'
  form.url = ''
  form.description = ''
}

const loadProgramData = () => {
  if (props.program) {
    form.name = props.program.name || ''
    form.organization = props.program.organization || ''
    form.platform = props.program.platform || 'private'
    form.url = props.program.url || ''
    form.description = props.program.description || ''
  } else {
    resetForm()
  }
}

watch(() => props.visible, (val) => {
  if (val) {
    loadProgramData()
  } else {
    resetForm()
  }
})

// Load data when program prop changes (for edit mode)
watch(() => props.program, () => {
  if (props.visible && props.program) {
    loadProgramData()
  }
}, { deep: true })
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
