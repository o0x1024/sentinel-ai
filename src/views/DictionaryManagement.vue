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
    <div class="tabs tabs-boxed mb-6">
      <a 
        v-for="type in dictionaryTypes" 
        :key="type.value"
        class="tab"
        :class="{ 'tab-active': selectedType === type.value }"
        @click="selectedType = type.value"
      >
        <i :class="type.icon + ' mr-2'"></i>
        {{ t(`dictionary.types.${type.value}`, type.label) }}
      </a>
    </div>

    <!-- 字典列表 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
      <div 
        v-for="dictionary in filteredDictionaries" 
        :key="dictionary.id"
        class="card bg-base-200 shadow-xl hover:shadow-2xl transition-shadow overflow-visible relative"
      >
        <div class="card-body">
          <div class="flex justify-between items-start mb-2">
            <h3 class="card-title text-lg">{{ dictionary.name }}</h3>
            <div class="dropdown dropdown-end z-50 relative">
              <label tabindex="0" class="btn btn-ghost btn-sm">
                <i class="fas fa-ellipsis-v"></i>
              </label>
              <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52 z-50">
                <li><a @click="editDictionary(dictionary)"><i class="fas fa-edit mr-2"></i>{{ t('common.edit', '编辑') }}</a></li>
                <li><a @click="exportDictionary(dictionary)"><i class="fas fa-download mr-2"></i>{{ t('common.export', '导出') }}</a></li>
                <li><a @click="duplicateDictionary(dictionary)"><i class="fas fa-copy mr-2"></i>{{ t('common.duplicate', '复制') }}</a></li>
                <li><a @click="markAsDefault(dictionary)"><i class="fas fa-star mr-2"></i>{{ t('dictionary.setDefault', '设为默认') }}</a></li>
                <li v-if="defaultMap[dictionary.dict_type] === dictionary.id"><a @click="clearDefault(dictionary)"><i class="fas fa-ban mr-2"></i>{{ t('dictionary.clearDefault', '取消默认') }}</a></li>
                <li v-if="!dictionary.is_builtin"><a @click="deleteDictionary(dictionary)" class="text-error"><i class="fas fa-trash mr-2"></i>{{ t('common.delete', '删除') }}</a></li>
              </ul>
            </div>
          </div>
          
          <p class="text-sm opacity-70 mb-3">{{ dictionary.description }}</p>
          
          <div class="flex flex-wrap gap-2 mb-3">
            <div class="badge badge-primary">{{ getDictionaryTypeLabel(dictionary.dict_type) }}</div>
            <div v-if="dictionary.service_type" class="badge badge-secondary">{{ getServiceTypeLabel(dictionary.service_type) }}</div>
            <div v-if="dictionary.is_builtin" class="badge badge-accent">{{ t('dictionary.builtin', '内置') }}</div>
            <div v-if="defaultMap[dictionary.dict_type] === dictionary.id" class="badge badge-success">{{ t('dictionary.default', '默认') }}</div>
          </div>
          
          <div class="stats stats-horizontal bg-base-100 rounded-lg">
            <div class="stat py-2">
              <div class="stat-title text-xs">{{ t('dictionary.wordCount', '词条数') }}</div>
              <div class="stat-value text-lg">{{ dictionary.word_count || 0 }}</div>
            </div>
            <div class="stat py-2">
              <div class="stat-title text-xs">{{ t('dictionary.lastUpdated', '最后更新') }}</div>
              <div class="stat-value text-xs">{{ formatDate(dictionary.updated_at) }}</div>
            </div>
          </div>
          
          <div class="card-actions justify-end mt-4">
            <button 
              class="btn btn-sm btn-outline" 
              @click="viewDictionaryWords(dictionary)"
            >
              <i class="fas fa-eye mr-1"></i>
              {{ t('dictionary.viewWords', '查看词条') }}
            </button>
            <button 
              class="btn btn-sm btn-primary" 
              @click="manageDictionaryWords(dictionary)"
            >
              <i class="fas fa-edit mr-1"></i>
              {{ t('dictionary.manage', '管理') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 创建/编辑字典模态框 -->
    <div v-if="showCreateModal || editingDictionary" class="modal modal-open">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4" >
          {{ editingDictionary ? t('dictionary.editDictionary', '编辑字典') : t('dictionary.createDictionary', '创建字典') }}
        </h3>
        
        <form @submit.prevent="saveDictionary" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('dictionary.name', '字典名称') }}</span>
            </label>
            <input 
              type="text" 
              v-model="dictionaryForm.name" 
              class="input input-bordered" 
              :placeholder="t('dictionary.namePlaceholder', '请输入字典名称')"
              required
            >
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('dictionary.description', '描述') }}</span>
            </label>
            <textarea 
              v-model="dictionaryForm.description" 
              class="textarea textarea-bordered" 
              :placeholder="t('dictionary.descriptionPlaceholder', '请输入字典描述')"
            ></textarea>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('dictionary.type', '字典类型') }}</span>
              </label>
              <select v-model="dictionaryForm.dictionary_type" class="select select-bordered" required>
                <option value="">{{ t('dictionary.selectType', '选择类型') }}</option>
                <option v-for="type in dictionaryTypes.slice(1)" :key="type.value" :value="type.value">
                  {{ t(`dictionary.types.${type.value}`, type.label) }}
                </option>
              </select>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('dictionary.serviceType', '服务类型') }}</span>
              </label>
              <select v-model="dictionaryForm.service_type" class="select select-bordered">
                <option value="">{{ t('dictionary.selectServiceType', '选择服务类型（可选）') }}</option>
                <option v-for="service in serviceTypes" :key="service.value" :value="service.value">
                  {{ t(`dictionary.serviceTypes.${service.value}`, service.label) }}
                </option>
              </select>
            </div>
          </div>
          
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('dictionary.isActive', '启用字典') }}</span>
              <input type="checkbox" v-model="dictionaryForm.is_active" class="toggle toggle-primary">
            </label>
          </div>
          
          <div class="modal-action">
            <button type="button" class="btn" @click="closeModal">{{ t('common.cancel', '取消') }}</button>
            <button type="submit" class="btn btn-primary" :disabled="saving">
              {{ saving ? t('common.saving', '保存中...') : t('common.save', '保存') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 词条管理模态框 -->
    <div v-if="managingDictionary" class="modal modal-open">
      <div class="modal-box max-w-4xl max-h-[80vh]">
        <h3 class="font-bold text-lg mb-4">
          {{ t('dictionary.manageWords', '管理词条') }} - {{ managingDictionary.name }}
        </h3>
        
        <div class="flex gap-4 mb-4">
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

        <!-- 虚拟滚动词条列表（提升大数据量下的渲染性能） -->
        <div class="border rounded-lg">
          <!-- 表头 -->
          <div class="sticky top-0 bg-base-200 px-4 py-2 dict-grid items-center text-sm font-medium border-b">
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

          <!-- 列表主体：虚拟列表 -->
          <VirtualList
            ref="virtualListRef"
            :items="listItems"
            :itemHeight="ROW_HEIGHT"
            :height="LIST_HEIGHT"
            class="virtual-list-host"
            keyField="id"
            @scroll="handleInfiniteScroll"
          >
            <template #default="{ item }">
              <div class="px-4 dict-grid items-center text-sm h-full w-full">
                <div class="py-2">
                  <input 
                    type="checkbox" 
                    class="checkbox" 
                    :value="item.id"
                    v-model="selectedWords"
                  >
                </div>
                <div class="truncate whitespace-nowrap min-w-0 py-2" :title="item.word">{{ item.word }}</div>
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
          <!-- 底部加载状态 -->
          <div class="px-4 py-2 text-center text-sm opacity-70">
            <span v-if="isLoadingMore">{{ t('common.loading', '加载中...') }}</span>
            <span v-else-if="!hasMore">{{ t('common.noMore', '没有更多了') }}</span>
          </div>
        </div>
        
        <div class="flex justify-between items-center mt-4">
          <div class="flex gap-2">
            <button 
              v-if="selectedWords.length > 0" 
              class="btn btn-error btn-sm" 
              @click="removeSelectedWords"
            >
              <i class="fas fa-trash mr-2"></i>
              {{ t('dictionary.removeSelected', '删除选中') }} ({{ selectedWords.length }})
            </button>
            <button 
              class="btn btn-warning btn-sm" 
              @click="clearDictionary"
            >
              <i class="fas fa-broom mr-2"></i>
              {{ t('dictionary.clearAll', '清空字典') }}
            </button>
          </div>
          
          <div class="modal-action">
            <button class="btn" @click="closeDictionaryWordsModal">{{ t('common.close', '关闭') }}</button>
          </div>
        </div>
        
      </div>
    </div>

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
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { writeTextFile }  from '@tauri-apps/plugin-fs';
import { dialog } from '@/composables/useDialog'
import VirtualList from '@/components/VirtualList.vue'
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

interface DictionaryWord {
  id: string;
  word: string;
  created_at: string;
}

interface DictionaryForm {
  name: string;
  description?: string;
  dictionary_type: string;
  service_type?: string;
  is_active: boolean;
}

// 响应式数据
const dictionaries = ref<Dictionary[]>([])
const selectedType = ref('all')
const showCreateModal = ref(false)
const editingDictionary = ref<Dictionary | null>(null)
const managingDictionary = ref<Dictionary | null>(null)
const showImportModal = ref(false)
const saving = ref(false)
const initializing = ref(false)
const importing = ref(false)
const newWord = ref('')
const searchQuery = ref('')
const debouncedSearch = ref('')
const virtualListRef = ref<InstanceType<typeof VirtualList> | null>(null)
const selectedWords = ref<string[]>([])
const importText = ref('')
const importMethod = ref('text')
const selectedFile = ref<File | null>(null)
const mergeMode = ref('append')
const dictionaryWords = ref<DictionaryWord[]>([])
// 无限滚动状态
const batchSize = ref(500) // 每次追加加载数量
const isLoadingMore = ref(false)
const hasMore = ref(true)
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

// 列表展示数据（服务端已过滤，这里不再二次过滤，避免大数据下的前端压力）
const listItems = computed(() => dictionaryWords.value)

// 常量：虚拟列表行高与容器高度（和样式保持一致）
const ROW_HEIGHT = 40
const LIST_HEIGHT = 384 // 与 max-h-96 相当（24rem）

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

const saveDictionary = async () => {
  saving.value = true
  try {
    if (editingDictionary.value) {
      await invoke('update_dictionary', {
        dictionary: {
          ...editingDictionary.value,
          ...dictionaryForm.value
        }
      })
    } else {
      await invoke('create_dictionary', {
        name: dictionaryForm.value.name,
        dict_type: dictionaryForm.value.dictionary_type,
        service_type: dictionaryForm.value.service_type || null,
        description: dictionaryForm.value.description || null,
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

const manageDictionaryWords = async (dictionary: Dictionary) => {
  managingDictionary.value = dictionary
  await resetAndLoadFirstPage()
}

const viewDictionaryWords = async (dictionary: Dictionary) => {
  managingDictionary.value = dictionary
  await resetAndLoadFirstPage()
}

const resetAndLoadFirstPage = async () => {
  if (!managingDictionary.value) return
  dictionaryWords.value = []
  selectedWords.value = []
  hasMore.value = true
  await loadMore()
}

const loadMore = async () => {
  if (!managingDictionary.value || isLoadingMore.value || !hasMore.value) return
  try {
    isLoadingMore.value = true
    const offset = dictionaryWords.value.length
    const limit = batchSize.value
    const pattern = debouncedSearch.value.trim() || null
    const result = await invoke('get_dictionary_words_paged', {
      dictionary_id: managingDictionary.value.id,
      offset,
      limit,
      pattern
    }) as DictionaryWord[]

    const chunk = result || []
    if (chunk.length > 0) {
      dictionaryWords.value = dictionaryWords.value.concat(chunk)
    }
    // 是否还有更多
    hasMore.value = chunk.length === limit
  } catch (error) {
    console.error('Failed to load more dictionary words:', error)
  } finally {
    isLoadingMore.value = false
  }
}

const handleInfiniteScroll = ({ scrollTop, clientHeight, scrollHeight }: { scrollTop: number; clientHeight: number; scrollHeight: number }) => {
  const threshold = 2 * ROW_HEIGHT // 距底部一个小阈值触发
  if (scrollTop + clientHeight >= scrollHeight - threshold) {
    loadMore()
  }
}

const addWord = async () => {
  if (!newWord.value.trim() || !managingDictionary.value) return
  
  try {
    await invoke('add_dictionary_words', {
      dictionary_id: managingDictionary.value.id,
      words: [newWord.value.trim()]
    })
    newWord.value = ''
  await resetAndLoadFirstPage()
    await loadDictionaries()
  } catch (error) {
    console.error('Failed to add word:', error)
  }
}

const removeWord = async (wordId: string) => {
  try {
    // 找到要删除的词条
    const wordToRemove = dictionaryWords.value.find(w => w.id === wordId)
    if (wordToRemove && managingDictionary.value) {
      await invoke('remove_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: [wordToRemove.word]
      })
  await resetAndLoadFirstPage()
      await loadDictionaries()
    }
  } catch (error) {
    console.error('Failed to remove word:', error)
  }
}

const removeSelectedWords = async () => {
  if (selectedWords.value.length === 0) return
  
  try {
    // 找到要删除的词条
    const wordsToRemove = dictionaryWords.value
      .filter(w => selectedWords.value.includes(w.id))
      .map(w => w.word)
    
    if (wordsToRemove.length > 0 && managingDictionary.value) {
      await invoke('remove_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: wordsToRemove
      })
      selectedWords.value = []
  await resetAndLoadFirstPage()
      await loadDictionaries()
    }
  } catch (error) {
    console.error('Failed to remove selected words:', error)
  }
}

const clearDictionary = async () => {
  const confirmed = await dialog.confirm(t('dictionary.confirmClear', '确定要清空这个字典吗？'));
  if (!confirmed || !managingDictionary.value) return
  
  try {
    await invoke('clear_dictionary', { dictionary_id: managingDictionary.value.id })
  await resetAndLoadFirstPage()
    await loadDictionaries()
  } catch (error) {
    console.error('Failed to clear dictionary:', error)
  }
}

const importWords = async () => {
  importing.value = true
  try {
    let words = []
    
    if (importMethod.value === 'text' && importText.value.trim()) {
      words = importText.value.split('\n')
        .map(word => word.trim())
        .filter(word => word.length > 0)
    } else if (importMethod.value === 'file' && selectedFile.value) {
      // 处理文件导入逻辑
      const content = await selectedFile.value.text()
      if (selectedFile.value.name.endsWith('.json')) {
        const data = JSON.parse(content)
        words = Array.isArray(data) ? data : data.words || []
      } else {
        words = content.split('\n')
          .map(word => word.trim())
          .filter(word => word.length > 0)
      }
    }
    
    if (words.length > 0 && managingDictionary.value) {
      // 使用add_dictionary_words来导入词条
      await invoke('add_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words
      })
      
      importText.value = ''
      selectedFile.value = null
      showImportModal.value = false
  await resetAndLoadFirstPage()
      await loadDictionaries()
    }
  } catch (error) {
    console.error('Failed to import words:', error)
  } finally {
    importing.value = false
  }
}

const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  selectedFile.value = target.files?.[0] || null
}

const toggleSelectAll = () => {
  if (selectedWords.value.length === listItems.value.length) {
    selectedWords.value = []
  } else {
    selectedWords.value = listItems.value.map(word => word.id)
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
    is_active: true
  }
}

const closeDictionaryWordsModal = () => {
  managingDictionary.value = null
  dictionaryWords.value = []
  selectedWords.value = []
  searchQuery.value = ''
  debouncedSearch.value = ''
  hasMore.value = true
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

// 搜索防抖：减少大数据量下频繁过滤导致的卡顿
let searchDebounceTimer: number | null = null
watch(searchQuery, (val) => {
  if (searchDebounceTimer) {
    clearTimeout(searchDebounceTimer)
  }
  searchDebounceTimer = window.setTimeout(() => {
    debouncedSearch.value = val
    // 搜索变化后重置滚动到顶部
    virtualListRef.value?.scrollToTop?.()
    // 搜索触发服务端重新加载第一页
    if (managingDictionary.value) {
      hasMore.value = true
      resetAndLoadFirstPage()
    }
  }, 200)
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

.sticky {
  position: sticky;
  z-index: 10;
}

/* 虚拟列表宿主：对齐样式 */
.virtual-list-host :deep(.virtual-list-item) {
  /* 让每行看起来像表格的分隔线 */
  border-bottom: 1px solid var(--fallback-b3, oklch(var(--b3)));
}

/* 统一表头与行的列布局，避免重复写法导致错位 */
.dict-grid {
  display: grid;
  grid-template-columns: 3rem minmax(0, 1fr) 12rem 5rem;
  column-gap: 1rem; /* 等价于 gap-x-4 */
}

/* 始终为滚动条预留间隙，防止出现/消失导致表头与内容错位（macOS overlay 亦保持稳定） */
.virtual-list-host :deep(.virtual-list-container) {
  scrollbar-gutter: stable both-edges;
}

/* 确保虚拟行占满可用宽度，避免内容宽度小于表头 */
.virtual-list-host :deep(.virtual-list-item) {
  width: 100%;
}
</style>