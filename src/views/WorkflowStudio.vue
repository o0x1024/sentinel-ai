<template>
  <div class="p-4 space-y-4">
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold">工作流工作室</h1>
        <div class="flex gap-2">
          <button class="btn btn-primary btn-sm" @click="refresh_catalog">刷新节点库</button>
          <button class="btn btn-outline btn-sm" @click="reset_canvas">重置画布</button>
          <button class="btn btn-success btn-sm" @click="start_run">运行</button>
        </div>
      </div>

    <div class="grid grid-cols-12 gap-4">
      <div class="col-span-3">
        <div class="card bg-base-100 shadow-xl">
          <div class="card-body p-3">
            <div class="flex items-center justify-between mb-2">
              <h2 class="text-base font-semibold">节点库</h2>
              <input v-model="search_query" class="input input-bordered input-xs w-32" placeholder="搜索" />
            </div>
            <div class="space-y-2">
              <div v-for="group in filtered_groups" :key="group.name" class="collapse collapse-arrow bg-base-200">
                <input type="checkbox" />
                <div class="collapse-title text-sm font-medium">{{ group.label }}</div>
                <div class="collapse-content">
                  <div class="grid grid-cols-2 gap-2">
                    <button
                      v-for="item in group.items"
                      :key="item.node_type"
                      class="btn btn-xs"
                      @click="add_node(item)"
                    >
                      {{ item.label }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="col-span-9">
        <FlowchartVisualization ref="flow_ref" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowEvents } from '@/composables/useWorkflowEvents'
import FlowchartVisualization from '@/components/FlowchartVisualization.vue'
import type { NodeCatalogItem, WorkflowGraph, NodeDef, EdgeDef } from '@/types/workflow'
import { validate_workflow_graph } from '@/types/workflow'
import { useToast } from '@/composables/useToast'

const flow_ref = ref<InstanceType<typeof FlowchartVisualization> | null>(null)
const catalog = ref<NodeCatalogItem[]>([])
const search_query = ref('')

const filtered_groups = computed(() => {
  const q = search_query.value.trim().toLowerCase()
  const items = q
    ? catalog.value.filter(i => i.label.toLowerCase().includes(q) || i.node_type.toLowerCase().includes(q))
    : catalog.value
  const groups: Record<string, NodeCatalogItem[]> = {}
  items.forEach(i => {
    const key = i.category
    if (!groups[key]) groups[key] = []
    groups[key].push(i)
  })
  return Object.keys(groups).sort().map(k => ({ name: k, label: group_label(k), items: groups[k] }))
})

const group_label = (k: string) => {
  if (k === 'trigger') return '触发器'
  if (k === 'control') return '控制流'
  if (k === 'data') return '数据'
  if (k === 'output') return '输出'
  if (k === 'tool') return '工具'
  return k
}

const refresh_catalog = async () => {
  const list = await invoke<NodeCatalogItem[]>('list_node_catalog')
  catalog.value = list
}

const add_node = (item: NodeCatalogItem) => {
  const node: any = {
    id: `node_${Date.now()}`,
    name: item.label,
    description: item.node_type,
    status: 'pending',
    x: Math.floor(Math.random() * 400) + 100,
    y: Math.floor(Math.random() * 200) + 80,
    type: item.node_type,
    dependencies: []
  }
  flow_ref.value?.addNode(node)
}

const reset_canvas = () => {
  flow_ref.value?.resetFlowchart()
}

const build_graph = (): WorkflowGraph => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const edges_simple = flow_ref.value?.getFlowchartEdges() || []
  const node_defs: NodeDef[] = nodes.map(n => ({
    id: n.id,
    node_type: n.type,
    node_name: n.name,
    x: Math.round(n.x),
    y: Math.round(n.y),
    params: {},
    input_ports: [{ id: 'in', name: '输入', port_type: 'json', required: false }],
    output_ports: [{ id: 'out', name: '输出', port_type: 'json', required: false }]
  }))
  const edge_defs: EdgeDef[] = edges_simple.map((e, idx) => ({
    id: `e_${idx}_${e.from_node}_${e.to_node}`,
    from_node: e.from_node,
    from_port: 'out',
    to_node: e.to_node,
    to_port: 'in'
  }))
  return {
    id: `wf_${Date.now()}`,
    name: '临时工作流',
    version: 'v1',
    nodes: node_defs,
    edges: edge_defs,
    variables: [],
    credentials: []
  }
}

const start_run = async () => {
  const toast = useToast()
  const graph = build_graph()
  const issues = validate_workflow_graph(graph)
  if (issues.length) {
    toast.error(`校验失败：${issues[0].message}`)
    return
  }
  try {
    const exec_id = await invoke<string>('start_workflow_run', { graph })
    toast.success(`已启动执行：${exec_id}`)
  } catch (e: any) {
    toast.error(`启动失败：${e}`)
  }
}

const wf_events = useWorkflowEvents()
const setup_event_listeners = async () => {
  await wf_events.on_step_start((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) flow_ref.value?.updateNodeStatus(step_id, 'running')
  })
  await wf_events.on_step_complete((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) flow_ref.value?.updateNodeStatus(step_id, 'completed')
  })
  await wf_events.on_run_complete(() => {})
}

onMounted(async () => {
  await refresh_catalog()
  await setup_event_listeners()
})

onUnmounted(() => { wf_events.unsubscribe_all() })
</script>

<style scoped>
</style>
