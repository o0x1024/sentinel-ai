<template>
  <div class="p-4 space-y-4 h-full flex flex-col">
    <WorkflowStudioHeader
      :workflow-name="workflow_name"
      :is-auto-saving="is_auto_saving"
      :has-unsaved-changes="has_unsaved_changes"
      :show-workflow-list-panel="show_workflow_list_panel"
      :workflow-running="workflow_running"
      :schedule-running="schedule_running"
      :has-schedule-trigger="has_schedule_trigger"
      :show-logs="show_logs"
      :show-execution-history="show_execution_history"
      :execution-history-count="execution_history.length"
      :on-open-meta-dialog="() => { show_meta_dialog = true }"
      :on-toggle-workflow-list-panel="toggle_workflow_list_panel"
      :on-save-workflow="handle_save_workflow_click"
      :on-export-workflow-json="export_workflow_json"
      :on-trigger-import-file="trigger_import_file"
      :on-export-workflow-image="export_workflow_image"
      :on-refresh-catalog="refresh_catalog"
      :on-reset-canvas="reset_canvas"
      :on-start-run="start_run"
      :on-stop-run="stop_run"
      :on-start-schedule="start_schedule"
      :on-stop-schedule="stop_schedule"
      :on-toggle-logs="() => { show_logs = !show_logs }"
      :on-toggle-execution-history="toggle_execution_history"
      @update:workflow-name="workflow_name = $event"
    />
    <input ref="import_file_input" type="file" accept=".json" class="hidden" @change="import_workflow_json" />

    <div class="flex-1 flex gap-4 min-h-0 overflow-hidden relative">
      <WorkflowStudioWorkflowListPanel
        :show-workflow-list-panel="show_workflow_list_panel"
        :workflow-list-tab="workflow_list_tab"
        :workflow-list-search="workflow_list_search"
        :workflow-id="workflow_id"
        :workflow-name="workflow_name"
        :filtered-workflow-list="filtered_workflow_list"
        :filtered-template-list="filtered_template_list"
        :on-close="() => { show_workflow_list_panel = false }"
        :on-switch-tab="switch_workflow_list_tab"
        :on-load-workflow="load_workflow_from_panel"
        :on-edit-workflow-metadata="edit_workflow_metadata"
        :on-clone-workflow="clone_workflow"
        :on-delete-workflow="delete_workflow"
        :on-use-template="use_template"
        :on-delete-template="delete_template"
        :on-create-new-workflow="create_new_workflow"
        :on-save-current-as-template="save_current_as_template"
        @update:workflow-list-search="workflow_list_search = $event"
      />

      <WorkflowStudioCanvasLayout
        :sidebar-collapsed="sidebar_collapsed"
        :sidebar-width="sidebar_width"
        :sidebar-transition-ready="sidebar_transition_ready"
        :is-resizing-sidebar="is_resizing_sidebar"
        :show-workflow-list-panel="show_workflow_list_panel"
        :search-query="search_query"
        :show-favorites-only="show_favorites_only"
        :filtered-groups="filtered_groups"
        :is-favorite="is_favorite"
        :on-toggle-sidebar-collapsed="() => { sidebar_collapsed = !sidebar_collapsed }"
        :on-start-sidebar-resize="start_sidebar_resize"
        :on-search-change="on_search_change"
        :on-clear-search="clear_search"
        :on-search-in-canvas="search_in_canvas"
        :on-add-node="add_node"
        :on-toggle-favorite="toggle_favorite"
        @update:search-query="search_query = $event"
        @update:show-favorites-only="show_favorites_only = $event"
      >
        <template #canvas>
          <FlowchartVisualization
            ref="flow_ref"
            @node-click="on_node_click"
            @connection-click="on_connection_click"
            @new-workflow="on_new_workflow"
            @change="on_flowchart_change"
            :highlightedNodes="highlighted_nodes"
          />
        </template>
      </WorkflowStudioCanvasLayout>
    </div>

    <WorkflowStudioPanels
      ref="workflow_panels_ref"
      :show-logs="show_logs"
      :execution-logs="execution_logs"
      :expanded-logs="expanded_logs"
      :get-log-class="get_log_class"
      :format-time="format_time"
      :show-new-workflow-confirm="show_new_workflow_confirm"
      :show-meta-dialog="show_meta_dialog"
      :workflow-name="workflow_name"
      :workflow-description="workflow_description"
      :workflow-tags="workflow_tags"
      :workflow-version="workflow_version"
      :workflow-is-tool="workflow_is_tool"
      :node-count="flow_ref?.getFlowchartNodes().length || 0"
      :edge-count="flow_ref?.getFlowchartEdges().length || 0"
      :drawer-open="drawer_open"
      :selected-node="selected_node"
      :selected-schema="selected_schema"
      :param-values="param_values"
      :notification-rules="notification_rules"
      :available-tools="available_tools"
      :json-errors="json_errors"
      :has-validation-errors="has_validation_errors"
      :show-detail-dialog="show_detail_dialog"
      :detail-dialog-fullscreen="detail_dialog_fullscreen"
      :detail-loading="detail_loading"
      :detail-data="detail_data"
      :show-result-panel="show_result_panel"
      :selected-step-result="selected_step_result"
      :selected-node-name="selected_node?.name || ''"
      :format-datetime="format_datetime"
      :format-duration="format_duration"
      :get-status-badge-class="get_status_badge_class"
      :get-status-text="get_status_text"
      :format-result="format_result"
      :get-enabled-providers="get_enabled_providers"
      :get-provider-models="get_provider_models"
      :on-clear-logs="clear_logs"
      :on-toggle-log-details="toggle_log_details"
      :on-confirm-new-workflow-save="confirm_new_workflow_save"
      :on-confirm-new-workflow-discard="confirm_new_workflow_discard"
      :on-close-drawer="close_drawer"
      :on-toggle-tool-selection="toggle_tool_selection"
      :on-validate-json="validate_json"
      :on-save-params-and-close="save_params_and_close"
      :on-copy-detail-result="copy_detail_result"
      :on-copy-result-to-clipboard="copy_result_to_clipboard"
      :on-close-result-panel="close_result_panel"
      :on-edit-node-params="edit_node_params"
      @update:show-logs="show_logs = $event"
      @update:show-new-workflow-confirm="show_new_workflow_confirm = $event"
      @update:show-meta-dialog="show_meta_dialog = $event"
      @update:workflow-name="workflow_name = $event"
      @update:workflow-description="workflow_description = $event"
      @update:workflow-tags="workflow_tags = $event"
      @update:workflow-version="workflow_version = $event"
      @update:workflow-is-tool="workflow_is_tool = $event"
      @update:show-detail-dialog="show_detail_dialog = $event"
      @update:detail-dialog-fullscreen="detail_dialog_fullscreen = $event"
    />

    <WorkflowStudioEdgeMappingDialog
      :open="show_edge_mapping_dialog"
      :edge-label="selected_edge_label"
      :source-path-options="edge_source_path_options"
      :target-path-options="edge_target_path_options"
      :form="edge_mapping_form"
      @close="close_edge_mapping_dialog"
      @save="save_edge_mapping"
    />

    <WorkflowStudioHistoryPanel
      ref="execution_history_ref"
      :show-execution-history="show_execution_history"
      :history-search-query="history_search_query"
      :history-loading="history_loading"
      :history-data="history_data"
      :history-total="history_total"
      :history-page="history_page"
      :history-page-size="history_page_size"
      :format-datetime="format_datetime"
      :format-duration="format_duration"
      :get-status-badge-class="get_status_badge_class"
      :get-status-text="get_status_text"
      :on-load-history="load_history_from_backend"
      :on-view-execution-detail="view_execution_detail"
      :on-delete-history-record="delete_history_record"
      :on-close="() => { show_execution_history = false }"
      @update:history-search-query="history_search_query = $event"
      @update:history-page="history_page = $event"
      @update:history-page-size="history_page_size = $event"
    />
      
    
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowEvents } from '@/composables/useWorkflowEvents'
import FlowchartVisualization from '@/components/workflow/FlowchartVisualization.vue'
import WorkflowStudioCanvasLayout from './WorkflowStudioCanvasLayout.vue'
import WorkflowStudioEdgeMappingDialog from './WorkflowStudioEdgeMappingDialog.vue'
import WorkflowStudioHeader from './WorkflowStudioHeader.vue'
import WorkflowStudioHistoryPanel from './WorkflowStudioHistoryPanel.vue'
import WorkflowStudioPanels from './WorkflowStudioPanels.vue'
import WorkflowStudioWorkflowListPanel from './WorkflowStudioWorkflowListPanel.vue'
import type { EdgeDef, EdgeMergeMode, EdgeSourceScope, NodeCatalogItem, WorkflowGraph } from '@/types/workflow'
import { validate_workflow_graph as validate_workflow_graph_client } from '@/types/workflow'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { applyWorkflowStudioGraphToCanvas, buildWorkflowStudioGraph } from './workflowStudioCanvasSupport'
import {
  buildSourcePathOptions,
  buildTargetPathOptions,
  type EdgeMappingOption,
} from './workflowStudioEdgeMappingSupport'
import {
  addWorkflowExecutionLog,
  clearWorkflowExecutionHistoryState,
  clearWorkflowExecutionLogs,
  completeWorkflowExecutionRecord,
  deleteWorkflowExecutionRecord,
  formatWorkflowDatetime,
  formatWorkflowDuration,
  formatWorkflowLogTime,
  formatWorkflowResult,
  formatWorkflowShortDate,
  getWorkflowLogClass,
  getWorkflowStatusBadgeClass,
  loadWorkflowExecutionHistory,
  saveWorkflowExecutionHistory,
  startWorkflowExecutionRecord,
  toggleWorkflowLogDetails,
  truncateWorkflowLogDetails,
  type DetailData,
  type ExecutionLog,
  type ExecutionRecord,
  type HistoryItem,
  updateWorkflowExecutionStepResult,
} from './workflowStudioExecutionSupport'

const { t } = useI18n()
const route = useRoute()

const flow_ref = ref<InstanceType<typeof FlowchartVisualization> | null>(null)
const catalog = ref<NodeCatalogItem[]>([])
const search_query = ref('')
const selected_node = ref<any | null>(null)
const selected_edge_id = ref<string | null>(null)
const selected_edge_label = ref('')
const show_edge_mapping_dialog = ref(false)
const edge_source_path_options = ref<EdgeMappingOption[]>([])
const edge_target_path_options = ref<EdgeMappingOption[]>([])
const edge_mapping_form = ref<{
  source_scope: EdgeSourceScope
  source_path: string
  target_path: string
  merge_mode: EdgeMergeMode
}>({
  source_scope: 'output',
  source_path: '',
  target_path: '',
  merge_mode: 'replace',
})
const param_values = ref<Record<string, any>>({})
const drawer_open = ref(false)
const ignore_close_once = ref(false)
const drawer_ref = ref<HTMLElement | null>(null)
const workflow_panels_ref = ref<{ logsContainerRef: HTMLElement | null; detailDialogRef: HTMLDialogElement | null; resultPanelRef: HTMLElement | null } | null>(null)
const ignore_result_panel_close_once = ref(false)
const execution_history_ref = ref<{ historyPanelRef: HTMLElement | null } | null>(null)
const ignore_execution_history_close_once = ref(false)
const sidebar_collapsed = ref(false)
const show_logs = ref(true) // 默认显示日志
const show_meta_dialog = ref(false)
const show_new_workflow_confirm = ref(false)
const template_list = ref<any[]>([])
const workflow_name = ref(t('trafficAnalysis.workflowStudio.defaults.unnamedWorkflow'))
const workflow_id = ref(`wf_${Date.now()}`)
const workflow_description = ref('')
const workflow_tags = ref('')
const workflow_version = ref('v1.0.0')
const workflow_is_tool = ref(false) // 是否设为AI工具
const workflow_list = ref<any[]>([])
// 工作流列表面板
const show_workflow_list_panel = ref(false)
const workflow_list_tab = ref<'workflows' | 'templates'>('workflows')
const workflow_list_search = ref('')
const schedule_running = ref(false) // 定时调度是否运行中
const schedule_info = ref<any>(null) // 当前调度信息
const workflow_running = ref(false) // 工作流是否正在运行
const current_exec_id = ref<string | null>(null) // 当前执行ID
const favorites = ref<Set<string>>(new Set())
const show_favorites_only = ref(false)
const notification_rules = ref<any[]>([]) // 通知规则列表
const ai_config = ref<any>(null) // AI 配置
const available_tools = ref<any[]>([]) // 可用工具列表
const import_file_input = ref<HTMLInputElement | null>(null)
const highlighted_nodes = ref<Set<string>>(new Set())
const step_results = ref<Record<string, any>>({}) // 存储当前执行的步骤结果
const show_result_panel = ref(false)
const selected_step_result = ref<{ step_id: string, result: any } | null>(null)
const auto_save_timer = ref<ReturnType<typeof setTimeout> | null>(null)
const is_auto_saving = ref(false)
const has_unsaved_changes = ref(false)
const is_loading_graph = ref(false)
const AUTO_SAVE_DELAY = 1000 // 1秒防抖延迟
const MAX_EXECUTION_LOGS = 500
const MAX_LOG_DETAILS_LENGTH = 2000
const SIDEBAR_WIDTH_KEY = 'workflow_studio_sidebar_width'
const SIDEBAR_DEFAULT_WIDTH = 360
const SIDEBAR_MIN_WIDTH = 260
const SIDEBAR_MAX_WIDTH = 520
const clamp_sidebar_width = (w: number) => {
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, w))
}

const get_initial_sidebar_width = () => {
  const saved_sidebar_width = localStorage.getItem(SIDEBAR_WIDTH_KEY)
  if (!saved_sidebar_width) return SIDEBAR_DEFAULT_WIDTH

  const parsed = Number(saved_sidebar_width)
  return Number.isNaN(parsed) ? SIDEBAR_DEFAULT_WIDTH : clamp_sidebar_width(parsed)
}

const sidebar_width = ref(get_initial_sidebar_width())
const sidebar_transition_ready = ref(false)
const is_resizing_sidebar = ref(false)
const sidebar_resize_start_x = ref(0)
const sidebar_resize_start_width = ref(sidebar_width.value)

const getExecutionHistoryElement = () => execution_history_ref.value?.historyPanelRef ?? null
const getWorkflowPanelsLogsElement = () => workflow_panels_ref.value?.logsContainerRef ?? null
const getWorkflowPanelsDetailDialogElement = () => workflow_panels_ref.value?.detailDialogRef ?? null
const getWorkflowPanelsResultPanelElement = () => workflow_panels_ref.value?.resultPanelRef ?? null
const logs_container_ref = computed(() => getWorkflowPanelsLogsElement())

defineOptions({
  name: 'WorkflowStudio'
});


const show_execution_history = ref(false)
const execution_history = ref<ExecutionRecord[]>([])
const selected_execution = ref<ExecutionRecord | null>(null)
const current_execution_id = ref<string | null>(null)
const history_search_query = ref('')
const history_page = ref(1)
const history_page_size = ref(10)
const history_total = ref(0)
const history_data = ref<HistoryItem[]>([])
const history_loading = ref(false)
const show_detail_dialog = ref(false)
const detail_dialog_fullscreen = ref(false)
const detail_loading = ref(false)
const detail_data = ref<DetailData | null>(null)
const last_route_execution_id = ref<string | null>(null)

const execution_logs = ref<ExecutionLog[]>([])
const json_errors = ref<Record<string, string>>({})
const expanded_logs = ref<Set<number>>(new Set())
const selected_schema = computed(() => {
  if (!selected_node.value) return null as any
  const item = catalog_index.value.get(selected_node.value.type)
  return item?.params_schema || null
})

const catalog_index = computed(() => {
  return new Map(catalog.value.map(item => [item.node_type, item]))
})

const filtered_groups = computed(() => {
  const q = search_query.value.trim().toLowerCase()
  let items = catalog.value
  
  // 应用搜索过滤
  if (q) {
    items = items.filter(i => 
      i.label.toLowerCase().includes(q) || 
      i.node_type.toLowerCase().includes(q)
    )
  }
  
  // 应用收藏过滤
  if (show_favorites_only.value) {
    items = items.filter(i => favorites.value.has(i.node_type))
  }
  
  const groups: Record<string, NodeCatalogItem[]> = {}
  items.forEach(i => {
    const key = i.category
    if (!groups[key]) groups[key] = []
    groups[key].push(i)
  })
  return Object.keys(groups).sort().map(k => ({ name: k, label: group_label(k), items: groups[k] }))
})

const group_label = (k: string) => {
  if (k === 'trigger') return t('trafficAnalysis.workflowStudio.groups.trigger')
  if (k === 'control') return t('trafficAnalysis.workflowStudio.groups.control')
  if (k === 'ai') return t('trafficAnalysis.workflowStudio.groups.ai')
  if (k === 'data') return t('trafficAnalysis.workflowStudio.groups.data')
  if (k === 'output') return t('trafficAnalysis.workflowStudio.groups.output')
  if (k === 'tool') return t('trafficAnalysis.workflowStudio.groups.tool')
  if (k === 'mcp') return t('trafficAnalysis.workflowStudio.groups.mcp')
  if (k === 'plugin') return t('trafficAnalysis.workflowStudio.groups.plugin')
  return k
}

// 检查是否有定时触发节点
const has_schedule_trigger = computed(() => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  return nodes.some(n => n.type === 'trigger_schedule')
})

// 获取定时触发配置
const get_schedule_config = () => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const triggerNode = nodes.find((n: any) => n.type === 'trigger_schedule')
  if (!triggerNode?.params) return null

  const to_number_or_default = (value: any, default_value: number) => {
    const num = Number(value)
    return Number.isFinite(num) ? num : default_value
  }

  // 确保数值类型正确
  return {
    trigger_type: String(triggerNode.params.trigger_type || 'interval'),
    interval_seconds: to_number_or_default(triggerNode.params.interval_seconds, 60),
    hour: to_number_or_default(triggerNode.params.hour, 9),
    minute: to_number_or_default(triggerNode.params.minute, 0),
    second: to_number_or_default(triggerNode.params.second, 0),
    weekdays: String(triggerNode.params.weekdays || '1,2,3,4,5'),
  }
}

const refresh_catalog = async () => {
  const list = await invoke<NodeCatalogItem[]>('list_node_catalog')
  catalog.value = list
}

// 加载通知规则
const load_notification_rules = () => {
  try {
    const raw = localStorage.getItem('sentinel-notification-rules')
    if (raw) {
      const rules = JSON.parse(raw)
      notification_rules.value = rules.filter((r: any) => r.enabled) // 只显示启用的规则
    }
  } catch (e) {
    console.error('Failed to load notification rules:', e)
  }
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
    dependencies: [],
    params: {},
    metadata: { input_ports: item.input_ports || [], output_ports: item.output_ports || [] }
  }
  flow_ref.value?.addNode(node)
}

const reset_canvas = () => {
  close_edge_mapping_dialog()
  flow_ref.value?.resetFlowchart()
}

const reset_schedule_state = () => {
  schedule_running.value = false
  schedule_info.value = null
}

// 新建工作流
const on_new_workflow = () => {
  // 检查是否有未保存的更改
  if (flow_ref.value?.hasUnsavedChanges()) {
    show_new_workflow_confirm.value = true
  } else {
    do_new_workflow()
  }
}

// 确认保存后新建
const confirm_new_workflow_save = async () => {
  const toast = useToast()
  show_new_workflow_confirm.value = false
  
  if (!workflow_name.value.trim()) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.enterWorkflowName'))
    return
  }
  const saved = await save_workflow()
  if (!saved) return
  do_new_workflow()
}

// 确认直接新建（丢弃更改）
const confirm_new_workflow_discard = () => {
  show_new_workflow_confirm.value = false
  do_new_workflow()
}

// 执行新建工作流
const do_new_workflow = () => {
  const toast = useToast()
  
  // 重置为新工作流
  workflow_id.value = `wf_${Date.now()}`
  workflow_name.value = t('trafficAnalysis.workflowStudio.defaults.unnamedWorkflow')
  workflow_description.value = ''
  workflow_tags.value = ''
  workflow_version.value = 'v1.0.0'
  workflow_is_tool.value = false
  flow_ref.value?.resetFlowchart()
  execution_history.value = []
  selected_execution.value = null
  execution_logs.value = []
  step_results.value = {}
  reset_schedule_state()
  has_unsaved_changes.value = false
  localStorage.removeItem('last_run_workflow_id')
  
  add_log('INFO', t('trafficAnalysis.workflowStudio.logs.newWorkflowCreated'))
  toast.success(t('trafficAnalysis.workflowStudio.toasts.newWorkflowCreated'))
}

const build_graph = (): WorkflowGraph => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const edges_detailed = (flow_ref.value as any)?.getFlowchartEdgesDetailed?.() || []
  return buildWorkflowStudioGraph({
    workflowId: workflow_id.value,
    workflowName: workflow_name.value,
    workflowVersion: workflow_version.value,
    workflowIsTool: workflow_is_tool.value,
    nodes,
    edgesDetailed: edges_detailed,
    fallbackEdges: flow_ref.value?.getFlowchartEdges() || [],
    catalogIndex: catalog_index.value,
    unnamedWorkflowLabel: t('trafficAnalysis.workflowStudio.defaults.unnamedWorkflow'),
    inputPortLabel: t('trafficAnalysis.workflowStudio.flowchart.ports.input'),
    outputPortLabel: t('trafficAnalysis.workflowStudio.flowchart.ports.output'),
  })
}

const get_current_edges = (): EdgeDef[] => {
  return ((flow_ref.value as any)?.getFlowchartEdgesDetailed?.() || []) as EdgeDef[]
}

const refresh_edge_mapping_options = (edge: EdgeDef | null) => {
  if (!edge) {
    edge_source_path_options.value = []
    edge_target_path_options.value = []
    return
  }

  const nodes = flow_ref.value?.getFlowchartNodes() || []
  edge_source_path_options.value = buildSourcePathOptions({
    edge,
    nodes,
    catalogIndex: catalog_index.value,
    stepResults: step_results.value,
  })
  edge_target_path_options.value = buildTargetPathOptions(edge, nodes, catalog_index.value)
}

const close_edge_mapping_dialog = () => {
  show_edge_mapping_dialog.value = false
  selected_edge_id.value = null
  selected_edge_label.value = ''
  edge_source_path_options.value = []
  edge_target_path_options.value = []
}

const on_connection_click = (connection: { edgeId?: string }) => {
  if (!connection?.edgeId) return

  const edge = get_current_edges().find(item => item.id === connection.edgeId)
  if (!edge) return

  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const fromNode = nodes.find(node => node.id === edge.from_node)
  const toNode = nodes.find(node => node.id === edge.to_node)
  refresh_edge_mapping_options(edge)

  selected_edge_id.value = edge.id
  selected_edge_label.value = `${fromNode?.name || edge.from_node} -> ${toNode?.name || edge.to_node}`
  edge_mapping_form.value = {
    source_scope: edge.source_scope || 'output',
    source_path: edge.source_path || '',
    target_path: edge.target_path || '',
    merge_mode: edge.merge_mode || 'replace',
  }
  drawer_open.value = false
  show_result_panel.value = false
  show_edge_mapping_dialog.value = true
}

const save_edge_mapping = (value: {
  source_scope: EdgeSourceScope
  source_path: string
  target_path: string
  merge_mode: EdgeMergeMode
}) => {
  if (!selected_edge_id.value) return

  ;(flow_ref.value as any)?.updateEdgeMapping?.(selected_edge_id.value, value)
  trigger_auto_save()
  close_edge_mapping_dialog()
}

watch(
  () => [show_edge_mapping_dialog.value, selected_edge_id.value, edge_mapping_form.value.source_scope] as const,
  ([open, edgeId, sourceScope]) => {
    if (!open || !edgeId) return
    const edge = get_current_edges().find(item => item.id === edgeId)
    if (!edge) return
    refresh_edge_mapping_options({
      ...edge,
      source_scope: sourceScope,
    })
  },
)

const has_nonempty_schema = (schema: any): boolean => {
  if (!schema || typeof schema !== 'object') return false
  if (Array.isArray(schema)) return schema.length > 0
  return Object.keys(schema).length > 0
}

const validate_tool_schemas = (graph: WorkflowGraph, silent: boolean): boolean => {
  if (!workflow_is_tool.value) return true

  const missing: string[] = []
  if (!has_nonempty_schema(graph.input_schema)) missing.push('input')
  if (!has_nonempty_schema(graph.output_schema)) missing.push('output')

  if (!missing.length) return true

  const toast = useToast()
  const message = missing.length === 2
    ? '缺少工具输入/输出 schema'
    : missing[0] === 'input'
      ? '缺少工具输入 schema'
      : '缺少工具输出 schema'

  add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.validationFailed', { message }))
  if (!silent) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.validationFailed', { message }))
  }
  return false
}

const add_log = (level: ExecutionLog['level'], message: string, node_id?: string, details?: string) => {
  addWorkflowExecutionLog({
    executionLogs: execution_logs,
    expandedLogs: expanded_logs,
    logsContainerRef: logs_container_ref,
    maxExecutionLogs: MAX_EXECUTION_LOGS,
    maxLogDetailsLength: MAX_LOG_DETAILS_LENGTH,
    level,
    message,
    nodeId: node_id,
    details,
  })
}

const clear_logs = () => {
  clearWorkflowExecutionLogs(execution_logs, expanded_logs)
}

const toggle_log_details = (idx: number) => {
  toggleWorkflowLogDetails(expanded_logs, idx)
}

const format_result = (result: any) => {
  return formatWorkflowResult(result, t('trafficAnalysis.workflowStudio.resultPanel.noResult'))
}

const copy_result_to_clipboard = async () => {
  const toast = useToast()
  if (!selected_step_result.value || selected_step_result.value.result === undefined) return
  
  try {
    const text = format_result(selected_step_result.value.result)
    await navigator.clipboard.writeText(text)
    toast.success(t('trafficAnalysis.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.copyFailed', { message: e.message }))
  }
}

const close_result_panel = () => {
  show_result_panel.value = false
  selected_step_result.value = null
}

const edit_node_params = () => {
  show_result_panel.value = false
  ignore_close_once.value = true
  drawer_open.value = true
}

const view_node_result = () => {
  if (!selected_node.value) return
  selected_step_result.value = {
    step_id: selected_node.value.id,
    result: step_results.value[selected_node.value.id]
  }
  drawer_open.value = false
  show_result_panel.value = true
}

// 执行历史相关方法
const toggle_execution_history = () => {
  if (!show_execution_history.value) {
    ignore_execution_history_close_once.value = true
    // 打开时加载数据
    load_history_from_backend()
  }
  show_execution_history.value = !show_execution_history.value
}

// 从后端加载执行历史
const load_history_from_backend = async () => {
  history_loading.value = true
  try {
    const result = await invoke<{ data: HistoryItem[], total: number }>('list_workflow_runs_paginated', {
      page: history_page.value,
      pageSize: history_page_size.value,
      search: history_search_query.value || null,
      workflowId: null // 显示所有工作流的历史
    })
    history_data.value = result.data
    history_total.value = result.total
  } catch (e: any) {
    console.error('Failed to load execution history:', e)
    const toast = useToast()
    toast.error(t('trafficAnalysis.workflowStudio.toasts.loadFailed', { error: String(e) }))
  } finally {
    history_loading.value = false
  }
}

// 查看执行详情
const view_execution_detail = async (runId: string) => {
  show_detail_dialog.value = true
  detail_dialog_fullscreen.value = false
  detail_loading.value = true
  detail_data.value = null
  try {
    const result = await invoke<DetailData | null>('get_workflow_run_detail', { runId })
    detail_data.value = result
  } catch (e: any) {
    console.error('Failed to load execution detail:', e)
    const toast = useToast()
    toast.error(t('trafficAnalysis.workflowStudio.toasts.loadFailed', { error: String(e) }))
    show_detail_dialog.value = false
  } finally {
    detail_loading.value = false
  }
}

const open_execution_from_route = async (executionId: string) => {
  if (!executionId || last_route_execution_id.value === executionId) return
  last_route_execution_id.value = executionId
  history_search_query.value = executionId
  history_page.value = 1
  if (!show_execution_history.value) {
    ignore_execution_history_close_once.value = true
    show_execution_history.value = true
  }
  await load_history_from_backend()
  await view_execution_detail(executionId)
}

// 删除执行记录（不需要确认）
const delete_history_record = async (runId: string) => {
  try {
    await invoke('delete_workflow_run', { runId })
    // 重新加载当前页
    await load_history_from_backend()
  } catch (e: any) {
    console.error('Failed to delete execution record:', e)
    const toast = useToast()
    toast.error(t('trafficAnalysis.workflowStudio.toasts.deleteFailed', { error: String(e) }))
  }
}

// 复制详情结果
const copy_detail_result = async () => {
  const toast = useToast()
  if (!detail_data.value) return
  try {
    const text = JSON.stringify(detail_data.value, null, 2)
    await navigator.clipboard.writeText(text)
    toast.success(t('trafficAnalysis.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.copyFailed', { message: e.message }))
  }
}

// 格式化日期时间
const format_datetime = formatWorkflowDatetime

const format_duration = formatWorkflowDuration

const get_status_badge_class = getWorkflowStatusBadgeClass

// 获取状态文本
const get_status_text = (status: string) => {
  switch (status) {
    case 'completed': return t('trafficAnalysis.workflowStudio.executionHistory.status.completed')
    case 'failed': return t('trafficAnalysis.workflowStudio.executionHistory.status.failed')
    case 'running': return t('trafficAnalysis.workflowStudio.executionHistory.status.running')
    case 'pending': return t('trafficAnalysis.workflowStudio.executionHistory.status.pending')
    case 'cancelled': return t('trafficAnalysis.workflowStudio.executionHistory.status.cancelled')
    default: return status
  }
}

const select_execution = (exec: ExecutionRecord) => {
  selected_execution.value = exec
}

const clear_execution_history = () => {
  clearWorkflowExecutionHistoryState(workflow_id.value, execution_history, selected_execution)
}

// 删除单条执行记录
const delete_single_execution = (execId: string) => {
  deleteWorkflowExecutionRecord(execution_history, selected_execution, execId)
  save_execution_history()
}

const copy_execution_result = async () => {
  const toast = useToast()
  if (!selected_execution.value) return
  
  try {
    const text = JSON.stringify(selected_execution.value.step_results, null, 2)
    await navigator.clipboard.writeText(text)
    toast.success(t('trafficAnalysis.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.copyFailed', { message: e.message }))
  }
}

const get_node_name = (nodeId: string): string => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const node = nodes.find((n: any) => n.id === nodeId) as any
  return node?.name || nodeId
}

const start_new_execution = (): string => {
  return startWorkflowExecutionRecord(execution_history, current_execution_id)
}

const update_execution_step_result = (stepId: string, result: any) => {
  updateWorkflowExecutionStepResult(execution_history, current_execution_id, stepId, result)
}

const complete_execution = (success: boolean) => {
  completeWorkflowExecutionRecord(execution_history, current_execution_id, success)
  save_execution_history()
}

// 重置所有节点的执行状态
const reset_node_status = () => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  nodes.forEach((node: any) => {
    flow_ref.value?.updateNodeStatus(node.id, 'pending')
  })
}

const save_execution_history = () => {
  try {
    saveWorkflowExecutionHistory(workflow_id.value, execution_history.value)
  } catch (e) {
    console.error('Failed to save execution history:', e)
  }
}

const load_execution_history = () => {
  try {
    execution_history.value = loadWorkflowExecutionHistory(workflow_id.value)
  } catch (e) {
    console.error('Failed to load execution history:', e)
  }
}

const get_log_class = getWorkflowLogClass
const format_time = formatWorkflowLogTime
const format_date = formatWorkflowShortDate

const start_run = async () => {
  const toast = useToast()
  const graph = build_graph()
  
  // 清空当前执行结果
  step_results.value = {}
  close_result_panel()
  
  // 使用后端校验
  try {
    const issues = await invoke<any[]>('validate_workflow_graph', { graph })
  if (issues.length) {
      add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.validationFailed', { message: issues[0].message }), issues[0].node_id)
    toast.error(t('trafficAnalysis.workflowStudio.toasts.validationFailed', { message: issues[0].message }))
      return
    }
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.validationError', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.validationError', { error: String(e) }))
    return
  }
  
  try {
    // 创建新的执行记录
    start_new_execution()
    
    add_log('INFO', t('trafficAnalysis.workflowStudio.logs.workflowExecutionStarted', { name: workflow_name.value }))
    show_logs.value = true
    workflow_running.value = true
    const exec_id = await invoke<string>('start_workflow_run', { graph })
    current_exec_id.value = exec_id
    add_log(
      'SUCCESS',
      t('trafficAnalysis.workflowStudio.logs.workflowStarted'),
      undefined,
      t('trafficAnalysis.workflowStudio.logs.executionId', { id: exec_id })
    )
    toast.success(t('trafficAnalysis.workflowStudio.toasts.executionStarted', { id: exec_id }))
    
    // 保存最后运行的工作流ID
    localStorage.setItem('last_run_workflow_id', workflow_id.value)
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.startFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.startFailed', { error: String(e) }))
    complete_execution(false)
    workflow_running.value = false
    current_exec_id.value = null
  }
}

// 停止工作流执行
const stop_run = async () => {
  const toast = useToast()
  
  if (!current_exec_id.value) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.noRunningWorkflow'))
    return
  }
  
  try {
    add_log('INFO', t('trafficAnalysis.workflowStudio.logs.stoppingWorkflow'))
    await invoke('stop_workflow_run', { executionId: current_exec_id.value })
    add_log('WARN', t('trafficAnalysis.workflowStudio.logs.workflowStopped'))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowStopped'))
    workflow_running.value = false
    current_exec_id.value = null
    complete_execution(false)
    
    // 重置节点状态
    reset_node_status()
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.stopFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.stopFailed', { error: String(e) }))
  }
}

// 启动定时调度
const start_schedule = async () => {
  const toast = useToast()
  
  // 先保存工作流
  const saved = await save_workflow()
  if (!saved) return
  
  const config = get_schedule_config()
  if (!config) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.scheduleMissingTrigger'))
    return
  }
  
  try {
    console.log('[Schedule] Starting with config:', config)
    console.log('[Schedule] Workflow ID:', workflow_id.value)
    console.log('[Schedule] Workflow Name:', workflow_name.value)
    
    await invoke('start_workflow_schedule', {
      workflowId: workflow_id.value,
      workflowName: workflow_name.value,
      config,
    })
    
    schedule_running.value = true
    const time = `${config.hour}:${String(config.minute).padStart(2, '0')}`
    const interval_desc = config.trigger_type === 'interval'
      ? t('trafficAnalysis.workflowStudio.schedule.everySeconds', { seconds: config.interval_seconds })
      : config.trigger_type === 'daily'
        ? t('trafficAnalysis.workflowStudio.schedule.dailyAt', { time })
        : t('trafficAnalysis.workflowStudio.schedule.weeklyAt', { weekdays: config.weekdays, time })
    
    add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.scheduleStarted', { desc: interval_desc }))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.scheduleStarted', { desc: interval_desc }))
    show_logs.value = true
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.scheduleStartFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.scheduleStartFailed', { error: String(e) }))
  }
}

// 停止定时调度
const stop_schedule = async () => {
  const toast = useToast()
  
  try {
    await invoke('stop_workflow_schedule', {
      workflowId: workflow_id.value,
    })
    
    schedule_running.value = false
    add_log('INFO', t('trafficAnalysis.workflowStudio.logs.scheduleStopped'))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.scheduleStopped'))
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.scheduleStopFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.scheduleStopFailed', { error: String(e) }))
  }
}

// 检查当前工作流是否有运行中的调度
const check_schedule_status = async () => {
  try {
    const info = await invoke<any>('get_workflow_schedule', {
      workflowId: workflow_id.value,
    })
    if (info) {
      schedule_running.value = info.is_running
      schedule_info.value = info
    } else {
      schedule_running.value = false
      schedule_info.value = null
    }
  } catch {
    schedule_running.value = false
    schedule_info.value = null
  }
}

const save_workflow = async (silent = false): Promise<boolean> => {
  const toast = useToast()
  const graph = build_graph()
  graph.id = workflow_id.value
  graph.name = workflow_name.value
  if (!validate_tool_schemas(graph, silent)) return false
  const client_issues = validate_workflow_graph_client(graph)
  if (client_issues.length) {
    const first_issue = client_issues[0]
    if (!silent) {
      add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.validationFailed', { message: first_issue.message }), first_issue.node_id)
      toast.error(t('trafficAnalysis.workflowStudio.toasts.validationFailed', { message: first_issue.message }))
    }
    return false
  }
  
  try {
    const issues = await invoke<any[]>('validate_workflow_graph', { graph })
    if (issues.length) {
      if (!silent) {
        add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.validationFailed', { message: issues[0].message }), issues[0].node_id)
        toast.error(t('trafficAnalysis.workflowStudio.toasts.validationFailed', { message: issues[0].message }))
      }
      return false
    }

    await invoke('save_workflow_definition', {
      graph,
      description: workflow_description.value || null,
      tags: workflow_tags.value || null,
      isTemplate: false,
      isTool: workflow_is_tool.value
    })
    sync_current_workflow_to_list()
    // 新建/克隆后列表中不存在该项时，面板打开状态下主动刷新一次
    if (
      show_workflow_list_panel.value &&
      workflow_list_tab.value === 'workflows' &&
      !workflow_list.value.some((wf) => wf.id === workflow_id.value)
    ) {
      await load_workflow_list()
    }
    has_unsaved_changes.value = false
    if (!silent) {
      add_log(
        'SUCCESS',
        workflow_is_tool.value
          ? t('trafficAnalysis.workflowStudio.logs.workflowSavedAsTool', { name: workflow_name.value })
          : t('trafficAnalysis.workflowStudio.logs.workflowSaved', { name: workflow_name.value })
      )
      toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowSaved'))
    }
    return true
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.saveFailed', { error: String(e) }))
    if (!silent) {
      toast.error(t('trafficAnalysis.workflowStudio.toasts.saveFailed', { error: String(e) }))
    }
    return false
  }
}

const on_save_workflow_click = (_evt: MouseEvent) => {
  void save_workflow(false)
}

const handle_save_workflow_click = () => {
  on_save_workflow_click(new MouseEvent('click'))
}

// 自动保存（防抖）
const trigger_auto_save = () => {
  // 如果工作流名称为空，不自动保存
  if (!workflow_name.value.trim()) return
  
  // 如果工作流正在运行，不自动保存
  if (workflow_running.value) return
  if (is_loading_graph.value) return
  
  has_unsaved_changes.value = true
  
  // 清除之前的定时器
  if (auto_save_timer.value) {
    clearTimeout(auto_save_timer.value)
  }
  
  // 设置新的定时器
  auto_save_timer.value = setTimeout(async () => {
    try {
      is_auto_saving.value = true
      const saved = await save_workflow(true) // 静默保存
      if (saved) {
        has_unsaved_changes.value = false
      }
    } finally {
      is_auto_saving.value = false
    }
  }, AUTO_SAVE_DELAY)
}

// 流程图变化处理
const on_flowchart_change = () => {
  if (is_loading_graph.value) return
  trigger_auto_save()
}

const apply_graph_to_canvas = (graph: WorkflowGraph) => {
  is_loading_graph.value = true
  try {
    close_edge_mapping_dialog()
    applyWorkflowStudioGraphToCanvas(flow_ref.value as any, graph)
  } finally {
    is_loading_graph.value = false
  }
}

const load_workflow = async (id: string) => {
  const toast = useToast()
  try {
    const data = await invoke<any>('get_workflow_definition', { id })
    if (data && data.graph) {
      const graph = data.graph
      reset_schedule_state()
      workflow_id.value = graph.id
      workflow_name.value = graph.name
      workflow_description.value = data.description || ''
      workflow_tags.value = data.tags || ''
      workflow_version.value = graph.version || 'v1.0.0'
      workflow_is_tool.value = data.is_tool || false
      apply_graph_to_canvas(graph)
      has_unsaved_changes.value = false
      
      add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.workflowLoaded', { name: workflow_name.value }))
      
      // 加载该工作流的执行历史
      load_execution_history()
      // 检查调度状态
      await check_schedule_status()
    }
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.loadFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.loadFailed', { error: String(e) }))
  }
}

const delete_workflow = async (id: string) => {
  const toast = useToast()
  if (!(await dialog.confirm(t('trafficAnalysis.workflowStudio.confirm.deleteWorkflow')))) return
  
  try {
    await invoke('delete_workflow_definition', { id })
    workflow_list.value = workflow_list.value.filter(wf => wf.id !== id)
    toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowDeleted'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.deleteFailed', { error: String(e) }))
  }
}

const load_workflow_list = async () => {
  try {
    workflow_list.value = await invoke<any[]>('list_workflow_definitions', { isTemplate: false })
  } catch (e) {
    console.error('Failed to load workflow list:', e)
  }
}

// 将当前编辑中的工作流元数据同步到列表，确保列表实时反映变更
const sync_current_workflow_to_list = () => {
  const idx = workflow_list.value.findIndex((wf) => wf.id === workflow_id.value)
  if (idx === -1) return
  const current = workflow_list.value[idx] || {}
  workflow_list.value[idx] = {
    ...current,
    id: workflow_id.value,
    name: workflow_name.value || t('trafficAnalysis.workflowStudio.defaults.unnamedWorkflow'),
    description: workflow_description.value || '',
    tags: workflow_tags.value || '',
    version: workflow_version.value || 'v1.0.0',
    is_tool: workflow_is_tool.value
  }
}

// 过滤后的工作流列表
const filtered_workflow_list = computed(() => {
  const search = workflow_list_search.value.toLowerCase().trim()
  if (!search) return workflow_list.value
  return workflow_list.value.filter(wf => 
    wf.name?.toLowerCase().includes(search) ||
    wf.description?.toLowerCase().includes(search) ||
    wf.tags?.toLowerCase().includes(search)
  )
})

// 过滤后的模板列表
const filtered_template_list = computed(() => {
  const search = workflow_list_search.value.toLowerCase().trim()
  if (!search) return template_list.value
  return template_list.value.filter(tpl => 
    tpl.name?.toLowerCase().includes(search) ||
    tpl.description?.toLowerCase().includes(search) ||
    tpl.tags?.toLowerCase().includes(search)
  )
})

// 切换工作流列表面板
const toggle_workflow_list_panel = () => {
  show_workflow_list_panel.value = !show_workflow_list_panel.value
  if (show_workflow_list_panel.value) {
    if (workflow_list_tab.value === 'workflows') {
      load_workflow_list()
    } else {
      load_template_list()
    }
  }
}

// 切换 tab
const switch_workflow_list_tab = (tab: 'workflows' | 'templates') => {
  workflow_list_tab.value = tab
  workflow_list_search.value = ''
  if (tab === 'workflows') {
    load_workflow_list()
  } else {
    load_template_list()
  }
}

// 从面板加载工作流
const load_workflow_from_panel = async (id: string) => {
  await load_workflow(id)
}

// 编辑工作流元数据
const edit_workflow_metadata = async (id: string) => {
  await load_workflow(id)
  if (workflow_id.value === id) {
    show_meta_dialog.value = true
  }
}

// 克隆工作流
const clone_workflow = async (id: string) => {
  const toast = useToast()
  try {
    await load_workflow(id)
    workflow_id.value = `wf_${Date.now()}`
    reset_schedule_state()
    workflow_name.value = t('trafficAnalysis.workflowStudio.defaults.duplicateWorkflowName', { name: workflow_name.value })
    has_unsaved_changes.value = true
    toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowDuplicated'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.duplicateFailed', { error: String(e) }))
  }
}

// 删除模板
const delete_template = async (id: string) => {
  const toast = useToast()
  if (!(await dialog.confirm(t('trafficAnalysis.workflowStudio.confirm.deleteTemplate')))) return
  
  try {
    await invoke('delete_workflow_definition', { id })
    template_list.value = template_list.value.filter(tpl => tpl.id !== id)
    toast.success(t('trafficAnalysis.workflowStudio.toasts.templateDeleted'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.deleteTemplateFailed', { error: String(e) }))
  }
}

// 新建工作流
const create_new_workflow = () => {
  if (has_unsaved_changes.value && workflow_name.value.trim()) {
    show_new_workflow_confirm.value = true
  } else {
    do_create_new_workflow()
  }
}

const do_create_new_workflow = () => {
  do_new_workflow()
  has_unsaved_changes.value = false
}

const toggle_favorite = (node_type: string) => {
  if (favorites.value.has(node_type)) {
    favorites.value.delete(node_type)
  } else {
    favorites.value.add(node_type)
  }
  // 保存到localStorage
  localStorage.setItem('workflow_favorites', JSON.stringify(Array.from(favorites.value)))
}

const on_sidebar_resize_mousemove = (e: MouseEvent) => {
  if (!is_resizing_sidebar.value) return
  const delta = e.clientX - sidebar_resize_start_x.value
  sidebar_width.value = clamp_sidebar_width(sidebar_resize_start_width.value + delta)
}

const stop_sidebar_resize = () => {
  if (!is_resizing_sidebar.value) return
  is_resizing_sidebar.value = false
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', on_sidebar_resize_mousemove)
  window.removeEventListener('mouseup', stop_sidebar_resize)
  localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebar_width.value))
}

const start_sidebar_resize = (e: MouseEvent) => {
  if (sidebar_collapsed.value) return
  e.preventDefault()
  e.stopPropagation()
  is_resizing_sidebar.value = true
  sidebar_resize_start_x.value = e.clientX
  sidebar_resize_start_width.value = sidebar_width.value
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', on_sidebar_resize_mousemove)
  window.addEventListener('mouseup', stop_sidebar_resize)
}

const is_favorite = (node_type: string) => {
  return favorites.value.has(node_type)
}

const validate_json = (key: string) => {
  const value = param_values.value[key]
  if (!value || typeof value !== 'string') {
    delete json_errors.value[key]
    return
  }
  
  try {
    JSON.parse(value)
    delete json_errors.value[key]
  } catch (e: any) {
    json_errors.value[key] = t('trafficAnalysis.workflowStudio.errors.jsonFormatError', { message: e.message })
  }
}

const has_validation_errors = computed(() => {
  return Object.keys(json_errors.value).length > 0
})

// 导出工作流为JSON
const export_workflow_json = () => {
  const toast = useToast()
  try {
    const graph = build_graph()
    const export_data = {
      workflow: graph,
      metadata: {
        description: workflow_description.value,
        tags: workflow_tags.value,
        is_tool: workflow_is_tool.value,
        exported_at: new Date().toISOString(),
        exported_by: t('trafficAnalysis.workflowStudio.export.exportedBy')
      }
    }
    
    const json_str = JSON.stringify(export_data, null, 2)
    const blob = new Blob([json_str], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${workflow_name.value.replace(/[^a-zA-Z0-9]/g, '_')}_${Date.now()}.json`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    
    add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.workflowExported', { filename: a.download }))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowExported'))
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.exportFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.exportFailed', { error: String(e) }))
  }
}

// 触发文件选择
const trigger_import_file = () => {
  import_file_input.value?.click()
}

// 导入工作流JSON
const import_workflow_json = async (event: Event) => {
  const toast = useToast()
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  
  if (!file) return
  
  try {
    const text = await file.text()
    const data = JSON.parse(text)
    
    // 检查数据格式
    if (!data.workflow || !data.workflow.nodes) {
      throw new Error(t('trafficAnalysis.workflowStudio.errors.invalidWorkflowFile'))
    }
    
    const graph = data.workflow
    
    // 重新生成ID避免冲突
    reset_schedule_state()
    workflow_id.value = `wf_${Date.now()}`
    workflow_name.value = graph.name || t('trafficAnalysis.workflowStudio.defaults.importedWorkflow')
    workflow_description.value = data.metadata?.description || ''
    workflow_tags.value = data.metadata?.tags || ''
    workflow_version.value = graph.version || 'v1.0.0'
    workflow_is_tool.value = data.metadata?.is_tool || false
    apply_graph_to_canvas(graph)
    has_unsaved_changes.value = true
    
    add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.workflowImported', { name: workflow_name.value }))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.workflowImported'))
    
    // 清空文件输入
    target.value = ''
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.importFailed', { message: e.message }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.importFailed', { message: e.message }))
    target.value = ''
  }
}

// 导出工作流为图片
const export_workflow_image = async () => {
  const toast = useToast()
  try {
    // 使用html2canvas库导出（需要先安装）
    toast.info(t('trafficAnalysis.workflowStudio.toasts.imageExportRequiresHtml2Canvas'))
    add_log('INFO', t('trafficAnalysis.workflowStudio.logs.imageExportTodo'))
    // TODO: 实现图片导出
    // const canvas = await html2canvas(flowchartContainer)
    // const url = canvas.toDataURL('image/png')
    // download(url, `${workflow_name.value}.png`)
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.exportFailed', { error: String(e) }))
  }
}

const wf_events = useWorkflowEvents()
const setup_event_listeners = async () => {
  // 监听工作流开始事件（定时触发或其他外部触发时会收到此事件）
  await wf_events.on_run_start((p: any) => {
    const exec_id = p?.exec_id || p?.execId || p?.execution_id || p?.executionId
    const workflow_id_from_event = p?.workflow_id || p?.workflowId
    
    // 只处理当前工作流的事件
    if (workflow_id_from_event && workflow_id_from_event !== workflow_id.value) {
      return
    }
    
    // 设置运行状态
    workflow_running.value = true
    if (exec_id) {
      current_exec_id.value = exec_id
    }
    
    // 如果是外部触发（定时等），需要创建执行记录
    if (!current_execution_id.value || !execution_history.value.find(e => e.id === current_execution_id.value && e.status === 'running')) {
      const id = start_new_execution()
      add_log(
        'INFO',
        t('trafficAnalysis.workflowStudio.logs.workflowExecutionStartedExternal'),
        undefined,
        t('trafficAnalysis.workflowStudio.logs.executionId', { id: exec_id || id })
      )
      show_logs.value = true
      // 重置节点状态
      reset_node_status()
    }
  })
  
  await wf_events.on_step_start((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) {
      flow_ref.value?.updateNodeStatus(step_id, 'running')
      add_log('INFO', t('trafficAnalysis.workflowStudio.logs.nodeStarted'), step_id)
    }
  })
  await wf_events.on_step_complete((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) {
      flow_ref.value?.updateNodeStatus(step_id, 'completed')
      
      // 保存步骤结果到当前执行和执行历史
      if (p && Object.prototype.hasOwnProperty.call(p, 'result')) {
        const result = p.result
        step_results.value[step_id] = result
        update_execution_step_result(step_id, result)
        
        const result_preview = truncateWorkflowLogDetails(result, MAX_LOG_DETAILS_LENGTH)
        add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.nodeCompleted'), step_id, result_preview)
      } else {
        add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.nodeCompleted'), step_id)
      }
    }
  })
  await wf_events.on_run_complete(async (p: any) => {
    add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.workflowCompleted'))
    complete_execution(true)
    
    // 保存执行ID用于查看详情
    const exec_id = current_exec_id.value
    
    workflow_running.value = false
    current_exec_id.value = null
    
    // 直接弹出执行详情对话框
    if (exec_id) {
      await view_execution_detail(exec_id)
    }
    
    // 延迟清除节点执行状态
    setTimeout(() => {
      if (!workflow_running.value && !current_exec_id.value) {
        reset_node_status()
      }
    }, 1500)
  })
  
  // 监听工作流停止事件
  await wf_events.on_run_stop((p: any) => {
    add_log('WARN', t('trafficAnalysis.workflowStudio.logs.workflowExecutionStopped'))
    workflow_running.value = false
    current_exec_id.value = null
    complete_execution(false)
    reset_node_status()
  })
}

const on_node_click = (node: any) => {
  close_edge_mapping_dialog()
  ignore_close_once.value = true
  selected_node.value = node
  const current = node.params || {}
  
  // 获取节点的参数 schema
  const item = catalog_index.value.get(node.type)
  const schema = item?.params_schema
  
  // 转换参数供编辑
  const converted: Record<string, any> = {}
  for (const [key, value] of Object.entries(current)) {
    const prop = schema?.properties?.[key]
    if (prop?.type === 'array' && Array.isArray(value)) {
      // 数组类型：转换为每行一个的格式（更友好）
      converted[key] = (value as any[]).map(v => typeof v === 'string' ? v : JSON.stringify(v)).join('\n')
    } else if (prop?.type === 'object' && typeof value === 'object' && value !== null) {
      // 对象类型：转换为格式化的 JSON 字符串
      converted[key] = JSON.stringify(value, null, 2)
    } else {
      converted[key] = value
    }
  }
  
  param_values.value = converted
  // 打开参数编辑抽屉
  drawer_open.value = true
}

const save_params = () => {
  if (!selected_node.value) return
  
  // 解析参数
  const parsed_params: Record<string, any> = {}
  const schema = selected_schema.value
  
  for (const [key, value] of Object.entries(param_values.value)) {
    const prop = schema?.properties?.[key]
    if (prop?.type === 'array' && typeof value === 'string') {
      // 数组类型：支持每行一个的格式，也支持 JSON 格式
      const trimmed = (value as string).trim()
      if (trimmed) {
        // 首先尝试解析为 JSON 数组
        if (trimmed.startsWith('[')) {
          try {
            parsed_params[key] = JSON.parse(trimmed)
            continue
          } catch { /* 不是有效 JSON，继续按行解析 */ }
        }
        // 按行解析：每行一个元素，过滤空行
        const lines = trimmed.split('\n').map(line => line.trim()).filter(line => line.length > 0)
        parsed_params[key] = lines
      } else {
        parsed_params[key] = []
      }
    } else if (prop?.type === 'object' && typeof value === 'string') {
      // 对象类型：解析 JSON
      const trimmed = (value as string).trim()
      if (trimmed) {
        try {
          parsed_params[key] = JSON.parse(trimmed)
        } catch {
          parsed_params[key] = value
        }
      } else {
        parsed_params[key] = {}
      }
    } else {
      parsed_params[key] = value
    }
  }
  
  // 如果是通知节点，需要附加通知规则的配置信息
  if (selected_node.value.type === 'notify' && parsed_params.notification_rule_id) {
    const rule = notification_rules.value.find(r => r.id === parsed_params.notification_rule_id)
    if (rule) {
      // 将通知规则的channel和config附加到参数中，供工作流执行时使用
      parsed_params._notification_channel = rule.channel
      parsed_params._notification_config = rule.config
    }
  }
  
  flow_ref.value?.updateNodeParams(selected_node.value.id, parsed_params)
}

const cancel_edit = () => {
  selected_node.value = null
  param_values.value = {}
}

const close_drawer = () => {
  drawer_open.value = false
}

const save_params_and_close = () => {
  save_params()
  close_drawer()
}

// 加载模板列表
const load_template_list = async () => {
  try {
    template_list.value = await invoke<any[]>('list_workflow_definitions', { isTemplate: true })
  } catch (e) {
    console.error('Failed to load template list:', e)
  }
}

// 加载我的模板
const load_my_templates = async () => {
  await load_template_list()
}

// 使用模板
const use_template = async (id: string) => {
  const toast = useToast()
  try {
    await load_workflow(id)
    // 重新生成ID，避免覆盖模板
    workflow_id.value = `wf_${Date.now()}`
    reset_schedule_state()
    workflow_name.value = t('trafficAnalysis.workflowStudio.defaults.duplicateWorkflowName', { name: workflow_name.value })
    has_unsaved_changes.value = true
    show_workflow_list_panel.value = false
    toast.success(t('trafficAnalysis.workflowStudio.toasts.templateApplied'))
  } catch (e: any) {
    toast.error(t('trafficAnalysis.workflowStudio.toasts.applyTemplateFailed', { error: String(e) }))
  }
}

// 保存当前工作流为模板
const save_current_as_template = async () => {
  const toast = useToast()
  const graph = build_graph()
  
  try {
    await invoke('save_workflow_definition', {
      graph,
      description: workflow_description.value || null,
      tags: workflow_tags.value || null,
      isTemplate: true,
      isTool: false // 模板不设为工具
    })
    add_log('SUCCESS', t('trafficAnalysis.workflowStudio.logs.templateSaved', { name: workflow_name.value }))
    toast.success(t('trafficAnalysis.workflowStudio.toasts.templateSaved'))
    await load_template_list()
  } catch (e: any) {
    add_log('ERROR', t('trafficAnalysis.workflowStudio.logs.templateSaveFailed', { error: String(e) }))
    toast.error(t('trafficAnalysis.workflowStudio.toasts.templateSaveFailed', { error: String(e) }))
  }
}

// 加载 AI 配置
const load_ai_config = async () => {
  try {
    ai_config.value = await invoke('get_ai_config')
  } catch (e) {
    console.error('Failed to load AI config:', e)
  }
}

// 加载可用工具列表
const load_available_tools = async () => {
  try {
    const tools = await invoke<any[]>('list_unified_tools')
    // 只保留可用的工具
    available_tools.value = tools.filter((t: any) => t.available)
  } catch (e) {
    console.error('Failed to load available tools:', e)
  }
}

// 获取已启用的 AI 提供商列表
const get_enabled_providers = () => {
  if (!ai_config.value?.providers) return []
  return Object.keys(ai_config.value.providers).filter(key => {
    const provider = ai_config.value.providers[key]
    return provider && provider.enabled === true
  })
}

// 获取指定提供商的模型列表
const get_provider_models = (providerKey: string) => {
  if (!providerKey || !ai_config.value?.providers) return []
  const provider = Object.keys(ai_config.value.providers).find(key => 
    key.toLowerCase() === providerKey.toLowerCase()
  )
  if (!provider) return []
  return ai_config.value.providers[provider]?.models || []
}

// 切换工具选择
const toggle_tool_selection = (key: string, toolName: string) => {
  if (!param_values.value[key]) {
    param_values.value[key] = []
  }
  const arr = param_values.value[key] as string[]
  const idx = arr.indexOf(toolName)
  if (idx === -1) {
    arr.push(toolName)
  } else {
    arr.splice(idx, 1)
  }
}

// 监听工作流元数据变化，触发自动保存
watch([workflow_name, workflow_description, workflow_tags, workflow_version, workflow_is_tool], () => {
  sync_current_workflow_to_list()
  trigger_auto_save()
})

// 搜索变化时清除高亮
const on_search_change = () => {
  highlighted_nodes.value.clear()
}

// 清空搜索
const clear_search = () => {
  search_query.value = ''
  highlighted_nodes.value.clear()
}

// 在画布中搜索节点
const search_in_canvas = () => {
  const query = search_query.value.trim().toLowerCase()
  if (!query) return
  
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const matches = nodes.filter(n => 
    n.name.toLowerCase().includes(query) || 
    n.type.toLowerCase().includes(query) ||
    n.description?.toLowerCase().includes(query)
  )
  
  highlighted_nodes.value = new Set(matches.map(n => n.id))
  
  if (matches.length > 0) {
    add_log('INFO', t('trafficAnalysis.workflowStudio.logs.foundMatchingNodes', { count: matches.length }))
    // 滚动到第一个匹配的节点
    const first = matches[0]
    // TODO: 实现画布滚动到节点位置
  } else {
    add_log('WARN', t('trafficAnalysis.workflowStudio.logs.noMatchingNodes'))
  }
}

// 处理键盘事件
const handle_keydown = (e: KeyboardEvent) => {
  // ESC 键关闭对话框（栈式关闭）
  if (e.key === 'Escape') {
    if (show_detail_dialog.value) {
      show_detail_dialog.value = false
      detail_dialog_fullscreen.value = false
      e.preventDefault()
      e.stopPropagation()
    }
  }
}

const handle_global_click = (e: MouseEvent) => {
  // 栈式关闭逻辑：先开后关
  // 如果详情对话框打开了，优先处理对话框的关闭
  if (show_detail_dialog.value) {
    const dialogEl = getWorkflowPanelsDetailDialogElement()
    // 检查点击是否在对话框的 modal-box 内部
    const modalBox = dialogEl?.querySelector('.modal-box')
    if (modalBox && !modalBox.contains(e.target as Node)) {
      // 点击在对话框外部，只关闭对话框
      show_detail_dialog.value = false
      detail_dialog_fullscreen.value = false
    }
    // 无论如何都不处理其他面板的关闭
    return
  }


  // 处理执行历史面板的关闭
  if (show_execution_history.value) {
    if (ignore_execution_history_close_once.value) {
      ignore_execution_history_close_once.value = false
    } else {
      const historyPanel = getExecutionHistoryElement()
      if (!historyPanel || !historyPanel.contains(e.target as Node)) {
        show_execution_history.value = false
      }
    }
  }
  
  // 处理结果面板的关闭
  if (show_result_panel.value) {
    if (ignore_result_panel_close_once.value) {
      ignore_result_panel_close_once.value = false
    } else {
      const panel = getWorkflowPanelsResultPanelElement()
      if (!panel || !panel.contains(e.target as Node)) {
        close_result_panel()
      }
    }
  }
}

const handle_global_mousedown = (_e: MouseEvent) => {
  // 抽屉关闭已通过遮罩层点击处理
}

onMounted(async () => {
  await refresh_catalog()
  await setup_event_listeners()
  load_notification_rules()
  load_ai_config()
  load_available_tools()
  
  // 从localStorage加载收藏
  const saved_favorites = localStorage.getItem('workflow_favorites')
  if (saved_favorites) {
    try {
      const arr = JSON.parse(saved_favorites)
      favorites.value = new Set(arr)
    } catch (e) {
      console.error('Failed to load favorites:', e)
    }
  }

  await nextTick()
  sidebar_transition_ready.value = true
  
  // 加载上次运行的工作流
  const last_workflow_id = localStorage.getItem('last_run_workflow_id')
  if (last_workflow_id) {
    try {
      await load_workflow(last_workflow_id)
      // 检查调度状态
      await check_schedule_status()
    } catch (e) {
      console.error('Failed to load last workflow:', e)
    }
  }
  
  window.addEventListener('click', handle_global_click)
  window.addEventListener('mousedown', handle_global_mousedown)
  window.addEventListener('keydown', handle_keydown)

  const routeExecutionId = typeof route.query.execution_id === 'string'
    ? route.query.execution_id
    : typeof route.query.executionId === 'string'
      ? route.query.executionId
      : ''
  if (routeExecutionId) {
    await open_execution_from_route(routeExecutionId)
  }
})

watch(
  () => [route.query.execution_id, route.query.executionId],
  async ([executionId, executionIdCamel]) => {
    const nextExecutionId = typeof executionId === 'string'
      ? executionId
      : typeof executionIdCamel === 'string'
        ? executionIdCamel
        : ''
    if (!nextExecutionId) {
      last_route_execution_id.value = null
      return
    }
    await open_execution_from_route(nextExecutionId)
  },
)

onUnmounted(() => {
  wf_events.unsubscribe_all()
  stop_sidebar_resize()
  // 清除自动保存定时器
  if (auto_save_timer.value) {
    clearTimeout(auto_save_timer.value)
  }
  window.removeEventListener('click', handle_global_click)
  window.removeEventListener('mousedown', handle_global_mousedown)
  window.removeEventListener('keydown', handle_keydown)
})
</script>

<style scoped>
/* 左侧抽屉滑入滑出动画 */
.drawer-enter-active,
.drawer-leave-active {
  transition: transform 0.25s ease;
}

.drawer-enter-from,
.drawer-leave-to {
  transform: translateX(-100%);
}

/* 右侧抽屉滑入滑出动画 */
.drawer-right-enter-active,
.drawer-right-leave-active {
  transition: transform 0.25s ease;
}

.drawer-right-enter-from,
.drawer-right-leave-to {
  transform: translateX(100%);
}

/* 遮罩淡入淡出动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
