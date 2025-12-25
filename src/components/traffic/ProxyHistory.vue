<template>
  <div class="flex flex-col h-full bg-base-200 overflow-hidden" @contextmenu.prevent>
    <!-- 证书错误提示弹窗 -->
    <dialog ref="certErrorDialog" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4 flex items-center gap-2">
          <i class="fas fa-exclamation-triangle text-warning"></i>
          {{ $t('trafficAnalysis.history.certificateError.title') }}
        </h3>
        
        <div class="space-y-4">
          <div class="alert alert-warning">
            <i class="fas fa-info-circle"></i>
            <span>{{ $t('trafficAnalysis.history.certificateError.message') }}</span>
          </div>
          
          <div v-if="certErrorInfo" class="bg-base-200 p-4 rounded-lg">
            <h4 class="font-semibold mb-2 text-sm">{{ $t('trafficAnalysis.history.certificateError.details') }}</h4>
            <div class="text-xs space-y-1 font-mono">
              <div><span class="text-base-content/70">Host:</span> {{ certErrorInfo.host }}</div>
              <div><span class="text-base-content/70">URL:</span> {{ certErrorInfo.url }}</div>
              <div v-if="certErrorInfo.error"><span class="text-base-content/70">Error:</span> {{ certErrorInfo.error }}</div>
            </div>
          </div>
          
          <div class="bg-base-300/50 p-4 rounded-lg">
            <h4 class="font-semibold mb-2 text-sm flex items-center gap-2">
              <i class="fas fa-lightbulb text-info"></i>
              {{ $t('trafficAnalysis.history.certificateError.commonIssues.invalidCN') }}
            </h4>
            <ul class="text-xs space-y-1 list-disc list-inside text-base-content/80">
              <li>{{ $t('trafficAnalysis.history.certificateError.tips.installCA') }}</li>
              <li>{{ $t('trafficAnalysis.history.certificateError.tips.serverCertIssue') }}</li>
            </ul>
          </div>
        </div>
        
        <div class="modal-action justify-between">
          <button class="btn btn-sm btn-ghost" @click="closeCertErrorDialog">
            {{ $t('trafficAnalysis.history.detailsPanel.close') }}
          </button>
          <div class="flex gap-2">
            <button class="btn btn-sm btn-info" @click="checkCAInstallation">
              <i class="fas fa-certificate mr-2"></i>
              {{ $t('trafficAnalysis.history.certificateError.tips.checkCAInstallation') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="closeCertErrorDialog">
              {{ $t('trafficAnalysis.history.certificateError.actions.ignore') }}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.history.detailsPanel.close') }}</button>
      </form>
    </dialog>
    
    <!-- 历史列表右键菜单 -->
    <div 
      v-if="contextMenu.visible"
      ref="contextMenuRef"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
      @click.stop
    >
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="sendToRepeater"
      >
        <i class="fas fa-redo text-primary"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendToRepeater') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="sendRequestToAssistantFromMenu"
      >
        <i class="fas fa-upload text-accent"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendRequestToAssistant') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="sendResponseToAssistantFromMenu"
      >
        <i class="fas fa-download text-accent"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendResponseToAssistant') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="copyUrl"
      >
        <i class="fas fa-link text-info"></i>
        {{ $t('trafficAnalysis.history.contextMenu.copyUrl') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="copyAsCurl"
      >
        <i class="fas fa-terminal text-warning"></i>
        {{ $t('trafficAnalysis.history.contextMenu.copyAsCurl') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="openInBrowser"
      >
        <i class="fas fa-external-link-alt text-success"></i>
        {{ $t('trafficAnalysis.history.contextMenu.openInBrowser') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2 text-error"
        @click="clearHistoryFromMenu"
      >
        <i class="fas fa-trash"></i>
        {{ $t('trafficAnalysis.history.contextMenu.clearHistory') }}
      </button>
    </div>

    <!-- 请求详情区域右键菜单 -->
    <div 
      v-if="detailContextMenu.visible"
      class="fixed z-50 bg-base-100 border border-base-300 rounded-lg shadow-xl py-1 min-w-48"
      :style="{ left: detailContextMenu.x + 'px', top: detailContextMenu.y + 'px' }"
      @click.stop
    >
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailSendToRepeater"
      >
        <i class="fas fa-redo text-primary"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendToRepeater') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailSendRequestToAssistant"
      >
        <i class="fas fa-upload text-accent"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendRequestToAssistant') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailSendResponseToAssistant"
      >
        <i class="fas fa-download text-accent"></i>
        {{ $t('trafficAnalysis.history.contextMenu.sendResponseToAssistant') }}
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailCopyUrl"
      >
        <i class="fas fa-link text-info"></i>
        {{ $t('trafficAnalysis.history.contextMenu.copyUrl') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailCopyRequest"
      >
        <i class="fas fa-copy text-secondary"></i>
        {{ $t('trafficAnalysis.history.contextMenu.copyRequest') }}
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="detailCopyAsCurl"
      >
        <i class="fas fa-terminal text-warning"></i>
        {{ $t('trafficAnalysis.history.contextMenu.copyAsCurl') }}
      </button>
    </div>

    <!-- 筛选器配置弹窗 -->
    <dialog ref="filterDialog" class="modal">
      <div class="modal-box max-w-6xl max-h-[90vh]">
        <h3 class="font-bold text-lg mb-4">Configure HTTP Proxy filter</h3>
        
        <!-- Mode Tabs -->
        <div class="tabs tabs-boxed mb-4 bg-base-200">
          <a 
            class="tab"
            :class="{ 'tab-active': filterMode === 'settings' }"
            @click="filterMode = 'settings'"
          >
            Settings mode
          </a>
          <a 
            class="tab"
            :class="{ 'tab-active': filterMode === 'bambda' }"
            @click="filterMode = 'bambda'"
          >
            Bambda mode
          </a>
        </div>

        <!-- Settings Mode Content -->
        <div v-if="filterMode === 'settings'" class="space-y-4 max-h-[60vh] overflow-y-auto pr-2">
          <div class="grid grid-cols-3 gap-4">
            <!-- Filter by request type -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by request type</h4>
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.showOnlyInScope" class="checkbox checkbox-sm" />
                  <span class="text-sm">Show only in-scope items</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.hideWithoutResponse" class="checkbox checkbox-sm" />
                  <span class="text-sm">Hide items without responses</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.requestType.showOnlyWithParams" class="checkbox checkbox-sm" />
                  <span class="text-sm">Show only parameterized requests</span>
                </label>
              </div>
            </div>

            <!-- Filter by MIME type -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by MIME type</h4>
              <div class="grid grid-cols-2 gap-x-3 gap-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.html" class="checkbox checkbox-sm" />
                  <span class="text-sm">HTML</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.otherText" class="checkbox checkbox-sm" />
                  <span class="text-sm">Other text</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.script" class="checkbox checkbox-sm" />
                  <span class="text-sm">Script</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.images" class="checkbox checkbox-sm" />
                  <span class="text-sm">Images</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.xml" class="checkbox checkbox-sm" />
                  <span class="text-sm">XML</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.flash" class="checkbox checkbox-sm" />
                  <span class="text-sm">Flash</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.css" class="checkbox checkbox-sm" />
                  <span class="text-sm">CSS</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.mimeType.otherBinary" class="checkbox checkbox-sm" />
                  <span class="text-sm">Other binary</span>
                </label>
              </div>
            </div>

            <!-- Filter by status code -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by status code</h4>
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s2xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">2xx [success]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s3xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">3xx [redirection]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s4xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">4xx [request error]</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.statusCode.s5xx" class="checkbox checkbox-sm" />
                  <span class="text-sm">5xx [server error]</span>
                </label>
              </div>
            </div>
          </div>

          <!-- Second Row -->
          <div class="grid grid-cols-3 gap-4">
            <!-- Filter by search term -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by search term</h4>
              <input 
                type="text" 
                v-model="filterConfig.search.term" 
                placeholder="Search..." 
                class="input input-bordered input-sm w-full mb-3" 
              />
              <div class="space-y-2">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.regex" class="checkbox checkbox-sm" />
                  <span class="text-sm">Regex</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.caseSensitive" class="checkbox checkbox-sm" />
                  <span class="text-sm">Case sensitive</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.search.negative" class="checkbox checkbox-sm" />
                  <span class="text-sm">Negative search</span>
                </label>
              </div>
            </div>

            <!-- Filter by file extension -->
            <div class="border border-base-300 rounded-lg p-3">
              <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by file extension</h4>
              <div class="space-y-3">
                <div class="form-control">
                  <label class="flex items-center gap-2 cursor-pointer mb-1">
                    <input type="checkbox" v-model="filterConfig.extension.showOnlyEnabled" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only:</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.extension.showOnly" 
                    placeholder="asp,aspx,jsp,php" 
                    class="input input-bordered input-sm w-full"
                    :disabled="!filterConfig.extension.showOnlyEnabled"
                  />
                </div>
                <div class="form-control">
                  <label class="flex items-center gap-2 cursor-pointer mb-1">
                    <input type="checkbox" v-model="filterConfig.extension.hideEnabled" class="checkbox checkbox-sm" />
                    <span class="text-sm">Hide:</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.extension.hide" 
                    placeholder="js,gif,jpg,png,css,ico,woff,woff2" 
                    class="input input-bordered input-sm w-full"
                    :disabled="!filterConfig.extension.hideEnabled"
                  />
                </div>
              </div>
            </div>

            <!-- Filter by annotation and listener -->
            <div class="space-y-4">
              <!-- Filter by annotation -->
              <div class="border border-base-300 rounded-lg p-3">
                <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by annotation</h4>
                <div class="space-y-2">
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" v-model="filterConfig.annotation.showOnlyWithNotes" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only items with notes</span>
                  </label>
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" v-model="filterConfig.annotation.showOnlyHighlighted" class="checkbox checkbox-sm" />
                    <span class="text-sm">Show only highlighted items</span>
                  </label>
                </div>
              </div>

              <!-- Filter by listener -->
              <div class="border border-base-300 rounded-lg p-3">
                <h4 class="text-sm font-semibold mb-3 text-base-content">Filter by listener</h4>
                <div class="form-control">
                  <label class="label py-0 pb-1">
                    <span class="label-text text-sm">Port</span>
                  </label>
                  <input 
                    type="text" 
                    v-model="filterConfig.listener.port" 
                    placeholder="e.g. 8080" 
                    class="input input-bordered input-sm w-full" 
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Bambda Mode Content -->
        <div v-else class="space-y-4">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span class="text-sm">Bambda mode allows you to write custom filter logic using JavaScript-like expressions.</span>
          </div>
          <textarea 
            class="textarea textarea-bordered w-full h-64 font-mono text-sm"
            placeholder="// Write your Bambda filter expression here&#10;// Example: request.url().contains('api') && response.statusCode() == 200"
            v-model="filterConfig.bambdaExpression"
          ></textarea>
          <div class="text-xs text-base-content/70">
            <p class="mb-1">Available variables:</p>
            <ul class="list-disc list-inside ml-2 space-y-0.5">
              <li><code class="bg-base-200 px-1 rounded">request</code> - Access request properties (url, method, headers, body)</li>
              <li><code class="bg-base-200 px-1 rounded">response</code> - Access response properties (statusCode, headers, body)</li>
              <li><code class="bg-base-200 px-1 rounded">annotations</code> - Access item annotations (notes, highlights)</li>
            </ul>
          </div>
        </div>

        <!-- 底部按钮 -->
        <div class="modal-action justify-between mt-6 pt-4 border-t border-base-300">
          <div class="flex gap-2">
            <button class="btn btn-sm btn-ghost" @click="showAllFilters">
              <i class="fas fa-eye mr-1"></i>
              Show all
            </button>
            <button class="btn btn-sm btn-ghost" @click="hideAllFilters">
              <i class="fas fa-eye-slash mr-1"></i>
              Hide all
            </button>
            <button class="btn btn-sm btn-ghost" @click="revertFilterChanges">
              <i class="fas fa-undo mr-1"></i>
              Revert changes
            </button>
          </div>
          <div class="flex gap-2 items-center">
            <button 
              v-if="filterMode === 'bambda'"
              class="btn btn-sm btn-ghost"
              @click="convertToBambda"
              title="Convert current settings to Bambda expression"
            >
              <i class="fas fa-exchange-alt mr-1"></i>
              Convert to Bambda
            </button>
            <button class="btn btn-sm" @click="closeFilterDialog">Cancel</button>
            <button class="btn btn-sm btn-primary" @click="applyFilterConfig">
              <i class="fas fa-check mr-1"></i>
              Apply
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>

    <!-- 筛选器工具栏 -->
    <div class="bg-base-100 border-b border-base-300 p-2 flex-shrink-0">
      <div class="flex items-center gap-2">
        <!-- 协议类型切换 -->
        <div class="tabs tabs-boxed tabs-xs bg-base-200 p-0.5">
          <a 
            class="tab tab-xs"
            :class="{ 'tab-active': protocolFilter === 'all' }"
            @click="protocolFilter = 'all'"
          >All</a>
          <a 
            class="tab tab-xs"
            :class="{ 'tab-active': protocolFilter === 'http' }"
            @click="protocolFilter = 'http'"
          >HTTP</a>
          <a 
            class="tab tab-xs"
            :class="{ 'tab-active': protocolFilter === 'websocket' }"
            @click="protocolFilter = 'websocket'"
          >
            <i class="fas fa-plug mr-1"></i>WS
          </a>
        </div>
        
        <!-- 筛选器状态按钮 -->
        <button 
          @click="openFilterDialog" 
          class="btn btn-sm btn-ghost gap-2 border border-base-300 flex-1 justify-start"
          :class="{ 'border-primary': hasActiveFilters }"
        >
          <i class="fas fa-filter" :class="{ 'text-primary': hasActiveFilters }"></i>
          <span class="text-xs truncate">{{ filterSummary }}</span>
        </button>
        
        <!-- 多选模式按钮 -->
        <button 
          @click="toggleMultiSelectMode" 
          class="btn btn-sm btn-ghost"
          :class="{ 'btn-active btn-primary': isMultiSelectMode }"
          :title="$t('trafficAnalysis.history.toolbar.filter')"
        >
          <i class="fas fa-check-square"></i>
        </button>
        
        <!-- 多选模式下的操作按钮 -->
        <template v-if="isMultiSelectMode">
          <button 
            @click="selectAllVisible" 
            class="btn btn-sm btn-ghost"
            :title="$t('trafficAnalysis.history.toolbar.clear')"
          >
            <i class="fas fa-check-double"></i>
          </button>
          <button 
            @click="clearSelection" 
            class="btn btn-sm btn-ghost"
            :title="$t('trafficAnalysis.history.toolbar.refresh')"
            :disabled="selectedRequests.size === 0"
          >
            <i class="fas fa-times"></i>
          </button>
          <div class="dropdown dropdown-end">
            <label 
              tabindex="0" 
              class="btn btn-sm btn-accent gap-1"
              :class="{ 'btn-disabled': selectedRequests.size === 0 }"
            >
              <i class="fas fa-brain"></i>
              <span class="text-xs">{{ $t('trafficAnalysis.history.toolbar.export') }} ({{ selectedRequests.size }})</span>
              <i class="fas fa-chevron-down text-xs"></i>
            </label>
            <ul tabindex="0" class="dropdown-content z-[100] menu p-2 shadow bg-base-100 rounded-box w-48">
              <li>
                <a @click="sendSelectedToAssistant('request')" class="flex items-center gap-2">
                  <i class="fas fa-upload text-accent"></i>
                  {{ $t('trafficAnalysis.history.contextMenu.sendRequestToAssistant') }}
                </a>
              </li>
              <li>
                <a @click="sendSelectedToAssistant('response')" class="flex items-center gap-2">
                  <i class="fas fa-download text-accent"></i>
                  {{ $t('trafficAnalysis.history.contextMenu.sendResponseToAssistant') }}
                </a>
              </li>
            </ul>
          </div>
        </template>
        
        <!-- 快捷操作按钮 -->
        <button @click="refreshRequests" class="btn btn-sm btn-ghost" :title="$t('trafficAnalysis.history.toolbar.refresh')">
          <i :class="['fas fa-sync-alt', { 'fa-spin': isLoading }]"></i>
        </button>
      </div>
    </div>

    <!-- 可调整大小的上下分割布局 -->
    <div ref="mainContainer" class="flex-1 flex flex-col min-h-0 overflow-hidden">
      <!-- 上半部分：请求历史列表 -->
      <div 
        ref="topPanel"
        class="bg-base-100 border-b border-base-300 overflow-hidden flex flex-col flex-shrink-0"
        :style="{ height: selectedRequest ? topPanelHeight + 'px' : '100%' }"
      >
        <div class="flex items-center justify-between px-4 py-2 border-b border-base-300 flex-shrink-0">
          <h3 class="font-semibold text-sm">
            <template v-if="protocolFilter === 'websocket'">
              <i class="fas fa-plug mr-2"></i>
              WebSocket Connections ({{ wsConnections.length }})
            </template>
            <template v-else>
              <i class="fas fa-history mr-2"></i>
              {{ $t('trafficAnalysis.history.title') }} ({{ filteredRequests.length }})
            </template>
          </h3>
          <div class="dropdown dropdown-end">
            <label tabindex="0" class="btn btn-xs btn-ghost">
              <i class="fas fa-cog"></i>
            </label>
            <div tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52 mt-2 max-h-96 overflow-y-auto">
              <li class="menu-title"><span>{{ $t('trafficAnalysis.history.table.actions') }}</span></li>
              <li v-for="col in columns" :key="col.id">
                <label class="label cursor-pointer justify-start gap-2">
                  <input type="checkbox" :checked="col.visible" @change="toggleColumn(col.id)" class="checkbox checkbox-xs" />
                  <span class="label-text text-xs">{{ col.label }}</span>
                </label>
              </li>
              <li><a @click="resetColumns" class="text-xs"><i class="fas fa-undo mr-1"></i>{{ $t('trafficAnalysis.history.filterDialog.reset') }}</a></li>
            </div>
          </div>
        </div>

        <!-- 虚拟滚动列表容器 -->
        <div 
          ref="scrollContainer" 
          class="flex-1 overflow-auto min-h-0"
          @scroll="handleScroll"
        >
          <div v-if="isLoading || isLoadingWs" class="flex items-center justify-center h-full">
            <i class="fas fa-spinner fa-spin text-2xl"></i>
          </div>

          <!-- WebSocket 连接列表 -->
          <div v-else-if="protocolFilter === 'websocket'" class="p-2">
            <div v-if="wsConnections.length === 0" class="flex flex-col items-center justify-center h-32 text-base-content/50">
              <i class="fas fa-plug text-3xl mb-2"></i>
              <span class="text-sm">No WebSocket connections yet</span>
            </div>
            <div v-else class="space-y-2">
              <div 
                v-for="conn in wsConnections" 
                :key="conn.id"
                class="bg-base-100 border border-base-300 rounded-lg overflow-hidden"
              >
                <!-- 连接头部 -->
                <div 
                  class="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-base-200"
                  @click="toggleWsConnection(conn.id)"
                >
                  <i 
                    class="fas fa-chevron-right transition-transform duration-200"
                    :class="{ 'rotate-90': expandedWsConnections.has(conn.id) }"
                  ></i>
                  <span 
                    class="badge badge-xs"
                    :class="{
                      'badge-success': conn.status === 'open',
                      'badge-ghost': conn.status === 'closed',
                      'badge-error': conn.status === 'error'
                    }"
                  >
                    {{ conn.status }}
                  </span>
                  <span class="text-xs font-mono text-primary truncate flex-1">{{ conn.url }}</span>
                  <span class="text-xs text-base-content/50">{{ conn.host }}</span>
                  <span class="text-xs text-base-content/40">{{ formatWsTime(conn.opened_at) }}</span>
                </div>
                
                <!-- 展开区域 -->
                <div 
                  v-if="expandedWsConnections.has(conn.id)"
                  class="border-t border-base-300 bg-base-200/30"
                >
                  <!-- 内部 Tabs -->
                  <div class="tabs tabs-boxed tabs-xs bg-transparent p-2 justify-start rounded-none border-b border-base-300/50">
                    <a 
                      class="tab tab-xs" 
                      :class="{ 'tab-active': getWsActiveTab(conn.id) === 'messages' }"
                      @click.stop="setWsActiveTab(conn.id, 'messages')"
                    >
                      Messages ({{ conn.message_ids?.length || 0 }})
                    </a>
                    <a 
                      class="tab tab-xs" 
                      :class="{ 'tab-active': getWsActiveTab(conn.id) === 'handshake' }"
                      @click.stop="setWsActiveTab(conn.id, 'handshake')"
                    >
                      Handshake
                    </a>
                  </div>

                  <!-- 消息列表 -->
                  <div v-if="getWsActiveTab(conn.id) === 'messages'" class="max-h-64 overflow-y-auto">
                    <div 
                      v-for="msg in getWsMessagesForConnection(conn.id)"
                      :key="msg.id"
                      class="flex items-start gap-2 px-4 py-1.5 text-xs border-b border-base-300/50 last:border-0 hover:bg-base-200/50 transition-colors"
                      :class="{
                        'bg-success/5': msg.direction === 'send',
                        'bg-info/5': msg.direction === 'receive'
                      }"
                    >
                      <i 
                        class="text-base"
                        :class="{
                          'fas fa-arrow-up text-success': msg.direction === 'send',
                          'fas fa-arrow-down text-info': msg.direction === 'receive'
                        }"
                        :title="msg.direction === 'send' ? $t('trafficAnalysis.history.websocket.toServer') : $t('trafficAnalysis.history.websocket.fromServer')"
                      ></i>
                      <span 
                        class="badge badge-xs font-mono"
                        :class="{
                          'badge-primary': msg.message_type === 'text',
                          'badge-secondary': msg.message_type === 'binary',
                          'badge-ghost': ['ping', 'pong'].includes(msg.message_type),
                          'badge-warning': msg.message_type === 'close'
                        }"
                      >{{ msg.message_type }}</span>
                      <div class="flex-1 min-w-0 font-mono break-all select-text">
                        {{ truncateWsContent(msg.content) }}
                      </div>
                      <span class="text-base-content/40 whitespace-nowrap">{{ msg.content_length }}B</span>
                      <span class="text-base-content/40 whitespace-nowrap">{{ formatWsTime(msg.timestamp) }}</span>
                    </div>
                    <div v-if="getWsMessagesForConnection(conn.id).length === 0" class="text-center py-4 text-base-content/50 text-xs">
                      No messages recorded yet
                    </div>
                  </div>

                  <!-- 握手详情 -->
                  <div v-else class="p-4 grid grid-cols-2 gap-4 text-xs font-mono">
                    <div class="bg-base-100 p-3 rounded border border-base-300">
                      <div class="font-bold mb-2 text-base-content/70">Request Headers</div>
                      <pre class="whitespace-pre-wrap break-all select-text overflow-x-auto max-h-48">{{ conn.request_headers || 'No headers captured' }}</pre>
                    </div>
                    <div class="bg-base-100 p-3 rounded border border-base-300">
                      <div class="font-bold mb-2 text-base-content/70">Response Headers</div>
                      <pre class="whitespace-pre-wrap break-all select-text overflow-x-auto max-h-48">{{ conn.response_headers || 'No headers captured' }}</pre>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- HTTP 请求列表 -->
          <div v-else-if="filteredRequests.length > 0" :style="{ height: totalHeight + 'px', position: 'relative' }">
            <!-- 表头 -->
            <div 
              class="sticky top-0 z-10 flex bg-base-200 border-b-2 border-base-300 font-semibold table-row-text"
              :style="{ height: headerHeight + 'px', minWidth: 'max-content' }"
            >
              <!-- 多选复选框列 -->
              <div 
                v-if="isMultiSelectMode"
                class="flex items-center justify-center px-2 border-r border-base-300"
                style="width: 40px; min-width: 40px;"
              >
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-xs checkbox-primary"
                  :checked="selectedRequests.size > 0 && selectedRequests.size === filteredRequests.length"
                  :indeterminate="selectedRequests.size > 0 && selectedRequests.size < filteredRequests.length"
                  @change="selectedRequests.size === filteredRequests.length ? clearSelection() : selectAllVisible()"
                />
              </div>
              <div 
                v-for="col in visibleColumns" 
                :key="col.id"
                class="flex items-center px-2 border-r border-base-300 relative"
                :style="{ width: col.width + 'px', minWidth: col.minWidth + 'px' }"
              >
                <span class="truncate">{{ col.label }}</span>
                <div 
                  class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30"
                  @mousedown="startResize(col.id, $event)"
                ></div>
              </div>
            </div>

            <!-- 数据行 -->
              <div 
                v-for="item in visibleItems" 
                :key="item.data.id"
                class="absolute left-0 right-0 flex hover:bg-base-200 cursor-pointer border-b border-base-300 table-row-text"
                :class="{ 
                  'bg-primary/10': selectedRequest?.id === item.data.id,
                  'bg-accent/10': isMultiSelectMode && isRequestSelected(item.data),
                  'bg-error/10 hover:bg-error/20': item.data.status_code === 0
                }"
                :style="{ 
                  top: (item.offset + headerHeight) + 'px', 
                  height: itemHeight + 'px',
                  minWidth: 'max-content'
                }"
                @click="isMultiSelectMode ? toggleSelectRequest(item.data) : (item.data.status_code === 0 ? showCertificateError(item.data) : selectRequest(item.data))"
                @contextmenu.prevent="showContextMenu($event, item.data)"
              >
              <!-- 多选复选框列 -->
              <div 
                v-if="isMultiSelectMode"
                class="flex items-center justify-center px-2 border-r border-base-300"
                style="width: 40px; min-width: 40px;"
                @click.stop="toggleSelectRequest(item.data)"
              >
                <input 
                  type="checkbox" 
                  class="checkbox checkbox-xs checkbox-accent"
                  :checked="isRequestSelected(item.data)"
                  @click.stop
                  @change="toggleSelectRequest(item.data)"
                />
              </div>
              <div 
                v-for="col in visibleColumns" 
                :key="col.id"
                class="flex items-center px-2 border-r border-base-300 overflow-hidden"
                :style="{ width: col.width + 'px', minWidth: col.minWidth + 'px' }"
              >
                <template v-if="col.id === 'method'">
                  <span :class="['badge badge-xs', getMethodClass(item.data.method)]">
                    {{ item.data.method }}
                  </span>
                </template>
                <template v-else-if="col.id === 'status'">
                  <div class="flex items-center gap-1">
                    <span :class="['badge badge-xs', getStatusClass(item.data.status_code)]" :title="getStatusTitle(item.data.status_code)">
                      {{ getStatusText(item.data.status_code) }}
                    </span>
                    <i 
                      v-if="item.data.status_code === 0" 
                      class="fas fa-exclamation-circle text-error text-xs cursor-help"
                      :title="$t('trafficAnalysis.history.certificateError.title')"
                      @click.stop="showCertificateError(item.data)"
                    ></i>
                  </div>
                </template>
                <template v-else-if="col.id === 'params'">
                  <span v-if="hasParams(item.data.url)" class="text-success">✓</span>
                </template>
                <template v-else-if="col.id === 'tls'">
                  <span v-if="item.data.protocol === 'https'" class="text-success">✓</span>
                </template>
                <template v-else>
                  <span class="truncate" :title="getColumnValue(item.data, col.id)">
                    {{ getColumnValue(item.data, col.id) }}
                  </span>
                </template>
              </div>
            </div>
          </div>

          <div v-else class="flex items-center justify-center h-full text-base-content/50">
            <div class="text-center">
              <i class="fas fa-inbox text-4xl mb-2"></i>
              <p>{{ $t('trafficAnalysis.history.emptyState.noRequests') }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- 水平分割条 -->
      <div 
        v-if="selectedRequest"
        ref="horizontalResizer"
        class="h-1 bg-base-300 cursor-row-resize hover:bg-primary/50 transition-colors flex-shrink-0"
        @mousedown="startHorizontalResize"
      ></div>

      <!-- 下半部分：请求/响应详情 -->
      <div 
        v-if="selectedRequest"
        ref="bottomPanel"
        class="bg-base-100 overflow-hidden flex flex-col flex-1 min-h-0"
      >
        <div class="flex items-center justify-between px-4 py-2 border-b border-base-300 flex-shrink-0">
          <h3 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.requestDetails') }} - ID: {{ selectedRequest.id }}</h3>
          <button @click="closeDetails" class="btn btn-xs btn-ghost">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="flex-1 flex min-h-0 overflow-hidden relative">
          <!-- 左侧：Request -->
          <div class="flex flex-col overflow-hidden" :style="{ width: leftPanelWidth + 'px' }">
            <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
              <div class="flex items-center gap-2">
                <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.request') }}</h4>
                <!-- Original/Edited 切换下拉 -->
                <div v-if="selectedRequest?.was_edited" class="dropdown dropdown-bottom">
                  <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
                    <span :class="requestViewMode === 'edited' ? 'text-warning' : ''">
                      {{ requestViewMode === 'original' ? $t('trafficAnalysis.history.detailsPanel.originalRequest') : $t('trafficAnalysis.history.detailsPanel.editedRequest') }}
                    </span>
                    <i class="fas fa-chevron-down text-xs"></i>
                  </label>
                  <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
                    <li>
                      <a 
                        :class="{ 'active': requestViewMode === 'original' }"
                        @click="requestViewMode = 'original'"
                      >
                        {{ $t('trafficAnalysis.history.detailsPanel.originalRequest') }}
                      </a>
                    </li>
                    <li>
                      <a 
                        :class="{ 'active': requestViewMode === 'edited' }"
                        @click="requestViewMode = 'edited'"
                      >
                        <span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedRequest') }}</span>
                      </a>
                    </li>
                  </ul>
                </div>
              </div>
              <div class="btn-group btn-group-xs">
                <button 
                  :class="['btn btn-xs', requestTab === 'pretty' ? 'btn-active' : '']"
                  @click="requestTab = 'pretty'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}
                </button>
                <button 
                  :class="['btn btn-xs', requestTab === 'raw' ? 'btn-active' : '']"
                  @click="requestTab = 'raw'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}
                </button>
                <button 
                  :class="['btn btn-xs', requestTab === 'hex' ? 'btn-active' : '']"
                  @click="requestTab = 'hex'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}
                </button>
              </div>
            </div>
            <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent="showDetailContextMenu($event)">
              <template v-if="requestTab !== 'hex'">
                <HttpCodeEditor
                  :modelValue="formatRequest(selectedRequest, requestTab, requestViewMode)"
                  :readonly="true"
                  height="100%"
                />
              </template>
              <template v-else>
                <div class="h-full overflow-auto p-2 font-mono text-xs bg-base-100">
                  <pre>{{ stringToHex(formatRequestRaw(selectedRequest, requestViewMode)) }}</pre>
                </div>
              </template>
            </div>
          </div>

          <!-- 垂直分割条 -->
          <div 
            ref="verticalResizer"
            class="w-1 bg-base-300 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0"
            @mousedown="startVerticalResize"
          ></div>

          <!-- 右侧：Response -->
          <div class="flex-1 flex flex-col overflow-hidden min-w-0">
            <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
              <div class="flex items-center gap-2">
                <h4 class="font-semibold text-sm">{{ $t('trafficAnalysis.history.detailsPanel.response') }}</h4>
                <span v-if="isResponseCompressed(selectedRequest)" class="badge badge-xs badge-info" title="响应已自动解压">
                  <i class="fas fa-file-archive mr-1"></i>{{ $t('trafficAnalysis.history.detailsPanel.decompressed') }}
                </span>
                <!-- Original/Edited 切换下拉 -->
                <div v-if="selectedRequest?.was_edited && hasEditedResponse(selectedRequest)" class="dropdown dropdown-bottom">
                  <label tabindex="0" class="btn btn-xs btn-ghost gap-1">
                    <span :class="responseViewMode === 'edited' ? 'text-warning' : ''">
                      {{ responseViewMode === 'original' ? $t('trafficAnalysis.history.detailsPanel.originalResponse') : $t('trafficAnalysis.history.detailsPanel.editedResponse') }}
                    </span>
                    <i class="fas fa-chevron-down text-xs"></i>
                  </label>
                  <ul tabindex="0" class="dropdown-content z-[1] menu p-1 shadow-lg bg-base-100 rounded-box w-40 border border-base-300">
                    <li>
                      <a 
                        :class="{ 'active': responseViewMode === 'original' }"
                        @click="responseViewMode = 'original'"
                      >
                        {{ $t('trafficAnalysis.history.detailsPanel.originalResponse') }}
                      </a>
                    </li>
                    <li>
                      <a 
                        :class="{ 'active': responseViewMode === 'edited' }"
                        @click="responseViewMode = 'edited'"
                      >
                        <span class="text-warning">{{ $t('trafficAnalysis.history.detailsPanel.editedResponse') }}</span>
                      </a>
                    </li>
                  </ul>
                </div>
              </div>
              <div class="btn-group btn-group-xs">
                <button 
                  :class="['btn btn-xs', responseTab === 'pretty' ? 'btn-active' : '']"
                  @click="responseTab = 'pretty'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.pretty') }}
                </button>
                <button 
                  :class="['btn btn-xs', responseTab === 'raw' ? 'btn-active' : '']"
                  @click="responseTab = 'raw'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.raw') }}
                </button>
                <button 
                  :class="['btn btn-xs', responseTab === 'hex' ? 'btn-active' : '']"
                  @click="responseTab = 'hex'"
                >
                  {{ $t('trafficAnalysis.history.detailsPanel.tabs.hex') }}
                </button>
              </div>
            </div>
            <div class="flex-1 overflow-hidden min-h-0" @contextmenu.prevent>
              <template v-if="responseTab !== 'hex'">
                <HttpCodeEditor
                  :modelValue="formatResponse(selectedRequest, responseTab, responseViewMode)"
                  :readonly="true"
                  height="100%"
                />
              </template>
              <template v-else>
                <div class="h-full overflow-auto p-2 font-mono text-xs bg-base-100">
                  <pre>{{ stringToHex(formatResponseRaw(selectedRequest, responseViewMode)) }}</pre>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch, inject } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit as tauriEmit } from '@tauri-apps/api/event';
import { useRouter } from 'vue-router';
import { dialog } from '@/composables/useDialog';
import HttpCodeEditor from '@/components/HttpCodeEditor.vue';

const router = useRouter();
const { t } = useI18n();

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 定义组件名称，用于 keep-alive
defineOptions({
  name: 'ProxyHistory'
});

// Emit 声明
const emit = defineEmits<{
  (e: 'sendToRepeater', request: { method: string; url: string; headers: Record<string, string>; body?: string }): void
  (e: 'sendToAssistant', requests: ProxyRequest[]): void
}>();

// 多选状态
const selectedRequests = ref<Set<number>>(new Set());
const isMultiSelectMode = ref(false);

// 历史列表右键菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  request: null as ProxyRequest | null,
});
const contextMenuRef = ref<HTMLElement | null>(null);

// 请求详情区域右键菜单状态
const detailContextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
});

// 类型定义
interface ProxyRequest {
  id: number;
  url: string;
  host: string;
  protocol: string;
  method: string;
  status_code: number;
  request_headers?: string;
  request_body?: string;
  response_headers?: string;
  response_body?: string;
  response_size: number;
  response_time: number;
  timestamp: string;
  ip?: string;
  listener?: string;
  extension?: string;
  title?: string;
  mime_type?: string;
  // Edited 字段（经过拦截修改后的数据）
  was_edited?: boolean;
  edited_request_headers?: string;
  edited_request_body?: string;
  edited_method?: string;
  edited_url?: string;
  edited_response_headers?: string;
  edited_response_body?: string;
  edited_status_code?: number;
}

interface VirtualItem {
  data: ProxyRequest;
  offset: number;
}

interface Column {
  id: string;
  label: string;
  visible: boolean;
  width: number;
  minWidth: number;
}

// WebSocket 连接记录
interface WebSocketConnection {
  id: string;
  url: string;
  host: string;
  protocol: string;
  request_headers?: string;
  response_headers?: string;
  status: 'open' | 'closed' | 'error';
  opened_at: string;
  closed_at?: string;
  close_code?: number;
  close_reason?: string;
  message_ids?: number[];
}

// WebSocket 消息记录
interface WebSocketMessage {
  id: number;
  connection_id: string;
  direction: 'send' | 'receive';
  message_type: 'text' | 'binary' | 'ping' | 'pong' | 'close';
  content?: string;
  content_length: number;
  timestamp: string;
}

// 响应式状态
const requests = ref<ProxyRequest[]>([]);
const selectedRequest = ref<ProxyRequest | null>(null);
// 协议类型过滤: 'all' | 'http' | 'websocket'
const protocolFilter = ref<'all' | 'http' | 'websocket'>('all');

// WebSocket 状态
const wsConnections = ref<WebSocketConnection[]>([]);
const wsMessages = ref<WebSocketMessage[]>([]);
const selectedWsConnection = ref<WebSocketConnection | null>(null);
const expandedWsConnections = ref<Set<string>>(new Set());
// WebSocket 连接内部 Tab 状态: connectionId -> 'messages' | 'handshake'
const activeWsTabs = ref<Map<string, 'messages' | 'handshake'>>(new Map());
const isLoadingWs = ref(false);

const showDetailsModal = ref(false);
const showColumnSettings = ref(false);
const isLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);

// 面板引用
const mainContainer = ref<HTMLElement | null>(null);
const topPanel = ref<HTMLElement | null>(null);
const bottomPanel = ref<HTMLElement | null>(null);
const horizontalResizer = ref<HTMLElement | null>(null);
const verticalResizer = ref<HTMLElement | null>(null);
let mainContainerResizeObserver: ResizeObserver | null = null;

// 面板尺寸（从 localStorage 恢复或使用默认值）
const STORAGE_KEY_TOP_HEIGHT = 'proxyHistory.topPanelHeight';
const STORAGE_KEY_LEFT_WIDTH = 'proxyHistory.leftPanelWidth';

const topPanelHeight = ref(300); // 上面板高度（会在 onMounted 中初始化）
const leftPanelWidth = ref(parseInt(localStorage.getItem(STORAGE_KEY_LEFT_WIDTH) || '600')); // 默认左面板宽度

// 拖拽状态
const isResizingHorizontal = ref(false);
const isResizingVertical = ref(false);
const resizeStartY = ref(0);
const resizeStartX = ref(0);
const resizeStartTopHeight = ref(0);
const resizeStartLeftWidth = ref(0);

// 分页和性能优化
const maxRequestsInMemory = 500; // 内存中最多保留 500 条记录
const initialLoadLimit = 100; // 初次加载 100 条
const batchUpdateThreshold = 5; // 批量更新阈值（降低以实现更实时的显示）
const loadMoreSize = 50; // 每次加载更多的数量
let pendingUpdates: ProxyRequest[] = []; // 待处理的新请求
let updateTimer: number | null = null; // 批量更新定时器
const hasMore = ref(true); // 是否还有更多数据
const isLoadingMore = ref(false); // 是否正在加载更多

// 详情面板的标签页
const requestTab = ref<'pretty' | 'raw' | 'hex'>('pretty');
const responseTab = ref<'pretty' | 'raw' | 'hex'>('pretty');

// Original/Edited 切换（类似 Burp Suite）
const requestViewMode = ref<'original' | 'edited'>('edited');
const responseViewMode = ref<'original' | 'edited'>('edited');

const stats = ref({
  total: 0,
  http: 0,
  https: 0,
  avgResponseTime: 0,
});

// 筛选器弹窗引用
const filterDialog = ref<HTMLDialogElement | null>(null);

// 证书错误弹窗引用
const certErrorDialog = ref<HTMLDialogElement | null>(null);
const certErrorInfo = ref<{host: string; url: string; error?: string} | null>(null);

// Filter mode
const filterMode = ref<'settings' | 'bambda'>('settings');

// 默认筛选器配置
const defaultFilterConfig = () => ({
  requestType: {
    showOnlyInScope: false,
    showOnlyWithParams: false,
    hideWithoutResponse: false,
  },
  mimeType: {
    html: true,
    script: true,
    xml: true,
    css: false,
    otherText: true,
    images: false,
    flash: true,
    otherBinary: false,
  },
  statusCode: {
    s2xx: true,
    s3xx: true,
    s4xx: true,
    s5xx: true,
  },
  search: {
    term: '',
    regex: false,
    caseSensitive: false,
    negative: false,
  },
  extension: {
    showOnlyEnabled: false,
    showOnly: '',
    hideEnabled: true,
    hide: 'js,gif,jpg,png,css,ico,woff,woff2,ttf,svg',
  },
  annotation: {
    showOnlyWithNotes: false,
    showOnlyHighlighted: false,
  },
  listener: {
    port: '',
  },
  bambdaExpression: '',
});

// 筛选器配置（可编辑）
const filterConfig = ref(defaultFilterConfig());
// 已应用的筛选器配置
const appliedFilterConfig = ref(defaultFilterConfig());
// 备份配置（用于 revert）
const backupFilterConfig = ref(defaultFilterConfig());

// 旧的 filters 保留用于兼容
const filters = ref({
  protocol: '',
  method: '',
  statusCode: '',
  search: '',
});

// 列配置（从 localStorage 恢复或使用默认值）
const STORAGE_KEY_COLUMNS = 'proxyHistory.columns';

const defaultColumns: Column[] = [
  { id: 'id', label: 'ID', visible: true, width: 60, minWidth: 50 },
  { id: 'host', label: 'Host', visible: true, width: 180, minWidth: 100 },
  { id: 'method', label: 'Method', visible: true, width: 80, minWidth: 60 },
  { id: 'url', label: 'URL', visible: true, width: 300, minWidth: 150 },
  { id: 'params', label: 'Params', visible: true, width: 70, minWidth: 60 },
  { id: 'status', label: 'Status', visible: true, width: 90, minWidth: 80 },
  { id: 'length', label: 'Length', visible: true, width: 80, minWidth: 60 },
  { id: 'mime', label: 'MIME Type', visible: true, width: 100, minWidth: 80 },
  { id: 'extension', label: 'Extension', visible: true, width: 90, minWidth: 70 },
  { id: 'title', label: 'Title', visible: true, width: 150, minWidth: 100 },
  { id: 'tls', label: 'TLS', visible: true, width: 60, minWidth: 50 },
  { id: 'ip', label: 'IP', visible: true, width: 120, minWidth: 100 },
  { id: 'time', label: 'Time', visible: true, width: 160, minWidth: 120 },
  { id: 'listener', label: 'Listener', visible: true, width: 100, minWidth: 80 },
  { id: 'responseTimer', label: 'Response Timer', visible: true, width: 120, minWidth: 100 },
];

// 从 localStorage 加载列配置
function loadColumnsFromStorage(): Column[] {
  try {
    const saved = localStorage.getItem(STORAGE_KEY_COLUMNS);
    if (saved) {
      const savedColumns = JSON.parse(saved) as Column[];
      // 合并保存的配置和默认配置（防止新增列丢失）
      return defaultColumns.map(defCol => {
        const savedCol = savedColumns.find(c => c.id === defCol.id);
        return savedCol ? { ...defCol, ...savedCol } : defCol;
      });
    }
  } catch (e) {
    console.error('Failed to load columns from storage:', e);
  }
  return defaultColumns;
}

const columns = ref<Column[]>(loadColumnsFromStorage());

// 列调整相关
const resizingColumn = ref<string | null>(null);
const columnResizeStartX = ref(0);
const resizeStartWidth = ref(0);

// 虚拟滚动相关
const itemHeight = 32;
const headerHeight = 34;
const scrollTop = ref(0);
const containerHeight = ref(600);
const bufferSize = 5;

// 事件监听器
let unlistenRequest: (() => void) | null = null;

// 计算属性
const filteredRequests = computed(() => {
  let result = requests.value;
  const config = appliedFilterConfig.value;

  // Filter by request type
  if (config.requestType.showOnlyWithParams) {
    result = result.filter(r => hasParams(r.url));
  }
  if (config.requestType.hideWithoutResponse) {
    result = result.filter(r => r.status_code > 0);
  }

  // Filter by status code
  result = result.filter(r => {
    const code = r.status_code;
    if (code === 0) return true; // 未收到响应的请求
    if (code >= 200 && code < 300) return config.statusCode.s2xx;
    if (code >= 300 && code < 400) return config.statusCode.s3xx;
    if (code >= 400 && code < 500) return config.statusCode.s4xx;
    if (code >= 500 && code < 600) return config.statusCode.s5xx;
    return true;
  });

  // Filter by MIME type
  result = result.filter(r => {
    const mime = getMimeTypeCategory(r);
    if (mime === 'html') return config.mimeType.html;
    if (mime === 'script') return config.mimeType.script;
    if (mime === 'xml') return config.mimeType.xml;
    if (mime === 'css') return config.mimeType.css;
    if (mime === 'image') return config.mimeType.images;
    if (mime === 'flash') return config.mimeType.flash;
    if (mime === 'text') return config.mimeType.otherText;
    if (mime === 'binary') return config.mimeType.otherBinary;
    return true; // 未知类型默认显示
  });

  // Filter by file extension
  if (config.extension.showOnlyEnabled && config.extension.showOnly) {
    const showExts = config.extension.showOnly.toLowerCase().split(',').map(e => e.trim());
    result = result.filter(r => {
      const ext = getExtension(r.url).toLowerCase();
      return !ext || showExts.includes(ext);
    });
  }
  if (config.extension.hideEnabled && config.extension.hide) {
    const hideExts = config.extension.hide.toLowerCase().split(',').map(e => e.trim());
    result = result.filter(r => {
      const ext = getExtension(r.url).toLowerCase();
      return !ext || !hideExts.includes(ext);
    });
  }

  // Filter by search term
  if (config.search.term) {
    const term = config.search.term;
    const caseSensitive = config.search.caseSensitive;
    const isRegex = config.search.regex;
    const isNegative = config.search.negative;
    
    result = result.filter(r => {
      const text = r.url + ' ' + r.host;
      let match = false;
      
      if (isRegex) {
        try {
          const regex = new RegExp(term, caseSensitive ? '' : 'i');
          match = regex.test(text);
        } catch {
          match = false;
        }
      } else {
        if (caseSensitive) {
          match = text.includes(term);
        } else {
          match = text.toLowerCase().includes(term.toLowerCase());
        }
      }
      
      return isNegative ? !match : match;
    });
  }

  // Filter by listener port
  if (config.listener.port) {
    result = result.filter(r => {
      const listenerPort = r.listener || '';
      return listenerPort.includes(config.listener.port);
    });
  }

  return result;
});

// 获取 MIME 类型分类
function getMimeTypeCategory(request: ProxyRequest): string {
  let contentType = '';
  
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers);
      contentType = (headers['content-type'] || headers['Content-Type'] || '').toLowerCase();
    } catch {
      // ignore
    }
  }
  
  if (!contentType) {
    // 根据扩展名推断
    const ext = getExtension(request.url).toLowerCase();
    const extMap: Record<string, string> = {
      'html': 'html', 'htm': 'html',
      'js': 'script', 'mjs': 'script',
      'xml': 'xml',
      'css': 'css',
      'png': 'image', 'jpg': 'image', 'jpeg': 'image', 'gif': 'image', 'webp': 'image', 'svg': 'image', 'ico': 'image',
      'swf': 'flash',
      'txt': 'text', 'json': 'text',
      'woff': 'binary', 'woff2': 'binary', 'ttf': 'binary', 'eot': 'binary', 'pdf': 'binary', 'zip': 'binary',
    };
    return extMap[ext] || 'unknown';
  }
  
  if (contentType.includes('html')) return 'html';
  if (contentType.includes('javascript') || contentType.includes('ecmascript')) return 'script';
  if (contentType.includes('xml')) return 'xml';
  if (contentType.includes('css')) return 'css';
  if (contentType.includes('image')) return 'image';
  if (contentType.includes('flash') || contentType.includes('shockwave')) return 'flash';
  if (contentType.includes('text') || contentType.includes('json')) return 'text';
  if (contentType.includes('octet-stream') || contentType.includes('binary') || 
      contentType.includes('font') || contentType.includes('application')) return 'binary';
  
  return 'unknown';
}

// 检查是否有激活的筛选器
const hasActiveFilters = computed(() => {
  const config = appliedFilterConfig.value;
  const def = defaultFilterConfig();
  
  // 检查是否与默认配置不同
  return (
    config.requestType.showOnlyWithParams !== def.requestType.showOnlyWithParams ||
    config.requestType.hideWithoutResponse !== def.requestType.hideWithoutResponse ||
    !config.statusCode.s2xx || !config.statusCode.s3xx || !config.statusCode.s4xx || !config.statusCode.s5xx ||
    !config.mimeType.html || !config.mimeType.script || !config.mimeType.xml ||
    config.mimeType.css || config.mimeType.images || !config.mimeType.otherText ||
    config.mimeType.otherBinary ||
    config.search.term !== '' ||
    config.extension.showOnlyEnabled ||
    config.listener.port !== ''
  );
});

// 筛选器摘要
const filterSummary = computed(() => {
  const config = appliedFilterConfig.value;
  const parts: string[] = [];
  
  // MIME 类型过滤
  const hiddenMime: string[] = [];
  if (!config.mimeType.css) hiddenMime.push('CSS');
  if (!config.mimeType.images) hiddenMime.push('image');
  if (!config.mimeType.otherBinary) hiddenMime.push('binary');
  
  if (hiddenMime.length > 0) {
    parts.push(`Hiding ${hiddenMime.join(', ')}`);
  }
  
  // 扩展名过滤
  if (config.extension.hideEnabled && config.extension.hide) {
    parts.push(`hiding extensions`);
  }
  
  if (config.search.term) {
    parts.push(`search: "${config.search.term}"`);
  }
  
  if (parts.length === 0) {
    return 'Filter settings: Showing all content';
  }
  
  return `Filter settings: ${parts.join(' and ')}`;
});

const translatedColumns = computed(() => {
  return columns.value.map(col => ({
    ...col,
    label: col.id === 'id' ? t('trafficAnalysis.history.table.id') :
           col.id === 'host' ? t('trafficAnalysis.history.table.host') :
           col.id === 'method' ? t('trafficAnalysis.history.table.method') :
           col.id === 'url' ? t('trafficAnalysis.history.table.url') :
           col.id === 'status' ? t('trafficAnalysis.history.table.status') :
           col.id === 'length' ? t('trafficAnalysis.history.table.length') :
           col.id === 'mime' ? t('trafficAnalysis.history.table.mimeType') :
           col.id === 'time' ? t('trafficAnalysis.history.table.time') :
           col.id === 'extension' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'title' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'tls' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'ip' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'listener' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'responseTimer' ? t('trafficAnalysis.history.table.actions') :
           col.id === 'params' ? t('trafficAnalysis.history.table.actions') : col.label
  }));
});

const visibleColumns = computed(() => {
  return translatedColumns.value.filter(col => col.visible);
});

// 虚拟滚动计算
const totalHeight = computed(() => {
  return filteredRequests.value.length * itemHeight + headerHeight;
});

const visibleItems = computed((): VirtualItem[] => {
  // 计算可见区域的起始和结束索引
  const startIndex = Math.max(0, Math.floor(scrollTop.value / itemHeight) - bufferSize);
  const visibleCount = Math.ceil(containerHeight.value / itemHeight);
  
  // 计算需要渲染的项目数量，确保覆盖可见区域 + 缓冲区
  const neededCount = visibleCount + bufferSize * 2;
  const endIndex = Math.min(filteredRequests.value.length, startIndex + neededCount);
  
  const items: VirtualItem[] = [];
  for (let i = startIndex; i < endIndex; i++) {
    items.push({
      data: filteredRequests.value[i],
      offset: i * itemHeight,
    });
  }
  
  return items;
});

// 方法
function handleScroll(event: Event) {
  const target = event.target as HTMLElement;
  scrollTop.value = target.scrollTop;

  const scrollHeight = target.scrollHeight;
  const clientHeight = target.clientHeight;
  const scrollBottom = scrollHeight - scrollTop.value - clientHeight;

  if (scrollBottom < 200 && hasMore.value && !isLoadingMore.value) {
    loadMoreRequests();
  }
}

// 更新容器高度
function updateContainerHeight() {
  if (scrollContainer.value) {
    containerHeight.value = scrollContainer.value.clientHeight;
  }
}

// 使用 ResizeObserver 监听容器大小变化
let resizeObserver: ResizeObserver | null = null;

function setupResizeObserver() {
  if (scrollContainer.value) {
    resizeObserver = new ResizeObserver(() => {
      updateContainerHeight();
    });
    resizeObserver.observe(scrollContainer.value);
  }
}

// 列调整相关方法
function startResize(columnId: string, event: MouseEvent) {
  event.preventDefault();
  resizingColumn.value = columnId;
  columnResizeStartX.value = event.clientX;
  const column = columns.value.find(col => col.id === columnId);
  if (column) {
    resizeStartWidth.value = column.width;
  }
  
  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
}

function handleResize(event: MouseEvent) {
  if (!resizingColumn.value) return;
  
  const column = columns.value.find(col => col.id === resizingColumn.value);
  if (!column) return;
  
  const diff = event.clientX - columnResizeStartX.value;
  const newWidth = Math.max(column.minWidth, resizeStartWidth.value + diff);
  column.width = newWidth;
}

function stopResize() {
  resizingColumn.value = null;
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  
  // 保存列配置到 localStorage
  localStorage.setItem(STORAGE_KEY_COLUMNS, JSON.stringify(columns.value));
}

// 水平分割条调整（上下面板）
function startHorizontalResize(event: MouseEvent) {
  event.preventDefault();
  isResizingHorizontal.value = true;
  resizeStartY.value = event.clientY;
  resizeStartTopHeight.value = topPanelHeight.value;
  
  document.addEventListener('mousemove', handleHorizontalResize);
  document.addEventListener('mouseup', stopHorizontalResize);
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';
}

function handleHorizontalResize(event: MouseEvent) {
  if (!isResizingHorizontal.value) return;
  
  const containerHeight = mainContainer.value?.clientHeight || 600;
  const diffY = event.clientY - resizeStartY.value;
  const newTopHeight = Math.max(150, Math.min(resizeStartTopHeight.value + diffY, containerHeight - 200));
  topPanelHeight.value = newTopHeight;
}

function stopHorizontalResize() {
  isResizingHorizontal.value = false;
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  // 保存尺寸到 localStorage
  localStorage.setItem(STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value));
}

// 垂直分割条调整（左右面板）
function startVerticalResize(event: MouseEvent) {
  event.preventDefault();
  isResizingVertical.value = true;
  resizeStartX.value = event.clientX;
  resizeStartLeftWidth.value = leftPanelWidth.value;
  
  document.addEventListener('mousemove', handleVerticalResize);
  document.addEventListener('mouseup', stopVerticalResize);
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function handleVerticalResize(event: MouseEvent) {
  if (!isResizingVertical.value) return;
  
  const diffX = event.clientX - resizeStartX.value;
  const containerWidth = bottomPanel.value?.clientWidth || 1200;
  const newLeftWidth = Math.max(300, Math.min(resizeStartLeftWidth.value + diffX, containerWidth - 300));
  leftPanelWidth.value = newLeftWidth;
}

function stopVerticalResize() {
  isResizingVertical.value = false;
  document.removeEventListener('mousemove', handleVerticalResize);
  document.removeEventListener('mouseup', stopVerticalResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  // 保存尺寸到 localStorage
  localStorage.setItem(STORAGE_KEY_LEFT_WIDTH, String(leftPanelWidth.value));
}

function toggleColumn(columnId: string) {
  const column = columns.value.find(col => col.id === columnId);
  if (column) {
    column.visible = !column.visible;
    // 保存列配置到 localStorage
    localStorage.setItem(STORAGE_KEY_COLUMNS, JSON.stringify(columns.value));
  }
}

function resetColumns() {
  columns.value = [...defaultColumns];
  // 清除保存的配置
  localStorage.removeItem(STORAGE_KEY_COLUMNS);
}

function getColumnValue(request: ProxyRequest, columnId: string): string {
  switch (columnId) {
    case 'id':
      return String(request.id);
    case 'host':
      return request.host;
    case 'method':
      return request.method;
    case 'url':
      return request.url;
    case 'params':
      return hasParams(request.url) ? '✓' : '';
    case 'status':
      return String(request.status_code);
    case 'length':
      return formatBytes(request.response_size);
    case 'mime':
      return request.mime_type || getMimeType(request);
    case 'extension':
      return request.extension || getExtension(request.url);
    case 'title':
      return request.title || '';
    case 'tls':
      return request.protocol === 'https' ? '✓' : '';
    case 'ip':
      return request.ip || '';
    case 'time':
      return formatTime(request.timestamp);
    case 'listener':
      return request.listener || 'Proxy';
    case 'responseTimer':
      return `${request.response_time}ms`;
    default:
      return '';
  }
}

function hasParams(url: string): boolean {
  try {
    const urlObj = new URL(url);
    return urlObj.search.length > 0;
  } catch {
    return url.includes('?');
  }
}

function getExtension(url: string): string {
  try {
    const urlObj = new URL(url);
    const pathname = urlObj.pathname;
    const parts = pathname.split('.');
    if (parts.length > 1) {
      const ext = parts[parts.length - 1].split(/[?#]/)[0];
      return ext.toLowerCase();
    }
    return '';
  } catch {
    return '';
  }
}

function getMimeType(request: ProxyRequest): string {
  // 尝试从响应头中获取 Content-Type
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers);
      const contentType = headers['content-type'] || headers['Content-Type'];
      if (contentType) {
        return contentType.split(';')[0].trim();
      }
    } catch {
      // 忽略解析错误
    }
  }
  
  // 根据扩展名推断
  const ext = getExtension(request.url);
  const mimeMap: Record<string, string> = {
    'html': 'HTML',
    'htm': 'HTML',
    'json': 'JSON',
    'xml': 'XML',
    'js': 'JavaScript',
    'css': 'CSS',
    'png': 'image',
    'jpg': 'image',
    'jpeg': 'image',
    'gif': 'image',
    'svg': 'image',
    'ico': 'image',
    'pdf': 'application/pdf',
    'txt': 'text',
  };
  
  return mimeMap[ext] || '';
}

function getMethodClass(method: string) {
  switch (method.toUpperCase()) {
    case 'GET': return 'badge-info';
    case 'POST': return 'badge-success';
    case 'PUT': return 'badge-warning';
    case 'DELETE': return 'badge-error';
    case 'PATCH': return 'badge-accent';
    default: return 'badge-ghost';
  }
}

function getStatusClass(statusCode: number) {
  if (statusCode === -1) return 'badge-warning'; // 等待响应（CONNECT 隧道）
  if (statusCode === 0) return 'badge-error'; // TLS 握手失败
  if (statusCode >= 200 && statusCode < 300) return 'badge-success';
  if (statusCode >= 300 && statusCode < 400) return 'badge-info';
  if (statusCode >= 400 && statusCode < 500) return 'badge-warning';
  if (statusCode >= 500) return 'badge-error';
  return 'badge-ghost';
}

function getStatusText(statusCode: number): string {
  if (statusCode === -1) return 'TUNNEL';
  if (statusCode === 0) return 'TLS ERR';
  return String(statusCode);
}

function getStatusTitle(statusCode: number): string {
  if (statusCode === -1) return 'HTTPS Tunnel (CONNECT) - TLS handshake may have failed';
  if (statusCode === 0) return 'TLS Handshake Failed - Certificate Error';
  return '';
}

// 显示证书错误弹窗
function showCertificateError(request: ProxyRequest) {
  certErrorInfo.value = {
    host: request.host,
    url: request.url,
    error: request.status_code === 0 ? 'TLS Handshake Failed' : 'Certificate validation error'
  };
  certErrorDialog.value?.showModal();
}

// 关闭证书错误弹窗
function closeCertErrorDialog() {
  certErrorDialog.value?.close();
  certErrorInfo.value = null;
}

// 检查CA证书安装
async function checkCAInstallation() {
  try {
    const response = await invoke<any>('export_root_ca');
    if (response.success && response.data) {
      dialog.toast.info(t('trafficAnalysis.history.certificateError.tips.installCA'));
      // 打开证书文件位置
      await invoke('show_in_folder', { path: response.data });
    }
  } catch (error: any) {
    console.error('Failed to check CA installation:', error);
    dialog.toast.error(`Failed to check certificate: ${error}`);
  }
}

function formatBytes(bytes: number) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

function formatHeaders(headers: string) {
  try {
    const parsed = JSON.parse(headers);
    return Object.entries(parsed)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n');
  } catch {
    return headers;
  }
}

function truncateText(text: string, maxLength: number) {
  if (!text) return '';
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + '...';
}

function formatTime(timestamp: string) {
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN');
}

async function refreshRequests() {
  isLoading.value = true;
  try {
    const response = await invoke<any>('list_proxy_requests', {
      limit: initialLoadLimit,
      offset: 0,
    });

    if (response.success && response.data) {
      requests.value = response.data;
      hasMore.value = response.data.length === initialLoadLimit;
      updateStats();
    }
  } catch (error: any) {
    console.error('Failed to refresh requests:', error);
    dialog.toast.error(`加载请求历史失败: ${error}`);
  } finally {
    isLoading.value = false;
  }
}

// 加载更多请求
async function loadMoreRequests() {
  if (isLoadingMore.value || !hasMore.value) return;
  
  isLoadingMore.value = true;
  try {
    const response = await invoke<any>('list_proxy_requests', {
      limit: loadMoreSize,
      offset: requests.value.length,
    });

    if (response.success && response.data) {
      if (response.data.length > 0) {
        requests.value = [...requests.value, ...response.data];
        hasMore.value = response.data.length === loadMoreSize;
        
        // 限制最大数量
        if (requests.value.length > maxRequestsInMemory) {
          requests.value = requests.value.slice(0, maxRequestsInMemory);
          hasMore.value = false;
        }
      } else {
        hasMore.value = false;
      }
    }
  } catch (error: any) {
    console.error('Failed to load more requests:', error);
    dialog.toast.error(`加载更多失败: ${error}`);
  } finally {
    isLoadingMore.value = false;
  }
}

// 优化统计计算 - 使用增量更新
function updateStats() {
  const total = requests.value.length;
  const https = requests.value.filter(r => r.protocol === 'https').length;
  const http = requests.value.filter(r => r.protocol === 'http').length;
  
  const totalResponseTime = requests.value.reduce((sum, r) => sum + r.response_time, 0);
  const avgResponseTime = total > 0 ? Math.round(totalResponseTime / total) : 0;

  stats.value = {
    total,
    http,
    https,
    avgResponseTime,
  };
}

// ========================================
// WebSocket 相关函数
// ========================================

// 切换 WebSocket 连接展开状态
function toggleWsConnection(connectionId: string) {
  if (expandedWsConnections.value.has(connectionId)) {
    expandedWsConnections.value.delete(connectionId);
  } else {
    expandedWsConnections.value.add(connectionId);
    // 默认选中 Messages 标签
    if (!activeWsTabs.value.has(connectionId)) {
      activeWsTabs.value.set(connectionId, 'messages');
    }
    // 加载该连接的消息
    loadWsMessages(connectionId);
  }
  // 触发响应式更新
  expandedWsConnections.value = new Set(expandedWsConnections.value);
}

// 获取 WebSocket 连接的当前活动标签页
function getWsActiveTab(connectionId: string): 'messages' | 'handshake' {
  return activeWsTabs.value.get(connectionId) || 'messages';
}

// 设置 WebSocket 连接的当前活动标签页
function setWsActiveTab(connectionId: string, tab: 'messages' | 'handshake') {
  activeWsTabs.value.set(connectionId, tab);
  // 触发响应式更新
  activeWsTabs.value = new Map(activeWsTabs.value);
}

// 格式化 WebSocket 时间
function formatWsTime(timestamp: string): string {
  try {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return timestamp;
  }
}

// 获取特定连接的消息
function getWsMessagesForConnection(connectionId: string): WebSocketMessage[] {
  return wsMessages.value.filter(msg => msg.connection_id === connectionId);
}

// 截断 WebSocket 消息内容
function truncateWsContent(content?: string, maxLength: number = 100): string {
  if (!content) return '[empty]';
  if (content.startsWith('[BASE64]')) {
    return '[binary data]';
  }
  if (content.length <= maxLength) return content;
  return content.substring(0, maxLength) + '...';
}

// 加载 WebSocket 连接列表
async function loadWsConnections() {
  isLoadingWs.value = true;
  try {
    const response = await invoke<any>('list_websocket_connections', {
      limit: 100,
      offset: 0,
    });
    
    if (response.success && response.data) {
      wsConnections.value = response.data;
    }
  } catch (error: any) {
    console.error('Failed to load WebSocket connections:', error);
  } finally {
    isLoadingWs.value = false;
  }
}

// 加载特定连接的消息
async function loadWsMessages(connectionId: string) {
  try {
    const response = await invoke<any>('list_websocket_messages', {
      connectionId,
      limit: 100,
      offset: 0,
    });
    
    if (response.success && response.data) {
      // 合并消息（避免重复）
      const existingIds = new Set(wsMessages.value.map(m => m.id));
      const newMessages = (response.data as WebSocketMessage[]).filter(m => !existingIds.has(m.id));
      wsMessages.value = [...wsMessages.value, ...newMessages];
    }
  } catch (error: any) {
    console.error('Failed to load WebSocket messages:', error);
  }
}

// 增量更新统计（用于新请求）
function updateStatsIncremental(newRequest: ProxyRequest) {
  stats.value.total++;
  if (newRequest.protocol === 'https') {
    stats.value.https++;
  } else if (newRequest.protocol === 'http') {
    stats.value.http++;
  }
  // 简单平均，不完全准确但足够快
  if (stats.value.total > 0) {
    stats.value.avgResponseTime = Math.round(
      (stats.value.avgResponseTime * (stats.value.total - 1) + newRequest.response_time) / stats.value.total
    );
  }
}



// 批量处理待更新的请求
function processPendingUpdates() {
  if (pendingUpdates.length === 0) return;
  
  console.log(`Processing ${pendingUpdates.length} pending updates`);
  
  // 去重：根据 ID 过滤掉已存在的请求
  const existingIds = new Set(requests.value.map(r => r.id));
  const newRequests = pendingUpdates.filter(req => !existingIds.has(req.id));
  
  if (newRequests.length > 0) {
    // 批量添加到列表头部
    requests.value = [...newRequests, ...requests.value];
    
    // 限制最大数量
    if (requests.value.length > maxRequestsInMemory) {
      requests.value = requests.value.slice(0, maxRequestsInMemory);
    }
    
    // 批量更新统计
    newRequests.forEach(req => updateStatsIncremental(req));
    
    console.log(`Added ${newRequests.length} new requests, total: ${requests.value.length}`);
  }
  
  // 清空待处理队列
  pendingUpdates = [];
  updateTimer = null;
}

function applyFilters() {
  scrollTop.value = 0;
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0;
  }
}


// 筛选器弹窗相关方法
function openFilterDialog() {
  // 备份当前配置
  backupFilterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
  // 复制应用的配置到编辑配置
  filterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
  filterDialog.value?.showModal();
}

function closeFilterDialog() {
  filterDialog.value?.close();
}

function applyFilterConfig() {
  // 应用配置
  appliedFilterConfig.value = JSON.parse(JSON.stringify(filterConfig.value));
  // 保存到 localStorage
  localStorage.setItem('proxyHistory.filterConfig', JSON.stringify(appliedFilterConfig.value));
  closeFilterDialog();
  applyFilters();
}

function revertFilterChanges() {
  filterConfig.value = JSON.parse(JSON.stringify(backupFilterConfig.value));
}

function showAllFilters() {
  filterConfig.value.mimeType = {
    html: true,
    script: true,
    xml: true,
    css: true,
    otherText: true,
    images: true,
    flash: true,
    otherBinary: true,
  };
  filterConfig.value.statusCode = {
    s2xx: true,
    s3xx: true,
    s4xx: true,
    s5xx: true,
  };
  filterConfig.value.extension.showOnlyEnabled = false;
  filterConfig.value.extension.hideEnabled = false;
  filterConfig.value.requestType.showOnlyInScope = false;
  filterConfig.value.requestType.showOnlyWithParams = false;
  filterConfig.value.requestType.hideWithoutResponse = false;
  filterConfig.value.annotation.showOnlyWithNotes = false;
  filterConfig.value.annotation.showOnlyHighlighted = false;
}

function hideAllFilters() {
  filterConfig.value.mimeType = {
    html: false,
    script: false,
    xml: false,
    css: false,
    otherText: false,
    images: false,
    flash: false,
    otherBinary: false,
  };
}

// Convert current settings to Bambda expression
function convertToBambda() {
  const conditions: string[] = [];
  
  // MIME type filters
  const enabledMimes = Object.entries(filterConfig.value.mimeType)
    .filter(([_, enabled]) => enabled)
    .map(([type]) => type);
  
  if (enabledMimes.length > 0 && enabledMimes.length < 8) {
    const mimeConditions = enabledMimes.map(type => {
      const mimeMap: Record<string, string> = {
        html: 'text/html',
        script: 'javascript',
        xml: 'xml',
        css: 'text/css',
        otherText: 'text/',
        images: 'image/',
        flash: 'flash',
        otherBinary: 'application/octet-stream'
      };
      return `response.mimeType().contains('${mimeMap[type]}')`;
    }).join(' || ');
    conditions.push(`(${mimeConditions})`);
  }
  
  // Status code filters
  const statusCodes: string[] = [];
  if (filterConfig.value.statusCode.s2xx) statusCodes.push('response.statusCode() >= 200 && response.statusCode() < 300');
  if (filterConfig.value.statusCode.s3xx) statusCodes.push('response.statusCode() >= 300 && response.statusCode() < 400');
  if (filterConfig.value.statusCode.s4xx) statusCodes.push('response.statusCode() >= 400 && response.statusCode() < 500');
  if (filterConfig.value.statusCode.s5xx) statusCodes.push('response.statusCode() >= 500 && response.statusCode() < 600');
  
  if (statusCodes.length > 0 && statusCodes.length < 4) {
    conditions.push(`(${statusCodes.join(' || ')})`);
  }
  
  // Search term
  if (filterConfig.value.search.term) {
    const term = filterConfig.value.search.term;
    const method = filterConfig.value.search.regex ? 'matches' : 'contains';
    const target = filterConfig.value.search.caseSensitive ? 'request.url()' : 'request.url().toLowerCase()';
    const searchTerm = filterConfig.value.search.caseSensitive ? term : term.toLowerCase();
    const condition = `${target}.${method}('${searchTerm}')`;
    conditions.push(filterConfig.value.search.negative ? `!${condition}` : condition);
  }
  
  // File extension
  if (filterConfig.value.extension.hideEnabled && filterConfig.value.extension.hide) {
    const exts = filterConfig.value.extension.hide.split(',').map(e => e.trim());
    const extConditions = exts.map(ext => `!request.url().endsWith('.${ext}')`).join(' && ');
    conditions.push(`(${extConditions})`);
  }
  
  filterConfig.value.bambdaExpression = conditions.length > 0 
    ? conditions.join(' && ') 
    : '// No filters configured';
    
  dialog.toast.success('Converted to Bambda expression');
}

// 加载保存的筛选器配置
function loadFilterConfig() {
  try {
    const saved = localStorage.getItem('proxyHistory.filterConfig');
    if (saved) {
      const parsed = JSON.parse(saved);
      // 合并默认配置和保存的配置
      appliedFilterConfig.value = { ...defaultFilterConfig(), ...parsed };
      filterConfig.value = JSON.parse(JSON.stringify(appliedFilterConfig.value));
    }
  } catch (e) {
    console.error('Failed to load filter config:', e);
  }
}

async function clearHistory() {
  const confirmed = await dialog.confirm('确定要清空所有请求历史吗？此操作不可恢复。');
  if (!confirmed) return;

  try {
    const response = await invoke<any>('clear_proxy_requests');
    if (response.success) {
      requests.value = [];
      updateStats();
      closeDetails();
      dialog.toast.success('请求历史已清空');
    }
  } catch (error: any) {
    console.error('Failed to clear requests:', error);
    dialog.toast.error(`清空失败: ${error}`);
  }
}

function openDetails(request: ProxyRequest) {
  selectedRequest.value = request;
  showDetailsModal.value = true;
}

function closeDetails() {
  showDetailsModal.value = false;
  selectedRequest.value = null;
  requestTab.value = 'pretty';
  responseTab.value = 'pretty';
}

function selectRequest(request: ProxyRequest) {
  if (selectedRequest.value?.id === request.id) {
    // 如果点击同一个请求，关闭详情
    closeDetails();
  } else {
    // 否则选中新请求并自动调整面板高度
    selectedRequest.value = request;
    requestTab.value = 'pretty';
    responseTab.value = 'pretty';
    
    // 自动调整面板高度比例（基于容器高度）
    const containerHeight = mainContainer.value?.clientHeight || 600;
    topPanelHeight.value = Math.floor(containerHeight * 0.4);
  }
}

// 右键菜单相关函数
function showContextMenu(event: MouseEvent, request: ProxyRequest) {
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    request,
  };
  
  // 添加点击外部关闭菜单的监听
  setTimeout(() => {
    document.addEventListener('click', hideContextMenu);
    document.addEventListener('contextmenu', hideContextMenu);
  }, 0);
}

function hideContextMenu() {
  contextMenu.value.visible = false;
  contextMenu.value.request = null;
  document.removeEventListener('click', hideContextMenu);
  document.removeEventListener('contextmenu', hideContextMenu);
}

// 请求详情区域右键菜单
function showDetailContextMenu(event: MouseEvent) {
  if (!selectedRequest.value) return;
  
  detailContextMenu.value = {
    visible: true,
    x: Math.min(event.clientX, window.innerWidth - 200),
    y: Math.min(event.clientY, window.innerHeight - 200),
  };
  
  setTimeout(() => {
    document.addEventListener('click', hideDetailContextMenu);
    document.addEventListener('contextmenu', hideDetailContextMenu);
  }, 0);
}

function hideDetailContextMenu() {
  detailContextMenu.value.visible = false;
  document.removeEventListener('click', hideDetailContextMenu);
  document.removeEventListener('contextmenu', hideDetailContextMenu);
}

function detailSendToRepeater() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  
  const req = selectedRequest.value;
  let headers: Record<string, string> = {};
  
  if (req.request_headers) {
    try {
      headers = JSON.parse(req.request_headers);
    } catch {
      // ignore
    }
  }
  
  emit('sendToRepeater', {
    method: req.method,
    url: req.url,
    headers,
    body: req.request_body || undefined,
  });
}

function detailCopyUrl() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  
  navigator.clipboard.writeText(selectedRequest.value.url)
    .then(() => dialog.toast.success('URL 已复制'))
    .catch(() => dialog.toast.error('复制失败'));
}

function detailCopyRequest() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  
  const requestText = formatRequest(selectedRequest.value, requestTab.value, requestViewMode.value);
  navigator.clipboard.writeText(requestText)
    .then(() => dialog.toast.success('请求已复制'))
    .catch(() => dialog.toast.error('复制失败'));
}

function detailCopyAsCurl() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  
  const req = selectedRequest.value;
  let curl = `curl -X ${req.method} '${req.url}'`;
  
  if (req.request_headers) {
    try {
      const headers = JSON.parse(req.request_headers);
      for (const [key, value] of Object.entries(headers)) {
        curl += ` \\\n  -H '${key}: ${value}'`;
      }
    } catch {
      // ignore
    }
  }
  
  if (req.request_body) {
    curl += ` \\\n  -d '${req.request_body.replace(/'/g, "'\\''")}'`;
  }
  
  navigator.clipboard.writeText(curl)
    .then(() => dialog.toast.success('cURL 命令已复制'))
    .catch(() => dialog.toast.error('复制失败'));
}

function sendToRepeater() {
  if (!contextMenu.value.request) return;
  
  const req = contextMenu.value.request;
  let headers: Record<string, string> = {};
  
  if (req.request_headers) {
    try {
      headers = JSON.parse(req.request_headers);
    } catch {
      // 忽略解析错误
    }
  }
  
  emit('sendToRepeater', {
    method: req.method,
    url: req.url,
    headers,
    body: req.request_body || undefined,
  });
  
  hideContextMenu();
}

function copyUrl() {
  if (!contextMenu.value.request) return;
  
  navigator.clipboard.writeText(contextMenu.value.request.url)
    .then(() => dialog.toast.success('URL 已复制'))
    .catch(() => dialog.toast.error('复制失败'));
  
  hideContextMenu();
}

function copyAsCurl() {
  if (!contextMenu.value.request) return;
  
  const req = contextMenu.value.request;
  let curl = `curl -X ${req.method} '${req.url}'`;
  
  if (req.request_headers) {
    try {
      const headers = JSON.parse(req.request_headers);
      for (const [key, value] of Object.entries(headers)) {
        curl += ` \\\n  -H '${key}: ${value}'`;
      }
    } catch {
      // 忽略解析错误
    }
  }
  
  if (req.request_body) {
    curl += ` \\\n  -d '${req.request_body.replace(/'/g, "'\\''")}'`;
  }
  
  navigator.clipboard.writeText(curl)
    .then(() => dialog.toast.success('cURL 命令已复制'))
    .catch(() => dialog.toast.error('复制失败'));
  
  hideContextMenu();
}

function openInBrowser() {
  if (!contextMenu.value.request) return;
  
  window.open(contextMenu.value.request.url, '_blank');
  hideContextMenu();
}

function clearHistoryFromMenu() {
  hideContextMenu();
  clearHistory();
}

// 多选相关方法
function toggleMultiSelectMode() {
  isMultiSelectMode.value = !isMultiSelectMode.value;
  if (!isMultiSelectMode.value) {
    selectedRequests.value.clear();
  }
}

function toggleSelectRequest(request: ProxyRequest) {
  if (selectedRequests.value.has(request.id)) {
    selectedRequests.value.delete(request.id);
  } else {
    selectedRequests.value.add(request.id);
  }
}

function selectAllVisible() {
  filteredRequests.value.forEach(req => {
    selectedRequests.value.add(req.id);
  });
}

function clearSelection() {
  selectedRequests.value.clear();
}

function isRequestSelected(request: ProxyRequest): boolean {
  return selectedRequests.value.has(request.id);
}

// 发送到 AI 助手 - type: 'request' | 'response' | 'both'
type SendType = 'request' | 'response' | 'both';

async function sendSelectedToAssistant(type: SendType = 'both') {
  const selected = filteredRequests.value.filter(req => selectedRequests.value.has(req.id));
  if (selected.length === 0) {
    dialog.toast.warning('请先选择要发送的请求');
    return;
  }
  
  // 发送全局事件通知 AI 助手
  await tauriEmit('traffic:send-to-assistant', { requests: selected, type });
  emit('sendToAssistant', selected);
  
  const typeText = type === 'request' ? '请求' : type === 'response' ? '响应' : '流量';
  dialog.toast.success(`已发送 ${selected.length} 条${typeText}到 AI 助手`);
  
  // 清除选择
  selectedRequests.value.clear();
  isMultiSelectMode.value = false;
  
  // 跳转到 AI 助手页面
  router.push('/ai-assistant');
}

async function sendSingleToAssistant(request: ProxyRequest, type: SendType = 'both') {
  // 发送全局事件通知 AI 助手
  await tauriEmit('traffic:send-to-assistant', { requests: [request], type });
  emit('sendToAssistant', [request]);
  
  const typeText = type === 'request' ? '请求' : type === 'response' ? '响应' : '流量';
  dialog.toast.success(`已发送${typeText}到 AI 助手`);
  
  // 跳转到 AI 助手页面
  router.push('/ai-assistant');
}

function sendRequestToAssistantFromMenu() {
  if (!contextMenu.value.request) return;
  sendSingleToAssistant(contextMenu.value.request, 'request');
  hideContextMenu();
}

function sendResponseToAssistantFromMenu() {
  if (!contextMenu.value.request) return;
  sendSingleToAssistant(contextMenu.value.request, 'response');
  hideContextMenu();
}

function detailSendRequestToAssistant() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  sendSingleToAssistant(selectedRequest.value, 'request');
}

function detailSendResponseToAssistant() {
  hideDetailContextMenu();
  if (!selectedRequest.value) return;
  sendSingleToAssistant(selectedRequest.value, 'response');
}

function formatRequest(request: ProxyRequest, tab: string, viewMode: 'original' | 'edited' = 'edited'): string {
  if (tab === 'hex') {
    return stringToHex(formatRequestRaw(request, viewMode));
  }
  
  if (tab === 'raw') {
    return formatRequestRaw(request, viewMode);
  }
  
  // 根据 viewMode 选择使用原始或修改后的数据
  const useEdited = viewMode === 'edited' && request.was_edited;
  const method = useEdited && request.edited_method ? request.edited_method : request.method;
  const url = useEdited && request.edited_url ? request.edited_url : request.url;
  const headers = useEdited && request.edited_request_headers ? request.edited_request_headers : request.request_headers;
  const body = useEdited && request.edited_request_body ? request.edited_request_body : request.request_body;
  
  // Pretty format - 从完整URL提取路径
  const requestPath = getRequestPath(url);
  let result = `${method} ${requestPath} HTTP/1.1\n`;
  const hostValue = request.host || getHostFromUrl(url);
  if (hostValue) result += `Host: ${hostValue}\n`;
  result += formatHeaderBlock(headers, { skipHost: !!hostValue });
  
  if (body) {
    result += '\n';
    // JSON body 格式化显示
    result += formatJsonBody(body);
  }
  
  return result;
}

function formatRequestRaw(request: ProxyRequest, viewMode: 'original' | 'edited' = 'edited'): string {
  // 根据 viewMode 选择使用原始或修改后的数据
  const useEdited = viewMode === 'edited' && request.was_edited;
  const method = useEdited && request.edited_method ? request.edited_method : request.method;
  const url = useEdited && request.edited_url ? request.edited_url : request.url;
  const headers = useEdited && request.edited_request_headers ? request.edited_request_headers : request.request_headers;
  const body = useEdited && request.edited_request_body ? request.edited_request_body : request.request_body;
  
  // 从完整URL提取路径
  const requestPath = getRequestPath(url);
  let result = `${method} ${requestPath} HTTP/1.1\n`;
  const hostValue = request.host || getHostFromUrl(url);
  if (hostValue) result += `Host: ${hostValue}\n`;
  // Raw tab should still be an HTTP-like text block; if stored headers are JSON, render as header lines.
  result += formatHeaderBlock(headers, { skipHost: !!hostValue });
  
  if (body) {
    result += '\n' + body;
  }
  
  return result;
}

// 从完整URL提取路径部分
function getRequestPath(url: string): string {
  try {
    const urlObj = new URL(url);
    return urlObj.pathname + urlObj.search || '/';
  } catch {
    // 如果URL解析失败，尝试提取路径部分
    const match = url.match(/^https?:\/\/[^/]+(\/.*)?$/);
    if (match) {
      return match[1] || '/';
    }
    return url;
  }
}

function getHostFromUrl(url: string): string | null {
  try {
    return new URL(url).host || null;
  } catch {
    return null;
  }
}

function formatHeaderBlock(
  headersJsonOrRaw: string | undefined,
  opts: { skipHost?: boolean } = {}
): string {
  if (!headersJsonOrRaw) return '';
  const skip = new Set<string>();
  if (opts.skipHost) skip.add('host');

  // Prefer JSON -> header lines (avoid showing JSON in "raw" tab)
  try {
    const parsed = JSON.parse(headersJsonOrRaw);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      let out = '';
      for (const [k, v] of Object.entries(parsed as Record<string, any>)) {
        const key = String(k);
        const keyLower = key.toLowerCase();
        if (skip.has(keyLower)) continue;
        if (Array.isArray(v)) {
          for (const item of v) out += `${key}: ${String(item)}\n`;
        } else {
          out += `${key}: ${String(v)}\n`;
        }
      }
      return out;
    }
  } catch {
    // fallthrough
  }

  const lines = headersJsonOrRaw
    .split(/\r?\n/)
    .map(l => l.trimEnd())
    .filter(l => l.trim().length > 0)
    .filter(line => {
      const idx = line.indexOf(':');
      if (idx <= 0) return true;
      const name = line.slice(0, idx).trim().toLowerCase();
      return !skip.has(name);
    });

  return lines.length ? lines.join('\n') + '\n' : '';
}

// 格式化JSON body
function formatJsonBody(body: string): string {
  if (!body) return '';
  try {
    const json = JSON.parse(body);
    return JSON.stringify(json, null, 2);
  } catch {
    return body;
  }
}

function formatResponse(request: ProxyRequest, tab: string, viewMode: 'original' | 'edited' = 'edited'): string {
  if (tab === 'hex') {
    return stringToHex(formatResponseRaw(request, viewMode));
  }
  
  if (tab === 'raw') {
    return formatResponseRaw(request, viewMode);
  }
  
  // 根据 viewMode 选择使用原始或修改后的数据
  const useEdited = viewMode === 'edited' && request.was_edited;
  const statusCode = useEdited && request.edited_status_code ? request.edited_status_code : request.status_code;
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers;
  const body = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body;
  
  // Pretty format
  let result = `HTTP/1.2 ${statusCode} OK\n`;
  result += formatHeaderBlock(headers);
  
  if (body) {
    result += '\n';
    
    // 检测内容类型
    const contentType = getResponseContentType(request, viewMode);
    
    // 尝试根据 Content-Type 格式化
    if (contentType.includes('json') || contentType.includes('application/json')) {
      try {
        const json = JSON.parse(body);
        result += JSON.stringify(json, null, 2);
      } catch {
        result += body;
      }
    } else if (contentType.includes('html') || contentType.includes('xml')) {
      // HTML/XML 直接显示
      result += body;
    } else if (contentType.includes('text/')) {
      // 其他文本类型
      result += body;
    } else {
      // 二进制或未知类型
      const bodySize = new Blob([body]).size;
      result += `[Binary data - ${formatBytes(bodySize)}]\n`;
      result += `Content-Type: ${contentType}\n`;
      result += `\nFirst 200 characters:\n${body.substring(0, 200)}...`;
    }
  }
  
  return result;
}

function getResponseContentType(request: ProxyRequest, viewMode: 'original' | 'edited' = 'edited'): string {
  const useEdited = viewMode === 'edited' && request.was_edited;
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers;
  
  if (headers) {
    try {
      const parsed = JSON.parse(headers);
      return parsed['content-type'] || parsed['Content-Type'] || '';
    } catch {
      return '';
    }
  }
  return '';
}

function isResponseCompressed(request: ProxyRequest): boolean {
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers);
      const encoding = headers['content-encoding'] || headers['Content-Encoding'];
      return encoding && (encoding.includes('gzip') || encoding.includes('br') || encoding.includes('deflate'));
    } catch {
      return false;
    }
  }
  return false;
}

function formatResponseRaw(request: ProxyRequest, viewMode: 'original' | 'edited' = 'edited'): string {
  // 根据 viewMode 选择使用原始或修改后的数据
  const useEdited = viewMode === 'edited' && request.was_edited;
  const statusCode = useEdited && request.edited_status_code ? request.edited_status_code : request.status_code;
  const headers = useEdited && request.edited_response_headers ? request.edited_response_headers : request.response_headers;
  const body = useEdited && request.edited_response_body ? request.edited_response_body : request.response_body;
  
  let result = `HTTP/1.2 ${statusCode} OK\n`;
  result += formatHeaderBlock(headers);
  
  if (body) {
    result += '\n' + body;
  }
  
  return result;
}

// 检查响应是否有 edited 数据
function hasEditedResponse(request: ProxyRequest): boolean {
  return !!(request.edited_response_headers || request.edited_response_body || request.edited_status_code);
}

function stringToHex(str: string): string {
  let hex = '';
  for (let i = 0; i < str.length; i++) {
    const charCode = str.charCodeAt(i);
    const hexValue = charCode.toString(16).padStart(2, '0');
    hex += hexValue + ' ';
    
    if ((i + 1) % 16 === 0) {
      hex += '\n';
    }
  }
  return hex;
}

async function setupEventListeners() {
  // 监听新的代理请求事件
  unlistenRequest = await listen<ProxyRequest>('proxy:request', (event) => {
    console.log('Received proxy request event:', event.payload);
    
    // 添加到待处理队列
    pendingUpdates.push(event.payload);
    
    // 如果达到批量更新阈值，立即处理
    if (pendingUpdates.length >= batchUpdateThreshold) {
      if (updateTimer !== null) {
        clearTimeout(updateTimer);
      }
      processPendingUpdates();
    } 
    // 否则设置定时器，在 50ms 后批量处理（降低延迟）
    else if (updateTimer === null) {
      updateTimer = window.setTimeout(() => {
        processPendingUpdates();
      }, 50);
    }
  });
}

// 初始化面板高度（基于容器实际高度）
function initPanelHeights() {
  const containerHeight = mainContainer.value?.clientHeight || 600;
  const savedTopHeight = localStorage.getItem(STORAGE_KEY_TOP_HEIGHT);
  
  if (savedTopHeight) {
    // 确保保存的值不超过容器高度
    topPanelHeight.value = Math.min(parseInt(savedTopHeight), containerHeight - 200);
  } else {
    topPanelHeight.value = Math.floor(containerHeight * 0.4);
  }
  
  // 初始化左面板宽度
  const containerWidth = mainContainer.value?.clientWidth || 1200;
  if (!localStorage.getItem(STORAGE_KEY_LEFT_WIDTH)) {
    leftPanelWidth.value = Math.floor(containerWidth * 0.5);
  }
}

// 设置主容器的 ResizeObserver
function setupMainContainerResizeObserver() {
  if (mainContainer.value) {
    mainContainerResizeObserver = new ResizeObserver(() => {
      // 当容器大小改变时，重新初始化面板高度
      if (selectedRequest.value) {
        const containerHeight = mainContainer.value?.clientHeight || 600;
        // 确保 topPanelHeight 不超过容器高度
        if (topPanelHeight.value > containerHeight - 200) {
          topPanelHeight.value = Math.floor(containerHeight * 0.4);
        }
      }
      updateContainerHeight();
    });
    mainContainerResizeObserver.observe(mainContainer.value);
  }
}

// 键盘快捷键处理
function handleKeydown(event: KeyboardEvent) {
  // Cmd/Ctrl + R 发送到 Repeater
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'r') {
    // 如果有选中的请求，发送到 Repeater
    if (selectedRequest.value) {
      event.preventDefault();
      event.stopPropagation();
      
      const req = selectedRequest.value;
      let headers: Record<string, string> = {};
      
      if (req.request_headers) {
        try {
          headers = JSON.parse(req.request_headers);
        } catch {
          // ignore
        }
      }
      
      emit('sendToRepeater', {
        method: req.method,
        url: req.url,
        headers,
        body: req.request_body || undefined,
      });
    }
  }
}

// 生命周期
onMounted(async () => {
  // 加载筛选器配置
  loadFilterConfig();
  
  await setupEventListeners();
  await refreshRequests();
  
  // 等待 DOM 渲染完成后再设置 ResizeObserver
  await nextTick();
  updateContainerHeight();
  setupResizeObserver();
  setupMainContainerResizeObserver();
  initPanelHeights();
  
  // 添加键盘快捷键监听
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  if (unlistenRequest) unlistenRequest();
  if (resizeObserver && scrollContainer.value) {
    resizeObserver.unobserve(scrollContainer.value);
    resizeObserver.disconnect();
  }
  if (mainContainerResizeObserver) {
    mainContainerResizeObserver.disconnect();
  }
  if (updateTimer !== null) {
    clearTimeout(updateTimer);
  }
  // 清理拖拽事件监听
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.removeEventListener('mousemove', handleVerticalResize);
  document.removeEventListener('mouseup', stopVerticalResize);
  // 清理键盘快捷键监听
  document.removeEventListener('keydown', handleKeydown);
});

// 监听详情面板的打开/关闭，更新容器高度
watch(selectedRequest, async () => {
  await nextTick();
  updateContainerHeight();
});

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyHistory] Refresh triggered by parent');
  await refreshRequests();
});

// 监听协议类型切换
// 监听协议类型切换
watch(protocolFilter, async (newFilter) => {
  console.log('[ProxyHistory] Protocol filter changed to:', newFilter);
  
  // 清除详情面板选择，避免混淆
  selectedRequest.value = null;
  
  if (newFilter === 'websocket') {
    // 切换到 WebSocket 时加载连接列表
    await loadWsConnections();
  } else {
    // 切换到 HTTP 时刷新请求
    await refreshRequests();
  }
});

// 设置 WebSocket 事件监听
let unlistenWsConnection: (() => void) | null = null;
let unlistenWsMessage: (() => void) | null = null;

async function setupWsEventListeners() {
  try {
    unlistenWsConnection = await listen<any>('proxy:websocket_connection', (event) => {
      console.log('[ProxyHistory] WebSocket connection event:', event.payload);
      // 添加到连接列表
      const conn = event.payload as WebSocketConnection;
      // 确保 message_ids 字段存在
      if (!conn.message_ids) {
        conn.message_ids = [];
      }
      const existingIndex = wsConnections.value.findIndex(c => c.id === conn.id);
      if (existingIndex >= 0) {
        wsConnections.value[existingIndex] = conn;
      } else {
        wsConnections.value.unshift(conn);
      }
    });

    unlistenWsMessage = await listen<any>('proxy:websocket_message', (event) => {
      console.log('[ProxyHistory] WebSocket message event:', event.payload);
      // 添加到消息列表
      const msg = event.payload as WebSocketMessage;
      wsMessages.value.push(msg);
    });
  } catch (error) {
    console.error('Failed to setup WebSocket event listeners:', error);
  }
}

// 在 onMounted 时也设置 WebSocket 事件
onMounted(async () => {
  await setupWsEventListeners();
});

// 在 onUnmounted 时清理 WebSocket 事件
onUnmounted(() => {
  if (unlistenWsConnection) unlistenWsConnection();
  if (unlistenWsMessage) unlistenWsMessage();
});
</script>

<style scoped>
/* 表格行文字使用系统字体大小 */
.table-row-text {
  font-size: var(--font-size-base, 14px);
}

pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
