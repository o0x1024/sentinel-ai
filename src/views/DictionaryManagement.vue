<template>
  <div class="dictionary-management-page page-content-padded safe-top">
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">{{ t('dictionary.title', '字典管理') }}</h2>
      <div class="flex gap-2">
        <button 
          class="btn btn-primary" 
          @click="showCreateModal = true"
        >
          <i class="fas fa-plus mr-2"></i>
          {{ t('dictionary.createNew', '新建字典') }}
        </button>
        <button 
          class="btn btn-secondary" 
          @click="initializeBuiltinDictionaries"
          :disabled="initializing"
        >
          <i class="fas fa-download mr-2"></i>
          {{ initializing ? t('dictionary.initializing', '初始化中...') : t('dictionary.initBuiltin', '初始化内置字典') }}
        </button>
      </div>
    </div>

    <!-- 字典分组导航 -->
    <div class="dictionary-group-nav mb-6">
      <div class="dictionary-group-bar">
        <button
          v-for="group in categoryGroups"
          :key="group.key"
          class="dict-group-btn"
          :class="{ active: activeGroup === group.key }"
          @click="onGroupChange(group.key)"
        >
          <i :class="group.icon"></i>
          <span>{{ group.label }}</span>
          <span v-if="group.count" class="dict-group-count">{{ group.count }}</span>
        </button>
      </div>

      <!-- 二级类型筛选 chips -->
      <div v-if="activeGroupTypes.length > 1" class="dictionary-type-chips">
        <button
          class="dict-type-chip"
          :class="{ active: selectedType === 'all' && activeGroup !== 'all' }"
          @click="onTypeChange('all')"
        >
          {{ t('dictionary.types.all', '全部') }}
        </button>
        <button
          v-for="type in activeGroupTypes"
          :key="type.value"
          class="dict-type-chip"
          :class="{ active: selectedType === type.value }"
          @click="onTypeChange(type.value)"
        >
          <i :class="type.icon" class="dict-chip-icon"></i>
          {{ t(`dictionary.types.${type.value}`, type.label) }}
        </button>
      </div>
    </div>

    <div class="mb-4 flex items-center gap-3">
      <span class="text-sm text-base-content/70">{{ t('dictionary.viewMode', '显示方式') }}</span>
      <div class="join">
        <button
          class="btn btn-xs join-item"
          :class="viewMode === 'list' ? 'btn-primary' : 'btn-ghost'"
          @click="viewMode = 'list'"
        >
          <i class="fas fa-list mr-1"></i>
          {{ t('dictionary.viewModes.list', '列表') }}
        </button>
        <button
          class="btn btn-xs join-item"
          :class="viewMode === 'card' ? 'btn-primary' : 'btn-ghost'"
          @click="viewMode = 'card'"
        >
          <i class="fas fa-th-large mr-1"></i>
          {{ t('dictionary.viewModes.card', '卡片') }}
        </button>
      </div>
    </div>

    <div v-if="loadingDictionaries" class="flex justify-center py-10">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <template v-else-if="dictionaries.length > 0">
      <DictionaryListTable
        v-if="viewMode === 'list'"
        :dictionaries="dictionaries"
        :default-map="defaultMap"
        :get-dictionary-type-label="getDictionaryTypeLabel"
        :get-dictionary-subtype-label="getDictionarySubtypeLabel"
        :get-dictionary-subtype-badge-class="getDictionarySubtypeBadgeClass"
        :get-service-type-label="getServiceTypeLabel"
        :format-date="formatDate"
        @edit="editDictionary"
        @export="exportDictionary"
        @duplicate="duplicateDictionary"
        @mark-default="markAsDefault"
        @clear-default="clearDefault"
        @delete="deleteDictionary"
        @view-words="viewDictionaryWords"
        @manage-words="manageDictionaryWords"
      />

      <DictionaryCardGrid
        v-else
        :dictionaries="dictionaries"
        :default-map="defaultMap"
        :get-dictionary-type-label="getDictionaryTypeLabel"
        :get-dictionary-subtype-label="getDictionarySubtypeLabel"
        :get-dictionary-subtype-badge-class="getDictionarySubtypeBadgeClass"
        :get-service-type-label="getServiceTypeLabel"
        :format-date="formatDate"
        @edit="editDictionary"
        @export="exportDictionary"
        @duplicate="duplicateDictionary"
        @mark-default="markAsDefault"
        @clear-default="clearDefault"
        @delete="deleteDictionary"
        @view-words="viewDictionaryWords"
        @manage-words="manageDictionaryWords"
      />
    </template>

    <div v-else class="rounded-lg border border-dashed border-base-300 py-12 text-center text-base-content/60 mb-6">
      {{ t('dictionary.empty', '暂无字典') }}
    </div>

    <div v-if="totalDictionaries > 0" class="flex flex-col gap-3 pt-2 xl:flex-row xl:items-center xl:justify-between">
      <div class="flex items-center gap-2 text-sm">
        <span class="text-base-content/70">{{ t('dictionary.pageSize', '每页显示') }}</span>
        <select v-model.number="pageSize" class="select select-bordered select-sm" :disabled="loadingDictionaries" @change="onPageSizeChange">
          <option v-for="size in pageSizeOptions" :key="size" :value="size">
            {{ size }}
          </option>
        </select>
      </div>

      <div class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
        <div class="join">
          <button class="join-item btn btn-sm" :disabled="currentPage <= 1 || loadingDictionaries" @click="goToFirstPage">
            {{ t('bugBounty.surface.inventory.firstPage', '首页') }}
          </button>
          <button class="join-item btn btn-sm" :disabled="currentPage <= 1 || loadingDictionaries" @click="goToPrevPage">
            {{ t('common.previous', '上一页') }}
          </button>
          <button class="join-item btn btn-sm">
            {{ t('bugBounty.surface.inventory.pageInfo', { page: currentPage, total: pageCount }) }}
          </button>
          <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount || loadingDictionaries" @click="goToNextPage">
            {{ t('common.next', '下一页') }}
          </button>
          <button class="join-item btn btn-sm" :disabled="currentPage >= pageCount || loadingDictionaries" @click="goToLastPage">
            {{ t('bugBounty.surface.inventory.lastPage', '末页') }}
          </button>
        </div>

        <div class="flex items-center gap-2">
          <input
            v-model="pageInput"
            type="number"
            min="1"
            :max="pageCount"
            class="input input-bordered input-sm w-24"
            :placeholder="t('bugBounty.surface.inventory.jumpPlaceholder', '跳转页码')"
            @keyup.enter="applyPageJump"
          />
          <button class="btn btn-sm btn-outline" :disabled="loadingDictionaries" @click="applyPageJump">
            {{ t('bugBounty.surface.inventory.jump', '跳转') }}
          </button>
        </div>
      </div>
    </div>

    <DictionaryFormModal
      :open="showCreateModal || Boolean(editingDictionary)"
      :is-editing="Boolean(editingDictionary)"
      :form="dictionaryForm"
      :dictionary-types="dictionaryTypes"
      :service-types="serviceTypes"
      :saving="saving"
      @cancel="closeModal"
      @save="saveDictionary"
    />

    <!-- 词条管理模态框 -->
    <Teleport to="body">
    <div v-if="managingDictionary" class="modal modal-open dictionary-modal" @click.self="closeManagedDictionaryWordsModal">
      <div class="modal-box max-w-4xl dictionary-modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ t('dictionary.manageWords', '管理词条') }} - {{ managingDictionary.name }}
        </h3>

        <div class="flex flex-wrap gap-2 mb-4">
          <div class="badge badge-primary">
            {{ getDictionaryTypeLabel(managingDictionary.dict_type) }}
          </div>
          <div
            v-if="getDictionarySubtypeLabel(managingDictionary)"
            class="badge"
            :class="getDictionarySubtypeBadgeClass(managingDictionary)"
          >
            {{ getDictionarySubtypeLabel(managingDictionary) }}
          </div>
          <div v-if="managingDictionary.service_type" class="badge badge-secondary">
            {{ getServiceTypeLabel(managingDictionary.service_type) }}
          </div>
          <div v-if="managingDictionary.is_builtin" class="badge badge-accent">
            {{ t('dictionary.builtin', '内置') }}
          </div>
        </div>
        
        <div class="flex gap-4 mb-4">
          <template v-if="!isStructuredManagingDictionary">
            <div class="form-control flex-1">
              <input 
                type="text" 
                v-model="newWord" 
                class="input input-bordered" 
                :placeholder="t('dictionary.addWordPlaceholder', '输入新词条')"
                @keyup.enter="addWord"
              >
            </div>
            <button class="btn btn-primary" @click="addWord" :disabled="!newWord.trim()">
              <i class="fas fa-plus mr-2"></i>
              {{ t('dictionary.addWord', '添加') }}
            </button>
          </template>
          <template v-else>
            <div class="alert alert-info flex-1">
              <span>规则类字典支持结构化编辑和 JSON 规则导入。</span>
            </div>
            <button class="btn btn-primary" @click="openRuleEditor()">
              <i class="fas fa-plus mr-2"></i>
              新增规则
            </button>
          </template>
          <button class="btn btn-secondary" @click="showImportModal = true">
            <i class="fas fa-upload mr-2"></i>
            {{ t('dictionary.import', '导入') }}
          </button>
          <button class="btn btn-accent" @click="showAiGenerateModal = true">
            <i class="fas fa-magic mr-2"></i>
            {{ isStructuredManagingDictionary ? 'AI生成规则' : 'AI生成字典' }}
          </button>
        </div>
        
        <div class="form-control mb-4">
          <input 
            type="text" 
            v-model="searchQuery" 
            class="input input-bordered" 
            :placeholder="t('dictionary.searchWords', '搜索词条...')"
          >
        </div>

        <StructuredRuleFilters
          v-if="isStructuredManagingDictionary"
          :dictionary-type="managingDictionary?.dict_type || ''"
          v-model:category="ruleCategoryFilter"
          v-model:severity="ruleSeverityFilter"
          v-model:matcher="ruleMatcherFilter"
          v-model:enabled="ruleEnabledFilter"
          v-model:service="ruleServiceFilter"
          v-model:probe-name="ruleProbeNameFilter"
          :category-options="structuredCategoryOptions"
          :severity-options="structuredSeverityOptions"
          :service-options="structuredServiceOptions"
          :probe-name-options="structuredProbeNameOptions"
        />

        <DictionaryEmptyRuleState
          v-if="isStructuredManagingDictionary && !isLoadingMore && listItems.length === 0"
          :subtype="managingDictionarySubtypeKey"
          :supports-nmap-import="supportsNmapServiceProbeImport"
          @import-nmap="importMethod = 'nmap'; showImportModal = true"
          @create-starter-rule="openRuleEditor()"
        />

        <StructuredRuleWordList
          v-if="isStructuredManagingDictionary && listItems.length > 0"
          v-model:selected-words="selectedWords"
          :items="listItems"
          :dictionary-type="managingDictionary?.dict_type || ''"
          :item-height="currentRowHeight"
          :height="LIST_HEIGHT"
          :is-loading-more="isLoadingMore"
          :has-more="hasMore"
          @edit="openRuleEditor"
          @remove="removeWord"
          @scroll="handleInfiniteScroll"
        />

        <div v-else class="border rounded-lg">
          <div class="sticky top-0 bg-base-200 px-4 py-2 items-center text-sm font-medium border-b dict-grid">
            <div>
              <input 
                type="checkbox" 
                class="checkbox" 
                :checked="selectedWords.length === listItems.length && listItems.length > 0"
                @change="toggleSelectAll"
              >
            </div>
            <div>{{ t('dictionary.word', '词条') }}</div>
            <div>{{ t('dictionary.addedAt', '添加时间') }}</div>
            <div class="text-right pr-2">{{ t('common.actions', '操作') }}</div>
          </div>

          <VirtualList
            ref="virtualListRef"
            :items="listItems"
            :itemHeight="currentRowHeight"
            :height="LIST_HEIGHT"
            class="virtual-list-host"
            keyField="id"
            @scroll="handleInfiniteScroll"
          >
            <template #default="{ item }">
              <div class="px-4 items-center text-sm h-full w-full dict-grid">
                <div class="py-2">
                  <input 
                    type="checkbox" 
                    class="checkbox" 
                    :value="item.id"
                    v-model="selectedWords"
                  >
                </div>
                <div class="min-w-0 py-2">
                  <div class="truncate whitespace-nowrap" :title="item.word">{{ item.word }}</div>
                </div>
                <div class="py-2 whitespace-nowrap text-base-content/80">{{ formatDate(item.created_at) }}</div>
                <div class="py-2 text-right pr-2 whitespace-nowrap">
                  <button 
                    class="btn btn-ghost btn-xs text-error" 
                    @click="removeWord(item.id)"
                  >
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </template>
          </VirtualList>

          <div class="px-4 py-2 text-center text-sm opacity-70">
            <span v-if="isLoadingMore">{{ t('common.loading', '加载中...') }}</span>
            <span v-else-if="!hasMore">{{ t('common.noMore', '没有更多了') }}</span>
          </div>
        </div>

        <DictionaryWordActionBar
          :is-structured="isStructuredManagingDictionary"
          :selected-count="selectedWords.length"
          @batch-edit="showBatchRuleEditor = true"
          @export-selected="exportSelectedRules"
          @remove-selected="removeSelectedWords"
          @clear="clearDictionary"
          @close="closeManagedDictionaryWordsModal"
        />
        
      </div>
    </div>
    </Teleport>

    <RuleEntryEditorModal
      :open="showRuleEditor"
      :dictionary-type="managingDictionary?.dict_type || ''"
      :dictionary-subtype="managingDictionary ? getDictionarySubtypeKey(managingDictionary) || '' : ''"
      :value="editingRuleEntry"
      @cancel="closeRuleEditor"
      @save="saveRuleEntry"
    />

    <RuleBatchEditModal
      :open="showBatchRuleEditor"
      :dictionary-type="managingDictionary?.dict_type || ''"
      :selected-count="selectedWords.length"
      @cancel="closeBatchRuleEditor"
      @save="applyBatchRuleEdit"
    />

    <DictionaryAIGenerateModal
      :open="showAiGenerateModal"
      :dictionary-name="managingDictionary?.name || ''"
      :dictionary-type="managingDictionary?.dict_type || ''"
      :is-structured="isStructuredManagingDictionary"
      :generating="aiGenerating"
      @cancel="closeAiGenerateModal"
      @generate="generateDictionaryEntries"
    />

    <!-- 导入模态框 -->
    <Teleport to="body">
    <div v-if="showImportModal" class="modal modal-open dictionary-modal" @click.self="showImportModal = false">
      <div class="modal-box dictionary-modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('dictionary.importWords', '导入词条') }}</h3>
        
        <div class="tabs tabs-boxed mb-4">
          <a class="tab" :class="{ 'tab-active': importMethod === 'text' }" @click="importMethod = 'text'">
            {{ t('dictionary.importFromText', '文本导入') }}
          </a>
          <a class="tab" :class="{ 'tab-active': importMethod === 'file' }" @click="importMethod = 'file'">
            {{ t('dictionary.importFromFile', '文件导入') }}
          </a>
          <a
            v-if="supportsNmapServiceProbeImport"
            class="tab"
            :class="{ 'tab-active': importMethod === 'nmap' }"
            @click="importMethod = 'nmap'"
          >
            {{ t('dictionary.importFromNmap', 'Nmap Service Probes') }}
          </a>
        </div>
        
        <div v-if="importMethod === 'text'">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('dictionary.pasteWords', '粘贴词条（每行一个）') }}</span>
            </label>
            <textarea 
              v-model="importText" 
              class="textarea textarea-bordered h-32" 
              :placeholder="t('dictionary.importTextPlaceholder', '每行输入一个词条...')"
            ></textarea>
          </div>
        </div>
        
        <div v-if="importMethod === 'file'">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('dictionary.selectFile', '选择文件') }}</span>
            </label>
            <input 
              type="file" 
              class="file-input file-input-bordered" 
              accept=".txt,.json,.csv"
              @change="handleFileSelect"
            >
          </div>
        </div>

        <div v-if="importMethod === 'nmap'">
          <div class="alert alert-info mb-4">
            <span>{{ t('dictionary.nmapImportHelp', '导入 nmap-service-probes 的 Probe/match/softmatch/ports/sslports 子集，并转换为服务识别规则。') }}</span>
          </div>
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('dictionary.selectFile', '选择文件') }}</span>
            </label>
            <input
              type="file"
              class="file-input file-input-bordered"
              accept=".txt,.probes,.conf"
              @change="handleFileSelect"
            >
          </div>
        </div>
        
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">{{ t('dictionary.mergeMode', '合并模式') }}</span>
          </label>
          <select v-model="mergeMode" class="select select-bordered">
            <option value="append">{{ t('dictionary.mergeModes.append', '追加（保留现有词条）') }}</option>
            <option value="replace">{{ t('dictionary.mergeModes.replace', '替换（清空后导入）') }}</option>
            <option value="merge">{{ t('dictionary.mergeModes.merge', '合并（去重）') }}</option>
          </select>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="showImportModal = false">{{ t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-primary" 
            @click="importWords" 
            :disabled="importing || (!importText.trim() && !selectedFile)"
          >
            {{ importing ? t('dictionary.importing', '导入中...') : t('dictionary.import', '导入') }}
          </button>
        </div>
      </div>
    </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { writeTextFile }  from '@tauri-apps/plugin-fs';
import DictionaryCardGrid from '@/components/Dictionary/DictionaryCardGrid.vue'
import DictionaryListTable from '@/components/Dictionary/DictionaryListTable.vue'
import DictionaryEmptyRuleState from '@/components/Dictionary/DictionaryEmptyRuleState.vue'
import DictionaryAIGenerateModal from '@/components/Dictionary/DictionaryAIGenerateModal.vue'
import DictionaryFormModal from '@/components/Dictionary/DictionaryFormModal.vue'
import VirtualList from '@/components/VirtualList.vue'
import DictionaryWordActionBar from '@/components/Dictionary/DictionaryWordActionBar.vue'
import RuleEntryEditorModal from '@/components/Dictionary/RuleEntryEditorModal.vue'
import RuleBatchEditModal from '@/components/Dictionary/RuleBatchEditModal.vue'
import StructuredRuleFilters from '@/components/Dictionary/StructuredRuleFilters.vue'
import StructuredRuleWordList from '@/components/Dictionary/StructuredRuleWordList.vue'
import { dialog } from '@/composables/useDialog'
import {
  getAllSubtypeOptions,
  getSubtypeOptions,
  getSubtypeBadgeClass,
  getSubtypeLabel,
} from '@/components/Dictionary/dictionarySubtypeConfig'
import {
  inferDictionarySubtypeKey,
  parseDictionaryMetadata,
} from '@/components/Dictionary/dictionarySubtypeUtils'
import { useDictionaryWordManager } from '@/composables/useDictionaryWordManager'
import { getDefaultMap as getDefaultMapApi, setDefaultId, clearDefaultForType } from '@/services/dictionary'

const { t } = useI18n()

// 类型定义
interface Dictionary {
  id: string;
  name: string;
  description?: string;
  dict_type: string;
  service_type?: string;
  is_builtin: boolean;
  is_active: boolean;
  word_count?: number;
  updated_at: string;
  created_at: string;
  category?: string;
  tags?: string[] | string;
  metadata?: string | null;
}

interface DictionaryPageResponse {
  items: Dictionary[]
  total: number
}

interface DictionaryForm {
  name: string;
  description: string;
  dictionary_type: string;
  service_type: string;
  subtype: string;
  is_active: boolean;
}

type DictionaryView = Pick<Dictionary, 'id' | 'name' | 'dict_type' | 'service_type' | 'category' | 'tags' | 'metadata'> & {
  is_builtin?: boolean;
}

interface GeneratedDictionaryEntry {
  word: string
  weight?: number | null
  category?: string | null
  metadata?: Record<string, any> | null
}

interface GenerateDictionaryEntriesResponse {
  entries: GeneratedDictionaryEntry[]
}

// 响应式数据
const dictionaries = ref<Dictionary[]>([])
const selectedType = ref('all')
const selectedSubtype = ref('')
const viewMode = ref<'list' | 'card'>('list')
const loadingDictionaries = ref(false)
const currentPage = ref(1)
const pageSize = ref(10)
const totalDictionaries = ref(0)
const pageInput = ref('1')
const showCreateModal = ref(false)
const editingDictionary = ref<Dictionary | null>(null)
const saving = ref(false)
const initializing = ref(false)
const showAiGenerateModal = ref(false)
const aiGenerating = ref(false)
// 默认字典映射：{ [dict_type]: dictionary_id }
const defaultMap = ref<Record<string, string>>({})
const pageSizeOptions = [10, 20, 50, 100]

// 表单数据
const dictionaryForm = ref({
  name: '',
  description: '',
  dictionary_type: '',
  service_type: '',
  subtype: '',
  is_active: true
})

// 字典类型定义
const dictionaryTypes = [
  { value: 'all', label: t('dictionary.types.all', '全部'), icon: 'fas fa-list' },
  { value: 'subdomain', label: t('dictionary.types.subdomain', '子域名'), icon: 'fas fa-sitemap' },
  { value: 'username', label: t('dictionary.types.username', '用户名'), icon: 'fas fa-user' },
  { value: 'password', label: t('dictionary.types.password', '密码'), icon: 'fas fa-key' },
  { value: 'path', label: t('dictionary.types.path', '路径'), icon: 'fas fa-folder' },
  { value: 'filename', label: t('dictionary.types.filename', '文件名'), icon: 'fas fa-file' },
  { value: 'extension', label: t('dictionary.types.extension', '扩展名'), icon: 'fas fa-file-code' },
  { value: 'port', label: t('dictionary.types.port', '端口'), icon: 'fas fa-network-wired' },
  { value: 'api_endpoint', label: t('dictionary.types.api_endpoint', 'API端点'), icon: 'fas fa-plug' },
  { value: 'sensitive_file', label: t('dictionary.types.sensitive_file', '敏感文件规则'), icon: 'fas fa-shield-alt' },
  { value: 'service_probe_rule', label: t('dictionary.types.service_probe_rule', '服务识别规则'), icon: 'fas fa-broadcast-tower' },
  { value: 'fingerprint_rule', label: t('dictionary.types.fingerprint_rule', '指纹规则'), icon: 'fas fa-fingerprint' },
  { value: 'poc_rule', label: t('dictionary.types.poc_rule', 'PoC规则'), icon: 'fas fa-bug' },
  { value: 'http_param', label: t('dictionary.types.parameter', 'HTTP参数'), icon: 'fas fa-code' },
  { value: 'xss_payload', label: t('dictionary.types.xss_payload', 'XSS载荷'), icon: 'fas fa-bug' },
  { value: 'sql_injection_payload', label: t('dictionary.types.sql_injection', 'SQL注入'), icon: 'fas fa-database' },
  { value: 'custom', label: t('dictionary.types.custom', '自定义'), icon: 'fas fa-cog' }
]

// 分组定义
interface CategoryGroup {
  key: string
  label: string
  icon: string
  types: string[]  // 该分组包含的 dict_type values
  count?: number
}

const categoryGroupDefs: CategoryGroup[] = [
  {
    key: 'all',
    label: t('dictionary.types.all', '全部'),
    icon: 'fas fa-layer-group',
    types: [],
  },
  {
    key: 'brute',
    label: t('dictionary.groups.brute', '爆破字典'),
    icon: 'fas fa-crosshairs',
    types: ['subdomain', 'username', 'password', 'path', 'filename', 'extension', 'port', 'api_endpoint', 'http_param'],
  },
  {
    key: 'rules',
    label: t('dictionary.groups.rules', '安全规则'),
    icon: 'fas fa-shield-alt',
    types: ['sensitive_file', 'service_probe_rule', 'fingerprint_rule', 'poc_rule'],
  },
  {
    key: 'payloads',
    label: t('dictionary.groups.payloads', '攻击载荷'),
    icon: 'fas fa-bolt',
    types: ['xss_payload', 'sql_injection_payload'],
  },
  {
    key: 'custom',
    label: t('dictionary.types.custom', '自定义'),
    icon: 'fas fa-cog',
    types: ['custom'],
  },
]

const activeGroup = ref('all')

const categoryGroups = computed(() => {
  return categoryGroupDefs.map(g => ({
    ...g,
    count: g.key === 'all' ? undefined : g.types.length,
  }))
})

const activeGroupTypes = computed(() => {
  const group = categoryGroupDefs.find(g => g.key === activeGroup.value)
  if (!group || group.types.length === 0) return []
  return group.types.map(typeValue => {
    const typeDef = dictionaryTypes.find(t => t.value === typeValue)
    return typeDef || { value: typeValue, label: typeValue, icon: 'fas fa-circle' }
  })
})

// 服务类型定义
const serviceTypes = [
  { value: 'web', label: t('dictionary.serviceTypes.web', '网站服务') },
  { value: 'ssh', label: t('dictionary.serviceTypes.ssh', 'SSH服务') },
  { value: 'database', label: t('dictionary.serviceTypes.database', '数据库服务') },
  { value: 'ftp', label: t('dictionary.serviceTypes.ftp', 'FTP服务') },
  { value: 'email', label: t('dictionary.serviceTypes.email', '邮件服务') },
  { value: 'generic', label: t('dictionary.serviceTypes.generic', '通用服务') }
]

// 计算属性
const pageCount = computed(() => Math.max(1, Math.ceil(totalDictionaries.value / pageSize.value)))

// 方法
/** 根据当前 group + type 选择，计算实际要发给后端的 dict_type 参数 */
const effectiveDictType = computed(() => {
  // 如果直接选了某个具体类型
  if (selectedType.value !== 'all') return selectedType.value
  // 如果 group 是 all，不传类型
  const group = categoryGroupDefs.find(g => g.key === activeGroup.value)
  if (!group || group.types.length === 0) return null
  // 如果 group 只有一个类型，直接返回
  if (group.types.length === 1) return group.types[0]
  // 多类型 group 且选择了 "全部"，返回 null 让后端不过滤
  // 但我们需要在前端做 group 范围过滤
  return null
})

/** group 内需要过滤的类型列表 (当 effectiveDictType 为 null 但 group != all 时) */
const groupTypeFilter = computed(() => {
  if (activeGroup.value === 'all' || selectedType.value !== 'all') return null
  const group = categoryGroupDefs.find(g => g.key === activeGroup.value)
  return group && group.types.length > 1 ? group.types : null
})

const loadDictionaries = async () => {
  loadingDictionaries.value = true
  try {
    const result = await invoke<DictionaryPageResponse>('get_dictionaries_paged', {
      dict_type: effectiveDictType.value,
      dict_types: groupTypeFilter.value,
      service_type: null,
      category: null,
      is_builtin: null,
      is_active: null,
      search_term: null,
      subtype: selectedSubtype.value || null,
      limit: pageSize.value,
      offset: (currentPage.value - 1) * pageSize.value,
    })
    dictionaries.value = result?.items || []
    totalDictionaries.value = result?.total || 0

    if (totalDictionaries.value === 0 && currentPage.value !== 1) {
      currentPage.value = 1
      pageInput.value = '1'
      await loadDictionaries()
      return
    }

    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
      pageInput.value = String(currentPage.value)
      await loadDictionaries()
      return
    }

    pageInput.value = String(currentPage.value)
  } catch (error) {
    console.error('Failed to load dictionaries:', error)
  } finally {
    loadingDictionaries.value = false
  }
}

const loadDefaultMap = async () => {
  try {
    const map = await getDefaultMapApi()
    defaultMap.value = map || {}
  } catch (error) {
    console.error('Failed to load default dictionary map:', error)
    defaultMap.value = {}
  }
}

const initializeBuiltinDictionaries = async () => {
  initializing.value = true
  try {
    await invoke('initialize_builtin_dictionaries')
    await loadDictionaries()
  } catch (error) {
    console.error('Failed to initialize builtin dictionaries:', error)
  } finally {
    initializing.value = false
  }
}

const saveDictionary = async (form: DictionaryForm) => {
  saving.value = true
  try {
    dictionaryForm.value = { ...form }
    const metadata = (() => {
      const existing = editingDictionary.value ? parseDictionaryMetadata(editingDictionary.value) : {}
      if (form.subtype?.trim()) {
        return JSON.stringify({
          ...existing,
          subtype: form.subtype.trim(),
        })
      }

      if (existing.subtype) {
        const next = { ...existing }
        delete next.subtype
        return Object.keys(next).length > 0 ? JSON.stringify(next) : null
      }

      return Object.keys(existing).length > 0 ? JSON.stringify(existing) : null
    })()

    if (editingDictionary.value) {
      await invoke('update_dictionary', {
        dictionary: {
          ...editingDictionary.value,
          ...form,
          metadata
        }
      })
    } else {
      await invoke('create_dictionary', {
        name: form.name,
        dict_type: form.dictionary_type,
        service_type: form.service_type || null,
        description: form.description || null,
        category: null,
        tags: null,
        metadata,
      })
    }
    await loadDictionaries()
    closeModal()
  } catch (error) {
    console.error('Failed to save dictionary:', error)
  } finally {
    saving.value = false
  }
}

const editDictionary = (dictionary: Dictionary) => {
  editingDictionary.value = dictionary
  dictionaryForm.value = {
    name: dictionary.name,
    description: dictionary.description || '',
    dictionary_type: dictionary.dict_type,
    service_type: dictionary.service_type || '',
    subtype: typeof parseDictionaryMetadata(dictionary).subtype === 'string'
      ? parseDictionaryMetadata(dictionary).subtype
      : '',
    is_active: dictionary.is_active
  }
}

const deleteDictionary = async (dictionary: Dictionary) => {
    try {
      await invoke('delete_dictionary', { id: dictionary.id })
      // 删除默认字典时，清除该类型默认设置
      if (defaultMap.value[dictionary.dict_type] === dictionary.id) {
        await clearDefaultForType(dictionary.dict_type)
        await loadDefaultMap()
      }
      await loadDictionaries()
    } catch (error) {
      console.error('Failed to delete dictionary:', error)
    }
  
}

const duplicateDictionary = async (dictionary: Dictionary) => {
  try {
    await invoke('create_dictionary', {
      name: `${dictionary.name} ${t('dictionary.copyPrefix', '(副本)')}`,
      dict_type: dictionary.dict_type,
      service_type: dictionary.service_type || null,
      description: dictionary.description || null,
      category: dictionary.category || null,
      tags: dictionary.tags || null,
      metadata: dictionary.metadata || null,
    })
    await loadDictionaries()
  } catch (error) {
    console.error('Failed to duplicate dictionary:', error)
  }
}

const exportDictionary = async (dictionary: Dictionary) => {
  try {
    const result = await invoke('export_dictionary', { dictionary_id: dictionary.id })
    const fileName = `${dictionary.name.replace(/[^a-zA-Z0-9]/g, '_')}.json`
    const filePath = await open({
      defaultPath: fileName,
      filters: [{ name: 'JSON', extensions: ['json'] }]
    })
    
    if (filePath) {
      await writeTextFile(filePath, JSON.stringify(result, null, 2))
    }
  } catch (error) {
    console.error('Failed to export dictionary:', error)
  }
}

// 默认字典：设置/取消（DB 持久化）
const markAsDefault = async (dictionary: Dictionary) => {
  try {
    await setDefaultId(dictionary.dict_type, dictionary.id)
    await loadDefaultMap()
  } catch (error) {
    console.error('Failed to set default dictionary:', error)
  }
}

const clearDefault = async (dictionary: Dictionary) => {
  try {
    await clearDefaultForType(dictionary.dict_type)
    await loadDefaultMap()
  } catch (error) {
    console.error('Failed to clear default dictionary:', error)
  }
}

const {
  LIST_HEIGHT,
  addWord,
  applyBatchRuleEdit,
  clearDictionary,
  closeBatchRuleEditor,
  closeDictionaryWordsModal,
  closeRuleEditor,
  currentRowHeight,
  handleFileSelect,
  handleInfiniteScroll,
  hasMore,
  importMethod,
  importing,
  importText,
  importWords,
  isLoadingMore,
  isStructuredManagingDictionary,
  supportsNmapServiceProbeImport,
  listItems,
  manageDictionaryWords,
  managingDictionary,
  mergeMode,
  newWord,
  openRuleEditor,
  removeSelectedWords,
  removeWord,
  resetAndLoadFirstPage,
  ruleCategoryFilter,
  ruleEnabledFilter,
  ruleProbeNameFilter,
  ruleServiceFilter,
  ruleMatcherFilter,
  ruleSeverityFilter,
  searchQuery,
  selectedFile,
  selectedWords,
  showBatchRuleEditor,
  showImportModal,
  showRuleEditor,
  structuredCategoryOptions,
  structuredProbeNameOptions,
  structuredSeverityOptions,
  structuredServiceOptions,
  toggleSelectAll,
  viewDictionaryWords,
  virtualListRef,
  editingRuleEntry,
  exportSelectedRules,
  saveRuleEntry,
} = useDictionaryWordManager({
  onDictionaryChanged: loadDictionaries,
  confirmClearMessage: t('dictionary.confirmClear', '确定要清空这个字典吗？'),
})

const closeAiGenerateModal = () => {
  if (aiGenerating.value) return
  showAiGenerateModal.value = false
}

const closeManagedDictionaryWordsModal = () => {
  if (aiGenerating.value) return
  showAiGenerateModal.value = false
  showImportModal.value = false
  closeDictionaryWordsModal()
}

const generateDictionaryEntries = async (payload: { prompt: string; count: number }) => {
  if (!managingDictionary.value || !payload.prompt.trim()) return

  aiGenerating.value = true
  try {
    const response = await invoke<GenerateDictionaryEntriesResponse>('generate_dictionary_entries', {
      request: {
        dictionary_name: managingDictionary.value.name,
        dict_type: managingDictionary.value.dict_type,
        service_type: managingDictionary.value.service_type || null,
        subtype: getDictionarySubtypeKey(managingDictionary.value),
        prompt: payload.prompt.trim(),
        count: payload.count,
      },
    })

    const entries = response.entries || []
    if (entries.length === 0) {
      throw new Error('AI 未返回可写入的词条')
    }

    if (isStructuredManagingDictionary.value) {
      await invoke('add_dictionary_entries', {
        dictionary_id: managingDictionary.value.id,
        entries,
      })
    } else {
      await invoke('add_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: entries.map(entry => entry.word).filter(Boolean),
      })
    }

    showAiGenerateModal.value = false
    await resetAndLoadFirstPage()
    await loadDictionaries()
    dialog.toast.success(`AI 已生成 ${entries.length} 条${isStructuredManagingDictionary.value ? '规则' : '词条'}`)
  } catch (error) {
    console.error('Failed to generate dictionary entries with AI:', error)
    await dialog.error(error instanceof Error ? error.message : String(error))
  } finally {
    aiGenerating.value = false
  }
}

const closeModal = () => {
  showCreateModal.value = false
  editingDictionary.value = null
  dictionaryForm.value = {
    name: '',
    description: '',
    dictionary_type: '',
    service_type: '',
    subtype: '',
    is_active: true
  }
}

const getDictionaryTypeLabel = (type: string) => {
  const typeObj = dictionaryTypes.find(t => t.value === type)
  return typeObj ? t(`dictionary.types.${type}`, typeObj.label) : type
}

const getDictionarySubtypeKey = (dictionary: DictionaryView): string | null => {
  return inferDictionarySubtypeKey(dictionary)
}

const getDictionarySubtypeLabel = (dictionary: DictionaryView) => {
  const subtype = getDictionarySubtypeKey(dictionary)
  return subtype ? t(`dictionary.subtypes.${subtype}`, getSubtypeLabel(subtype)) : null
}

const getDictionarySubtypeBadgeClass = (dictionary: DictionaryView) => {
  return getSubtypeBadgeClass(getDictionarySubtypeKey(dictionary))
}

const availableSubtypeOptions = computed(() => {
  const options = selectedType.value === 'all'
    ? getAllSubtypeOptions()
    : getSubtypeOptions(selectedType.value)

  return options.map(option => ({
    value: option.value,
    label: t(`dictionary.subtypes.${option.value}`, option.label),
  }))
})

const subtypeFilterVisible = computed(() => availableSubtypeOptions.value.length > 0)
const managingDictionarySubtypeKey = computed(() =>
  managingDictionary.value ? getDictionarySubtypeKey(managingDictionary.value) : null
)

const onGroupChange = (groupKey: string) => {
  if (activeGroup.value === groupKey) return
  activeGroup.value = groupKey
  const group = categoryGroupDefs.find(g => g.key === groupKey)
  if (!group || group.types.length === 0) {
    // "全部" 分组
    selectedType.value = 'all'
  } else if (group.types.length === 1) {
    // 只有一个类型的分组，直接选中
    selectedType.value = group.types[0]
  } else {
    // 多类型分组，默认显示全部
    selectedType.value = 'all'
  }
  if (!availableSubtypeOptions.value.some(option => option.value === selectedSubtype.value)) {
    selectedSubtype.value = ''
  }
  currentPage.value = 1
  pageInput.value = '1'
  loadDictionaries()
}

const onTypeChange = (value: string) => {
  if (selectedType.value === value) return
  selectedType.value = value
  if (!availableSubtypeOptions.value.some(option => option.value === selectedSubtype.value)) {
    selectedSubtype.value = ''
  }
  currentPage.value = 1
  pageInput.value = '1'
  loadDictionaries()
}

const onSubtypeChange = () => {
  currentPage.value = 1
  pageInput.value = '1'
  loadDictionaries()
}

const onPageSizeChange = () => {
  currentPage.value = 1
  pageInput.value = '1'
  loadDictionaries()
}

const goToPrevPage = () => {
  if (currentPage.value <= 1) return
  currentPage.value -= 1
  pageInput.value = String(currentPage.value)
  loadDictionaries()
}

const goToNextPage = () => {
  if (currentPage.value >= pageCount.value) return
  currentPage.value += 1
  pageInput.value = String(currentPage.value)
  loadDictionaries()
}

const goToFirstPage = () => {
  if (currentPage.value <= 1) return
  currentPage.value = 1
  pageInput.value = '1'
  loadDictionaries()
}

const goToLastPage = () => {
  if (currentPage.value >= pageCount.value) return
  currentPage.value = pageCount.value
  pageInput.value = String(currentPage.value)
  loadDictionaries()
}

const applyPageJump = () => {
  const nextPage = Number.parseInt(pageInput.value, 10)
  if (Number.isNaN(nextPage)) {
    pageInput.value = String(currentPage.value)
    return
  }
  const clamped = Math.min(Math.max(1, nextPage), pageCount.value)
  if (clamped === currentPage.value) {
    pageInput.value = String(currentPage.value)
    return
  }
  currentPage.value = clamped
  pageInput.value = String(currentPage.value)
  loadDictionaries()
}

const getServiceTypeLabel = (type: string) => {
  const serviceObj = serviceTypes.find(s => s.value === type)
  return serviceObj ? t(`dictionary.serviceTypes.${type}`, serviceObj.label) : type
}

const formatDate = (dateString: string) => {
  if (!dateString) return '-'
  return new Date(dateString).toLocaleDateString()
}

// 生命周期
onMounted(async () => {
  await Promise.all([loadDictionaries(), loadDefaultMap()])
})
</script>

<style scoped>
.dictionary-management-page {
  padding: 1rem;
}

.dictionary-modal {
  z-index: 70;
  align-items: flex-start;
  padding: 5rem 1rem 1.5rem;
}

.dictionary-modal-box {
  max-height: calc(100vh - 6.5rem);
  overflow-y: auto;
}

.stats-horizontal .stat {
  padding: 0.5rem;
}

.table-compact th,
.table-compact td {
  padding: 0.5rem;
}

.dict-grid {
  display: grid;
  grid-template-columns: 3rem minmax(0, 1fr) 12rem 5rem;
  column-gap: 1rem;
}

/* ── 分组导航 ── */
.dictionary-group-nav {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.dictionary-group-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding: 0.375rem;
  background: oklch(var(--b2));
  border-radius: 0.75rem;
}

.dict-group-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-radius: 0.5rem;
  border: none;
  background: transparent;
  color: oklch(var(--bc) / 0.65);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.dict-group-btn:hover {
  background: oklch(var(--b3));
  color: oklch(var(--bc));
}

.dict-group-btn.active {
  background: oklch(var(--p));
  color: oklch(var(--pc));
  box-shadow: 0 1px 3px oklch(var(--p) / 0.3);
}

.dict-group-btn i {
  font-size: 0.8rem;
  flex-shrink: 0;
}

.dict-group-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.25rem;
  height: 1.25rem;
  padding: 0 0.35rem;
  border-radius: 999px;
  font-size: 0.7rem;
  font-weight: 600;
  background: oklch(var(--bc) / 0.1);
  color: oklch(var(--bc) / 0.6);
}

.dict-group-btn.active .dict-group-count {
  background: oklch(var(--pc) / 0.2);
  color: oklch(var(--pc));
}

/* ── 二级类型 chips ── */
.dictionary-type-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  padding-left: 0.25rem;
}

.dict-type-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.3rem 0.75rem;
  border-radius: 999px;
  border: 1px solid oklch(var(--bc) / 0.12);
  background: transparent;
  color: oklch(var(--bc) / 0.6);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.dict-type-chip:hover {
  border-color: oklch(var(--p) / 0.4);
  color: oklch(var(--p));
  background: oklch(var(--p) / 0.06);
}

.dict-type-chip.active {
  border-color: oklch(var(--p));
  background: oklch(var(--p) / 0.1);
  color: oklch(var(--p));
  font-weight: 500;
}

.dict-chip-icon {
  font-size: 0.7rem;
}

@media (max-width: 768px) {
  .dict-group-btn {
    padding: 0.4rem 0.75rem;
    font-size: 0.8rem;
    gap: 0.375rem;
  }

  .dict-type-chip {
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
  }
}
</style>
