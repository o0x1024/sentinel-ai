<template>
  <div v-bind="$attrs" class="page-content h-full flex flex-col p-4 gap-4">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-bold flex items-center gap-2">
        <i class="fas fa-robot text-primary"></i>
        Agent管理
      </h1>
      <div class="flex gap-2">
        <button class="btn btn-sm btn-primary" @click="openCreateModal"><i class="fas fa-plus"></i> 新增Agent</button>
        <button class="btn btn-sm" @click="loadAgents"><i class="fas fa-sync"></i> 刷新</button>
      </div>
    </div>

    <div v-if="loading" class="alert">
      <i class="fas fa-spinner animate-spin"></i>
      正在加载...
    </div>

    <div v-if="agents.length === 0 && !loading" class="text-sm opacity-70">暂无Agent，点击“新增Agent”开始配置。</div>

    <div class="overflow-y-auto">
      <table class="table table-zebra w-full">
        <thead>
          <tr>
            <th class="w-28">状态</th>
            <th class="w-56">名称</th>
            <th class="w-40">引擎</th>
            <th>描述</th>
            <th class="w-24">版本</th>
            <th class="w-48">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(agent, idx) in agents" :key="agent.id || idx">
            <td>
              <div class="flex items-center gap-2">
                <span class="badge" :class="agent.enabled ? 'badge-success' : 'badge-ghost'">{{ agent.enabled ? '启用' : '停用' }}</span>
              </div>
            </td>
            <td>
              <div class="text-sm font-medium">{{ agent.name || '未命名' }}</div>
              <div class="text-xs opacity-60">ID: {{ agent.id }}</div>
            </td>
            <td>
              <span class="badge badge-outline">{{ agent.engine }}</span>
            </td>
            <td>
              <div class="text-sm max-w-xs truncate" :title="agent.description">{{ agent.description || '无描述' }}</div>
            </td>
            <td>
              <span class="text-sm">{{ agent.version || '1.0.0' }}</span>
            </td>
            <td>
              <div class="flex gap-2 justify-end">
                <button class="btn btn-sm btn-info" @click="viewAgentDetails(agent)" title="查看详情"><i class="fas fa-eye"></i></button>
                <button class="btn btn-sm btn-primary" @click="editAgent(agent)" title="编辑设置"><i class="fas fa-edit"></i></button>
                <button class="btn btn-sm btn-error" @click="removeAgent(idx)" title="删除"><i class="fas fa-trash"></i></button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 编辑弹窗 -->
  <dialog :class="['modal', { 'modal-open': showEditModal }]">
    <div class="modal-box max-w-4xl">
      <div class="flex items-center justify-between mb-3">
        <h3 class="font-bold text-lg">
          <i class="fas fa-edit mr-2"></i>
          {{ isCreating ? '新增Agent' : '编辑Agent' }}
        </h3>
        <button class="btn btn-sm btn-ghost" @click="closeEditModal">✕</button>
      </div>

      <div v-if="editingAgent" class="space-y-4">
        <!-- 基本信息 -->
        <div class="grid grid-cols-2 gap-3">
          <div class="form-control">
            <label class="label"><span class="label-text">名称 *</span></label>
            <input v-model="editingAgent.name" class="input input-sm input-bordered w-full" placeholder="例如：安全扫描助手" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">启用状态</span></label>
            <input type="checkbox" class="toggle toggle-sm" v-model="editingAgent.enabled" />
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="form-control">
            <label class="label"><span class="label-text">引擎</span></label>
            <select v-model="editingAgent.engine" class="select select-sm select-bordered w-full">
              <option value="auto">auto</option>
              <option value="plan-execute">plan-execute</option>
              <option value="react">react</option>
              <option value="rewoo">rewoo</option>
              <option value="llm-compiler">llm-compiler</option>
            </select>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">版本</span></label>
            <input v-model="editingAgent.version" class="input input-sm input-bordered w-full" placeholder="1.0.0" />
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">描述</span></label>
          <textarea v-model="editingAgent.description" class="textarea textarea-bordered textarea-sm" rows="3" placeholder="该Agent的用途描述"></textarea>
        </div>

        <!-- 执行参数 -->
        <div class="divider">执行参数</div>
        <div class="grid grid-cols-3 gap-3">
          <div class="form-control">
            <label class="label"><span class="label-text">超时(秒)</span></label>
            <input v-model.number="editingAgent.execution.timeout_sec" type="number" min="0" class="input input-sm input-bordered w-full" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">重试次数</span></label>
            <input v-model.number="editingAgent.execution.retry.max_retries" type="number" min="0" class="input input-sm input-bordered w-full" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">退避策略</span></label>
            <select v-model="editingAgent.execution.retry.backoff" class="select select-sm select-bordered w-full">
              <option value="fixed">fixed</option>
              <option value="linear">linear</option>
              <option value="exponential">exponential</option>
            </select>
          </div>
        </div>

        <!-- 统一提示词系统 -->
        <div class="divider">提示词策略</div>
        
        <!-- 策略选择 -->
        <div class="form-control">
          <label class="label"><span class="label-text">提示词策略</span></label>
          <div class="flex gap-4">
            <label class="label cursor-pointer">
              <input type="radio" v-model="editingAgent.prompt_strategy" value="follow_group" class="radio radio-sm" />
              <span class="label-text ml-2">跟随分组</span>
            </label>
            <label class="label cursor-pointer">
              <input type="radio" v-model="editingAgent.prompt_strategy" value="custom" class="radio radio-sm" />
              <span class="label-text ml-2">自定义选择</span>
            </label>
            <label class="label cursor-pointer">
              <input type="radio" v-model="editingAgent.prompt_strategy" value="user_config" class="radio radio-sm" />
              <span class="label-text ml-2">使用用户配置</span>
            </label>
          </div>
        </div>

        <!-- 分组选择 -->
        <div v-if="editingAgent.prompt_strategy === 'follow_group'" class="form-control">
          <label class="label"><span class="label-text">选择Prompt分组</span></label>
          <select v-model="editingAgent.group_id" class="select select-sm select-bordered w-full">
            <option :value="null">选择分组</option>
            <option v-for="group in promptGroups" :key="group.id" :value="group.id">
              {{ group.name }} - {{ group.architecture }} {{ group.is_default ? '(默认)' : '' }}
            </option>
          </select>
        </div>

        <!-- 规范阶段展示 -->
        <div v-if="editingAgent.prompt_strategy === 'custom'" class="space-y-3">
          <div class="text-sm font-medium text-base-content/70">规范阶段配置</div>
          
          <!-- System阶段 -->
          <div class="grid grid-cols-2 gap-3 p-3 bg-base-100 rounded">
            <div class="form-control">
              <label class="label label-text text-xs">系统提示</label>
              <select v-model="editingAgent.prompt_ids.system" class="select select-xs select-bordered">
                <option :value="null">不设置</option>
                <option v-for="tpl in promptTemplates.filter(t => t.template_type === 'SystemPrompt')" 
                        :key="tpl.id" :value="tpl.id">
                  {{ tpl.name }}
                </option>
              </select>
            </div>
            <div class="form-control">
              <textarea v-model="editingAgent.prompts.system" class="textarea textarea-xs textarea-bordered" 
                       rows="2" placeholder="系统级提示词覆盖"></textarea>
            </div>
          </div>

          <!-- Planner阶段 -->
          <div class="grid grid-cols-2 gap-3 p-3 bg-base-100 rounded">
            <div class="form-control">
              <label class="label label-text text-xs">规划器</label>
              <select v-model="editingAgent.prompt_ids.planner" class="select select-xs select-bordered">
                <option :value="null">不设置</option>
                <option v-for="tpl in promptTemplates.filter(t => t.template_type === 'Planner')" 
                        :key="tpl.id" :value="tpl.id">
                  {{ tpl.name }} ({{ tpl.architecture }})
                </option>
              </select>
            </div>
            <div class="form-control">
              <textarea v-model="editingAgent.prompts.planner" class="textarea textarea-xs textarea-bordered" 
                       rows="2" placeholder="规划阶段提示词覆盖"></textarea>
            </div>
          </div>

          <!-- Executor阶段 -->
          <div class="grid grid-cols-2 gap-3 p-3 bg-base-100 rounded">
            <div class="form-control">
              <label class="label label-text text-xs">执行器</label>
              <select v-model="editingAgent.prompt_ids.executor" class="select select-xs select-bordered">
                <option :value="null">不设置</option>
                <option v-for="tpl in promptTemplates.filter(t => t.template_type === 'Executor')" 
                        :key="tpl.id" :value="tpl.id">
                  {{ tpl.name }} ({{ tpl.architecture }})
                </option>
              </select>
            </div>
            <div class="form-control">
              <textarea v-model="editingAgent.prompts.executor" class="textarea textarea-xs textarea-bordered" 
                       rows="2" placeholder="执行阶段提示词覆盖"></textarea>
            </div>
          </div>

          <!-- Replanner阶段 -->
          <div class="grid grid-cols-2 gap-3 p-3 bg-base-100 rounded">
            <div class="form-control">
              <label class="label label-text text-xs">重规划器</label>
              <select v-model="editingAgent.prompt_ids.replanner" class="select select-xs select-bordered">
                <option :value="null">不设置</option>
                <option v-for="tpl in promptTemplates.filter(t => t.template_type === 'Replanner')" 
                        :key="tpl.id" :value="tpl.id">
                  {{ tpl.name }} ({{ tpl.architecture }})
                </option>
              </select>
            </div>
            <div class="form-control">
              <textarea v-model="editingAgent.prompts.replanner" class="textarea textarea-xs textarea-bordered" 
                       rows="2" placeholder="重规划阶段提示词覆盖"></textarea>
            </div>
          </div>

          <!-- Evaluator阶段 -->
          <div class="grid grid-cols-2 gap-3 p-3 bg-base-100 rounded">
            <div class="form-control">
              <label class="label label-text text-xs">评估器</label>
              <select v-model="editingAgent.prompt_ids.evaluator" class="select select-xs select-bordered">
                <option :value="null">不设置</option>
                <option v-for="tpl in promptTemplates.filter(t => t.template_type === 'Evaluator')" 
                        :key="tpl.id" :value="tpl.id">
                  {{ tpl.name }} ({{ tpl.architecture }})
                </option>
              </select>
            </div>
            <div class="form-control">
              <textarea v-model="editingAgent.prompts.evaluator" class="textarea textarea-xs textarea-bordered" 
                       rows="2" placeholder="评估阶段提示词覆盖"></textarea>
            </div>
          </div>
        </div>

        <!-- 最终生效Prompt预览 -->
        <div class="form-control">
          <div class="flex items-center justify-between">
            <label class="label"><span class="label-text">最终生效Prompt预览</span></label>
            <button type="button" class="btn btn-xs btn-outline" @click="previewFinalPrompt">
              预览生效配置
            </button>
          </div>
          <div v-if="showPromptPreview" class="bg-base-100 p-3 rounded text-xs">
            <pre class="whitespace-pre-wrap">{{ finalPromptPreview }}</pre>
          </div>
        </div>

        <!-- 工具选择 -->
        <div class="divider">可用工具</div>
        <div class="max-h-64 overflow-y-auto p-4 bg-base-100 rounded border space-y-4">
          <!-- 工具栏：展开/折叠全部 + 刷新 -->
          <div class="flex items-center gap-2 pb-2">
            <button type="button" class="btn btn-xs" @click="expandAllTools">展开全部</button>
            <button type="button" class="btn btn-xs" @click="collapseAllTools">折叠全部</button>
            <button type="button" class="btn btn-xs btn-outline btn-primary" @click="refreshTools" :disabled="isRefreshingTools">
              <i :class="['fas', 'fa-sync-alt', { 'fa-spin': isRefreshingTools }]"></i>
              <span class="ml-1">刷新工具</span>
            </button>
          </div>
          <!-- 全选 -->
          <div class="flex items-center gap-2 pb-2 border-b border-base-300">
            <input type="checkbox" class="checkbox checkbox-sm"
                   :checked="allToolsSelected"
                   :indeterminate="someToolsSelected && !allToolsSelected"
                   @change="toggleAllTools($event)"/>
            <span class="text-sm">全选</span>
            <span class="text-xs opacity-60">(已选 {{ editingAgent?.tools.allow.length || 0 }})</span>
          </div>
          <!-- 内置工具 -->
          <div>
            <div class="flex items-center gap-2 text-xs font-semibold opacity-70 mb-2">
              <button type="button" class="btn btn-ghost btn-xs px-1" @click="toggleBuiltinCollapse">
                <i :class="['fas', showBuiltin ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
              </button>
              <input type="checkbox" class="checkbox checkbox-xs"
                     :checked="builtinAllSelected"
                     :indeterminate="builtinSomeSelected && !builtinAllSelected"
                     @change="toggleBuiltinGroup($event)"/>
              <span>内置工具</span>
            </div>
            <div v-show="showBuiltin" class="grid grid-cols-3 gap-2">
              <label v-for="tool in builtinTools" :key="'builtin-' + tool.name" class="label cursor-pointer gap-2 text-sm px-3 py-2 rounded hover:bg-base-200">
                <input type="checkbox" class="checkbox checkbox-sm"
                       :checked="editingAgent.tools.allow.includes(tool.name)"
                       @change="toggleToolForEdit(tool.name, $event)"/>
                <span class="flex-1">{{ tool.title || tool.name }}</span>
              </label>
              <div v-if="builtinTools.length === 0" class="text-xs opacity-60 col-span-3">暂无内置工具</div>
            </div>
          </div>

          <!-- MCP工具分组 -->
          <div v-for="group in mcpToolGroups" :key="'mcp-' + group.connection" class="pt-2 border-t border-base-300">
            <div class="flex items-center gap-2 text-xs font-semibold opacity-70 mb-2">
              <button type="button" class="btn btn-ghost btn-xs px-1" @click="toggleGroupCollapse(group.connection)">
                <i :class="['fas', !collapsedGroups[group.connection] ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
              </button>
              <input type="checkbox" class="checkbox checkbox-xs"
                     :checked="isGroupAllSelected(group.tools)"
                     :indeterminate="isGroupSomeSelected(group.tools) && !isGroupAllSelected(group.tools)"
                     @change="toggleMcpGroup(group.connection, group.tools, $event)"/>
              <span>MCP: {{ group.connection }}</span>
            </div>
            <div v-show="!collapsedGroups[group.connection]" class="grid grid-cols-3 gap-2">
              <label v-for="tool in group.tools" :key="group.connection + '-' + tool.name" class="label cursor-pointer gap-2 text-sm px-3 py-2 rounded hover:bg-base-200">
                <input type="checkbox" class="checkbox checkbox-sm"
                       :checked="editingAgent.tools.allow.includes(tool.name)"
                       @change="toggleToolForEdit(tool.name, $event)"/>
                <span class="flex-1">{{ tool.title || tool.name }}</span>
              </label>
              <div v-if="group.tools.length === 0" class="text-xs opacity-60 col-span-3">该连接暂无工具</div>
            </div>
          </div>

          <!-- 插件工具分组 -->
          <div class="pt-2 border-t border-base-300">
            <div class="flex items-center gap-2 text-xs font-semibold opacity-70 mb-2">
              <button type="button" class="btn btn-ghost btn-xs px-1" @click="togglePluginToolsCollapse">
                <i :class="['fas', showPluginTools ? 'fa-chevron-down' : 'fa-chevron-right']"></i>
              </button>
              <input type="checkbox" class="checkbox checkbox-xs"
                     :checked="pluginToolsAllSelected"
                     :indeterminate="pluginToolsSomeSelected && !pluginToolsAllSelected"
                     @change="togglePluginToolsGroup($event)"/>
              <span>插件工具</span>
              <span class="text-xs opacity-50">({{ pluginTools.length }})</span>
            </div>
            <div v-show="showPluginTools" class="grid grid-cols-3 gap-2">
              <label v-for="plugin in pluginTools" :key="'plugin-' + plugin.metadata.id" class="label cursor-pointer gap-2 text-sm px-3 py-2 rounded hover:bg-base-200">
                <input type="checkbox" class="checkbox checkbox-sm"
                       :checked="editingAgent.tools.allow.includes('plugin::' + plugin.metadata.id)"
                       @change="togglePluginTool(plugin.metadata.id, $event)"/>
                <span class="flex-1" :title="plugin.metadata.description">{{ plugin.metadata.name }}</span>
              </label>
              <div v-if="pluginTools.length === 0" class="text-xs opacity-60 col-span-3">暂无插件工具，可前往插件管理创建</div>
            </div>
          </div>
        </div>

        <div v-if="!isCreating" class="text-xs opacity-60">ID: {{ editingAgent.id }}</div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm" @click="closeEditModal">取消</button>
        <button class="btn btn-sm btn-primary" @click="confirmSaveAgent"><i class="fas fa-save mr-1"></i>保存</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="closeEditModal">close</button>
    </form>
  </dialog>

  <!-- 详情弹窗 -->
  <dialog :class="['modal', { 'modal-open': showDetailsModal }]" class="mt-8">
    <div class="modal-box max-w-3xl">
      <div class="flex items-center justify-between mb-3">
        <h3 class="font-bold text-lg"><i class="fas fa-eye mr-2"></i>Agent详情</h3>
        <button class="btn btn-sm btn-ghost" @click="closeDetailsModal">✕</button>
      </div>

      <div v-if="viewingAgent" class="space-y-4">
        <!-- 基本信息 -->
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">名称</span></label>
            <div class="text-lg font-medium">{{ viewingAgent.name || '未命名' }}</div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">状态</span></label>
            <span class="badge" :class="viewingAgent.enabled ? 'badge-success' : 'badge-ghost'">
              {{ viewingAgent.enabled ? '启用' : '停用' }}
            </span>
          </div>
        </div>

        <div class="grid grid-cols-3 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">引擎</span></label>
            <div class="text-sm">{{ viewingAgent.engine }}</div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">版本</span></label>
            <div class="text-sm">{{ viewingAgent.version || '1.0.0' }}</div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">ID</span></label>
            <div class="text-xs opacity-60 font-mono">{{ viewingAgent.id }}</div>
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">描述</span></label>
          <div class="text-sm">{{ viewingAgent.description || '无描述' }}</div>
        </div>

        <!-- 执行参数 -->
        <div class="divider">执行参数</div>
        <div class="grid grid-cols-3 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">超时时间</span></label>
            <div class="text-sm">{{ viewingAgent.execution.timeout_sec || 600 }} 秒</div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">重试次数</span></label>
            <div class="text-sm">{{ viewingAgent.execution.retry.max_retries || 1 }} 次</div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">退避策略</span></label>
            <div class="text-sm">{{ viewingAgent.execution.retry.backoff || 'fixed' }}</div>
          </div>
        </div>

        <!-- 提示词 -->
        <div class="divider">提示词配置</div>
        <div class="space-y-3">
          <!-- 策略信息 -->
          <div class="form-control">
            <label class="label"><span class="label-text">提示词策略</span></label>
            <div class="text-sm">
              <span class="badge badge-outline">
                {{ viewingAgent.prompt_strategy === 'follow_group' ? '跟随分组' : 
                   viewingAgent.prompt_strategy === 'custom' ? '自定义选择' : '使用用户配置' }}
              </span>
              <span v-if="viewingAgent.prompt_strategy === 'follow_group' && viewingAgent.group_id" class="ml-2 text-xs opacity-60">
                分组ID: {{ viewingAgent.group_id }}
              </span>
            </div>
          </div>
          
          <!-- 解析后的提示词内容：折叠面板显示 -->
          <div class="space-y-2">
            <div v-for="stage in viewingStages" :key="stage" class="collapse collapse-arrow bg-base-100 border border-base-300">
              <input type="checkbox" class="peer" />
              <div class="collapse-title text-sm font-medium flex items-center gap-2">
                <span class="badge badge-sm badge-outline">{{ stageLabels[stage] }}</span>
                <span class="text-xs opacity-60">(最终生效)</span>
                <span v-if="resolvingPromptsLoading" class="loading loading-spinner loading-xs ml-auto"></span>
              </div>
              <div class="collapse-content">
                <div class="p-4 bg-base-200 rounded-lg">
                  <div v-if="resolvingPromptsLoading" class="text-center opacity-70 py-4">
                    <span class="loading loading-spinner loading-sm"></span>
                    <span class="ml-2">解析中...</span>
                  </div>
                  <template v-else>
                    <div v-if="resolvedPrompts[stage]" class="prose prose-sm max-w-none">
                      <div class="markdown-content text-sm leading-relaxed text-base-content" 
                           v-html="renderMarkdown(resolvedPrompts[stage] || '')">
                      </div>
                    </div>
                    <div v-else class="text-sm opacity-60 text-center py-4">
                      未配置或解析失败
                    </div>
                  </template>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 工具列表 -->
        <div class="divider">可用工具</div>
        <div class="max-h-32 overflow-y-auto">
          <div class="flex flex-wrap gap-2">
            <span v-for="toolName in viewingAgent.tools.allow" :key="toolName" class="badge badge-outline">
              {{ getToolTitle(toolName) }}
            </span>
            <span v-if="viewingAgent.tools.allow.length === 0" class="text-sm opacity-60">未选择任何工具</span>
          </div>
        </div>

        <!-- 时间信息 -->
        <div class="divider">时间信息</div>
        <div class="grid grid-cols-2 gap-4 text-xs opacity-60">
          <div>创建时间: {{ formatDate(viewingAgent.created_at) }}</div>
          <div>更新时间: {{ formatDate(viewingAgent.updated_at) }}</div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm" @click="closeDetailsModal">关闭</button>
        <button class="btn btn-sm btn-primary" @click="editFromDetails"><i class="fas fa-edit mr-1"></i>编辑</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="closeDetailsModal">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref, onMounted, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useMessageUtils } from '@/composables/useMessageUtils'

const { renderMarkdown } = useMessageUtils()

interface AgentProfile {
  id: string
  name: string
  description?: string
  enabled: boolean
  version?: string
  engine: string
  llm: { default: { provider: string, model: string, temperature?: number|null, max_tokens?: number|null } }
  prompts: { system?: string|null, planner?: string|null, executor?: string|null, replanner?: string|null, evaluator?: string|null }
  prompt_ids?: { system?: number|null, planner?: number|null, executor?: number|null, replanner?: number|null, evaluator?: number|null }
  // 新增统一提示词系统字段
  prompt_strategy?: 'follow_group' | 'custom' | 'user_config'
  group_id?: number | null
  pinned_versions?: Record<number, string>
  tools: { allow: string[], deny?: string[]|null }
  execution: { timeout_sec?: number|null, retry: { max_retries: number, backoff: string, interval_ms?: number|null }, concurrency?: number|null, strict_mode?: boolean|null }
  created_at?: string
  updated_at?: string
}

const agents = ref<AgentProfile[]>([])
const showEditModal = ref(false)
const showDetailsModal = ref(false)
const editingAgent = ref<AgentProfile | null>(null)
const viewingAgent = ref<AgentProfile | null>(null)
type StageKey = 'system' | 'planner' | 'executor' | 'replanner' | 'evaluator'
const resolvedPrompts = ref<Partial<Record<StageKey, string>>>({})
const resolvingPromptsLoading = ref(false)
const viewingStages = ref<StageKey[]>([])
const stageLabels: Record<StageKey, string> = {
  system: '系统提示',
  planner: '规划器',
  executor: '执行器',
  replanner: '重规划器',
  evaluator: '评估器',
}
const computeStages = (engine: string | null | undefined): StageKey[] => {
  switch (engine) {
    case 'react':
      return ['system', 'planner'] // ReAct 使用 system 和 planner（作为主循环提示）
    case 'rewoo':
      return ['system', 'planner', 'executor', 'evaluator']
    case 'llm-compiler':
      return ['system', 'planner', 'executor', 'replanner', 'evaluator']
    case 'plan-execute':
    default:
      // 默认走Plan-Execute
      return ['system', 'planner', 'executor', 'replanner']
  }
}

const mapArchToEngine = (arch?: string | null): string | null => {
  switch (arch) {
    case 'ReAct': return 'react'
    case 'ReWOO': return 'rewoo'
    case 'LLMCompiler': return 'llm-compiler'
    case 'PlanExecute': return 'plan-execute'
    default: return null
  }
}

const getEffectiveEngineForAgent = (agent: AgentProfile): string => {
  if (agent.prompt_strategy === 'follow_group' && agent.group_id) {
    const group = promptGroups.value.find(g => g.id === agent.group_id)
    const mapped = mapArchToEngine(group?.architecture)
    if (mapped) return mapped
  }
  if (!agent.engine || agent.engine === 'auto') {
    return 'plan-execute'
  }
  return agent.engine
}
const isCreating = ref(false)
const toolsCatalog = ref<any[]>([])
const builtinTools = ref<any[]>([])
const mcpToolGroups = ref<Array<{ connection: string, tools: any[] }>>([])
const pluginTools = ref<any[]>([]) // 插件工具列表
const showBuiltin = ref(true)
const showPluginTools = ref(true) // 插件工具展开状态
const collapsedGroups = ref<Record<string, boolean>>({})
const promptTemplates = ref<any[]>([])
const selectedPlannerTemplateEdit = ref<number | null>(null)
const selectedExecutorTemplateEdit = ref<number | null>(null)
const loading = ref(false)
const isRefreshingTools = ref(false)

// 统一提示词系统相关数据
const promptGroups = ref<any[]>([])
const finalPromptPreview = ref('')
const showPromptPreview = ref(false)

const newProfile = (): AgentProfile => ({
  id: `agent_${Date.now()}`,
  name: '',
  description: '',
  enabled: true,
  version: '1.0.0',
  engine: 'auto',
  llm: { default: { provider: 'auto', model: 'auto', temperature: null, max_tokens: null } },
  prompts: { system: null, planner: null, executor: null, replanner: null, evaluator: null },
  prompt_ids: { system: null, planner: null, executor: null, replanner: null, evaluator: null },
  // 新增统一提示词系统字段默认值
  prompt_strategy: 'follow_group',
  group_id: null,
  pinned_versions: {},
  tools: { allow: [], deny: null },
  execution: { timeout_sec: 600, retry: { max_retries: 1, backoff: 'fixed', interval_ms: null }, concurrency: null, strict_mode: null },
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
})

const normalizeAgent = (a: any): AgentProfile => {
  const llmDefault = (a?.llm && a.llm.default) ? a.llm.default : { provider: 'auto', model: 'auto', temperature: null, max_tokens: null }
  const prompts = a?.prompts ?? { system: null, planner: null, executor: null, replanner: null, evaluator: null }
  const prompt_ids = a?.prompt_ids ?? { system: null, planner: null, executor: null, replanner: null, evaluator: null }
  const tools = a?.tools ?? { allow: [], deny: null }
  const execution = a?.execution ?? { timeout_sec: 600, retry: { max_retries: 1, backoff: 'fixed', interval_ms: null }, concurrency: null, strict_mode: null }
  return {
    id: a?.id ?? `agent_${Date.now()}`,
    name: a?.name ?? '',
    description: a?.description ?? '',
    enabled: a?.enabled ?? true,
    version: a?.version ?? '1.0.0',
    engine: a?.engine ?? 'auto',
    llm: { default: llmDefault },
    prompts,
    prompt_ids,
    // 统一提示词系统字段
    prompt_strategy: a?.prompt_strategy ?? 'follow_group',
    group_id: a?.group_id ?? null,
    pinned_versions: a?.pinned_versions ?? {},
    tools,
    execution,
    created_at: a?.created_at ?? new Date().toISOString(),
    updated_at: a?.updated_at ?? new Date().toISOString(),
  } as AgentProfile
}

const loadAgents = async () => {
  loading.value = true
  try {
    const list = await invoke<any[]>('list_scenario_agents').catch(() => [])
    agents.value = Array.isArray(list) && list.length > 0 ? list.map(normalizeAgent) as AgentProfile[] : []
  } finally {
    loading.value = false
  }
}

const loadTools = async () => {
  try {
    const grouped = await invoke<any>('list_unified_tools_grouped').catch(() => null)
    if (grouped && grouped.builtin && grouped.mcp) {
      builtinTools.value = Array.isArray(grouped.builtin) ? grouped.builtin.filter((t: any) => t?.available) : []
      mcpToolGroups.value = Array.isArray(grouped.mcp)
        ? grouped.mcp.map((g: any) => ({ connection: g.connection, tools: (g.tools || []).filter((t: any) => t?.available) }))
        : []
      // 扁平化供 getToolTitle 使用
      toolsCatalog.value = [...builtinTools.value, ...mcpToolGroups.value.flatMap(g => g.tools)]
      // 初始化折叠状态
      const map: Record<string, boolean> = {}
      for (const g of mcpToolGroups.value) map[g.connection] = false
      collapsedGroups.value = map
    } else {
      builtinTools.value = []
      mcpToolGroups.value = []
      toolsCatalog.value = []
      collapsedGroups.value = {}
    }
  } catch {
    builtinTools.value = []
    mcpToolGroups.value = []
    toolsCatalog.value = []
    collapsedGroups.value = {}
  }
}

// 加载插件工具（仅 agentTools 类型）
const loadPluginTools = async () => {
  try {
    const response = await invoke<any>('list_plugins')
    if (response.success && response.data) {
      // 只加载 agentTools 类型的已启用插件
      pluginTools.value = response.data.filter((plugin: any) => 
        plugin.metadata.category === 'agentTools' && plugin.status === 'Enabled'
      )
    } else {
      pluginTools.value = []
    }
  } catch (error) {
    console.error('Failed to load plugin tools:', error)
    pluginTools.value = []
  }
}

const loadPromptTemplates = async () => {
  try {
    promptTemplates.value = await invoke<any[]>('list_prompt_templates_api').catch(() => [])
  } catch { promptTemplates.value = [] }
}

// 将前端 engine 字符串映射为后端 ArchitectureType
const mapEngineToArchitectureType = (engine: string | null | undefined): string | null => {
  if (!engine || engine === 'auto') return null
  switch (engine) {
    case 'plan-execute': return 'PlanExecute'
    case 'react': return 'ReAct'
    case 'rewoo': return 'ReWOO'
    case 'llm-compiler': return 'LLMCompiler'
    default: return null
  }
}

const loadPromptGroups = async (engine?: string) => {
  try {
    const arch = mapEngineToArchitectureType(engine)
    promptGroups.value = await invoke<any[]>('list_prompt_groups_api', { architecture: arch }).catch(() => [])
  } catch { promptGroups.value = [] }
}

const openCreateModal = async () => {
  editingAgent.value = newProfile()
  isCreating.value = true
  selectedPlannerTemplateEdit.value = null
  selectedExecutorTemplateEdit.value = null
  // 刷新工具列表以获取最新数据
  await loadTools()
  await loadPluginTools() // 确保插件工具列表最新
  showEditModal.value = true
}

const editAgent = async (agent: AgentProfile) => {
  editingAgent.value = JSON.parse(JSON.stringify(agent)) // 深拷贝
  isCreating.value = false
  selectedPlannerTemplateEdit.value = agent.prompt_ids?.planner ?? null
  selectedExecutorTemplateEdit.value = agent.prompt_ids?.executor ?? null
  // 刷新工具列表以获取最新数据
  await loadTools()
  await loadPluginTools() // 确保插件工具列表最新
  showEditModal.value = true
}

const viewAgentDetails = async (agent: AgentProfile) => {
  viewingAgent.value = agent
  // 若跟随分组，则以分组的架构为准计算阶段；否则用agent.engine
  const effective = getEffectiveEngineForAgent(agent)
  viewingStages.value = computeStages(effective)
  showDetailsModal.value = true
  
  // 解析最终生效的提示词
  await resolveAgentPrompts(agent)
}

const resolveAgentPrompts = async (agent: AgentProfile) => {
  resolvedPrompts.value = {}
  const stages = computeStages(agent.engine)
  resolvingPromptsLoading.value = true
  
  try {
    const agent_config = {
      prompt_strategy: agent.prompt_strategy,
      group_id: agent.group_id,
      prompt_ids: agent.prompt_ids,
      prompts: agent.prompts,
      pinned_versions: agent.pinned_versions,
    }
    
    // 并行解析所有阶段
    await Promise.all(stages.map(async (stage) => {
      try {
        const content = await invoke<string>('preview_resolved_prompt_api', {
          engine: getEffectiveEngineForAgent(agent),
          stage,
          agentConfig: agent_config,
        })
        const normalized = (content || '').trim()
        resolvedPrompts.value[stage] = normalized && normalized !== '' ? normalized : undefined
      } catch (e) {
        console.warn(`Failed to resolve ${stage} prompt:`, e)
        resolvedPrompts.value[stage] = (agent.prompts as any)?.[stage] || undefined
      }
    }))
  } catch (error) {
    console.error('Failed to resolve agent prompts:', error)
    // 降级到直接显示agent.prompts中的内容
    for (const s of stages) {
      resolvedPrompts.value[s] = (agent.prompts as any)?.[s] || undefined
    }
  }
  finally {
    resolvingPromptsLoading.value = false
  }
}

const editFromDetails = () => {
  if (viewingAgent.value) {
    closeDetailsModal()
    editAgent(viewingAgent.value)
  }
}

const toggleAgentStatus = async (agent: AgentProfile) => {
  agent.enabled = !agent.enabled
  agent.updated_at = new Date().toISOString()
  try {
    await saveAgent(agent)
  } catch (e) {
    console.warn('Failed to update agent status', e)
    agent.enabled = !agent.enabled // 回滚
  }
}
const removeAgent = async (idx: number) => {
  const item = agents.value[idx]
  agents.value.splice(idx, 1)
  if (item?.id) {
    try { await invoke('delete_scenario_agent', { id: item.id }) } catch {}
  }
}

const toggleToolForEdit = (name: string, e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  const set = new Set(editingAgent.value.tools.allow)
  if (checked) set.add(name); else set.delete(name)
  editingAgent.value.tools.allow = Array.from(set)
}

const getToolTitle = (toolName: string) => {
  // 检查是否是插件工具
  if (toolName.startsWith('plugin::')) {
    const pluginId = toolName.substring(8) // 移除 'plugin::' 前缀
    const plugin = pluginTools.value.find(p => p.metadata.id === pluginId)
    return plugin ? `${plugin.metadata.name} (插件)` : toolName
  }
  // 普通工具
  const tool = toolsCatalog.value.find(t => t.name === toolName)
  return tool?.title || toolName
}

// 计算属性：全选状态
const allToolsSelected = computed(() => {
  if (!editingAgent.value) return false
  const total = builtinTools.value.length + 
                mcpToolGroups.value.reduce((acc, g) => acc + g.tools.length, 0) +
                pluginTools.value.length
  return total > 0 && editingAgent.value.tools.allow.length >= total
})
const someToolsSelected = computed(() => {
  if (!editingAgent.value) return false
  return editingAgent.value.tools.allow.length > 0
})

// 计算属性：内置分组选中状态
const builtinAllSelected = computed(() => {
  if (!editingAgent.value) return false
  const names = new Set(builtinTools.value.map(t => t.name))
  return names.size > 0 && Array.from(names).every(n => editingAgent.value!.tools.allow.includes(n))
})
const builtinSomeSelected = computed(() => {
  if (!editingAgent.value) return false
  const names = new Set(builtinTools.value.map(t => t.name))
  return Array.from(names).some(n => editingAgent.value!.tools.allow.includes(n))
})

// 分组选择判定
const isGroupAllSelected = (tools: any[]) => {
  if (!editingAgent.value) return false
  const names = tools.map(t => t.name)
  return names.length > 0 && names.every(n => editingAgent.value!.tools.allow.includes(n))
}
const isGroupSomeSelected = (tools: any[]) => {
  if (!editingAgent.value) return false
  const names = tools.map(t => t.name)
  return names.some(n => editingAgent.value!.tools.allow.includes(n))
}

// 动作：全选/全不选
const toggleAllTools = (e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  if (checked) {
    const all = [
      ...builtinTools.value.map(t => t.name),
      ...mcpToolGroups.value.flatMap(g => g.tools.map(t => t.name)),
      ...pluginTools.value.map(p => `plugin::${p.metadata.id}`)
    ]
    editingAgent.value.tools.allow = Array.from(new Set(all))
  } else {
    editingAgent.value.tools.allow = []
  }
}

// 动作：内置分组切换
const toggleBuiltinGroup = (e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  const names = builtinTools.value.map(t => t.name)
  const set = new Set(editingAgent.value.tools.allow)
  if (checked) names.forEach(n => set.add(n)); else names.forEach(n => set.delete(n))
  editingAgent.value.tools.allow = Array.from(set)
}

// 动作：MCP分组切换
const toggleMcpGroup = (_conn: string, tools: any[], e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  const names = tools.map(t => t.name)
  const set = new Set(editingAgent.value.tools.allow)
  if (checked) names.forEach(n => set.add(n)); else names.forEach(n => set.delete(n))
  editingAgent.value.tools.allow = Array.from(set)
}

// 折叠/展开
const toggleBuiltinCollapse = () => { showBuiltin.value = !showBuiltin.value }
const toggleGroupCollapse = (conn: string) => {
  collapsedGroups.value[conn] = !collapsedGroups.value[conn]
  collapsedGroups.value = { ...collapsedGroups.value }
}

// 插件工具相关计算属性
const pluginToolsAllSelected = computed(() => {
  if (!editingAgent.value || pluginTools.value.length === 0) return false
  const pluginNames = pluginTools.value.map(p => `plugin::${p.metadata.id}`)
  return pluginNames.every(n => editingAgent.value!.tools.allow.includes(n))
})

const pluginToolsSomeSelected = computed(() => {
  if (!editingAgent.value || pluginTools.value.length === 0) return false
  const pluginNames = pluginTools.value.map(p => `plugin::${p.metadata.id}`)
  return pluginNames.some(n => editingAgent.value!.tools.allow.includes(n))
})

// 插件工具切换方法
const togglePluginToolsCollapse = () => { showPluginTools.value = !showPluginTools.value }

const togglePluginToolsGroup = (e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  const pluginNames = pluginTools.value.map(p => `plugin::${p.metadata.id}`)
  const set = new Set(editingAgent.value.tools.allow)
  if (checked) {
    pluginNames.forEach(n => set.add(n))
  } else {
    pluginNames.forEach(n => set.delete(n))
  }
  editingAgent.value.tools.allow = Array.from(set)
}

const togglePluginTool = (pluginId: string, e: Event) => {
  if (!editingAgent.value) return
  const checked = (e.target as HTMLInputElement).checked
  const toolName = `plugin::${pluginId}`
  const set = new Set(editingAgent.value.tools.allow)
  if (checked) {
    set.add(toolName)
  } else {
    set.delete(toolName)
  }
  editingAgent.value.tools.allow = Array.from(set)
}

const expandAllTools = () => {
  showBuiltin.value = true
  showPluginTools.value = true
  const map: Record<string, boolean> = {}
  for (const g of mcpToolGroups.value) map[g.connection] = false
  collapsedGroups.value = map
}
const collapseAllTools = () => {
  showBuiltin.value = false
  showPluginTools.value = false
  const map: Record<string, boolean> = {}
  for (const g of mcpToolGroups.value) map[g.connection] = true
  collapsedGroups.value = map
}

// 手动刷新工具列表
const refreshTools = async () => {
  isRefreshingTools.value = true
  try {
    await Promise.all([loadTools(), loadPluginTools()])
  } finally {
    isRefreshingTools.value = false
  }
}

const formatDate = (dateStr?: string) => {
  if (!dateStr) return '未知'
  return new Date(dateStr).toLocaleString('zh-CN')
}

const saveAgent = async (agent: AgentProfile) => {
  agent.updated_at = new Date().toISOString()
  await invoke('save_scenario_agent', { profile: agent })
}


const resetAgent = (agent: AgentProfile) => {
  // noop placeholder; could reload original from backend if needed
}

onMounted(async () => {
  await Promise.all([loadAgents(), loadTools(), loadPluginTools(), loadPromptTemplates(), loadPromptGroups()])
  
  // 监听插件变化事件
  listen('plugin:changed', async () => {
    await loadPluginTools()
  })
  
  // 监听MCP工具变更事件
  listen('mcp:tools-changed', async () => {
    console.log('AgentManager: MCP tools changed, refreshing tools...')
    await loadTools()
    await loadPluginTools()
  })
})

// 当编辑中的Agent切换引擎时，按引擎过滤提示词分组
watch(() => editingAgent.value?.engine, async (eng) => {
  await loadPromptGroups(eng ?? undefined)
})

const applyTemplateToEdit = async (kind: 'planner' | 'executor', templateId: number|null) => {
  if (!editingAgent.value || !templateId) return
  
  if (!editingAgent.value.prompt_ids) {
    editingAgent.value.prompt_ids = { system: null, planner: null, executor: null, replanner: null, evaluator: null }
  }
  
  if (kind === 'planner') editingAgent.value.prompt_ids.planner = templateId
  if (kind === 'executor') editingAgent.value.prompt_ids.executor = templateId
  
  // 获取模板内容并应用到提示词
  try {
    const template = promptTemplates.value.find(t => t.id === templateId)
    if (template?.content) {
      if (kind === 'planner') editingAgent.value.prompts.planner = template.content
      if (kind === 'executor') editingAgent.value.prompts.executor = template.content
    }
  } catch (e) {
    console.warn('Failed to apply template', e)
  }
}

const closeEditModal = () => {
  showEditModal.value = false
  editingAgent.value = null
  isCreating.value = false
  selectedPlannerTemplateEdit.value = null
  selectedExecutorTemplateEdit.value = null
}

const closeDetailsModal = () => {
  showDetailsModal.value = false
  viewingAgent.value = null
  resolvedPrompts.value = {}
}

const previewFinalPrompt = async () => {
  if (!editingAgent.value) return
  
  try {
    let preview = '最终生效Prompt配置:\n\n'
    
    if (editingAgent.value.prompt_strategy === 'follow_group') {
      preview += '策略: 跟随分组\n'
      if (editingAgent.value.group_id) {
        const group = promptGroups.value.find(g => g.id === editingAgent.value!.group_id)
        preview += `分组: ${group?.name || '未找到'} (${group?.architecture})\n`
        preview += '注: 将使用分组中配置的各阶段模板\n'
        const arch = mapEngineToArchitectureType(editingAgent.value.engine)
        if (arch && group?.architecture && group.architecture !== arch) {
          preview += '⚠️ 分组架构与当前引擎不匹配，可能无法命中对应阶段模板\n'
        }
      } else {
        preview += '分组: 未选择\n'
      }
    } else if (editingAgent.value.prompt_strategy === 'custom') {
      preview += '策略: 自定义选择\n'
      const stages = ['system', 'planner', 'executor', 'replanner', 'evaluator'] as const
      for (const stage of stages) {
        const templateId = editingAgent.value.prompt_ids?.[stage]
        const overrideText = editingAgent.value.prompts?.[stage]
        
        if (templateId || overrideText) {
          preview += `\n${stage}阶段:\n`
          if (templateId) {
            const template = promptTemplates.value.find(t => t.id === templateId)
            preview += `  模板: ${template?.name || '未找到'}\n`
          }
          if (overrideText) {
            preview += `  覆盖: ${overrideText.substring(0, 100)}...\n`
          }
        }
      }
    } else {
      preview += '策略: 使用用户配置\n'
      preview += '注: 将使用用户在PromptManagement中的全局配置\n'
    }
    
    // 向后端请求某阶段的最终解析示例（以 planner 阶段为例）
    try {
      const arch = editingAgent.value.engine
      const agent_config = {
        prompt_strategy: editingAgent.value.prompt_strategy,
        group_id: editingAgent.value.group_id,
        prompt_ids: editingAgent.value.prompt_ids,
        prompts: editingAgent.value.prompts,
        pinned_versions: editingAgent.value.pinned_versions,
      }
      const resolved = await invoke<string>('preview_resolved_prompt_api', {
        engine: arch,
        stage: 'planner',
        agentConfig: agent_config,
      }).catch(() => '')
      if (resolved) {
        preview += `\n—— 解析示例 (planner) ——\n${resolved.substring(0, 400)}...\n`
      }
    } catch {}

    finalPromptPreview.value = preview
    showPromptPreview.value = true
  } catch (error) {
    console.error('Failed to preview prompt:', error)
    finalPromptPreview.value = '预览失败'
    showPromptPreview.value = true
  }
}

const confirmSaveAgent = async () => {
  if (!editingAgent.value) return
  
  if (!editingAgent.value.name || editingAgent.value.name.trim().length === 0) {
    editingAgent.value.name = 'New Agent'
  }
  
  editingAgent.value.updated_at = new Date().toISOString()
  
  try {
    if (isCreating.value) {
      // 新增
      agents.value.unshift(editingAgent.value)
    } else {
      // 编辑
      const index = agents.value.findIndex(a => a.id === editingAgent.value!.id)
      if (index !== -1) {
        agents.value[index] = editingAgent.value
      }
    }
    
    await saveAgent(editingAgent.value)
    closeEditModal()
  } catch (e) {
    console.warn('Failed to save agent', e)
  }
}
</script>

<style scoped>
.page-content { overflow: hidden; }

/* 折叠面板增强样式 */
.collapse {
  transition: all 0.2s ease;
}

.collapse:hover {
  border-color: oklch(var(--bc) / 0.2);
}

.collapse-title {
  transition: all 0.2s ease;
}

.collapse input[type="checkbox"]:checked ~ .collapse-title {
  background-color: oklch(var(--b2) / 0.7);
}

/* Markdown内容沉浸式样式 */
.markdown-content {
  font-family: 'Inter', 'SF Pro Text', system-ui, sans-serif;
  line-height: 1.7;
  color: oklch(var(--bc) / 0.9);
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  margin: 1.2em 0 0.6em 0;
  font-weight: 600;
  color: oklch(var(--bc));
}

.markdown-content :deep(h1) { font-size: 1.3em; }
.markdown-content :deep(h2) { font-size: 1.2em; }
.markdown-content :deep(h3) { font-size: 1.1em; }

.markdown-content :deep(p) {
  margin: 0.8em 0;
  text-align: justify;
}

.markdown-content :deep(code) {
  background: oklch(var(--b3) / 0.8);
  color: oklch(var(--bc) / 0.95);
  padding: 0.2em 0.4em;
  border-radius: 0.25rem;
  font-size: 0.9em;
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
}

.markdown-content :deep(pre) {
  background: oklch(var(--b3) / 0.6);
  border: 1px solid oklch(var(--bc) / 0.1);
  border-radius: 0.5rem;
  padding: 1rem;
  overflow-x: auto;
  margin: 1em 0;
}

.markdown-content :deep(pre code) {
  background: none;
  padding: 0;
  border-radius: 0;
}

.markdown-content :deep(blockquote) {
  border-left: 3px solid oklch(var(--p) / 0.5);
  background: oklch(var(--b2) / 0.5);
  margin: 1em 0;
  padding: 0.8em 1.2em;
  border-radius: 0 0.5rem 0.5rem 0;
  font-style: italic;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 0.8em 0;
  padding-left: 1.5em;
}

.markdown-content :deep(li) {
  margin: 0.3em 0;
}

.markdown-content :deep(strong) {
  font-weight: 600;
  color: oklch(var(--bc));
}

.markdown-content :deep(em) {
  font-style: italic;
  color: oklch(var(--bc) / 0.8);
}

.markdown-content :deep(a) {
  color: oklch(var(--p));
  text-decoration: none;
  border-bottom: 1px solid oklch(var(--p) / 0.3);
  transition: all 0.2s ease;
}

.markdown-content :deep(a:hover) {
  border-bottom-color: oklch(var(--p));
}

/* 表格样式 */
.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 1em 0;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  border: 1px solid oklch(var(--bc) / 0.2);
  padding: 0.5em 0.8em;
  text-align: left;
}

.markdown-content :deep(th) {
  background: oklch(var(--b2) / 0.7);
  font-weight: 600;
}

.markdown-content :deep(tr:nth-child(even)) {
  background: oklch(var(--b2) / 0.3);
}
</style>

