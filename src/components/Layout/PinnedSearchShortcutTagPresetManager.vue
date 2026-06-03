<template>
  <div class="space-y-2">
    <div class="flex flex-col gap-2 sm:flex-row">
      <label class="input input-sm input-bordered flex flex-1 items-center gap-2">
        <i class="fas fa-tag text-base-content/40"></i>
        <input
          v-model="presetInput"
          type="text"
          class="grow"
          :placeholder="placeholder"
          @keydown.enter.prevent="submitPreset"
        />
      </label>
      <button class="btn btn-sm btn-ghost" :disabled="!normalizedPresetInput" @click="submitPreset">
        添加预设
      </button>
    </div>

    <div v-if="tagPresets.length > 0" class="flex flex-wrap gap-2">
      <div
        v-for="tag in tagPresets"
        :key="tag"
        class="flex items-center gap-1 rounded-full border px-2 py-1 transition-colors"
        :class="activeTags.includes(tag)
          ? 'border-primary bg-primary/10 text-primary'
          : 'border-base-300 bg-base-100 text-base-content/70'"
      >
        <button class="text-xs font-medium" @click="$emit('toggle', tag)">
          #{{ tag }}
        </button>
        <template v-if="enablePresetManagement && removableTags.includes(tag)">
          <button
            class="text-[10px] text-base-content/45 transition-colors hover:text-primary"
            :disabled="!canMoveTag(tag, -1)"
            @click.stop="$emit('reorder', { activeTag: tag, targetTag: getMoveTarget(tag, -1) })"
          >
            <i class="fas fa-arrow-up"></i>
          </button>
          <button
            class="text-[10px] text-base-content/45 transition-colors hover:text-primary"
            :disabled="!canMoveTag(tag, 1)"
            @click.stop="$emit('reorder', { activeTag: tag, targetTag: getMoveTarget(tag, 1) })"
          >
            <i class="fas fa-arrow-down"></i>
          </button>
          <button
            class="text-[10px] text-base-content/45 transition-colors hover:text-primary"
            @click.stop="startEditing(tag)"
          >
            <i class="fas fa-pen"></i>
          </button>
          <button
            class="text-[10px] text-base-content/45 transition-colors hover:text-error"
            @click.stop="$emit('remove', { tag, replacementTag: migrationTargets[tag] || '' })"
          >
            <i class="fas fa-xmark"></i>
          </button>
        </template>
      </div>
    </div>

    <div v-if="enablePresetManagement && editingTag" class="rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
      <div class="text-xs font-medium text-base-content/55">编辑标签预设</div>
      <div class="mt-2 flex flex-col gap-2 lg:flex-row lg:items-center">
        <label class="input input-sm input-bordered flex flex-1 items-center gap-2">
          <i class="fas fa-pen text-base-content/40"></i>
          <input
            v-model="editingValue"
            type="text"
            class="grow"
            placeholder="新的标签名称"
            @keydown.enter.prevent="submitRename"
          />
        </label>
        <select v-model="migrationTargets[editingTag]" class="select select-sm select-bordered lg:w-48">
          <option value="">删除时不迁移</option>
          <option
            v-for="candidate in removableMigrationTargets(editingTag)"
            :key="`${editingTag}-${candidate}`"
            :value="candidate"
          >
            迁移到 #{{ candidate }}
          </option>
        </select>
        <div class="flex items-center gap-2">
          <button class="btn btn-sm btn-ghost" @click="cancelEditing">取消</button>
          <button class="btn btn-sm btn-primary" :disabled="!normalizedEditingValue" @click="submitRename">
            保存
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{
  tagPresets: string[]
  activeTags?: string[]
  removableTags?: string[]
  placeholder?: string
  enablePresetManagement?: boolean
}>(), {
  activeTags: () => [],
  removableTags: () => [],
  placeholder: '新增标签预设',
  enablePresetManagement: true,
})

const presetInput = ref('')
const normalizedPresetInput = computed(() => presetInput.value.trim())
const editingTag = ref('')
const editingValue = ref('')
const normalizedEditingValue = computed(() => editingValue.value.trim())
const migrationTargets = ref<Record<string, string>>({})

const emit = defineEmits<{
  toggle: [tag: string]
  add: [tag: string]
  remove: [payload: { tag: string; replacementTag?: string }]
  rename: [payload: { currentTag: string; nextTag: string }]
  reorder: [payload: { activeTag: string; targetTag: string }]
}>()

const submitPreset = () => {
  if (!normalizedPresetInput.value) {
    return
  }

  emit('add', normalizedPresetInput.value)
  presetInput.value = ''
}

const removableMigrationTargets = (tag: string) =>
  props.tagPresets.filter(candidate => candidate !== tag)

const canMoveTag = (tag: string, direction: 1 | -1) => {
  const removableIndex = props.removableTags.indexOf(tag)
  if (removableIndex < 0) {
    return false
  }

  return removableIndex + direction >= 0 && removableIndex + direction < props.removableTags.length
}

const getMoveTarget = (tag: string, direction: 1 | -1) => {
  const removableIndex = props.removableTags.indexOf(tag)
  if (removableIndex < 0) {
    return tag
  }

  return props.removableTags[removableIndex + direction] || tag
}

const startEditing = (tag: string) => {
  editingTag.value = tag
  editingValue.value = tag
}

const cancelEditing = () => {
  editingTag.value = ''
  editingValue.value = ''
}

const submitRename = () => {
  if (!editingTag.value || !normalizedEditingValue.value) {
    return
  }

  emit('rename', {
    currentTag: editingTag.value,
    nextTag: normalizedEditingValue.value,
  })
  cancelEditing()
}
</script>
