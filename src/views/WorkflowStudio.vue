<template>
  <div class="p-4 space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold">工作流工作室</h1>
          <input v-model="workflow_name" class="input input-bordered input-sm w-48" placeholder="工作流名称" />
          <button class="btn btn-xs btn-ghost" @click="show_meta_dialog = true" title="编辑工作流元数据">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
          </button>
        </div>
        <div class="flex gap-2">
          <button class="btn btn-sm btn-outline" @click="show_load_dialog = true" title="加载工作流">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" />
            </svg>
            加载
          </button>
          <button class="btn btn-sm btn-outline" @click="show_template_dialog = true" title="模板市场">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
            模板
          </button>
          <button class="btn btn-sm btn-primary" @click="save_workflow" :disabled="!workflow_name.trim()" title="保存工作流">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
            </svg>
            保存
          </button>
          <div class="dropdown dropdown-end">
            <button tabindex="0" class="btn btn-sm btn-outline" title="导出/导入">
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
                导出为JSON
              </a></li>
              <li><a @click="trigger_import_file">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                </svg>
                从JSON导入
              </a></li>
              <li><a @click="export_workflow_image">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
                导出为图片
              </a></li>
            </ul>
          </div>
          <input ref="import_file_input" type="file" accept=".json" class="hidden" @change="import_workflow_json" />
          <button class="btn btn-sm btn-outline" @click="refresh_catalog" title="刷新节点库">刷新节点库</button>
          <button class="btn btn-sm btn-outline" @click="reset_canvas" title="重置画布">重置画布</button>
          <button class="btn btn-sm btn-success" @click="start_run" title="运行工作流">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            运行
          </button>
          <button class="btn btn-sm btn-ghost" @click="show_logs = !show_logs" title="切换日志面板">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            日志
          </button>
        </div>
      </div>

    <div class="grid grid-cols-12 gap-4">
      <div :class="sidebar_collapsed ? 'col-span-1' : 'col-span-3'" class="transition-all duration-300">
        <div class="card bg-base-100 shadow-xl h-full">
          <div class="card-body p-3">
            <div class="flex items-center justify-between mb-2">
              <h2 v-if="!sidebar_collapsed" class="text-base font-semibold">节点库</h2>
              <button class="btn btn-xs btn-ghost" @click="sidebar_collapsed = !sidebar_collapsed" :title="sidebar_collapsed ? '展开侧边栏' : '折叠侧边栏'">
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
                <input v-model="search_query" class="input input-bordered input-sm w-full pr-16" placeholder="搜索节点..." @input="on_search_change" />
                <button v-if="search_query" class="btn btn-xs btn-ghost absolute right-8 top-1/2 -translate-y-1/2" @click="clear_search" title="清空搜索">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
                <button class="btn btn-xs btn-ghost absolute right-1 top-1/2 -translate-y-1/2" @click="search_in_canvas" title="在画布中搜索" :disabled="!search_query">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                  </svg>
                </button>
              </div>
              <div class="form-control mb-2">
                <label class="label cursor-pointer py-1">
                  <span class="label-text text-xs">仅显示收藏</span>
                  <input type="checkbox" v-model="show_favorites_only" class="checkbox checkbox-xs" />
                </label>
              </div>
              <div class="space-y-2 overflow-y-auto" style="max-height: calc(100vh - 250px)">
                <div v-if="filtered_groups.length === 0" class="text-center text-sm text-base-content/60 py-4">
                  未找到匹配的节点
                </div>
              <div v-for="group in filtered_groups" :key="group.name" class="collapse collapse-arrow bg-base-200">
                  <input type="checkbox" :checked="group.name === 'tool'" />
                  <div class="collapse-title text-sm font-medium py-2">
                    {{ group.label }} ({{ group.items.length }})
                  </div>
                <div class="collapse-content">
                  <div class="grid grid-cols-2 gap-2">
                    <button
                      v-for="item in group.items"
                      :key="item.node_type"
                        class="btn btn-xs relative"
                      @click="add_node(item)"
                        :title="item.node_type"
                      >
                        <span class="truncate">{{ item.label }}</span>
                        <button 
                          class="absolute top-0 right-0 btn btn-ghost btn-xs p-0 w-4 h-4"
                          @click.stop="toggle_favorite(item.node_type)"
                          :title="is_favorite(item.node_type) ? '取消收藏' : '收藏'"
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
        <FlowchartVisualization ref="flow_ref" @nodeClick="on_node_click" :highlightedNodes="highlighted_nodes" />
      </div>
    </div>

    <!-- 执行日志面板 -->
    <div v-if="show_logs" class="card bg-base-100 shadow-xl mt-4">
      <div class="card-body p-3">
        <div class="flex items-center justify-between mb-2">
          <h2 class="text-base font-semibold">执行日志</h2>
          <div class="flex gap-2">
            <button class="btn btn-xs btn-outline" @click="clear_logs">清空</button>
            <button class="btn btn-xs btn-ghost" @click="show_logs = false">✕</button>
          </div>
        </div>
        <div class="overflow-y-auto bg-base-200 rounded p-2 font-mono text-xs" style="max-height: 300px">
          <div v-if="execution_logs.length === 0" class="text-center text-base-content/60 py-4">
            暂无日志
          </div>
          <div v-for="(log, idx) in execution_logs" :key="idx" class="mb-1" :class="get_log_class(log.level)">
            <span class="opacity-60">[{{ format_time(log.timestamp) }}]</span>
            <span class="font-semibold">[{{ log.level }}]</span>
            <span v-if="log.node_id" class="text-primary">[{{ log.node_id }}]</span>
            <span>{{ log.message }}</span>
            <pre v-if="log.details" class="ml-4 mt-1 text-xs opacity-80">{{ log.details }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- 加载工作流对话框 -->
    <dialog :open="show_load_dialog" class="modal" @click.self="show_load_dialog = false">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">加载工作流</h3>
        <div class="space-y-2 max-h-96 overflow-y-auto">
          <div v-if="workflow_list.length === 0" class="text-center text-base-content/60 py-8">
            暂无已保存的工作流
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
                    <span>版本: {{ wf.version }}</span>
                    <span>更新: {{ format_date(wf.updated_at) }}</span>
                    <span v-if="wf.tags" class="badge badge-xs">{{ wf.tags }}</span>
                  </div>
                </div>
                <button class="btn btn-xs btn-error btn-ghost" @click.stop="delete_workflow(wf.id)" title="删除">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-sm" @click="show_load_dialog = false">关闭</button>
        </div>
      </div>
    </dialog>

    <!-- 模板市场对话框 -->
    <dialog :open="show_template_dialog" class="modal" @click.self="show_template_dialog = false">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">工作流模板市场</h3>
        
        <div class="tabs tabs-boxed mb-4">
          <a class="tab tab-active">推荐模板</a>
          <a class="tab" @click="load_my_templates">我的模板</a>
        </div>
        
        <div class="grid grid-cols-2 gap-4 max-h-96 overflow-y-auto">
          <div v-if="template_list.length === 0" class="col-span-2 text-center text-base-content/60 py-8">
            暂无模板
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
                    <span v-if="tpl.is_template" class="badge badge-primary badge-xs">模板</span>
                  </h4>
                  <p v-if="tpl.description" class="text-sm text-base-content/70 mt-1 line-clamp-2">{{ tpl.description }}</p>
                  <div class="flex gap-2 mt-2 text-xs text-base-content/60">
                    <span>{{ tpl.node_count || 0 }} 个节点</span>
                    <span v-if="tpl.tags" class="badge badge-xs">{{ tpl.tags }}</span>
                  </div>
                </div>
              </div>
              <div class="card-actions justify-end mt-2">
                <button class="btn btn-xs btn-primary" @click="use_template(tpl.id)">使用模板</button>
                <button v-if="!tpl.is_builtin" class="btn btn-xs btn-outline" @click="save_as_template">另存为模板</button>
              </div>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-sm btn-primary" @click="save_current_as_template">保存当前为模板</button>
          <button class="btn btn-sm" @click="show_template_dialog = false">关闭</button>
        </div>
      </div>
    </dialog>

    <!-- 工作流元数据对话框 -->
    <dialog :open="show_meta_dialog" class="modal" @click.self="show_meta_dialog = false">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">工作流元数据</h3>
        <div class="space-y-3">
          <div class="form-control">
            <label class="label">
              <span class="label-text">工作流名称 <span class="text-error">*</span></span>
            </label>
            <input v-model="workflow_name" class="input input-bordered" placeholder="请输入工作流名称" />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">描述</span>
            </label>
            <textarea v-model="workflow_description" class="textarea textarea-bordered" rows="3" placeholder="描述工作流的用途和功能"></textarea>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">标签</span>
            </label>
            <input v-model="workflow_tags" class="input input-bordered" placeholder="用逗号分隔多个标签，如：自动化,数据处理" />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">版本</span>
            </label>
            <input v-model="workflow_version" class="input input-bordered" placeholder="v1.0.0" />
          </div>
          
          <div class="stats shadow w-full">
            <div class="stat py-2">
              <div class="stat-title text-xs">节点数</div>
              <div class="stat-value text-2xl">{{ flow_ref?.getFlowchartNodes().length || 0 }}</div>
            </div>
            <div class="stat py-2">
              <div class="stat-title text-xs">连接数</div>
              <div class="stat-value text-2xl">{{ flow_ref?.getFlowchartEdges().length || 0 }}</div>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-sm btn-primary" @click="show_meta_dialog = false" :disabled="!workflow_name.trim()">确定</button>
          <button class="btn btn-sm" @click="show_meta_dialog = false">取消</button>
        </div>
      </div>
    </dialog>

    <div v-if="drawer_open" ref="drawer_ref" class="fixed inset-y-0 right-0 w-[350px] bg-base-100 shadow-xl border-l border-base-300 z-50">
      <div class="p-3 flex items-center justify-between border-b border-base-300">
        <h2 class="text-base font-semibold">参数编辑</h2>
        <button class="btn btn-xs btn-ghost" @click="close_drawer">✕</button>
      </div>
      <div class="p-3 border-b border-base-300">
        <div class="text-sm font-semibold">{{ selected_node?.name }}</div>
        <div class="text-xs text-base-content/60 mt-1">{{ selected_node?.type }}</div>
      </div>
      <div class="p-3 space-y-3 overflow-auto h-[calc(100%-140px)]" v-if="selected_schema">
        <div v-if="!selected_schema.properties || Object.keys(selected_schema.properties).length === 0" class="text-center text-sm text-base-content/60 py-4">
          此节点无需配置参数
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
          <div v-if="key === 'notification_rule_id' && selected_node?.type === 'notify'" class="space-y-2">
            <select 
              class="select select-bordered select-sm w-full" 
              v-model="param_values[key]"
              :class="{ 'select-error': selected_schema.required?.includes(key) && !param_values[key] }"
            >
              <option value="">-- 请选择通知规则 --</option>
              <option v-for="rule in notification_rules" :key="rule.id" :value="rule.id">
                {{ rule.type_name }} ({{ rule.channel }})
              </option>
            </select>
            <div v-if="notification_rules.length === 0" class="text-xs text-warning">
              <span>⚠️ 暂无可用的通知规则，</span>
              <router-link to="/notification-management" class="link link-primary">前往配置</router-link>
            </div>
          </div>
          
          <!-- 字符串类型 -->
          <input 
            v-else-if="prop.type === 'string' && !prop.enum" 
            class="input input-bordered input-sm" 
            v-model="param_values[key]"
            :placeholder="prop.default || `请输入${key}`"
            :class="{ 'input-error': selected_schema.required?.includes(key) && !param_values[key] }"
          />
          
          <!-- 数字类型 -->
          <input 
            v-else-if="prop.type === 'integer' || prop.type === 'float' || prop.type === 'number'" 
            type="number" 
            class="input input-bordered input-sm" 
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
            <option value="">-- 请选择 --</option>
            <option v-for="opt in prop.enum" :key="opt" :value="opt">{{ opt }}</option>
          </select>
          
          <!-- 布尔类型 -->
          <div v-else-if="prop.type === 'boolean'" class="flex items-center gap-2">
            <input type="checkbox" class="toggle toggle-sm toggle-primary" v-model="param_values[key]" />
            <span class="text-xs">{{ param_values[key] ? '是' : '否' }}</span>
          </div>
          
          <!-- 数组/对象类型 -->
          <div v-else-if="prop.type === 'array' || prop.type === 'object'" class="space-y-1">
            <textarea 
              class="textarea textarea-bordered textarea-sm font-mono text-xs" 
              v-model="param_values[key]"
              :placeholder="prop.type === 'array' ? '[\n  \n]' : '{\n  \n}'"
              rows="4"
              @blur="validate_json(key)"
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
          <label v-if="prop.description" class="label py-0">
            <span class="label-text-alt text-xs opacity-60">{{ prop.description }}</span>
          </label>
          
          <!-- 默认值提示 -->
          <label v-if="prop.default !== undefined && !param_values[key]" class="label py-0">
            <span class="label-text-alt text-xs text-info">默认: {{ prop.default }}</span>
          </label>
        </div>
      </div>
      <div class="p-3 flex gap-2 border-t border-base-300">
        <button class="btn btn-primary btn-sm flex-1" @click="save_params_and_close" :disabled="has_validation_errors">
          保存
        </button>
        <button class="btn btn-outline btn-sm" @click="close_drawer">取消</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowEvents } from '@/composables/useWorkflowEvents'
import FlowchartVisualization from '@/components/FlowchartVisualization.vue'
import type { NodeCatalogItem, WorkflowGraph, NodeDef, EdgeDef } from '@/types/workflow'
import { useToast } from '@/composables/useToast'

const flow_ref = ref<InstanceType<typeof FlowchartVisualization> | null>(null)
const catalog = ref<NodeCatalogItem[]>([])
const search_query = ref('')
const selected_node = ref<any | null>(null)
const param_values = ref<Record<string, any>>({})
const drawer_open = ref(false)
const ignore_close_once = ref(false)
const drawer_ref = ref<HTMLElement | null>(null)
const sidebar_collapsed = ref(false)
const show_logs = ref(false)
const show_load_dialog = ref(false)
const show_meta_dialog = ref(false)
const show_template_dialog = ref(false)
const template_list = ref<any[]>([])
const workflow_name = ref('未命名工作流')
const workflow_id = ref(`wf_${Date.now()}`)
const workflow_description = ref('')
const workflow_tags = ref('')
const workflow_version = ref('v1.0.0')
const workflow_list = ref<any[]>([])
const favorites = ref<Set<string>>(new Set())
const show_favorites_only = ref(false)
const notification_rules = ref<any[]>([]) // 通知规则列表
const import_file_input = ref<HTMLInputElement | null>(null)
const highlighted_nodes = ref<Set<string>>(new Set())

interface ExecutionLog {
  timestamp: Date
  level: 'INFO' | 'WARN' | 'ERROR' | 'SUCCESS'
  message: string
  node_id?: string
  details?: string
}

const execution_logs = ref<ExecutionLog[]>([])
const json_errors = ref<Record<string, string>>({})
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
      return item?.input_ports?.length ? item.input_ports : [{ id: 'in', name: '输入', port_type: 'Json', required: false }]
    })(),
    output_ports: (() => {
      const item = catalog.value.find(i => i.node_type === n.type)
      return item?.output_ports?.length ? item.output_ports : [{ id: 'out', name: '输出', port_type: 'Json', required: false }]
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
    name: workflow_name.value || '未命名工作流',
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
  
  // 使用后端校验
  try {
    const issues = await invoke<any[]>('validate_workflow_graph', { graph })
  if (issues.length) {
      add_log('ERROR', `工作流校验失败: ${issues[0].message}`, issues[0].node_id)
    toast.error(`校验失败：${issues[0].message}`)
      return
    }
  } catch (e: any) {
    add_log('ERROR', `校验出错: ${e}`)
    toast.error(`校验出错：${e}`)
    return
  }
  
  try {
    add_log('INFO', `开始执行工作流: ${workflow_name.value}`)
    show_logs.value = true
    const exec_id = await invoke<string>('start_workflow_run', { graph })
    add_log('SUCCESS', `工作流已启动`, undefined, `执行ID: ${exec_id}`)
    toast.success(`已启动执行：${exec_id}`)
  } catch (e: any) {
    add_log('ERROR', `启动失败: ${e}`)
    toast.error(`启动失败：${e}`)
  }
}

const save_workflow = async () => {
  const toast = useToast()
  const graph = build_graph()
  graph.id = workflow_id.value
  graph.name = workflow_name.value
  
  try {
    await invoke('save_workflow_definition', {
      graph,
      description: workflow_description.value || null,
      tags: workflow_tags.value || null,
      isTemplate: false
    })
    add_log('SUCCESS', `工作流已保存: ${workflow_name.value}`)
    toast.success('工作流已保存')
  } catch (e: any) {
    add_log('ERROR', `保存失败: ${e}`)
    toast.error(`保存失败：${e}`)
  }
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
      
      add_log('SUCCESS', `工作流已加载: ${workflow_name.value}`)
      toast.success('工作流已加载')
      show_load_dialog.value = false
    }
  } catch (e: any) {
    add_log('ERROR', `加载失败: ${e}`)
    toast.error(`加载失败：${e}`)
  }
}

const delete_workflow = async (id: string) => {
  const toast = useToast()
  if (!confirm('确定要删除这个工作流吗？')) return
  
  try {
    await invoke('delete_workflow_definition', { id })
    workflow_list.value = workflow_list.value.filter(wf => wf.id !== id)
    toast.success('工作流已删除')
  } catch (e: any) {
    toast.error(`删除失败：${e}`)
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
    json_errors.value[key] = 'JSON格式错误: ' + e.message
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
        exported_at: new Date().toISOString(),
        exported_by: 'Sentinel AI Workflow Studio'
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
    
    add_log('SUCCESS', `工作流已导出: ${a.download}`)
    toast.success('工作流已导出')
  } catch (e: any) {
    add_log('ERROR', `导出失败: ${e}`)
    toast.error(`导出失败：${e}`)
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
      throw new Error('无效的工作流文件格式')
    }
    
    const graph = data.workflow
    
    // 重新生成ID避免冲突
    workflow_id.value = `wf_${Date.now()}`
    workflow_name.value = graph.name || '导入的工作流'
    workflow_description.value = data.metadata?.description || ''
    workflow_tags.value = data.metadata?.tags || ''
    workflow_version.value = graph.version || 'v1.0.0'
    
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
    
    add_log('SUCCESS', `工作流已导入: ${workflow_name.value}`)
    toast.success('工作流已导入')
    
    // 清空文件输入
    target.value = ''
  } catch (e: any) {
    add_log('ERROR', `导入失败: ${e.message}`)
    toast.error(`导入失败：${e.message}`)
    target.value = ''
  }
}

// 导出工作流为图片
const export_workflow_image = async () => {
  const toast = useToast()
  try {
    // 使用html2canvas库导出（需要先安装）
    toast.info('图片导出功能需要安装html2canvas库')
    add_log('INFO', '图片导出功能待实现')
    // TODO: 实现图片导出
    // const canvas = await html2canvas(flowchartContainer)
    // const url = canvas.toDataURL('image/png')
    // download(url, `${workflow_name.value}.png`)
  } catch (e: any) {
    toast.error(`导出失败：${e}`)
  }
}

const wf_events = useWorkflowEvents()
const setup_event_listeners = async () => {
  await wf_events.on_step_start((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) {
      flow_ref.value?.updateNodeStatus(step_id, 'running')
      add_log('INFO', `节点开始执行`, step_id)
    }
  })
  await wf_events.on_step_complete((p: any) => {
    const step_id = p?.step_id || p?.stepId
    if (step_id) {
      flow_ref.value?.updateNodeStatus(step_id, 'completed')
      add_log('SUCCESS', `节点执行完成`, step_id)
    }
  })
  await wf_events.on_run_complete(() => {
    add_log('SUCCESS', '工作流执行完成')
  })
}

const on_node_click = (node: any) => {
  ignore_close_once.value = true
  selected_node.value = node
  const current = node.params || {}
  param_values.value = JSON.parse(JSON.stringify(current))
  drawer_open.value = true
}

const save_params = () => {
  if (!selected_node.value) return
  
  // 如果是通知节点，需要附加通知规则的配置信息
  if (selected_node.value.type === 'notify' && param_values.value.notification_rule_id) {
    const rule = notification_rules.value.find(r => r.id === param_values.value.notification_rule_id)
    if (rule) {
      // 将通知规则的channel和config附加到参数中，供工作流执行时使用
      param_values.value._notification_channel = rule.channel
      param_values.value._notification_config = rule.config
    }
  }
  
  flow_ref.value?.updateNodeParams(selected_node.value.id, param_values.value)
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
  workflow_name.value = `${workflow_name.value} (副本)`
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
      isTemplate: true
    })
    add_log('SUCCESS', `已保存为模板: ${workflow_name.value}`)
    toast.success('已保存为模板')
    await load_template_list()
  } catch (e: any) {
    add_log('ERROR', `保存模板失败: ${e}`)
    toast.error(`保存模板失败：${e}`)
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
    add_log('INFO', `找到 ${matches.length} 个匹配的节点`)
    // 滚动到第一个匹配的节点
    const first = matches[0]
    // TODO: 实现画布滚动到节点位置
  } else {
    add_log('WARN', '未找到匹配的节点')
  }
}

onMounted(async () => {
  await refresh_catalog()
  await setup_event_listeners()
  load_notification_rules()
  
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
  
  const handle_global_click = (e: MouseEvent) => {
    if (!drawer_open.value) return
    if (ignore_close_once.value) { ignore_close_once.value = false; return }
    const drawer = drawer_ref.value
    if (drawer && drawer.contains(e.target as Node)) return
    drawer_open.value = false
  }
  window.addEventListener('click', handle_global_click)
  onUnmounted(() => {
    window.removeEventListener('click', handle_global_click)
  })
})

onUnmounted(() => { wf_events.unsubscribe_all() })
</script>

<style scoped>
</style>
