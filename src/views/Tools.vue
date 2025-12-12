<template>
  <div class="page-content-padded safe-top space-y-6">
    <!-- 页面标题 -->
    <div class="flex justify-between items-center">
      <div>
        <h1 class="text-3xl font-bold">{{ $t('Tools.serversTitle') }}</h1>
        <p class="text-base-content/70 mt-2">{{ $t('Tools.serversDescription') }}</p>
      </div>
      <div class="flex gap-3">
        <button @click="showAddServerModal = true" class="btn btn-primary">
          <i class="fas fa-plus mr-2"></i>
          {{ $t('common.add') }}
        </button>
        <button @click="refreshAll" class="btn btn-outline btn-primary">
          <i class="fas fa-sync-alt mr-2"></i>
          {{ $t('common.refresh') }}
        </button>
        <div class="dropdown dropdown-end">
          <button tabindex="0" class="btn btn-outline btn-secondary">
            <i class="fas fa-cog mr-2"></i>
            管理
          </button>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
            <li><a @click="cleanupDuplicateServers"><i class="fas fa-broom mr-2"></i>清理重复服务器</a></li>
          </ul>
        </div>
      </div>
    </div>

    <!-- 选项卡 -->
    <div class="tabs tabs-boxed bg-base-200">
      <button 
        @click="activeTab = 'builtin_tools'"
        :class="['tab', { 'tab-active': activeTab === 'builtin_tools' }]"
      >
        <i class="fas fa-tools mr-2"></i>
        内置工具
      </button>
      <button 
        @click="activeTab = 'workflow_tools'"
        :class="['tab', { 'tab-active': activeTab === 'workflow_tools' }]"
      >
        <i class="fas fa-project-diagram mr-2"></i>
        工作流工具
      </button>
      <button 
        @click="activeTab = 'plugin_tools'"
        :class="['tab', { 'tab-active': activeTab === 'plugin_tools' }]"
      >
        <i class="fas fa-plug mr-2"></i>
        插件工具
      </button>
      <button 
        @click="activeTab = 'my_servers'"
        :class="['tab', { 'tab-active': activeTab === 'my_servers' }]"
      >
        <i class="fas fa-server mr-2"></i>
        {{ $t('Tools.mcpServers') }}
      </button>
      <button 
        @click="activeTab = 'marketplace'"
        :class="['tab', { 'tab-active': activeTab === 'marketplace' }]"
      >
        <i class="fas fa-store mr-2"></i>
        {{ $t('Tools.marketplace') }}
      </button>

    </div>

    <!-- Tab 内容 -->
    <BuiltinToolsTab v-if="activeTab === 'builtin_tools'" ref="builtinToolsRef" />
    <WorkflowToolsTab v-if="activeTab === 'workflow_tools'" ref="workflowToolsRef" />
    <McpServersTab 
      v-if="activeTab === 'my_servers'" 
      ref="mcpServersRef"
      @show-details="openDetailsModal"
      @test-server="openTestServerModal"
    />
    <MarketplaceTab 
      v-if="activeTab === 'marketplace'" 
      ref="marketplaceRef"
      :added-server-names="addedServerNames"
    />
    <PluginToolsTab 
      v-if="activeTab === 'plugin_tools'" 
      ref="pluginToolsRef"
      @show-upload="showUploadPluginModal = true"
    />

    <!-- 服务器详情模态框 -->
    <dialog :class="['modal', { 'modal-open': showDetailsModal }]">
      <div v-if="showDetailsModal" class="modal-box w-11/12 max-w-5xl">
        <div v-if="selectedServer">
          <div class="flex justify-between items-center mb-4">
            <h3 class="font-bold text-lg">{{ $t('Tools.serverDetails.title') }}: {{ selectedServer.name }}</h3>
            <button @click="closeDetailsModal" class="btn btn-sm btn-ghost">✕</button>
          </div>

          <div class="tabs tabs-boxed mb-4">
            <button @click="detailsTab = 'general'" :class="['tab', { 'tab-active': detailsTab === 'general' }]">
              <i class="fas fa-cog mr-2"></i>{{ $t('Tools.serverDetails.general') }}
            </button>
            <button @click="detailsTab = 'tools'" :class="['tab', { 'tab-active': detailsTab === 'tools' }]">
              <i class="fas fa-tools mr-2"></i>{{ $t('Tools.serverDetails.tools') }} ({{ serverTools.length }})
            </button>
          </div>

          <!-- 通用设置 -->
          <div v-if="detailsTab === 'general'" class="space-y-4">
            <div class="flex justify-end">
              <div class="join">
                <button @click="editMode = 'form'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': editMode === 'form'}]">
                  <i class="fas fa-edit mr-1"></i>表单编辑
                </button>
                <button @click="editMode = 'json'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': editMode === 'json'}]">
                  <i class="fas fa-code mr-1"></i>JSON 编辑
                </button>
              </div>
            </div>

            <div v-if="editMode === 'form'" class="space-y-4">
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('common.name') }}</span></label>
                <input type="text" v-model="editableServer.name" class="input input-bordered" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('common.description') }}</span></label>
                <input type="text" v-model="editableServer.description" class="input input-bordered" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('common.type') }}</span></label>
                <select class="select select-bordered" v-model="editableServer.transport_type">
                  <option value="stdio">标准输入/输出 (stdio)</option>
                  <option value="sse">服务器发送事件 (sse)</option>
                  <option value="streamableHttp">可流式HTTP (streamableHttp)</option>
                </select>
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.command') }}</span></label>
                <input type="text" v-model="editableServer.command" class="input input-bordered font-mono" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.args') }}</span></label>
                <textarea v-model="editableServer.args" class="textarea textarea-bordered font-mono" rows="3"></textarea>
              </div>
            </div>

            <div v-if="editMode === 'json'" class="space-y-4">
              <div class="alert alert-warning">
                <i class="fas fa-exclamation-triangle"></i>
                <span>直接编辑 JSON 配置，请确保格式正确</span>
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">服务器配置 (JSON)</span></label>
                <textarea v-model="editableServerJson" class="textarea textarea-bordered font-mono text-sm" rows="15" spellcheck="false"></textarea>
              </div>
            </div>
          </div>

          <!-- 工具列表 -->
          <div v-if="detailsTab === 'tools'">
            <div v-if="isLoadingTools" class="text-center p-8">
              <i class="fas fa-spinner fa-spin text-2xl"></i>
            </div>
            <div v-else-if="serverTools.length > 0" class="space-y-2 max-h-[60vh] overflow-y-auto">
              <div v-for="tool in serverTools" :key="tool.name" class="collapse collapse-arrow border border-base-300 bg-base-100">
                <input type="checkbox" /> 
                <div class="collapse-title text-md font-medium">
                  {{ tool.name }}
                  <p v-if="tool.description" class="text-sm text-base-content/60 font-normal">{{ tool.description }}</p>
                </div>
                <div class="collapse-content bg-base-200/50 p-0">
                  <div v-if="tool.input_schema && tool.input_schema.properties" class="overflow-x-auto">
                    <table class="table table-sm w-full">
                      <thead>
                        <tr>
                          <th>{{ $t('Tools.serverDetails.paramName') }}</th>
                          <th>{{ $t('Tools.serverDetails.paramType') }}</th>
                          <th>{{ $t('Tools.serverDetails.paramRequired') }}</th>
                          <th>{{ $t('common.description') }}</th>
                          <th>{{ $t('Tools.serverDetails.paramConstraints') }}</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr v-for="prop in getToolProperties(tool.input_schema)" :key="prop.name">
                          <td class="font-mono text-primary">{{ prop.name }}</td>
                          <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                          <td><span v-if="prop.required" class="badge badge-error badge-sm">{{ $t('common.yes') }}</span></td>
                          <td>{{ prop.description }}</td>
                          <td class="font-mono text-xs">{{ prop.constraints }}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                  <pre v-else class="text-xs p-4 rounded-md bg-black/50 text-white font-mono whitespace-pre-wrap"><code>{{ JSON.stringify(tool.input_schema, null, 2) }}</code></pre>
                </div>
              </div>
            </div>
            <div v-else class="text-center p-8">
              <p>{{ selectedServer.status === 'Connected' ? $t('Tools.serverDetails.noTools') : $t('Tools.serverDetails.connectToViewTools') }}</p>
            </div>
          </div>
        </div>
        <div class="modal-action">
          <button @click="closeDetailsModal" class="btn">{{ $t('common.cancel') }}</button>
          <button v-if="detailsTab === 'general'" @click="saveServerDetails" class="btn btn-primary">{{ $t('common.save') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 添加服务器模态框 -->
    <dialog :class="['modal', { 'modal-open': showAddServerModal }]">
      <div v-if="showAddServerModal" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">{{ $t('Tools.addServer.title') }}</h3>
          <button @click="showAddServerModal = false" class="btn btn-sm btn-ghost">✕</button>
        </div>



        <div class="space-y-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('Tools.addServer.jsonPaste') }}<span class="text-error">*</span></span></label>
            <textarea v-model="jsonImportConfig" class="textarea textarea-bordered font-mono" rows="15" placeholder='{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/path/to/directory"
      ]
    }
  }
}'></textarea>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showAddServerModal = false" class="btn">{{ $t('common.cancel') }}</button>
          <button @click="handleImportFromJson" class="btn btn-primary">{{ $t('Tools.addServer.import') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 服务器工具测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showTestServerModal }]">
      <div v-if="showTestServerModal" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">测试服务器工具: {{ testingServer?.name }}</h3>
          <button @click="closeTestServerModal" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div v-if="isLoadingTestTools" class="text-center p-8">
          <i class="fas fa-spinner fa-spin text-2xl"></i>
          <p class="mt-2">正在加载服务器工具列表...</p>
        </div>

        <div v-else class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>选择一个工具进行测试，可以使用默认参数或自定义参数。</span>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">选择工具</span></label>
            <select v-model="selectedTestToolName" class="select select-bordered">
              <option v-for="tool in testServerTools" :key="tool.name" :value="tool.name">
                {{ tool.name }}{{ tool.description ? ' - ' + tool.description : '' }}
              </option>
            </select>
          </div>

          <div v-if="selectedTestTool" class="space-y-3">
            <div class="collapse collapse-arrow border border-base-300 bg-base-100">
              <input type="checkbox" />
              <div class="collapse-title text-md font-medium">输入参数说明</div>
              <div class="collapse-content">
                <div v-if="selectedTestTool.input_schema && selectedTestTool.input_schema.properties" class="overflow-x-auto">
                  <table class="table table-sm w-full">
                    <thead>
                      <tr>
                        <th>参数名</th>
                        <th>类型</th>
                        <th>必填</th>
                        <th>描述</th>
                        <th>约束</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="prop in getToolProperties(selectedTestTool.input_schema)" :key="prop.name">
                        <td class="font-mono text-primary">{{ prop.name }}</td>
                        <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                        <td><span v-if="prop.required" class="badge badge-error badge-sm">必填</span></td>
                        <td>{{ prop.description }}</td>
                        <td class="font-mono text-xs">{{ prop.constraints }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <pre v-else class="text-xs p-4 rounded-md bg-black/50 text-white font-mono whitespace-pre-wrap"><code>{{ JSON.stringify(selectedTestTool.input_schema, null, 2) }}</code></pre>
              </div>
            </div>

            <div class="form-control">
              <label class="label"><span class="label-text">测试参数 (JSON，可选)</span></label>
              <textarea v-model="testToolParamsJson" class="textarea textarea-bordered font-mono text-sm" placeholder="留空使用默认参数，或输入 JSON 对象覆盖默认参数" rows="6" spellcheck="false"></textarea>
            </div>

            <div class="form-control">
              <label class="label"><span class="label-text">测试结果</span></label>
              <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap min-h-40 max-h-60 overflow-auto bg-base-200">{{ testToolResult }}</pre>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeTestServerModal" class="btn">{{ $t('common.cancel') }}</button>
          <button class="btn btn-primary" :disabled="!selectedTestToolName || isTestingTool" @click="runTestTool">
            <i v-if="isTestingTool" class="fas fa-spinner fa-spin mr-1"></i>
            <i v-else class="fas fa-play mr-1"></i>
            运行测试
          </button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, emit } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'

// 导入子组件
import BuiltinToolsTab from '@/components/Tools/BuiltinToolsTab.vue'
import WorkflowToolsTab from '@/components/Tools/WorkflowToolsTab.vue'
import MarketplaceTab from '@/components/Tools/MarketplaceTab.vue'
import PluginToolsTab from '@/components/Tools/PluginToolsTab.vue'
import McpServersTab from '@/components/Tools/McpServersTab.vue'


const { t } = useI18n()

// 类型定义
interface McpConnection {
  db_id: string
  id: string | null
  name: string
  description: string | null
  transport_type: string
  endpoint: string
  status: string
  command: string
  args: string[]
}

interface FrontendTool {
  name: string
  description: string | null
  input_schema: any
}

// 子组件引用
const builtinToolsRef = ref<InstanceType<typeof BuiltinToolsTab> | null>(null)
const workflowToolsRef = ref<InstanceType<typeof WorkflowToolsTab> | null>(null)
const mcpServersRef = ref<InstanceType<typeof McpServersTab> | null>(null)
const marketplaceRef = ref<InstanceType<typeof MarketplaceTab> | null>(null)
const pluginToolsRef = ref<InstanceType<typeof PluginToolsTab> | null>(null)

// 状态
const activeTab = ref('builtin_tools')
const showAddServerModal = ref(false)
const showUploadPluginModal = ref(false)

// 服务器详情模态框
const showDetailsModal = ref(false)
const detailsTab = ref('general')
const editMode = ref('form')
const selectedServer = ref<McpConnection | null>(null)
const editableServer = reactive({ db_id: '', name: '', description: '', command: '', args: '', enabled: true, transport_type: 'stdio' })
const editableServerJson = ref('')
const serverTools = ref<FrontendTool[]>([])
const isLoadingTools = ref(false)

// 服务器工具测试模态框
const showTestServerModal = ref(false)
const testingServer = ref<McpConnection | null>(null)
const testServerTools = ref<FrontendTool[]>([])
const isLoadingTestTools = ref(false)
const selectedTestToolName = ref('')
const testToolParamsJson = ref('')
const testToolResult = ref('')
const isTestingTool = ref(false)

// 添加服务器表单
// 添加服务器表单
const jsonImportConfig = ref('')

// 计算属性
const addedServerNames = computed(() => {
  return mcpServersRef.value?.connections?.map(c => c.name) || []
})

const selectedTestTool = computed(() => {
  if (!selectedTestToolName.value) return null
  return testServerTools.value.find(t => t.name === selectedTestToolName.value) || null
})

// 监听添加服务器模态框打开，设置默认 JSON 模板
watch(showAddServerModal, (isOpen) => {
  if (isOpen && !jsonImportConfig.value) {
    jsonImportConfig.value = `{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/Users/username/Desktop"
      ]
    }
  }
}`
  }
})

// 监听工具选择变化
watch(selectedTestTool, (newTool) => {
  if (newTool && newTool.input_schema) {
    testToolParamsJson.value = generateDefaultParams(newTool.input_schema)
  } else {
    testToolParamsJson.value = '{}'
  }
})

// 方法
function generateDefaultParams(schema: any): string {
  if (!schema || !schema.properties) return '{}'
  const params: any = {}
  for (const name in schema.properties) {
    const prop = schema.properties[name]
    if (prop.default !== undefined) params[name] = prop.default
    else {
      switch (prop.type) {
        case 'string': params[name] = ''; break
        case 'number':
        case 'integer': params[name] = prop.minimum ?? 0; break
        case 'boolean': params[name] = false; break
        case 'array': params[name] = []; break
        case 'object': params[name] = {}; break
        default: params[name] = null
      }
    }
  }
  return JSON.stringify(params, null, 2)
}

function getToolProperties(schema: any) {
  if (!schema || !schema.properties) return []
  const requiredParams = new Set(schema.required || [])
  const properties = []
  for (const name in schema.properties) {
    const details = schema.properties[name]
    const constraints = []
    if (details.minimum !== undefined) constraints.push(`min: ${details.minimum}`)
    if (details.maximum !== undefined) constraints.push(`max: ${details.maximum}`)
    properties.push({ name, type: details.type, required: requiredParams.has(name), description: details.description || '', constraints: constraints.join(', ') })
  }
  return properties
}

async function refreshAll() {
  builtinToolsRef.value?.refresh?.()
  workflowToolsRef.value?.refresh?.()
  mcpServersRef.value?.refresh?.()
  pluginToolsRef.value?.refresh?.()
}

// 服务器详情模态框
function openDetailsModal(connection: McpConnection) {
  selectedServer.value = { ...connection }
  Object.assign(editableServer, {
    db_id: connection.db_id,
    name: connection.name,
    description: connection.description || '',
    command: connection.command,
    args: connection.args.join(' '),
    enabled: true,
    transport_type: connection.transport_type
  })
  editableServerJson.value = JSON.stringify({
    db_id: connection.db_id, id: connection.id, name: connection.name,
    description: connection.description || '', transport_type: connection.transport_type,
    endpoint: connection.endpoint, status: connection.status, command: connection.command, args: connection.args,
  }, null, 2)
  detailsTab.value = 'general'
  editMode.value = 'form'
  serverTools.value = []
  showDetailsModal.value = true
  if (connection.status === 'Connected' && connection.id) {
    fetchServerTools()
  }
}

function closeDetailsModal() {
  showDetailsModal.value = false
  setTimeout(() => { selectedServer.value = null; editMode.value = 'form'; serverTools.value = [] }, 300)
}

async function fetchServerTools() {
  if (!selectedServer.value?.id) { serverTools.value = []; return }
  isLoadingTools.value = true
  try { serverTools.value = await invoke('mcp_get_connection_tools', { connectionId: selectedServer.value.id }) }
  catch (error) { console.error('Failed to fetch server tools:', error); serverTools.value = [] }
  finally { isLoadingTools.value = false }
}

async function saveServerDetails() {
  if (!selectedServer.value) return
  try {
    let payload
    if (editMode.value === 'json') {
      try {
        const jsonData = JSON.parse(editableServerJson.value)
        payload = {
          db_id: jsonData.db_id, id: jsonData.id || null, name: jsonData.name, description: jsonData.description || '',
          command: jsonData.command, args: Array.isArray(jsonData.args) ? jsonData.args : [],
          transport_type: jsonData.transport_type || 'stdio', endpoint: jsonData.endpoint || '',
          status: jsonData.status || selectedServer.value.status || 'Disconnected',
        }
      } catch (e) { dialog.toast.error('JSON 格式错误，请检查语法'); return }
    } else {
      payload = {
        db_id: editableServer.db_id, id: selectedServer.value.id, name: editableServer.name,
        description: editableServer.description || '', command: editableServer.command,
        args: editableServer.args.split(' ').filter(s => s.trim() !== ''),
        transport_type: editableServer.transport_type, endpoint: selectedServer.value.endpoint, status: selectedServer.value.status,
      }
    }
    await invoke('mcp_update_server_config', { payload })
    const wasConnected = selectedServer.value.status === 'Connected' && selectedServer.value.id
    if (wasConnected) {
      try {
        await invoke('mcp_disconnect_server', { connectionId: selectedServer.value.id })
        await new Promise(resolve => setTimeout(resolve, 500))
        await invoke('add_child_process_mcp_server', { name: payload.name, command: payload.command, args: payload.args })
        dialog.toast.success(t('Tools.updateSuccess') + '，服务器已重新连接')
      } catch (reconnectError) {
        console.error('Failed to reconnect server:', reconnectError)
        dialog.toast.warning(t('Tools.updateSuccess') + '，但重新连接失败，请手动重连')
      }
    } else { dialog.toast.success(t('Tools.updateSuccess')) }
    closeDetailsModal()
    mcpServersRef.value?.fetchConnections?.()
  } catch (error) { console.error("Failed to save server details:", error); dialog.toast.error(`${t('Tools.updateFailed')}: ${error}`) }
}

// 服务器工具测试模态框
function openTestServerModal(connection: McpConnection) {
  testingServer.value = { ...connection }
  showTestServerModal.value = true
  testServerTools.value = []
  selectedTestToolName.value = ''
  testToolParamsJson.value = ''
  testToolResult.value = ''
  if (!connection.id) { dialog.toast.error('当前服务器未处于连接状态，无法测试工具'); return }
  void (async () => {
    isLoadingTestTools.value = true
    try {
      const tools = await invoke<FrontendTool[]>('mcp_get_connection_tools', { connectionId: connection.id })
      testServerTools.value = tools || []
      if (testServerTools.value.length > 0) {
        selectedTestToolName.value = testServerTools.value[0].name
        if (testServerTools.value[0].input_schema) testToolParamsJson.value = generateDefaultParams(testServerTools.value[0].input_schema)
      }
    } catch (error) { console.error('Failed to fetch tools for testing:', error); dialog.toast.error('加载服务器工具列表失败') }
    finally { isLoadingTestTools.value = false }
  })()
}

function closeTestServerModal() {
  showTestServerModal.value = false
  setTimeout(() => { testingServer.value = null; testServerTools.value = []; selectedTestToolName.value = ''; testToolParamsJson.value = ''; testToolResult.value = '' }, 300)
}

async function runTestTool() {
  if (!testingServer.value || !testingServer.value.id || !selectedTestToolName.value) { dialog.toast.error('请选择要测试的工具'); return }
  let args: any = {}
  if (testToolParamsJson.value.trim()) {
    try { args = JSON.parse(testToolParamsJson.value) }
    catch (e) { dialog.toast.error('参数 JSON 格式错误，请检查'); return }
  }
  isTestingTool.value = true
  testToolResult.value = '正在执行测试...'
  try {
    const result = await invoke<any>('mcp_test_server_tool', { connectionId: testingServer.value.id, toolName: selectedTestToolName.value, args })
    testToolResult.value = typeof result === 'string' ? result : JSON.stringify(result, null, 2)
    dialog.toast.success('工具测试完成')
  } catch (error: any) {
    console.error('Failed to test server tool:', error)
    testToolResult.value = `测试失败: ${error?.message || String(error)}`
    dialog.toast.error('工具测试失败')
  } finally { isTestingTool.value = false }
}



async function handleImportFromJson() {
  if (!jsonImportConfig.value.trim()) { await dialog.error(t('Tools.addServer.jsonRequired')); return }
  try {
    await invoke('import_mcp_servers_from_json', { jsonConfig: jsonImportConfig.value })
    dialog.toast.success(t('Tools.importSuccess'))
    showAddServerModal.value = false
    mcpServersRef.value?.fetchConnections?.()
    await emit('mcp:tools-changed', { action: 'servers_imported' })
  } catch (error) { console.error("从JSON导入服务器失败:", error); dialog.toast.error(`${t('Tools.importFailed')}: ${error}`) }
}

async function cleanupDuplicateServers() {
  try {
    const confirmed = await dialog.confirm('确定要清理重复的MCP服务器配置吗？这将删除重复的配置，只保留最新的。')
    if (!confirmed) return
    const removedDuplicates: string[] = await invoke('cleanup_duplicate_mcp_servers')
    if (removedDuplicates.length > 0) { dialog.toast.success(`已清理 ${removedDuplicates.length} 个重复配置`) }
    else { dialog.toast.info('没有发现重复的服务器配置') }
    mcpServersRef.value?.fetchConnections?.()
  } catch (error) { console.error('清理重复服务器失败:', error); dialog.toast.error(`清理失败: ${error}`) }
}

// 生命周期
onMounted(async () => {
  listen('plugin:changed', async () => { pluginToolsRef.value?.refresh?.() })
  listen('mcp:tools-changed', async (event) => {
    console.log('MCP tools changed event received:', event.payload)
    builtinToolsRef.value?.refresh?.()
    mcpServersRef.value?.fetchConnections?.()
  })
  listen('workflow:changed', async () => { workflowToolsRef.value?.refresh?.() })
})
</script>

<style scoped>
.modal { transition: opacity 0.2s ease-in-out; }
.modal-box { transition: transform 0.2s ease-in-out, opacity 0.2s ease-in-out; }
.modal-open .modal-box { animation: modalSlideIn 0.2s ease-out; }
@keyframes modalSlideIn {
  from { opacity: 0; transform: translateY(-20px); }
  to { opacity: 1; transform: translateY(0); }
}
.space-y-4 > * { transition: opacity 0.15s ease-in-out; }
</style>
