<template>
  <div class="flex flex-col h-screen bg-base-200">

    <!-- 筛选器工具栏 -->
    <div class="bg-base-100 border-b border-base-300 p-3 flex-shrink-0">
      <div class="flex flex-wrap gap-2 items-end">
        <div class="form-control">
          <label class="label py-0 pb-1"><span class="label-text text-xs">协议</span></label>
          <select v-model="filters.protocol" class="select select-bordered select-sm w-24">
            <option value="">全部</option>
            <option value="http">HTTP</option>
            <option value="https">HTTPS</option>
          </select>
        </div>

        <div class="form-control">
          <label class="label py-0 pb-1"><span class="label-text text-xs">方法</span></label>
          <select v-model="filters.method" class="select select-bordered select-sm w-28">
            <option value="">全部</option>
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option>
            <option value="PATCH">PATCH</option>
          </select>
        </div>

        <div class="form-control">
          <label class="label py-0 pb-1"><span class="label-text text-xs">状态码</span></label>
          <select v-model="filters.statusCode" class="select select-bordered select-sm w-32">
            <option value="">全部</option>
            <option value="2xx">2xx</option>
            <option value="3xx">3xx</option>
            <option value="4xx">4xx</option>
            <option value="5xx">5xx</option>
          </select>
        </div>

        <div class="form-control flex-1 min-w-[200px]">
          <label class="label py-0 pb-1"><span class="label-text text-xs">搜索</span></label>
          <input v-model="filters.search" type="text" placeholder="URL、主机名..."
            class="input input-bordered input-sm" @input="applyFilters" />
        </div>

        <button @click="clearHistory" class="btn btn-sm btn-error btn-outline" title="清空历史">
          <i class="fas fa-trash"></i>
        </button>
        <button @click="resetFilters" class="btn btn-sm btn-outline" title="重置筛选">
          <i class="fas fa-redo"></i>
        </button>
        <button @click="refreshRequests" class="btn btn-sm btn-outline" title="刷新">
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
              class="sticky top-0 z-10 flex bg-base-200 border-b-2 border-base-300 text-xs font-semibold"
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
              class="absolute left-0 right-0 flex text-xs hover:bg-base-200 cursor-pointer border-b border-base-300"
              :class="{ 'bg-primary/10': selectedRequest?.id === item.data.id }"
              :style="{ 
                top: (item.offset + headerHeight) + 'px', 
                height: itemHeight + 'px',
                minWidth: 'max-content'
              }"
              @click="selectRequest(item.data)"
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
                  <span :class="['badge badge-xs', getStatusClass(item.data.status_code)]">
                    {{ item.data.status_code }}
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
            <div class="flex-1 overflow-auto p-3  font-mono text-xs min-h-0">
              <pre class="whitespace-pre-wrap break-all">{{ formatRequest(selectedRequest, requestTab) }}</pre>
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
            <div class="flex-1 overflow-auto p-3  font-mono text-xs min-h-0">
              <pre class="whitespace-pre-wrap break-all">{{ formatResponse(selectedRequest, responseTab) }}</pre>
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

  // 过滤掉 CONNECT 请求
  // result = result.filter(r => r.method.toUpperCase() !== 'CONNECT');

  // 使用早期返回优化性能
  if (!filters.value.protocol && 
      !filters.value.method && 
      !filters.value.statusCode && 
      !filters.value.search) {
    return result;
  }

  if (filters.value.protocol) {
    result = result.filter(r => r.protocol === filters.value.protocol);
  }

  if (filters.value.method) {
    result = result.filter(r => r.method === filters.value.method);
  }

  if (filters.value.statusCode) {
    const statusRange = filters.value.statusCode;
    result = result.filter(r => {
      const code = r.status_code;
      if (statusRange === '2xx') return code >= 200 && code < 300;
      if (statusRange === '3xx') return code >= 300 && code < 400;
      if (statusRange === '4xx') return code >= 400 && code < 500;
      if (statusRange === '5xx') return code >= 500 && code < 600;
      return true;
    });
  }

  if (filters.value.search) {
    const search = filters.value.search.toLowerCase();
    result = result.filter(r =>
      r.url.toLowerCase().includes(search) ||
      r.host.toLowerCase().includes(search)
    );
  }

  return result;
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
  if (statusCode >= 200 && statusCode < 300) return 'badge-success';
  if (statusCode >= 300 && statusCode < 400) return 'badge-info';
  if (statusCode >= 400 && statusCode < 500) return 'badge-warning';
  if (statusCode >= 500) return 'badge-error';
  return 'badge-ghost';
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

function resetFilters() {
  filters.value = {
    protocol: '',
    method: '',
    statusCode: '',
    search: '',
  };
  scrollTop.value = 0;
  if (scrollContainer.value) {
    scrollContainer.value.scrollTop = 0;
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
  await loadHistory();
});
</script>
