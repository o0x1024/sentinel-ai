<template>
  <div v-if="open" class="modal modal-open">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg mb-4">
        {{ isEditing ? '编辑字典' : '创建字典' }}
      </h3>

      <form class="space-y-4" @submit.prevent="submit">
        <div class="form-control">
          <label class="label">
            <span class="label-text">字典名称</span>
          </label>
          <input
            v-model="localForm.name"
            type="text"
            class="input input-bordered"
            placeholder="请输入字典名称"
            required
          >
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">描述</span>
          </label>
          <textarea
            v-model="localForm.description"
            class="textarea textarea-bordered"
            placeholder="请输入字典描述"
          />
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">字典类型</span>
            </label>
            <select v-model="localForm.dictionary_type" class="select select-bordered" required>
              <option value="">选择类型</option>
              <option v-for="type in selectableDictionaryTypes" :key="type.value" :value="type.value">
                {{ type.label }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">服务类型</span>
            </label>
            <select v-model="localForm.service_type" class="select select-bordered">
              <option value="">选择服务类型（可选）</option>
              <option v-for="service in serviceTypes" :key="service.value" :value="service.value">
                {{ service.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text">启用字典</span>
            <input v-model="localForm.is_active" type="checkbox" class="toggle toggle-primary">
          </label>
        </div>

        <div class="modal-action">
          <button type="button" class="btn" @click="$emit('cancel')">取消</button>
          <button type="submit" class="btn btn-primary" :disabled="saving">
            {{ saving ? '保存中...' : '保存' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'

interface DictionaryFormValue {
  name: string
  description?: string
  dictionary_type: string
  service_type?: string
  is_active: boolean
}

interface OptionItem {
  value: string
  label: string
}

const props = defineProps<{
  open: boolean
  isEditing: boolean
  form: DictionaryFormValue
  dictionaryTypes: OptionItem[]
  serviceTypes: OptionItem[]
  saving: boolean
}>()

const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'save', value: DictionaryFormValue): void
}>()

const localForm = reactive<DictionaryFormValue>({
  name: '',
  description: '',
  dictionary_type: '',
  service_type: '',
  is_active: true,
})

const selectableDictionaryTypes = computed(() => props.dictionaryTypes.filter(type => type.value !== 'all'))

watch(
  () => [props.open, props.form],
  () => {
    localForm.name = props.form.name || ''
    localForm.description = props.form.description || ''
    localForm.dictionary_type = props.form.dictionary_type || ''
    localForm.service_type = props.form.service_type || ''
    localForm.is_active = props.form.is_active
  },
  { immediate: true, deep: true }
)

function submit() {
  emit('save', {
    name: localForm.name,
    description: localForm.description || '',
    dictionary_type: localForm.dictionary_type,
    service_type: localForm.service_type || '',
    is_active: localForm.is_active,
  })
}
</script>
