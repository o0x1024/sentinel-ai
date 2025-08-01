<template>
  <div class="dictionary-management-page">
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
        class="card bg-base-200 shadow-xl hover:shadow-2xl transition-shadow"
      >
        <div class="card-body">
          <div class="flex justify-between items-start mb-2">
            <h3 class="card-title text-lg">{{ dictionary.name }}</h3>
            <div class="dropdown dropdown-end">
              <label tabindex="0" class="btn btn-ghost btn-sm">
                <i class="fas fa-ellipsis-v"></i>
              </label>
              <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52">
                <li><a @click="editDictionary(dictionary)"><i class="fas fa-edit mr-2"></i>{{ t('common.edit', '编辑') }}</a></li>
                <li><a @click="exportDictionary(dictionary)"><i class="fas fa-download mr-2"></i>{{ t('common.export', '导出') }}</a></li>
                <li><a @click="duplicateDictionary(dictionary)"><i class="fas fa-copy mr-2"></i>{{ t('common.duplicate', '复制') }}</a></li>
                <li v-if="!dictionary.is_builtin"><a @click="deleteDictionary(dictionary)" class="text-error"><i class="fas fa-trash mr-2"></i>{{ t('common.delete', '删除') }}</a></li>
              </ul>
            </div>
          </div>
          
          <p class="text-sm opacity-70 mb-3">{{ dictionary.description }}</p>
          
          <div class="flex flex-wrap gap-2 mb-3">
            <div class="badge badge-primary">{{ getDictionaryTypeLabel(dictionary.dictionary_type) }}</div>
            <div v-if="dictionary.service_type" class="badge badge-secondary">{{ getServiceTypeLabel(dictionary.service_type) }}</div>
            <div v-if="dictionary.is_builtin" class="badge badge-accent">{{ t('dictionary.builtin', '内置') }}</div>
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
        <h3 class="font-bold text-lg mb-4">
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
        
        <div class="overflow-y-auto max-h-96 border rounded-lg">
          <table class="table table-compact w-full">
            <thead class="sticky top-0 bg-base-200">
              <tr>
                <th class="w-12">
                  <input 
                    type="checkbox" 
                    class="checkbox" 
                    :checked="selectedWords.length === filteredWords.length && filteredWords.length > 0"
                    @change="toggleSelectAll"
                  >
                </th>
                <th>{{ t('dictionary.word', '词条') }}</th>
                <th>{{ t('dictionary.addedAt', '添加时间') }}</th>
                <th class="w-20">{{ t('common.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="word in filteredWords" :key="word.id">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox" 
                    :value="word.id"
                    v-model="selectedWords"
                  >
                </td>
                <td>{{ word.word }}</td>
                <td>{{ formatDate(word.created_at) }}</td>
                <td>
                  <button 
                    class="btn btn-ghost btn-xs text-error" 
                    @click="removeWord(word.id)"
                  >
                    <i class="fas fa-trash"></i>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
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
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { writeTextFile }  from '@tauri-apps/plugin-fs';

const { t } = useI18n()

// 响应式数据
const dictionaries = ref([])
const selectedType = ref('all')
const showCreateModal = ref(false)
const editingDictionary = ref(null)
const managingDictionary = ref(null)
const showImportModal = ref(false)
const saving = ref(false)
const initializing = ref(false)
const importing = ref(false)
const newWord = ref('')
const searchQuery = ref('')
const selectedWords = ref([])
const importText = ref('')
const importMethod = ref('text')
const selectedFile = ref(null)
const mergeMode = ref('append')
const dictionaryWords = ref([])

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
  { value: 'parameter', label: t('dictionary.types.parameter', 'HTTP参数'), icon: 'fas fa-code' },
  { value: 'xss_payload', label: t('dictionary.types.xss_payload', 'XSS载荷'), icon: 'fas fa-bug' },
  { value: 'sql_injection', label: t('dictionary.types.sql_injection', 'SQL注入'), icon: 'fas fa-database' },
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
  return dictionaries.value.filter(dict => dict.dictionary_type === selectedType.value)
})

const filteredWords = computed(() => {
  if (!searchQuery.value.trim()) {
    return dictionaryWords.value
  }
  return dictionaryWords.value.filter(word => 
    word.word.toLowerCase().includes(searchQuery.value.toLowerCase())
  )
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
    })
    dictionaries.value = result || []
  } catch (error) {
    console.error('Failed to load dictionaries:', error)
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

const editDictionary = (dictionary) => {
  editingDictionary.value = dictionary
  dictionaryForm.value = { ...dictionary }
}

const deleteDictionary = async (dictionary) => {
  if (confirm(t('dictionary.confirmDelete', '确定要删除这个字典吗？'))) {
    try {
      await invoke('delete_dictionary', { id: dictionary.id })
      await loadDictionaries()
    } catch (error) {
      console.error('Failed to delete dictionary:', error)
    }
  }
}

const duplicateDictionary = async (dictionary) => {
  try {
    await invoke('create_dictionary', {
      name: `${dictionary.name} ${t('dictionary.copyPrefix', '(副本)')}`,
      dict_type: dictionary.dictionary_type,
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

const exportDictionary = async (dictionary) => {
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

const manageDictionaryWords = async (dictionary) => {
  managingDictionary.value = dictionary
  await loadDictionaryWords(dictionary.id)
}

const viewDictionaryWords = async (dictionary) => {
  managingDictionary.value = dictionary
  await loadDictionaryWords(dictionary.id)
}

const loadDictionaryWords = async (dictionaryId) => {
  try {
    const result = await invoke('get_dictionary_words', {
      dictionary_id: dictionaryId
    })
    dictionaryWords.value = result || []
  } catch (error) {
    console.error('Failed to load dictionary words:', error)
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
    await loadDictionaryWords(managingDictionary.value.id)
    await loadDictionaries()
  } catch (error) {
    console.error('Failed to add word:', error)
  }
}

const removeWord = async (wordId) => {
  try {
    // 找到要删除的词条
    const wordToRemove = dictionaryWords.value.find(w => w.id === wordId)
    if (wordToRemove) {
      await invoke('remove_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: [wordToRemove.word]
      })
      await loadDictionaryWords(managingDictionary.value.id)
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
    
    if (wordsToRemove.length > 0) {
      await invoke('remove_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words: wordsToRemove
      })
      selectedWords.value = []
      await loadDictionaryWords(managingDictionary.value.id)
      await loadDictionaries()
    }
  } catch (error) {
    console.error('Failed to remove selected words:', error)
  }
}

const clearDictionary = async () => {
  if (!confirm(t('dictionary.confirmClear', '确定要清空这个字典吗？'))) return
  
  try {
    await invoke('clear_dictionary', { dictionary_id: managingDictionary.value.id })
    await loadDictionaryWords(managingDictionary.value.id)
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
    
    if (words.length > 0) {
      // 使用add_dictionary_words来导入词条
      await invoke('add_dictionary_words', {
        dictionary_id: managingDictionary.value.id,
        words
      })
      
      importText.value = ''
      selectedFile.value = null
      showImportModal.value = false
      await loadDictionaryWords(managingDictionary.value.id)
      await loadDictionaries()
    }
  } catch (error) {
    console.error('Failed to import words:', error)
  } finally {
    importing.value = false
  }
}

const handleFileSelect = (event) => {
  selectedFile.value = event.target.files[0]
}

const toggleSelectAll = () => {
  if (selectedWords.value.length === filteredWords.value.length) {
    selectedWords.value = []
  } else {
    selectedWords.value = filteredWords.value.map(word => word.id)
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
}

const getDictionaryTypeLabel = (type) => {
  const typeObj = dictionaryTypes.find(t => t.value === type)
  return typeObj ? t(`dictionary.types.${type}`, typeObj.label) : type
}

const getServiceTypeLabel = (type) => {
  const serviceObj = serviceTypes.find(s => s.value === type)
  return serviceObj ? t(`dictionary.serviceTypes.${type}`, serviceObj.label) : type
}

const formatDate = (dateString) => {
  if (!dateString) return '-'
  return new Date(dateString).toLocaleDateString()
}

// 生命周期
onMounted(() => {
  loadDictionaries()
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
</style>