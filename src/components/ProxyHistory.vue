<template>
  <div class="flex flex-col h-screen bg-base-200">
    <!-- 右键菜单 -->
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
        Send to Repeater
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="copyUrl"
      >
        <i class="fas fa-link text-info"></i>
        Copy URL
      </button>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="copyAsCurl"
      >
        <i class="fas fa-terminal text-warning"></i>
        Copy as cURL
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
        @click="openInBrowser"
      >
        <i class="fas fa-external-link-alt text-success"></i>
        Open in Browser
      </button>
      <div class="divider my-1 h-0"></div>
      <button 
        class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2 text-error"
        @click="clearHistoryFromMenu"
      >
        <i class="fas fa-trash"></i>
        Clear History
      </button>
    </div>

    <!-- 筛选器配置弹窗 -->
    <dialog ref="filterDialog" class="modal">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">Configure HTTP Proxy filter</h3>
        
        <div class="grid grid-cols-4 gap-4">
          <!-- Filter by request type -->
          <div class="border border-base-300 rounded-lg p-3">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by request type</h4>
            <div class="space-y-1">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.requestType.showOnlyWithParams" class="checkbox checkbox-xs" />
                <span class="text-xs">Show only parameterized requests</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.requestType.hideWithoutResponse" class="checkbox checkbox-xs" />
                <span class="text-xs">Hide items without responses</span>
              </label>
            </div>
          </div>

          <!-- Filter by MIME type -->
          <div class="border border-base-300 rounded-lg p-3">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by MIME type</h4>
            <div class="grid grid-cols-2 gap-x-2 gap-y-1">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.html" class="checkbox checkbox-xs" />
                <span class="text-xs">HTML</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.otherText" class="checkbox checkbox-xs" />
                <span class="text-xs">Other text</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.script" class="checkbox checkbox-xs" />
                <span class="text-xs">Script</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.images" class="checkbox checkbox-xs" />
                <span class="text-xs">Images</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.xml" class="checkbox checkbox-xs" />
                <span class="text-xs">XML</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.flash" class="checkbox checkbox-xs" />
                <span class="text-xs">Flash</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.css" class="checkbox checkbox-xs" />
                <span class="text-xs">CSS</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.mimeType.otherBinary" class="checkbox checkbox-xs" />
                <span class="text-xs">Other binary</span>
              </label>
            </div>
          </div>

          <!-- Filter by status code -->
          <div class="border border-base-300 rounded-lg p-3">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by status code</h4>
            <div class="space-y-1">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.statusCode.s2xx" class="checkbox checkbox-xs" />
                <span class="text-xs">2xx [success]</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.statusCode.s3xx" class="checkbox checkbox-xs" />
                <span class="text-xs">3xx [redirection]</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.statusCode.s4xx" class="checkbox checkbox-xs" />
                <span class="text-xs">4xx [request error]</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.statusCode.s5xx" class="checkbox checkbox-xs" />
                <span class="text-xs">5xx [server error]</span>
              </label>
            </div>
          </div>

          <!-- Filter by listener -->
          <div class="border border-base-300 rounded-lg p-3">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by listener</h4>
            <div class="form-control">
              <label class="label py-0">
                <span class="label-text text-xs">Port</span>
              </label>
              <input type="text" v-model="filterConfig.listener.port" placeholder="e.g. 8080" class="input input-bordered input-xs w-full" />
            </div>
          </div>

          <!-- Filter by search term -->
          <div class="border border-base-300 rounded-lg p-3 col-span-2">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by search term</h4>
            <input type="text" v-model="filterConfig.search.term" placeholder="Search..." class="input input-bordered input-xs w-full mb-2" />
            <div class="flex gap-4">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.search.regex" class="checkbox checkbox-xs" />
                <span class="text-xs">Regex</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.search.caseSensitive" class="checkbox checkbox-xs" />
                <span class="text-xs">Case sensitive</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="filterConfig.search.negative" class="checkbox checkbox-xs" />
                <span class="text-xs">Negative search</span>
              </label>
            </div>
          </div>

          <!-- Filter by file extension -->
          <div class="border border-base-300 rounded-lg p-3 col-span-2">
            <h4 class="text-sm font-semibold mb-2 text-base-content/70">Filter by file extension</h4>
            <div class="grid grid-cols-2 gap-2">
              <div class="form-control">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.extension.showOnlyEnabled" class="checkbox checkbox-xs" />
                  <span class="label-text text-xs">Show only:</span>
                </label>
                <input type="text" v-model="filterConfig.extension.showOnly" placeholder="asp,aspx,jsp,php" class="input input-bordered input-xs w-full mt-1" />
              </div>
              <div class="form-control">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" v-model="filterConfig.extension.hideEnabled" class="checkbox checkbox-xs" />
                  <span class="label-text text-xs">Hide:</span>
                </label>
                <input type="text" v-model="filterConfig.extension.hide" placeholder="js,gif,jpg,png,css" class="input input-bordered input-xs w-full mt-1" />
              </div>
            </div>
          </div>
        </div>

        <!-- 底部按钮 -->
        <div class="modal-action justify-between">
          <div class="flex gap-2">
            <button class="btn btn-sm" @click="showAllFilters">Show all</button>
            <button class="btn btn-sm" @click="hideAllFilters">Hide all</button>
            <button class="btn btn-sm" @click="revertFilterChanges">Revert changes</button>
          </div>
          <div class="flex gap-2">
            <button class="btn btn-sm" @click="closeFilterDialog">Cancel</button>
            <button class="btn btn-sm btn-primary" @click="applyFilterConfig">Apply</button>
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
        <!-- 筛选器状态按钮 -->
        <button 
          @click="openFilterDialog" 
          class="btn btn-sm btn-ghost gap-2 border border-base-300 flex-1 justify-start"
          :class="{ 'border-primary': hasActiveFilters }"
        >
          <i class="fas fa-filter" :class="{ 'text-primary': hasActiveFilters }"></i>
          <span class="text-xs truncate">{{ filterSummary }}</span>
        </button>
        
        <!-- 快捷操作按钮 -->
        <button @click="refreshRequests" class="btn btn-sm btn-ghost" title="刷新">
          <i :class="['fas fa-sync-alt', { 'fa-spin': isLoading }]"></i>
        </button>
      </div>
    </div>

    <!-- 可调整大小的上下分割布局 -->
    <div class="flex-1 flex flex-col min-h-0 relative">
      <!-- 上半部分：请求历史列表 -->
      <div 
        ref="topPanel"
        class="bg-base-100 border-b border-base-300 overflow-hidden flex flex-col"
        :style="{ height: topPanelHeight + 'px' }"
      >
        <div class="flex items-center justify-between px-4 py-2 border-b border-base-300 flex-shrink-0">
          <h3 class="font-semibold text-sm">
            <i class="fas fa-history mr-2"></i>
            HTTP 历史记录 ({{ filteredRequests.length }})
          </h3>
          <div class="dropdown dropdown-end">
            <label tabindex="0" class="btn btn-xs btn-ghost">
              <i class="fas fa-cog"></i>
            </label>
            <div tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52 mt-2 max-h-96 overflow-y-auto">
              <li class="menu-title"><span>显示列</span></li>
              <li v-for="col in columns" :key="col.id">
                <label class="label cursor-pointer justify-start gap-2">
                  <input type="checkbox" :checked="col.visible" @change="toggleColumn(col.id)" class="checkbox checkbox-xs" />
                  <span class="label-text text-xs">{{ col.label }}</span>
                </label>
              </li>
              <li><a @click="resetColumns" class="text-xs"><i class="fas fa-undo mr-1"></i>重置</a></li>
            </div>
          </div>
        </div>

        <!-- 虚拟滚动列表容器 -->
        <div 
          ref="scrollContainer" 
          class="flex-1 overflow-auto min-h-0"
          @scroll="handleScroll"
        >
          <div v-if="isLoading" class="flex items-center justify-center h-full">
            <i class="fas fa-spinner fa-spin text-2xl"></i>
          </div>

          <div v-else-if="filteredRequests.length > 0" :style="{ height: totalHeight + 'px', position: 'relative' }">
            <!-- 表头 -->
            <div 
              class="sticky top-0 z-10 flex bg-base-200 border-b-2 border-base-300 font-semibold table-row-text"
              :style="{ height: headerHeight + 'px', minWidth: 'max-content' }"
            >
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
              :class="{ 'bg-primary/10': selectedRequest?.id === item.data.id }"
              :style="{ 
                top: (item.offset + headerHeight) + 'px', 
                height: itemHeight + 'px',
                minWidth: 'max-content'
              }"
              @click="selectRequest(item.data)"
              @contextmenu.prevent="showContextMenu($event, item.data)"
            >
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
                  <span :class="['badge badge-xs', getStatusClass(item.data.status_code)]" :title="getStatusTitle(item.data.status_code)">
                    {{ getStatusText(item.data.status_code) }}
                  </span>
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
              <p>暂无请求历史</p>
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
        class="bg-base-100 overflow-hidden flex flex-col"
        :style="{ height: bottomPanelHeight + 'px' }"
      >
        <div class="flex items-center justify-between px-4 py-2 border-b border-base-300 flex-shrink-0">
          <h3 class="font-semibold text-sm">请求详情 - ID: {{ selectedRequest.id }}</h3>
          <button @click="closeDetails" class="btn btn-xs btn-ghost">
            <i class="fas fa-times"></i>
          </button>
        </div>

        <div class="flex-1 flex min-h-0 overflow-hidden relative">
          <!-- 左侧：Request -->
          <div class="flex flex-col overflow-hidden" :style="{ width: leftPanelWidth + 'px' }">
            <div class="bg-base-200 px-4 py-2 border-b border-base-300 flex items-center justify-between flex-shrink-0">
              <h4 class="font-semibold text-sm">Request</h4>
              <div class="btn-group btn-group-xs">
                <button 
                  :class="['btn btn-xs', requestTab === 'pretty' ? 'btn-active' : '']"
                  @click="requestTab = 'pretty'"
                >
                  Pretty
                </button>
                <button 
                  :class="['btn btn-xs', requestTab === 'raw' ? 'btn-active' : '']"
                  @click="requestTab = 'raw'"
                >
                  Raw
                </button>
                <button 
                  :class="['btn btn-xs', requestTab === 'hex' ? 'btn-active' : '']"
                  @click="requestTab = 'hex'"
                >
                  Hex
                </button>
              </div>
            </div>
            <div class="flex-1 overflow-hidden flex min-h-0">
              <div class="line-numbers select-none">
                <div v-for="n in getLineCount(formatRequest(selectedRequest, requestTab))" :key="n" class="line-number">{{ n }}</div>
              </div>
              <div 
                class="flex-1 overflow-auto p-2 http-content"
                v-html="highlightHttpRequest(formatRequest(selectedRequest, requestTab))"
              ></div>
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
                <h4 class="font-semibold text-sm">Response</h4>
                <span v-if="isResponseCompressed(selectedRequest)" class="badge badge-xs badge-info" title="响应已自动解压">
                  <i class="fas fa-file-archive mr-1"></i>Decompressed
                </span>
              </div>
              <div class="btn-group btn-group-xs">
                <button 
                  :class="['btn btn-xs', responseTab === 'pretty' ? 'btn-active' : '']"
                  @click="responseTab = 'pretty'"
                >
                  Pretty
                </button>
                <button 
                  :class="['btn btn-xs', responseTab === 'raw' ? 'btn-active' : '']"
                  @click="responseTab = 'raw'"
                >
                  Raw
                </button>
                <button 
                  :class="['btn btn-xs', responseTab === 'hex' ? 'btn-active' : '']"
                  @click="responseTab = 'hex'"
                >
                  Hex
                </button>
              </div>
            </div>
            <div class="flex-1 overflow-hidden flex min-h-0">
              <div class="line-numbers select-none">
                <div v-for="n in getLineCount(formatResponse(selectedRequest, responseTab))" :key="n" class="line-number">{{ n }}</div>
              </div>
              <div 
                class="flex-1 overflow-auto p-2 http-content"
                v-html="highlightHttpResponse(formatResponse(selectedRequest, responseTab))"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch, inject } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { dialog } from '@/composables/useDialog';

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0));

// 定义组件名称，用于 keep-alive
defineOptions({
  name: 'ProxyHistory'
});

// Emit 声明
const emit = defineEmits<{
  (e: 'sendToRepeater', request: { method: string; url: string; headers: Record<string, string>; body?: string }): void
}>();

// 右键菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  request: null as ProxyRequest | null,
});
const contextMenuRef = ref<HTMLElement | null>(null);

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

// 响应式状态
const requests = ref<ProxyRequest[]>([]);
const selectedRequest = ref<ProxyRequest | null>(null);
const showDetailsModal = ref(false);
const showColumnSettings = ref(false);
const isLoading = ref(false);
const scrollContainer = ref<HTMLElement | null>(null);

// 面板引用
const topPanel = ref<HTMLElement | null>(null);
const bottomPanel = ref<HTMLElement | null>(null);
const horizontalResizer = ref<HTMLElement | null>(null);
const verticalResizer = ref<HTMLElement | null>(null);

// 面板尺寸（从 localStorage 恢复或使用默认值）
// 20条记录 × 32px + 表头34px + 标题栏42px = 714px，向上取整到720
const STORAGE_KEY_TOP_HEIGHT = 'proxyHistory.topPanelHeight';
const STORAGE_KEY_BOTTOM_HEIGHT = 'proxyHistory.bottomPanelHeight';
const STORAGE_KEY_LEFT_WIDTH = 'proxyHistory.leftPanelWidth';

const topPanelHeight = ref(parseInt(localStorage.getItem(STORAGE_KEY_TOP_HEIGHT) || '720')); // 默认上面板高度（显示约20条记录）
const bottomPanelHeight = ref(parseInt(localStorage.getItem(STORAGE_KEY_BOTTOM_HEIGHT) || '400')); // 默认下面板高度
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

const stats = ref({
  total: 0,
  http: 0,
  https: 0,
  avgResponseTime: 0,
});

// 筛选器弹窗引用
const filterDialog = ref<HTMLDialogElement | null>(null);

// 默认筛选器配置
const defaultFilterConfig = () => ({
  requestType: {
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
  listener: {
    port: '',
  },
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
  { id: 'status', label: 'Status code', visible: true, width: 90, minWidth: 80 },
  { id: 'length', label: 'Length', visible: true, width: 80, minWidth: 60 },
  { id: 'mime', label: 'MIME type', visible: true, width: 100, minWidth: 80 },
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
const itemHeight = 32; // 每个项目的高度（像素）
const headerHeight = 34; // 表头高度（像素）
const scrollTop = ref(0);
const containerHeight = ref(600); // 容器高度（响应式）
const bufferSize = 5; // 缓冲区大小（额外渲染的项目数）
const maxVisibleItems = 20; // 一屏最多显示的条目数

// 滚动节流
let scrollTimer: number | null = null;
const scrollThrottleDelay = 16; // 约 60fps

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
      let text = r.url + ' ' + r.host;
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

const visibleColumns = computed(() => {
  return columns.value.filter(col => col.visible);
});

// 虚拟滚动计算
const totalHeight = computed(() => {
  return filteredRequests.value.length * itemHeight + headerHeight;
});

const visibleItems = computed((): VirtualItem[] => {
  // 计算可见区域的起始和结束索引
  const startIndex = Math.max(0, Math.floor(scrollTop.value / itemHeight) - bufferSize);
  const visibleCount = Math.ceil(containerHeight.value / itemHeight);
  
  // 限制最大可见数量为 maxVisibleItems
  const maxEndIndex = startIndex + Math.min(visibleCount + bufferSize * 2, maxVisibleItems);
  const endIndex = Math.min(filteredRequests.value.length, maxEndIndex);
  
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
  
  // 使用节流优化滚动性能
  if (scrollTimer !== null) {
    return;
  }
  
  scrollTimer = window.setTimeout(() => {
    scrollTop.value = target.scrollTop;
    scrollTimer = null;
    
    // 检查是否滚动到底部，触发加载更多
    const scrollHeight = target.scrollHeight;
    const clientHeight = target.clientHeight;
    const scrollBottom = scrollHeight - scrollTop.value - clientHeight;
    
    // 距离底部 200px 时触发加载更多
    if (scrollBottom < 200 && hasMore.value && !isLoadingMore.value) {
      loadMoreRequests();
    }
  }, scrollThrottleDelay);
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
  
  const diffY = event.clientY - resizeStartY.value;
  const newTopHeight = Math.max(200, Math.min(resizeStartTopHeight.value + diffY, window.innerHeight - 300));
  topPanelHeight.value = newTopHeight;
  
  // 计算下面板高度
  const availableHeight = window.innerHeight - topPanelHeight.value - 120; // 120 为工具栏和分割条高度
  bottomPanelHeight.value = Math.max(200, availableHeight);
}

function stopHorizontalResize() {
  isResizingHorizontal.value = false;
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  
  // 保存尺寸到 localStorage
  localStorage.setItem(STORAGE_KEY_TOP_HEIGHT, String(topPanelHeight.value));
  localStorage.setItem(STORAGE_KEY_BOTTOM_HEIGHT, String(bottomPanelHeight.value));
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
  if (statusCode === 0) return 'TLS Handshake Failed';
  return '';
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

// 增量更新统计（用于新请求）
function updateStatsIncremental(newRequest: ProxyRequest) {
  stats.value.total++;
  if (newRequest.protocol === 'https') {
    stats.value.https++;
  } else if (newRequest.protocol === 'http') {
    stats.value.http++;
  }
  
  // 重新计算平均响应时间
  const totalResponseTime = (stats.value.avgResponseTime * (stats.value.total - 1)) + newRequest.response_time;
  stats.value.avgResponseTime = Math.round(totalResponseTime / stats.value.total);
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
  filterConfig.value.requestType.showOnlyWithParams = false;
  filterConfig.value.requestType.hideWithoutResponse = false;
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
  // 重置面板高度
  topPanelHeight.value = window.innerHeight - 120;
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
    
    // 自动调整面板高度比例
    const availableHeight = window.innerHeight - 120; // 减去工具栏高度
    topPanelHeight.value = Math.floor(availableHeight * 0.4);
    bottomPanelHeight.value = Math.floor(availableHeight * 0.6);
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

function formatRequest(request: ProxyRequest, tab: string): string {
  if (tab === 'hex') {
    return stringToHex(formatRequestRaw(request));
  }
  
  if (tab === 'raw') {
    return formatRequestRaw(request);
  }
  
  // Pretty format
  let result = `${request.method} ${request.url} HTTP/1.1\n`;
  result += `Host: ${request.host}\n`;
  
  if (request.request_headers) {
    try {
      const headers = JSON.parse(request.request_headers);
      for (const [key, value] of Object.entries(headers)) {
        result += `${key}: ${value}\n`;
      }
    } catch {
      result += request.request_headers + '\n';
    }
  }
  
  if (request.request_body) {
    result += '\n' + request.request_body;
  }
  
  return result;
}

function formatRequestRaw(request: ProxyRequest): string {
  let result = `${request.method} ${request.url} HTTP/1.1\n`;
  result += `Host: ${request.host}\n`;
  
  if (request.request_headers) {
    result += request.request_headers + '\n';
  }
  
  if (request.request_body) {
    result += '\n' + request.request_body;
  }
  
  return result;
}

function formatResponse(request: ProxyRequest, tab: string): string {
  if (tab === 'hex') {
    return stringToHex(formatResponseRaw(request));
  }
  
  if (tab === 'raw') {
    return formatResponseRaw(request);
  }
  
  // Pretty format
  let result = `HTTP/1.2 ${request.status_code} OK\n`;
  
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers);
      for (const [key, value] of Object.entries(headers)) {
        result += `${key}: ${value}\n`;
      }
    } catch {
      result += request.response_headers + '\n';
    }
  }
  
  if (request.response_body) {
    result += '\n';
    
    // 检测内容类型
    const contentType = getResponseContentType(request);
    
    // 尝试根据 Content-Type 格式化
    if (contentType.includes('json') || contentType.includes('application/json')) {
      try {
        const json = JSON.parse(request.response_body);
        result += JSON.stringify(json, null, 2);
      } catch {
        result += request.response_body;
      }
    } else if (contentType.includes('html') || contentType.includes('xml')) {
      // HTML/XML 直接显示
      result += request.response_body;
    } else if (contentType.includes('text/')) {
      // 其他文本类型
      result += request.response_body;
    } else {
      // 二进制或未知类型
      const bodySize = new Blob([request.response_body]).size;
      result += `[Binary data - ${formatBytes(bodySize)}]\n`;
      result += `Content-Type: ${contentType}\n`;
      result += `\nFirst 200 characters:\n${request.response_body.substring(0, 200)}...`;
    }
  }
  
  return result;
}

function getResponseContentType(request: ProxyRequest): string {
  if (request.response_headers) {
    try {
      const headers = JSON.parse(request.response_headers);
      return headers['content-type'] || headers['Content-Type'] || '';
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

function formatResponseRaw(request: ProxyRequest): string {
  let result = `HTTP/1.2 ${request.status_code} OK\n`;
  
  if (request.response_headers) {
    result += request.response_headers + '\n';
  }
  
  if (request.response_body) {
    result += '\n' + request.response_body;
  }
  
  return result;
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

// 行号和语法高亮辅助函数
function getLineCount(text: string): number {
  if (!text) return 1;
  return text.split(/\r\n|\r|\n/).length;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function highlightHttpRequest(raw: string): string {
  if (!raw) return '';
  
  const lines = raw.split(/\r\n|\r|\n/);
  const result: string[] = [];
  let inBody = false;
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    if (i === 0) {
      // 请求行: GET /path HTTP/1.1
      const match = line.match(/^(\w+)\s+(.+?)\s+(HTTP\/[\d.]+)$/);
      if (match) {
        result.push(`<span class="http-method">${escapeHtml(match[1])}</span> <span class="http-path">${escapeHtml(match[2])}</span> <span class="http-version">${escapeHtml(match[3])}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else if (!inBody && line === '') {
      inBody = true;
      result.push('');
    } else if (!inBody) {
      // 请求头: Header-Name: value
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex);
        const value = line.substring(colonIndex + 1);
        result.push(`<span class="http-header-key">${escapeHtml(key)}</span>:<span class="http-header-value">${escapeHtml(value)}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else {
      // Body
      result.push(escapeHtml(line));
    }
  }
  
  return result.join('\n');
}

function highlightHttpResponse(raw: string): string {
  if (!raw) return '';
  
  const lines = raw.split(/\r\n|\r|\n/);
  const result: string[] = [];
  let inBody = false;
  let contentType = '';
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    
    if (i === 0) {
      // 状态行: HTTP/1.1 200 OK
      const match = line.match(/^(HTTP\/[\d.]+)\s+(\d+)\s*(.*)$/);
      if (match) {
        const statusClass = getStatusColorClass(parseInt(match[2]));
        result.push(`<span class="http-version">${escapeHtml(match[1])}</span> <span class="${statusClass}">${escapeHtml(match[2])}</span> <span class="http-status-text">${escapeHtml(match[3])}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else if (!inBody && line === '') {
      inBody = true;
      result.push('');
    } else if (!inBody) {
      // 响应头
      const colonIndex = line.indexOf(':');
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex);
        const value = line.substring(colonIndex + 1);
        if (key.toLowerCase() === 'content-type') {
          contentType = value.trim();
        }
        result.push(`<span class="http-header-key">${escapeHtml(key)}</span>:<span class="http-header-value">${escapeHtml(value)}</span>`);
      } else {
        result.push(escapeHtml(line));
      }
    } else {
      // Body - 根据 Content-Type 高亮
      if (contentType.includes('html')) {
        result.push(highlightHtml(line));
      } else if (contentType.includes('json')) {
        result.push(highlightJson(line));
      } else {
        result.push(escapeHtml(line));
      }
    }
  }
  
  return result.join('\n');
}

function getStatusColorClass(status: number): string {
  if (status >= 200 && status < 300) return 'http-status-2xx';
  if (status >= 300 && status < 400) return 'http-status-3xx';
  if (status >= 400 && status < 500) return 'http-status-4xx';
  if (status >= 500) return 'http-status-5xx';
  return '';
}

function highlightHtml(line: string): string {
  return line
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/&lt;(\/?)([\w-]+)/g, '&lt;$1<span class="html-tag">$2</span>')
    .replace(/\s([\w-]+)=/g, ' <span class="html-attr">$1</span>=')
    .replace(/="([^"]*)"/g, '="<span class="html-value">$1</span>"');
}

function highlightJson(line: string): string {
  return escapeHtml(line)
    .replace(/"([^"]+)":/g, '<span class="json-key">"$1"</span>:')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/:\s*(\d+\.?\d*)/g, ': <span class="json-number">$1</span>')
    .replace(/:\s*(true|false|null)/g, ': <span class="json-keyword">$1</span>');
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
  
  // 初始化面板高度（仅在没有保存值时使用默认值）
  const availableHeight = window.innerHeight - 120;
  if (!localStorage.getItem(STORAGE_KEY_TOP_HEIGHT)) {
    topPanelHeight.value = availableHeight;
  }
  if (!localStorage.getItem(STORAGE_KEY_BOTTOM_HEIGHT)) {
    bottomPanelHeight.value = Math.floor(availableHeight * 0.6);
  }
  
  // 初始化左面板宽度（仅在没有保存值时使用默认值）
  if (!localStorage.getItem(STORAGE_KEY_LEFT_WIDTH)) {
    leftPanelWidth.value = Math.floor(window.innerWidth * 0.5);
  }
});

onUnmounted(() => {
  if (unlistenRequest) unlistenRequest();
  if (resizeObserver && scrollContainer.value) {
    resizeObserver.unobserve(scrollContainer.value);
    resizeObserver.disconnect();
  }
  // 清理定时器
  if (updateTimer !== null) {
    clearTimeout(updateTimer);
  }
  if (scrollTimer !== null) {
    clearTimeout(scrollTimer);
  }
  // 清理拖拽事件监听
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
  document.removeEventListener('mousemove', handleHorizontalResize);
  document.removeEventListener('mouseup', stopHorizontalResize);
  document.removeEventListener('mousemove', handleVerticalResize);
  document.removeEventListener('mouseup', stopVerticalResize);
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
</script>

<style scoped>
/* 表格行文字使用系统字体大小 */
.table-row-text {
  font-size: var(--font-size-base, 14px);
}

/* 行号 */
.line-numbers {
  background: oklch(var(--b2));
  border-right: 1px solid oklch(var(--b3));
  padding: 0.5rem 0;
  min-width: 3rem;
  text-align: right;
  overflow: hidden;
}
.line-number {
  padding: 0 0.5rem;
  font-size: var(--font-size-base, 14px);
  line-height: 1.5;
  color: oklch(var(--bc) / 0.4);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* HTTP 内容 */
.http-content {
  white-space: pre;
  font-size: var(--font-size-base, 14px);
  line-height: 1.5;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* HTTP 语法高亮 */
.http-content :deep(.http-method) {
  color: #c41a16;
  font-weight: 600;
}
.http-content :deep(.http-path) {
  color: #1c00cf;
}
.http-content :deep(.http-version) {
  color: #5c5c5c;
}
.http-content :deep(.http-header-key) {
  color: #c41a16;
}
.http-content :deep(.http-header-value) {
  color: #000;
}
.http-content :deep(.http-status-2xx) {
  color: #18794e;
  font-weight: 600;
}
.http-content :deep(.http-status-3xx) {
  color: #0550ae;
  font-weight: 600;
}
.http-content :deep(.http-status-4xx) {
  color: #cf222e;
  font-weight: 600;
}
.http-content :deep(.http-status-5xx) {
  color: #8250df;
  font-weight: 600;
}
.http-content :deep(.http-status-text) {
  color: #5c5c5c;
}

/* HTML 语法高亮 */
.http-content :deep(.html-tag) {
  color: #0550ae;
}
.http-content :deep(.html-attr) {
  color: #c41a16;
}
.http-content :deep(.html-value) {
  color: #0a3069;
}

/* JSON 语法高亮 */
.http-content :deep(.json-key) {
  color: #0550ae;
}
.http-content :deep(.json-string) {
  color: #0a3069;
}
.http-content :deep(.json-number) {
  color: #0550ae;
}
.http-content :deep(.json-keyword) {
  color: #cf222e;
}

/* 暗色主题适配 */
[data-theme="dark"] .http-content :deep(.http-method),
[data-theme="dark"] .http-content :deep(.http-header-key),
[data-theme="dark"] .http-content :deep(.html-attr) {
  color: #ff7b72;
}
[data-theme="dark"] .http-content :deep(.http-path),
[data-theme="dark"] .http-content :deep(.html-tag),
[data-theme="dark"] .http-content :deep(.json-key),
[data-theme="dark"] .http-content :deep(.json-number) {
  color: #79c0ff;
}
[data-theme="dark"] .http-content :deep(.http-version),
[data-theme="dark"] .http-content :deep(.http-status-text) {
  color: #8b949e;
}
[data-theme="dark"] .http-content :deep(.http-header-value) {
  color: #c9d1d9;
}
[data-theme="dark"] .http-content :deep(.http-status-2xx) {
  color: #3fb950;
}
[data-theme="dark"] .http-content :deep(.http-status-3xx) {
  color: #58a6ff;
}
[data-theme="dark"] .http-content :deep(.http-status-4xx) {
  color: #f85149;
}
[data-theme="dark"] .http-content :deep(.http-status-5xx) {
  color: #a371f7;
}
[data-theme="dark"] .http-content :deep(.html-value),
[data-theme="dark"] .http-content :deep(.json-string) {
  color: #a5d6ff;
}
[data-theme="dark"] .http-content :deep(.json-keyword) {
  color: #ff7b72;
}
</style>
