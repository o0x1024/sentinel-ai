<template>
  <div class="container mx-auto p-6">
    <!-- 页面标题 -->
    <div class="flex justify-between items-center mb-6">
      <h1 class="text-3xl font-bold">{{ t('ragManagement.title') }}</h1>
      <div class="flex gap-2">
        <button 
          @click="showCreateCollectionModal = true"
          class="btn btn-primary"
        >
          <i class="fas fa-plus mr-2"></i>
          {{ t('ragManagement.createCollection') }}
        </button>
        <button 
          @click="refreshCollections"
          class="btn btn-outline"
        >
          <i class="fas fa-refresh mr-2"></i>
          {{ t('ragManagement.refresh') }}
        </button>
      </div>
    </div>

    <!-- 统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-primary">
          <i class="fas fa-database text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('ragManagement.stats.totalCollections') }}</div>
        <div class="stat-value text-primary">{{ collections.length }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-secondary">
          <i class="fas fa-file-alt text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('ragManagement.stats.totalDocuments') }}</div>
        <div class="stat-value text-secondary">{{ totalDocuments }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-accent">
          <i class="fas fa-puzzle-piece text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('ragManagement.stats.totalChunks') }}</div>
        <div class="stat-value text-accent">{{ totalChunks }}</div>
      </div>
      
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-figure text-info">
          <i class="fas fa-search text-2xl"></i>
        </div>
        <div class="stat-title">{{ t('ragManagement.stats.totalQueries') }}</div>
        <div class="stat-value text-info">{{ totalQueries }}</div>
      </div>
    </div>

    <!-- 集合列表 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title mb-2">{{ t('ragManagement.collections.title') }}</h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ t('ragManagement.collections.description') }}
        </p>
        
        <!-- 搜索和过滤 -->
        <div class="flex gap-4 mb-4">
          <div class="form-control flex-1">
            <input 
              v-model="searchQuery"
              type="text" 
              :placeholder="t('ragManagement.collections.searchPlaceholder')" 
              class="input input-bordered"
            >
          </div>
          <select v-model="statusFilter" class="select select-bordered">
            <option value="">{{ t('ragManagement.collections.allStatus') }}</option>
            <option value="active">{{ t('ragManagement.collections.active') }}</option>
            <option value="inactive">{{ t('ragManagement.collections.inactive') }}</option>
          </select>
        </div>

        <!-- 集合表格 -->
        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>{{ t('ragManagement.table.name') }}</th>
                <th>{{ t('ragManagement.table.description') }}</th>
                <th>{{ t('ragManagement.table.documentCount') }}</th>
                <th>{{ t('ragManagement.table.createdAt') }}</th>
                <th class="text-center">{{ t('ragManagement.table.activate') }}</th>
                <th>{{ t('ragManagement.table.actions') }}</th>
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
                      <div class="flex items-center gap-2">
                        <span class="font-bold">{{ collection.name }}</span>
                        <span 
                          v-if="collection.name === 'agent_memory'"
                          class="badge badge-info badge-sm"
                          :title="t('ragManagement.collection.memoryTooltip')"
                        >
                          <i class="fas fa-brain mr-1"></i>
                          {{ t('ragManagement.collection.memoryBadge') }}
                        </span>
                      </div>
                      <div class="text-sm opacity-50">{{ collection.id }}</div>
                    </div>
                  </div>
                </td>
                <td>{{ collection.description || t('ragManagement.collection.noDescription') }}</td>
                <td>{{ collection.document_count }}</td>
                <td>{{ formatDate(collection.created_at) }}</td>
                <td class="text-center">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-primary"
                    :checked="!!collection.is_active"
                    @change="onActiveToggle(collection, $event)"
                    :aria-label="collection.is_active ? t('ragManagement.collections.activated') : t('ragManagement.collections.notActivated')"
                  />
                </td>
                <td>
                  <div class="flex gap-2">
                    <button 
                      @click="viewCollection(collection)"
                      class="btn btn-ghost btn-xs"
                      :title="t('ragManagement.actions.view')"
                    >
                      <i class="fas fa-eye"></i>
                    </button>
                    <button 
                      @click="editCollection(collection)"
                      class="btn btn-ghost btn-xs"
                      :title="t('ragManagement.actions.edit')"
                    >
                      <i class="fas fa-edit"></i>
                    </button>
                    <button 
                      @click="showIngestModal(collection)"
                      class="btn btn-ghost btn-xs"
                      :title="t('ragManagement.actions.add')"
                    >
                      <i class="fas fa-plus"></i>
                    </button>
                    <button 
                      @click="queryCollection(collection)"
                      class="btn btn-ghost btn-xs"
                      :title="t('ragManagement.actions.query')"
                    >
                      <i class="fas fa-search"></i>
                    </button>
                    <button 
                      @click="deleteCollection(collection)"
                      class="btn btn-ghost btn-xs text-error"
                      :title="t('ragManagement.actions.delete')"
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
    <div v-if="showCreateCollectionModal" class="modal modal-open" @click.self="showCreateCollectionModal = false">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('ragManagement.collection.createTitle') }}</h3>
        
        <form @submit.prevent="createCollection">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.collection.nameLabel') }}</span>
            </label>
            <input 
              v-model="newCollection.name"
              type="text" 
              class="input input-bordered" 
              :placeholder="t('ragManagement.collection.namePlaceholder')"
              required
            >
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.collection.descriptionLabel') }}</span>
            </label>
            <textarea 
              v-model="newCollection.description"
              class="textarea textarea-bordered" 
              :placeholder="t('ragManagement.collection.descriptionPlaceholder')"
              rows="3"
            ></textarea>
          </div>
          
          <div class="modal-action">
            <button type="button" @click="showCreateCollectionModal = false" class="btn">
              {{ t('ragManagement.cancel') }}
            </button>
            <button type="submit" class="btn btn-primary" :disabled="creating">
              <span v-if="creating" class="loading loading-spinner loading-sm"></span>
              {{ creating ? t('ragManagement.creating') : t('ragManagement.create') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 编辑集合模态框 -->
    <div v-if="showEditCollectionModal" class="modal modal-open" @click.self="showEditCollectionModal = false">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ t('ragManagement.collection.editTitle') }}</h3>
        
        <form @submit.prevent="updateCollection">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.collection.nameLabel') }}</span>
            </label>
            <input 
              v-model="editingCollection.name"
              type="text" 
              class="input input-bordered" 
              :placeholder="t('ragManagement.collection.namePlaceholder')"
              required
            >
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.collection.descriptionLabel') }}</span>
            </label>
            <textarea 
              v-model="editingCollection.description"
              class="textarea textarea-bordered" 
              :placeholder="t('ragManagement.collection.descriptionPlaceholder')"
              rows="3"
            ></textarea>
          </div>
          
          <div class="modal-action">
            <button type="button" @click="showEditCollectionModal = false" class="btn">
              {{ t('ragManagement.cancel') }}
            </button>
            <button type="submit" class="btn btn-primary" :disabled="updating">
              <span v-if="updating" class="loading loading-spinner loading-sm"></span>
              {{ updating ? t('ragManagement.saving') : t('ragManagement.save') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 文档摄取模态框 -->
    <div v-if="showIngestDocumentModal" class="modal modal-open" @click.self="closeIngestModal">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ t('ragManagement.ingest.title', { name: selectedCollection?.name }) }}</h3>
        
        <!-- 选项卡切换 -->
        <div class="tabs tabs-boxed mb-4">
          <a 
            class="tab" 
            :class="{ 'tab-active': ingestMode === 'file' }"
            @click="ingestMode = 'file'"
          >
            <i class="fas fa-file-upload mr-2"></i>
            {{ t('ragManagement.ingest.modeFile') }}
          </a>
          <a 
            class="tab" 
            :class="{ 'tab-active': ingestMode === 'manual' }"
            @click="ingestMode = 'manual'"
          >
            <i class="fas fa-keyboard mr-2"></i>
            {{ t('ragManagement.ingest.modeManual') }}
          </a>
        </div>
        
        <form @submit.prevent="ingestDocument">
          <!-- 文件选择模式 -->
          <div v-if="ingestMode === 'file'">
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">{{ t('ragManagement.ingest.selectFile') }}</span>
              </label>
              <div class="flex gap-2">
                <input 
                  ref="fileInput"
                  type="file" 
                  class="file-input file-input-bordered flex-1" 
                  :accept="supportedFileTypes.map(t => '.' + t).join(',')"
                  multiple
                  @change="handleFileSelect"
                >
                <button 
                  type="button"
                  @click="selectFileWithDialog"
                  class="btn btn-outline"
                  :title="t('ragManagement.ingest.selectSingleFile')"
                >
                  <i class="fas fa-file"></i>
                </button>
                <button 
                  type="button"
                  @click="selectFolderWithDialog"
                  class="btn btn-outline"
                  :title="t('ragManagement.ingest.selectFolder')"
                >
                  <i class="fas fa-folder-open"></i>
                </button>
              </div>
              <label class="label">
                <span class="label-text-alt">{{ t('ragManagement.ingest.supportedFormats') }}</span>
              </label>
            </div>
            
            <div v-if="selectedFiles.length > 0" class="alert alert-info mb-4">
              <i class="fas fa-info-circle"></i>
              <div>
                <span v-if="selectedFiles.length === 1">{{ t('ragManagement.ingest.fileSelected', { name: selectedFiles[0].name }) }}</span>
                <span v-else>{{ t('ragManagement.ingest.filesSelected', { count: selectedFiles.length }) }}</span>
                <div v-if="selectedFolder" class="text-sm mt-1">
                  {{ t('ragManagement.ingest.folder', { path: selectedFolder }) }}
                </div>
              </div>
            </div>
            
            <!-- 文件列表 -->
            <div v-if="selectedFiles.length > 1 && failedFiles.length === 0" class="mb-4">
              <h5 class="font-semibold mb-2">{{ t('ragManagement.ingest.fileList') }}</h5>
              <div class="max-h-32 overflow-y-auto bg-base-200 rounded p-2">
                <div v-for="(file, index) in selectedFiles" :key="index" class="text-sm py-1">
                  <i class="fas fa-file mr-2"></i>{{ file.name }}
                </div>
              </div>
            </div>

            <!-- 批量导入进度条 -->
            <div v-if="ingesting && batchProgress.total > 0" class="mb-4">
              <div class="mb-2 flex justify-between text-sm">
                <span>{{ t('ragManagement.ingest.progress', 'Progress') }}: {{ batchProgress.current }} / {{ batchProgress.total }}</span>
                <span>
                  <span class="text-success">✓ {{ batchProgress.success }}</span>
                  <span v-if="batchProgress.failed > 0" class="text-error ml-2">✗ {{ batchProgress.failed }}</span>
                </span>
              </div>
              <progress 
                class="progress progress-primary w-full" 
                :value="batchProgress.current" 
                :max="batchProgress.total"
              ></progress>
            </div>

            <!-- 失败文件详情 -->
            <div v-if="failedFiles.length > 0" class="mb-4">
              <h5 class="font-semibold text-error mb-2">
                <i class="fas fa-exclamation-triangle mr-2"></i>
                {{ t('ragManagement.ingest.failedFiles', 'Failed Files') }}
              </h5>
              <div class="max-h-48 overflow-y-auto bg-error/5 border border-error/20 rounded p-2">
                <div v-for="(file, index) in failedFiles" :key="index" class="mb-2 last:mb-0">
                  <div class="text-sm font-medium text-error">{{ file.name }}</div>
                  <div class="text-xs opacity-70">{{ file.error }}</div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 手动输入模式 -->
          <div v-else>
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">{{ t('ragManagement.ingest.titleRequired') }}</span>
              </label>
              <input 
                v-model="manualInput.title"
                type="text" 
                class="input input-bordered" 
                :placeholder="t('ragManagement.ingest.titlePlaceholder')"
                required
              >
            </div>
            
            <div class="form-control mb-4">
              <label class="label">
                <span class="label-text">{{ t('ragManagement.ingest.contentRequired') }}</span>
              </label>
              <textarea 
                v-model="manualInput.content"
                class="textarea textarea-bordered h-48" 
                :placeholder="t('ragManagement.ingest.contentPlaceholder')"
                required
              ></textarea>
              <label class="label">
                <span class="label-text-alt">
                  {{ t('ragManagement.ingest.characterCount', { count: manualInput.content.length }) }}
                </span>
              </label>
            </div>
          </div>
          
          <div class="modal-action">
            <button type="button" @click="closeIngestModal" class="btn">
              {{ t('ragManagement.cancel') }}
            </button>
            <button 
              v-if="ingestMode === 'file'"
              type="submit" 
              class="btn btn-primary" 
              :disabled="ingesting || selectedFiles.length === 0"
            >
              <span v-if="ingesting" class="loading loading-spinner loading-sm"></span>
              <span v-if="ingesting && batchProgress.total > 0">
                {{ t('ragManagement.ingest.processingProgress', { current: batchProgress.current, total: batchProgress.total }) }}
                ({{ t('ragManagement.ingest.successCount', { count: batchProgress.success }) }})
              </span>
              <span v-else-if="ingesting">
                {{ t('ragManagement.processing') }}
              </span>
              <span v-else>
                {{ selectedFiles.length > 1 ? t('ragManagement.ingest.addMultiple', { count: selectedFiles.length }) : t('ragManagement.addDocument') }}
              </span>
            </button>
            <button 
              v-else
              type="submit" 
              class="btn btn-primary" 
              :disabled="ingesting || !manualInput.title.trim() || !manualInput.content.trim()"
            >
              <span v-if="ingesting" class="loading loading-spinner loading-sm"></span>
              {{ ingesting ? t('ragManagement.processing') : t('ragManagement.addDocument') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 集合详情模态框 -->
    <div v-if="showCollectionDetailsModal" class="modal modal-open" @click.self="closeDetailsModal">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ t('ragManagement.details.title', { name: selectedCollection?.name }) }}</h3>
        
        <!-- 基本信息 -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">{{ t('ragManagement.details.documentCount') }}</div>
            <div class="stat-value text-primary">{{ collectionDetails.stats.totalDocuments }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">{{ t('ragManagement.details.chunkCount') }}</div>
            <div class="stat-value text-secondary">{{ collectionDetails.stats.totalChunks }}</div>
          </div>
        </div>

        <!-- 集合描述 -->
        <div class="mb-4" v-if="selectedCollection?.description">
          <h4 class="font-semibold mb-2">{{ t('ragManagement.details.description') }}</h4>
          <p class="text-base-content/70">{{ selectedCollection.description }}</p>
        </div>

        <!-- 操作按钮 -->
        <div class="flex gap-2 mb-4">
          <button 
            @click="switchToIngestModal"
            class="btn btn-primary btn-sm"
          >
            <i class="fas fa-plus mr-2"></i>
            {{ t('ragManagement.addDocument') }}
          </button>
          <button 
            @click="switchToQueryModal"
            class="btn btn-outline btn-sm"
          >
            <i class="fas fa-search mr-2"></i>
            {{ t('ragManagement.queryDocument') }}
          </button>
          <button @click="closeDetailsModal" class="btn btn-sm">
            {{ t('ragManagement.close') }}
          </button>
        </div>

        <!-- 文档列表 -->
        <div class="mb-2 flex items-center justify-between gap-2">
          <h4 class="font-semibold">{{ t('ragManagement.details.documentList') }}</h4>
          <div class="flex items-center gap-2">
            <input
              v-model="documentSearch"
              type="text"
              :placeholder="t('ragManagement.details.searchPlaceholder')"
              class="input input-bordered input-sm"
            >
            <select v-model.number="docPageSize" class="select select-bordered select-sm">
              <option :value="10">{{ t('ragManagement.details.perPage', { size: 10 }) }}</option>
              <option :value="20">{{ t('ragManagement.details.perPage', { size: 20 }) }}</option>
              <option :value="50">{{ t('ragManagement.details.perPage', { size: 50 }) }}</option>
            </select>
            <button class="btn btn-ghost btn-xs" @click="reloadDocuments" :disabled="loadingDocuments">
              <span v-if="loadingDocuments" class="loading loading-spinner loading-xs"></span>
              {{ t('ragManagement.refresh') }}
            </button>
          </div>
        </div>
        <div class="overflow-x-auto border rounded-lg">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>{{ t('ragManagement.table.fileName') }}</th>
                <th>{{ t('ragManagement.table.size') }}</th>
                <th>{{ t('ragManagement.table.createdAt') }}</th>
                <th>{{ t('ragManagement.table.operations') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="loadingDocuments">
                <td colspan="4">
                  <div class="flex items-center gap-2">
                    <span class="loading loading-spinner loading-sm"></span>
                    {{ t('ragManagement.details.loading') }}
                  </div>
                </td>
              </tr>
              <tr v-else-if="documents.length === 0">
                <td colspan="4" class="text-base-content/60">{{ t('ragManagement.details.noDocuments') }}</td>
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
                    <button class="btn btn-ghost btn-xs" :title="t('ragManagement.actions.preview')" @click="viewDocument(doc)">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button class="btn btn-ghost btn-xs text-error" :title="t('ragManagement.actions.delete')" @click="deleteDocument(doc)">
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
          <button class="btn btn-xs" :disabled="docCurrentPage <= 1" @click="docCurrentPage = docCurrentPage - 1">{{ t('ragManagement.details.prevPage') }}</button>
          <div class="text-sm">{{ t('ragManagement.details.pageInfo', { current: docCurrentPage, total: totalDocPages, count: filteredDocuments.length }) }}</div>
          <button class="btn btn-xs" :disabled="docCurrentPage >= totalDocPages" @click="docCurrentPage = docCurrentPage + 1">{{ t('ragManagement.details.nextPage') }}</button>
        </div>

        <!-- 加载状态 -->
        <div v-if="loadingDetails" class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      </div>
    </div>

    <!-- 文档预览模态框 -->
    <div v-if="showDocumentModal" class="modal modal-open" @click.self="showDocumentModal = false">
      <div class="modal-box max-w-5xl">
        <h3 class="font-bold text-lg mb-2">{{ t('ragManagement.document.previewTitle', { name: selectedDocument?.file_name }) }}</h3>
        <div class="text-xs text-base-content/60 mb-4 break-all">{{ selectedDocument?.file_path }}</div>

        <div class="mb-3 flex items-center justify-between">
          <div class="text-sm">{{ t('ragManagement.document.chunks', { count: documentChunks.length }) }}</div>
          <button class="btn btn-ghost btn-xs" @click="reloadDocumentChunks" :disabled="loadingChunks">
            <span v-if="loadingChunks" class="loading loading-spinner loading-xs"></span>
            {{ t('ragManagement.refresh') }}
          </button>
        </div>

        <div v-if="loadingChunks" class="flex items-center gap-2">
          <span class="loading loading-spinner loading-sm"></span>
          {{ t('ragManagement.document.loadingContent') }}
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
          <button class="btn" @click="showDocumentModal = false">{{ t('ragManagement.close') }}</button>
        </div>
      </div>
    </div>

    <!-- 查询模态框 -->
    <div v-if="showQueryModal" class="modal modal-open" @click.self="showQueryModal = false">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ t('ragManagement.query.title', { name: selectedCollection?.name }) }}</h3>
        
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">{{ t('ragManagement.query.contentLabel') }}</span>
          </label>
          <textarea 
            v-model="queryText"
            class="textarea textarea-bordered" 
            :placeholder="t('ragManagement.query.contentPlaceholder')"
            rows="3"
          ></textarea>
        </div>
        
        <div class="flex gap-4 mb-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.query.topKLabel') }}</span>
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
              <span class="label-text">{{ t('ragManagement.query.useEmbeddingLabel') }}</span>
            </label>
            <input type="checkbox" class="toggle" v-model="queryUseEmbedding" />
          </div>
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.query.rerankingLabel') }}</span>
            </label>
            <input type="checkbox" class="toggle" v-model="queryReranking" />
          </div>
          <div class="form-control flex-1">
            <label class="label">
              <span class="label-text">{{ t('ragManagement.query.thresholdLabel', '相似度阈值') }} ({{ queryThreshold }})</span>
            </label>
            <input type="range" min="0" max="1" step="0.05" v-model.number="queryThreshold" class="range range-xs range-primary" />
          </div>
        </div>
        
        <div class="flex gap-2 mb-4">
          <button 
            @click="executeQuery"
            class="btn btn-primary" 
            :disabled="querying || !queryText.trim()"
          >
            <span v-if="querying" class="loading loading-spinner loading-sm"></span>
            {{ querying ? t('ragManagement.querying') : t('ragManagement.query.execute') }}
          </button>
          <button @click="showQueryModal = false" class="btn">
            {{ t('ragManagement.close') }}
          </button>
        </div>
        
        <!-- 查询结果 -->
        <div v-if="queryResults.length > 0" class="mt-6">
          <h4 class="font-bold mb-4">{{ t('ragManagement.query.resultsTitle') }}</h4>
          <div class="space-y-4">
            <div 
              v-for="(result, index) in queryResults" 
              :key="index"
              class="card bg-base-200 shadow-sm"
            >
              <div class="card-body p-4">
                <div class="flex justify-between items-start mb-2">
                  <div class="badge badge-primary">{{ t('ragManagement.query.similarity', { score: (result.score * 100).toFixed(1) }) }}</div>
                  <div class="badge badge-outline">{{ t('ragManagement.query.rank', { rank: result.rank }) }}</div>
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
import { useI18n } from 'vue-i18n'
import Toast from '@/components/Toast.vue'

const { t } = useI18n()

// 响应式数据
const collections = ref([])
const searchQuery = ref('')
const statusFilter = ref('')
const showCreateCollectionModal = ref(false)
const showEditCollectionModal = ref(false)
const showIngestDocumentModal = ref(false)
const showQueryModal = ref(false)
const showCollectionDetailsModal = ref(false)
const creating = ref(false)
const updating = ref(false)
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
    showToast(checked ? t('ragManagement.messages.activateSuccess') : t('ragManagement.messages.deactivateSuccess'), 'info')
  } catch (e) {
    console.error('更新激活状态失败:', e)
    // revert UI
    collection.is_active = !checked
    showToast(t('ragManagement.messages.updateActiveFailed'), 'error')
  }
}

// 新集合表单
const newCollection = ref({
  name: '',
  description: ''
})

// 编辑集合表单
const editingCollection = ref({
  id: '',
  name: '',
  description: ''
})

// 文档摄取
const selectedCollection = ref(null)
const selectedFiles = ref([])
const failedFiles = ref<Array<{name: string, error: string}>>([])
const selectedFolder = ref('')
const fileInput = ref(null)
const batchProgress = ref({
  current: 0,
  total: 0,
  success: 0,
  failed: 0
})
const ingestMode = ref<'file' | 'manual'>('file')
const manualInput = ref({
  title: '',
  content: ''
})

// 支持的文件类型
const supportedFileTypes = ref<string[]>(['txt', 'md', 'pdf', 'docx'])

const loadSupportedFileTypes = async () => {
  try {
    const types = await invoke('rag_get_supported_file_types') as string[]
    if (types && types.length > 0) {
      supportedFileTypes.value = types
    }
  } catch (e) {
    console.warn('Failed to load supported file types', e)
  }
}

// 查询相关
const queryText = ref('')
const queryTopK = ref(5)
const queryResults = ref([])
const queryUseEmbedding = ref(true)
const queryReranking = ref(true)
const queryThreshold = ref(0.2)

// 加载全局 RAG 配置以同步默认值
const loadRagConfig = async () => {
  try {
    const config = await invoke('get_rag_config') as any
    if (config && typeof config.similarity_threshold === 'number') {
      queryThreshold.value = config.similarity_threshold
    }
  } catch (e) {
    console.warn('Failed to load RAG config:', e)
  }
}

// 集合详情相关
interface CollectionStats {
  totalDocuments: number
  totalChunks: number
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
    totalChunks: 0
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
    showToast(t('ragManagement.messages.loadFailed'), 'error')
  }
}

const createCollection = async () => {
  if (!newCollection.value.name) {
    showToast(t('ragManagement.messages.nameRequired'), 'warning')
    return
  }

  creating.value = true
  try {
    await invoke('create_rag_collection', {
      name: newCollection.value.name,
      description: newCollection.value.description || null
    })
    
    showToast(t('ragManagement.messages.createSuccess'), 'success')
    showCreateCollectionModal.value = false
    newCollection.value = { name: '', description: '' }
    await refreshCollections()
  } catch (error) {
    console.error('创建集合失败:', error)
    showToast(t('ragManagement.messages.createFailed', { error }), 'error')
  } finally {
    creating.value = false
  }
}

const editCollection = (collection: any) => {
  editingCollection.value = {
    id: collection.id,
    name: collection.name,
    description: collection.description || ''
  }
  showEditCollectionModal.value = true
}

const updateCollection = async () => {
  if (!editingCollection.value.name) {
    showToast(t('ragManagement.messages.nameRequired'), 'warning')
    return
  }

  updating.value = true
  try {
    await invoke('update_rag_collection', {
      collectionId: editingCollection.value.id,
      name: editingCollection.value.name,
      description: editingCollection.value.description || null
    })
    
    showToast(t('ragManagement.messages.updateSuccess'), 'success')
    showEditCollectionModal.value = false
    editingCollection.value = { id: '', name: '', description: '' }
    await refreshCollections()
  } catch (error) {
    console.error('更新集合失败:', error)
    showToast(t('ragManagement.messages.updateFailed', { error }), 'error')
  } finally {
    updating.value = false
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
        totalChunks: selectedCollection.value?.chunk_count || 0
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
    // 使用新的分页接口
    const response = await invoke('list_rag_documents_paginated', { 
      collectionId: selectedCollection.value.id,
      page: docCurrentPage.value,
      pageSize: docPageSize.value,
      searchQuery: documentSearch.value || null
    }) as any
    
    documents.value = response.documents || []
    // 更新总数用于分页计算
    if (response.total_count !== undefined) {
      // 存储总数以便分页使用
      collectionDetails.value.stats.totalDocuments = response.total_count
    }
  } catch (e) {
    console.error('获取文档列表失败:', e)
    showToast(t('ragManagement.messages.loadDocumentsFailed'), 'error')
  } finally {
    loadingDocuments.value = false
  }
}

// 服务端分页，不需要前端过滤和分页
const filteredDocuments = computed(() => {
  return documents.value || []
})

const totalDocPages = computed(() => {
  const total = collectionDetails.value.stats.totalDocuments || 0
  const size = Math.max(1, Number(docPageSize.value) || 10)
  return Math.max(1, Math.ceil(total / size))
})

const paginatedDocuments = computed(() => {
  // 服务端已经分页，直接返回
  return documents.value || []
})

// 监听搜索、分页大小、页码变化，重新加载数据
watch([documentSearch, docPageSize], () => {
  // 搜索或页大小变化时，重置到第一页
  docCurrentPage.value = 1
  reloadDocuments()
})

watch(docCurrentPage, () => {
  // 页码变化时重新加载
  reloadDocuments()
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
    showToast(t('ragManagement.messages.loadChunksFailed'), 'error')
  } finally {
    loadingChunks.value = false
  }
}

const deleteDocument = async (doc: any) => {
  try {
    await invoke('delete_rag_document', { documentId: doc.id })
    showToast(t('ragManagement.messages.deleteDocumentSuccess'), 'success')
    await reloadDocuments()
    await refreshCollections()
  } catch (e) {
    console.error('删除文档失败:', e)
    showToast(t('ragManagement.messages.deleteDocumentFailed', { error: e }), 'error')
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
    showToast(t('ragManagement.messages.fileSelectFailed', { error }), 'error')
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
          showToast(t('ragManagement.messages.noSupportedFiles'), 'warning')
        } else {
          showToast(t('ragManagement.messages.foundFiles', { count: files.length }), 'success')
        }
      } catch (error) {
        console.error('获取文件夹文件失败:', error)
        showToast(t('ragManagement.messages.folderFilesFailed', { error }), 'error')
      }
    }
  } catch (error) {
    console.error('文件夹选择失败:', error)
    showToast(t('ragManagement.messages.folderSelectFailed', { error }), 'error')
  }
}

const ingestDocument = async () => {
  if (!selectedCollection.value) {
    showToast(t('ragManagement.messages.selectCollection'), 'warning')
    return
  }

  // 根据模式验证输入
  if (ingestMode.value === 'file') {
    if (selectedFiles.value.length === 0) {
      showToast(t('ragManagement.messages.selectFile'), 'warning')
      return
    }
  } else {
    if (!manualInput.value.title.trim() || !manualInput.value.content.trim()) {
      showToast(t('ragManagement.messages.fillRequired'), 'warning')
      return
    }
  }

  ingesting.value = true
  failedFiles.value = [] // 重置失败列表
  
  try {
    if (ingestMode.value === 'manual') {
      // 手动输入模式
      const response = await invoke('rag_ingest_text', {
        title: manualInput.value.title.trim(),
        content: manualInput.value.content.trim(),
        collectionId: selectedCollection.value.id,
        metadata: {
          source: 'manual_input',
          createdAt: new Date().toISOString()
        }
      }) as any
      
      showToast(t('ragManagement.messages.ingestSuccess', { chunks: response.chunks_created || 0 }), 'success')
      closeIngestModal()
      await refreshCollections()
      return
    }
    
    // 文件选择模式 - 使用新的批量导入接口
    batchProgress.value = {
      current: 0,
      total: selectedFiles.value.length,
      success: 0,
      failed: 0
    }
    
    // 准备文件路径列表
    const filePaths = selectedFiles.value.map(file => file.path || file.name)
    
    // 调用批量导入接口
    const response = await invoke('rag_batch_ingest_sources', {
      filePaths,
      collectionId: selectedCollection.value.id
    }) as any
    
    // 更新失败文件列表
    if (response.failures && response.failures.length > 0) {
      failedFiles.value = response.failures.map((f: any) => ({
        name: f.file_path.split('/').pop() || f.file_path,
        error: f.error
      }))
    }
    
    // 显示最终结果
    if (response.failed === 0) {
      showToast(t('ragManagement.messages.ingestSuccess', { chunks: response.total_chunks }), 'success')
      closeIngestModal()
    } else if (response.success > 0) {
      showToast(t('ragManagement.messages.ingestPartialSuccess', { 
        success: response.success, 
        failed: response.failed, 
        chunks: response.total_chunks 
      }), 'warning')
    } else {
      showToast(t('ragManagement.messages.ingestAllFailed'), 'error')
    }
    
    await refreshCollections()
    
  } catch (error) {
    console.error('文档摄取失败:', error)
    const errorMsg = String(error)
    
    // 检查是否是维度不匹配错误
    if (errorMsg.includes('Embedding dimension mismatch') || errorMsg.includes('different schema')) {
      showToast(t('ragManagement.messages.embeddingDimensionMismatchDetail'), 'error')
    } else {
      showToast(t('ragManagement.messages.ingestFailed', { error }), 'error')
    }
  } finally {
    ingesting.value = false
    batchProgress.value = { current: 0, total: 0, success: 0, failed: 0 }
  }
}

const closeIngestModal = () => {
  showIngestDocumentModal.value = false
  selectedFiles.value = []
  selectedFolder.value = ''
  manualInput.value = { title: '', content: '' }
  ingestMode.value = 'file'
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

const queryCollection = async (collection: any) => {
  selectedCollection.value = collection
  queryText.value = ''
  queryResults.value = []
  await loadRagConfig()
  showQueryModal.value = true
}

const executeQuery = async () => {
  if (!queryText.value.trim() || !selectedCollection.value) {
    showToast(t('ragManagement.messages.queryRequired'), 'warning')
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
        reranking_enabled: queryReranking.value,
        similarity_threshold: queryThreshold.value
      }
    }) as any
    
    queryResults.value = response.results || []
    showToast(t('ragManagement.messages.querySuccess', { count: queryResults.value.length }), 'success')
  } catch (error) {
    console.error('查询失败:', error)
    showToast(t('ragManagement.messages.queryFailed', { error }), 'error')
  } finally {
    querying.value = false
  }
}

const deleteCollection = async (collection: any) => {

  try {
    await invoke('delete_rag_collection', { collectionId: collection.id })
    showToast(t('ragManagement.messages.deleteSuccess'), 'success')
    await refreshCollections()
  } catch (error) {
    console.error('删除集合失败:', error)
    showToast(t('ragManagement.messages.deleteFailed', { error }), 'error')
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
onMounted(async () => {
  refreshCollections()
  loadSupportedFileTypes()
  
  // 监听批量导入进度事件
  const { listen } = await import('@tauri-apps/api/event')
  
  listen('rag_batch_ingest_progress', (event: any) => {
    const progress = event.payload
    if (progress) {
      batchProgress.value = {
        current: progress.current || 0,
        total: progress.total || 0,
        success: progress.success || 0,
        failed: progress.failed || 0
      }
      
      // 如果有当前文件信息，可以显示在UI上
      if (progress.current_file) {
        console.log(`正在处理: ${progress.current_file}`)
      }
    }
  })
  
  // 监听RAG配置更新事件
  listen('rag_config_updated', async (event: any) => {
    console.log('RAG配置已更新:', event.payload)
    // 重新加载全局配置，更新查询参数
    await loadRagConfig()
    showToast(t('ragManagement.messages.configUpdated', 'Configuration updated'), 'info')
  })
  
  // 监听RAG服务重载事件
  listen('rag_service_reloaded', () => {
    console.log('RAG服务已重载')
    showToast(t('ragManagement.messages.serviceReloaded', 'Service reloaded successfully'), 'success')
  })
  
  listen('rag_service_reload_failed', (event: any) => {
    console.error('RAG服务重载失败:', event.payload)
    showToast(t('ragManagement.messages.serviceReloadFailed', 'Service reload failed') + ': ' + event.payload, 'error')
  })
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
  font-size: calc(var(--font-size-base, 14px) * 0.75);
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