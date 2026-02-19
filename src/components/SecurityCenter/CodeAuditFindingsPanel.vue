<template>
  <div class="space-y-6">
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="flex flex-wrap gap-3 items-center">
        <div class="form-control">
          <select v-model="filters.severity" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">全部严重级别</option>
            <option value="critical">critical</option>
            <option value="high">high</option>
            <option value="medium">medium</option>
            <option value="low">low</option>
            <option value="info">info</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.status" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">全部状态</option>
            <option value="open">open</option>
            <option value="reviewed">reviewed</option>
            <option value="false_positive">false_positive</option>
            <option value="fixed">fixed</option>
          </select>
        </div>
        <div class="form-control flex-1 min-w-[220px]">
          <input
            v-model="filters.search"
            type="text"
            placeholder="搜索标题/描述/CWE"
            class="input input-bordered input-sm"
            @input="applyFilters"
          />
        </div>
        <button @click="refreshFindings" class="btn btn-outline btn-sm">刷新</button>
        <button
          @click="deleteSelectedFindings"
          :disabled="selectedIds.length === 0 || isDeleting"
          class="btn btn-warning btn-sm"
        >
          删除选中 ({{ selectedIds.length }})
        </button>
        <button
          @click="deleteAllFindings"
          :disabled="totalCount === 0 || isDeleting"
          class="btn btn-error btn-sm"
        >
          删除全部
        </button>
      </div>
    </div>

    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div v-if="isLoading" class="text-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
        <p class="mt-2 text-sm">{{ $t('common.loading') }}...</p>
      </div>

      <div v-else-if="findings.length > 0" class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th class="w-10">
                <input
                  type="checkbox"
                  class="checkbox checkbox-xs"
                  :checked="isAllCurrentPageSelected"
                  @change="toggleSelectAllCurrentPage(($event.target as HTMLInputElement).checked)"
                />
              </th>
              <th>严重级别</th>
              <th>标题</th>
              <th>CWE</th>
              <th>状态</th>
              <th>会话</th>
              <th>命中</th>
              <th>最后发现</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="finding in findings" :key="finding.id">
              <td>
                <input
                  type="checkbox"
                  class="checkbox checkbox-xs"
                  :checked="selectedIds.includes(finding.id)"
                  @change="toggleSelectFinding(finding.id, ($event.target as HTMLInputElement).checked)"
                />
              </td>
              <td><span :class="getSeverityBadgeClass(finding.severity)" class="badge badge-sm">{{ finding.severity }}</span></td>
              <td class="font-medium max-w-xs"><div class="line-clamp-2">{{ finding.title }}</div></td>
              <td><span class="badge badge-outline badge-xs">{{ finding.cwe || '-' }}</span></td>
              <td><span class="badge badge-ghost badge-sm">{{ finding.status }}</span></td>
              <td class="font-mono text-xs max-w-[180px]"><div class="line-clamp-1">{{ finding.conversation_id }}</div></td>
              <td><span class="badge badge-primary badge-xs">{{ finding.hit_count || 0 }}</span></td>
              <td class="text-xs opacity-70">{{ formatTime(finding.last_seen_at) }}</td>
              <td>
                <button @click="openDetails(finding)" class="btn btn-xs btn-outline">详情</button>
                <button @click="deleteSingleFinding(finding)" class="btn btn-xs btn-error ml-2" :disabled="isDeleting">
                  删除
                </button>
              </td>
            </tr>
          </tbody>
        </table>

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
        <p class="text-sm">暂无代码审计漏洞</p>
      </div>
    </div>

    <dialog :class="['modal', { 'modal-open': showDetailsModal }]" @click.self="closeDetails" @keydown.esc="closeDetails">
      <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto">
        <div v-if="selectedFinding">
          <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
            <h3 class="font-bold text-lg">代码审计漏洞详情</h3>
            <button @click="closeDetails" class="btn btn-sm btn-circle btn-ghost">✕</button>
          </div>

          <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm text-base-content/70 mb-1">严重级别</p>
                <span :class="getSeverityBadgeClass(selectedFinding.severity)" class="badge">
                  {{ selectedFinding.severity }}
                </span>
                <p v-if="selectedFinding.severity_raw && selectedFinding.severity_raw !== selectedFinding.severity" class="text-xs opacity-60 mt-1">
                  原始值: {{ selectedFinding.severity_raw }}
                </p>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">状态</p>
                <span class="badge badge-ghost">{{ selectedFinding.status }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">命中次数</p>
                <span class="badge badge-primary">{{ selectedFinding.hit_count }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">置信度</p>
                <span class="badge badge-info">{{ selectedFinding.confidence ?? '-' }}</span>
              </div>
            </div>

            <div>
              <p class="text-sm text-base-content/70 mb-1">标题</p>
              <p class="font-semibold text-base break-words">{{ selectedFinding.title }}</p>
            </div>

            <div>
              <p class="text-sm text-base-content/70 mb-1">描述</p>
              <div class="bg-base-200 p-3 rounded-lg">
                <p class="whitespace-pre-wrap break-words">{{ selectedFinding.description }}</p>
              </div>
            </div>

            <div v-if="selectedFinding.fix">
              <p class="text-sm text-base-content/70 mb-1">修复建议</p>
              <div class="bg-success/10 border border-success/20 p-3 rounded-lg">
                <p class="whitespace-pre-wrap text-sm">{{ selectedFinding.fix }}</p>
              </div>
            </div>

            <div>
              <p class="text-sm text-base-content/70 mb-2">影响文件</p>
              <div class="space-y-1" v-if="getDisplayFiles(selectedFinding).length">
                <div v-for="file in getDisplayFiles(selectedFinding)" :key="file" class="font-mono text-xs break-all">{{ file }}</div>
              </div>
              <p v-else class="text-sm opacity-60">无文件路径</p>
            </div>

            <div v-if="selectedFinding.source">
              <p class="text-sm text-base-content/70 mb-1">Source</p>
              <div class="bg-base-200 p-3 rounded-lg">
                <pre class="text-xs whitespace-pre-wrap break-words">{{ toJsonText(selectedFinding.source) }}</pre>
              </div>
            </div>

            <div v-if="selectedFinding.sink">
              <p class="text-sm text-base-content/70 mb-1">Sink</p>
              <div class="bg-base-200 p-3 rounded-lg">
                <pre class="text-xs whitespace-pre-wrap break-words">{{ toJsonText(selectedFinding.sink) }}</pre>
              </div>
            </div>

            <div v-if="selectedFinding.trace_path?.length">
              <p class="text-sm text-base-content/70 mb-2">Source-Sink 路径</p>
              <div class="space-y-2">
                <div
                  v-for="(step, idx) in selectedFinding.trace_path"
                  :key="`${selectedFinding.id}-trace-${idx}`"
                  class="bg-base-200 p-3 rounded-lg"
                >
                  <p class="text-xs font-medium mb-1">Step {{ idx + 1 }}</p>
                  <pre class="text-xs whitespace-pre-wrap break-words">{{ toJsonText(step) }}</pre>
                </div>
              </div>
            </div>

            <div v-if="selectedFinding.evidence?.length">
              <p class="text-sm text-base-content/70 mb-2">证据</p>
              <div class="space-y-2">
                <div
                  v-for="(item, idx) in selectedFinding.evidence"
                  :key="`${selectedFinding.id}-ev-${idx}`"
                  class="bg-base-200 p-3 rounded-lg text-sm whitespace-pre-wrap break-words"
                >
                  {{ item }}
                </div>
              </div>
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface AgentAuditFinding {
  id: string;
  conversation_id: string;
  finding_id: string;
  title: string;
  severity: string;
  severity_raw?: string;
  status: string;
  confidence?: number;
  cwe?: string;
  files: string[];
  source?: Record<string, any> | null;
  sink?: Record<string, any> | null;
  trace_path?: Array<Record<string, any>>;
  evidence?: string[];
  fix?: string;
  description: string;
  source_message_id?: string;
  hit_count: number;
  first_seen_at: string;
  last_seen_at: string;
  created_at: string;
  updated_at: string;
}

const findings = ref<AgentAuditFinding[]>([]);
const isLoading = ref(false);
const showDetailsModal = ref(false);
const selectedFinding = ref<AgentAuditFinding | null>(null);
const selectedIds = ref<string[]>([]);
const isDeleting = ref(false);
const currentPage = ref(1);
const pageSize = 10;
const totalCount = ref(0);

const filters = ref({
  severity: '',
  status: '',
  search: '',
});

const totalPages = computed(() => {
  return Math.max(1, Math.ceil(totalCount.value / pageSize));
});

const isAllCurrentPageSelected = computed(() => {
  if (findings.value.length === 0) return false;
  return findings.value.every((item) => selectedIds.value.includes(item.id));
});

const refreshFindings = async () => {
  isLoading.value = true;
  try {
    const countResponse = await invoke<any>('count_agent_audit_findings', {
      severityFilter: filters.value.severity || null,
      statusFilter: filters.value.status || null,
      conversationId: null,
      search: filters.value.search || null,
    });

    if (countResponse.success) {
      totalCount.value = countResponse.data;
    }

    const offset = (currentPage.value - 1) * pageSize;
    const listResponse = await invoke<any>('list_agent_audit_findings', {
      limit: pageSize,
      offset,
      severityFilter: filters.value.severity || null,
      statusFilter: filters.value.status || null,
      conversationId: null,
      search: filters.value.search || null,
    });

    if (listResponse.success && Array.isArray(listResponse.data)) {
      findings.value = listResponse.data;
    } else {
      findings.value = [];
    }
  } catch (error) {
    console.error('Failed to refresh code audit findings:', error);
    findings.value = [];
    totalCount.value = 0;
  } finally {
    isLoading.value = false;
  }
};

const applyFilters = () => {
  currentPage.value = 1;
  refreshFindings();
};

const openDetails = (finding: AgentAuditFinding) => {
  selectedFinding.value = finding;
  showDetailsModal.value = true;
};

const closeDetails = () => {
  showDetailsModal.value = false;
  selectedFinding.value = null;
};

const toggleSelectFinding = (id: string, checked: boolean) => {
  if (checked) {
    if (!selectedIds.value.includes(id)) {
      selectedIds.value = [...selectedIds.value, id];
    }
    return;
  }
  selectedIds.value = selectedIds.value.filter((item) => item !== id);
};

const toggleSelectAllCurrentPage = (checked: boolean) => {
  const currentIds = findings.value.map((item) => item.id);
  if (checked) {
    const merged = new Set([...selectedIds.value, ...currentIds]);
    selectedIds.value = Array.from(merged);
    return;
  }
  selectedIds.value = selectedIds.value.filter((id) => !currentIds.includes(id));
};

const deleteSingleFinding = async (finding: AgentAuditFinding) => {
  if (!window.confirm(`确认删除漏洞「${finding.title}」吗？`)) return;
  isDeleting.value = true;
  try {
    const resp = await invoke<any>('delete_agent_audit_finding', { findingId: finding.id });
    if (!resp?.success) {
      throw new Error(resp?.error || 'delete_agent_audit_finding failed');
    }
    selectedIds.value = selectedIds.value.filter((id) => id !== finding.id);
    if (selectedFinding.value?.id === finding.id) {
      closeDetails();
    }
    await refreshFindings();
  } catch (error) {
    console.error('Failed to delete agent audit finding:', error);
  } finally {
    isDeleting.value = false;
  }
};

const deleteSelectedFindings = async () => {
  if (selectedIds.value.length === 0) return;
  if (!window.confirm(`确认删除选中的 ${selectedIds.value.length} 条漏洞吗？`)) return;
  isDeleting.value = true;
  try {
    const resp = await invoke<any>('delete_agent_audit_findings_batch', {
      findingIds: selectedIds.value,
    });
    if (!resp?.success) {
      throw new Error(resp?.error || 'delete_agent_audit_findings_batch failed');
    }
    if (selectedFinding.value && selectedIds.value.includes(selectedFinding.value.id)) {
      closeDetails();
    }
    selectedIds.value = [];
    await refreshFindings();
  } catch (error) {
    console.error('Failed to delete selected agent audit findings:', error);
  } finally {
    isDeleting.value = false;
  }
};

const deleteAllFindings = async () => {
  if (totalCount.value === 0) return;
  if (!window.confirm(`确认删除全部 ${totalCount.value} 条代码审计漏洞吗？`)) return;
  isDeleting.value = true;
  try {
    const resp = await invoke<any>('delete_all_agent_audit_findings');
    if (!resp?.success) {
      throw new Error(resp?.error || 'delete_all_agent_audit_findings failed');
    }
    selectedIds.value = [];
    closeDetails();
    currentPage.value = 1;
    await refreshFindings();
  } catch (error) {
    console.error('Failed to delete all agent audit findings:', error);
  } finally {
    isDeleting.value = false;
  }
};

const toJsonText = (value: unknown) => {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value ?? '');
  }
};

const extractFilePathsFromText = (text: string): string[] => {
  const regex = /(?:^|[\s"'`(])((?:[\w.-]+\/)+[\w.-]+\.[a-zA-Z0-9]+)(?=$|[\s"'`):,])/g;
  const found: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = regex.exec(text)) !== null) {
    if (match[1]) found.push(match[1]);
  }
  return found;
};

const getDisplayFiles = (finding: AgentAuditFinding): string[] => {
  const merged = new Set<string>((finding.files || []).filter(Boolean));
  const candidateTexts: string[] = [];
  if (finding.source) candidateTexts.push(toJsonText(finding.source));
  if (finding.sink) candidateTexts.push(toJsonText(finding.sink));
  if (Array.isArray(finding.trace_path)) {
    for (const step of finding.trace_path) {
      candidateTexts.push(toJsonText(step));
    }
  }
  for (const text of candidateTexts) {
    for (const file of extractFilePathsFromText(text)) {
      merged.add(file);
    }
  }
  return Array.from(merged);
};

const getSeverityBadgeClass = (severity?: string) => {
  switch ((severity || '').toLowerCase()) {
    case 'critical':
      return 'badge-error';
    case 'high':
      return 'badge-warning';
    case 'medium':
      return 'badge-info';
    case 'low':
      return 'badge-success';
    default:
      return 'badge-ghost';
  }
};

const formatTime = (time?: string) => {
  if (!time) return '-';
  try {
    return new Date(time).toLocaleString();
  } catch {
    return time;
  }
};

const handleRefresh = () => {
  refreshFindings();
};

watch(currentPage, () => {
  refreshFindings();
});

onMounted(() => {
  refreshFindings();
  window.addEventListener('security-center-refresh', handleRefresh);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
});
</script>
