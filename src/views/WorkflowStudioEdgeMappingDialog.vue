<template>
  <dialog :open="open" class="modal" @click.self="$emit('close')">
    <div class="modal-box max-w-xl">
      <h3 class="font-bold text-lg">边映射</h3>
      <p class="mt-2 text-sm text-base-content/70">{{ edgeLabel }}</p>

      <div class="mt-4 space-y-4">
        <label class="form-control">
          <span class="label-text text-sm">映射来源</span>
          <select v-model="localForm.source_scope" class="select select-bordered select-sm">
            <option value="output">上游输出</option>
            <option value="input">上游输入</option>
          </select>
        </label>

        <label class="form-control">
          <span class="label-text text-sm">来源路径</span>
          <input v-model="localForm.source_path" class="input input-bordered input-sm" placeholder="留空表示整个值，例如 output.response.data" />
          <span class="label-text-alt">支持点路径，如 `response.data.0.url`</span>
        </label>
        <div v-if="sourcePathOptions.length" class="space-y-2">
          <div class="text-xs font-medium text-base-content/70">上游可选路径</div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="option in sourcePathOptions"
              :key="option.path"
              class="btn btn-xs btn-outline"
              type="button"
              :title="option.description || option.label"
              @click="localForm.source_path = option.path"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <label class="form-control">
          <span class="label-text text-sm">目标路径</span>
          <input v-model="localForm.target_path" class="input input-bordered input-sm" placeholder="例如 prompt / content / metadata.title" />
          <span class="label-text-alt">必须填写当前节点参数路径，如 `prompt`、`query`、`metadata.title`</span>
        </label>
        <div v-if="targetPathOptions.length" class="space-y-2">
          <div class="text-xs font-medium text-base-content/70">目标节点参数</div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="option in targetPathOptions"
              :key="option.path"
              class="btn btn-xs btn-outline"
              type="button"
              :title="option.description || option.label"
              @click="localForm.target_path = option.path"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <p v-if="targetPathError" class="text-xs text-error">{{ targetPathError }}</p>

        <label class="form-control">
          <span class="label-text text-sm">合并策略</span>
          <select v-model="localForm.merge_mode" class="select select-bordered select-sm">
            <option value="replace">replace 覆盖</option>
            <option value="deep_merge">deep_merge 深合并</option>
            <option value="append">append 追加</option>
          </select>
        </label>
      </div>

      <div class="modal-action">
        <button class="btn btn-primary btn-sm" :disabled="Boolean(targetPathError)" @click="onSave">保存</button>
        <button class="btn btn-ghost btn-sm" @click="$emit('close')">取消</button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import type { EdgeMergeMode, EdgeSourceScope } from '@/types/workflow'
import type { EdgeMappingOption } from './workflowStudioEdgeMappingSupport'

const props = defineProps<{
  open: boolean
  edgeLabel: string
  sourcePathOptions: EdgeMappingOption[]
  targetPathOptions: EdgeMappingOption[]
  form: {
    source_scope: EdgeSourceScope
    source_path: string
    target_path: string
    merge_mode: EdgeMergeMode
  }
}>()

const emit = defineEmits<{
  close: []
  save: [value: { source_scope: EdgeSourceScope; source_path: string; target_path: string; merge_mode: EdgeMergeMode }]
}>()

const localForm = reactive({
  source_scope: 'output' as EdgeSourceScope,
  source_path: '',
  target_path: '',
  merge_mode: 'replace' as EdgeMergeMode,
})

watch(
  () => props.form,
  (next) => {
    localForm.source_scope = next.source_scope
    localForm.source_path = next.source_path
    localForm.target_path = next.target_path
    localForm.merge_mode = next.merge_mode
  },
  { immediate: true, deep: true },
)

const targetPathError = computed(() => {
  return localForm.target_path.trim() ? '' : '目标路径不能为空'
})

const onSave = () => {
  if (targetPathError.value) return
  emit('save', {
    source_scope: localForm.source_scope,
    source_path: localForm.source_path.trim(),
    target_path: localForm.target_path.trim(),
    merge_mode: localForm.merge_mode,
  })
}
</script>
