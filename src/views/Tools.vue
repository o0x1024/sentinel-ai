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
        <button @click="refreshConnections" class="btn btn-outline btn-primary">
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
        @click="activeTab = 'my_servers'"
        :class="['tab', { 'tab-active': activeTab === 'my_servers' }]"
      >
        <i class="fas fa-server mr-2"></i>
        {{ $t('Tools.myServers') }}
      </button>
      <button 
        @click="activeTab = 'marketplace'"
        :class="['tab', { 'tab-active': activeTab === 'marketplace' }]"
      >
        <i class="fas fa-store mr-2"></i>
        {{ $t('Tools.marketplace') }}
      </button>
      <button 
        @click="activeTab = 'plugin_tools'"
        :class="['tab', { 'tab-active': activeTab === 'plugin_tools' }]"
      >
        <i class="fas fa-plug mr-2"></i>
        插件工具
      </button>
    </div>

    <!-- 内置工具列表 -->
    <div v-if="activeTab === 'builtin_tools'" class="space-y-4">
      <div class="flex justify-between items-center">
        <div class="alert alert-info flex-1 mr-4">
          <i class="fas fa-info-circle"></i>
          <span>这些是系统内置的MCP工具，已自动注册并可供AI助手调用。</span>
        </div>
        <div class="join">
          <button @click="builtinToolsView = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': builtinToolsView === 'card'}]">
            <i class="fas fa-th-large"></i>
          </button>
          <button @click="builtinToolsView = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': builtinToolsView === 'list'}]">
            <i class="fas fa-list"></i>
          </button>
        </div>
      </div>
      
      <div v-if="isLoadingBuiltinTools" class="text-center p-8">
        <i class="fas fa-spinner fa-spin text-2xl"></i>
        <p class="mt-2">正在加载内置工具...</p>
      </div>
      
      <!-- 卡片视图 -->
      <div v-else-if="builtinTools.length > 0 && builtinToolsView === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
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
              <div class="flex gap-2">
                <button 
                  @click="testBuiltinTool(tool)"
                  class="btn btn-success btn-sm"
                  :disabled="tool.is_testing"
                  title="快速测试（使用默认参数）"
                >
                  <i v-if="tool.is_testing" class="fas fa-spinner fa-spin mr-1"></i>
                  <i v-else class="fas fa-play mr-1"></i>
                  测试
                </button>
                <button 
                  @click="openTestBuiltinToolModal(tool)"
                  class="btn btn-outline btn-info btn-sm"
                  title="高级测试（自定义参数）"
                >
                  <i class="fas fa-cog"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 列表视图 -->
      <div v-else-if="builtinTools.length > 0 && builtinToolsView === 'list'" class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-1/12">启用</th>
              <th>名称</th>
              <th>分类</th>
              <th>描述</th>
              <th>版本</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="tool in builtinTools" :key="tool.id">
              <td>
                <input 
                  type="checkbox" 
                  class="toggle toggle-success toggle-sm" 
                  :checked="tool.enabled !== false"
                  @change="toggleBuiltinTool(tool)"
                  :disabled="tool.is_toggling"
                />
              </td>
              <td>
                <div class="flex items-center gap-2">
                  <i :class="getToolIcon(tool.name)" class="text-success"></i>
                  <span class="font-semibold">{{ tool.name }}</span>
                </div>
              </td>
              <td><span class="badge badge-success badge-sm">{{ tool.category }}</span></td>
              <td class="text-sm">{{ tool.description }}</td>
              <td class="text-xs text-base-content/60">v{{ tool.version }}</td>
              <td>
                <div class="flex gap-1">
                  <button 
                    @click="testBuiltinTool(tool)"
                    class="btn btn-success btn-xs"
                    :disabled="tool.is_testing"
                    title="快速测试（使用默认参数）"
                  >
                    <i v-if="tool.is_testing" class="fas fa-spinner fa-spin mr-1"></i>
                    <i v-else class="fas fa-play"></i>
                  </button>
                  <button 
                    @click="openTestBuiltinToolModal(tool)"
                    class="btn btn-outline btn-info btn-xs"
                    title="高级测试（自定义参数）"
                  >
                    <i class="fas fa-cog"></i>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
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
      <div class="flex justify-end mb-4">
        <div class="join">
          <button @click="myServersView = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': myServersView === 'card'}]">
            <i class="fas fa-th-large"></i>
          </button>
          <button @click="myServersView = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': myServersView === 'list'}]">
            <i class="fas fa-list"></i>
          </button>
        </div>
      </div>
      
      <!-- 列表视图 -->
      <div v-if="myServersView === 'list'" class="overflow-x-auto">
        <table class="table w-full">
          <thead>
            <tr>
              <th class="w-1/12">{{ $t('Tools.addServer.enabled') }}</th>
              <th>{{ $t('common.name') }}</th>
              <th>{{ $t('common.type') }}</th>
              <th>{{ $t('common.status') }}</th>
              <th>{{ $t('Tools.endpoint') }}</th>
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
                <span :class="getStatusBadgeClass(connection.status)" class="flex items-center gap-1">
                  <i :class="getStatusIcon(connection.status)"></i>
                  {{ connection.status }}
                </span>
              </td>
              <td class="text-xs font-mono">{{ connection.endpoint }}</td>
              <td>
                <div class="flex gap-1">
                  <button 
                    v-if="connection.status === 'Connected' && connection.id"
                    @click="disconnectMcpServer(connection)" 
                    class="btn btn-xs btn-outline btn-warning" 
                    :title="'断开连接'"
                  >
                    <i class="fas fa-unlink"></i>
                  </button>
                  <button 
                    v-if="connection.status === 'Connected' && connection.id"
                    @click="openTestServerModal(connection)" 
                    class="btn btn-xs btn-outline btn-info" 
                    :title="'测试服务器工具'"
                  >
                    <i class="fas fa-vial"></i>
                  </button>
                  <button 
                    v-else-if="connection.status !== 'Connected'"
                    @click="connectMcpServer(connection)" 
                    class="btn btn-xs btn-outline btn-success" 
                    :title="'连接'"
                  >
                    <i class="fas fa-link"></i>
                  </button>
                  <button 
                    @click="deleteMcpServer(connection)" 
                    class="btn btn-xs btn-outline btn-error" 
                    :title="$t('common.delete')"
                  >
                    <i class="fas fa-trash"></i>
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
              <td colspan="6" class="text-center py-4">{{ $t('Tools.noConnections') }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      
      <!-- 卡片视图 -->
      <div v-if="myServersView === 'card'" class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
        <div 
          v-for="connection in mcpConnections" 
          :key="connection.id || connection.name"
          class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
        >
          <div class="card-body">
            <div class="flex items-center gap-3">
              <div class="avatar">
                <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                  <i class="fas fa-server text-primary text-xl"></i>
                </div>
              </div>
              <div class="flex-1">
                <h3 class="card-title text-lg">{{ connection.name }}</h3>
                <span class="badge badge-ghost badge-sm">{{ connection.transport_type }}</span>
              </div>
              <div class="form-control">
                <label class="label cursor-pointer">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-sm toggle-success" 
                    :checked="connection.status === 'Connected'" 
                    @change="toggleServerEnabled(connection)" 
                  />
                </label>
              </div>
            </div>

            <p class="text-sm mt-2 h-12 overflow-hidden">{{ connection.endpoint }}</p>

            <div class="flex items-center justify-between mt-2">
              <span :class="getStatusBadgeClass(connection.status)" class="flex items-center gap-1">
                <i :class="getStatusIcon(connection.status)"></i>
                {{ connection.status }}
              </span>
            </div>

            <div class="card-actions justify-end mt-4">
              <button 
                v-if="connection.status === 'Connected' && connection.id"
                @click="disconnectMcpServer(connection)" 
                class="btn btn-xs btn-outline btn-warning" 
                :title="'断开连接'"
              >
                <i class="fas fa-unlink"></i>
              </button>
              <button 
                v-if="connection.status === 'Connected' && connection.id"
                @click="openTestServerModal(connection)" 
                class="btn btn-xs btn-outline btn-info" 
                :title="'测试服务器工具'"
              >
                <i class="fas fa-vial"></i>
              </button>
              <button 
                v-else-if="connection.status !== 'Connected'"
                @click="connectMcpServer(connection)" 
                class="btn btn-xs btn-outline btn-success" 
                :title="'连接'"
              >
                <i class="fas fa-link"></i>
              </button>
              <button 
                @click="deleteMcpServer(connection)" 
                class="btn btn-xs btn-outline btn-error" 
                :title="$t('common.delete')"
              >
                <i class="fas fa-trash"></i>
              </button>
              <button 
                @click="openDetailsModal(connection)"
                class="btn btn-xs btn-outline" 
                :title="$t('common.details')"
              >
                <i class="fas fa-info"></i>
              </button>
            </div>
          </div>
        </div>
        
        <div v-if="mcpConnections.length === 0" class="col-span-full text-center py-8">
          {{ $t('Tools.noConnections') }}
        </div>
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
                {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('Tools.added') : $t('common.add')) }}
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
                  {{ server.is_adding ? $t('common.loading') : (isServerAdded(server) ? $t('Tools.added') : $t('common.add')) }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 插件工具管理 -->
    <div v-if="activeTab === 'plugin_tools'" class="space-y-4">
      <div class="flex justify-between items-center">
        <div class="alert alert-info flex-1 mr-4">
          <i class="fas fa-info-circle"></i>
          <span>管理 Agent 插件工具，可在创建 Agent 时选择启用的插件工具</span>
        </div>
        <div class="join">
          <button @click="pluginToolsView = 'card'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': pluginToolsView === 'card'}]">
            <i class="fas fa-th-large"></i>
          </button>
          <button @click="pluginToolsView = 'list'" :class="['join-item', 'btn', 'btn-sm', {'btn-primary': pluginToolsView === 'list'}]">
            <i class="fas fa-list"></i>
          </button>
        </div>
      </div>
      
      <!-- 插件列表 -->
      <div v-if="isLoadingPlugins" class="text-center p-8">
        <i class="fas fa-spinner fa-spin text-2xl"></i>
        <p class="mt-2">正在加载插件...</p>
      </div>
      
      <div v-else-if="passivePlugins.length > 0" class="space-y-4">
        <div class="flex justify-end mb-4">
          <button @click="showUploadPluginModal = true" class="btn btn-primary btn-sm">
            <i class="fas fa-upload mr-2"></i>
            上传插件
          </button>
        </div>
        
        <!-- 卡片视图 -->
        <div v-if="pluginToolsView === 'card'" class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div 
            v-for="plugin in passivePlugins" 
            :key="plugin.metadata.id"
            class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow"
          >
            <div class="card-body">
              <div class="flex items-center gap-3">
                <div class="avatar">
                  <div class="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <i class="fas fa-puzzle-piece text-primary text-xl"></i>
                  </div>
                </div>
                <div class="flex-1">
                  <h3 class="card-title text-lg">{{ plugin.metadata.name }}</h3>
                  <span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span>
                </div>
                <div class="form-control">
                  <label class="label cursor-pointer">
                    <input 
                      type="checkbox" 
                      class="toggle toggle-primary toggle-sm" 
                      :checked="plugin.status === 'Enabled'"
                      @change="togglePlugin(plugin)"
                      :disabled="plugin.is_toggling"
                    />
                  </label>
                </div>
              </div>

              <p class="text-sm mt-2 h-16">{{ plugin.metadata.description }}</p>

              <div class="flex flex-wrap gap-2 mt-2">
                <span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span>
                <span 
                  v-for="perm in plugin.metadata.permissions" 
                  :key="perm"
                  class="badge badge-warning badge-xs"
                >
                  {{ perm }}
                </span>
              </div>

              <div class="card-actions justify-between items-center mt-4">
                <div class="flex gap-1">
                  <span 
                    :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']"
                  >
                    {{ plugin.status }}
                  </span>
                  <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                    <i class="fas fa-exclamation-triangle"></i>
                  </span>
                </div>
                <div class="flex gap-1">
                  <button 
                    @click="editPlugin(plugin)"
                    class="btn btn-xs btn-outline"
                    :title="'编辑'"
                  >
                    <i class="fas fa-edit"></i>
                  </button>
                  <button 
                    @click="viewPluginInfo(plugin)"
                    class="btn btn-xs btn-outline"
                    :title="'详情'"
                  >
                    <i class="fas fa-info"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 列表视图 -->
        <div v-if="pluginToolsView === 'list'" class="overflow-x-auto">
          <table class="table w-full">
            <thead>
              <tr>
                <th class="w-1/12">启用</th>
                <th>名称</th>
                <th>版本</th>
                <th>作者</th>
                <th>描述</th>
                <th>状态</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="plugin in passivePlugins" :key="plugin.metadata.id">
                <td>
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary toggle-sm" 
                    :checked="plugin.status === 'Enabled'"
                    @change="togglePlugin(plugin)"
                    :disabled="plugin.is_toggling"
                  />
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <i class="fas fa-puzzle-piece text-primary"></i>
                    <span class="font-semibold">{{ plugin.metadata.name }}</span>
                  </div>
                </td>
                <td><span class="badge badge-ghost badge-sm">v{{ plugin.metadata.version }}</span></td>
                <td><span class="badge badge-outline badge-xs">{{ plugin.metadata.author }}</span></td>
                <td class="text-sm">{{ plugin.metadata.description }}</td>
                <td>
                  <div class="flex flex-col gap-1">
                    <span :class="['badge badge-sm', plugin.status === 'Enabled' ? 'badge-success' : 'badge-ghost']">
                      {{ plugin.status }}
                    </span>
                    <span v-if="plugin.last_error" class="badge badge-error badge-sm" :title="plugin.last_error">
                      <i class="fas fa-exclamation-triangle"></i>
                    </span>
                  </div>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button 
                      @click="editPlugin(plugin)"
                      class="btn btn-xs btn-outline"
                      :title="'编辑'"
                    >
                      <i class="fas fa-edit"></i>
                    </button>
                    <button 
                      @click="viewPluginInfo(plugin)"
                      class="btn btn-xs btn-outline"
                      :title="'详情'"
                    >
                      <i class="fas fa-info"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      
      <div v-else class="text-center p-8">
        <i class="fas fa-plug text-4xl text-base-content/30 mb-4"></i>
        <p class="text-lg font-semibold">暂无插件工具</p>
        <p class="text-base-content/70 mt-2">前往插件管理创建 Agent 工具插件</p>
        <button @click="goToPluginManagement" class="btn btn-primary mt-4">
          <i class="fas fa-plus mr-2"></i>
          创建插件工具
        </button>
      </div>
    </div>

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
            <!-- 编辑模式切换 -->
            <div class="flex justify-end">
              <div class="join">
                <button 
                  @click="editMode = 'form'" 
                  :class="['join-item', 'btn', 'btn-sm', {'btn-primary': editMode === 'form'}]"
                >
                  <i class="fas fa-edit mr-1"></i>
                  表单编辑
                </button>
                <button 
                  @click="editMode = 'json'" 
                  :class="['join-item', 'btn', 'btn-sm', {'btn-primary': editMode === 'json'}]"
                >
                  <i class="fas fa-code mr-1"></i>
                  JSON 编辑
                </button>
              </div>
            </div>

            <!-- 表单编辑模式 -->
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
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.command') }}</span></label>
                <input type="text" v-model="editableServer.command" class="input input-bordered font-mono" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.args') }}</span></label>
                <textarea v-model="editableServer.args" class="textarea textarea-bordered font-mono" rows="3"></textarea>
              </div>
            </div>

            <!-- JSON 编辑模式 -->
            <div v-if="editMode === 'json'" class="space-y-4">
              <div class="alert alert-warning">
                <i class="fas fa-exclamation-triangle"></i>
                <span>直接编辑 JSON 配置，请确保格式正确</span>
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">服务器配置 (JSON)</span></label>
                <textarea 
                  v-model="editableServerJson" 
                  class="textarea textarea-bordered font-mono text-sm" 
                  rows="15"
                  spellcheck="false"
                ></textarea>
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
                  <p class="text-sm text-base-content/60 font-normal">{{ tool.description }}</p>
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

    <!-- 添加服务器模态框 (保持不变) -->
    <dialog :class="['modal', { 'modal-open': showAddServerModal }]">
      <div v-if="showAddServerModal" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">{{ $t('Tools.addServer.title') }}</h3>
          <button @click="showAddServerModal = false" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="tabs tabs-boxed mb-4">
          <button @click="addServerMode = 'quick'" :class="['tab', { 'tab-active': addServerMode === 'quick' }]">
            <i class="fas fa-magic mr-2"></i>{{ $t('Tools.addServer.quickCreate') }}
          </button>
          <button @click="addServerMode = 'json'" :class="['tab', { 'tab-active': addServerMode === 'json' }]">
            <i class="fas fa-file-code mr-2"></i>{{ $t('Tools.addServer.importFromJson') }}
          </button>
        </div>

        <!-- 快速创建表单 -->
        <div v-if="addServerMode === 'quick'" class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="flex items-center justify-between">
            <label class="label">{{ $t('Tools.addServer.enabled') }}</label>
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
            <label class="label"><span class="label-text">{{ $t('Tools.addServer.params') }}</span></label>
            <textarea class="textarea textarea-bordered font-mono" :placeholder="$t('Tools.addServer.paramsPlaceholder')" rows="3" v-model="quickCreateForm.params"></textarea>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('Tools.addServer.envVars') }}</span></label>
            <textarea class="textarea textarea-bordered font-mono" placeholder="KEY1=value1&#10;KEY2=value2" rows="3" v-model="quickCreateForm.envVars"></textarea>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('Tools.addServer.timeout') }}</span></label>
            <input type="number" placeholder="60" class="input input-bordered" v-model.number="quickCreateForm.timeout" />
          </div>

          <div class="collapse collapse-arrow border border-base-300 bg-base-100">
            <input type="checkbox" /> 
            <div class="collapse-title text-md font-medium">{{ $t('Tools.addServer.advancedSettings') }}</div>
            <div class="collapse-content space-y-4">
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.providerName') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.providerName" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.providerWebsite') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.providerWebsite" />
              </div>
              <div class="form-control">
                <label class="label"><span class="label-text">{{ $t('Tools.addServer.logoUrl') }}</span></label>
                <input type="text" class="input input-bordered" v-model="quickCreateForm.logoUrl" />
              </div>
            </div>
          </div>
        </div>

        <!-- 从JSON导入 -->
        <div v-if="addServerMode === 'json'" class="space-y-4">
           <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('Tools.addServer.jsonPaste') }}<span class="text-error">*</span></span></label>
            <textarea v-model="jsonImportConfig" class="textarea textarea-bordered font-mono" rows="15"></textarea>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showAddServerModal = false" class="btn">{{ $t('common.cancel') }}</button>
          <button v-if="addServerMode === 'quick'" @click="handleQuickCreateServer" class="btn btn-primary">{{ $t('common.save') }}</button>
          <button v-if="addServerMode === 'json'" @click="handleImportFromJson" class="btn btn-primary">{{ $t('Tools.addServer.import') }}</button>
        </div>
      </div>
    </dialog>

    <!-- 内置工具测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showTestBuiltinToolModal }]">
      <div v-if="testingBuiltinTool" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">
            测试内置工具: {{ testingBuiltinTool.name }}
          </h3>
          <button @click="closeTestBuiltinToolModal" class="btn btn-sm btn-ghost">✕</button>
        </div>

        <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>输入测试参数后点击运行测试，可以验证工具是否正常工作。</span>
          </div>

          <!-- 工具描述 -->
          <div class="bg-base-200 p-4 rounded-lg">
            <p class="text-sm">{{ testingBuiltinTool.description }}</p>
            <div class="flex gap-2 mt-2">
              <span class="badge badge-success badge-sm">{{ testingBuiltinTool.category }}</span>
              <span class="badge badge-ghost badge-sm">v{{ testingBuiltinTool.version }}</span>
            </div>
          </div>

          <!-- 参数说明（仅当有 input_schema 时显示） -->
          <div v-if="testingBuiltinTool.input_schema && testingBuiltinTool.input_schema.properties && Object.keys(testingBuiltinTool.input_schema.properties).length > 0" class="collapse collapse-arrow border border-base-300 bg-base-100">
            <input type="checkbox" checked />
            <div class="collapse-title text-md font-medium">
              输入参数说明
            </div>
            <div class="collapse-content">
              <div class="overflow-x-auto">
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
                    <tr v-for="prop in getToolProperties(testingBuiltinTool.input_schema)" :key="prop.name">
                      <td class="font-mono text-primary">{{ prop.name }}</td>
                      <td><span class="badge badge-outline">{{ prop.type }}</span></td>
                      <td>
                        <span v-if="prop.required" class="badge badge-error badge-sm">必填</span>
                      </td>
                      <td>{{ prop.description }}</td>
                      <td class="font-mono text-xs">{{ prop.constraints }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- 无参数工具提示 -->
          <div v-else class="alert alert-warning">
            <i class="fas fa-exclamation-triangle"></i>
            <span>此工具没有输入参数或参数信息暂未提供，可直接运行测试。</span>
          </div>

          <!-- 测试参数输入（始终显示） -->
          <div class="form-control">
            <label class="label"><span class="label-text">测试参数 (JSON)</span></label>
            <textarea
              v-model="testBuiltinToolParamsJson"
              class="textarea textarea-bordered font-mono text-sm"
              placeholder="输入 JSON 格式的测试参数，例如: {}"
              rows="6"
              spellcheck="false"
            ></textarea>
          </div>

          <!-- 测试结果（始终显示） -->
          <div class="form-control">
            <label class="label"><span class="label-text">测试结果</span></label>
            <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap h-40 bg-base-200 overflow-auto">{{ testBuiltinToolResult || '点击"运行测试"查看结果' }}</pre>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeTestBuiltinToolModal" class="btn">{{ $t('common.cancel') }}</button>
          <button 
            class="btn btn-primary"
            :disabled="isTestingBuiltinTool"
            @click="runTestBuiltinTool"
          >
            <i v-if="isTestingBuiltinTool" class="fas fa-spinner fa-spin mr-1"></i>
            <i v-else class="fas fa-play mr-1"></i>
            运行测试
          </button>
        </div>
      </div>
    </dialog>

    <!-- 服务器工具测试模态框 -->
    <dialog :class="['modal', { 'modal-open': showTestServerModal }]">
      <div v-if="showTestServerModal" class="modal-box w-11/12 max-w-5xl">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">
            测试服务器工具: {{ testingServer?.name }}
          </h3>
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
                {{ tool.name }} - {{ tool.description }}
              </option>
            </select>
          </div>

          <div v-if="selectedTestTool" class="space-y-3">
            <div class="collapse collapse-arrow border border-base-300 bg-base-100">
              <input type="checkbox" />
              <div class="collapse-title text-md font-medium">
                输入参数说明
              </div>
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
                        <td>
                          <span v-if="prop.required" class="badge badge-error badge-sm">必填</span>
                        </td>
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
              <textarea
                v-model="testToolParamsJson"
                class="textarea textarea-bordered font-mono text-sm"
                placeholder="留空使用默认参数，或输入 JSON 对象覆盖默认参数"
                rows="6"
                spellcheck="false"
              ></textarea>
            </div>

            <div class="form-control">
              <label class="label"><span class="label-text">测试结果</span></label>
              <pre class="textarea textarea-bordered font-mono text-xs whitespace-pre-wrap h-40 bg-base-200">
{{ testToolResult }}
              </pre>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="closeTestServerModal" class="btn">{{ $t('common.cancel') }}</button>
          <button 
            class="btn btn-primary"
            :disabled="!selectedTestToolName || isTestingTool"
            @click="runTestTool"
          >
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
import { ref, onMounted, onUnmounted, reactive, computed, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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

interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  main_category: string;
  category: string;
  permissions: string[];
}

interface PluginRecord {
  metadata: PluginMetadata;
  path: string;
  status: string;
  last_error: string | null;
  is_toggling?: boolean;
}

// --- 响应式状态 ---
const activeTab = ref('builtin_tools');
const mcpConnections = ref<McpConnection[]>([]);
const marketplaceServers = ref<MarketplaceServer[]>([]);
const builtinTools = ref<any[]>([]);
const isLoadingBuiltinTools = ref(false);
const passivePlugins = ref<PluginRecord[]>([]);
const isLoadingPlugins = ref(false);
const showUploadPluginModal = ref(false);
const showAddServerModal = ref(false);
const addServerMode = ref('quick');
const marketplaceView = ref('list'); // 'card' or 'list'
const builtinToolsView = ref('list'); // 'card' or 'list'
const myServersView = ref('list'); // 'card' or 'list'
const pluginToolsView = ref('list'); // 'card' or 'list'
const showDetailsModal = ref(false);
const detailsTab = ref('general');
const editMode = ref('form'); // 'form' or 'json'
const selectedServer = ref<McpConnection | null>(null);
const editableServer = reactive({
  db_id: -1,
  name: '',
  description: '',
  command: '',
  args: '',
  enabled: true,
});
const editableServerJson = ref('');
const serverTools = ref<FrontendTool[]>([]);
const isLoadingTools = ref(false);
const showTestServerModal = ref(false);
const testingServer = ref<McpConnection | null>(null);
const testServerTools = ref<FrontendTool[]>([]);
const isLoadingTestTools = ref(false);
const selectedTestToolName = ref('');
const testToolParamsJson = ref('');
const testToolResult = ref('');
const isTestingTool = ref(false);

// 内置工具测试相关状态
const showTestBuiltinToolModal = ref(false);
const testingBuiltinTool = ref<any>(null);
const testBuiltinToolParamsJson = ref('');
const testBuiltinToolResult = ref('');
const isTestingBuiltinTool = ref(false);
// const statusUpdateInterval = ref<NodeJS.Timeout | null>(null);

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

// 测试模态框选中的工具
const selectedTestTool = computed(() => {
  if (!selectedTestToolName.value) return null;
  return testServerTools.value.find(t => t.name === selectedTestToolName.value) || null;
});

// 监听工具选择变化，自动填充默认参数
watch(selectedTestTool, (newTool) => {
  if (newTool && newTool.input_schema) {
    testToolParamsJson.value = generateDefaultParams(newTool.input_schema);
  } else {
    testToolParamsJson.value = '{}';
  }
});

// --- 方法 ---

function generateDefaultParams(schema: any): string {
  if (!schema || !schema.properties) {
    return '{}';
  }

  const params: any = {};
  for (const name in schema.properties) {
    const prop = schema.properties[name];
    if (prop.default !== undefined) {
      params[name] = prop.default;
    } else {
      // 根据类型设置空值
      switch (prop.type) {
        case 'string':
          params[name] = '';
          break;
        case 'number':
        case 'integer':
          params[name] = prop.minimum !== undefined ? prop.minimum : 0;
          break;
        case 'boolean':
          params[name] = false;
          break;
        case 'array':
          params[name] = [];
          break;
        case 'object':
          params[name] = {};
          break;
        default:
          params[name] = null;
      }
    }
  }
  return JSON.stringify(params, null, 2);
}

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Connected': return 'badge badge-sm badge-success';
    case 'Error': return 'badge badge-sm badge-error';
    case 'Disconnected': return 'badge badge-sm badge-warning';
    case 'Connecting': return 'badge badge-sm badge-info';
    default: return 'badge badge-sm';
  }
};

const getStatusIcon = (status: string) => {
  switch (status) {
    case 'Connected': return 'fas fa-check-circle';
    case 'Error': return 'fas fa-exclamation-circle';
    case 'Disconnected': return 'fas fa-times-circle';
    case 'Connecting': return 'fas fa-spinner fa-spin';
    default: return 'fas fa-question-circle';
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
  // 先初始化所有数据，防止闪烁
  selectedServer.value = { ...connection };
  Object.assign(editableServer, {
    db_id: connection.db_id,
    name: connection.name,
    description: connection.description || '',
    command: connection.command,
    args: connection.args.join(' '), // 将数组转为空格分隔的字符串以便编辑
    enabled: true,
  });
  
  // 初始化 JSON 编辑模式的数据
  editableServerJson.value = JSON.stringify({
    db_id: connection.db_id,
    id: connection.id,
    name: connection.name,
    description: connection.description || '',
    transport_type: connection.transport_type,
    endpoint: connection.endpoint,
    status: connection.status,
    command: connection.command,
    args: connection.args,
  }, null, 2);
  
  detailsTab.value = 'general';
  editMode.value = 'form';
  serverTools.value = [];
  
  // 最后再显示模态框
  showDetailsModal.value = true;
  
  // 异步加载工具列表
  if (connection.status === 'Connected' && connection.id) {
    fetchServerTools();
  }
}

function closeDetailsModal() {
  showDetailsModal.value = false;
  // 延迟清空数据，避免闪烁
  setTimeout(() => {
    selectedServer.value = null;
    editMode.value = 'form';
    serverTools.value = [];
  }, 300);
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

async function openTestServerModal(connection: McpConnection) {
  testingServer.value = { ...connection };
  showTestServerModal.value = true;
  testServerTools.value = [];
  selectedTestToolName.value = '';
  testToolParamsJson.value = '';
  testToolResult.value = '';

  if (!connection.id) {
    dialog.toast.error('当前服务器未处于连接状态，无法测试工具');
    return;
  }

  isLoadingTestTools.value = true;
  try {
    const tools = await invoke<FrontendTool[]>('mcp_get_connection_tools', { connectionId: connection.id });
    testServerTools.value = tools || [];
    if (testServerTools.value.length > 0) {
      selectedTestToolName.value = testServerTools.value[0].name;
      // 自动填充第一个工具的默认参数
      if (testServerTools.value[0].input_schema) {
        testToolParamsJson.value = generateDefaultParams(testServerTools.value[0].input_schema);
      }
    }
  } catch (error) {
    console.error('Failed to fetch tools for testing:', error);
    dialog.toast.error('加载服务器工具列表失败');
  } finally {
    isLoadingTestTools.value = false;
  }
}

function closeTestServerModal() {
  showTestServerModal.value = false;
  setTimeout(() => {
    testingServer.value = null;
    testServerTools.value = [];
    selectedTestToolName.value = '';
    testToolParamsJson.value = '';
    testToolResult.value = '';
  }, 300);
}

async function runTestTool() {
  if (!testingServer.value || !testingServer.value.id || !selectedTestToolName.value) {
    dialog.toast.error('请选择要测试的工具');
    return;
  }

  let args: any = {};
  if (testToolParamsJson.value.trim()) {
    try {
      args = JSON.parse(testToolParamsJson.value);
    } catch (e) {
      dialog.toast.error('参数 JSON 格式错误，请检查');
      return;
    }
  }

  isTestingTool.value = true;
  testToolResult.value = '正在执行测试...';
  try {
    const result = await invoke<any>('mcp_test_server_tool', {
      connectionId: testingServer.value.id,
      toolName: selectedTestToolName.value,
      args,
    });

    testToolResult.value = typeof result === 'string'
      ? result
      : JSON.stringify(result, null, 2);
    dialog.toast.success('工具测试完成');
  } catch (error: any) {
    console.error('Failed to test server tool:', error);
    testToolResult.value = `测试失败: ${error?.message || String(error)}`;
    dialog.toast.error('工具测试失败');
  } finally {
    isTestingTool.value = false;
  }
}

async function saveServerDetails() {
  if (!selectedServer.value) return;
  try {
    let payload;
    
    if (editMode.value === 'json') {
      // JSON 模式：解析 JSON 字符串
      try {
        const jsonData = JSON.parse(editableServerJson.value);
        payload = {
          db_id: jsonData.db_id,
          id: jsonData.id || null,
          name: jsonData.name,
          description: jsonData.description || '',
          command: jsonData.command,
          args: Array.isArray(jsonData.args) ? jsonData.args : [],
          transport_type: jsonData.transport_type || 'stdio',
          endpoint: jsonData.endpoint || '',
          status: jsonData.status || selectedServer.value.status || 'Disconnected',
        };
      } catch (e) {
        dialog.toast.error('JSON 格式错误，请检查语法');
        return;
      }
    } else {
      // 表单模式：从表单数据构建 payload
      payload = {
        db_id: editableServer.db_id,
        id: selectedServer.value.id,
        name: editableServer.name,
        description: editableServer.description || '',
        command: editableServer.command,
        args: editableServer.args.split(' ').filter(s => s.trim() !== ''), // 将字符串转回数组
        transport_type: selectedServer.value.transport_type,
        endpoint: selectedServer.value.endpoint,
        status: selectedServer.value.status,
      };
    }
    
    // 保存配置
    await invoke('mcp_update_server_config', { payload });
    
    // 如果服务器当前已连接，需要断开并重新连接以应用更改
    const wasConnected = selectedServer.value.status === 'Connected' && selectedServer.value.id;
    if (wasConnected) {
      try {
        await invoke('mcp_disconnect_server', { connectionId: selectedServer.value.id });
        // 等待一小段时间确保断开完成
        await new Promise(resolve => setTimeout(resolve, 500));
        // 重新连接
        await invoke('add_child_process_mcp_server', { 
          name: payload.name, 
          command: payload.command, 
          args: payload.args 
        });
        dialog.toast.success(t('Tools.updateSuccess') + '，服务器已重新连接');
      } catch (reconnectError) {
        console.error('Failed to reconnect server:', reconnectError);
        dialog.toast.warning(t('Tools.updateSuccess') + '，但重新连接失败，请手动重连');
      }
    } else {
      dialog.toast.success(t('Tools.updateSuccess'));
    }
    
    closeDetailsModal();
    await fetchConnections();
  } catch (error) {
    console.error("Failed to save server details:", error);
    dialog.toast.error(`${t('Tools.updateFailed')}: ${error}`);
  }
}

async function fetchConnections() {
  try {
    mcpConnections.value = await invoke('mcp_get_connections');
    // 获取实时连接状态
    await updateConnectionStatus();
  } catch (error) {
    console.error('Failed to fetch MCP connections:', error);
    mcpConnections.value = [];
  }
}

async function updateConnectionStatus() {
  try {
    const statusMap = await invoke('mcp_get_connection_status') as Record<string, string>;
    // 更新连接状态
    mcpConnections.value.forEach(conn => {
      if (conn.name && statusMap[conn.name]) {
        conn.status = statusMap[conn.name];
      }
    });
  } catch (error) {
    console.error('Failed to fetch connection status:', error);
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

// 打开内置工具测试模态框
function openTestBuiltinToolModal(tool: any) {
  // 先设置工具数据
  testingBuiltinTool.value = { ...tool };
  testBuiltinToolResult.value = '';
  
  // 根据工具的 input_schema 生成默认参数
  if (tool.input_schema) {
    testBuiltinToolParamsJson.value = generateDefaultParams(tool.input_schema);
  } else {
    testBuiltinToolParamsJson.value = '{}';
  }
  
  // 使用 nextTick 确保数据设置完成后再显示模态框，避免闪屏
  nextTick(() => {
    showTestBuiltinToolModal.value = true;
  });
}

// 关闭内置工具测试模态框
function closeTestBuiltinToolModal() {
  // 先关闭模态框，延迟清空数据避免闪烁
  showTestBuiltinToolModal.value = false;
  // 使用较长的延迟确保动画完成后再清空数据
  setTimeout(() => {
    testingBuiltinTool.value = null;
    testBuiltinToolParamsJson.value = '';
    testBuiltinToolResult.value = '';
  }, 350);
}

// 运行内置工具测试
async function runTestBuiltinTool() {
  if (!testingBuiltinTool.value) {
    dialog.toast.error('请选择要测试的工具');
    return;
  }

  let inputs: any = {};
  if (testBuiltinToolParamsJson.value.trim()) {
    try {
      inputs = JSON.parse(testBuiltinToolParamsJson.value);
    } catch (e) {
      dialog.toast.error('参数 JSON 格式错误，请检查');
      return;
    }
  }

  isTestingBuiltinTool.value = true;
  testBuiltinToolResult.value = '正在执行测试...';
  
  try {
    const result = await invoke<any>('unified_execute_tool', {
      toolName: testingBuiltinTool.value.name,
      inputs,
      context: null,
      timeout: 60,
    });

    if (result.success) {
      testBuiltinToolResult.value = typeof result.output === 'string'
        ? result.output
        : JSON.stringify(result.output, null, 2);
      dialog.toast.success('工具测试完成');
    } else {
      testBuiltinToolResult.value = `测试失败: ${result.error || '未知错误'}`;
      dialog.toast.error('工具测试失败');
    }
  } catch (error: any) {
    console.error('Failed to test builtin tool:', error);
    testBuiltinToolResult.value = `测试失败: ${error?.message || String(error)}`;
    dialog.toast.error('工具测试失败');
  } finally {
    isTestingBuiltinTool.value = false;
  }
}

// 快速测试内置工具（使用默认参数）
async function testBuiltinTool(tool: any) {
  tool.is_testing = true;
  try {
    // 使用默认测试参数
    const defaultParams = tool.input_schema ? JSON.parse(generateDefaultParams(tool.input_schema)) : {};
    
    const result = await invoke<any>('unified_execute_tool', {
      toolName: tool.name,
      inputs: defaultParams,
      context: null,
      timeout: 60,
    });
    
    if (result.success) {
      dialog.toast.success(`工具 ${tool.name} 测试成功`);
    } else {
      dialog.toast.error(`工具 ${tool.name} 测试失败：${result.error || '未知错误'}`);
    }
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
      // 断开连接
      await invoke('mcp_disconnect_server', { connectionId: connection.id });
      dialog.toast.success(`已断开服务器 ${connection.name}`);
    } else {
      // 连接服务器 - 检查是否已经存在连接
      const existingConnection = mcpConnections.value.find(conn => 
        conn.name === connection.name && conn.status === 'Connected'
      );
      
      if (existingConnection) {
        dialog.toast.warning(`服务器 ${connection.name} 已经连接`);
        return;
      }
      
      await invoke('add_child_process_mcp_server', { 
        name: connection.name, 
        command: connection.command, 
        args: connection.args 
      });
      dialog.toast.success(`已连接服务器 ${connection.name}`);
    }
    await fetchConnections();
  } catch (error) {
    console.error(`Failed to toggle server ${connection.name} state:`, error);
    dialog.toast.error(`切换服务器 ${connection.name} 状态失败: ${error}`);
  }
}

async function disconnectMcpServer(connection: McpConnection) {
  if (!connection.id) return;
  try {
    await invoke('mcp_disconnect_server', { connectionId: connection.id });
    dialog.toast.success(`已断开服务器 ${connection.name}`);
    await fetchConnections();
  } catch (error) {
    console.error('Failed to disconnect MCP server:', error);
    dialog.toast.error(`断开服务器失败: ${error}`);
  }
}

async function connectMcpServer(connection: McpConnection) {
  try {
    console.log('Connecting MCP server:', {
      name: connection.name,
      command: connection.command,
      args: connection.args
    });
    await invoke('add_child_process_mcp_server', { 
      name: connection.name, 
      command: connection.command, 
      args: connection.args 
    });
    dialog.toast.success(`已连接服务器 ${connection.name}`);
    await fetchConnections();
  } catch (error) {
    console.error('Failed to connect MCP server:', error);
    dialog.toast.error(`连接服务器失败: ${error}`);
  }
}

async function deleteMcpServer(connection: McpConnection) {
  try {
    const confirmed = await dialog.confirm(`确定要删除服务器 "${connection.name}" 吗？此操作将删除数据库配置且不可恢复。`);
    if (!confirmed) return;
    
    // 如果服务器已连接，先断开
    if (connection.status === 'Connected' && connection.id) {
      try {
        await invoke('mcp_disconnect_server', { connectionId: connection.id });
      } catch (e) {
        console.warn('Failed to disconnect before delete:', e);
      }
    }
    
    // 删除数据库配置
    await invoke('mcp_delete_server_config', { dbId: connection.db_id });
    dialog.toast.success(`已删除服务器 ${connection.name}`);
    await fetchConnections();
  } catch (error) {
    console.error('Failed to delete MCP server:', error);
    dialog.toast.error(`删除服务器失败: ${error}`);
  }
}

async function handleQuickCreateServer() {
  if (!quickCreateForm.name) {
    await dialog.error(t('Tools.addServer.nameRequired'));
    return;
  }
  try {
    await invoke('quick_create_mcp_server', { config: quickCreateForm });
    showAddServerModal.value = false;
    Object.assign(quickCreateForm, { enabled: true, name: '', description: '', type: 'stdio', params: '', envVars: '', timeout: 60, providerName: '', providerWebsite: '', logoUrl: '' });
    await fetchConnections();
  } catch (error) {
    console.error("快速创建服务器失败:", error);
    await dialog.error(`${t('Tools.addServerFailed')}: ${error}`);
  }
}

async function handleImportFromJson() {
  if (!jsonImportConfig.value.trim()) {
    await dialog.error(t('Tools.addServer.jsonRequired'));
    return;
  }
  try {
    await invoke('import_mcp_servers_from_json', { jsonConfig: jsonImportConfig.value });
    dialog.toast.success(t('Tools.importSuccess'));
    showAddServerModal.value = false;
    await fetchConnections();
  } catch (error) {
    console.error("从JSON导入服务器失败:", error);
    dialog.toast.error(`${t('Tools.importFailed')}: ${error}`);
  }
}

async function cleanupDuplicateServers() {
  try {
    const confirmed = await dialog.confirm('确定要清理重复的MCP服务器配置吗？这将删除重复的配置，只保留最新的。');
    if (!confirmed) return;
    
    const removedDuplicates: string[] = await invoke('cleanup_duplicate_mcp_servers');
    
    if (removedDuplicates.length > 0) {
      dialog.toast.success(`已清理 ${removedDuplicates.length} 个重复配置`);
      console.log('清理的重复配置:', removedDuplicates);
    } else {
      dialog.toast.info('没有发现重复的服务器配置');
    }
    
    await fetchConnections();
  } catch (error) {
    console.error('清理重复服务器失败:', error);
    dialog.toast.error(`清理失败: ${error}`);
  }
}

// --- 被动扫描插件管理 ---
async function fetchPassivePlugins() {
  isLoadingPlugins.value = true;
  try {
    const response = await invoke<any>('list_plugins');
    if (response.success && response.data) {
      // 只加载 agents 类型的插件
      passivePlugins.value = response.data.filter((plugin: PluginRecord) => 
        plugin.metadata.main_category === 'agent'
      );
    }
  } catch (error) {
    console.error('Failed to fetch agent tool plugins:', error);
    passivePlugins.value = [];
  } finally {
    isLoadingPlugins.value = false;
  }
}

// 导航到插件管理页面
function goToPluginManagement() {
  // 使用路由导航到插件管理页面
  window.location.hash = '#/plugin-management';
}

async function togglePlugin(plugin: PluginRecord) {
  plugin.is_toggling = true;
  try {
    const isEnabled = plugin.status === 'Enabled';
    if (isEnabled) {
      await invoke('disable_plugin', { pluginId: plugin.metadata.id });
      dialog.toast.success(`已禁用插件: ${plugin.metadata.name}`);
    } else {
      await invoke('enable_plugin', { pluginId: plugin.metadata.id });
      dialog.toast.success(`已启用插件: ${plugin.metadata.name}`);
    }
    await fetchPassivePlugins();
  } catch (error: any) {
    console.error(`Failed to toggle plugin ${plugin.metadata.id}:`, error);
    dialog.toast.error(`切换插件状态失败: ${error}`);
  } finally {
    plugin.is_toggling = false;
  }
}

async function scanPluginDirectory() {
  try {
    const response = await invoke<any>('scan_plugin_directory');
    if (response.success && response.data) {
      dialog.toast.success(`已扫描并加载 ${response.data.length} 个插件`);
      await fetchPassivePlugins();
    }
  } catch (error: any) {
    console.error('Failed to scan plugin directory:', error);
    dialog.toast.error(`扫描插件目录失败: ${error}`);
  }
}

function editPlugin(plugin: PluginRecord) {
  // TODO: 实现插件编辑功能（Phase 6.8 高级功能）
  dialog.toast.info('插件编辑功能开发中...');
}

function viewPluginInfo(plugin: PluginRecord) {
  // 显示插件详细信息
  const info = `
插件名称: ${plugin.metadata.name}
版本: ${plugin.metadata.version}
作者: ${plugin.metadata.author}
描述: ${plugin.metadata.description}
权限: ${plugin.metadata.permissions.join(', ')}
状态: ${plugin.status}
${plugin.last_error ? `错误: ${plugin.last_error}` : ''}
  `.trim();
  
  dialog.info(info);
}

// 启动状态更新定时器
// function startStatusUpdates() {
//   if (statusUpdateInterval.value) {
//     clearInterval(statusUpdateInterval.value);
//   }
  
//   statusUpdateInterval.value = setInterval(async () => {
//     if (activeTab.value === 'my_servers') {
//       await updateConnectionStatus();
//     }
//   }, 2000); // 每2秒更新一次状态
// }

// // 停止状态更新定时器
// function stopStatusUpdates() {
//   if (statusUpdateInterval.value) {
//     clearInterval(statusUpdateInterval.value);
//     statusUpdateInterval.value = null;
//   }
// }

// --- 生命周期钩子 ---
onMounted(async () => {
  fetchConnections();
  fetchBuiltinTools();
  // 移除内置服务器列表
  marketplaceServers.value = [];
  // startStatusUpdates();
  
  // 加载被动扫描插件
  await fetchPassivePlugins();
  
  // 监听插件状态变化事件
  listen('plugin:changed', async () => {
    await fetchPassivePlugins();
  });
  
  // 监听MCP工具变更事件
  listen('mcp:tools-changed', async (event) => {
    console.log('MCP tools changed event received:', event.payload);
    // 刷新连接列表和工具列表
    await fetchConnections();
    await fetchBuiltinTools();
  });
});

// onUnmounted(() => {
//   stopStatusUpdates();
// });
</script>

<style scoped>
/* 优化模态框动画，防止闪烁 */
.modal {
  transition: opacity 0.2s ease-in-out;
}

.modal-box {
  transition: transform 0.2s ease-in-out, opacity 0.2s ease-in-out;
}

.modal-open .modal-box {
  animation: modalSlideIn 0.2s ease-out;
}

@keyframes modalSlideIn {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 防止内容切换时的闪烁 */
.space-y-4 > * {
  transition: opacity 0.15s ease-in-out;
}
</style>