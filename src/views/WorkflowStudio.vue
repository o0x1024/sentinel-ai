<template>
  <div class="p-4 space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold">{{ t('passiveScan.workflowStudio.title') }}</h1>
          <input v-model="workflow_name" class="input input-bordered input-sm w-48" :placeholder="t('passiveScan.workflowStudio.header.namePlaceholder')" />
          <button class="btn btn-xs btn-ghost" @click="show_meta_dialog = true" :title="t('passiveScan.workflowStudio.header.editMetadataTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
          </button>
          
          <div class="text-xs text-base-content/60 ml-2" v-if="workflow_name.trim()">
            <span v-if="is_auto_saving" class="flex items-center gap-1">
              <span class="loading loading-spinner loading-xs"></span>
              {{ t('passiveScan.workflowStudio.status.saving') }}
            </span>
            <span v-else-if="has_unsaved_changes" class="text-warning">
              {{ t('passiveScan.workflowStudio.status.unsaved') }}
            </span>
            <span v-else class="text-success">
              {{ t('passiveScan.workflowStudio.status.saved') }}
            </span>
          </div>
        </div>
        <div class="flex gap-2">
          <button class="btn btn-sm btn-outline" @click="show_load_dialog = true" :title="t('passiveScan.workflowStudio.toolbar.loadTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.load') }}
          </button>
          <button class="btn btn-sm btn-outline" @click="show_template_dialog = true" :title="t('passiveScan.workflowStudio.toolbar.templateMarketTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.templates') }}
          </button>
          <button class="btn btn-sm btn-primary" @click="on_save_workflow_click" :disabled="!workflow_name.trim()" :title="t('passiveScan.workflowStudio.toolbar.saveTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.save') }}
          </button>
          <div class="dropdown dropdown-end">
            <button tabindex="0" class="btn btn-sm btn-outline" :title="t('passiveScan.workflowStudio.toolbar.exportImportTooltip')">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
              </svg>
              <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52 z-50">
              <li><a @click="export_workflow_json">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                </svg>
                {{ t('passiveScan.workflowStudio.export.exportJson') }}
              </a></li>
              <li><a @click="trigger_import_file">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                </svg>
                {{ t('passiveScan.workflowStudio.export.importJson') }}
              </a></li>
              <li><a @click="export_workflow_image">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
                {{ t('passiveScan.workflowStudio.export.exportImage') }}
              </a></li>
            </ul>
          </div>
          <input ref="import_file_input" type="file" accept=".json" class="hidden" @change="import_workflow_json" />
          <button class="btn btn-sm btn-outline" @click="refresh_catalog" :title="t('passiveScan.workflowStudio.toolbar.refreshCatalogTooltip')">{{ t('passiveScan.workflowStudio.toolbar.refreshCatalog') }}</button>
          <button class="btn btn-sm btn-outline" @click="reset_canvas" :title="t('passiveScan.workflowStudio.toolbar.resetCanvasTooltip')">{{ t('passiveScan.workflowStudio.toolbar.resetCanvas') }}</button>
          <button 
            v-if="!workflow_running" 
            class="btn btn-sm btn-success" 
            @click="start_run" 
            :title="t('passiveScan.workflowStudio.toolbar.runTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.run') }}
          </button>
          <button 
            v-else 
            class="btn btn-sm btn-error" 
            @click="stop_run" 
            :title="t('passiveScan.workflowStudio.toolbar.stopTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.stop') }}
          </button>
          <!-- 定时调度按钮 -->
          <button 
            v-if="!schedule_running" 
            class="btn btn-sm btn-warning" 
            @click="start_schedule" 
            :disabled="!workflow_name.trim() || !has_schedule_trigger"
            :title="has_schedule_trigger ? t('passiveScan.workflowStudio.toolbar.startScheduleTooltip') : t('passiveScan.workflowStudio.toolbar.startScheduleDisabledTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.schedule') }}
          </button>
          <button 
            v-else 
            class="btn btn-sm btn-error" 
            @click="stop_schedule" 
            :title="t('passiveScan.workflowStudio.toolbar.stopScheduleTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.stop') }}
          </button>
          <button 
            class="btn btn-sm" 
            :class="show_logs ? 'btn-primary' : 'btn-ghost'" 
            @click="show_logs = !show_logs" 
            :title="t('passiveScan.workflowStudio.toolbar.toggleLogsTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.logs') }}
          </button>
          <button 
            class="btn btn-sm" 
            :class="show_execution_history ? 'btn-secondary' : 'btn-ghost'" 
            @click="toggle_execution_history" 
            :title="t('passiveScan.workflowStudio.toolbar.executionHistoryTooltip')"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            {{ t('passiveScan.workflowStudio.toolbar.history') }}
            <span v-if="execution_history.length" class="badge badge-xs badge-primary ml-1">{{ execution_history.length }}</span>
          </button>
        </div>
      </div>

    <div class="grid grid-cols-12 gap-4">
      <div :class="sidebar_collapsed ? 'col-span-1' : 'col-span-3'" class="transition-all duration-300">
        <div class="card bg-base-100 shadow-xl h-full">
          <div class="card-body p-3">
            <div class="flex items-center justify-between mb-2">
              <h2 v-if="!sidebar_collapsed" class="text-base font-semibold">{{ t('passiveScan.workflowStudio.sidebar.nodeLibrary') }}</h2>
              <button class="btn btn-xs btn-ghost" @click="sidebar_collapsed = !sidebar_collapsed" :title="sidebar_collapsed ? t('passiveScan.workflowStudio.sidebar.expandSidebar') : t('passiveScan.workflowStudio.sidebar.collapseSidebar')">
                <svg v-if="sidebar_collapsed" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                </svg>
                <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
                </svg>
              </button>
            </div>
            <div v-if="!sidebar_collapsed">
              <div class="relative mb-2">
                <input v-model="search_query" class="input input-bordered input-sm w-full pr-16" :placeholder="t('passiveScan.workflowStudio.sidebar.searchPlaceholder')" @input="on_search_change" />
                <button v-if="search_query" class="btn btn-xs btn-ghost absolute right-8 top-1/2 -translate-y-1/2" @click="clear_search" :title="t('passiveScan.workflowStudio.sidebar.clearSearchTooltip')">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
                <button class="btn btn-xs btn-ghost absolute right-1 top-1/2 -translate-y-1/2" @click="search_in_canvas" :title="t('passiveScan.workflowStudio.sidebar.searchInCanvasTooltip')" :disabled="!search_query">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                  </svg>
                </button>
              </div>
              <div class="form-control mb-2">
                <label class="label cursor-pointer py-1">
                  <span class="label-text text-xs">{{ t('passiveScan.workflowStudio.sidebar.favoritesOnly') }}</span>
                  <input type="checkbox" v-model="show_favorites_only" class="checkbox checkbox-xs" />
                </label>
              </div>
              <div class="space-y-2 overflow-y-auto" style="max-height: calc(100vh - 250px)">
                <div v-if="filtered_groups.length === 0" class="text-center text-sm text-base-content/60 py-4">
                  {{ t('passiveScan.workflowStudio.sidebar.noMatchingNodes') }}
                </div>
              <div v-for="group in filtered_groups" :key="group.name" class="collapse collapse-arrow bg-base-200">
                  <input type="checkbox" :checked="group.name === 'tool'" />
                  <div class="collapse-title text-sm font-medium py-2">
                    {{ group.label }} ({{ group.items.length }})
                  </div>
                <div class="collapse-content">
                  <!-- MCP/Plugin 单列显示，其他双列 -->
                  <div :class="['mcp', 'plugin'].includes(group.name) ? 'flex flex-col gap-1' : 'grid grid-cols-2 gap-2'">
                    <button
                      v-for="item in group.items"
                      :key="item.node_type"
                        class="btn btn-xs relative text-left justify-start"
                      @click="add_node(item)"
                        :title="item.node_type"
                      >
                        <span class="truncate flex-1">{{ item.label }}</span>
                        <button 
                          class="btn btn-ghost btn-xs p-0 w-4 h-4 ml-1 flex-shrink-0"
                          @click.stop="toggle_favorite(item.node_type)"
                          :title="is_favorite(item.node_type) ? t('passiveScan.workflowStudio.sidebar.unfavorite') : t('passiveScan.workflowStudio.sidebar.favorite')"
                        >
                          <span v-if="is_favorite(item.node_type)">⭐</span>
                          <span v-else class="opacity-40">☆</span>
                        </button>
                    </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div :class="sidebar_collapsed ? 'col-span-11' : 'col-span-9'" class="transition-all duration-300">
        <FlowchartVisualization ref="flow_ref" @nodeClick="on_node_click" @newWorkflow="on_new_workflow" @change="on_flowchart_change" :highlightedNodes="highlighted_nodes" />
      </div>
    </div>

    <!-- 执行日志面板 -->
    <div v-if="show_logs" class="card bg-base-100 shadow-xl mt-4">
      <div class="card-body p-3">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-base font-semibold">{{ t('passiveScan.workflowStudio.logs.title') }}</h2>
          <div class="flex gap-2">
            <button class="btn btn-xs btn-outline" @click="clear_logs">{{ t('passiveScan.workflowStudio.logs.clear') }}</button>
            <button class="btn btn-xs btn-ghost" @click="show_logs = false">✕</button>
          </div>
        </div>
        <div class="overflow-y-auto bg-base-200 rounded p-2 font-mono text-xs" style="max-height: 300px">
          <div v-if="execution_logs.length === 0" class="text-center text-base-content/60 py-4">
            {{ t('passiveScan.workflowStudio.logs.empty') }}
          </div>
          <div v-for="(log, idx) in execution_logs" :key="idx" class="mb-1">
            <div :class="get_log_class(log.level)">
              <span class="opacity-60">[{{ format_time(log.timestamp) }}]</span>
              <span class="font-semibold">[{{ log.level }}]</span>
              <span v-if="log.node_id" class="text-primary">[{{ log.node_id }}]</span>
              <span>{{ log.message }}</span>
              <button v-if="log.details" 
                class="btn btn-xs btn-ghost ml-2" 
                @click="toggle_log_details(idx)"
                :title="expanded_logs.has(idx) ? t('passiveScan.workflowStudio.logs.collapseDetails') : t('passiveScan.workflowStudio.logs.expandDetails')">
                {{ expanded_logs.has(idx) ? '▼' : '▶' }}
              </button>
            </div>
            <pre v-if="log.details && expanded_logs.has(idx)" 
              class="ml-4 mt-1 text-xs opacity-80 bg-base-300 p-2 rounded overflow-x-auto max-h-60">{{ log.details }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- 加载工作流对话框 -->
    <dialog :open="show_load_dialog" class="modal" @click.self="show_load_dialog = false">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('passiveScan.workflowStudio.loadDialog.title') }}</h3>
        <div class="space-y-2 max-h-96 overflow-y-auto">
          <div v-if="workflow_list.length === 0" class="text-center text-base-content/60 py-8">
            {{ t('passiveScan.workflowStudio.loadDialog.empty') }}
          </div>
          <div 
            v-for="wf in workflow_list" 
            :key="wf.id" 
            class="card bg-base-200 hover:bg-base-300 cursor-pointer transition-colors"
            @click="load_workflow(wf.id)"
          >
            <div class="card-body p-3">
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <h4 class="font-semibold">{{ wf.name }}</h4>
                  <p v-if="wf.description" class="text-sm text-base-content/70 mt-1">{{ wf.description }}</p>
                  <div class="flex gap-2 mt-2 text-xs text-base-content/60">
                    <span>{{ t('passiveScan.workflowStudio.loadDialog.version', { version: wf.version }) }}</span>
                    <span>{{ t('passiveScan.workflowStudio.loadDialog.updated', { date: format_date(wf.updated_at) }) }}</span>
                    <span v-if="wf.tags" class="badge badge-xs">{{ wf.tags }}</span>
                  </div>
                </div>
                <button class="btn btn-xs btn-error btn-ghost" @click.stop="delete_workflow(wf.id)" :title="t('passiveScan.workflowStudio.loadDialog.deleteTooltip')">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-sm" @click="show_load_dialog = false">{{ t('passiveScan.workflowStudio.loadDialog.close') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 模板市场对话框 -->
    <dialog :open="show_template_dialog" class="modal" @click.self="show_template_dialog = false">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ t('passiveScan.workflowStudio.templateMarket.title') }}</h3>
        
        <div class="tabs tabs-boxed mb-4">
          <a class="tab tab-active">{{ t('passiveScan.workflowStudio.templateMarket.recommended') }}</a>
          <a class="tab" @click="load_my_templates">{{ t('passiveScan.workflowStudio.templateMarket.myTemplates') }}</a>
        </div>
        
        <div class="grid grid-cols-2 gap-4 max-h-96 overflow-y-auto">
          <div v-if="template_list.length === 0" class="col-span-2 text-center text-base-content/60 py-8">
            {{ t('passiveScan.workflowStudio.templateMarket.empty') }}
          </div>
          
          <div 
            v-for="tpl in template_list" 
            :key="tpl.id" 
            class="card bg-base-200 hover:bg-base-300 cursor-pointer transition-colors"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <h4 class="font-semibold flex items-center gap-2">
                    {{ tpl.name }}
                    <span v-if="tpl.is_template" class="badge badge-primary badge-xs">{{ t('passiveScan.workflowStudio.templateMarket.templateBadge') }}</span>
                  </h4>
                  <p v-if="tpl.description" class="text-sm text-base-content/70 mt-1 line-clamp-2">{{ tpl.description }}</p>
                  <div class="flex gap-2 mt-2 text-xs text-base-content/60">
                    <span>{{ t('passiveScan.workflowStudio.templateMarket.nodeCount', { count: tpl.node_count || 0 }) }}</span>
                    <span v-if="tpl.tags" class="badge badge-xs">{{ tpl.tags }}</span>
                  </div>
                </div>
              </div>
              <div class="card-actions justify-end mt-2">
                <button class="btn btn-xs btn-primary" @click="use_template(tpl.id)">{{ t('passiveScan.workflowStudio.templateMarket.useTemplate') }}</button>
                <button v-if="!tpl.is_builtin" class="btn btn-xs btn-outline" @click="save_current_as_template">{{ t('passiveScan.workflowStudio.templateMarket.saveAsTemplate') }}</button>
              </div>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-sm btn-primary" @click="save_current_as_template">{{ t('passiveScan.workflowStudio.templateMarket.saveCurrentAsTemplate') }}</button>
          <button class="btn btn-sm" @click="show_template_dialog = false">{{ t('passiveScan.workflowStudio.templateMarket.close') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 新建工作流确认对话框 -->
    <dialog :open="show_new_workflow_confirm" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.title') }}</h3>
        <p class="text-base-content/80">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.message') }}</p>
        <div class="modal-action">
          <button class="btn btn-primary btn-sm" @click="confirm_new_workflow_save">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.saveAndNew') }}</button>
          <button class="btn btn-warning btn-sm" @click="confirm_new_workflow_discard">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.discardAndNew') }}</button>
          <button class="btn btn-ghost btn-sm" @click="show_new_workflow_confirm = false">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.cancel') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="show_new_workflow_confirm = false">{{ t('passiveScan.workflowStudio.newWorkflowConfirm.close') }}</button>
      </form>
    </dialog>

    <!-- 工作流元数据对话框 -->
    <dialog :open="show_meta_dialog" class="modal" @click.self="show_meta_dialog = false">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('passiveScan.workflowStudio.metaDialog.title') }}</h3>
        <div class="space-y-3">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('passiveScan.workflowStudio.metaDialog.name') }} <span class="text-error">*</span></span>
            </label>
            <input v-model="workflow_name" class="input input-bordered" :placeholder="t('passiveScan.workflowStudio.metaDialog.namePlaceholder')" />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('passiveScan.workflowStudio.metaDialog.description') }}</span>
            </label>
            <textarea v-model="workflow_description" class="textarea textarea-bordered" rows="3" :placeholder="t('passiveScan.workflowStudio.metaDialog.descriptionPlaceholder')"></textarea>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('passiveScan.workflowStudio.metaDialog.tags') }}</span>
            </label>
            <input v-model="workflow_tags" class="input input-bordered" :placeholder="t('passiveScan.workflowStudio.metaDialog.tagsPlaceholder')" />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('passiveScan.workflowStudio.metaDialog.version') }}</span>
            </label>
            <input v-model="workflow_version" class="input input-bordered" placeholder="v1.0.0" />
          </div>
          
          <!-- 设置为工具 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('passiveScan.workflowStudio.metaDialog.asAiTool') }}</span>
              <input type="checkbox" v-model="workflow_is_tool" class="toggle toggle-primary" />
            </label>
            <label class="label py-0">
              <span class="label-text-alt text-base-content/60">{{ t('passiveScan.workflowStudio.metaDialog.asAiToolHelp') }}</span>
            </label>
          </div>
          
          <div class="stats shadow w-full">
            <div class="stat py-2">
              <div class="stat-title text-xs">{{ t('passiveScan.workflowStudio.metaDialog.stats.nodes') }}</div>
              <div class="stat-value text-2xl">{{ flow_ref?.getFlowchartNodes().length || 0 }}</div>
            </div>
            <div class="stat py-2">
              <div class="stat-title text-xs">{{ t('passiveScan.workflowStudio.metaDialog.stats.edges') }}</div>
              <div class="stat-value text-2xl">{{ flow_ref?.getFlowchartEdges().length || 0 }}</div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-sm btn-primary" @click="show_meta_dialog = false" :disabled="!workflow_name.trim()">{{ t('passiveScan.workflowStudio.metaDialog.confirm') }}</button>
          <button class="btn btn-sm" @click="show_meta_dialog = false">{{ t('passiveScan.workflowStudio.metaDialog.cancel') }}</button>
        </div>
      </div>
    </dialog>

    <div v-if="drawer_open" ref="drawer_ref" class="fixed inset-y-0 right-0 w-[350px] bg-base-100 shadow-xl border-l border-base-300 z-50">
      <div class="p-3 flex items-center justify-between border-b border-base-300">
        <h2 class="text-base font-semibold">{{ t('passiveScan.workflowStudio.paramsEditor.title') }}</h2>
        <button class="btn btn-xs btn-ghost" @click="close_drawer">✕</button>
      </div>
      <div class="p-3 border-b border-base-300">
        <div class="text-sm font-semibold">{{ selected_node?.name }}</div>
        <div class="text-xs text-base-content/60 mt-1">{{ selected_node?.type }}</div>
      </div>
      <div class="p-3 space-y-3 overflow-auto h-[calc(100%-140px)]" v-if="selected_schema">
        <div v-if="!selected_schema.properties || Object.keys(selected_schema.properties).length === 0" class="text-center text-sm text-base-content/60 py-4">
          {{ t('passiveScan.workflowStudio.paramsEditor.noParams') }}
        </div>
        <div v-for="(prop, key) in selected_schema.properties" :key="key" class="form-control">
          <label class="label py-1">
            <span class="label-text text-xs font-semibold">
              {{ key }}
              <span v-if="selected_schema.required?.includes(key)" class="text-error">*</span>
            </span>
            <span v-if="prop.description" class="label-text-alt text-xs opacity-60" :title="prop.description">?</span>
          </label>
          
          <!-- 通知规则选择器 (特殊处理) -->
          <div v-if="String(key) === 'notification_rule_id' && selected_node?.type === 'notify'" class="space-y-2">
            <select 
              class="select select-bordered select-sm w-full" 
              v-model="param_values[key]"
              :class="{ 'select-error': selected_schema.required?.includes(key) && !param_values[key] }"
            >
              <option value="">{{ t('passiveScan.workflowStudio.paramsEditor.selectNotificationRule') }}</option>
              <option v-for="rule in notification_rules" :key="rule.id" :value="rule.id">
                {{ rule.type_name }} ({{ rule.channel }})
              </option>
            </select>
            <div v-if="notification_rules.length === 0" class="text-xs text-warning">
              <span>{{ t('passiveScan.workflowStudio.paramsEditor.noNotificationRules') }}</span>
              <router-link to="/notification-management" class="link link-primary">{{ t('passiveScan.workflowStudio.paramsEditor.goToConfigure') }}</router-link>
            </div>
          </div>
          
          <!-- AI 提供商选择器 -->
          <div v-else-if="prop['x-ui-widget'] === 'ai-provider-select'" class="space-y-2">
            <select 
              class="select select-bordered select-sm w-full" 
              v-model="param_values[key]"
            >
              <option value="">{{ t('passiveScan.workflowStudio.paramsEditor.useDefaultConfig') }}</option>
              <option v-for="provider in get_enabled_providers()" :key="provider" :value="provider">
                {{ provider }}
              </option>
            </select>
            <div v-if="get_enabled_providers().length === 0" class="text-xs text-warning">
              <span>{{ t('passiveScan.workflowStudio.paramsEditor.noAiProviders') }}</span>
              <router-link to="/settings" class="link link-primary">{{ t('passiveScan.workflowStudio.paramsEditor.goToConfigure') }}</router-link>
            </div>
          </div>
          
          <!-- AI 模型选择器 -->
          <div v-else-if="prop['x-ui-widget'] === 'ai-model-select'" class="space-y-2">
            <select 
              class="select select-bordered select-sm w-full" 
              v-model="param_values[key]"
              :disabled="!param_values['provider']"
            >
              <option value="">-- {{ param_values['provider'] ? t('passiveScan.workflowStudio.paramsEditor.selectModel') : t('passiveScan.workflowStudio.paramsEditor.selectProviderFirst') }} --</option>
              <option v-for="model in get_provider_models(param_values['provider'])" :key="model.id" :value="model.id">
                {{ model.name }}{{ model.description ? ' - ' + model.description : '' }}
              </option>
            </select>
          </div>
          
          <!-- 工具多选器 -->
          <div v-else-if="prop['x-ui-widget'] === 'tools-multiselect'" class="space-y-2">
            <div class="max-h-48 overflow-y-auto border border-base-300 rounded-lg p-2 space-y-1">
              <div v-if="available_tools.length === 0" class="text-xs text-base-content/60 text-center py-2">
                {{ t('passiveScan.workflowStudio.paramsEditor.noTools') }}
              </div>
              <label v-for="tool in available_tools" :key="tool.name" class="flex items-center gap-2 p-1 hover:bg-base-200 rounded cursor-pointer">
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-sm checkbox-primary" 
                  :value="tool.name"
                  :checked="(param_values[key] || []).includes(tool.name)"
                  @change="toggle_tool_selection(String(key), tool.name)"
                />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium truncate">{{ tool.name }}</div>
                  <div v-if="tool.description" class="text-xs text-base-content/60 truncate">{{ tool.description }}</div>
                </div>
              </label>
            </div>
            <div class="text-xs text-base-content/60">
              {{ t('passiveScan.workflowStudio.paramsEditor.selectedToolsCount', { count: (param_values[key] || []).length }) }}
            </div>
          </div>
          
          <!-- Textarea 类型 -->
          <textarea 
            v-else-if="prop['x-ui-widget'] === 'textarea'" 
            class="textarea textarea-bordered textarea-sm w-full" 
            v-model="param_values[key]"
            :placeholder="prop.default || t('passiveScan.workflowStudio.paramsEditor.enterField', { key: String(key) })"
            :class="{ 'textarea-error': selected_schema.required?.includes(key) && !param_values[key] }"
            rows="3"
          ></textarea>
          
          <!-- Textarea-lines 类型（每行一个值） -->
          <div v-else-if="prop['x-ui-widget'] === 'textarea-lines'" class="space-y-1">
            <textarea 
              class="textarea textarea-bordered textarea-sm font-mono text-xs w-full" 
              v-model="param_values[key]"
              :placeholder="prop.description || t('passiveScan.workflowStudio.paramsEditor.onePerLine')"
              :class="{ 'textarea-error': selected_schema.required?.includes(key) && !param_values[key] }"
              rows="4"
            ></textarea>
            <div class="text-xs text-base-content/50">{{ t('passiveScan.workflowStudio.paramsEditor.onePerLine') }}</div>
          </div>
          
          <!-- 字符串类型 -->
          <input 
            v-else-if="prop.type === 'string' && !prop.enum" 
            class="input input-bordered input-sm w-full" 
            v-model="param_values[key]"
            :placeholder="prop.default || t('passiveScan.workflowStudio.paramsEditor.enterField', { key: String(key) })"
            :class="{ 'input-error': selected_schema.required?.includes(key) && !param_values[key] }"
          />
          
          <!-- 数字类型 -->
          <input 
            v-else-if="prop.type === 'integer' || prop.type === 'float' || prop.type === 'number'" 
            type="number" 
            class="input input-bordered input-sm w-full" 
            v-model.number="param_values[key]"
            :placeholder="prop.default?.toString() || '0'"
            :min="prop.minimum"
            :max="prop.maximum"
            :step="prop.type === 'integer' ? 1 : 0.1"
          />
          
          <!-- 枚举类型 -->
          <select 
            v-else-if="prop.enum && prop.enum.length" 
            class="select select-bordered select-sm" 
            v-model="param_values[key]"
          >
            <option value="">{{ t('passiveScan.workflowStudio.paramsEditor.pleaseSelect') }}</option>
            <option v-for="opt in prop.enum" :key="opt" :value="opt">{{ opt }}</option>
          </select>
          
          <!-- 布尔类型 -->
          <div v-else-if="prop.type === 'boolean'" class="flex items-center gap-2">
            <input type="checkbox" class="toggle toggle-sm toggle-primary" v-model="param_values[key]" />
            <span class="text-xs">{{ param_values[key] ? t('passiveScan.workflowStudio.paramsEditor.booleanYes') : t('passiveScan.workflowStudio.paramsEditor.booleanNo') }}</span>
          </div>
          
          <!-- 数组类型：每行一个 -->
          <div v-else-if="prop.type === 'array'" class="space-y-1">
            <textarea 
              class="textarea textarea-bordered textarea-sm font-mono text-xs w-full" 
              v-model="param_values[key]"
              :placeholder="t('passiveScan.workflowStudio.paramsEditor.arrayPlaceholder')"
              rows="4"
            ></textarea>
            <div class="text-xs text-base-content/50">{{ t('passiveScan.workflowStudio.paramsEditor.onePerLine') }}</div>
          </div>
          
          <!-- 对象类型：JSON格式 -->
          <div v-else-if="prop.type === 'object'" class="space-y-1">
            <textarea 
              class="textarea textarea-bordered textarea-sm font-mono text-xs" 
              v-model="param_values[key]"
              placeholder='{ "key": "value" }'
              rows="4"
              @blur="validate_json(String(key))"
            ></textarea>
            <div v-if="json_errors[key]" class="text-xs text-error">{{ json_errors[key] }}</div>
          </div>
          
          <!-- 其他类型 -->
          <textarea 
            v-else 
            class="textarea textarea-bordered textarea-sm" 
            v-model="param_values[key]"
            rows="2"
          ></textarea>
          
          <!-- 参数说明 -->
          <label v-if="prop.description && prop.description.trim() && prop.description.trim() !== '/'" class="label py-0">
            <span class="label-text-alt text-xs opacity-60">{{ prop.description }}</span>
          </label>
          
          <!-- 默认值提示 -->
          <label v-if="prop.default !== undefined && !param_values[key]" class="label py-0">
            <span class="label-text-alt text-xs text-info">{{ t('passiveScan.workflowStudio.paramsEditor.defaultValue', { value: String(prop.default) }) }}</span>
          </label>
        </div>
      </div>
      <div class="p-3 flex gap-2 border-t border-base-300">
        <button class="btn btn-primary btn-sm flex-1" @click="save_params_and_close" :disabled="has_validation_errors">
          {{ t('passiveScan.workflowStudio.paramsEditor.save') }}
        </button>
        <button class="btn btn-outline btn-sm" @click="close_drawer">{{ t('passiveScan.workflowStudio.paramsEditor.cancel') }}</button>
      </div>
    </div>

    <!-- 执行历史面板 -->
    <div v-if="show_execution_history" ref="execution_history_ref" class="fixed inset-y-0 right-0 w-[700px] bg-base-100 shadow-xl border-l border-base-300 z-50 flex flex-col">
      <div class="p-3 flex items-center justify-between border-b border-base-300">
        <h2 class="text-base font-semibold">{{ t('passiveScan.workflowStudio.executionHistory.title') }}</h2>
        <button class="btn btn-xs btn-ghost" @click="show_execution_history = false">✕</button>
      </div>
      
      <!-- 搜索栏 -->
      <div class="p-3 border-b border-base-300">
        <div class="flex gap-2">
          <input 
            v-model="history_search_query" 
            class="input input-bordered input-sm flex-1" 
            :placeholder="t('passiveScan.workflowStudio.executionHistory.searchPlaceholder')"
            @keyup.enter="load_history_from_backend"
          />
          <button class="btn btn-sm btn-primary" @click="load_history_from_backend">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </button>
        </div>
      </div>
      
      <!-- 表格 -->
      <div class="flex-1 overflow-auto p-3">
        <table class="table table-sm table-zebra w-full">
          <thead class="sticky top-0 bg-base-100 z-10">
            <tr>
              <th class="w-48">{{ t('passiveScan.workflowStudio.executionHistory.table.name') }}</th>
              <th class="w-40">{{ t('passiveScan.workflowStudio.executionHistory.table.startTime') }}</th>
              <th class="w-24 text-right">{{ t('passiveScan.workflowStudio.executionHistory.table.duration') }}</th>
              <th class="w-24 text-center">{{ t('passiveScan.workflowStudio.executionHistory.table.status') }}</th>
              <th class="w-28 text-center">{{ t('passiveScan.workflowStudio.executionHistory.table.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="history_loading">
              <td colspan="5" class="text-center py-8">
                <span class="loading loading-spinner loading-md"></span>
              </td>
            </tr>
            <tr v-else-if="history_data.length === 0">
              <td colspan="5" class="text-center py-8 text-base-content/50">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <p class="text-sm">{{ t('passiveScan.workflowStudio.executionHistory.emptyTitle') }}</p>
              </td>
            </tr>
            <tr v-for="(exec, idx) in history_data" :key="exec.execution_id" class="hover">
              <td 
                class="truncate max-w-[180px] cursor-pointer hover:text-primary font-medium" 
                :title="exec.workflow_name"
                @click="view_execution_detail(exec.execution_id)"
              >
                {{ exec.workflow_name }} #{{ history_total - (history_page - 1) * history_page_size - idx }}
              </td>
              <td class="text-xs text-base-content/70">{{ format_datetime(exec.started_at) }}</td>
              <td class="text-right text-xs">{{ format_duration(exec.duration_ms) }}</td>
              <td class="text-center">
                <span :class="get_status_badge_class(exec.status)" class="badge badge-sm">
                  {{ get_status_text(exec.status) }}
                </span>
              </td>
              <td class="text-center">
                <div class="flex justify-center gap-1">
                  <button 
                    class="btn btn-xs btn-ghost" 
                    @click="view_execution_detail(exec.execution_id)"
                    :title="t('passiveScan.workflowStudio.executionHistory.table.viewDetail')"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                  </button>
                  <button 
                    class="btn btn-xs btn-ghost text-error" 
                    @click="delete_history_record(exec.execution_id)"
                    :title="t('passiveScan.workflowStudio.executionHistory.table.delete')"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      
      <!-- 分页 -->
      <div class="p-3 border-t border-base-300 flex items-center justify-between">
        <span class="text-sm text-base-content/60">
          {{ t('passiveScan.workflowStudio.executionHistory.pagination.total', { total: history_total }) }}
        </span>
        <div class="join">
          <button 
            class="join-item btn btn-sm" 
            :disabled="history_page <= 1"
            @click="history_page--; load_history_from_backend()"
          >«</button>
          <button class="join-item btn btn-sm">{{ history_page }} / {{ Math.max(1, Math.ceil(history_total / history_page_size)) }}</button>
          <button 
            class="join-item btn btn-sm" 
            :disabled="history_page >= Math.ceil(history_total / history_page_size)"
            @click="history_page++; load_history_from_backend()"
          >»</button>
        </div>
        <select class="select select-bordered select-sm w-24" v-model="history_page_size" @change="history_page = 1; load_history_from_backend()">
          <option :value="10">10</option>
          <option :value="20">20</option>
          <option :value="50">50</option>
        </select>
      </div>
    </div>
    
    <!-- 执行详情对话框 -->
    <dialog :open="show_detail_dialog" class="modal" @click.self="show_detail_dialog = false">
      <div class="modal-box max-w-3xl max-h-[80vh]">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-bold text-lg">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.title') }}</h3>
          <button class="btn btn-sm btn-ghost" @click="show_detail_dialog = false">✕</button>
        </div>
        
        <div v-if="detail_loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="detail_data" class="space-y-4">
          <!-- 基本信息 -->
          <div class="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span class="text-base-content/60">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.workflowName') }}:</span>
              <span class="font-medium ml-2">{{ detail_data.workflow_name }}</span>
            </div>
            <div>
              <span class="text-base-content/60">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.status') }}:</span>
              <span :class="get_status_badge_class(detail_data.status)" class="badge badge-sm ml-2">{{ get_status_text(detail_data.status) }}</span>
            </div>
            <div>
              <span class="text-base-content/60">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.startTime') }}:</span>
              <span class="ml-2">{{ format_datetime(detail_data.started_at) }}</span>
            </div>
            <div>
              <span class="text-base-content/60">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.duration') }}:</span>
              <span class="ml-2">{{ format_duration(detail_data.duration_ms) }}</span>
            </div>
            <div v-if="detail_data.error_message" class="col-span-2">
              <span class="text-base-content/60">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.error') }}:</span>
              <span class="text-error ml-2">{{ detail_data.error_message }}</span>
            </div>
          </div>
          
          <!-- 步骤列表 -->
          <div class="divider">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.steps') }}</div>
          <div class="space-y-2 max-h-[40vh] overflow-y-auto">
            <div v-if="!detail_data.steps || detail_data.steps.length === 0" class="text-center text-base-content/50 py-4">
              {{ t('passiveScan.workflowStudio.executionHistory.detailDialog.noSteps') }}
            </div>
            <div 
              v-for="(step, idx) in detail_data.steps" 
              :key="step.step_id" 
              class="collapse collapse-arrow bg-base-200"
            >
              <input type="checkbox" :checked="idx === 0" />
              <div class="collapse-title text-sm font-medium flex items-center gap-2">
                <span class="badge badge-xs" :class="get_status_badge_class(step.status)">{{ idx + 1 }}</span>
                <span>{{ step.step_name || step.step_id }}</span>
                <span class="text-xs text-base-content/50 ml-auto mr-4">{{ format_duration(step.duration_ms) }}</span>
              </div>
              <div class="collapse-content">
                <div v-if="step.error_message" class="text-error text-xs mb-2">{{ step.error_message }}</div>
                <pre v-if="step.result !== undefined && step.result !== null" class="text-xs bg-base-300 p-2 rounded overflow-x-auto max-h-48">{{ format_result(step.result) }}</pre>
                <div v-else class="text-xs text-base-content/50">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.noResult') }}</div>
              </div>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-sm btn-ghost" @click="copy_detail_result" :title="t('passiveScan.workflowStudio.executionHistory.copyResultsTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            {{ t('passiveScan.workflowStudio.executionHistory.detailDialog.copy') }}
          </button>
          <button class="btn btn-sm" @click="show_detail_dialog = false">{{ t('passiveScan.workflowStudio.executionHistory.detailDialog.close') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 步骤结果查看面板（保留用于点击节点时查看当前执行结果） -->
    <div v-if="show_result_panel" ref="result_panel_ref" class="fixed inset-y-0 right-0 w-[500px] bg-base-100 shadow-xl border-l border-base-300 z-50">
      <div class="p-3 flex items-center justify-between border-b border-base-300">
        <h2 class="text-base font-semibold">{{ t('passiveScan.workflowStudio.resultPanel.title') }}</h2>
        <div class="flex gap-2">
          <button class="btn btn-xs btn-outline" @click="copy_result_to_clipboard" :title="t('passiveScan.workflowStudio.resultPanel.copyTooltip')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
          </button>
          <button class="btn btn-xs btn-ghost" @click="close_result_panel">✕</button>
        </div>
      </div>
      <div class="p-3 border-b border-base-300">
        <div class="text-sm font-semibold">{{ t('passiveScan.workflowStudio.resultPanel.nodeId') }}</div>
        <div class="text-xs text-base-content/60 mt-1 font-mono">{{ selected_step_result?.step_id }}</div>
        <div class="text-sm font-semibold mt-2">{{ t('passiveScan.workflowStudio.resultPanel.nodeName') }}</div>
        <div class="text-xs text-base-content/60 mt-1">{{ selected_node?.name || t('passiveScan.workflowStudio.resultPanel.unknown') }}</div>
      </div>
      <div class="p-3 overflow-auto h-[calc(100%-140px)]">
        <div class="text-sm font-semibold mb-2">{{ t('passiveScan.workflowStudio.resultPanel.executionResult') }}</div>
        <pre class="bg-base-200 p-3 rounded text-xs font-mono overflow-x-auto">{{ format_result(selected_step_result?.result) }}</pre>
      </div>
      <div class="p-3 flex gap-2 border-t border-base-300">
        <button class="btn btn-primary btn-sm flex-1" @click="edit_node_params">
          {{ t('passiveScan.workflowStudio.resultPanel.editParams') }}
        </button>
        <button class="btn btn-outline btn-sm" @click="close_result_panel">{{ t('passiveScan.workflowStudio.resultPanel.close') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowEvents } from '@/composables/useWorkflowEvents'
import FlowchartVisualization from '@/components/workflow/FlowchartVisualization.vue'
import type { NodeCatalogItem, WorkflowGraph, NodeDef, EdgeDef } from '@/types/workflow'
import { useToast } from '@/composables/useToast'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const flow_ref = ref<InstanceType<typeof FlowchartVisualization> | null>(null)
const catalog = ref<NodeCatalogItem[]>([])
const search_query = ref('')
const selected_node = ref<any | null>(null)
const param_values = ref<Record<string, any>>({})
const drawer_open = ref(false)
const ignore_close_once = ref(false)
const drawer_ref = ref<HTMLElement | null>(null)
const result_panel_ref = ref<HTMLElement | null>(null)
const ignore_result_panel_close_once = ref(false)
const execution_history_ref = ref<HTMLElement | null>(null)
const ignore_execution_history_close_once = ref(false)
const sidebar_collapsed = ref(false)
const show_logs = ref(true) // 默认显示日志
const show_load_dialog = ref(false)
const show_meta_dialog = ref(false)
const show_template_dialog = ref(false)
const show_new_workflow_confirm = ref(false)
const template_list = ref<any[]>([])
const workflow_name = ref(t('passiveScan.workflowStudio.defaults.unnamedWorkflow'))
const workflow_id = ref(`wf_${Date.now()}`)
const workflow_description = ref('')
const workflow_tags = ref('')
const workflow_version = ref('v1.0.0')
const workflow_is_tool = ref(false) // 是否设为AI工具
const workflow_list = ref<any[]>([])
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
const AUTO_SAVE_DELAY = 1000 // 1秒防抖延迟

defineOptions({
  name: 'WorkflowStudio'
});


// 执行历史
interface ExecutionRecord {
  id: string
  start_time: string
  end_time?: string
  duration?: number
  status: 'pending' | 'running' | 'completed' | 'failed'
  step_results: Record<string, any>
}
const show_execution_history = ref(false)
const execution_history = ref<ExecutionRecord[]>([])
const selected_execution = ref<ExecutionRecord | null>(null)
const current_execution_id = ref<string | null>(null)

// 执行历史表格相关
interface HistoryItem {
  execution_id: string
  workflow_id: string
  workflow_name: string
  version: string
  status: string
  started_at: string
  completed_at?: string
  duration_ms?: number
  progress: number
  total_steps: number
  completed_steps: number
  error_message?: string
}
interface DetailData {
  execution_id: string
  workflow_id: string
  workflow_name: string
  version: string
  status: string
  started_at: string
  completed_at?: string
  duration_ms?: number
  progress: number
  total_steps: number
  completed_steps: number
  error_message?: string
  steps: Array<{
    step_id: string
    step_name?: string
    step_order?: number
    status: string
    started_at?: string
    completed_at?: string
    duration_ms?: number
    result?: any
    error_message?: string
  }>
}
const history_search_query = ref('')
const history_page = ref(1)
const history_page_size = ref(10)
const history_total = ref(0)
const history_data = ref<HistoryItem[]>([])
const history_loading = ref(false)
const show_detail_dialog = ref(false)
const detail_loading = ref(false)
const detail_data = ref<DetailData | null>(null)

interface ExecutionLog {
  timestamp: Date
  level: 'INFO' | 'WARN' | 'ERROR' | 'SUCCESS'
  message: string
  node_id?: string
  details?: string
}

const execution_logs = ref<ExecutionLog[]>([])
const json_errors = ref<Record<string, string>>({})
const expanded_logs = ref<Set<number>>(new Set())
const selected_schema = computed(() => {
  if (!selected_node.value) return null as any
  const item = catalog.value.find(i => i.node_type === selected_node.value.type)
  return item?.params_schema || null
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
  if (k === 'trigger') return t('passiveScan.workflowStudio.groups.trigger')
  if (k === 'control') return t('passiveScan.workflowStudio.groups.control')
  if (k === 'ai') return t('passiveScan.workflowStudio.groups.ai')
  if (k === 'data') return t('passiveScan.workflowStudio.groups.data')
  if (k === 'output') return t('passiveScan.workflowStudio.groups.output')
  if (k === 'tool') return t('passiveScan.workflowStudio.groups.tool')
  if (k === 'mcp') return t('passiveScan.workflowStudio.groups.mcp')
  if (k === 'plugin') return t('passiveScan.workflowStudio.groups.plugin')
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
  
  // 确保数值类型正确
  return {
    trigger_type: String(triggerNode.params.trigger_type || 'interval'),
    interval_seconds: Number(triggerNode.params.interval_seconds) || 60,
    hour: Number(triggerNode.params.hour) || 9,
    minute: Number(triggerNode.params.minute) || 0,
    second: Number(triggerNode.params.second) || 0,
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
  flow_ref.value?.resetFlowchart()
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
    toast.error(t('passiveScan.workflowStudio.toasts.enterWorkflowName'))
    return
  }
  await save_workflow()
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
  workflow_name.value = t('passiveScan.workflowStudio.defaults.unnamedWorkflow')
  workflow_description.value = ''
  workflow_tags.value = ''
  workflow_version.value = 'v1.0.0'
  workflow_is_tool.value = false
  flow_ref.value?.resetFlowchart()
  execution_history.value = []
  selected_execution.value = null
  execution_logs.value = []
  step_results.value = {}
  schedule_running.value = false
  schedule_info.value = null
  localStorage.removeItem('last_run_workflow_id')
  
  add_log('INFO', t('passiveScan.workflowStudio.logs.newWorkflowCreated'))
  toast.success(t('passiveScan.workflowStudio.toasts.newWorkflowCreated'))
}

const build_graph = (): WorkflowGraph => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const edges_detailed = (flow_ref.value as any)?.getFlowchartEdgesDetailed?.() || []
  const node_defs: NodeDef[] = nodes.map(n => ({
    id: n.id,
    node_type: n.type,
    node_name: n.name,
    x: Math.round(n.x),
    y: Math.round(n.y),
    params: n.params || {},
    input_ports: (() => {
      const item = catalog.value.find(i => i.node_type === n.type)
      return item?.input_ports?.length
        ? item.input_ports
        : [{ id: 'in', name: t('passiveScan.workflowStudio.flowchart.ports.input'), port_type: 'Json', required: false }]
    })(),
    output_ports: (() => {
      const item = catalog.value.find(i => i.node_type === n.type)
      return item?.output_ports?.length
        ? item.output_ports
        : [{ id: 'out', name: t('passiveScan.workflowStudio.flowchart.ports.output'), port_type: 'Json', required: false }]
    })()
  }))
  const edge_defs: EdgeDef[] = edges_detailed.length
    ? edges_detailed.map((e: any, idx: number) => ({
        id: `e_${idx}_${e.from_node}_${e.to_node}`,
        from_node: e.from_node,
        from_port: e.from_port || 'out',
        to_node: e.to_node,
        to_port: e.to_port || 'in'
      }))
    : ((flow_ref.value?.getFlowchartEdges() || []).map((e, idx) => ({
        id: `e_${idx}_${e.from_node}_${e.to_node}`,
        from_node: e.from_node,
        from_port: 'out',
        to_node: e.to_node,
        to_port: 'in'
      })))
  return {
    id: workflow_id.value,
    name: workflow_name.value || t('passiveScan.workflowStudio.defaults.unnamedWorkflow'),
    version: workflow_version.value || 'v1.0.0',
    nodes: node_defs,
    edges: edge_defs,
    variables: [],
    credentials: []
  }
}

const add_log = (level: ExecutionLog['level'], message: string, node_id?: string, details?: string) => {
  execution_logs.value.push({
    timestamp: new Date(),
    level,
    message,
    node_id,
    details
  })
  // 自动滚动到底部
  setTimeout(() => {
    const logContainer = document.querySelector('.overflow-y-auto.bg-base-200')
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight
    }
  }, 100)
}

const clear_logs = () => {
  execution_logs.value = []
  expanded_logs.value.clear()
}

const toggle_log_details = (idx: number) => {
  if (expanded_logs.value.has(idx)) {
    expanded_logs.value.delete(idx)
  } else {
    expanded_logs.value.add(idx)
  }
}

const format_result = (result: any) => {
  if (result === undefined || result === null) return t('passiveScan.workflowStudio.resultPanel.noResult')
  if (typeof result === 'object') {
    return JSON.stringify(result, null, 2)
  }
  return String(result)
}

const copy_result_to_clipboard = async () => {
  const toast = useToast()
  if (!selected_step_result.value?.result) return
  
  try {
    const text = format_result(selected_step_result.value.result)
    await navigator.clipboard.writeText(text)
    toast.success(t('passiveScan.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('passiveScan.workflowStudio.toasts.copyFailed', { message: e.message }))
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
    toast.error(t('passiveScan.workflowStudio.toasts.loadFailed', { error: String(e) }))
  } finally {
    history_loading.value = false
  }
}

// 查看执行详情
const view_execution_detail = async (runId: string) => {
  show_detail_dialog.value = true
  detail_loading.value = true
  detail_data.value = null
  try {
    const result = await invoke<DetailData | null>('get_workflow_run_detail', { runId })
    detail_data.value = result
  } catch (e: any) {
    console.error('Failed to load execution detail:', e)
    const toast = useToast()
    toast.error(t('passiveScan.workflowStudio.toasts.loadFailed', { error: String(e) }))
    show_detail_dialog.value = false
  } finally {
    detail_loading.value = false
  }
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
    toast.error(t('passiveScan.workflowStudio.toasts.deleteFailed', { error: String(e) }))
  }
}

// 复制详情结果
const copy_detail_result = async () => {
  const toast = useToast()
  if (!detail_data.value) return
  try {
    const text = JSON.stringify(detail_data.value, null, 2)
    await navigator.clipboard.writeText(text)
    toast.success(t('passiveScan.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('passiveScan.workflowStudio.toasts.copyFailed', { message: e.message }))
  }
}

// 格式化日期时间
const format_datetime = (dateStr?: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

// 格式化耗时
const format_duration = (ms?: number) => {
  if (ms === undefined || ms === null) return '-'
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`
}

// 获取状态徽章样式
const get_status_badge_class = (status: string) => {
  switch (status) {
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    case 'running': return 'badge-warning'
    case 'pending': return 'badge-ghost'
    case 'cancelled': return 'badge-neutral'
    default: return 'badge-ghost'
  }
}

// 获取状态文本
const get_status_text = (status: string) => {
  switch (status) {
    case 'completed': return t('passiveScan.workflowStudio.executionHistory.status.completed')
    case 'failed': return t('passiveScan.workflowStudio.executionHistory.status.failed')
    case 'running': return t('passiveScan.workflowStudio.executionHistory.status.running')
    case 'pending': return t('passiveScan.workflowStudio.executionHistory.status.pending')
    case 'cancelled': return t('passiveScan.workflowStudio.executionHistory.status.cancelled')
    default: return status
  }
}

const select_execution = (exec: ExecutionRecord) => {
  selected_execution.value = exec
}

const clear_execution_history = () => {
  execution_history.value = []
  selected_execution.value = null
  localStorage.removeItem(`workflow_execution_history_${workflow_id.value}`)
}

// 删除单条执行记录
const delete_single_execution = (execId: string) => {
  const idx = execution_history.value.findIndex(e => e.id === execId)
  if (idx === -1) return
  
  // 如果删除的是当前选中的记录，清除选中状态
  if (selected_execution.value?.id === execId) {
    selected_execution.value = null
  }
  
  execution_history.value.splice(idx, 1)
  save_execution_history()
}

const copy_execution_result = async () => {
  const toast = useToast()
  if (!selected_execution.value) return
  
  try {
    const text = JSON.stringify(selected_execution.value.step_results, null, 2)
    await navigator.clipboard.writeText(text)
    toast.success(t('passiveScan.workflowStudio.toasts.copiedToClipboard'))
  } catch (e: any) {
    toast.error(t('passiveScan.workflowStudio.toasts.copyFailed', { message: e.message }))
  }
}

const get_node_name = (nodeId: string): string => {
  const nodes = flow_ref.value?.getFlowchartNodes() || []
  const node = nodes.find((n: any) => n.id === nodeId) as any
  return node?.name || nodeId
}

const start_new_execution = (): string => {
  const id = `exec_${Date.now()}`
  const now = new Date().toLocaleString('zh-CN')
  
  const record: ExecutionRecord = {
    id,
    start_time: now,
    status: 'running',
    step_results: {}
  }
  
  execution_history.value.unshift(record)
  current_execution_id.value = id
  
  // 限制历史记录数量
  if (execution_history.value.length > 20) {
    execution_history.value = execution_history.value.slice(0, 20)
  }
  
  return id
}

const update_execution_step_result = (stepId: string, result: any) => {
  const exec = execution_history.value.find(e => e.id === current_execution_id.value)
  if (exec) {
    exec.step_results[stepId] = result
  }
}

const complete_execution = (success: boolean) => {
  const exec = execution_history.value.find(e => e.id === current_execution_id.value)
  if (exec) {
    exec.status = success ? 'completed' : 'failed'
    exec.end_time = new Date().toLocaleString('zh-CN')
    const start = new Date(exec.start_time).getTime()
    exec.duration = Date.now() - start
  }
  
  // 保存到 localStorage
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
    localStorage.setItem(
      `workflow_execution_history_${workflow_id.value}`,
      JSON.stringify(execution_history.value.slice(0, 10)) // 只保存最近10条
    )
  } catch (e) {
    console.error('Failed to save execution history:', e)
  }
}

const load_execution_history = () => {
  try {
    const saved = localStorage.getItem(`workflow_execution_history_${workflow_id.value}`)
    if (saved) {
      execution_history.value = JSON.parse(saved)
    }
  } catch (e) {
    console.error('Failed to load execution history:', e)
  }
}

const get_log_class = (level: string) => {
  switch (level) {
    case 'ERROR': return 'text-error'
    case 'WARN': return 'text-warning'
    case 'SUCCESS': return 'text-success'
    default: return 'text-base-content'
  }
}

const format_time = (date: Date) => {
  return date.toLocaleTimeString('zh-CN', { hour12: false })
}

const format_date = (dateStr: string) => {
  return new Date(dateStr).toLocaleString('zh-CN', { 
    year: 'numeric', 
    month: '2-digit', 
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

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
      add_log('ERROR', t('passiveScan.workflowStudio.logs.validationFailed', { message: issues[0].message }), issues[0].node_id)
    toast.error(t('passiveScan.workflowStudio.toasts.validationFailed', { message: issues[0].message }))
      return
    }
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.validationError', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.validationError', { error: String(e) }))
    return
  }
  
  try {
    // 创建新的执行记录
    start_new_execution()
    
    add_log('INFO', t('passiveScan.workflowStudio.logs.workflowExecutionStarted', { name: workflow_name.value }))
    show_logs.value = true
    workflow_running.value = true
    const exec_id = await invoke<string>('start_workflow_run', { graph })
    current_exec_id.value = exec_id
    add_log(
      'SUCCESS',
      t('passiveScan.workflowStudio.logs.workflowStarted'),
      undefined,
      t('passiveScan.workflowStudio.logs.executionId', { id: exec_id })
    )
    toast.success(t('passiveScan.workflowStudio.toasts.executionStarted', { id: exec_id }))
    
    // 保存最后运行的工作流ID
    localStorage.setItem('last_run_workflow_id', workflow_id.value)
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.startFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.startFailed', { error: String(e) }))
    complete_execution(false)
    workflow_running.value = false
    current_exec_id.value = null
  }
}

// 停止工作流执行
const stop_run = async () => {
  const toast = useToast()
  
  if (!current_exec_id.value) {
    toast.error(t('passiveScan.workflowStudio.toasts.noRunningWorkflow'))
    return
  }
  
  try {
    add_log('INFO', t('passiveScan.workflowStudio.logs.stoppingWorkflow'))
    await invoke('stop_workflow_run', { executionId: current_exec_id.value })
    add_log('WARN', t('passiveScan.workflowStudio.logs.workflowStopped'))
    toast.success(t('passiveScan.workflowStudio.toasts.workflowStopped'))
    workflow_running.value = false
    current_exec_id.value = null
    complete_execution(false)
    
    // 重置节点状态
    reset_node_status()
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.stopFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.stopFailed', { error: String(e) }))
  }
}

// 启动定时调度
const start_schedule = async () => {
  const toast = useToast()
  
  // 先保存工作流
  await save_workflow()
  
  const config = get_schedule_config()
  if (!config) {
    toast.error(t('passiveScan.workflowStudio.toasts.scheduleMissingTrigger'))
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
      ? t('passiveScan.workflowStudio.schedule.everySeconds', { seconds: config.interval_seconds })
      : config.trigger_type === 'daily'
        ? t('passiveScan.workflowStudio.schedule.dailyAt', { time })
        : t('passiveScan.workflowStudio.schedule.weeklyAt', { weekdays: config.weekdays, time })
    
    add_log('SUCCESS', t('passiveScan.workflowStudio.logs.scheduleStarted', { desc: interval_desc }))
    toast.success(t('passiveScan.workflowStudio.toasts.scheduleStarted', { desc: interval_desc }))
    show_logs.value = true
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.scheduleStartFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.scheduleStartFailed', { error: String(e) }))
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
    add_log('INFO', t('passiveScan.workflowStudio.logs.scheduleStopped'))
    toast.success(t('passiveScan.workflowStudio.toasts.scheduleStopped'))
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.scheduleStopFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.scheduleStopFailed', { error: String(e) }))
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

const save_workflow = async (silent = false) => {
  const toast = useToast()
  const graph = build_graph()
  graph.id = workflow_id.value
  graph.name = workflow_name.value
  
  try {
    await invoke('save_workflow_definition', {
      graph,
      description: workflow_description.value || null,
      tags: workflow_tags.value || null,
      isTemplate: false,
      isTool: workflow_is_tool.value
    })
    has_unsaved_changes.value = false
    if (!silent) {
      add_log(
        'SUCCESS',
        workflow_is_tool.value
          ? t('passiveScan.workflowStudio.logs.workflowSavedAsTool', { name: workflow_name.value })
          : t('passiveScan.workflowStudio.logs.workflowSaved', { name: workflow_name.value })
      )
      toast.success(t('passiveScan.workflowStudio.toasts.workflowSaved'))
    }
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.saveFailed', { error: String(e) }))
    if (!silent) {
      toast.error(t('passiveScan.workflowStudio.toasts.saveFailed', { error: String(e) }))
    }
  }
}

const on_save_workflow_click = (_evt: MouseEvent) => {
  void save_workflow(false)
}

// 自动保存（防抖）
const trigger_auto_save = () => {
  // 如果工作流名称为空，不自动保存
  if (!workflow_name.value.trim()) return
  
  // 如果工作流正在运行，不自动保存
  if (workflow_running.value) return
  
  has_unsaved_changes.value = true
  
  // 清除之前的定时器
  if (auto_save_timer.value) {
    clearTimeout(auto_save_timer.value)
  }
  
  // 设置新的定时器
  auto_save_timer.value = setTimeout(async () => {
    is_auto_saving.value = true
    await save_workflow(true) // 静默保存
    is_auto_saving.value = false
    has_unsaved_changes.value = false
  }, AUTO_SAVE_DELAY)
}

// 流程图变化处理
const on_flowchart_change = () => {
  trigger_auto_save()
}

const load_workflow = async (id: string) => {
  const toast = useToast()
  try {
    const data = await invoke<any>('get_workflow_definition', { id })
    if (data && data.graph) {
      const graph = data.graph
      workflow_id.value = graph.id
      workflow_name.value = graph.name
      workflow_description.value = data.description || ''
      workflow_tags.value = data.tags || ''
      workflow_is_tool.value = data.is_tool || false
      
      // 清空画布
      flow_ref.value?.resetFlowchart()
      
      // 加载节点
      graph.nodes.forEach((n: NodeDef) => {
        const node: any = {
          id: n.id,
          name: n.node_name,
          description: n.node_type,
          status: 'pending',
          x: n.x,
          y: n.y,
          type: n.node_type,
          dependencies: [],
          params: n.params || {},
          metadata: { input_ports: n.input_ports || [], output_ports: n.output_ports || [] }
        }
        flow_ref.value?.addNode(node)
      })
      
      // 加载连接
      graph.edges.forEach((e: EdgeDef) => {
        flow_ref.value?.addConnectionWithPorts(e.from_node, e.to_node, e.from_port, e.to_port)
      })
      
      add_log('SUCCESS', t('passiveScan.workflowStudio.logs.workflowLoaded', { name: workflow_name.value }))
      show_load_dialog.value = false
      
      // 加载该工作流的执行历史
      load_execution_history()
      // 检查调度状态
      check_schedule_status()
    }
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.loadFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.loadFailed', { error: String(e) }))
  }
}

const delete_workflow = async (id: string) => {
  const toast = useToast()
  if (!confirm(t('passiveScan.workflowStudio.confirm.deleteWorkflow'))) return
  
  try {
    await invoke('delete_workflow_definition', { id })
    workflow_list.value = workflow_list.value.filter(wf => wf.id !== id)
    toast.success(t('passiveScan.workflowStudio.toasts.workflowDeleted'))
  } catch (e: any) {
    toast.error(t('passiveScan.workflowStudio.toasts.deleteFailed', { error: String(e) }))
  }
}

const load_workflow_list = async () => {
  try {
    workflow_list.value = await invoke<any[]>('list_workflow_definitions', { isTemplate: null })
  } catch (e) {
    console.error('Failed to load workflow list:', e)
  }
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
    json_errors.value[key] = t('passiveScan.workflowStudio.errors.jsonFormatError', { message: e.message })
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
        exported_by: t('passiveScan.workflowStudio.export.exportedBy')
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
    
    add_log('SUCCESS', t('passiveScan.workflowStudio.logs.workflowExported', { filename: a.download }))
    toast.success(t('passiveScan.workflowStudio.toasts.workflowExported'))
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.exportFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.exportFailed', { error: String(e) }))
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
      throw new Error(t('passiveScan.workflowStudio.errors.invalidWorkflowFile'))
    }
    
    const graph = data.workflow
    
    // 重新生成ID避免冲突
    workflow_id.value = `wf_${Date.now()}`
    workflow_name.value = graph.name || t('passiveScan.workflowStudio.defaults.importedWorkflow')
    workflow_description.value = data.metadata?.description || ''
    workflow_tags.value = data.metadata?.tags || ''
    workflow_version.value = graph.version || 'v1.0.0'
    workflow_is_tool.value = data.metadata?.is_tool || false
    
    // 清空画布
    flow_ref.value?.resetFlowchart()
    
    // 加载节点
    graph.nodes.forEach((n: NodeDef) => {
      const node: any = {
        id: n.id,
        name: n.node_name,
        description: n.node_type,
        status: 'pending',
        x: n.x,
        y: n.y,
        type: n.node_type,
        dependencies: [],
        params: n.params || {},
        metadata: { input_ports: n.input_ports || [], output_ports: n.output_ports || [] }
      }
      flow_ref.value?.addNode(node)
    })
    
    // 加载连接
    graph.edges.forEach((e: EdgeDef) => {
      flow_ref.value?.addConnectionWithPorts(e.from_node, e.to_node, e.from_port, e.to_port)
    })
    
    add_log('SUCCESS', t('passiveScan.workflowStudio.logs.workflowImported', { name: workflow_name.value }))
    toast.success(t('passiveScan.workflowStudio.toasts.workflowImported'))
    
    // 清空文件输入
    target.value = ''
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.importFailed', { message: e.message }))
    toast.error(t('passiveScan.workflowStudio.toasts.importFailed', { message: e.message }))
    target.value = ''
  }
}

// 导出工作流为图片
const export_workflow_image = async () => {
  const toast = useToast()
  try {
    // 使用html2canvas库导出（需要先安装）
    toast.info(t('passiveScan.workflowStudio.toasts.imageExportRequiresHtml2Canvas'))
    add_log('INFO', t('passiveScan.workflowStudio.logs.imageExportTodo'))
    // TODO: 实现图片导出
    // const canvas = await html2canvas(flowchartContainer)
    // const url = canvas.toDataURL('image/png')
    // download(url, `${workflow_name.value}.png`)
  } catch (e: any) {
    toast.error(t('passiveScan.workflowStudio.toasts.exportFailed', { error: String(e) }))
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
        t('passiveScan.workflowStudio.logs.workflowExecutionStartedExternal'),
        undefined,
        t('passiveScan.workflowStudio.logs.executionId', { id: exec_id || id })
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
      add_log('INFO', t('passiveScan.workflowStudio.logs.nodeStarted'), step_id)
    }
  })
  await wf_events.on_step_complete((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) {
      flow_ref.value?.updateNodeStatus(step_id, 'completed')
      
      // 保存步骤结果到当前执行和执行历史
      const result = p?.result
      if (result) {
        step_results.value[step_id] = result
        update_execution_step_result(step_id, result)
        
        const result_preview = typeof result === 'object' 
          ? JSON.stringify(result, null, 2)
          : String(result)
        add_log('SUCCESS', t('passiveScan.workflowStudio.logs.nodeCompleted'), step_id, result_preview)
      } else {
        add_log('SUCCESS', t('passiveScan.workflowStudio.logs.nodeCompleted'), step_id)
      }
    }
  })
  await wf_events.on_run_complete(() => {
    add_log('SUCCESS', t('passiveScan.workflowStudio.logs.workflowCompleted'))
    complete_execution(true)
    workflow_running.value = false
    current_exec_id.value = null
    
    // 自动打开执行历史面板并选中当前执行记录
    show_execution_history.value = true
    const currentExec = execution_history.value.find(e => e.id === current_execution_id.value)
    if (currentExec) {
      selected_execution.value = currentExec
    }
    
    // 延迟清除节点执行状态
    setTimeout(() => {
      reset_node_status()
    }, 1500)
  })
  
  // 监听工作流停止事件
  await wf_events.on_run_stop((p: any) => {
    add_log('WARN', t('passiveScan.workflowStudio.logs.workflowExecutionStopped'))
    workflow_running.value = false
    current_exec_id.value = null
    complete_execution(false)
    reset_node_status()
  })
}

const on_node_click = (node: any) => {
  ignore_close_once.value = true
  selected_node.value = node
  const current = node.params || {}
  
  // 获取节点的参数 schema
  const item = catalog.value.find(i => i.node_type === node.type)
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
  await load_workflow(id)
  // 重新生成ID，避免覆盖模板
  workflow_id.value = `wf_${Date.now()}`
  workflow_name.value = t('passiveScan.workflowStudio.defaults.duplicateWorkflowName', { name: workflow_name.value })
  show_template_dialog.value = false
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
    add_log('SUCCESS', t('passiveScan.workflowStudio.logs.templateSaved', { name: workflow_name.value }))
    toast.success(t('passiveScan.workflowStudio.toasts.templateSaved'))
    await load_template_list()
  } catch (e: any) {
    add_log('ERROR', t('passiveScan.workflowStudio.logs.templateSaveFailed', { error: String(e) }))
    toast.error(t('passiveScan.workflowStudio.toasts.templateSaveFailed', { error: String(e) }))
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

// 监听show_load_dialog变化，加载工作流列表
watch(show_load_dialog, (newVal) => {
  if (newVal) {
    load_workflow_list()
  }
})

// 监听show_template_dialog变化，加载模板列表
watch(show_template_dialog, (newVal) => {
  if (newVal) {
    load_template_list()
  }
})

// 监听工作流元数据变化，触发自动保存
watch([workflow_name, workflow_description, workflow_tags, workflow_version, workflow_is_tool], () => {
  trigger_auto_save()
}, { deep: true })

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
    add_log('INFO', t('passiveScan.workflowStudio.logs.foundMatchingNodes', { count: matches.length }))
    // 滚动到第一个匹配的节点
    const first = matches[0]
    // TODO: 实现画布滚动到节点位置
  } else {
    add_log('WARN', t('passiveScan.workflowStudio.logs.noMatchingNodes'))
  }
}

const handle_global_click = (e: MouseEvent) => {
  // 如果详情对话框打开了，不处理其他面板的关闭
  if (show_detail_dialog.value) return

  // 处理参数编辑抽屉的关闭
  if (drawer_open.value) {
    if (ignore_close_once.value) { 
      ignore_close_once.value = false
    } else {
        const drawer = drawer_ref.value
        if (!drawer || !drawer.contains(e.target as Node)) {
          drawer_open.value = false
      }
    }
  }
  
  // 处理执行历史面板的关闭
  if (show_execution_history.value) {
    if (ignore_execution_history_close_once.value) {
      ignore_execution_history_close_once.value = false
    } else {
      const historyPanel = execution_history_ref.value
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
      const panel = result_panel_ref.value
      if (!panel || !panel.contains(e.target as Node)) {
        close_result_panel()
      }
    }
  }
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
})

onUnmounted(() => {
  wf_events.unsubscribe_all()
  // 清除自动保存定时器
  if (auto_save_timer.value) {
    clearTimeout(auto_save_timer.value)
  }
  window.removeEventListener('click', handle_global_click)
})
</script>

<style scoped>
</style>
