<template>
  <div class="p-6 space-y-6">
    <!-- 页面标题 -->
    <div class="flex justify-between items-center">
      <div>
        <h1 class="text-3xl font-bold">{{ $t('mcpTools.serversTitle') }}</h1>
        <p class="text-base-content/70 mt-2">{{ $t('mcpTools.serversDescription') }}</p>
      </div>
      <div class="flex gap-3">
        <button @click="showAddServerModal = true" class="btn btn-primary">
          <i class="fas fa-plus mr-2"></i>
          {{ $t('common.add') }}
        </button>
        <button @click="refreshConnections" class="btn btn-outline btn-primary">
          <i class="fas fa-sync-alt mr-2"></i>
          {{ $t('common.refresh') }}
        </button>
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
        @click="activeTab = 'my_servers'"
        :class="['tab', { 'tab-active': activeTab === 'my_servers' }]"
      >
        <i class="fas fa-server mr-2"></i>
        {{ $t('mcpTools.myServers') }}
      </button>
      <button 
        @click="activeTab = 'marketplace'"
        :class="['tab', { 'tab-active': activeTab === 'marketplace' }]"
      >
        <i class="fas fa-store mr-2"></i>
        {{ $t('mcpTools.marketplace') }}
      </button>
    </div>

    <!-- 内置工具列表 -->
    <div v-if="activeTab === 'builtin_tools'" class="space-y-4">
      <div class="alert alert-info">
        <i class="fas fa-info-circle"></i>
        <span>这些是系统内置的MCP工具，已自动注册并可供AI助手调用。</span>
      </div>
      
      <div v-if="isLoadingBuiltinTools" class="text-center p-8">
        <i class="fas fa-spinner fa-spin text-2xl"></i>
        <p class="mt-2">正在加载内置工具...</p>
      </div>
      
      <div v-else-if="builtinTools.length > 0" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
        <div 
          v-for="tool in builtinTools" 
          :key="tool.id"
          class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
        >
          <div class="card-body">
            <div class="flex items-center gap-3">
              <div class="avatar">
                <div class="w-12 h-12 rounded-lg bg-success/10 flex items-center justify-center">
                  <i :class="getToolIcon(tool.name)" class="text-success text-xl"></i>
                </div>
              </div>
              <div class="flex-1">
                <h3 class="card-title text-lg">{{ tool.name }}</h3>
                <span class="badge badge-success badge-sm">{{ tool.category }}</span>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-success toggle-sm" 
                    :checked="tool.enabled !== false"
                    @change="toggleBuiltinTool(tool)"
                    :disabled="tool.is_toggling"
                  />
                </label>
              </div>
            </div>

            <p class="text-sm mt-2 h-16">{{ tool.description }}</p>

            <div class="card-actions justify-between items-center mt-4">
              <span class="text-xs text-base-content/60">v{{ tool.version }}</span>
              <button 
                @click="testBuiltinTool(tool)"
                class="btn btn-success btn-sm"
                :disabled="tool.is_testing"
              >
                <i v-if="tool.is_testing" class="fas fa-spinner fa-spin mr-1"></i>
                <i v-else class="fas fa-play mr-1"></i>
                {{ tool.is_testing ? '测试中...' : '测试工具' }}
              </button>
            </div>
          </div>
        </div>
      </div>
      
      <div v-else class="text-center p-8">
        <i class="fas fa-exclamation-triangle text-4xl text-warning mb-4"></i>
        <p class="text-lg font-semibold">未找到内置工具</p>
        <p class="text-base-content/70">请检查MCP服务是否正常运行</p>
        <button @click="refreshBuiltinTools" class="btn btn-primary mt-4">
          <i class="fas fa-sync-alt mr-2"></i>
          重新加载
        </button>
      </div>
    </div>

    <!-- 我的服务器列表 -->
    <div v-if="activeTab === 'my_servers'" class="space-y-4">
      <div class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-1/12">{{ $t('mcpTools.addServer.enabled') }}</th>
              <th>{{ $t('common.name') }}</th>
              <th>{{ $t('common.type') }}</th>
              <th>{{ $t('common.status') }}</th>
              <th>{{ $t('mcpTools.endpoint') }}</th>
              <th>{{ $t('common.operations') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="connection in mcpConnections" :key="connection.id || connection.name">
              <td>
                <input type="checkbox" class="toggle toggle-sm toggle-success" :checked="connection.status === 'Connected'" @change="toggleServerEnabled(connection)" />
              </td>
              <td>{{ connection.name }}</td>
              <td><span class="badge badge-ghost">{{ connection.transport_type }}</span></td>
              <td>
                <span :class="getStatusBadgeClass(connection.status)">
                  {{ connection.status }}
                </span>
              </td>
              <td class="text-xs font-mono">{{ connection.endpoint }}</td>
              <td>
                <div class="flex gap-1">
                  <button 
                    v-if="connection.id"
                    @click="disconnectMcpServer(connection.id)" 
                    class="btn btn-xs btn-outline btn-error" 
                    :title="$t('common.delete')"
                  >
                    <i class="fas fa-unlink"></i>
                  </button>
                  <button 
                    @click="openDetailsModal(connection)"
                    class="btn btn-xs btn-outline" 
                    :title="$t('common.details')"
                  >
                    <i class="fas fa-info"></i>
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="mcpConnections.length === 0">
              <td colspan="6" class="text-center py-4">{{ $t('mcpTools.noConnections') }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 服务器市场 -->
    <div v-if="activeTab === 'marketplace'">
      <!-- 市场视图切换 -->
      <div class="flex justify-end mb-4">
        <div class="join">
          <button @click="marketplaceView = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': marketplaceView === 'card'}]">
            <i class="fas fa-th-large"></i>
          </button>
          <button @click="marketplaceView = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': marketplaceView === 'list'}]">
            <i class="fas fa-list"></i>
          </button>
        </div>
      </div>

      <!-- 卡片视图 -->
      <div v-if="marketplaceView === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
        <div 
          v-for="server in marketplaceServers" 
          :key="server.name"
          class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
        >
          <div class="card-body">
            <div class="flex items-center gap-3">
              <div class="avatar">
                <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                  <i :class="server.icon || 'fas fa-server'" class="text-primary text-xl"></i>
                </div>
              </div>
              <div class="flex-1">
                <h3 class="card-title text-lg">{{ server.name }}</h3>
              </div>
            </div>

            <p class="text-sm mt-2 h-16">{{ server.description }}</p>

            <div class="card-actions justify-end mt-4">
              <button 
                @click="addMarketplaceServer(server)"
                :disabled="server.is_adding || isServerAdded(server)"
                class="btn btn-primary btn-sm"
              >
                <i v-if="server.is_adding" class="fas fa-spinner fa-spin mr-1"></i>
                <i v-else-if="!isServerAdded(server)" class="fas fa-plus mr-1"></i>
                {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('mcpTools.added') : $t('common.add')) }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 列表视图 -->
      <div v-if="marketplaceView === 'list'" class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-12"></th>
              <th>{{ $t('common.name') }}</th>
              <th>{{ $t('common.description') }}</th>
              <th class="w-40">{{ $t('common.operations') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="server in marketplaceServers" :key="server.name">
              <td>
                 <div class="avatar">
                    <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                      <i :class="server.icon || 'fas fa-server'" class="text-primary text-lg"></i>
                    </div>
                  </div>
              </td>
              <td>{{ server.name }}</td>
              <td>{{ server.description }}</td>
              <td>
                 <button 
                  @click="addMarketplaceServer(server)"
                  :disabled="server.is_adding || isServerAdded(server)"
                  class="btn btn-primary btn-sm"
                >
                  <i v-if="server.is_adding" class="fas fa-spinner fa-spin mr-1"></i>
                  <i v-else-if="!isServerAdded(server)" class="fas fa-plus mr-1"></i>
                  {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('mcpTools.added') : $t('common.add')) }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 服务器详情模态框 -->
    <dialog :class="['modal', { 'modal-open': showDetailsModal }]">
      <div class="modal-box w-11/12 max-w-5xl">
        <div v-if="selectedServer">
          <div class="flex justify-between items-center mb-4">
            <h3 class="font-bold text-lg">{{ $t('mcpTools.serverDetails.title') }}: {{ selectedServer.name }}</h3>
            <button @click="closeDetailsModal" class="btn btn-sm btn-ghost">✕</button>
          </div>

          <div class="tabs tabs-boxed mb-4">
            <button @click="detailsTab = 'general'" :class="['tab', { 'tab-active': detailsTab === 'general' }]">
              <i class="fas fa-cog mr-2"></i>{{ $t('mcpTools.serverDetails.general') }}
            </button>
            <button @click="detailsTab = 'tools'" :class="['tab', { 'tab-active': detailsTab === 'tools' }]">
              <i class="fas fa-tools mr-2"></i>{{ $t('mcpTools.serverDetails.tools') }} ({{ serverTools.length }})
            </button>
          </div>

          <!-- 通用设置 -->
          <div v-if="detailsTab === 'general'" class="space-y-4">
            <div class="form-control">
              <label class="label"><span class="label-text">{{ $t('common.name') }}</span></label>
              <input type="text" v-model="editableServer.name" class="input input-bordered" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text">{{ $t('common.description') }}</span></label>
              <input type="text" v-model="editableServer.description" class="input input-bordered" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.command') }}</span></label>
              <input type="text" v-model="editableServer.command" class="input input-bordered font-mono" />
            </div>
             <div class="form-control">
              <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.args') }}</span></label>
              <textarea v-model="editableServer.args" class="textarea textarea-bordered font-mono" rows="3"></textarea>
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
                  <p class="text-sm text-base-content/60 font-normal">{{ tool.description }}</p>
                </div>
                <div class="collapse-content bg-base-200/50 p-0">
                  <div v-if="tool.input_schema && tool.input_schema.properties" class="overflow-x-auto">
                    <table class="table table-sm w-full">
                      <thead>
                        <tr>
                          <th>{{ $t('mcpTools.serverDetails.paramName') }}</th>
                          <th>{{ $t('mcpTools.serverDetails.paramType') }}</th>
                          <th>{{ $t('mcpTools.serverDetails.paramRequired') }}</th>
                          <th>{{ $t('common.description') }}</th>
                          <th>{{ $t('mcpTools.serverDetails.paramConstraints') }}</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr v-for="prop in getToolProperties(tool.input_schema)" :key="prop.name">
                          <td class="font-mono text-primary">{{ prop.name }}</td>
                          <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                          <td>
                            <span v-if="prop.required" class="badge badge-error badge-sm">{{ $t('common.yes') }}</span>
                          </td>
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
              <p>{{ selectedServer.status === 'Connected' ? $t('mcpTools.serverDetails.noTools') : $t('mcpTools.serverDetails.connectToViewTools') }}</p>
            </div>
          </div>
        </div>
         <div class="modal-action">
          <button @click="closeDetailsModal" class="btn">{{ $t('common.cancel') }}</button>
          <button v-if="detailsTab === 'general'" @click="saveServerDetails" class="btn btn-primary">{{ $t('common.save') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 添加服务器模态框 (保持不变) -->
    <dialog :class="['modal', { 'modal-open': showAddServerModal }]">
      <div class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">{{ $t('mcpTools.addServer.title') }}</h3>
          <button @click="showAddServerModal = false" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="tabs tabs-boxed mb-4">
          <button @click="addServerMode = 'quick'" :class="['tab', { 'tab-active': addServerMode === 'quick' }]">
            <i class="fas fa-magic mr-2"></i>{{ $t('mcpTools.addServer.quickCreate') }}
          </button>
          <button @click="addServerMode = 'json'" :class="['tab', { 'tab-active': addServerMode === 'json' }]">
            <i class="fas fa-file-code mr-2"></i>{{ $t('mcpTools.addServer.importFromJson') }}
          </button>
        </div>

        <!-- 快速创建表单 -->
        <div v-if="addServerMode === 'quick'" class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="flex items-center justify-between">
            <label class="label">{{ $t('mcpTools.addServer.enabled') }}</label>
            <input type="checkbox" class="toggle toggle-success" v-model="quickCreateForm.enabled" />
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('common.name') }}<span class="text-error">*</span></span></label>
            <input type="text" :placeholder="$t('common.name')" class="input input-bordered" v-model="quickCreateForm.name" />
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('common.description') }}</span></label>
            <textarea class="textarea textarea-bordered" :placeholder="$t('common.description')" v-model="quickCreateForm.description"></textarea>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('common.type') }}<span class="text-error">*</span></span></label>
            <select class="select select-bordered" v-model="quickCreateForm.type">
              <option value="stdio">标准输入/输出 (stdio)</option>
              <option value="sse">服务器发送事件 (sse)</option>
              <option value="streamableHttp">可流式HTTP (streamableHttp)</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.params') }}</span></label>
            <textarea class="textarea textarea-bordered font-mono" :placeholder="$t('mcpTools.addServer.paramsPlaceholder')" rows="3" v-model="quickCreateForm.params"></textarea>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.envVars') }}</span></label>
            <textarea class="textarea textarea-bordered font-mono" placeholder="KEY1=value1&#10;KEY2=value2" rows="3" v-model="quickCreateForm.envVars"></textarea>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.timeout') }}</span></label>
            <input type="number" placeholder="60" class="input input-bordered" v-model.number="quickCreateForm.timeout" />
          </div>

          <div class="collapse collapse-arrow border border-base-300 bg-base-100">
            <input type="checkbox" /> 
            <div class="collapse-title text-md font-medium">{{ $t('mcpTools.addServer.advancedSettings') }}</div>
            <div class="collapse-content space-y-4">
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.providerName') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.providerName" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.providerWebsite') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.providerWebsite" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.logoUrl') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.logoUrl" />
              </div>
            </div>
          </div>
        </div>

        <!-- 从JSON导入 -->
        <div v-if="addServerMode === 'json'" class="space-y-4">
           <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('mcpTools.addServer.jsonPaste') }}<span class="text-error">*</span></span></label>
            <textarea v-model="jsonImportConfig" class="textarea textarea-bordered font-mono" rows="15"></textarea>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showAddServerModal = false" class="btn">{{ $t('common.cancel') }}</button>
          <button v-if="addServerMode === 'quick'" @click="handleQuickCreateServer" class="btn btn-primary">{{ $t('common.save') }}</button>
          <button v-if="addServerMode === 'json'" @click="handleImportFromJson" class="btn btn-primary">{{ $t('mcpTools.addServer.import') }}</button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { dialog } from '@/composables/useDialog';

const { t } = useI18n()

// --- 类型定义 ---
interface McpConnection {
  db_id: number;
  id: string | null;
  name: string;
  description: string | null;
  transport_type: string;
  endpoint: string;
  status: string;
  command: string;
  args: string[];
}

interface MarketplaceServer {
  name: string;
  description: string;
  command: string;
  args: string[];
  icon: string;
  is_adding?: boolean;
}

interface FrontendTool {
  name: string;
  description: string;
  input_schema: any;
}

// --- 响应式状态 ---
const activeTab = ref('builtin_tools');
const mcpConnections = ref<McpConnection[]>([]);
const marketplaceServers = ref<MarketplaceServer[]>([]);
const builtinTools = ref<any[]>([]);
const isLoadingBuiltinTools = ref(false);
const showAddServerModal = ref(false);
const addServerMode = ref('quick');
const marketplaceView = ref('card'); // 'card' or 'list'
const showDetailsModal = ref(false);
const detailsTab = ref('general');
const selectedServer = ref<McpConnection | null>(null);
const editableServer = reactive({
  db_id: -1,
  name: '',
  description: '',
  command: '',
  args: '',
  enabled: true,
});
const serverTools = ref<FrontendTool[]>([]);
const isLoadingTools = ref(false);

const quickCreateForm = reactive({
  enabled: true,
  name: '',
  description: '',
  type: 'stdio',
  params: '',
  envVars: '',
  timeout: 60,
  providerName: '',
  providerWebsite: '',
  logoUrl: '',
});

const jsonImportConfig = ref('');

// --- 计算属性 ---
const isServerAdded = (server: MarketplaceServer) => {
  return mcpConnections.value.some(conn => conn.name === server.name);
};

// --- 方法 ---

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Connected': return 'badge badge-sm badge-success';
    case 'Error': return 'badge badge-sm badge-error';
    case 'Disconnected': return 'badge badge-sm badge-warning';
    case 'Connecting': return 'badge badge-sm badge-info';
    default: return 'badge badge-sm';
  }
};

function getToolProperties(schema: any) {
  if (!schema || !schema.properties) {
    return [];
  }

  const requiredParams = new Set(schema.required || []);
  const properties = [];

  for (const name in schema.properties) {
    const details = schema.properties[name];
    const constraints = [];
    if (details.minimum !== undefined) {
      constraints.push(`min: ${details.minimum}`);
    }
    if (details.maximum !== undefined) {
      constraints.push(`max: ${details.maximum}`);
    }

    properties.push({
      name: name,
      type: details.type,
      required: requiredParams.has(name),
      description: details.description || '',
      constraints: constraints.join(', '),
    });
  }
  return properties;
}

async function openDetailsModal(connection: McpConnection) {
  selectedServer.value = connection;
  Object.assign(editableServer, {
    ...connection,
    args: connection.args.join(' '), // 将数组转为空格分隔的字符串以便编辑
    description: connection.description || '',
  });
  detailsTab.value = 'general';
  showDetailsModal.value = true;
  
  if (connection.status === 'Connected' && connection.id) {
    fetchServerTools();
  } else {
    serverTools.value = [];
  }
}

function closeDetailsModal() {
  showDetailsModal.value = false;
  selectedServer.value = null;
}

async function fetchServerTools() {
  if (!selectedServer.value?.id) {
    serverTools.value = [];
    return;
  }
  isLoadingTools.value = true;
  try {
    serverTools.value = await invoke('mcp_get_connection_tools', { connectionId: selectedServer.value.id });
  } catch (error) {
    console.error('Failed to fetch server tools:', error);
    serverTools.value = [];
  } finally {
    isLoadingTools.value = false;
  }
}

async function saveServerDetails() {
  if (!selectedServer.value) return;
  try {
    const payload = {
      ...editableServer,
      args: editableServer.args.split(' ').filter(s => s.trim() !== ''), // 将字符串转回数组
    };
    await invoke('mcp_update_server_config', { payload });
    dialog.toast.success(t('mcpTools.updateSuccess'));
    closeDetailsModal();
    await fetchConnections();
  } catch (error) {
    console.error("Failed to save server details:", error);
    dialog.toast.error(`${t('mcpTools.updateFailed')}: ${error}`);
  }
}

async function fetchConnections() {
  try {
    mcpConnections.value = await invoke('mcp_get_connections');
  } catch (error) {
    console.error('Failed to fetch MCP connections:', error);
    mcpConnections.value = [];
  }
}

async function refreshConnections() {
  await fetchConnections();
}

function getToolIcon(toolName: string) {
  switch (toolName) {
    case 'subdomain_scanner':
      return 'fas fa-sitemap';
    case 'port_scanner':
      return 'fas fa-network-wired';
    default:
      return 'fas fa-tools';
  }
}

async function testBuiltinTool(tool: any) {
  tool.is_testing = true;
  try {
    // 调用后端真实测试接口，传递工具名称而不是ID
    const result = await invoke('test_mcp_tools_registration', { toolId: tool.name }) as any;
    dialog.toast.success(`工具 ${tool.name} 测试成功：${result && result.message ? result.message : '已完成'}`);
  } catch (error: any) {
    console.error(`Failed to test tool ${tool.name}:`, error);
    dialog.toast.error(`工具 ${tool.name} 测试失败：${error && error.message ? error.message : error}`);
  } finally {
    tool.is_testing = false;
  }
}

async function toggleBuiltinTool(tool: any) {
  tool.is_toggling = true;
  try {
    const newState = tool.enabled === false;
    await invoke('toggle_builtin_tool', { toolName: tool.name, enabled: newState });
    tool.enabled = newState;
    dialog.toast.success(`工具 ${tool.name} 已${newState ? '启用' : '禁用'}`);
  } catch (error: any) {
    console.error(`Failed to toggle tool ${tool.name}:`, error);
    dialog.toast.error(`切换工具 ${tool.name} 状态失败：${error && error.message ? error.message : error}`);
  } finally {
    tool.is_toggling = false;
  }
}

async function fetchBuiltinTools() {
  isLoadingBuiltinTools.value = true;
  try {
    const tools: any[] = await invoke('get_builtin_tools_with_status');
    console.log('get_builtin_tools_with_status 返回:', tools); // 临时调试输出
    builtinTools.value = tools;
  } catch (error) {
    console.error('Failed to fetch builtin tools:', error);
    builtinTools.value = [];
  } finally {
    isLoadingBuiltinTools.value = false;
  }
}

async function refreshBuiltinTools() {
  await fetchBuiltinTools();
}

async function addMarketplaceServer(server: MarketplaceServer) {
  server.is_adding = true;
  try {
    const { command, args, name } = server;
    await invoke('add_child_process_mcp_server', { name, command, args });
    await fetchConnections();
  } catch (error) {
    console.error(`Failed to add marketplace server ${server.name}:`, error);
    // 可选: 显示错误通知
  } finally {
    server.is_adding = false;
  }
}

async function toggleServerEnabled(connection: McpConnection) {
  try {
    if (connection.status === 'Connected' && connection.id) {
      await invoke('mcp_disconnect_server', { connectionId: connection.id });
    } else {
      await invoke('add_child_process_mcp_server', { 
        name: connection.name, 
        command: connection.command, 
        args: connection.args 
      });
    }
    await fetchConnections();
  } catch (error) {
    console.error(`Failed to toggle server ${connection.name} state:`, error);
  }
}

async function disconnectMcpServer(id: string) {
  if (!id) return;
  try {
    await invoke('mcp_disconnect_server', { connectionId: id });
    await fetchConnections();
  } catch (error) {
    console.error('Failed to disconnect MCP server:', error);
  }
}

async function handleQuickCreateServer() {
  if (!quickCreateForm.name) {
    await dialog.error(t('mcpTools.addServer.nameRequired'));
    return;
  }
  try {
    await invoke('quick_create_mcp_server', { config: quickCreateForm });
    showAddServerModal.value = false;
    Object.assign(quickCreateForm, { enabled: true, name: '', description: '', type: 'stdio', params: '', envVars: '', timeout: 60, providerName: '', providerWebsite: '', logoUrl: '' });
    await fetchConnections();
  } catch (error) {
    console.error("快速创建服务器失败:", error);
    await dialog.error(`${t('mcpTools.addServerFailed')}: ${error}`);
  }
}

async function handleImportFromJson() {
  if (!jsonImportConfig.value.trim()) {
    await dialog.error(t('mcpTools.addServer.jsonRequired'));
    return;
  }
  try {
    await invoke('import_mcp_servers_from_json', { jsonConfig: jsonImportConfig.value });
    dialog.toast.success(t('mcpTools.importSuccess'));
    showAddServerModal.value = false;
    await fetchConnections();
  } catch (error) {
    console.error("从JSON导入服务器失败:", error);
    dialog.toast.error(`${t('mcpTools.importFailed')}: ${error}`);
  }
}

// --- 生命周期钩子 ---
onMounted(() => {
  fetchConnections();
  fetchBuiltinTools();
  // 移除内置服务器列表
  marketplaceServers.value = [];
});
</script>