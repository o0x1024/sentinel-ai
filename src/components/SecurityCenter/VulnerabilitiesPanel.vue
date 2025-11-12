<template>
  <div class="space-y-6">
    <!-- 统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <div class="stat bg-error/10 rounded-lg shadow-sm">
        <div class="stat-title text-xs">{{ $t('vulnerabilities.severity.critical') }}</div>
        <div class="stat-value text-error text-lg">{{ stats.critical }}</div>
      </div>
      <div class="stat bg-warning/10 rounded-lg shadow-sm">
        <div class="stat-title text-xs">{{ $t('vulnerabilities.severity.high') }}</div>
        <div class="stat-value text-warning text-lg">{{ stats.high }}</div>
      </div>
      <div class="stat bg-info/10 rounded-lg shadow-sm">
        <div class="stat-title text-xs">{{ $t('vulnerabilities.severity.medium') }}</div>
        <div class="stat-value text-info text-lg">{{ stats.medium }}</div>
      </div>
      <div class="stat bg-success/10 rounded-lg shadow-sm">
        <div class="stat-title text-xs">{{ $t('vulnerabilities.severity.low') }}</div>
        <div class="stat-value text-success text-lg">{{ stats.low }}</div>
      </div>
    </div>

    <!-- 筛选器 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="flex flex-wrap gap-3 items-center">
        <!-- 批量操作 -->
        <div v-if="selectedIds.size > 0" class="flex items-center gap-2 mr-auto">
          <span class="text-sm text-base-content/70">已选择 {{ selectedIds.size }} 项</span>
          <button @click="deleteSelected" class="btn btn-error btn-sm">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
            </svg>
            删除选中
          </button>
          <button @click="selectedIds.clear()" class="btn btn-ghost btn-sm">取消选择</button>
        </div>
        
        <div class="form-control">
          <select v-model="filters.severity" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('vulnerabilities.allSeverities') }}</option>
            <option value="critical">{{ $t('vulnerabilities.severity.critical') }}</option>
            <option value="high">{{ $t('vulnerabilities.severity.high') }}</option>
            <option value="medium">{{ $t('vulnerabilities.severity.medium') }}</option>
            <option value="low">{{ $t('vulnerabilities.severity.low') }}</option>
          </select>
        </div>
        <div class="form-control flex-1">
          <input 
            v-model="filters.search" 
            type="text" 
            :placeholder="$t('common.search') + '...'"
            class="input input-bordered input-sm" 
            @input="applyFilters"
          />
        </div>
        <button @click="refreshFindings" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
        <button @click="deleteAll" class="btn btn-error btn-outline btn-sm" :disabled="totalCount === 0">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
          </svg>
          清空全部
        </button>
      </div>
    </div>

    <!-- 漏洞列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div v-if="isLoading" class="text-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
        <p class="mt-2 text-sm">{{ $t('common.loading') }}...</p>
      </div>

      <div v-else-if="filteredFindings.length > 0" class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th>
                <label>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm" 
                    :checked="isAllSelected"
                    @change="toggleSelectAll"
                  />
                </label>
              </th>
              <th>{{ $t('vulnerabilities.severity.title') }}</th>
              <th>{{ $t('vulnerabilities.title') }}</th>
              <th>类型</th>
              <th>{{ $t('common.url') }}</th>
              <th>{{ $t('vulnerabilities.plugin') }}</th>
              <th>命中</th>
              <th>{{ $t('common.time') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="finding in paginatedFindings" :key="finding.id">
              <td>
                <label>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm" 
                    :checked="selectedIds.has(finding.id)"
                    @change="toggleSelect(finding.id)"
                  />
                </label>
              </td>
              <td>
                <span :class="getSeverityBadgeClass(finding?.severity)" class="badge badge-sm">
                  {{ finding?.severity || 'unknown' }}
                </span>
              </td>
              <td class="font-medium max-w-xs">
                <div class="line-clamp-2">{{ finding?.title || 'N/A' }}</div>
              </td>
              <td><span class="badge badge-outline badge-xs">{{ finding?.vuln_type || 'N/A' }}</span></td>
              <td class="font-mono text-xs max-w-xs">
                <div class="line-clamp-1">{{ finding?.url || 'N/A' }}</div>
              </td>
              <td><span class="badge badge-ghost badge-sm">{{ finding?.plugin_id || 'N/A' }}</span></td>
              <td>
                <span class="badge badge-primary badge-xs">{{ finding?.hit_count || 0 }}</span>
              </td>
              <td class="text-xs opacity-70">{{ formatTime(finding?.last_seen_at) }}</td>
              <td>
                <div class="flex gap-1">
                  <button @click="openDetails(finding)" class="btn btn-xs btn-outline" title="查看详情">
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"></path>
                      <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button @click="deleteSingle(finding.id)" class="btn btn-xs btn-error btn-outline" title="删除">
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>

        <!-- 分页 -->
        <div class="flex justify-between items-center py-4 px-4">
          <div class="text-sm text-base-content/70">
            共 {{ totalCount }} 条记录，每页 {{ pageSize }} 条
          </div>
          <div class="join">
            <button @click="currentPage--" :disabled="currentPage === 1" class="join-item btn btn-sm">«</button>
            <button class="join-item btn btn-sm btn-active">{{ currentPage }} / {{ totalPages }}</button>
            <button @click="currentPage++" :disabled="currentPage === totalPages" class="join-item btn btn-sm">»</button>
          </div>
        </div>
      </div>

      <div v-else class="text-center py-8 text-base-content/50">
        <svg class="w-16 h-16 mx-auto mb-2 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"></path>
        </svg>
        <p class="text-sm">{{ $t('vulnerabilities.noFindings') }}</p>
      </div>
    </div>

    <!-- 详情模态框 -->
    <dialog :class="['modal', { 'modal-open': showDetailsModal }]" @click.self="closeDetails" @keydown.esc="closeDetails">
      <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto">
        <div v-if="selectedFinding">
          <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
            <h3 class="font-bold text-lg">{{ $t('vulnerabilities.details') }}</h3>
            <button @click="closeDetails" class="btn btn-sm btn-circle btn-ghost">✕</button>
          </div>

          <div class="space-y-4">
            <!-- 基本信息 -->
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm text-base-content/70 mb-1">{{ $t('vulnerabilities.severity.title') }}</p>
                <span :class="getSeverityBadgeClass(selectedFinding.severity)" class="badge">
                  {{ selectedFinding.severity }}
                </span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">置信度</p>
                <span class="badge badge-info">{{ selectedFinding.confidence }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">状态</p>
                <span class="badge badge-ghost">{{ selectedFinding.status }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">命中次数</p>
                <span class="badge badge-primary">{{ selectedFinding.hit_count }}</span>
              </div>
            </div>

            <!-- 标题和描述 -->
            <div>
              <p class="text-sm text-base-content/70 mb-1">{{ $t('vulnerabilities.title') }}</p>
              <p class="font-semibold text-base break-words">{{ selectedFinding.title }}</p>
            </div>

            <div>
              <p class="text-sm text-base-content/70 mb-1">{{ $t('vulnerabilities.description') }}</p>
              <div class="bg-base-200 p-3 rounded-lg">
                <p class="whitespace-pre-wrap break-words">{{ selectedFinding.description }}</p>
              </div>
            </div>

            <!-- URL 和方法 -->
            <div>
              <p class="text-sm text-base-content/70 mb-1">{{ $t('common.url') }}</p>
              <p class="font-mono text-sm break-all bg-base-200 p-2 rounded whitespace-pre-wrap">{{ selectedFinding.url }}</p>
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm text-base-content/70 mb-1">{{ $t('vulnerabilities.plugin') }}</p>
                <span class="badge badge-ghost">{{ selectedFinding.plugin_id }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">漏洞类型</p>
                <span class="badge badge-outline">{{ selectedFinding.vuln_type }}</span>
              </div>
            </div>

            <!-- CWE 和 OWASP -->
            <div class="grid grid-cols-2 gap-4" v-if="selectedFinding.cwe || selectedFinding.owasp">
              <div v-if="selectedFinding.cwe">
                <p class="text-sm text-base-content/70 mb-1">CWE</p>
                <p class="font-mono text-sm">{{ selectedFinding.cwe }}</p>
              </div>
              <div v-if="selectedFinding.owasp">
                <p class="text-sm text-base-content/70 mb-1">OWASP</p>
                <p class="font-mono text-sm">{{ selectedFinding.owasp }}</p>
              </div>
            </div>

            <!-- 修复建议 -->
            <div v-if="selectedFinding.remediation">
              <p class="text-sm text-base-content/70 mb-1">修复建议</p>
              <div class="bg-success/10 border border-success/20 p-3 rounded-lg">
                <p class="whitespace-pre-wrap text-sm">{{ selectedFinding.remediation }}</p>
              </div>
            </div>

            <!-- 时间信息 -->
            <div class="grid grid-cols-3 gap-4 text-xs">
              <div>
                <p class="text-base-content/70 mb-1">首次发现</p>
                <p>{{ formatTime(selectedFinding.first_seen_at) }}</p>
              </div>
              <div>
                <p class="text-base-content/70 mb-1">最后发现</p>
                <p>{{ formatTime(selectedFinding.last_seen_at) }}</p>
              </div>
              <div>
                <p class="text-base-content/70 mb-1">创建时间</p>
                <p>{{ formatTime(selectedFinding.created_at) }}</p>
              </div>
            </div>

            <!-- 证据信息 -->
            <div class="divider">证据信息</div>

            <div v-if="selectedFinding.evidence && selectedFinding.evidence.length > 0">
              <div v-for="(evidence, idx) in selectedFinding.evidence" :key="evidence.id"
                class="collapse collapse-arrow bg-base-200 mb-3">
                <input type="checkbox" :checked="idx === 0" />
                <div class="collapse-title font-medium">
                  <div class="flex items-center gap-2">
                    <span class="badge badge-sm">证据 #{{ idx + 1 }}</span>
                    <span class="badge badge-outline badge-sm">{{ evidence.method }}</span>
                    <span class="text-xs opacity-70">{{ evidence.location }}</span>
                  </div>
                </div>
                <div class="collapse-content space-y-3">
                  <!-- 证据片段 -->
                  <div>
                    <p class="text-sm font-semibold text-base-content/70 mb-2">证据片段</p>
                    <pre class="bg-black/80 text-green-400 p-3 rounded text-xs overflow-x-auto whitespace-pre-wrap break-words">{{ evidence.evidence_snippet }}</pre>
                  </div>

                  <!-- 请求信息 -->
                  <div class="space-y-2">
                    <p class="text-sm font-semibold text-primary">请求信息</p>
                    
                    <div v-if="evidence.request_headers">
                      <p class="text-xs text-base-content/70 mb-1">请求头</p>
                      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-60 whitespace-pre-wrap break-words">{{ formatJson(evidence.request_headers) }}</pre>
                    </div>

                    <div v-if="evidence.request_body">
                      <p class="text-xs text-base-content/70 mb-1">请求体</p>
                      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-60 whitespace-pre-wrap break-words">{{ evidence.request_body.length > 2000 ? truncateText(evidence.request_body, 2000) : evidence.request_body }}</pre>
                    </div>
                  </div>

                  <!-- 响应信息 -->
                  <div class="space-y-2" v-if="evidence.response_status || evidence.response_headers || evidence.response_body">
                    <p class="text-sm font-semibold text-secondary">响应信息</p>
                    
                    <div v-if="evidence.response_status">
                      <p class="text-xs text-base-content/70 mb-1">响应状态</p>
                      <span :class="['badge badge-sm', evidence.response_status >= 400 ? 'badge-error' : 'badge-success']">
                        {{ evidence.response_status }}
                      </span>
                    </div>

                    <div v-if="evidence.response_headers">
                      <p class="text-xs text-base-content/70 mb-1">响应头</p>
                      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-60 whitespace-pre-wrap break-words">{{ formatJson(evidence.response_headers) }}</pre>
                    </div>

                    <div v-if="evidence.response_body">
                      <p class="text-xs text-base-content/70 mb-1">响应体</p>
                      <pre class="bg-base-300 p-3 rounded text-xs overflow-x-auto max-h-60 whitespace-pre-wrap break-words">{{ evidence.response_body.length > 2000 ? truncateText(evidence.response_body, 2000) : evidence.response_body }}</pre>
                    </div>
                  </div>

                  <!-- 时间戳 -->
                  <div class="text-xs opacity-70">
                    <span>记录时间: {{ formatTime(evidence.timestamp) }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="text-center py-8 text-base-content/50">
              <svg class="w-12 h-12 mx-auto mb-2 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
              </svg>
              <p class="text-sm">暂无证据信息</p>
            </div>
          </div>
        </div>

        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button @click="closeDetails" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeDetails">close</button>
      </form>
    </dialog>

    <!-- 删除全部确认对话框 -->
    <dialog :class="['modal', { 'modal-open': showDeleteAllModal }]">
      <div class="modal-box">
        <h3 class="font-bold text-lg text-error">
          <svg class="w-6 h-6 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
          </svg>
          确认删除
        </h3>
        <p class="py-4">
          确定要清空所有 <span class="font-bold text-error">{{ totalCount }}</span> 条漏洞记录吗？
        </p>
        <p class="text-sm text-warning pb-4">
          ⚠️ 此操作不可恢复，所有漏洞记录及相关证据将被永久删除！
        </p>
        <div class="modal-action">
          <button @click="showDeleteAllModal = false" class="btn btn-ghost">取消</button>
          <button @click="confirmDeleteAll" class="btn btn-error">
            <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
            </svg>
            确认删除
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="showDeleteAllModal = false">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();
const emit = defineEmits<{
  'stats-updated': [stats: { total: number; critical: number }]
}>();

interface Evidence {
  id: string;
  vuln_id: string;
  url: string;
  method: string;
  location: string;
  evidence_snippet: string;
  request_headers?: string;
  request_body?: string;
  response_status?: number;
  response_headers?: string;
  response_body?: string;
  timestamp: string;
}

interface Finding {
  id: string;
  plugin_id: string;
  vuln_type: string;
  severity: string;
  confidence: string;
  title: string;
  description: string;
  cwe?: string;
  owasp?: string;
  remediation?: string;
  status: string;
  url: string;
  method?: string;
  hit_count: number;
  first_seen_at: string;
  last_seen_at: string;
  created_at: string;
  updated_at: string;
  evidence?: Evidence[];
}

const findings = ref<Finding[]>([]);
const isLoading = ref(false);
const showDetailsModal = ref(false);
const showDeleteAllModal = ref(false);
const selectedFinding = ref<Finding | null>(null);
const selectedIds = ref<Set<string>>(new Set());

const stats = ref({
  critical: 0,
  high: 0,
  medium: 0,
  low: 0,
});

const filters = ref({
  severity: '',
  search: '',
});

const currentPage = ref(1);
const pageSize = 10; // 每页10条（后端分页）
const totalCount = ref(0); // 总条数

const filteredFindings = computed(() => {
  // 后端分页，直接返回当前页数据
  return findings.value;
});

const paginatedFindings = computed(() => {
  // 后端已分页，直接使用
  return findings.value;
});

const totalPages = computed(() => {
  return Math.ceil(totalCount.value / pageSize);
});

const isAllSelected = computed(() => {
  return paginatedFindings.value.length > 0 && 
         paginatedFindings.value.every(f => selectedIds.value.has(f.id));
});

const refreshFindings = async () => {
  isLoading.value = true;
  try {
    // 获取总数
    const countResponse = await invoke<any>('count_findings', {
      severityFilter: filters.value.severity || null,
    });
    
    if (countResponse.success) {
      totalCount.value = countResponse.data;
    }
    
    // 获取当前页数据（后端分页）
    const offset = (currentPage.value - 1) * pageSize;
    const response = await invoke<any>('list_findings', {
      limit: pageSize,
      offset: offset,
      severityFilter: filters.value.severity || null,
    });

    if (response.success && response.data) {
      console.log('Raw API response:', response.data);
      
      // 数据结构：VulnerabilityWithEvidence 已被 flatten
      // 直接包含所有 vulnerability 字段 + evidence 数组
      findings.value = response.data.map((item: any) => {
        console.log('Item structure:', item);
        
        // 确保必要字段存在
        if (!item.id || !item.severity) {
          console.error('Missing required fields in item:', item);
          return null;
        }
        
        // 数据已经是扁平的，直接使用
        return {
          ...item,
          evidence: item.evidence || [], // 保存证据数据，默认空数组
        };
      }).filter((f: any) => f !== null); // 过滤掉 null 项
      
      console.log(`Loaded ${findings.value.length} findings with evidence`);
      console.log('First finding:', findings.value[0]);
      updateStats();
    }
  } catch (error) {
    console.error('Failed to refresh findings:', error);
    findings.value = [];
    totalCount.value = 0;
  } finally {
    isLoading.value = false;
  }
};

const updateStats = () => {
  stats.value = {
    critical: findings.value.filter(f => f && f.severity === 'critical').length,
    high: findings.value.filter(f => f && f.severity === 'high').length,
    medium: findings.value.filter(f => f && f.severity === 'medium').length,
    low: findings.value.filter(f => f && f.severity === 'low').length,
  };

  emit('stats-updated', {
    total: findings.value.length,
    critical: stats.value.critical
  });
};

const applyFilters = () => {
  currentPage.value = 1;
  refreshFindings();
};

const openDetails = (finding: Finding) => {
  selectedFinding.value = finding;
  showDetailsModal.value = true;
};

const closeDetails = () => {
  showDetailsModal.value = false;
  selectedFinding.value = null;
};

const toggleSelect = (id: string) => {
  if (selectedIds.value.has(id)) {
    selectedIds.value.delete(id);
  } else {
    selectedIds.value.add(id);
  }
};

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    // 取消全选当前页
    paginatedFindings.value.forEach(f => selectedIds.value.delete(f.id));
  } else {
    // 全选当前页
    paginatedFindings.value.forEach(f => selectedIds.value.add(f.id));
  }
};

const deleteSingle = async (id: string) => {

  try {
    const response = await invoke<any>('delete_passive_vulnerability', { vulnId: id });
    if (response.success) {
      console.log('Vulnerability deleted:', id);
      await refreshFindings();
      selectedIds.value.delete(id);
    } else {
      alert('删除失败: ' + (response.error || '未知错误'));
    }
  } catch (error) {
    console.error('Failed to delete vulnerability:', error);
    alert('删除失败: ' + error);
  }
};

const deleteSelected = async () => {
  if (selectedIds.value.size === 0) return;
  

  
  try {
    const ids = Array.from(selectedIds.value);
    const response = await invoke<any>('delete_passive_vulnerabilities_batch', { vulnIds: ids });
    if (response.success) {
      console.log(`Deleted ${ids.length} vulnerabilities`);
      await refreshFindings();
      selectedIds.value.clear();
    } else {
      alert('批量删除失败: ' + (response.error || '未知错误'));
    }
  } catch (error) {
    console.error('Failed to delete vulnerabilities:', error);
    alert('批量删除失败: ' + error);
  }
};

const deleteAll = () => {
  if (totalCount.value === 0) return;
  showDeleteAllModal.value = true;
};

const confirmDeleteAll = async () => {
  showDeleteAllModal.value = false;
  isLoading.value = true;
  
  try {
    const response = await invoke<any>('delete_all_passive_vulnerabilities');
    if (response.success) {
      console.log('All vulnerabilities deleted');
      await refreshFindings();
      selectedIds.value.clear();
    } else {
      alert('清空失败: ' + (response.error || '未知错误'));
    }
  } catch (error) {
    console.error('Failed to delete all vulnerabilities:', error);
    alert('清空失败: ' + error);
  } finally {
    isLoading.value = false;
  }
};

const getSeverityBadgeClass = (severity?: string) => {
  if (!severity) return 'badge-ghost';
  switch (severity.toLowerCase()) {
    case 'critical': return 'badge-error';
    case 'high': return 'badge-warning';
    case 'medium': return 'badge-info';
    case 'low': return 'badge-success';
    default: return 'badge-ghost';
  }
};

const truncateUrl = (url: string, maxLength = 50) => {
  if (!url || url.length <= maxLength) return url;
  return url.substring(0, maxLength) + '...';
};

const formatTime = (timestamp: string) => {
  if (!timestamp) return '-';
  const date = new Date(timestamp);
  return date.toLocaleString('zh-CN');
};

const truncateText = (text: string, maxLength = 500) => {
  if (!text) return '';
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + '...';
};

const formatJson = (jsonString?: string) => {
  if (!jsonString) return '';
  try {
    const obj = JSON.parse(jsonString);
    return JSON.stringify(obj, null, 2);
  } catch {
    // 如果不是有效的 JSON，直接返回原文本
    return jsonString;
  }
};

const handleRefresh = () => {
  refreshFindings();
};

// 监听页码变化
watch(currentPage, () => {
  refreshFindings();
});

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showDetailsModal.value) {
    closeDetails();
  }
};

onMounted(() => {
  refreshFindings();
  window.addEventListener('security-center-refresh', handleRefresh);
  window.addEventListener('keydown', handleKeyDown);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
  window.removeEventListener('keydown', handleKeyDown);
});
</script>
