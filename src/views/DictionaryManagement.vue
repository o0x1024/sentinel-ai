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

    <!-- 字典类型过滤器 -->
    <div class="dictionary-type-tabs mb-6">
      <div class="tabs tabs-boxed dictionary-tabs-list">
      <a 
        v-for="type in dictionaryTypes" 
        :key="type.value"
        class="tab dictionary-tab"
        :class="{ 'tab-active': selectedType === type.value }"
        @click="selectedType = type.value"
      >
        <i :class="type.icon"></i>
        <span class="dictionary-tab-label">{{ t(`dictionary.types.${type.value}`, type.label) }}</span>
      </a>
      </div>
    </div>

    <DictionaryCardGrid
      :dictionaries="filteredDictionaries"
      :default-map="defaultMap"
      :get-dictionary-type-label="getDictionaryTypeLabel"
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
    <div v-if="managingDictionary" class="modal modal-open">
      <div class="modal-box max-w-4xl max-h-[80vh]">
        <h3 class="font-bold text-lg mb-4">
          {{ t('dictionary.manageWords', '管理词条') }} - {{ managingDictionary.name }}
        </h3>
        
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
          v-model:category="ruleCategoryFilter"
          v-model:severity="ruleSeverityFilter"
          v-model:matcher="ruleMatcherFilter"
          v-model:enabled="ruleEnabledFilter"
          :category-options="structuredCategoryOptions"
          :severity-options="structuredSeverityOptions"
        />

        <StructuredRuleWordList
          v-if="isStructuredManagingDictionary"
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
          @close="closeDictionaryWordsModal"
        />
        
      </div>
    </div>

    <RuleEntryEditorModal
      :open="showRuleEditor"
      :dictionary-type="managingDictionary?.dict_type || ''"
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

    <!-- 导入模态框 -->
    <div v-if="showImportModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('dictionary.importWords', '导入词条') }}</h3>
        
        <div class="tabs tabs-boxed mb-4">
          <a class="tab" :class="{ 'tab-active': importMethod === 'text' }" @click="importMethod = 'text'">
            {{ t('dictionary.importFromText', '文本导入') }}
          </a>
          <a class="tab" :class="{ 'tab-active': importMethod === 'file' }" @click="importMethod = 'file'">
            {{ t('dictionary.importFromFile', '文件导入') }}
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { writeTextFile }  from '@tauri-apps/plugin-fs';
import DictionaryCardGrid from '@/components/Dictionary/DictionaryCardGrid.vue'
import DictionaryFormModal from '@/components/Dictionary/DictionaryFormModal.vue'
import VirtualList from '@/components/VirtualList.vue'
import DictionaryWordActionBar from '@/components/Dictionary/DictionaryWordActionBar.vue'
import RuleEntryEditorModal from '@/components/Dictionary/RuleEntryEditorModal.vue'
import RuleBatchEditModal from '@/components/Dictionary/RuleBatchEditModal.vue'
import StructuredRuleFilters from '@/components/Dictionary/StructuredRuleFilters.vue'
import StructuredRuleWordList from '@/components/Dictionary/StructuredRuleWordList.vue'
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
  tags?: string[];
}

interface DictionaryForm {
  name: string;
  description: string;
  dictionary_type: string;
  service_type: string;
  is_active: boolean;
}

// 响应式数据
const dictionaries = ref<Dictionary[]>([])
const selectedType = ref('all')
const showCreateModal = ref(false)
const editingDictionary = ref<Dictionary | null>(null)
const saving = ref(false)
const initializing = ref(false)
// 默认字典映射：{ [dict_type]: dictionary_id }
const defaultMap = ref<Record<string, string>>({})

// 表单数据
const dictionaryForm = ref({
  name: '',
  description: '',
  dictionary_type: '',
  service_type: '',
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
  { value: 'fingerprint_rule', label: t('dictionary.types.fingerprint_rule', '指纹规则'), icon: 'fas fa-fingerprint' },
  { value: 'poc_rule', label: t('dictionary.types.poc_rule', 'PoC规则'), icon: 'fas fa-bug' },
  { value: 'http_param', label: t('dictionary.types.parameter', 'HTTP参数'), icon: 'fas fa-code' },
  { value: 'xss_payload', label: t('dictionary.types.xss_payload', 'XSS载荷'), icon: 'fas fa-bug' },
  { value: 'sql_injection_payload', label: t('dictionary.types.sql_injection', 'SQL注入'), icon: 'fas fa-database' },
  { value: 'custom', label: t('dictionary.types.custom', '自定义'), icon: 'fas fa-cog' }
]

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
const filteredDictionaries = computed(() => {
  if (selectedType.value === 'all') {
    return dictionaries.value
  }
  return dictionaries.value.filter(dict => dict.dict_type === selectedType.value)
})

// 方法
const loadDictionaries = async () => {
  try {
    const result = await invoke('get_dictionaries', {
      dict_type: selectedType.value === 'all' ? null : selectedType.value,
      service_type: null,
      category: null,
      is_builtin: null,
      is_active: null,
      search_term: null
    }) as Dictionary[]
    dictionaries.value = result || []
  } catch (error) {
    console.error('Failed to load dictionaries:', error)
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
    if (editingDictionary.value) {
      await invoke('update_dictionary', {
        dictionary: {
          ...editingDictionary.value,
          ...form
        }
      })
    } else {
      await invoke('create_dictionary', {
        name: form.name,
        dict_type: form.dictionary_type,
        service_type: form.service_type || null,
        description: form.description || null,
        category: null,
        tags: null
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
      tags: dictionary.tags || null
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
  listItems,
  manageDictionaryWords,
  managingDictionary,
  mergeMode,
  newWord,
  openRuleEditor,
  removeSelectedWords,
  removeWord,
  ruleCategoryFilter,
  ruleEnabledFilter,
  ruleMatcherFilter,
  ruleSeverityFilter,
  searchQuery,
  selectedFile,
  selectedWords,
  showBatchRuleEditor,
  showImportModal,
  showRuleEditor,
  structuredCategoryOptions,
  structuredSeverityOptions,
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

const closeModal = () => {
  showCreateModal.value = false
  editingDictionary.value = null
  dictionaryForm.value = {
    name: '',
    description: '',
    dictionary_type: '',
    service_type: '',
    is_active: true
  }
}

const getDictionaryTypeLabel = (type: string) => {
  const typeObj = dictionaryTypes.find(t => t.value === type)
  return typeObj ? t(`dictionary.types.${type}`, typeObj.label) : type
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

.modal-box {
  max-height: 90vh;
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
  column-gap: 1rem; /* 等价于 gap-x-4 */
}

.dictionary-type-tabs {
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 0.25rem;
  scrollbar-width: thin;
}

.dictionary-tabs-list {
  display: inline-flex;
  flex-wrap: nowrap;
  min-width: max-content;
}

.dictionary-tab {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  white-space: nowrap;
}

.dictionary-tab i {
  flex-shrink: 0;
}

.dictionary-tab-label {
  white-space: nowrap;
}

@media (max-width: 768px) {
  .dictionary-tab {
    min-height: 2.5rem;
    padding-inline: 0.75rem;
    font-size: 0.875rem;
    gap: 0.375rem;
  }
}
</style>
