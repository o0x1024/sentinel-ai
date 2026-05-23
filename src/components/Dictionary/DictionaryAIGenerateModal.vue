<template>
  <Teleport to="body">
    <div v-if="open" class="modal modal-open dictionary-modal" @click.self="$emit('cancel')">
      <div class="modal-box max-w-2xl dictionary-modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ isStructured ? 'AI生成规则' : 'AI生成字典' }}
        </h3>

        <form class="space-y-4" @submit.prevent="submit">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">目标字典</span>
              </label>
              <input class="input input-bordered" :value="dictionaryName" disabled>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">生成数量</span>
              </label>
              <input
                v-model.number="localCount"
                type="number"
                min="1"
                max="100"
                class="input input-bordered"
                required
              >
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ isStructured ? '规则需求' : '字典需求' }}</span>
            </label>
            <textarea
              v-model.trim="localPrompt"
              class="textarea textarea-bordered h-36"
              :placeholder="placeholder"
              required
            />
            <label class="label">
              <span class="label-text-alt text-base-content/70">
                AI 会按当前字典类型生成可直接导入的词条；规则类字典会包含 category、weight 和 metadata。
              </span>
            </label>
          </div>

          <div class="modal-action">
            <button type="button" class="btn" :disabled="generating" @click="$emit('cancel')">取消</button>
            <button type="submit" class="btn btn-primary" :disabled="generating || !localPrompt.trim()">
              {{ generating ? '生成中...' : '生成并写入' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'

const props = defineProps<{
  open: boolean
  dictionaryName: string
  dictionaryType: string
  isStructured: boolean
  generating: boolean
}>()

const emit = defineEmits<{
  (e: 'cancel'): void
  (e: 'generate', payload: { prompt: string; count: number }): void
}>()

const localPrompt = ref('')
const localCount = ref(10)

const placeholder = computed(() => {
  if (props.isStructured) {
    return '例如：生成 20 条 Spring Boot / Swagger / Actuator 暴露检测规则，包含路径、严重级别、标签和 matcher。'
  }
  if (props.dictionaryType === 'subdomain') {
    return '例如：生成适用于 SaaS 企业站点的常见子域名字典，覆盖登录、API、管理后台、监控和开发环境。'
  }
  if (props.dictionaryType === 'password') {
    return '例如：生成弱口令字典，贴近企业内部系统默认密码和年份组合。'
  }
  return '例如：生成面向 Web 资产发现的高价值字典项，避免重复和无意义占位内容。'
})

watch(
  () => props.open,
  open => {
    if (!open) {
      localPrompt.value = ''
      localCount.value = 10
    }
  }
)

function submit() {
  const count = Number.isFinite(localCount.value)
    ? Math.min(Math.max(Math.trunc(localCount.value), 1), 100)
    : 10
  emit('generate', {
    prompt: localPrompt.value.trim(),
    count,
  })
}
</script>

<style scoped>
.dictionary-modal {
  z-index: 80;
  align-items: flex-start;
  padding: 5rem 1rem 1.5rem;
}

.dictionary-modal-box {
  max-height: calc(100vh - 6.5rem);
  overflow-y: auto;
}
</style>
