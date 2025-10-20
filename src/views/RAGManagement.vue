<template>
  <div class="container mx-auto p-6">
    <!-- 页面标题 -->
    <div class="flex justify-between items-center mb-6">
      <h1 class="text-3xl font-bold">RAG 知识库管理</h1>
      <div class="flex gap-2">
        <button 
          @click="showCreateCollectionModal = true"
          class="btn btn-primary"
        >
          <i class="fas fa-plus mr-2"></i>
          创建集合
        </button>
        <button 
          @click="refreshCollections"
          class="btn btn-outline"
        >
          <i class="fas fa-refresh mr-2"></i>
          刷新
        </button>
      </div>
    </div>

    <!-- 统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-primary">
          <i class="fas fa-database text-2xl"></i>
        </div>
        <div class="stat-title">集合总数</div>
        <div class="stat-value text-primary">{{ collections.length }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-secondary">
          <i class="fas fa-file-alt text-2xl"></i>
        </div>
        <div class="stat-title">文档总数</div>
        <div class="stat-value text-secondary">{{ totalDocuments }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-accent">
          <i class="fas fa-puzzle-piece text-2xl"></i>
        </div>
        <div class="stat-title">块总数</div>
        <div class="stat-value text-accent">{{ totalChunks }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-info">
          <i class="fas fa-search text-2xl"></i>
        </div>
        <div class="stat-title">查询总数</div>
        <div class="stat-value text-info">{{ totalQueries }}</div>
      </div>
    </div>

    <!-- 集合列表 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title mb-2">知识库集合</h2>
        <p class="text-sm text-base-content/70 mb-4">
          已激活的集合会在 AI 助手的 RAG 模式下被联合检索。
        </p>
        
        <!-- 搜索和过滤 -->
        <div class="flex gap-4 mb-4">
          <div class="form-control flex-1">
            <input 
              v-model="searchQuery"
              type="text" 
              placeholder="搜索集合..." 
              class="input input-bordered"
            >
          </div>
          <select v-model="statusFilter" class="select select-bordered">
            <option value="">全部状态</option>
            <option value="active">已激活</option>
            <option value="inactive">未激活</option>
          </select>
        </div>

        <!-- 集合表格 -->
        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>名称</th>
                <th>描述</th>
                <th>嵌入模型</th>
                <th>文档数</th>
                <th>创建时间</th>
                <th class="text-center">激活</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="collection in filteredCollections" :key="collection.id">
                <td>
                  <div class="flex items-center gap-3">
                    <div class="avatar">
                      <div class="mask mask-squircle w-10 h-10 bg-primary/10 flex items-center justify-center">
                        <i class="fas fa-database text-primary"></i>
                      </div>
                    </div>
                    <div>
                      <div class="font-bold">{{ collection.name }}</div>
                      <div class="text-sm opacity-50">{{ collection.id }}</div>
                    </div>
                  </div>
                </td>
                <td>{{ collection.description || '无描述' }}</td>
                <td>
                  <div class="badge badge-outline">{{ collection.embedding_model }}</div>
                </td>
                <td>{{ collection.document_count }}</td>
                <td>{{ formatDate(collection.created_at) }}</td>
                <td class="text-center">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary"
                    :checked="!!collection.is_active"
                    @change="onActiveToggle(collection, $event)"
                    :aria-label="collection.is_active ? '已激活' : '未激活'"
                  />
                </td>
                <td>
                  <div class="flex gap-2">
                    <button 
                      @click="viewCollection(collection)"
                      class="btn btn-ghost btn-xs"
                      title="查看详情"
                    >
                      <i class="fas fa-eye"></i>
                    </button>
                    <button 
                      @click="showIngestModal(collection)"
                      class="btn btn-ghost btn-xs"
                      title="添加文档"
                    >
                      <i class="fas fa-plus"></i>
                    </button>
                    <button 
                      @click="queryCollection(collection)"
                      class="btn btn-ghost btn-xs"
                      title="查询"
                    >
                      <i class="fas fa-search"></i>
                    </button>
                    <button 
                      @click="deleteCollection(collection)"
                      class="btn btn-ghost btn-xs text-error"
                      title="删除"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 创建集合模态框 -->
    <div v-if="showCreateCollectionModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">创建新集合</h3>
        
        <form @submit.prevent="createCollection">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">集合名称</span>
            </label>
            <input 
              v-model="newCollection.name"
              type="text" 
              class="input input-bordered" 
              placeholder="输入集合名称"
              required
            >
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">描述</span>
            </label>
            <textarea 
              v-model="newCollection.description"
              class="textarea textarea-bordered" 
              placeholder="输入集合描述"
              rows="3"
            ></textarea>
          </div>
          
          <div class="modal-action">
            <button type="button" @click="showCreateCollectionModal = false" class="btn">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="creating">
              <span v-if="creating" class="loading loading-spinner loading-sm"></span>
              {{ creating ? '创建中...' : '创建' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 文档摄取模态框 -->
    <div v-if="showIngestDocumentModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">添加文档到 {{ selectedCollection?.name }}</h3>
        
        <form @submit.prevent="ingestDocument">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">选择文件</span>
            </label>
            <div class="flex gap-2">
              <input 
                ref="fileInput"
                type="file" 
                class="file-input file-input-bordered flex-1" 
                accept=".txt,.md,.pdf,.docx"
                multiple
                @change="handleFileSelect"
              >
              <button 
                type="button"
                @click="selectFileWithDialog"
                class="btn btn-outline"
                title="选择单个文件"
              >
                <i class="fas fa-file"></i>
              </button>
              <button 
                type="button"
                @click="selectFolderWithDialog"
                class="btn btn-outline"
                title="选择文件夹"
              >
                <i class="fas fa-folder-open"></i>
              </button>
            </div>
            <label class="label">
              <span class="label-text-alt">支持格式: TXT, MD, PDF, DOCX</span>
            </label>
          </div>
          
          <div v-if="selectedFiles.length > 0" class="alert alert-info mb-4">
            <i class="fas fa-info-circle"></i>
            <div>
              <span v-if="selectedFiles.length === 1">已选择文件: {{ selectedFiles[0].name }}</span>
              <span v-else>已选择 {{ selectedFiles.length }} 个文件</span>
              <div v-if="selectedFolder" class="text-sm mt-1">
                文件夹: {{ selectedFolder }}
              </div>
            </div>
          </div>
          
          <!-- 文件列表 -->
          <div v-if="selectedFiles.length > 1" class="mb-4">
            <h5 class="font-semibold mb-2">文件列表:</h5>
            <div class="max-h-32 overflow-y-auto bg-base-200 rounded p-2">
              <div v-for="(file, index) in selectedFiles" :key="index" class="text-sm py-1">
                <i class="fas fa-file mr-2"></i>{{ file.name }}
              </div>
            </div>
          </div>
          
          <div class="modal-action">
            <button type="button" @click="showIngestDocumentModal = false" class="btn">
              取消
            </button>
            <button type="submit" class="btn btn-primary" :disabled="ingesting || selectedFiles.length === 0">
              <span v-if="ingesting" class="loading loading-spinner loading-sm"></span>
              {{ ingesting ? `处理中... ${batchProgress.current}/${batchProgress.total}` : (selectedFiles.length > 1 ? `添加 ${selectedFiles.length} 个文档` : '添加文档') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 集合详情模态框 -->
    <div v-if="showCollectionDetailsModal" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">集合详情: {{ selectedCollection?.name }}</h3>
        
        <!-- 基本信息 -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">文档数量</div>
            <div class="stat-value text-primary">{{ collectionDetails.stats.totalDocuments }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">文本块数量</div>
            <div class="stat-value text-secondary">{{ collectionDetails.stats.totalChunks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">嵌入模型</div>
            <div class="stat-value text-sm">{{ collectionDetails.stats.embeddingModel }}</div>
          </div>
        </div>

        <!-- 集合描述 -->
        <div class="mb-4" v-if="selectedCollection?.description">
          <h4 class="font-semibold mb-2">描述</h4>
          <p class="text-base-content/70">{{ selectedCollection.description }}</p>
        </div>

        <!-- 操作按钮 -->
        <div class="flex gap-2 mb-4">
          <button 
            @click="switchToIngestModal"
            class="btn btn-primary btn-sm"
          >
            <i class="fas fa-plus mr-2"></i>
            添加文档
          </button>
          <button 
            @click="switchToQueryModal"
            class="btn btn-outline btn-sm"
          >
            <i class="fas fa-search mr-2"></i>
            查询文档
          </button>
          <button @click="closeDetailsModal" class="btn btn-sm">
            关闭
          </button>
        </div>

        <!-- 文档列表 -->
        <div class="mb-2 flex items-center justify-between gap-2">
          <h4 class="font-semibold">文档列表</h4>
          <div class="flex items-center gap-2">
            <input
              v-model="documentSearch"
              type="text"
              placeholder="按文件名搜索..."
              class="input input-bordered input-sm"
            >
            <select v-model.number="docPageSize" class="select select-bordered select-sm">
              <option :value="10">10/页</option>
              <option :value="20">20/页</option>
              <option :value="50">50/页</option>
            </select>
            <button class="btn btn-ghost btn-xs" @click="reloadDocuments" :disabled="loadingDocuments">
              <span v-if="loadingDocuments" class="loading loading-spinner loading-xs"></span>
              刷新
            </button>
          </div>
        </div>
        <div class="overflow-x-auto border rounded-lg">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>文件名</th>
                <th>大小</th>
                <th>创建时间</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="loadingDocuments">
                <td colspan="4">
                  <div class="flex items-center gap-2">
                    <span class="loading loading-spinner loading-sm"></span>
                    正在加载文档...
                  </div>
                </td>
              </tr>
              <tr v-else-if="documents.length === 0">
                <td colspan="4" class="text-base-content/60">暂无文档</td>
              </tr>
              <tr v-else v-for="doc in paginatedDocuments" :key="doc.id">
                <td>
                  <div class="font-medium">{{ doc.file_name }}</div>
                  <div class="text-xs opacity-60">{{ doc.file_path }}</div>
                </td>
                <td>{{ formatBytes(doc.file_size) }}</td>
                <td>{{ formatDate(doc.created_at) }}</td>
                <td>
                  <div class="flex gap-2">
                    <button class="btn btn-ghost btn-xs" title="预览" @click="viewDocument(doc)">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs text-error" title="删除" @click="deleteDocument(doc)">
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 分页器 -->
        <div class="flex items-center justify-end gap-2 mt-3">
          <button class="btn btn-xs" :disabled="docCurrentPage <= 1" @click="docCurrentPage = docCurrentPage - 1">上一页</button>
          <div class="text-sm">第 {{ docCurrentPage }} / {{ totalDocPages }} 页（共 {{ filteredDocuments.length }} 条）</div>
          <button class="btn btn-xs" :disabled="docCurrentPage >= totalDocPages" @click="docCurrentPage = docCurrentPage + 1">下一页</button>
        </div>

        <!-- 加载状态 -->
        <div v-if="loadingDetails" class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      </div>
    </div>

    <!-- 文档预览模态框 -->
    <div v-if="showDocumentModal" class="modal modal-open">
      <div class="modal-box max-w-5xl">
        <h3 class="font-bold text-lg mb-2">文档预览: {{ selectedDocument?.file_name }}</h3>
        <div class="text-xs text-base-content/60 mb-4 break-all">{{ selectedDocument?.file_path }}</div>

        <div class="mb-3 flex items-center justify-between">
          <div class="text-sm">文本块: {{ documentChunks.length }}</div>
          <button class="btn btn-ghost btn-xs" @click="reloadDocumentChunks" :disabled="loadingChunks">
            <span v-if="loadingChunks" class="loading loading-spinner loading-xs"></span>
            刷新
          </button>
        </div>

        <div v-if="loadingChunks" class="flex items-center gap-2">
          <span class="loading loading-spinner loading-sm"></span>
          正在加载内容...
        </div>
        <div v-else class="space-y-3 max-h-[60vh] overflow-y-auto">
          <div v-for="(chunk, idx) in documentChunks" :key="chunk.id" class="card bg-base-200">
            <div class="card-body p-4">
              <div class="flex items-center justify-between mb-2">
                <div class="badge badge-outline">#{{ idx + 1 }}</div>
                <div class="text-xs text-base-content/60">{{ formatDate(chunk.created_at) }}</div>
              </div>
              <pre class="text-sm whitespace-pre-wrap break-words">{{ chunk.content }}</pre>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn" @click="showDocumentModal = false">关闭</button>
        </div>
      </div>
    </div>

    <!-- 查询模态框 -->
    <div v-if="showQueryModal" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">查询 {{ selectedCollection?.name }}</h3>
        
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">查询内容</span>
          </label>
          <textarea 
            v-model="queryText"
            class="textarea textarea-bordered" 
            placeholder="输入您的查询..."
            rows="3"
          ></textarea>
        </div>
        
        <div class="flex gap-4 mb-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">返回结果数</span>
            </label>
            <input 
              v-model.number="queryTopK"
              type="number" 
              class="input input-bordered w-24" 
              min="1" 
              max="20"
            >
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">使用嵌入检索</span>
            </label>
            <input type="checkbox" class="toggle" v-model="queryUseEmbedding" />
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">启用重排</span>
            </label>
            <input type="checkbox" class="toggle" v-model="queryReranking" />
          </div>
        </div>
        
        <div class="flex gap-2 mb-4">
          <button 
            @click="executeQuery"
            class="btn btn-primary" 
            :disabled="querying || !queryText.trim()"
          >
            <span v-if="querying" class="loading loading-spinner loading-sm"></span>
            {{ querying ? '查询中...' : '执行查询' }}
          </button>
          <button @click="showQueryModal = false" class="btn">
            关闭
          </button>
        </div>
        
        <!-- 查询结果 -->
        <div v-if="queryResults.length > 0" class="mt-6">
          <h4 class="font-bold mb-4">查询结果</h4>
          <div class="space-y-4">
            <div 
              v-for="(result, index) in queryResults" 
              :key="index"
              class="card bg-base-200 shadow-sm"
            >
              <div class="card-body p-4">
                <div class="flex justify-between items-start mb-2">
                  <div class="badge badge-primary">相似度: {{ (result.score * 100).toFixed(1) }}%</div>
                  <div class="badge badge-outline">排名: {{ result.rank }}</div>
                </div>
                <p class="text-sm">{{ result.chunk.content }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Toast 通知 -->
    <Toast 
      v-if="toast.show"
      :message="toast.message"
      :type="toast.type"
      @close="toast.show = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Toast from '@/components/Toast.vue'

// 响应式数据
const collections = ref([])
const searchQuery = ref('')
const statusFilter = ref('')
const showCreateCollectionModal = ref(false)
const showIngestDocumentModal = ref(false)
const showQueryModal = ref(false)
const showCollectionDetailsModal = ref(false)
const creating = ref(false)
const ingesting = ref(false)
const querying = ref(false)
const loadingDetails = ref(false)
const loadingDocuments = ref(false)
const loadingChunks = ref(false)

// 激活集合管理（后端持久化）
const onActiveToggle = async (collection: any, ev: Event) => {
  const checked = (ev.target as HTMLInputElement)?.checked ?? false
  try {
    await invoke('set_rag_collection_active', { collectionId: collection.id, active: checked })
    collection.is_active = checked
    showToast(checked ? '已激活该集合，AI助手将联合检索' : '已取消激活该集合', 'info')
  } catch (e) {
    console.error('更新激活状态失败:', e)
    // revert UI
    collection.is_active = !checked
    showToast('更新激活状态失败', 'error')
  }
}

// 新集合表单
const newCollection = ref({
  name: '',
  description: ''
})

// 文档摄取
const selectedCollection = ref(null)
const selectedFiles = ref([])
const selectedFolder = ref('')
const fileInput = ref(null)
const batchProgress = ref({
  current: 0,
  total: 0,
  success: 0,
  failed: 0
})

// 查询相关
const queryText = ref('')
const queryTopK = ref(5)
const queryResults = ref([])
const queryUseEmbedding = ref(true)
const queryReranking = ref(true)

// 集合详情相关
interface CollectionStats {
  totalDocuments: number
  totalChunks: number
  embeddingModel: string
}

interface CollectionDetails {
  documents: any[]
  chunks: any[]
  stats: CollectionStats
}

const collectionDetails = ref<CollectionDetails>({
  documents: [],
  chunks: [],
  stats: {
    totalDocuments: 0,
    totalChunks: 0,
    embeddingModel: 'default'
  }
})

// 文档浏览
const documents = ref<any[]>([])
const selectedDocument = ref<any | null>(null)
const showDocumentModal = ref(false)
const documentChunks = ref<any[]>([])
const documentSearch = ref('')
const docPageSize = ref(10)
const docCurrentPage = ref(1)

// Toast 通知
const toast = ref({
  show: false,
  message: '',
  type: 'info'
})

// 计算属性
const filteredCollections = computed(() => {
  let filtered = collections.value as any[]

  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    filtered = filtered.filter(c => 
      (c.name || '').toLowerCase().includes(q) ||
      ((c.description || '').toLowerCase().includes(q))
    )
  }

  if (statusFilter.value === 'active') {
    filtered = filtered.filter(c => !!c.is_active)
  } else if (statusFilter.value === 'inactive') {
    filtered = filtered.filter(c => !c.is_active)
  }

  return filtered
})

const totalDocuments = computed(() => {
  return collections.value.reduce((sum, collection) => sum + (collection.document_count || 0), 0)
})

const totalChunks = computed(() => {
  return collections.value.reduce((sum, collection) => sum + (collection.chunk_count || 0), 0)
})

const totalQueries = computed(() => {
  return collections.value.reduce((sum, collection) => sum + (collection.query_count || 0), 0)
})

// 方法
const showToast = (message: string, type: string = 'info') => {
  toast.value = { show: true, message, type }
  setTimeout(() => {
    toast.value.show = false
  }, 3000)
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString('zh-CN')
}

const formatBytes = (bytes: number) => {
  if (!bytes || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let val = bytes
  while (val >= 1024 && i < units.length - 1) {
    val = val / 1024
    i++
  }
  return `${val.toFixed(1)} ${units[i]}`
}

const refreshCollections = async () => {
  try {
    const status = await invoke('get_rag_status') as any
    collections.value = status.collections || []
  } catch (error) {
    console.error('获取集合列表失败:', error)
    showToast('获取集合列表失败', 'error')
  }
}

const createCollection = async () => {
  if (!newCollection.value.name) {
    showToast('请填写集合名称', 'warning')
    return
  }

  creating.value = true
  try {
    await invoke('create_rag_collection', {
      name: newCollection.value.name,
      description: newCollection.value.description || null
    })
    
    showToast('集合创建成功', 'success')
    showCreateCollectionModal.value = false
    newCollection.value = { name: '', description: '' }
    await refreshCollections()
  } catch (error) {
    console.error('创建集合失败:', error)
    showToast('创建集合失败: ' + error, 'error')
  } finally {
    creating.value = false
  }
}

const viewCollection = (collection: any) => {
  selectedCollection.value = collection
  showCollectionDetailsModal.value = true
  loadCollectionDetails(collection.id)
  reloadDocuments()
  docCurrentPage.value = 1
  documentSearch.value = ''
}

const loadCollectionDetails = async (collectionId: string) => {
  loadingDetails.value = true
  try {
    // 这里可以添加获取集合详细信息的API调用
    // 暂时显示基本信息
    collectionDetails.value = {
      documents: [],
      chunks: [],
      stats: {
        totalDocuments: selectedCollection.value?.document_count || 0,
        totalChunks: selectedCollection.value?.chunk_count || 0,
        embeddingModel: selectedCollection.value?.embedding_model || 'default'
      }
    }
  } catch (error) {
    console.error('加载集合详情失败:', error)
    showToast('加载集合详情失败: ' + error, 'error')
  } finally {
    loadingDetails.value = false
  }
}

const reloadDocuments = async () => {
  if (!selectedCollection.value) return
  loadingDocuments.value = true
  try {
    const list = await invoke('list_rag_documents', { collectionId: selectedCollection.value.id }) as any[]
    documents.value = list || []
  } catch (e) {
    console.error('获取文档列表失败:', e)
    showToast('获取文档列表失败', 'error')
  } finally {
    loadingDocuments.value = false
  }
}

const filteredDocuments = computed(() => {
  const list = documents.value || []
  const q = (documentSearch.value || '').trim().toLowerCase()
  if (!q) return list
  return list.filter((d: any) => (d.file_name || '').toLowerCase().includes(q))
})

const totalDocPages = computed(() => {
  const total = filteredDocuments.value.length
  const size = Math.max(1, Number(docPageSize.value) || 10)
  return Math.max(1, Math.ceil(total / size))
})

const paginatedDocuments = computed(() => {
  const size = Math.max(1, Number(docPageSize.value) || 10)
  const page = Math.min(Math.max(1, docCurrentPage.value), totalDocPages.value)
  const start = (page - 1) * size
  return filteredDocuments.value.slice(start, start + size)
})

watch([filteredDocuments, docPageSize], () => {
  // 当过滤或页大小变化时，重置/矫正页码
  if (docCurrentPage.value > totalDocPages.value) {
    docCurrentPage.value = 1
  }
})

const viewDocument = async (doc: any) => {
  selectedDocument.value = doc
  showDocumentModal.value = true
  await reloadDocumentChunks()
}

const reloadDocumentChunks = async () => {
  if (!selectedDocument.value) return
  loadingChunks.value = true
  try {
    const chunks = await invoke('get_rag_document_chunks', { documentId: selectedDocument.value.id }) as any[]
    documentChunks.value = chunks || []
  } catch (e) {
    console.error('获取文档内容失败:', e)
    showToast('获取文档内容失败', 'error')
  } finally {
    loadingChunks.value = false
  }
}

const deleteDocument = async (doc: any) => {
  try {
    await invoke('delete_rag_document', { documentId: doc.id })
    showToast('文档删除成功', 'success')
    await reloadDocuments()
    await refreshCollections()
  } catch (e) {
    console.error('删除文档失败:', e)
    showToast('删除文档失败: ' + e, 'error')
  }
}

const showIngestModal = (collection: any) => {
  selectedCollection.value = collection
  showIngestDocumentModal.value = true
}

const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    selectedFiles.value = Array.from(target.files)
    selectedFolder.value = '' // 清除文件夹选择
  }
}

const selectFileWithDialog = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const filePaths = await open({
      multiple: true,
      filters: [
        {
          name: 'Documents',
          extensions: ['txt', 'md', 'pdf', 'docx']
        }
      ]
    })
    
    if (filePaths && filePaths.length > 0) {
      // 创建文件对象数组
      selectedFiles.value = filePaths.map(filePath => ({
        name: filePath.split('/').pop() || 'unknown',
        path: filePath,
        size: 0, // 无法获取大小，设为0
        type: 'application/octet-stream'
      }))
      selectedFolder.value = '' // 清除文件夹选择
    }
  } catch (error) {
    console.error('文件选择失败:', error)
    showToast('文件选择失败: ' + error, 'error')
  }
}

const selectFolderWithDialog = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const folderPath = await open({
      directory: true,
      multiple: false
    })
    
    if (folderPath) {
      selectedFolder.value = folderPath
      // 调用后端命令来获取文件夹中的所有支持的文件
      try {
        const files = await invoke('get_folder_files', {
          folderPath: folderPath,
          extensions: ['txt', 'md', 'pdf', 'docx']
        }) as string[]
        
        selectedFiles.value = files.map(filePath => ({
          name: filePath.split('/').pop() || 'unknown',
          path: filePath,
          size: 0,
          type: 'application/octet-stream'
        }))
        
        if (files.length === 0) {
          showToast('文件夹中没有找到支持的文档文件', 'warning')
        } else {
          showToast(`找到 ${files.length} 个文档文件`, 'success')
        }
      } catch (error) {
        console.error('获取文件夹文件失败:', error)
        showToast('获取文件夹文件失败: ' + error, 'error')
      }
    }
  } catch (error) {
    console.error('文件夹选择失败:', error)
    showToast('文件夹选择失败: ' + error, 'error')
  }
}

const ingestDocument = async () => {
  if (selectedFiles.value.length === 0 || !selectedCollection.value) {
    showToast('请选择文件', 'warning')
    return
  }

  ingesting.value = true
  
  // 初始化批量处理进度
  batchProgress.value = {
    current: 0,
    total: selectedFiles.value.length,
    success: 0,
    failed: 0
  }
  
  const failedFiles = []
  let totalChunks = 0
  
  try {
    for (let i = 0; i < selectedFiles.value.length; i++) {
      const file = selectedFiles.value[i]
      batchProgress.value.current = i + 1
      
      try {
        const filePath = file.path || file.name
        
        const response = await invoke('rag_ingest_source', {
          filePath: filePath,
          collectionId: selectedCollection.value.id,
          metadata: {
            originalName: file.name,
            fileSize: file.size.toString(),
            fileType: file.type
          }
        }) as any
        
        totalChunks += response.chunks_created || 0
        batchProgress.value.success++
        
      } catch (error) {
        console.error(`文件 ${file.name} 摄取失败:`, error)
        batchProgress.value.failed++
        failedFiles.push({
          name: file.name,
          error: error.toString()
        })
      }
    }
    
    // 显示最终结果
    if (batchProgress.value.failed === 0) {
      showToast(`所有文档添加成功！共处理了 ${totalChunks} 个文本块`, 'success')
    } else if (batchProgress.value.success > 0) {
      showToast(`部分文档添加成功！成功: ${batchProgress.value.success}, 失败: ${batchProgress.value.failed}, 共处理了 ${totalChunks} 个文本块`, 'warning')
    } else {
      showToast('所有文档添加失败', 'error')
    }
    
    // 如果有失败的文件，在控制台输出详细错误信息
    if (failedFiles.length > 0) {
      console.warn('失败的文件:', failedFiles)
    }
    
    showIngestDocumentModal.value = false
    selectedFiles.value = []
    selectedFolder.value = ''
    if (fileInput.value) {
      fileInput.value.value = ''
    }
    await refreshCollections()
    
  } catch (error) {
    console.error('批量文档摄取失败:', error)
    showToast('批量文档摄取失败: ' + error, 'error')
  } finally {
    ingesting.value = false
    batchProgress.value = { current: 0, total: 0, success: 0, failed: 0 }
  }
}

const queryCollection = (collection: any) => {
  selectedCollection.value = collection
  queryText.value = ''
  queryResults.value = []
  showQueryModal.value = true
}

const executeQuery = async () => {
  if (!queryText.value.trim() || !selectedCollection.value) {
    showToast('请输入查询内容', 'warning')
    return
  }

  querying.value = true
  try {
    const response = await invoke('query_rag', {
      request: {
        collectionId: selectedCollection.value.id,
        query: queryText.value,
        top_k: queryTopK.value,
        use_embedding: queryUseEmbedding.value,
        reranking_enabled: queryReranking.value
      }
    }) as any
    
    queryResults.value = response.results || []
    showToast(`找到 ${queryResults.value.length} 个相关结果`, 'success')
  } catch (error) {
    console.error('查询失败:', error)
    showToast('查询失败: ' + error, 'error')
  } finally {
    querying.value = false
  }
}

const deleteCollection = async (collection: any) => {

  try {
    await invoke('delete_rag_collection', { collectionId: collection.id })
    showToast('集合删除成功', 'success')
    await refreshCollections()
  } catch (error) {
    console.error('删除集合失败:', error)
    showToast('删除集合失败: ' + error, 'error')
  }
}

// 模态框切换方法，避免闪屏
const switchToIngestModal = async () => {
  showCollectionDetailsModal.value = false
  await nextTick() // 等待 DOM 更新
  // 短暂延迟确保过渡动画完成
  setTimeout(() => {
    showIngestModal(selectedCollection.value)
  }, 150)
}

const switchToQueryModal = async () => {
  showCollectionDetailsModal.value = false
  await nextTick() // 等待 DOM 更新
  setTimeout(() => {
    queryCollection(selectedCollection.value)
  }, 150)
}

const closeDetailsModal = () => {
  showCollectionDetailsModal.value = false
}

// 生命周期
onMounted(() => {
  refreshCollections()
})
</script>

<style scoped>
.container {
  max-width: 1200px;
}

.stat {
  padding: 1.5rem;
}

.modal-box {
  max-height: 90vh;
  overflow-y: auto;
  transition: opacity 0.15s ease-in-out, transform 0.15s ease-in-out;
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}

.card-body {
  padding: 1.5rem;
}

.table th {
  background-color: hsl(var(--b2));
}

.badge {
  font-size: 0.75rem;
}

.loading {
  margin-right: 0.5rem;
}

/* 模态框过渡动画 */
.modal {
  transition: opacity 0.15s ease-in-out;
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
}

.modal.modal-open {
  animation: modal-fade-in 0.15s ease-out;
}

@keyframes modal-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.modal-box {
  animation: modal-scale-in 0.15s ease-out;
}

@keyframes modal-scale-in {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* 按钮点击效果优化 */
.btn {
  transition: transform 0.1s ease-in-out, box-shadow 0.1s ease-in-out;
  will-change: transform;
}

.btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.btn:active {
  transform: translateY(0);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

/* 防止模态框内按钮hover时的闪屏 */
.modal .btn {
  transition: background-color 0.1s ease-in-out, color 0.1s ease-in-out;
  transform: none !important;
}

.modal .btn:hover {
  transform: none !important;
  box-shadow: none !important;
}

.modal .btn:active {
  transform: none !important;
}

/* 特殊处理表格内的小按钮 */
.btn.btn-xs {
  transition: background-color 0.1s ease-in-out, color 0.1s ease-in-out;
  transform: none !important;
}

.btn.btn-xs:hover {
  transform: none !important;
  box-shadow: none !important;
}

/* 防止文件输入按钮闪屏 */
.file-input {
  transition: border-color 0.1s ease-in-out;
}

/* 优化加载动画，防止闪烁 */
.loading {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>