<template>
  <div class="space-y-6">
    <!-- 页面标题和操作 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('projects.title') }}</h1>
      <div class="flex space-x-2">
        <button @click="showAddModal = true" class="btn btn-primary">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
          {{ $t('projects.newProject') }}
        </button>
        <button @click="syncProjects" class="btn btn-outline">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 筛选和搜索 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="flex flex-wrap gap-4 items-center">
        <div class="form-control">
          <input 
            v-model="searchQuery" 
            type="text" 
            :placeholder="$t('common.search') + '...'" 
            class="input input-bordered input-sm w-64"
          />
        </div>
        <div class="form-control">
          <select v-model="platformFilter" class="select select-bordered select-sm">
            <option value="">{{ $t('projects.filters.all') }} {{ $t('projects.platform').toLowerCase() }}</option>
            <option value="HackerOne">HackerOne</option>
            <option value="Bugcrowd">Bugcrowd</option>
            <option value="Intigriti">Intigriti</option>
            <option value="YesWeHack">YesWeHack</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="statusFilter" class="select select-bordered select-sm">
            <option value="">{{ $t('projects.filters.all') }} {{ $t('projects.status').toLowerCase() }}</option>
            <option value="Active">{{ $t('projects.filters.active') }}</option>
            <option value="Paused">{{ $t('common.pending') }}</option>
            <option value="Closed">{{ $t('projects.filters.completed') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="sortBy" class="select select-bordered select-sm">
            <option value="reward_desc">{{ $t('projects.earnings') }} ({{ $t('common.high') }} - {{ $t('common.low') }})</option>
            <option value="reward_asc">{{ $t('projects.earnings') }} ({{ $t('common.low') }} - {{ $t('common.high') }})</option>
            <option value="name_asc">{{ $t('common.name') }} A-Z</option>
            <option value="updated_desc">{{ $t('common.update') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 项目网格 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div v-for="project in filteredProjects" :key="project.id" class="project-card">
        <div class="flex items-start justify-between mb-3">
          <div class="flex items-center space-x-2">
            <img :src="project.logo" :alt="project.name" class="w-8 h-8 rounded">
            <div>
              <h3 class="font-semibold text-sm">{{ project.name }}</h3>
              <div class="badge badge-outline badge-xs">{{ project.platform }}</div>
            </div>
          </div>
          <div :class="getStatusBadgeClass(project.status)" class="badge badge-xs">
            {{ getStatusLabel(project.status) }}
          </div>
        </div>

        <p class="text-xs opacity-70 mb-3 line-clamp-2">{{ project.description }}</p>

        <div class="grid grid-cols-2 gap-2 mb-3 text-xs">
          <div>
            <span class="opacity-70">{{ $t('projects.budget') }} {{ $t('common.low') }}:</span>
            <span class="font-medium text-success">${{ project.min_reward }}</span>
          </div>
          <div>
            <span class="opacity-70">{{ $t('projects.budget') }} {{ $t('common.high') }}:</span>
            <span class="font-medium text-success">${{ project.max_reward }}</span>
          </div>
          <div>
            <span class="opacity-70">{{ $t('projects.reportsSubmitted') }}:</span>
            <span class="font-medium">{{ project.participants }}</span>
          </div>
          <div>
            <span class="opacity-70">ROI:</span>
            <span :class="project.roi_score > 7 ? 'text-success' : project.roi_score > 4 ? 'text-warning' : 'text-error'" class="font-medium">
              {{ project.roi_score }}/10
            </span>
          </div>
        </div>

        <div class="flex flex-wrap gap-1 mb-3">
          <div v-for="tech in project.technologies.slice(0, 3)" :key="tech" class="badge badge-ghost badge-xs">
            {{ tech }}
          </div>
          <div v-if="project.technologies.length > 3" class="badge badge-ghost badge-xs">
            +{{ project.technologies.length - 3 }}
          </div>
        </div>

        <div class="flex space-x-2">
          <button @click="startScan(project)" class="btn btn-primary btn-xs flex-1">
            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
            </svg>
            {{ $t('scanTasks.newScan') }}
          </button>
          <button @click="viewProject(project)" class="btn btn-outline btn-xs">
            <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
              <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"></path>
              <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- 添加项目模态框 -->
    <div v-if="showAddModal" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('projects.newProject') }}</h3>
        
        <form @submit.prevent="addProject" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('projects.projectName') }}</span>
            </label>
            <input 
              v-model="newProject.name" 
              type="text" 
              :placeholder="$t('projects.form.namePlaceholder')" 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('projects.platform') }}</span>
            </label>
            <select v-model="newProject.platform" class="select select-bordered" required>
              <option value="">{{ $t('projects.form.selectType') }}</option>
              <option value="HackerOne">HackerOne</option>
              <option value="Bugcrowd">Bugcrowd</option>
              <option value="Intigriti">Intigriti</option>
              <option value="YesWeHack">YesWeHack</option>
              <option value="Custom">{{ $t('common.other') }}</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('projects.projectName') }} URL</span>
            </label>
            <input 
              v-model="newProject.url" 
              type="url" 
              placeholder="https://..." 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('projects.budget') }} {{ $t('common.low') }} ($)</span>
              </label>
              <input 
                v-model.number="newProject.min_reward" 
                type="number" 
                min="0" 
                class="input input-bordered" 
                required
              />
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('projects.budget') }} {{ $t('common.high') }} ($)</span>
              </label>
              <input 
                v-model.number="newProject.max_reward" 
                type="number" 
                min="0" 
                class="input input-bordered" 
                required
              />
            </div>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('common.description') }}</span>
            </label>
            <textarea 
              v-model="newProject.description" 
              class="textarea textarea-bordered" 
              rows="3"
              :placeholder="$t('projects.form.scopePlaceholder')"
            ></textarea>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('common.type') }}</span>
            </label>
            <input 
              v-model="technologiesInput" 
              type="text" 
              placeholder="React, Node.js, MongoDB..." 
              class="input input-bordered"
            />
          </div>

          <div class="modal-action">
            <button type="button" @click="showAddModal = false" class="btn">{{ $t('common.cancel') }}</button>
            <button type="submit" class="btn btn-primary" :disabled="isAdding">
              <span v-if="isAdding" class="loading loading-spinner loading-sm"></span>
              {{ isAdding ? $t('common.loading') : $t('common.add') + ' ' + $t('projects.projectName') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 项目详情模态框 -->
    <div v-if="showDetailsModal && selectedProject" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ selectedProject.name }}</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('projects.projectDetails') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('projects.platform') }}:</span> {{ selectedProject.platform }}</div>
                <div><span class="opacity-70">{{ $t('projects.status') }}:</span> 
                  <div :class="getStatusBadgeClass(selectedProject.status)" class="badge badge-sm">
                    {{ getStatusLabel(selectedProject.status) }}
                  </div>
                </div>
                <div><span class="opacity-70">{{ $t('projects.budget') }}:</span> ${{ selectedProject.min_reward }} - ${{ selectedProject.max_reward }}</div>
                <div><span class="opacity-70">{{ $t('projects.reportsSubmitted') }}:</span> {{ selectedProject.participants }}</div>
                <div><span class="opacity-70">ROI {{ $t('common.statistics') }}:</span> {{ selectedProject.roi_score }}/10</div>
              </div>
            </div>

            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.type') }}</h4>
              <div class="flex flex-wrap gap-1">
                <div v-for="tech in selectedProject.technologies" :key="tech" class="badge badge-outline badge-sm">
                  {{ tech }}
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.description') }}</h4>
              <div class="bg-base-200 p-3 rounded text-sm">
                {{ selectedProject.description }}
              </div>
            </div>

            <div v-if="selectedProject.scope">
              <h4 class="font-semibold mb-2">{{ $t('projects.scope') }}</h4>
              <div class="bg-base-200 p-3 rounded text-sm">
                <div v-for="scope in selectedProject.scope" :key="scope" class="mb-1">
                  • {{ scope }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showDetailsModal = false" class="btn">{{ $t('common.close') }}</button>
          <button @click="startScan(selectedProject)" class="btn btn-primary">{{ $t('scanTasks.newScan') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

// 类型定义
interface Project {
  id: string;
  name: string;
  platform: string;
  url: string;
  status: string;
  description: string;
  min_reward: number;
  max_reward: number;
  participants: number;
  roi_score: number;
  technologies: string[];
  logo: string;
  scope?: string[];
}

// 响应式数据
const projects = ref<Project[]>([
  {
    id: 'proj-1',
    name: 'Example Corp',
    platform: 'HackerOne',
    url: 'https://example.com',
    status: 'Active',
    description: '一个大型电商平台，专注于安全测试和漏洞发现',
    min_reward: 100,
    max_reward: 5000,
    participants: 1250,
    roi_score: 8.5,
    technologies: ['React', 'Node.js', 'MongoDB', 'Redis'],
    logo: 'https://via.placeholder.com/32x32',
    scope: ['*.example.com', 'api.example.com', 'mobile apps']
  },
  {
    id: 'proj-2',
    name: 'TechStart',
    platform: 'Bugcrowd',
    url: 'https://techstart.io',
    status: 'Active',
    description: '创新的科技初创公司，寻找Web和移动应用的安全问题',
    min_reward: 50,
    max_reward: 2000,
    participants: 680,
    roi_score: 6.2,
    technologies: ['Vue.js', 'Python', 'PostgreSQL'],
    logo: 'https://via.placeholder.com/32x32'
  },
  {
    id: 'proj-3',
    name: 'SecureBank',
    platform: 'Intigriti',
    url: 'https://securebank.com',
    status: 'Paused',
    description: '金融科技公司，重点关注支付系统和用户数据保护',
    min_reward: 200,
    max_reward: 10000,
    participants: 2100,
    roi_score: 9.1,
    technologies: ['Angular', 'Java', 'Oracle', 'Kubernetes'],
    logo: 'https://via.placeholder.com/32x32'
  }
]);

const searchQuery = ref('');
const platformFilter = ref('');
const statusFilter = ref('');
const sortBy = ref('reward_desc');
const showAddModal = ref(false);
const showDetailsModal = ref(false);
const selectedProject = ref<Project | null>(null);
const isAdding = ref(false);

const newProject = ref({
  name: '',
  platform: '',
  url: '',
  description: '',
  min_reward: 0,
  max_reward: 0
});

const technologiesInput = ref('');

// 计算属性
const filteredProjects = computed(() => {
  const filtered = projects.value.filter(project => {
    const matchesSearch = project.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
                         project.description.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesPlatform = !platformFilter.value || project.platform === platformFilter.value;
    const matchesStatus = !statusFilter.value || project.status === statusFilter.value;
    
    return matchesSearch && matchesPlatform && matchesStatus;
  });

  // 排序
  return filtered.sort((a, b) => {
    switch (sortBy.value) {
      case 'reward_desc':
        return b.max_reward - a.max_reward;
      case 'reward_asc':
        return a.max_reward - b.max_reward;
      case 'name_asc':
        return a.name.localeCompare(b.name);
      case 'updated_desc':
        return 0; // 这里需要实际的更新时间
      default:
        return 0;
    }
  });
});

// 方法
const syncProjects = async () => {
  try {
    // 这里将来会调用后端API同步项目
    console.log('同步项目数据...');
  } catch (error) {
    console.error('同步失败:', error);
  }
};

const addProject = async () => {
  isAdding.value = true;
  try {
    const projectData = {
      ...newProject.value,
      technologies: technologiesInput.value.split(',').map(t => t.trim()).filter(t => t)
    };

    const response = await invoke('add_project', { projectData });
    console.log('项目添加成功:', response);
    
    showAddModal.value = false;
    
    // 重置表单
    newProject.value = {
      name: '',
      platform: '',
      url: '',
      description: '',
      min_reward: 0,
      max_reward: 0
    };
    technologiesInput.value = '';
    
  } catch (error) {
    console.error('添加项目失败:', error);
    alert(t('common.error'));
  } finally {
    isAdding.value = false;
  }
};

const viewProject = (project: Project) => {
  selectedProject.value = project;
  showDetailsModal.value = true;
};

const startScan = async (project: Project) => {
  try {
    const response = await invoke('create_scan_task', {
      target: project.url,
      config: {
        tools: ['subfinder', 'httpx'],
        depth: 2,
        timeout: 120,
        include_subdomains: true
      }
    });
    
    console.log('扫描任务创建成功:', response);
    alert(t('scanTasks.notifications.scanStarted') + ': ' + project.name);
  } catch (error) {
    console.error('创建扫描任务失败:', error);
    alert(t('scanTasks.notifications.scanFailed'));
  }
};

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Active': return 'badge-success';
    case 'Paused': return 'badge-warning';
    case 'Closed': return 'badge-error';
    default: return 'badge-ghost';
  }
};

const getStatusLabel = (status: string) => {
  switch (status) {
    case 'Active': return t('projects.filters.active');
    case 'Paused': return t('common.pending');
    case 'Closed': return t('projects.filters.completed');
    default: return status;
  }
};

// 生命周期
onMounted(() => {
  syncProjects();
});
</script>

<style scoped>
.project-card {
  @apply bg-base-100 rounded-lg shadow-sm border border-base-300 p-4;
}

.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style> 