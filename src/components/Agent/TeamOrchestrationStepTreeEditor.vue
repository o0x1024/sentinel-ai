<template>
  <div class="flow-editor space-y-3" :class="depth > 0 ? 'pl-3 border-l-2 border-base-300/70' : ''">
    <div class="flex items-center justify-between gap-2">
      <div class="text-xs font-semibold" :class="depth > 0 ? 'text-base-content/70' : 'text-base-content'">
        {{ depth > 0 ? `子流程层级 L${depth}` : '流程画布' }}
      </div>
      <div class="flex items-center gap-1">
        <button class="btn btn-xs btn-outline" @click="handleAddStep('agent')">
          <i class="fas fa-user mr-1"></i> Agent
        </button>
        <button class="btn btn-xs btn-outline" @click="handleAddStep('serial')">
          <i class="fas fa-list-ol mr-1"></i> 串行
        </button>
        <button class="btn btn-xs btn-outline" @click="handleAddStep('parallel')">
          <i class="fas fa-code-branch mr-1"></i> 并行
        </button>
      </div>
    </div>

    <div class="text-[11px] text-base-content/55 rounded border border-dashed border-base-300 px-2 py-1.5">
      拖拽节点可调整顺序；拖到“嵌入区域”可变成子流程。点击“配置节点”展开详细参数。
    </div>

    <div v-if="localSteps.length === 0" class="text-xs text-base-content/50 rounded border border-dashed border-base-300 p-3 bg-base-100/40">
      当前层暂无节点，请用右上角按钮添加。
    </div>

    <div v-else class="space-y-2">
      <div
        v-for="(step, index) in localSteps"
        :key="`step-${depth}-${step.id}-${index}`"
        class="flow-step"
        draggable="true"
        @dragstart="handleDragStart(index, $event)"
        @dragend="handleDragEnd"
      >
        <div
          class="h-2 rounded transition-colors"
          :class="dragOverIndex === index && dragOverMode === 'before' ? 'bg-primary/40' : 'bg-transparent'"
          @dragover.prevent="handleDragOverZone(index, 'before', $event)"
          @drop.prevent="handleDropZone(index, 'before', $event)"
        />

        <div
          class="rounded-xl border p-2.5 space-y-2 transition-colors shadow-sm bg-base-100"
          :class="[
            stepTypeCardClass(step.type),
            dragOverIndex === index ? 'ring-1 ring-primary/40' : ''
          ]"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="flex items-center gap-2 min-w-0">
              <span class="cursor-grab text-base-content/45" title="拖拽移动">
                <i class="fas fa-grip-vertical"></i>
              </span>
              <span class="badge badge-xs" :class="stepTypeBadgeClass(step.type)">
                <i :class="`${stepTypeIcon(step.type)} mr-1`"></i>{{ stepTypeLabel(step.type) }}
              </span>
              <span class="text-[11px] text-base-content/55">#{{ buildPathLabel(index) }}</span>
              <span class="text-xs font-medium truncate">{{ step.name?.trim() || step.id }}</span>
            </div>
            <div class="flex items-center gap-1">
              <button
                class="btn btn-xs btn-ghost"
                :disabled="depth <= 0"
                title="提升一层"
                @click="handlePromoteStep(index)"
              >
                <i class="fas fa-arrow-up-right-from-square"></i>
              </button>
              <button
                class="btn btn-xs btn-ghost"
                :disabled="index <= 0"
                title="嵌入前一个节点"
                @click="handleNestStep(index)"
              >
                <i class="fas fa-arrow-down"></i>
              </button>
              <button class="btn btn-xs btn-ghost" :disabled="index === 0" @click="handleMoveStep(index, -1)">
                <i class="fas fa-arrow-up"></i>
              </button>
              <button class="btn btn-xs btn-ghost" :disabled="index >= localSteps.length - 1" @click="handleMoveStep(index, 1)">
                <i class="fas fa-arrow-down"></i>
              </button>
              <button class="btn btn-xs btn-error btn-outline" @click="handleRemoveStep(index)">
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>

          <div class="text-[11px] text-base-content/65 bg-base-200/60 rounded px-2 py-1">
            <span class="font-medium">执行摘要：</span>
            <span v-if="step.type === 'agent'">
              成员={{ step.member || '未绑定' }}，phase={{ step.phase || '-' }}，retry={{ step.retry?.max_attempts ?? 1 }}/{{ step.retry?.backoff_ms ?? 800 }}ms
            </span>
            <span v-else>
              {{ step.type === 'parallel' ? '并行分支' : '串行子流程' }}，children={{ step.children?.length || 0 }}
            </span>
          </div>

          <details class="rounded border border-base-300 bg-base-50/70 p-2" :open="depth === 0">
            <summary class="cursor-pointer text-xs font-semibold text-base-content/75">配置节点</summary>
            <div class="mt-2 space-y-2">
              <div class="grid grid-cols-1 gap-2 md:grid-cols-3">
                <label class="form-control">
                  <div class="label py-0.5"><span class="label-text text-[11px]">Step ID</span></div>
                  <input
                    v-model="step.id"
                    type="text"
                    class="input input-bordered input-xs"
                    placeholder="step_id"
                  >
                </label>
                <label class="form-control">
                  <div class="label py-0.5"><span class="label-text text-[11px]">类型</span></div>
                  <select
                    v-model="step.type"
                    class="select select-bordered select-xs"
                    @change="handleStepTypeChanged(step)"
                  >
                    <option value="agent">agent</option>
                    <option value="serial">serial</option>
                    <option value="parallel">parallel</option>
                  </select>
                </label>
                <label class="form-control">
                  <div class="label py-0.5"><span class="label-text text-[11px]">Name</span></div>
                  <input
                    v-model="step.name"
                    type="text"
                    class="input input-bordered input-xs"
                    placeholder="可读名称（可选）"
                  >
                </label>
              </div>

              <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
                <label class="form-control">
                  <div class="label py-0.5"><span class="label-text text-[11px]">Phase</span></div>
                  <input
                    v-model="step.phase"
                    type="text"
                    class="input input-bordered input-xs"
                    placeholder="phase（可选）"
                  >
                </label>
              </div>

              <label class="form-control">
                <div class="label py-0.5"><span class="label-text text-[11px]">Instruction</span></div>
                <textarea
                  v-model="step.instruction"
                  class="textarea textarea-bordered w-full min-h-[64px] text-xs leading-5"
                  placeholder="指令（可选）"
                ></textarea>
              </label>

              <div v-if="step.type === 'agent'" class="space-y-2">
                <label class="form-control">
                  <div class="label py-0.5"><span class="label-text text-[11px]">执行成员（member）</span></div>
                  <select
                    v-model="step.member"
                    class="select select-bordered select-xs w-full"
                  >
                    <option value="">选择成员</option>
                    <option v-for="memberName in memberOptions" :key="`member-option-${memberName}`" :value="memberName">
                      {{ memberName }}
                    </option>
                  </select>
                </label>
                <div class="grid grid-cols-2 gap-2">
                  <label class="form-control">
                    <div class="label py-0.5"><span class="label-text text-[11px]">max_attempts</span></div>
                    <input
                      :value="step.retry?.max_attempts ?? 1"
                      type="number"
                      min="1"
                      class="input input-bordered input-xs"
                      @change="handleStepRetryChange(step, 'max_attempts', $event)"
                    >
                  </label>
                  <label class="form-control">
                    <div class="label py-0.5"><span class="label-text text-[11px]">backoff_ms</span></div>
                    <input
                      :value="step.retry?.backoff_ms ?? 800"
                      type="number"
                      min="100"
                      class="input input-bordered input-xs"
                      @change="handleStepRetryChange(step, 'backoff_ms', $event)"
                    >
                  </label>
                </div>
              </div>
            </div>
          </details>

          <div
            class="rounded border border-dashed border-base-300 px-2 py-1 text-[11px] text-base-content/50 transition-colors"
            :class="dragOverIndex === index && dragOverMode === 'inside' ? 'border-primary bg-primary/10 text-primary' : ''"
            @dragover.prevent="handleDragOverZone(index, 'inside', $event)"
            @drop.prevent="handleDropZone(index, 'inside', $event)"
          >
            拖到此处可嵌入为 {{ step.type === 'parallel' ? '并行' : '串行' }} 子节点
          </div>

          <div
            v-if="step.type !== 'agent'"
            class="rounded-lg border border-base-300/80 p-2"
            :class="step.type === 'parallel' ? 'bg-info/5' : 'bg-success/5'"
          >
            <div class="text-[11px] font-semibold mb-1" :class="step.type === 'parallel' ? 'text-info' : 'text-success'">
              {{ containerHint(step.type) }}
            </div>
            <TeamOrchestrationStepTreeEditor
              :steps="step.children || []"
              :member-options="memberOptions"
              :depth="depth + 1"
              :path-prefix="buildPath(index)"
              @update:steps="(children) => { step.children = children }"
              @dirty="handleChildDirty"
              @promote-step="(path) => emit('promote-step', path)"
              @nest-step="(path) => emit('nest-step', path)"
              @move-step="(payload) => emit('move-step', payload)"
            />
          </div>
        </div>

        <div v-if="index < localSteps.length - 1" class="flex items-center justify-center py-0.5 text-base-content/35">
          <i class="fas fa-arrow-down text-xs"></i>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

defineOptions({
  name: 'TeamOrchestrationStepTreeEditor',
})

type TeamOrchestrationStepType = 'agent' | 'serial' | 'parallel'

interface TeamOrchestrationRetry {
  max_attempts?: number
  backoff_ms?: number
}

interface TeamOrchestrationStep {
  id: string
  type: TeamOrchestrationStepType
  name?: string
  member?: string
  phase?: string
  instruction?: string
  retry?: TeamOrchestrationRetry
  children?: TeamOrchestrationStep[]
}

interface TeamStepMovePayload {
  sourcePath: number[]
  targetPath: number[]
  mode: 'before' | 'inside'
}

const props = withDefaults(defineProps<{
  steps: TeamOrchestrationStep[]
  memberOptions: string[]
  depth?: number
  pathPrefix?: number[]
}>(), {
  depth: 0,
  pathPrefix: () => [],
})

const emit = defineEmits<{
  (e: 'update:steps', steps: TeamOrchestrationStep[]): void
  (e: 'dirty'): void
  (e: 'promote-step', path: number[]): void
  (e: 'nest-step', path: number[]): void
  (e: 'move-step', payload: TeamStepMovePayload): void
}>()

const localSteps = ref<TeamOrchestrationStep[]>([])
const syncFromPropsInProgress = ref(false)
const draggingIndex = ref<number | null>(null)
const dragOverIndex = ref<number | null>(null)
const dragOverMode = ref<'before' | 'inside' | null>(null)

const deepClone = <T>(value: T): T => JSON.parse(JSON.stringify(value))

const generateStepId = () => {
  const random = Math.random().toString(36).slice(2, 7)
  return `step-${Date.now().toString(36)}-${random}`
}

const defaultStep = (type: TeamOrchestrationStepType = 'agent'): TeamOrchestrationStep => ({
  id: generateStepId(),
  type,
  name: '',
  member: '',
  phase: type === 'agent' ? 'orchestrating' : '',
  instruction: '',
  retry: type === 'agent'
    ? { max_attempts: 1, backoff_ms: 800 }
    : undefined,
  children: type === 'agent' ? undefined : [],
})

const stepTypeLabel = (type: TeamOrchestrationStepType) => {
  if (type === 'parallel') return '并行'
  if (type === 'serial') return '串行'
  return 'Agent'
}

const stepTypeIcon = (type: TeamOrchestrationStepType) => {
  if (type === 'parallel') return 'fas fa-code-branch'
  if (type === 'serial') return 'fas fa-list-ol'
  return 'fas fa-user'
}

const stepTypeBadgeClass = (type: TeamOrchestrationStepType) => {
  if (type === 'parallel') return 'badge-info'
  if (type === 'serial') return 'badge-success'
  return 'badge-primary'
}

const stepTypeCardClass = (type: TeamOrchestrationStepType) => {
  if (type === 'parallel') return 'border-info/40'
  if (type === 'serial') return 'border-success/40'
  return 'border-primary/40'
}

const containerHint = (type: TeamOrchestrationStepType) => {
  if (type === 'parallel') return '并行容器：子节点会并发执行，适合多维审计或并行评估。'
  return '串行容器：子节点按顺序执行，适合分阶段推进。'
}

const normalizeStepAfterTypeChange = (step: TeamOrchestrationStep) => {
  if (step.type === 'agent') {
    step.children = undefined
    step.member = step.member || ''
    if (!step.retry) {
      step.retry = { max_attempts: 1, backoff_ms: 800 }
    }
    if (!step.phase || !step.phase.trim()) {
      step.phase = 'orchestrating'
    }
  } else {
    step.member = ''
    step.retry = undefined
    if (!Array.isArray(step.children)) {
      step.children = []
    }
  }
}

watch(
  () => props.steps,
  (nextSteps) => {
    syncFromPropsInProgress.value = true
    localSteps.value = deepClone(Array.isArray(nextSteps) ? nextSteps : [])
    syncFromPropsInProgress.value = false
  },
  { deep: true, immediate: true },
)

watch(
  localSteps,
  (nextSteps) => {
    if (syncFromPropsInProgress.value) return
    emit('update:steps', deepClone(nextSteps))
    emit('dirty')
  },
  { deep: true },
)

const handleAddStep = (type: TeamOrchestrationStepType) => {
  localSteps.value.push(defaultStep(type))
}

const handleRemoveStep = (index: number) => {
  localSteps.value.splice(index, 1)
}

const handleMoveStep = (index: number, delta: -1 | 1) => {
  const target = index + delta
  if (target < 0 || target >= localSteps.value.length) return
  const [step] = localSteps.value.splice(index, 1)
  localSteps.value.splice(target, 0, step)
}

const handleStepTypeChanged = (step: TeamOrchestrationStep) => {
  normalizeStepAfterTypeChange(step)
}

const handleStepRetryChange = (
  step: TeamOrchestrationStep,
  field: 'max_attempts' | 'backoff_ms',
  event: Event,
) => {
  const target = event.target as HTMLInputElement
  const raw = Number(target.value || (field === 'max_attempts' ? 1 : 800))
  if (!step.retry) {
    step.retry = { max_attempts: 1, backoff_ms: 800 }
  }
  if (field === 'max_attempts') {
    step.retry.max_attempts = Number.isFinite(raw) ? Math.max(1, Math.floor(raw)) : 1
  } else {
    step.retry.backoff_ms = Number.isFinite(raw) ? Math.max(100, Math.floor(raw)) : 800
  }
}

const handleDragStart = (index: number, event: DragEvent) => {
  draggingIndex.value = index
  dragOverIndex.value = index
  dragOverMode.value = null
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    const payload = JSON.stringify({ sourcePath: buildPath(index) })
    event.dataTransfer.setData('application/x-team-step-path', payload)
    event.dataTransfer.setData('text/plain', payload)
  }
}

const handleDragOverZone = (
  index: number,
  mode: 'before' | 'inside',
  event: DragEvent,
) => {
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
  dragOverIndex.value = index
  dragOverMode.value = mode
}

const extractSourcePathFromDragEvent = (event: DragEvent): number[] | null => {
  const raw =
    event.dataTransfer?.getData('application/x-team-step-path') ||
    event.dataTransfer?.getData('text/plain')
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed?.sourcePath)) return null
    return parsed.sourcePath.filter((item: any) => Number.isInteger(item) && item >= 0)
  } catch {
    return null
  }
}

const handleDropZone = (
  index: number,
  mode: 'before' | 'inside',
  event: DragEvent,
) => {
  const sourcePath = extractSourcePathFromDragEvent(event)
  if (!sourcePath || sourcePath.length === 0) {
    handleDragEnd()
    return
  }
  const targetPath = buildPath(index)
  emit('move-step', {
    sourcePath,
    targetPath,
    mode,
  })
  handleDragEnd()
}

const handleDragEnd = () => {
  draggingIndex.value = null
  dragOverIndex.value = null
  dragOverMode.value = null
}

const handleChildDirty = () => {
  emit('dirty')
}

const buildPath = (index: number): number[] => [...props.pathPrefix, index]
const buildPathLabel = (index: number): string => buildPath(index).map((item) => item + 1).join('.')

const handlePromoteStep = (index: number) => {
  emit('promote-step', buildPath(index))
}

const handleNestStep = (index: number) => {
  emit('nest-step', buildPath(index))
}
</script>

<style scoped>
.flow-step {
  position: relative;
}
</style>
