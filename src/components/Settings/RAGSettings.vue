<template>
  <div class="rag-settings">
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">知识库配置</h2>
      <div class="flex gap-2">
        <button class="btn btn-outline" @click="testEmbeddingConnection">
          <i class="fas fa-vial"></i>
          测试嵌入连接
        </button>
        <button class="btn btn-outline" @click="resetRagConfig">
          <i class="fas fa-undo"></i>
          重置为默认
        </button>
        <button class="btn btn-primary" @click="saveRagConfig">
          <i class="fas fa-save"></i>
          保存配置
        </button>
      </div>
    </div>

    <!-- RAG 配置主体 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 左侧：嵌入配置 -->
      <div class="card bg-base-100 shadow-sm">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-vector-square"></i>
            嵌入配置
          </h3>
          
          <div class="space-y-4">
            <!-- 嵌入提供商选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">嵌入提供商</span>
              </label>
              <select class="select select-bordered" 
                      v-model="ragConfig.embedding_provider" 
                      @change="onProviderChange('rag_embedding', $event)">
                <option value="">选择提供商</option>
                <option v-for="provider in availableProviders" :key="provider" :value="provider">
                  {{ provider }}
                </option>
              </select>
            </div>

            <!-- 嵌入模型选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">嵌入模型</span>
              </label>
              <select class="select select-bordered" 
                      v-model="ragConfig.embedding_model" 
                      @change="saveRagConfig"
                      :disabled="!ragConfig.embedding_provider">
                <option value="">选择嵌入模型</option>
                <option v-for="model in getEmbeddingModels(ragConfig.embedding_provider)" 
                        :key="model.id" :value="model.id">
                  {{ model.name }}
                </option>
              </select>
            </div>

            <!-- 嵌入维度 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">嵌入维度</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="ragConfig.embedding_dimensions"
                     @blur="saveRagConfig"
                     placeholder="自动检测">
              <label class="label">
                <span class="label-text-alt">留空则自动检测模型维度</span>
              </label>
            </div>

            
          </div>
        </div>
      </div>

      <!-- 右侧：分块配置 -->
      <div class="card bg-base-100 shadow-sm">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-cut"></i>
            分块配置
          </h3>
          
          <div class="space-y-4">
            <!-- 分块策略 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">分块策略</span>
              </label>
              <select class="select select-bordered" v-model="ragConfig.chunking_strategy" @change="saveRagConfig">
                <option value="FixedSize">固定大小</option>
                <option value="RecursiveCharacter">递归字符分割 (推荐)</option>
                <option value="Semantic">语义分块</option>
                <option value="StructureAware">结构感知分块</option>
              </select>
              <label class="label">
                <span class="label-text-alt">递归字符：优先按段落/句子/词分割</span>
              </label>
            </div>

            <!-- 分块大小 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">分块大小 (字符)</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1"
                       v-model.number="ragConfig.chunk_size_chars"
                       min="200" max="2000" step="100"
                       @change="saveRagConfig">
                <span class="text-sm min-w-[80px]">{{ ragConfig.chunk_size_chars }}</span>
              </div>
              <label class="label">
                <span class="label-text-alt">推荐范围: 500-1500字符</span>
              </label>
            </div>

            <!-- 重叠大小 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">重叠大小 (字符)</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-secondary flex-1"
                       v-model.number="ragConfig.chunk_overlap_chars"
                       min="0" :max="Math.floor(ragConfig.chunk_size_chars * 0.5)" step="50"
                       @change="saveRagConfig">
                <span class="text-sm min-w-[80px]">{{ ragConfig.chunk_overlap_chars }}</span>
              </div>
              <label class="label">
                <span class="label-text-alt">重叠有助于保持上下文连续性</span>
              </label>
            </div>

            <!-- 最小/最大分块大小 -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最小分块大小 (字符)</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="ragConfig.min_chunk_size_chars"
                       @blur="saveRagConfig"
                       min="50" max="1000"
                       placeholder="100">
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">最大分块大小 (字符)</span>
                </label>
                <input type="number" class="input input-bordered" 
                       v-model.number="ragConfig.max_chunk_size_chars"
                       @blur="saveRagConfig"
                       min="1000" max="4000"
                       placeholder="3000">
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 重排序模型配置 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-sort-amount-down"></i>
          重排序模型配置
        </h3>
        
        <div class="space-y-4">
          <!-- 启用重排序 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用重排序</span>
              <input type="checkbox" class="toggle toggle-warning" 
                     v-model="ragConfig.reranking_enabled"
                     @change="saveRagConfig">
            </label>
            <label class="label">
              <span class="label-text-alt">重排序可以提高检索结果的相关性，但会增加处理时间</span>
            </label>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-4" 
               :class="{ 'opacity-50': !ragConfig.reranking_enabled }">
            <!-- 重排序提供商选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">重排序提供商</span>
              </label>
              <select class="select select-bordered" 
                      v-model="ragConfig.reranking_provider" 
                      @change="onProviderChange('rag_reranking', $event)"
                      :disabled="!ragConfig.reranking_enabled">
                <option value="">选择提供商</option>
                <option v-for="provider in availableProviders" :key="provider" :value="provider">
                  {{ provider }}
                </option>
              </select>
            </div>

            <!-- 重排序模型选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">重排序模型</span>
              </label>
              <select class="select select-bordered" 
                      v-model="ragConfig.reranking_model" 
                      @change="saveRagConfig"
                      :disabled="!ragConfig.reranking_provider || !ragConfig.reranking_enabled">
                <option value="">选择重排序模型</option>
                <option v-for="model in getRerankingModels(ragConfig.reranking_provider)" 
                        :key="model.id" :value="model.id">
                  {{ model.name }}
                </option>
              </select>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 检索配置 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-search"></i>
          检索配置
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <!-- 检索数量 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">检索数量 (Top-K)</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="ragConfig.top_k"
                   @blur="saveRagConfig"
                   min="1" max="20"
                   placeholder="5">
            <label class="label">
              <span class="label-text-alt">返回最相关的K个文档块</span>
            </label>
          </div>

          <!-- MMR Lambda -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">多样性参数 (MMR Lambda)</span>
            </label>
            <div class="flex items-center gap-4">
              <input type="range" class="range range-accent flex-1"
                     v-model.number="ragConfig.mmr_lambda"
                     min="0" max="1" step="0.1"
                     @change="saveRagConfig">
              <span class="text-sm min-w-[60px]">{{ ragConfig.mmr_lambda }}</span>
            </div>
            <label class="label">
              <span class="label-text-alt">0=多样性优先, 1=相似性优先</span>
            </label>
          </div>

          <!-- 相似度阈值 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">相似度阈值</span>
            </label>
            <div class="flex items-center gap-4">
              <input type="range" class="range range-warning flex-1"
                     v-model.number="ragConfig.similarity_threshold"
                     min="0" max="1" step="0.05"
                     @change="saveRagConfig">
              <span class="text-sm min-w-[60px]">{{ ragConfig.similarity_threshold || 0.65 }}</span>
            </div>
            <label class="label">
              <span class="label-text-alt">低于此阈值的结果将被过滤</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- 性能配置 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-tachometer-alt"></i>
          性能配置
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">批处理大小</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="ragConfig.batch_size"
                   @blur="saveRagConfig"
                   min="1" max="100"
                   placeholder="10">
            <label class="label">
              <span class="label-text-alt">每批处理的文档数量</span>
            </label>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">最大并发数</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="ragConfig.max_concurrent"
                   @blur="saveRagConfig"
                   min="1" max="16"
                   placeholder="4">
            <label class="label">
              <span class="label-text-alt">并发处理的任务数</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">缓存大小 (MB)</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="ragConfig.cache_size_mb"
                   @blur="saveRagConfig"
                   min="10" max="1000"
                   placeholder="100">
            <label class="label">
              <span class="label-text-alt">嵌入向量缓存大小</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">超时时间 (秒)</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="ragConfig.timeout_seconds"
                   @blur="saveRagConfig"
                   min="5" max="300"
                   placeholder="30">
            <label class="label">
              <span class="label-text-alt">单次请求超时时间</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- 高级配置 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-cogs"></i>
          高级配置
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <!-- 左侧：功能开关 -->
          <!-- <div class="space-y-4">
            <h4 class="text-lg font-semibold border-b pb-2">功能开关</h4>
            
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">启用RAG增强</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="ragConfig.augmentation_enabled"
                       @change="saveRagConfig">
              </label>
              <label class="label">
                <span class="label-text-alt">开启后AI回答将基于知识库内容</span>
              </label>
            </div>


          </div> -->

          <!-- 右侧：质量控制 -->
          <div class="space-y-4">
            <h4 class="text-lg font-semibold border-b pb-2">质量控制</h4>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">最小文档长度 (字符)</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="ragConfig.min_doc_length"
                     @blur="saveRagConfig"
                     min="10" max="1000"
                     placeholder="50">
              <label class="label">
                <span class="label-text-alt">过滤过短的文档</span>
              </label>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">最大文档长度 (字符)</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="ragConfig.max_doc_length"
                     @blur="saveRagConfig"
                     min="1000" max="50000"
                     placeholder="10000">
              <label class="label">
                <span class="label-text-alt">截断过长的文档</span>
              </label>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">重复检测阈值</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-error flex-1"
                       v-model.number="ragConfig.duplicate_threshold"
                       min="0.5" max="1" step="0.05"
                       @change="saveRagConfig">
                <span class="text-sm min-w-[60px]">{{ ragConfig.duplicate_threshold || 0.95 }}</span>
              </div>
              <label class="label">
                <span class="label-text-alt">相似度超过此值的文档将被视为重复</span>
              </label>
            </div>

            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">启用内容过滤</span>
                <input type="checkbox" class="toggle toggle-error" 
                       v-model="ragConfig.content_filtering_enabled"
                       @change="saveRagConfig">
              </label>
              <label class="label">
                <span class="label-text-alt">过滤低质量或不相关的内容</span>
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 统计信息 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-chart-bar"></i>
          知识库统计
        </h3>
        
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">文档总数</div>
            <div class="stat-value text-primary">{{ ragStats.total_documents || 0 }}</div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">文档块数</div>
            <div class="stat-value text-secondary">{{ ragStats.total_chunks || 0 }}</div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">向量维度</div>
            <div class="stat-value text-accent">{{ ragStats.embedding_dimensions || 'N/A' }}</div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">索引大小</div>
            <div class="stat-value text-warning">{{ formatFileSize(ragStats.index_size_bytes || 0) }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Props
interface Props {
  ragConfig?: any
  saving?: boolean
  availableProviders?: string[]
  availableModels?: Array<{id: string, name: string, provider: string, type?: string}>
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:ragConfig': [value: any]
  'saveRagConfig': []
  'testEmbeddingConnection': []
  'resetRagConfig': []
}

const emit = defineEmits<Emits>()

// 本地状态
const ragStats = ref({
  total_documents: 0,
  total_chunks: 0,
  embedding_dimensions: null,
  index_size_bytes: 0
})

// Computed
const ragConfig = computed({
  get: () => props.ragConfig || {
    embedding_provider: 'ollama',
    embedding_model: 'nomic-embed-text',
    embedding_dimensions: null,
    embedding_api_key: '',
    embedding_base_url: 'http://localhost:11434',
    chunk_size_chars: 1000,
    chunk_overlap_chars: 200,
    chunking_strategy: 'RecursiveCharacter',
    min_chunk_size_chars: 100,
    max_chunk_size_chars: 3000,
    top_k: 5,
    mmr_lambda: 0.7,
    similarity_threshold: 0.65,
    batch_size: 10,
    max_concurrent: 4,
    cache_size_mb: 100,
    timeout_seconds: 30,
    augmentation_enabled: true,
    reranking_enabled: false,
    reranking_provider: '',
    reranking_model: '',
    min_doc_length: 50,
    max_doc_length: 10000,
    duplicate_threshold: 0.95,
    content_filtering_enabled: true
  },
  set: (value) => emit('update:ragConfig', value)
})

// Methods
const saveRagConfig = () => {
  emit('saveRagConfig')
}

const testEmbeddingConnection = () => {
  emit('testEmbeddingConnection')
}

const resetRagConfig = () => {
  emit('resetRagConfig')
}

// 获取嵌入模型列表
const getEmbeddingModels = (provider: string) => {
  if (!provider || !props.availableModels) return []
  
  // 从可用模型中过滤出该提供商的嵌入模型
  const providerModels = props.availableModels.filter(model => 
    model.provider.toLowerCase() === provider.toLowerCase()
  )
  
  // 如果模型有type字段，过滤embedding类型；否则返回所有模型（默认可用于embedding）
  if (providerModels.length > 0 && providerModels[0].type !== undefined) {
    return providerModels.filter(model => !model.type || model.type === 'embedding')
  }
  
  // 排除明显的reranking模型（通过名称识别）
  return providerModels.filter(model => 
    !model.name?.toLowerCase().includes('rerank') && 
    !model.id?.toLowerCase().includes('rerank')
  )
}

// 获取重排序模型列表
const getRerankingModels = (provider: string) => {
  if (!provider || !props.availableModels) return []
  
  // 从可用模型中过滤出该提供商的重排序模型
  const providerModels = props.availableModels.filter(model => 
    model.provider.toLowerCase() === provider.toLowerCase()
  )
  
  // 如果模型有type字段，过滤reranking类型；否则通过名称识别reranking模型
  if (providerModels.length > 0 && providerModels[0].type !== undefined) {
    return providerModels.filter(model => model.type === 'reranking')
  }
  
  // 通过名称识别reranking模型
  const rerankingModels = providerModels.filter(model => 
    model.name?.toLowerCase().includes('rerank') || 
    model.id?.toLowerCase().includes('rerank')
  )
  
  // 如果没有专门的重排序模型，对于某些提供商可以使用嵌入模型进行重排序
  if (rerankingModels.length === 0 && ['openai', 'azure'].includes(provider.toLowerCase())) {
    return providerModels.filter(model => 
      model.name?.toLowerCase().includes('embedding') || 
      model.id?.toLowerCase().includes('embedding')
    ).map(model => ({
      ...model,
      name: `${model.name} (Rerank)`
    }))
  }
  
  return rerankingModels
}

// 获取可用提供商列表
const availableProviders = computed(() => {
  return props.availableProviders || []
})

// 提供商变更处理
const onProviderChange = (stage: string, evt: Event) => {
  const target = evt?.target as HTMLSelectElement | null
  const provider = target?.value || ''
  
  // 当提供商改变时，重置对应的模型选择
  if (stage === 'rag_embedding') {
    ragConfig.value.embedding_model = ''
  } else if (stage === 'rag_reranking') {
    ragConfig.value.reranking_model = ''
  }
  
  saveRagConfig()
}

// 格式化文件大小
const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// 加载RAG统计信息
const loadRagStats = async () => {
  try {
    const stats = await invoke('get_rag_stats')
    if (stats && typeof stats === 'object') {
      ragStats.value = { ...ragStats.value, ...(stats as Record<string, any>) }
    }
  } catch (error) {
    console.warn('Failed to load RAG stats:', error)
  }
}

// 生命周期
onMounted(() => {
  loadRagStats()
})
</script>

<style scoped>
.rag-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}
</style>
