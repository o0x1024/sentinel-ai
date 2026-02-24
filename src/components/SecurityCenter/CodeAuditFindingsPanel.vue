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
        <div class="form-control">
          <select v-model="filters.lifecycleStage" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">全部生命周期</option>
            <option value="candidate">candidate</option>
            <option value="triaged">triaged</option>
            <option value="verified">verified</option>
            <option value="confirmed">confirmed</option>
            <option value="rejected">rejected</option>
            <option value="archived">archived</option>
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
        <div class="form-control min-w-[240px]">
          <input
            v-model="filters.conversationId"
            type="text"
            placeholder="按会话ID过滤（可选）"
            class="input input-bordered input-sm"
            @input="applyFilters"
          />
        </div>
        <button @click="refreshFindings" class="btn btn-outline btn-sm">刷新</button>
        <button
          @click="openDeleteSelectedModal"
          :disabled="selectedIds.length === 0 || isDeleting"
          class="btn btn-warning btn-sm"
        >
          删除选中 ({{ selectedIds.length }})
        </button>
        <button
          @click="openDeleteAllModal"
          :disabled="totalCount === 0 || isDeleting"
          class="btn btn-error btn-sm"
        >
          删除全部
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-4 gap-3 mt-4">
        <div class="stats shadow bg-base-200/60">
          <div class="stat p-3">
            <div class="stat-title text-xs">证据率</div>
            <div class="stat-value text-lg">{{ formatPct(qualityGateMetrics?.evidence_rate) }}</div>
            <div class="stat-desc text-xs">阈值 ≥ {{ formatPct(qualityGateMetrics?.thresholds?.min_evidence_rate) }}</div>
          </div>
        </div>
        <div class="stats shadow bg-base-200/60">
          <div class="stat p-3">
            <div class="stat-title text-xs">不确定占比</div>
            <div class="stat-value text-lg">{{ formatPct(qualityGateMetrics?.uncertain_rate) }}</div>
            <div class="stat-desc text-xs">阈值 ≤ {{ formatPct(qualityGateMetrics?.thresholds?.max_uncertain_rate) }}</div>
          </div>
        </div>
        <div class="stats shadow bg-base-200/60">
          <div class="stat p-3">
            <div class="stat-title text-xs">误报回退率</div>
            <div class="stat-value text-lg">{{ formatPct(qualityGateMetrics?.false_positive_rate) }}</div>
            <div class="stat-desc text-xs">阈值 ≤ {{ formatPct(qualityGateMetrics?.thresholds?.max_false_positive_rate) }}</div>
          </div>
        </div>
        <div class="stats shadow" :class="qualityGateMetrics?.gate_passed ? 'bg-success/10' : 'bg-error/10'">
          <div class="stat p-3">
            <div class="stat-title text-xs">质量门禁</div>
            <div class="stat-value text-lg">
              <span v-if="isLoadingQualityGate" class="loading loading-spinner loading-sm"></span>
              <span v-else>{{ qualityGateMetrics?.gate_passed ? 'PASS' : 'FAIL' }}</span>
            </div>
            <div class="stat-desc text-xs">样本 {{ qualityGateMetrics?.total_findings ?? 0 }}</div>
          </div>
        </div>
      </div>

      <div class="mt-4 p-3 rounded-lg border border-base-300 bg-base-200/40">
        <div class="flex items-center justify-between mb-2">
          <p class="text-sm font-medium">质量门禁阈值配置</p>
          <div class="flex gap-2">
            <button class="btn btn-xs btn-ghost" @click="loadThresholdDraft">加载阈值</button>
            <button class="btn btn-xs btn-ghost" @click="resetThresholdDraft">重置</button>
            <button class="btn btn-xs btn-primary" :disabled="isSavingThresholds" @click="saveThresholds">
              <span v-if="isSavingThresholds" class="loading loading-spinner loading-xs"></span>
              保存阈值
            </button>
          </div>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-3">
          <label class="form-control">
            <span class="label-text text-xs">阈值作用域</span>
            <select v-model="thresholdScope" class="select select-bordered select-sm">
              <option value="global">全局默认</option>
              <option value="conversation">会话覆盖</option>
            </select>
          </label>
          <label class="form-control" v-if="thresholdScope === 'conversation'">
            <span class="label-text text-xs">阈值会话ID</span>
            <input v-model="thresholdConversationId" type="text" class="input input-bordered input-sm" placeholder="输入会话ID" />
          </label>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
          <label class="form-control">
            <span class="label-text text-xs">最小证据率 (0-1)</span>
            <input v-model.number="thresholdDraft.min_evidence_rate" type="number" min="0" max="1" step="0.01" class="input input-bordered input-sm" />
          </label>
          <label class="form-control">
            <span class="label-text text-xs">最大不确定占比 (0-1)</span>
            <input v-model.number="thresholdDraft.max_uncertain_rate" type="number" min="0" max="1" step="0.01" class="input input-bordered input-sm" />
          </label>
          <label class="form-control">
            <span class="label-text text-xs">最大误报回退率 (0-1)</span>
            <input v-model.number="thresholdDraft.max_false_positive_rate" type="number" min="0" max="1" step="0.01" class="input input-bordered input-sm" />
          </label>
        </div>
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
              <th>生命周期</th>
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
              <td><span class="badge badge-warning badge-sm">{{ finding.lifecycle_stage }}</span></td>
              <td class="font-mono text-xs max-w-[180px]"><div class="line-clamp-1">{{ finding.conversation_id }}</div></td>
              <td><span class="badge badge-primary badge-xs">{{ finding.hit_count || 0 }}</span></td>
              <td class="text-xs opacity-70">{{ formatTime(finding.last_seen_at) }}</td>
              <td>
                <button @click="openDetails(finding)" class="btn btn-xs btn-outline">详情</button>
                <button @click="openDeleteSingleModal(finding)" class="btn btn-xs btn-error ml-2" :disabled="isDeleting">
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
                <p class="text-sm text-base-content/70 mb-1">生命周期</p>
                <span class="badge badge-warning">{{ selectedFinding.lifecycle_stage }}</span>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">验证状态</p>
                <span class="badge badge-info">{{ selectedFinding.verification_status }}</span>
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

            <div v-if="selectedFinding.required_evidence?.length">
              <p class="text-sm text-base-content/70 mb-2">要求证据</p>
              <div class="space-y-2">
                <div
                  v-for="(item, idx) in selectedFinding.required_evidence"
                  :key="`${selectedFinding.id}-required-ev-${idx}`"
                  class="bg-warning/10 border border-warning/20 p-3 rounded-lg text-sm whitespace-pre-wrap break-words"
                >
                  {{ item }}
                </div>
              </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <p class="text-sm text-base-content/70 mb-1">审计者上下文</p>
                <div class="bg-base-200 p-3 rounded-lg">
                  <pre class="text-xs whitespace-pre-wrap break-words">{{ toJsonText(selectedFinding.verifier) }}</pre>
                </div>
              </div>
              <div>
                <p class="text-sm text-base-content/70 mb-1">Judge 上下文</p>
                <div class="bg-base-200 p-3 rounded-lg">
                  <pre class="text-xs whitespace-pre-wrap break-words">{{ toJsonText(selectedFinding.judge) }}</pre>
                </div>
              </div>
            </div>

            <div>
              <p class="text-sm text-base-content/70 mb-1">生命周期流转</p>
              <div class="flex flex-wrap items-center gap-2">
                <select v-model="lifecycleTargetStage" class="select select-bordered select-sm min-w-[180px]">
                  <option value="candidate">candidate</option>
                  <option value="triaged">triaged</option>
                  <option value="verified">verified</option>
                  <option value="confirmed">confirmed</option>
                  <option value="rejected">rejected</option>
                  <option value="archived">archived</option>
                </select>
                <select v-model="lifecycleTargetVerification" class="select select-bordered select-sm min-w-[180px]">
                  <option value="">保持当前验证状态</option>
                  <option value="unverified">unverified</option>
                  <option value="pending">pending</option>
                  <option value="passed">passed</option>
                  <option value="failed">failed</option>
                  <option value="needs_more_evidence">needs_more_evidence</option>
                </select>
                <button
                  class="btn btn-sm btn-primary"
                  :disabled="isTransitioningLifecycle || !hasLifecycleTransitionPermission"
                  @click="transitionLifecycle"
                >
                  <span v-if="isTransitioningLifecycle" class="loading loading-spinner loading-xs"></span>
                  更新生命周期
                </button>
              </div>
              <p v-if="!hasLifecycleTransitionPermission" class="text-xs text-warning mt-2">
                当前角色无生命周期流转权限。请为当前角色配置 capability `audit.lifecycle.transition`，或设置 localStorage `security:auditLifecycleTransitionEnabled=true` 临时放行。
              </p>
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

    <dialog :class="['modal', { 'modal-open': showDeleteModal }]" @click.self="showDeleteModal = false" @keydown.esc="showDeleteModal = false">
      <div class="modal-box">
        <h3 class="font-bold text-lg text-error">确认删除</h3>
        <p class="py-4">{{ deleteModalMessage }}</p>
        <div class="modal-action">
          <button @click="showDeleteModal = false" class="btn btn-sm">取消</button>
          <button @click="executeDelete" class="btn btn-sm btn-error" :disabled="isDeleting">确认删除</button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRoleManagement } from '@/composables/useRoleManagement';
import { canOperateAuditLifecycle } from '@/utils/auditLifecycleAccess';

const emit = defineEmits<{
  (e: 'count-updated', count: number): void;
}>();

interface AgentAuditFinding {
  id: string;
  conversation_id: string;
  finding_id: string;
  title: string;
  severity: string;
  severity_raw?: string;
  status: string;
  lifecycle_stage: string;
  verification_status: string;
  confidence?: number;
  cwe?: string;
  files: string[];
  source?: Record<string, any> | null;
  sink?: Record<string, any> | null;
  trace_path?: Array<Record<string, any>>;
  evidence?: string[];
  required_evidence?: string[];
  verifier?: Record<string, any> | null;
  judge?: Record<string, any> | null;
  provenance?: Record<string, any> | null;
  fix?: string;
  description: string;
  source_message_id?: string;
  hit_count: number;
  first_seen_at: string;
  last_seen_at: string;
  created_at: string;
  updated_at: string;
  last_transition_at?: string | null;
}

interface AgentAuditQualityGateMetrics {
  total_findings: number;
  with_evidence_count: number;
  uncertain_count: number;
  false_positive_or_rejected_count: number;
  evidence_rate: number;
  uncertain_rate: number;
  false_positive_rate: number;
  thresholds: {
    min_evidence_rate: number;
    max_uncertain_rate: number;
    max_false_positive_rate: number;
  };
  gate_passed: boolean;
}

const findings = ref<AgentAuditFinding[]>([]);
const isLoading = ref(false);
const showDetailsModal = ref(false);
const selectedFinding = ref<AgentAuditFinding | null>(null);
const selectedIds = ref<string[]>([]);
const isDeleting = ref(false);
const isTransitioningLifecycle = ref(false);
const isLoadingQualityGate = ref(false);
const isSavingThresholds = ref(false);
const qualityGateMetrics = ref<AgentAuditQualityGateMetrics | null>(null);
const thresholdDraft = ref({
  min_evidence_rate: 0.7,
  max_uncertain_rate: 0.3,
  max_false_positive_rate: 0.2,
});
const thresholdScope = ref<'global' | 'conversation'>('global');
const thresholdConversationId = ref('');

const showDeleteModal = ref(false);
const deleteModalMessage = ref('');
const deleteActionType = ref<'single' | 'selected' | 'all'>('single');
const findingToDelete = ref<AgentAuditFinding | null>(null);

const currentPage = ref(1);
const pageSize = 10;
const totalCount = ref(0);

const filters = ref({
  severity: '',
  status: '',
  lifecycleStage: '',
  conversationId: '',
  search: '',
});

const lifecycleTargetStage = ref('confirmed');
const lifecycleTargetVerification = ref('');
const lifecycleAccessOverride = ref(false);
const { selectedRole, loadRoles } = useRoleManagement();

const hasLifecycleTransitionPermission = computed(() => {
  return canOperateAuditLifecycle(selectedRole.value, lifecycleAccessOverride.value);
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
  isLoadingQualityGate.value = true;
  try {
    const countResponse = await invoke<any>('count_agent_audit_findings', {
      severityFilter: filters.value.severity || null,
      statusFilter: filters.value.status || null,
      lifecycleStageFilter: filters.value.lifecycleStage || null,
      conversationId: filters.value.conversationId || null,
      search: filters.value.search || null,
    });

    if (countResponse.success) {
      totalCount.value = countResponse.data;
      emit('count-updated', countResponse.data);
    }

    const qualityResp = await invoke<any>('get_agent_audit_quality_gate_metrics', {
      severityFilter: filters.value.severity || null,
      statusFilter: filters.value.status || null,
      lifecycleStageFilter: filters.value.lifecycleStage || null,
      conversationId: filters.value.conversationId || null,
      search: filters.value.search || null,
    });
    if (qualityResp?.success) {
      qualityGateMetrics.value = qualityResp.data || null;
      if (qualityResp.data?.thresholds) {
        thresholdDraft.value = { ...qualityResp.data.thresholds };
      }
    } else {
      qualityGateMetrics.value = null;
    }

    const offset = (currentPage.value - 1) * pageSize;
    const listResponse = await invoke<any>('list_agent_audit_findings', {
      limit: pageSize,
      offset,
      severityFilter: filters.value.severity || null,
      statusFilter: filters.value.status || null,
      lifecycleStageFilter: filters.value.lifecycleStage || null,
      conversationId: filters.value.conversationId || null,
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
    qualityGateMetrics.value = null;
  } finally {
    isLoading.value = false;
    isLoadingQualityGate.value = false;
  }
};

const applyFilters = () => {
  currentPage.value = 1;
  refreshFindings();
};

const openDetails = (finding: AgentAuditFinding) => {
  selectedFinding.value = finding;
  lifecycleTargetStage.value = finding.lifecycle_stage || 'confirmed';
  lifecycleTargetVerification.value = '';
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

const openDeleteSingleModal = (finding: AgentAuditFinding) => {
  findingToDelete.value = finding;
  deleteActionType.value = 'single';
  deleteModalMessage.value = `确认删除漏洞「${finding.title}」吗？`;
  showDeleteModal.value = true;
};

const openDeleteSelectedModal = () => {
  if (selectedIds.value.length === 0) return;
  deleteActionType.value = 'selected';
  deleteModalMessage.value = `确认删除选中的 ${selectedIds.value.length} 条漏洞吗？`;
  showDeleteModal.value = true;
};

const openDeleteAllModal = () => {
  if (totalCount.value === 0) return;
  deleteActionType.value = 'all';
  deleteModalMessage.value = `确认删除所有相关代码审计漏洞吗？此操作不可逆！`;
  showDeleteModal.value = true;
};

const executeDelete = async () => {
  showDeleteModal.value = false;
  if (deleteActionType.value === 'single') {
    await executeDeleteSingle();
  } else if (deleteActionType.value === 'selected') {
    await executeDeleteSelected();
  } else if (deleteActionType.value === 'all') {
    await executeDeleteAll();
  }
};

const executeDeleteSingle = async () => {
  const finding = findingToDelete.value;
  if (!finding) return;
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

const executeDeleteSelected = async () => {
  if (selectedIds.value.length === 0) return;
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

const executeDeleteAll = async () => {
  if (totalCount.value === 0) return;
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

const transitionLifecycle = async () => {
  if (!selectedFinding.value) return;
  if (!hasLifecycleTransitionPermission.value) return;
  isTransitioningLifecycle.value = true;
  try {
    const resp = await invoke<any>('transition_agent_audit_finding_lifecycle', {
      request: {
        finding_id: selectedFinding.value.id,
        lifecycle_stage: lifecycleTargetStage.value,
        verification_status: lifecycleTargetVerification.value || null,
        provenance: {
          source: 'security_center_ui',
          at: new Date().toISOString(),
        },
      },
    });

    if (!resp?.success) {
      throw new Error(resp?.error || 'transition_agent_audit_finding_lifecycle failed');
    }

    await refreshFindings();
    if (selectedFinding.value) {
      const updated = findings.value.find((item) => item.id === selectedFinding.value?.id);
      if (updated) {
        selectedFinding.value = updated;
      }
    }
  } catch (error) {
    console.error('Failed to transition finding lifecycle:', error);
  } finally {
    isTransitioningLifecycle.value = false;
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

const formatPct = (value?: number | null) => {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-';
  return `${(value * 100).toFixed(1)}%`;
};

const clamp01 = (value: number) => {
  if (Number.isNaN(value)) return 0;
  return Math.min(1, Math.max(0, value));
};

const resetThresholdDraft = () => {
  if (qualityGateMetrics.value?.thresholds) {
    thresholdDraft.value = { ...qualityGateMetrics.value.thresholds };
    return;
  }
  thresholdDraft.value = {
    min_evidence_rate: 0.7,
    max_uncertain_rate: 0.3,
    max_false_positive_rate: 0.2,
  };
};

const resolveThresholdConversationId = () => {
  if (thresholdScope.value !== 'conversation') return null;
  const cid = thresholdConversationId.value.trim();
  return cid.length ? cid : null;
};

const loadThresholdDraft = async () => {
  try {
    const resp = await invoke<any>('get_agent_audit_quality_gate_thresholds', {
      conversationId: resolveThresholdConversationId(),
    });
    if (resp?.success && resp?.data) {
      thresholdDraft.value = { ...resp.data };
    }
  } catch (error) {
    console.error('Failed to load quality gate thresholds:', error);
  }
};

const saveThresholds = async () => {
  isSavingThresholds.value = true;
  try {
    const payload = {
      min_evidence_rate: clamp01(Number(thresholdDraft.value.min_evidence_rate)),
      max_uncertain_rate: clamp01(Number(thresholdDraft.value.max_uncertain_rate)),
      max_false_positive_rate: clamp01(Number(thresholdDraft.value.max_false_positive_rate)),
    };
    const resp = await invoke<any>('save_agent_audit_quality_gate_thresholds', {
      thresholds: payload,
      conversationId: resolveThresholdConversationId(),
    });
    if (!resp?.success) {
      throw new Error(resp?.error || 'save_agent_audit_quality_gate_thresholds failed');
    }
    thresholdDraft.value = { ...resp.data };
    await refreshFindings();
  } catch (error) {
    console.error('Failed to save quality gate thresholds:', error);
  } finally {
    isSavingThresholds.value = false;
  }
};

const handleRefresh = () => {
  refreshFindings();
};

watch(currentPage, () => {
  refreshFindings();
});

watch([thresholdScope, thresholdConversationId], () => {
  loadThresholdDraft();
});

onMounted(() => {
  lifecycleAccessOverride.value = localStorage.getItem('security:auditLifecycleTransitionEnabled') === 'true';
  loadRoles().catch((error) => {
    console.warn('Failed to load roles for lifecycle permission check:', error);
  });
  refreshFindings();
  loadThresholdDraft();
  window.addEventListener('security-center-refresh', handleRefresh);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
});
</script>
