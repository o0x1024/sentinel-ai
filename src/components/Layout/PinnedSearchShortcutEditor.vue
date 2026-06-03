<template>
  <Teleport to="body">
    <div v-if="visible && shortcut" class="fixed inset-0 z-[1250]">
      <button
        class="absolute inset-0 bg-base-content/25 backdrop-blur-sm"
        aria-label="Close pinned shortcut editor"
        @click="$emit('close')"
      ></button>

      <div class="relative mx-auto mt-[14vh] w-[min(32rem,calc(100vw-2rem))]">
        <div class="rounded-3xl border border-base-300 bg-base-100 shadow-2xl">
          <div class="border-b border-base-300/70 px-5 py-4">
            <div class="flex items-start gap-3">
              <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                <i :class="shortcut.icon"></i>
              </div>
              <div class="min-w-0">
                <div class="text-lg font-semibold">编辑固定快捷入口</div>
                <p class="mt-1 text-sm text-base-content/60">{{ shortcut.title }}</p>
              </div>
            </div>
          </div>

          <form class="space-y-4 px-5 py-5" @submit.prevent="handleSubmit">
            <label class="form-control">
              <span class="label-text text-sm text-base-content/70">显示标题</span>
              <input
                v-model="formTitle"
                type="text"
                class="input input-bordered"
                :placeholder="defaultTitlePlaceholder"
              />
              <span class="label-text-alt mt-1 text-base-content/45">
                默认标题：{{ defaultTitle }}
              </span>
            </label>

            <label class="form-control">
              <span class="label-text text-sm text-base-content/70">备注</span>
              <textarea
                v-model="formDescription"
                class="textarea textarea-bordered min-h-24"
                :placeholder="defaultDescriptionPlaceholder"
              ></textarea>
              <span class="label-text-alt mt-1 text-base-content/45">
                默认备注：{{ defaultDescription || '无' }}
              </span>
            </label>

            <label class="form-control">
              <span class="label-text text-sm text-base-content/70">标签</span>
              <input
                v-model="formTags"
                type="text"
                class="input input-bordered"
                placeholder="例如：常用, 巡检, 漏洞"
              />
              <span class="label-text-alt mt-1 text-base-content/45">
                用逗号分隔多个标签，仅作用于当前固定快捷入口
              </span>
            </label>

            <PinnedSearchShortcutTagPresetManager
              :tag-presets="tagPresets"
              :active-tags="activeTags"
              :removable-tags="customTagPresets"
              :enable-preset-management="false"
              placeholder="新增自定义标签预设"
              @toggle="togglePresetTag"
              @add="addPresetTag"
            />

            <div class="flex items-center justify-between gap-3">
              <button type="button" class="btn btn-ghost btn-sm" @click="resetToDefault">
                恢复默认
              </button>
              <div class="flex items-center gap-2">
                <button type="button" class="btn btn-ghost" @click="$emit('close')">取消</button>
                <button type="submit" class="btn btn-primary">保存</button>
              </div>
            </div>
          </form>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { usePinnedSearchShortcutTagPresets } from '@/composables/usePinnedSearchShortcutTagPresets'
import type { GlobalSearchEntry } from '@/services/globalSearch'
import {
  buildPinnedSearchShortcutTagPresets,
  normalizePinnedSearchShortcutTags,
  togglePinnedSearchShortcutTag,
} from '@/services/pinnedSearchShortcutTags'
import PinnedSearchShortcutTagPresetManager from './PinnedSearchShortcutTagPresetManager.vue'

const props = defineProps<{
  visible: boolean
  shortcut: GlobalSearchEntry | null
  customTitle?: string
  customDescription?: string
  customTags?: string[]
  defaultTitle?: string
  defaultDescription?: string
}>()

const emit = defineEmits<{
  close: []
  save: [payload: { id: string; customTitle: string; customDescription: string; customTags: string[] }]
}>()

const formTitle = ref('')
const formDescription = ref('')
const formTags = ref('')
const { customTagPresets, addCustomTagPreset } = usePinnedSearchShortcutTagPresets()
const defaultTitle = computed(() => props.defaultTitle?.trim() || props.shortcut?.title || '')
const defaultDescription = computed(() => props.defaultDescription?.trim() || props.shortcut?.description || '')
const defaultTitlePlaceholder = computed(() => defaultTitle.value || '留空则使用默认标题')
const defaultDescriptionPlaceholder = computed(() => defaultDescription.value || '留空则使用默认描述')
const activeTags = computed(() => normalizePinnedSearchShortcutTags(formTags.value.split(/[，,]/)))
const tagPresets = computed(() =>
  buildPinnedSearchShortcutTagPresets({
    customTags: customTagPresets.value,
    dynamicTags: [...(props.customTags || []), ...activeTags.value],
  }),
)

watch(
  () => [props.shortcut?.id, props.customTitle, props.customDescription, props.customTags?.join('|')],
  () => {
    formTitle.value = props.customTitle?.trim() || ''
    formDescription.value = props.customDescription?.trim() || ''
    formTags.value = props.customTags?.join(', ') || ''
  },
  { immediate: true },
)

const resetToDefault = () => {
  formTitle.value = ''
  formDescription.value = ''
  formTags.value = ''
}

const togglePresetTag = (tag: string) => {
  formTags.value = togglePinnedSearchShortcutTag(activeTags.value, tag).join(', ')
}

const addPresetTag = (tag: string) => {
  addCustomTagPreset(tag)

  if (activeTags.value.includes(tag)) {
    return
  }

  formTags.value = [...activeTags.value, tag].join(', ')
}

const handleSubmit = () => {
  if (!props.shortcut) {
    return
  }

  const normalizedTitle = formTitle.value.trim()
  const normalizedDescription = formDescription.value.trim()
  const normalizedTags = activeTags.value

  emit('save', {
    id: props.shortcut.id,
    customTitle: normalizedTitle && normalizedTitle !== defaultTitle.value ? normalizedTitle : '',
    customDescription: normalizedDescription && normalizedDescription !== defaultDescription.value
      ? normalizedDescription
      : '',
    customTags: normalizedTags,
  })
}
</script>
