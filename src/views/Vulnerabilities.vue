<template>
  <div class="p-6">
    <!-- 统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat-card">
        <div class="text-sm text-gray-500">{{ $t('common.critical') }}</div>
        <div class="text-2xl font-bold text-red-500">{{ stats.critical }}</div>
      </div>
      <div class="stat-card">
        <div class="text-sm text-gray-500">{{ $t('common.high') }}</div>
        <div class="text-2xl font-bold text-orange-500">{{ stats.high }}</div>
      </div>
      <div class="stat-card">
        <div class="text-sm text-gray-500">{{ $t('common.medium') }}</div>
        <div class="text-2xl font-bold text-yellow-500">{{ stats.medium }}</div>
      </div>
      <div class="stat-card">
        <div class="text-sm text-gray-500">{{ $t('common.low') }}</div>
        <div class="text-2xl font-bold text-green-500">{{ stats.low }}</div>
      </div>
    </div>

    <!-- 搜索和筛选 -->
    <div class="card bg-base-100 shadow-lg mb-6">
      <div class="card-body">
        <div class="flex flex-col lg:flex-row gap-4 items-center justify-between">
          <div class="flex flex-col sm:flex-row gap-2 w-full lg:w-auto">
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="$t('common.search') + '...'"
              class="input input-bordered w-full sm:w-64"
            />
            <select v-model="severityFilter" class="select select-bordered">
              <option value="">{{ $t('vulnerabilities.filters.all') }}</option>
              <option value="Critical">{{ $t('common.critical') }}</option>
              <option value="High">{{ $t('common.high') }}</option>
              <option value="Medium">{{ $t('common.medium') }}</option>
              <option value="Low">{{ $t('common.low') }}</option>
            </select>
          </div>
          <div class="flex gap-2">
            <button 
              @click="refreshData" 
              class="btn btn-outline btn-primary"
            >
              <i class="fas fa-sync-alt mr-2"></i>{{ $t('common.refresh') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 批量操作 -->
    <div v-if="selectedVulns.length > 0" class="alert alert-info mb-4">
      <div class="flex justify-between items-center w-full">
        <span>{{ $t('vulnerabilities.selected', { count: selectedVulns.length }) }}</span>
        <div class="flex gap-2">
          <button 
            @click="batchUpdateStatus('Resolved')" 
            class="btn btn-sm btn-success"
          >
            {{ $t('vulnerabilities.markAsFixed') }}
          </button>
          <button 
            @click="batchExport()" 
            class="btn btn-sm btn-primary"
          >
            {{ $t('vulnerabilities.export') }}
          </button>
          <button 
            @click="clearSelection()" 
            class="btn btn-sm btn-ghost"
          >
            {{ $t('common.cancel') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 漏洞列表 -->
    <div class="card bg-base-100 shadow-lg">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">{{ $t('vulnerabilities.allVulnerabilities') }}</h2>
          <div class="flex items-center gap-2">
            <input
              type="checkbox"
              class="checkbox"
              :checked="selectedVulns.length === filteredVulns.length && filteredVulns.length > 0"
              @change="toggleSelectAll"
            />
            <span class="text-sm text-gray-500">{{ $t('common.all') }}</span>
          </div>
        </div>
        
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>{{ $t('common.select') }}</th>
                <th>{{ $t('common.id') }}</th>
                <th>{{ $t('vulnerabilities.vulnerabilityName') }}</th>
                <th>{{ $t('common.severity') }}</th>
                <th>{{ $t('common.status') }}</th>
                <th>{{ $t('vulnerabilities.vulnerabilityType') }}</th>
                <th>{{ $t('vulnerabilities.discoveryDate') }}</th>
                <th>{{ $t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="vuln in filteredVulns" :key="vuln.id">
                <td>
                  <input
                    type="checkbox"
                    class="checkbox"
                    :checked="selectedVulns.includes(vuln.id)"
                    @change="toggleSelect(vuln.id)"
                  />
                </td>
                <td class="font-mono text-sm">{{ vuln.id }}</td>
                <td>
                  <div class="font-medium">{{ vuln.title }}</div>
                  <div class="text-sm text-gray-500 truncate max-w-xs">
                    {{ vuln.description }}
                  </div>
                </td>
                <td>
                  <div class="badge" :class="getSeverityClass(vuln.severity)">
                    {{ getSeverityText(vuln.severity) }}
                  </div>
                </td>
                <td>
                  <div class="badge" :class="getStatusClass(vuln.status)">
                    {{ getStatusText(vuln.status) }}
                  </div>
                </td>
                <td>{{ vuln.category }}</td>
                <td class="text-sm text-gray-500">
                  {{ formatDate(vuln.discovered_at) }}
                </td>
                <td>
                  <div class="flex gap-1">
                    <button 
                      @click="viewDetails(vuln)" 
                      class="btn btn-ghost btn-xs"
                      :title="$t('common.viewDetails')"
                    >
                      <i class="fas fa-eye"></i>
                    </button>
                    <button 
                      @click="exportVulnReport(vuln)" 
                      class="btn btn-ghost btn-xs"
                      :title="$t('vulnerabilities.export')"
                    >
                      <i class="fas fa-download"></i>
                    </button>
                    <div class="dropdown dropdown-end">
                      <label tabindex="0" class="btn btn-ghost btn-xs">
                        <i class="fas fa-ellipsis-v"></i>
                      </label>
                      <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
                        <li><a @click="updateVulnStatus(vuln.id, 'Open')">{{ $t('vulnerabilities.filters.open') }}</a></li>
                        <li><a @click="updateVulnStatus(vuln.id, 'InProgress')">{{ $t('vulnerabilities.filters.inProgress') }}</a></li>
                        <li><a @click="updateVulnStatus(vuln.id, 'Resolved')">{{ $t('vulnerabilities.filters.fixed') }}</a></li>
                        <li><a @click="deleteVuln(vuln.id)" class="text-red-500">{{ $t('common.delete') }}</a></li>
                      </ul>
                    </div>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          
          <div v-if="filteredVulns.length === 0" class="text-center py-8 text-gray-500">
            {{ $t('common.noData') }}
          </div>
        </div>
      </div>
    </div>

    <!-- 详情模态框 -->
    <div v-if="showModal" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('vulnerabilities.vulnerabilityDetails') }}</h3>
        
        <div v-if="selectedVuln" class="space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ $t('common.id') }}</span>
              </label>
              <div class="text-sm font-mono bg-gray-100 p-2 rounded">
                {{ selectedVuln.id }}
              </div>
            </div>
            <div>
              <label class="label">
                <span class="label-text font-medium">{{ $t('common.severity') }}</span>
              </label>
              <div class="badge" :class="getSeverityClass(selectedVuln.severity)">
                {{ getSeverityText(selectedVuln.severity) }}
              </div>
            </div>
          </div>

          <div>
            <label class="label">
              <span class="label-text font-medium">{{ $t('vulnerabilities.vulnerabilityName') }}</span>
            </label>
            <div class="text-lg font-medium">{{ selectedVuln.title }}</div>
          </div>

          <div>
            <label class="label">
              <span class="label-text font-medium">{{ $t('common.description') }}</span>
            </label>
            <div class="bg-gray-50 p-3 rounded">{{ selectedVuln.description }}</div>
          </div>

          <div v-if="selectedVuln.poc">
            <label class="label">
              <span class="label-text font-medium">{{ $t('vulnerabilities.proofOfConcept') }}</span>
            </label>
            <pre class="bg-gray-900 text-green-400 p-3 rounded text-sm overflow-x-auto">{{ selectedVuln.poc }}</pre>
          </div>

          <div v-if="selectedVuln.remediation">
            <label class="label">
              <span class="label-text font-medium">{{ $t('vulnerabilities.remediationSteps') }}</span>
            </label>
            <div class="bg-blue-50 border-l-4 border-blue-400 p-3">
              <pre class="whitespace-pre-wrap text-sm">{{ selectedVuln.remediation }}</pre>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showModal = false" class="btn">{{ $t('common.close') }}</button>
          <button 
            v-if="selectedVuln" 
            @click="exportVulnReport(selectedVuln)" 
            class="btn btn-primary"
          >
            {{ $t('vulnerabilities.export') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { dialog } from '@/composables/useDialog';

const { t } = useI18n();

interface VulnData {
  id: string;
  title: string;
  severity: string;
  status: string;
  description: string;
  discovered_at: string;
  category?: string;
  poc?: string;
  remediation?: string;
}

const vulnerabilities = ref<VulnData[]>([
  {
    id: 'vuln-002',
    title: t('vulnerabilities.types.sqli'),
    severity: 'High',
    status: 'InProgress',
    description: t('vulnerabilities.description'),
    category: t('vulnerabilities.types.sqli'),
    poc: 'POST /api/user\\nContent-Type: application/json\\n\\n{"id": "1 OR 1=1--"}\\n\\nReturned all user data',
    remediation: '1. Use parameterized queries\\n2. Input validation\\n3. Database access control',
    discovered_at: new Date(Date.now() - 1000 * 60 * 60 * 3).toISOString()
  },
  {
    id: 'vuln-003',
    title: t('vulnerabilities.types.infoDisclosure'),
    severity: 'Low',
    status: 'Resolved',
    description: t('vulnerabilities.description'),
    category: t('vulnerabilities.types.infoDisclosure'),
    poc: 'GET /.env\\nHost: example.com\\n\\nReturned config file with database credentials',
    remediation: '1. Remove sensitive files from web root\\n2. Configure web server access\\n3. Use environment variables',
    discovered_at: new Date(Date.now() - 1000 * 60 * 60 * 6).toISOString()
  }
]);

const stats = ref({
  critical: 0,
  high: 1,
  medium: 1,
  low: 1
});

const searchQuery = ref('');
const severityFilter = ref('');
const showModal = ref(false);
const selectedVuln = ref<VulnData | null>(null);
const selectedVulns = ref<string[]>([]);

const filteredVulns = computed(() => {
  return vulnerabilities.value.filter(vuln => {
    const matchesSearch = vuln.title.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesSeverity = !severityFilter.value || vuln.severity === severityFilter.value;
    return matchesSearch && matchesSeverity;
  });
});

const refreshData = () => {
  console.log(t('common.refresh'));
};

const viewDetails = (vuln: VulnData) => {
  selectedVuln.value = vuln;
  showModal.value = true;
};

const deleteVuln = async (vulnId: string) => {
  const confirmed = await dialog.confirm(t('common.confirm'));
  if (confirmed) {
    vulnerabilities.value = vulnerabilities.value.filter(vuln => vuln.id !== vulnId);
  }
};

const getSeverityClass = (severity: string) => {
  switch (severity) {
    case 'Critical': return 'badge-error';
    case 'High': return 'badge-warning';
    case 'Medium': return 'badge-info';
    case 'Low': return 'badge-success';
    default: return 'badge-ghost';
  }
};

const getStatusClass = (status: string) => {
  switch (status) {
    case 'Open': return 'badge-error';
    case 'InProgress': return 'badge-warning';
    case 'Resolved': return 'badge-success';
    default: return 'badge-ghost';
  }
};

const getSeverityText = (severity: string) => {
  const labels: Record<string, string> = {
    'Critical': t('common.critical'),
    'High': t('common.high'),
    'Medium': t('common.medium'),
    'Low': t('common.low')
  };
  return labels[severity] || severity;
};

const getStatusText = (status: string) => {
  const labels: Record<string, string> = {
    'Open': t('vulnerabilities.filters.open'),
    'InProgress': t('vulnerabilities.filters.inProgress'),
    'Resolved': t('vulnerabilities.filters.fixed')
  };
  return labels[status] || status;
};

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const hours = Math.floor(diff / (1000 * 60 * 60));
  
  if (hours > 24) return `${Math.floor(hours / 24)}${t('common.date')}`;
  if (hours > 0) return `${hours}${t('common.time')}`;
  return t('common.time');
};

const exportVulnReport = (vuln: VulnData) => {
  console.log(t('vulnerabilities.export'), vuln.id);
  // 这里将来会调用后端API导出报告
};

const updateVulnStatus = (vulnId: string, newStatus: string) => {
  const vuln = vulnerabilities.value.find(v => v.id === vulnId);
  if (vuln) {
    vuln.status = newStatus;
    console.log(t('common.status'), vulnId, newStatus);
    // 这里将来会调用后端API更新状态
  }
};

const batchUpdateStatus = (newStatus: string) => {
  selectedVulns.value.forEach(vulnId => {
    updateVulnStatus(vulnId, newStatus);
  });
  clearSelection();
};

const batchExport = () => {
  console.log(t('vulnerabilities.export'), selectedVulns.value);
  // 这里将来会调用后端API批量导出
  clearSelection();
};

const clearSelection = () => {
  selectedVulns.value = [];
};

const toggleSelect = (vulnId: string) => {
  const index = selectedVulns.value.indexOf(vulnId);
  if (index > -1) {
    selectedVulns.value.splice(index, 1);
  } else {
    selectedVulns.value.push(vulnId);
  }
};

const toggleSelectAll = () => {
  if (selectedVulns.value.length === filteredVulns.value.length) {
    clearSelection();
  } else {
    selectedVulns.value = filteredVulns.value.map(vuln => vuln.id);
  }
};

onMounted(() => {
  refreshData();
});
</script>

<style scoped>
.stat-card {
  @apply card bg-base-100 shadow-lg p-6;
}
</style>