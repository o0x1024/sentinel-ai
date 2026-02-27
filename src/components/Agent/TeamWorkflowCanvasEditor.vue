<template>
  <div class="workflow-editor space-y-3">
    <div class="flex items-center justify-between">
      <div class="text-xs text-base-content/60">
        拖拽左侧角色到画布，拖动节点调整位置，点击“连线”后再点目标节点建立依赖。
      </div>
      <div class="flex items-center gap-2">
        <button class="btn btn-xs btn-ghost" @click="handleAutoLayout">自动布局</button>
        <button class="btn btn-xs btn-ghost" @click="clearSelection">取消选择</button>
      </div>
    </div>

    <div class="grid grid-cols-1 gap-3 lg:grid-cols-[220px_1fr]">
      <div class="rounded-xl border border-base-300 bg-base-50/50 p-2">
        <div class="mb-2 text-xs font-semibold text-base-content/70">角色节点库</div>
        <div class="space-y-1.5 max-h-72 overflow-auto pr-1">
          <div
            v-for="(member, idx) in memberOptions"
            :key="`member-palette-${member}-${idx}`"
            class="rounded-lg border border-base-300 bg-base-100 px-2 py-1.5 text-xs cursor-grab active:cursor-grabbing flex items-center justify-between gap-2"
            draggable="true"
            @dragstart="onPaletteDragStart(member, $event)"
            @dragend="onPaletteDragEnd"
            @dblclick="addNodeFromPalette(member)"
          >
            <span class="truncate">{{ member }}</span>
            <button
              class="btn btn-ghost btn-xs h-5 min-h-0 px-1.5"
              title="添加到画布"
              @click.stop="addNodeFromPalette(member)"
            >
              <i class="fas fa-plus text-[10px]"></i>
            </button>
          </div>
          <div v-if="memberOptions.length === 0" class="text-xs text-base-content/40 p-2">
            请先在下方“角色配置”中添加角色。
          </div>
        </div>
      </div>

      <div class="rounded-xl border border-base-300 bg-base-100 p-2">
        <div
          ref="canvasRef"
          class="relative h-[420px] overflow-hidden rounded-lg border border-dashed border-base-300 bg-base-50/40"
          @dragenter.prevent="onCanvasDragOver"
          @dragover.prevent="onCanvasDragOver"
          @drop.prevent="onCanvasDrop"
          @mouseup="onCanvasMouseUp"
          @click="clearSelection"
        >
          <svg class="absolute inset-0 h-full w-full pointer-events-none">
            <defs>
              <marker
                id="workflow-arrow"
                markerWidth="10"
                markerHeight="8"
                refX="8"
                refY="4"
                orient="auto"
                markerUnits="strokeWidth"
              >
                <path d="M0,0 L10,4 L0,8 z" fill="#64748b" />
              </marker>
            </defs>
            <line
              v-for="edge in edgeGeometries"
              :key="edge.id"
              :x1="edge.x1"
              :y1="edge.y1"
              :x2="edge.x2"
              :y2="edge.y2"
              stroke="#94a3b8"
              stroke-width="1.5"
              marker-end="url(#workflow-arrow)"
            />
          </svg>

          <div
            v-for="node in localNodes"
            :key="node.id"
            class="absolute w-[176px] rounded-lg border bg-base-100 shadow-sm select-none"
            :class="[
              selectedNodeId === node.id ? 'border-primary ring-1 ring-primary/30' : 'border-base-300',
              pendingConnectSourceId === node.id ? 'ring-1 ring-info/40 border-info' : '',
            ]"
            :style="{ left: `${node.x}px`, top: `${node.y}px` }"
            @mousedown.stop="startNodeDrag(node, $event)"
            @click.stop="onNodeClick(node)"
          >
            <div class="flex items-center justify-between px-2 py-1.5 border-b border-base-200 bg-base-50/70 rounded-t-lg">
              <div class="text-[11px] font-semibold truncate">
                {{ node.title || node.member || '未命名节点' }}
              </div>
              <div class="flex items-center gap-1">
                <button
                  class="btn btn-ghost btn-xs h-5 min-h-0 px-1 text-info"
                  title="从该节点开始连线"
                  @click.stop="beginConnect(node.id)"
                >
                  <i class="fas fa-link text-[10px]"></i>
                </button>
                <button
                  class="btn btn-ghost btn-xs h-5 min-h-0 px-1 text-error"
                  title="删除节点"
                  @click.stop="removeNode(node.id)"
                >
                  <i class="fas fa-trash text-[10px]"></i>
                </button>
              </div>
            </div>
            <div class="px-2 py-1.5 text-[11px] text-base-content/70">
              <div class="truncate">角色：{{ node.member || '未设置' }}</div>
              <div class="truncate">Phase：{{ node.phase || '-' }}</div>
            </div>
          </div>

          <div
            v-if="pendingConnectSourceId"
            class="absolute right-2 top-2 rounded-md border border-info/40 bg-info/10 px-2 py-1 text-[11px] text-info"
          >
            连线模式：点击目标节点完成连线
            <button class="ml-2 underline" @click.stop="cancelConnect">取消</button>
          </div>
        </div>

        <div class="mt-2 flex flex-wrap gap-1.5">
          <span
            v-for="edge in localEdges"
            :key="`edge-chip-${edge.id}`"
            class="inline-flex items-center gap-1 rounded-full border border-base-300 bg-base-50 px-2 py-0.5 text-[11px]"
          >
            {{ edge.source }} -> {{ edge.target }}
            <button class="text-error" @click="removeEdge(edge.id)">
              <i class="fas fa-times"></i>
            </button>
          </span>
          <span v-if="localEdges.length === 0" class="text-[11px] text-base-content/45">暂无依赖连线</span>
        </div>
      </div>
    </div>

    <div v-if="selectedNode" class="rounded-xl border border-base-300 bg-base-50/40 p-3 space-y-2">
      <div class="text-xs font-semibold text-base-content/70">节点属性</div>
      <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
        <label class="form-control">
          <span class="label-text text-xs">标题</span>
          <input
            v-model="selectedNode.title"
            type="text"
            class="input input-bordered input-xs"
            @input="emitGraphChange"
          />
        </label>
        <label class="form-control">
          <span class="label-text text-xs">角色</span>
          <select v-model="selectedNode.member" class="select select-bordered select-xs" @change="emitGraphChange">
            <option value="">请选择角色</option>
            <option v-for="member in memberOptions" :key="`node-member-${member}`" :value="member">{{ member }}</option>
          </select>
        </label>
        <label class="form-control">
          <span class="label-text text-xs">Phase</span>
          <input
            v-model="selectedNode.phase"
            type="text"
            class="input input-bordered input-xs"
            placeholder="如: analysis/implementation"
            @input="emitGraphChange"
          />
        </label>
        <label class="form-control">
          <span class="label-text text-xs">最大重试次数</span>
          <input
            v-model.number="selectedNode.retry.max_attempts"
            type="number"
            min="0"
            class="input input-bordered input-xs"
            @input="emitGraphChange"
          />
        </label>
        <label class="form-control md:col-span-2">
          <span class="label-text text-xs">指令</span>
          <textarea
            v-model="selectedNode.instruction"
            class="textarea textarea-bordered textarea-xs min-h-[80px]"
            @input="emitGraphChange"
          ></textarea>
        </label>
        <label class="form-control">
          <span class="label-text text-xs">重试退避(ms)</span>
          <input
            v-model.number="selectedNode.retry.backoff_ms"
            type="number"
            min="0"
            class="input input-bordered input-xs"
            @input="emitGraphChange"
          />
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'

export interface WorkflowRetry {
  max_attempts?: number
  backoff_ms?: number
}

export interface WorkflowNode {
  id: string
  member: string
  title?: string
  phase?: string
  instruction?: string
  retry: WorkflowRetry
  x: number
  y: number
}

export interface WorkflowEdge {
  id: string
  source: string
  target: string
}

const props = withDefaults(defineProps<{
  nodes: WorkflowNode[]
  edges: WorkflowEdge[]
  memberOptions: string[]
}>(), {
  nodes: () => [],
  edges: () => [],
  memberOptions: () => [],
})

const emit = defineEmits<{
  (e: 'update:nodes', value: WorkflowNode[]): void
  (e: 'update:edges', value: WorkflowEdge[]): void
}>()

const canvasRef = ref<HTMLDivElement | null>(null)
const localNodes = ref<WorkflowNode[]>([])
const localEdges = ref<WorkflowEdge[]>([])
const selectedNodeId = ref('')
const pendingConnectSourceId = ref('')
const draggingPaletteMember = ref('')
const isNodeDragging = ref(false)

watch(
  () => props.nodes,
  (value) => {
    localNodes.value = (value || []).map((item) => ({
      id: item.id,
      member: item.member || '',
      title: item.title || '',
      phase: item.phase || '',
      instruction: item.instruction || '',
      retry: {
        max_attempts: Number.isFinite(Number(item.retry?.max_attempts)) ? Number(item.retry?.max_attempts) : undefined,
        backoff_ms: Number.isFinite(Number(item.retry?.backoff_ms)) ? Number(item.retry?.backoff_ms) : undefined,
      },
      x: Number.isFinite(Number(item.x)) ? Number(item.x) : 20,
      y: Number.isFinite(Number(item.y)) ? Number(item.y) : 20,
    }))
  },
  { immediate: true, deep: true },
)

watch(
  () => props.edges,
  (value) => {
    localEdges.value = (value || []).map((item) => ({
      id: item.id,
      source: item.source,
      target: item.target,
    }))
  },
  { immediate: true, deep: true },
)

const selectedNode = computed(() => localNodes.value.find((node) => node.id === selectedNodeId.value))

const edgeGeometries = computed(() => {
  const nodeMap = new Map(localNodes.value.map((node) => [node.id, node]))
  return localEdges.value
    .map((edge) => {
      const source = nodeMap.get(edge.source)
      const target = nodeMap.get(edge.target)
      if (!source || !target) return null
      return {
        id: edge.id,
        x1: source.x + 88,
        y1: source.y + 38,
        x2: target.x + 88,
        y2: target.y + 38,
      }
    })
    .filter((item): item is { id: string; x1: number; y1: number; x2: number; y2: number } => !!item)
})

const emitGraphChange = () => {
  emit('update:nodes', localNodes.value.map((node) => ({ ...node, retry: { ...node.retry } })))
  emit('update:edges', localEdges.value.map((edge) => ({ ...edge })))
}

const generateId = (prefix: string) => `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`

const onPaletteDragStart = (member: string, event: DragEvent) => {
  if (!event.dataTransfer) return
  draggingPaletteMember.value = member
  event.dataTransfer.setData('text/plain', member)
  event.dataTransfer.setData('application/x-workflow-member', member)
  event.dataTransfer.effectAllowed = 'copy'
}

const onPaletteDragEnd = () => {
  window.setTimeout(() => {
    draggingPaletteMember.value = ''
  }, 0)
}

const onCanvasDragOver = (event: DragEvent) => {
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
}

const addNodeFromPalette = (member: string) => {
  if (!member) return
  const index = localNodes.value.length
  const node: WorkflowNode = {
    id: generateId('node'),
    member,
    title: member,
    phase: '',
    instruction: '',
    retry: { max_attempts: 1, backoff_ms: 300 },
    x: 12 + (index % 4) * 190,
    y: 12 + Math.floor(index / 4) * 100,
  }
  localNodes.value.push(node)
  selectedNodeId.value = node.id
  emitGraphChange()
}

const onCanvasDrop = (event: DragEvent) => {
  const member = event.dataTransfer?.getData('application/x-workflow-member')
    || event.dataTransfer?.getData('text/plain')
    || draggingPaletteMember.value
    || ''
  if (!member || !canvasRef.value) return
  const rect = canvasRef.value.getBoundingClientRect()
  const x = Math.max(8, Math.min(rect.width - 184, event.clientX - rect.left - 88))
  const y = Math.max(8, Math.min(rect.height - 86, event.clientY - rect.top - 28))
  const node: WorkflowNode = {
    id: generateId('node'),
    member,
    title: member,
    phase: '',
    instruction: '',
    retry: { max_attempts: 1, backoff_ms: 300 },
    x,
    y,
  }
  localNodes.value.push(node)
  selectedNodeId.value = node.id
  draggingPaletteMember.value = ''
  emitGraphChange()
}

const onCanvasMouseUp = (event: MouseEvent) => {
  if (!draggingPaletteMember.value || !canvasRef.value || isNodeDragging.value) return
  const rect = canvasRef.value.getBoundingClientRect()
  const x = Math.max(8, Math.min(rect.width - 184, event.clientX - rect.left - 88))
  const y = Math.max(8, Math.min(rect.height - 86, event.clientY - rect.top - 28))
  const member = draggingPaletteMember.value
  const node: WorkflowNode = {
    id: generateId('node'),
    member,
    title: member,
    phase: '',
    instruction: '',
    retry: { max_attempts: 1, backoff_ms: 300 },
    x,
    y,
  }
  localNodes.value.push(node)
  selectedNodeId.value = node.id
  draggingPaletteMember.value = ''
  emitGraphChange()
}

const startNodeDrag = (node: WorkflowNode, event: MouseEvent) => {
  const target = event.target as HTMLElement | null
  if (target?.closest('button,input,textarea,select,option,label')) return
  if (!canvasRef.value) return

  selectedNodeId.value = node.id
  const rect = canvasRef.value.getBoundingClientRect()
  const startX = event.clientX
  const startY = event.clientY
  const originX = node.x
  const originY = node.y
  isNodeDragging.value = true

  const onMouseMove = (moveEvent: MouseEvent) => {
    const dx = moveEvent.clientX - startX
    const dy = moveEvent.clientY - startY
    node.x = Math.max(8, Math.min(rect.width - 184, originX + dx))
    node.y = Math.max(8, Math.min(rect.height - 86, originY + dy))
  }

  const onMouseUp = () => {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
    window.setTimeout(() => {
      isNodeDragging.value = false
    }, 0)
    emitGraphChange()
  }

  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
}

const beginConnect = (sourceId: string) => {
  pendingConnectSourceId.value = sourceId
}

const cancelConnect = () => {
  pendingConnectSourceId.value = ''
}

const onNodeClick = (node: WorkflowNode) => {
  if (pendingConnectSourceId.value && pendingConnectSourceId.value !== node.id) {
    const sourceId = pendingConnectSourceId.value
    const hasExisting = localEdges.value.some((edge) => edge.source === sourceId && edge.target === node.id)
    if (!hasExisting) {
      localEdges.value.push({
        id: generateId('edge'),
        source: sourceId,
        target: node.id,
      })
      emitGraphChange()
    }
    pendingConnectSourceId.value = ''
    selectedNodeId.value = node.id
    return
  }
  selectedNodeId.value = node.id
}

const removeEdge = (edgeId: string) => {
  localEdges.value = localEdges.value.filter((edge) => edge.id !== edgeId)
  emitGraphChange()
}

const removeNode = (nodeId: string) => {
  localNodes.value = localNodes.value.filter((node) => node.id !== nodeId)
  localEdges.value = localEdges.value.filter((edge) => edge.source !== nodeId && edge.target !== nodeId)
  if (selectedNodeId.value === nodeId) {
    selectedNodeId.value = ''
  }
  if (pendingConnectSourceId.value === nodeId) {
    pendingConnectSourceId.value = ''
  }
  emitGraphChange()
}

const clearSelection = () => {
  selectedNodeId.value = ''
  pendingConnectSourceId.value = ''
}

const handleAutoLayout = () => {
  if (localNodes.value.length === 0) return
  const ids = localNodes.value.map((node) => node.id)
  const indegree = new Map(ids.map((id) => [id, 0]))
  const outgoing = new Map(ids.map((id) => [id, [] as string[]]))

  localEdges.value.forEach((edge) => {
    if (!indegree.has(edge.source) || !indegree.has(edge.target)) return
    outgoing.get(edge.source)!.push(edge.target)
    indegree.set(edge.target, (indegree.get(edge.target) || 0) + 1)
  })

  const queue: string[] = ids.filter((id) => (indegree.get(id) || 0) === 0)
  const levels = new Map<string, number>()
  queue.forEach((id) => levels.set(id, 0))

  for (let index = 0; index < queue.length; index += 1) {
    const current = queue[index]
    const currentLevel = levels.get(current) || 0
    for (const next of outgoing.get(current) || []) {
      const nextLevel = Math.max(levels.get(next) || 0, currentLevel + 1)
      levels.set(next, nextLevel)
      indegree.set(next, (indegree.get(next) || 0) - 1)
      if ((indegree.get(next) || 0) === 0) {
        queue.push(next)
      }
    }
  }

  if (queue.length !== ids.length) {
    localNodes.value.forEach((node, idx) => {
      node.x = 16 + (idx % 4) * 200
      node.y = 16 + Math.floor(idx / 4) * 108
    })
    emitGraphChange()
    return
  }

  const grouped = new Map<number, WorkflowNode[]>()
  localNodes.value.forEach((node) => {
    const level = levels.get(node.id) || 0
    if (!grouped.has(level)) grouped.set(level, [])
    grouped.get(level)!.push(node)
  })

  Array.from(grouped.entries())
    .sort((a, b) => a[0] - b[0])
    .forEach(([level, nodes]) => {
      nodes.forEach((node, idx) => {
        node.x = 16 + idx * 200
        node.y = 16 + level * 108
      })
    })

  emitGraphChange()
}
</script>
