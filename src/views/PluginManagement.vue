<template>
  <div class="container mx-auto p-6">
    <div class="mb-6">
      <h1 class="text-3xl font-bold">{{ $t('plugins.title', '插件管理') }}</h1>
      <p class="text-base-content/70 mt-2">{{ $t('plugins.description', '管理和配置安全测试插件') }}</p>
    </div>
    
    <!-- Operation Bar -->
    <div class="flex gap-2 mb-6 flex-wrap">
      <button class="btn btn-primary" @click="openCreateDialog">
        <i class="fas fa-plus mr-2"></i>
        {{ $t('plugins.newPlugin', '新增插件') }}
      </button>
      <button class="btn btn-secondary" @click="openUploadDialog">
        <i class="fas fa-upload mr-2"></i>
        {{ $t('plugins.uploadPlugin', '上传插件') }}
      </button>
      <button class="btn btn-accent" @click="openAIGenerateDialog">
        <i class="fas fa-magic mr-2"></i>
        {{ $t('plugins.aiGenerate', 'AI生成插件') }}
      </button>
      <button class="btn btn-info" @click="refreshPlugins">
        <i class="fas fa-sync-alt mr-2"></i>
        {{ $t('common.refresh', '刷新列表') }}
      </button>
      <!-- <button class="btn btn-outline" @click="scanPluginDirectory">
        <i class="fas fa-folder-open mr-2"></i>
        {{ $t('plugins.scanDirectory', '扫描目录') }}
      </button> -->
    </div>
    
    <!-- Category Filter -->
    <div class="tabs tabs-boxed mb-6 flex-wrap gap-2">
      <button 
        v-for="cat in categories" 
        :key="cat.value"
        class="tab"
        :class="{ 'tab-active': selectedCategory === cat.value }"
        @click="selectedCategory = cat.value"
      >
        <i :class="cat.icon" class="mr-2"></i>
        {{ cat.label }}
        <span v-if="getCategoryCount(cat.value) > 0" class="ml-2 badge badge-sm">
          {{ getCategoryCount(cat.value) }}
        </span>
      </button>
      
      <!-- 插件审核 Tab -->
      <button 
        class="tab"
        :class="{ 'tab-active': selectedCategory === 'review' }"
        @click="selectedCategory = 'review'"
      >
        <i class="fas fa-check-double mr-2"></i>
        {{ $t('plugins.pluginReview', '插件审核') }}
        <span v-if="reviewStats.pending > 0" class="ml-2 badge badge-sm badge-warning">
          {{ reviewStats.pending }}
        </span>
      </button>
    </div>
    
    <!-- Plugin Manager Content (Merged) -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        
        <!-- Plugin Review Section -->
        <div v-if="selectedCategory === 'review'">
          <!-- Stats Cards -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
            <div class="stats shadow">
              <div class="stat">
                <div class="stat-figure text-warning">
                  <i class="fas fa-clock text-3xl"></i>
                </div>
                <div class="stat-title">{{ $t('plugins.review.pending', '待审核') }}</div>
                <div class="stat-value text-warning">{{ reviewStats.pending }}</div>
              </div>
            </div>
            <div class="stats shadow">
              <div class="stat">
                <div class="stat-figure text-success">
                  <i class="fas fa-check-circle text-3xl"></i>
                </div>
                <div class="stat-title">{{ $t('plugins.review.approved', '已批准') }}</div>
                <div class="stat-value text-success">{{ reviewStats.approved }}</div>
              </div>
            </div>
            <div class="stats shadow">
              <div class="stat">
                <div class="stat-figure text-error">
                  <i class="fas fa-times-circle text-3xl"></i>
                </div>
                <div class="stat-title">{{ $t('plugins.review.rejected', '已拒绝') }}</div>
                <div class="stat-value text-error">{{ reviewStats.rejected }}</div>
              </div>
            </div>
            <div class="stats shadow">
              <div class="stat">
                <div class="stat-figure text-base-content opacity-50">
                  <i class="fas fa-exclamation-triangle text-3xl"></i>
                </div>
                <div class="stat-title">{{ $t('plugins.review.failed', '验证失败') }}</div>
                <div class="stat-value">{{ reviewStats.failed }}</div>
              </div>
            </div>
          </div>
          
          <!-- Status Filter Buttons -->
          <div class="flex gap-2 mb-4 flex-wrap">
            <button 
              class="btn btn-sm"
              :class="reviewStatusFilter === 'all' ? 'btn-primary' : 'btn-ghost'"
              @click="changeReviewStatusFilter('all')"
            >
              <i class="fas fa-list mr-1"></i>
              {{ $t('plugins.allStatus', '全部') }} ({{ reviewStats.total }})
            </button>
            <button 
              class="btn btn-sm"
              :class="reviewStatusFilter === 'PendingReview' ? 'btn-warning' : 'btn-ghost'"
              @click="changeReviewStatusFilter('PendingReview')"
            >
              <i class="fas fa-clock mr-1"></i>
              {{ $t('plugins.review.pending', '待审核') }} ({{ reviewStats.pending }})
            </button>
            <button 
              class="btn btn-sm"
              :class="reviewStatusFilter === 'Approved' ? 'btn-success' : 'btn-ghost'"
              @click="changeReviewStatusFilter('Approved')"
            >
              <i class="fas fa-check-circle mr-1"></i>
              {{ $t('plugins.review.approved', '已批准') }} ({{ reviewStats.approved }})
            </button>
            <button 
              class="btn btn-sm"
              :class="reviewStatusFilter === 'Rejected' ? 'btn-error' : 'btn-ghost'"
              @click="changeReviewStatusFilter('Rejected')"
            >
              <i class="fas fa-times-circle mr-1"></i>
              {{ $t('plugins.review.rejected', '已拒绝') }} ({{ reviewStats.rejected }})
            </button>
            <button 
              class="btn btn-sm"
              :class="reviewStatusFilter === 'ValidationFailed' ? 'btn-ghost' : 'btn-ghost'"
              @click="changeReviewStatusFilter('ValidationFailed')"
            >
              <i class="fas fa-exclamation-triangle mr-1"></i>
              {{ $t('plugins.review.failed', '验证失败') }} ({{ reviewStats.failed }})
            </button>
          </div>
          
          <!-- Review Actions -->
          <div class="flex gap-2 mb-4 flex-wrap">
            <input 
              v-model="reviewSearchText" 
              type="text" 
              :placeholder="$t('plugins.searchPlugins', '搜索插件...')" 
              class="input input-bordered flex-1"
            />
            <button class="btn btn-success" @click="approveSelected" :disabled="selectedReviewPlugins.length === 0">
              <i class="fas fa-check mr-2"></i>
              {{ $t('plugins.batchApprove', '批量批准') }} ({{ selectedReviewPlugins.length }})
            </button>
            <button class="btn btn-error" @click="rejectSelected" :disabled="selectedReviewPlugins.length === 0">
              <i class="fas fa-times mr-2"></i>
              {{ $t('plugins.batchReject', '批量拒绝') }} ({{ selectedReviewPlugins.length }})
            </button>
          </div>
          
          <!-- Review Plugins Table -->
          <div v-if="filteredReviewPlugins.length === 0" class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>{{ $t('plugins.noReviewPlugins', '暂无待审核的插件') }}</span>
          </div>
          
          <div v-else>
            <!-- Pagination Info -->
            <div class="flex justify-between items-center mb-4">
              <div class="text-sm text-base-content/70">
                {{ $t('plugins.showing', '显示') }} {{ reviewPaginationInfo.start }}-{{ reviewPaginationInfo.end }} 
                {{ $t('plugins.of', '共') }} {{ reviewPaginationInfo.total }} {{ $t('plugins.items', '条') }}
              </div>
              <div class="flex items-center gap-2">
                <span class="text-sm">{{ $t('plugins.pageSize', '每页') }}:</span>
                <select 
                  v-model.number="reviewPageSize" 
                  @change="changeReviewPageSize(reviewPageSize)"
                  class="select select-bordered select-sm"
                >
                  <option :value="5">5</option>
                  <option :value="10">10</option>
                  <option :value="20">20</option>
                  <option :value="50">50</option>
                </select>
              </div>
            </div>
            
            <div class="overflow-x-auto">
              <table class="table table-zebra w-full">
                <thead>
                  <tr>
                    <th class="w-12">
                      <input 
                        type="checkbox" 
                        class="checkbox checkbox-sm" 
                        @change="toggleSelectAll"
                        :checked="isAllSelected"
                      />
                    </th>
                    <th>{{ $t('plugins.pluginName', '插件名称') }}</th>
                    <th class="w-32">{{ $t('plugins.qualityScore', '质量评分') }}</th>
                    <th class="w-24">{{ $t('common.status', '状态') }}</th>
                    <th class="w-24">{{ $t('plugins.model', '模型') }}</th>
                    <th class="w-40">{{ $t('plugins.generatedAt', '生成时间') }}</th>
                    <th class="w-48">{{ $t('common.actions', '操作') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="plugin in paginatedReviewPlugins" :key="plugin.plugin_id" class="hover">
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm" 
                      :checked="isPluginSelected(plugin)"
                      @change="togglePluginSelection(plugin)"
                    />
                  </td>
                  <td>
                    <div>
                      <div class="flex items-center gap-2">
                        <span 
                          class="badge badge-sm"
                          :class="{
                            // Injection / command execution
                            'badge-error': plugin.vuln_type === 'sqli' || plugin.vuln_type === 'command_injection',
                            // XSS
                            'badge-warning': plugin.vuln_type === 'xss',
                            // IDOR / auth bypass
                            'badge-info': plugin.vuln_type === 'idor' || plugin.vuln_type === 'auth_bypass',
                            // CSRF
                            'badge-primary': plugin.vuln_type === 'csrf',
                            // Information disclosure
                            'badge-success': plugin.vuln_type === 'info_leak',
                            // File upload / inclusion
                            'badge-secondary': plugin.vuln_type === 'file_upload' || plugin.vuln_type === 'file_inclusion',
                            // Path traversal
                            'badge-accent': plugin.vuln_type === 'path_traversal',
                            // XXE / SSRF / other server-side issues
                            'badge-neutral': plugin.vuln_type === 'xxe' || plugin.vuln_type === 'ssrf'
                          }"
                        >
                          {{ plugin.vuln_type.toUpperCase() }}
                        </span>
                        <span class="font-bold">{{ plugin.plugin_name }}</span>
                      </div>
                      <div class="text-xs text-gray-500 mt-1">{{ plugin.plugin_id }}</div>
                    </div>
                  </td>
                  <td>
                    <div class="flex items-center gap-2">
                      <progress 
                        class="progress w-20" 
                        :class="{
                          'progress-success': plugin.quality_score >= 80,
                          'progress-warning': plugin.quality_score >= 60 && plugin.quality_score < 80,
                          'progress-error': plugin.quality_score < 60
                        }"
                        :value="plugin.quality_score" 
                        max="100"
                      ></progress>
                      <span class="text-sm font-semibold">{{ plugin.quality_score }}%</span>
                    </div>
                  </td>
                  <td>
                    <span 
                      class="badge badge-sm"
                      :class="{
                        'badge-warning': plugin.status === 'PendingReview',
                        'badge-success': plugin.status === 'Approved',
                        'badge-error': plugin.status === 'Rejected',
                        'badge-ghost': plugin.status === 'ValidationFailed'
                      }"
                    >
                      {{ getReviewStatusText(plugin.status) }}
                    </span>
                  </td>
                  <td>
                    <span class="text-xs">{{ plugin.model }}</span>
                  </td>
                  <td>
                    <span class="text-xs">{{ formatDate(plugin.generated_at) }}</span>
                  </td>
                  <td>
                    <div class="flex gap-1">
                      <button 
                        class="btn btn-sm btn-info"
                        @click="viewReviewPluginDetail(plugin)"
                      >
                        <i class="fas fa-eye"></i>
                      </button>
                      <button 
                        class="btn btn-sm btn-success"
                        @click="approvePlugin(plugin)"
                        :disabled="plugin.status === 'Approved'"
                      >
                        <i class="fas fa-check"></i>
                      </button>
                      <button 
                        class="btn btn-sm btn-error"
                        @click="rejectPlugin(plugin)"
                        :disabled="plugin.status === 'Rejected'"
                      >
                        <i class="fas fa-times"></i>
                      </button>
                      <button 
                        class="btn btn-sm btn-ghost"
                        @click="deleteReviewPlugin(plugin)"
                      >
                        <i class="fas fa-trash"></i>
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          
          <!-- Pagination Controls -->
          <div class="flex justify-center items-center gap-2 mt-4">
            <button 
              class="btn btn-sm"
              :disabled="reviewCurrentPage === 1"
              @click="goToReviewPage(1)"
            >
              <i class="fas fa-angle-double-left"></i>
            </button>
            <button 
              class="btn btn-sm"
              :disabled="reviewCurrentPage === 1"
              @click="goToReviewPage(reviewCurrentPage - 1)"
            >
              <i class="fas fa-angle-left"></i>
            </button>
            
            <template v-for="page in reviewTotalPages" :key="page">
              <button 
                v-if="Math.abs(page - reviewCurrentPage) <= 2 || page === 1 || page === reviewTotalPages"
                class="btn btn-sm"
                :class="{ 'btn-primary': page === reviewCurrentPage }"
                @click="goToReviewPage(page)"
              >
                {{ page }}
              </button>
              <span 
                v-else-if="(page === reviewCurrentPage - 3 || page === reviewCurrentPage + 3) && reviewTotalPages > 7"
                class="px-2"
              >
                ...
              </span>
            </template>
            
            <button 
              class="btn btn-sm"
              :disabled="reviewCurrentPage === reviewTotalPages"
              @click="goToReviewPage(reviewCurrentPage + 1)"
            >
              <i class="fas fa-angle-right"></i>
            </button>
            <button 
              class="btn btn-sm"
              :disabled="reviewCurrentPage === reviewTotalPages"
              @click="goToReviewPage(reviewTotalPages)"
            >
              <i class="fas fa-angle-double-right"></i>
            </button>
          </div>
        </div>
        </div>
        
        <!-- Regular Plugin List -->
        <div v-else>
          <!-- View Mode Toggle for Passive Scan Plugins -->
          <div v-if="selectedCategory === 'passive'" class="flex gap-2 mb-4">
            <button 
              class="btn btn-sm"
              :class="pluginViewMode === 'favorited' ? 'btn-primary' : 'btn-ghost'"
              @click="pluginViewMode = 'favorited'"
            >
              <i class="fas fa-star mr-1"></i>
              {{ $t('plugins.favorited', '已收藏') }}
            </button>
            <button 
              class="btn btn-sm"
              :class="pluginViewMode === 'all' ? 'btn-primary' : 'btn-ghost'"
              @click="pluginViewMode = 'all'"
            >
              <i class="fas fa-list mr-1"></i>
              {{ $t('plugins.allPlugins', '全部插件') }}
            </button>
          </div>
          
          <!-- Plugin List -->
          <div v-if="filteredPlugins.length === 0" class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>{{ $t('plugins.noPlugins', '暂无插件，请上传或扫描插件目录') }}</span>
          </div>
        
        <div v-else class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th class="w-12">{{ $t('common.status', '状态') }}</th>
                <th>{{ $t('plugins.pluginName', '插件名称') }}</th>
                <th class="w-24">{{ $t('plugins.version', '版本') }}</th>
                <th class="w-16 text-center">{{ $t('plugins.category', '分类') }}</th>
                <th class="w-32">{{ $t('plugins.author', '作者') }}</th>
                <th class="w-48">{{ $t('plugins.tags', '标签') }}</th>
                <th class="w-64">{{ $t('common.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="plugin in filteredPlugins" :key="plugin.metadata.id">
                <!-- Status Indicator -->
                <td>
                  <div class="flex items-center gap-2">
                    <div 
                      class="tooltip" 
                      :data-tip="getStatusText(plugin.status)"
                    >
                      <div
                        class="w-3 h-3 rounded-full"
                        :class="{
                          'bg-success': plugin.status === 'Enabled',
                          'bg-warning': plugin.status === 'Disabled',
                          'bg-error': plugin.status === 'Error'
                        }"
                      ></div>
                    </div>
                  </div>
                </td>
                
                <!-- Plugin Name -->
                <td>
                  <div class="font-bold">{{ plugin.metadata.name }}</div>
                  <div class="text-sm text-gray-500">{{ plugin.metadata.id }}</div>
                  <div v-if="plugin.metadata.description" class="text-xs text-gray-400 mt-1">
                    {{ plugin.metadata.description }}
                  </div>
                </td>
                
                <!-- Version -->
                <td>
                  <span class="badge badge-outline">{{ plugin.metadata.version }}</span>
                </td>
                
                <!-- Category -->
                <td class="text-center">
                  <div class="tooltip" :data-tip="getCategoryLabel(plugin.metadata.category)">
                    <i 
                      :class="getCategoryIcon(plugin.metadata.category)"
                      class="text-primary text-lg"
                    ></i>
                  </div>
                </td>
                
                <!-- Author -->
                <td>{{ plugin.metadata.author || '-' }}</td>
                
                <!-- Tags -->
                <td>
                  <div class="flex flex-wrap gap-1 max-w-xs">
                    <span 
                      v-for="(tag, idx) in plugin.metadata.tags.slice(0, 3)" 
                      :key="tag" 
                      class="badge badge-sm badge-ghost whitespace-nowrap"
                    >
                      {{ tag }}
                    </span>
                    <span 
                      v-if="plugin.metadata.tags.length > 3"
                      class="badge badge-sm badge-outline tooltip"
                      :data-tip="plugin.metadata.tags.slice(3).join(', ')"
                    >
                      +{{ plugin.metadata.tags.length - 3 }}
                    </span>
                  </div>
                </td>
                
                <!-- Action Buttons -->
                <td>
                  <div class="flex gap-1 flex-wrap">
                    <!-- Favorite Button (仅被动扫描插件显示) -->
                    <div 
                      v-if="isPassiveScanPluginType(plugin)"
                      class="tooltip" 
                      :data-tip="isPluginFavorited(plugin) ? '取消收藏' : '收藏插件'"
                    >
                      <button
                        class="btn btn-sm btn-ghost"
                        @click="togglePluginFavorite(plugin)"
                      >
                        <i 
                          :class="isPluginFavorited(plugin) ? 'fas fa-star text-yellow-500' : 'far fa-star'"
                        ></i>
                      </button>
                    </div>
                    
                    <!-- Test Plugin - 显示不同的提示信息 -->
                    <div class="tooltip" :data-tip="isAgentPluginType(plugin) ? '测试 Agent 工具 (analyze)' : '测试被动扫描 (scan_request/scan_response)'">
                      <button
                        class="btn btn-sm btn-outline"
                        @click="testPlugin(plugin)"
                        :disabled="plugin.status !== 'Enabled'"
                      >
                        <i class="fas fa-vial mr-1"></i>
                        {{ $t('plugins.test', '测试') }}
                      </button>
                    </div>

                    <!-- Advanced Test - 仅对被动扫描插件显示 -->
                    <div 
                      v-if="isPassiveScanPluginType(plugin)"
                      class="tooltip" 
                      data-tip="高级并发测试 (仅被动扫描)"
                    >
                      <button
                        class="btn btn-sm btn-outline"
                        @click="openAdvancedDialog(plugin)"
                        :disabled="plugin.status !== 'Enabled'"
                      >
                        <i class="fas fa-gauge-high mr-1"></i>
                        {{ $t('plugins.advancedTest', '高级') }}
                      </button>
                    </div>
                    
                    <!-- Enable/Disable Toggle -->
                    <button
                      class="btn btn-sm"
                      :class="plugin.status === 'Enabled' ? 'btn-warning' : 'btn-success'"
                      @click="togglePlugin(plugin)"
                    >
                      <i 
                        :class="plugin.status === 'Enabled' ? 'fas fa-pause' : 'fas fa-play'"
                        class="mr-1"
                      ></i>
                      {{ plugin.status === 'Enabled' ? $t('plugins.disable', '禁用') : $t('plugins.enable', '启用') }}
                    </button>
                    
                    <!-- View/Edit Code -->
                    <button 
                      class="btn btn-sm btn-info"
                      @click="viewPluginCode(plugin)"
                    >
                      <i class="fas fa-code mr-1"></i>
                      {{ $t('plugins.code', '代码') }}
                    </button>
                    
                    <!-- Delete -->
                    <button 
                      class="btn btn-sm btn-error"
                      @click="confirmDeletePlugin(plugin)"
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
    </div>
    
    <!-- Review Plugin Detail Dialog -->
    <dialog ref="reviewDetailDialog" class="modal">
      <div class="modal-box w-11/12 max-w-6xl max-h-[90vh] overflow-y-auto">
        <!-- Fixed Header -->
        <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
          <h3 class="font-bold text-lg">
          <i class="fas fa-eye mr-2"></i>
          {{ $t('plugins.pluginDetail', '插件详情') }}
        </h3>
          <button @click="closeReviewDetailDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
        </div>
        
        <div v-if="selectedReviewPlugin" class="space-y-4">
          <!-- Basic Info -->
          <div class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-semibold mb-3">{{ $t('plugins.basicInfo', '基本信息') }}</h4>
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span class="text-gray-500">{{ $t('plugins.pluginId', '插件ID') }}:</span>
                  <span class="ml-2 font-mono">{{ selectedReviewPlugin.plugin_id }}</span>
                </div>
                <div>
                  <span class="text-gray-500">{{ $t('plugins.pluginName', '插件名称') }}:</span>
                  <span class="ml-2">{{ selectedReviewPlugin.plugin_name }}</span>
                </div>
                <div>
                  <span class="text-gray-500">{{ $t('plugins.vulnType', '漏洞类型') }}:</span>
                  <span 
                    class="ml-2 badge badge-sm"
                    :class="{
                      'badge-error': selectedReviewPlugin.vuln_type === 'sqli' || selectedReviewPlugin.vuln_type === 'command_injection',
                      'badge-warning': selectedReviewPlugin.vuln_type === 'xss',
                      'badge-info': selectedReviewPlugin.vuln_type === 'idor' || selectedReviewPlugin.vuln_type === 'auth_bypass',
                      'badge-primary': selectedReviewPlugin.vuln_type === 'csrf',
                      'badge-success': selectedReviewPlugin.vuln_type === 'info_leak',
                      'badge-secondary': selectedReviewPlugin.vuln_type === 'file_upload' || selectedReviewPlugin.vuln_type === 'file_inclusion',
                      'badge-accent': selectedReviewPlugin.vuln_type === 'path_traversal',
                      'badge-neutral': selectedReviewPlugin.vuln_type === 'xxe' || selectedReviewPlugin.vuln_type === 'ssrf'
                    }"
                  >
                    {{ selectedReviewPlugin.vuln_type.toUpperCase() }}
                  </span>
                </div>
                <div>
                  <span class="text-gray-500">{{ $t('plugins.model', '生成模型') }}:</span>
                  <span class="ml-2">{{ selectedReviewPlugin.model }}</span>
                </div>
                <div class="col-span-2">
                  <span class="text-gray-500">{{ $t('plugins.qualityScore', '质量评分') }}:</span>
                  <div class="flex items-center gap-2 mt-1">
                    <progress 
                      class="progress w-full" 
                      :class="{
                        'progress-success': selectedReviewPlugin.quality_score >= 80,
                        'progress-warning': selectedReviewPlugin.quality_score >= 60 && selectedReviewPlugin.quality_score < 80,
                        'progress-error': selectedReviewPlugin.quality_score < 60
                      }"
                      :value="selectedReviewPlugin.quality_score" 
                      max="100"
                    ></progress>
                    <span class="font-semibold">{{ selectedReviewPlugin.quality_score }}%</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Quality Breakdown -->
          <div v-if="selectedReviewPlugin.quality_breakdown" class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-semibold mb-3">{{ $t('plugins.qualityBreakdown', '质量评分细分') }}</h4>
              <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
                <div class="text-center">
                  <div class="text-xs text-gray-500 mb-2">{{ $t('plugins.syntaxScore', '语法正确性') }}</div>
                  <div 
                    class="radial-progress" 
                    :style="`--value:${selectedReviewPlugin.quality_breakdown.syntax_score}; --size:4rem;`"
                    :class="{
                      'text-success': selectedReviewPlugin.quality_breakdown.syntax_score >= 80,
                      'text-warning': selectedReviewPlugin.quality_breakdown.syntax_score >= 60 && selectedReviewPlugin.quality_breakdown.syntax_score < 80,
                      'text-error': selectedReviewPlugin.quality_breakdown.syntax_score < 60
                    }"
                  >
                    {{ selectedReviewPlugin.quality_breakdown.syntax_score }}%
                  </div>
                </div>
                <div class="text-center">
                  <div class="text-xs text-gray-500 mb-2">{{ $t('plugins.logicScore', '逻辑完整性') }}</div>
                  <div 
                    class="radial-progress" 
                    :style="`--value:${selectedReviewPlugin.quality_breakdown.logic_score}; --size:4rem;`"
                    :class="{
                      'text-success': selectedReviewPlugin.quality_breakdown.logic_score >= 80,
                      'text-warning': selectedReviewPlugin.quality_breakdown.logic_score >= 60 && selectedReviewPlugin.quality_breakdown.logic_score < 80,
                      'text-error': selectedReviewPlugin.quality_breakdown.logic_score < 60
                    }"
                  >
                    {{ selectedReviewPlugin.quality_breakdown.logic_score }}%
                  </div>
                </div>
                <div class="text-center">
                  <div class="text-xs text-gray-500 mb-2">{{ $t('plugins.securityScore', '安全性') }}</div>
                  <div 
                    class="radial-progress" 
                    :style="`--value:${selectedReviewPlugin.quality_breakdown.security_score}; --size:4rem;`"
                    :class="{
                      'text-success': selectedReviewPlugin.quality_breakdown.security_score >= 80,
                      'text-warning': selectedReviewPlugin.quality_breakdown.security_score >= 60 && selectedReviewPlugin.quality_breakdown.security_score < 80,
                      'text-error': selectedReviewPlugin.quality_breakdown.security_score < 60
                    }"
                  >
                    {{ selectedReviewPlugin.quality_breakdown.security_score }}%
                  </div>
                </div>
                <div class="text-center">
                  <div class="text-xs text-gray-500 mb-2">{{ $t('plugins.codeQuality', '代码质量') }}</div>
                  <div 
                    class="radial-progress" 
                    :style="`--value:${selectedReviewPlugin.quality_breakdown.code_quality_score}; --size:4rem;`"
                    :class="{
                      'text-success': selectedReviewPlugin.quality_breakdown.code_quality_score >= 80,
                      'text-warning': selectedReviewPlugin.quality_breakdown.code_quality_score >= 60 && selectedReviewPlugin.quality_breakdown.code_quality_score < 80,
                      'text-error': selectedReviewPlugin.quality_breakdown.code_quality_score < 60
                    }"
                  >
                    {{ selectedReviewPlugin.quality_breakdown.code_quality_score }}%
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Validation Result -->
          <div v-if="selectedReviewPlugin.validation" class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-semibold mb-3">{{ $t('plugins.validationResult', '验证结果') }}</h4>
              <div 
                class="alert" 
                :class="selectedReviewPlugin.validation.is_valid ? 'alert-success' : 'alert-error'"
              >
                <i :class="selectedReviewPlugin.validation.is_valid ? 'fas fa-check-circle' : 'fas fa-exclamation-circle'"></i>
                <div>
                  <div class="font-semibold">
                    {{ selectedReviewPlugin.validation.is_valid ? $t('plugins.validationPassed', '验证通过') : $t('plugins.validationFailed', '验证失败') }}
                  </div>
                  <div v-if="selectedReviewPlugin.validation.errors.length > 0" class="mt-2">
                    <strong>{{ $t('plugins.errors', '错误') }}:</strong>
                    <ul class="list-disc list-inside mt-1">
                      <li v-for="(error, index) in selectedReviewPlugin.validation.errors" :key="index" class="text-sm">
                        {{ error }}
                      </li>
                    </ul>
                  </div>
                  <div v-if="selectedReviewPlugin.validation.warnings.length > 0" class="mt-2">
                    <strong>{{ $t('plugins.warnings', '警告') }}:</strong>
                    <ul class="list-disc list-inside mt-1">
                      <li v-for="(warning, index) in selectedReviewPlugin.validation.warnings" :key="index" class="text-sm">
                        {{ warning }}
                      </li>
                    </ul>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Code Editor -->
          <div class="card bg-base-200">
            <div class="card-body p-4">
              <div class="flex justify-between items-center mb-3">
                <h4 class="font-semibold">{{ $t('plugins.pluginCode', '插件代码') }}</h4>
                <div class="flex gap-2">
                  <button class="btn btn-sm btn-outline" @click="copyReviewCode">
                    <i class="fas fa-copy mr-1"></i>
                    {{ $t('plugins.copy', '复制') }}
                  </button>
                  <button class="btn btn-sm btn-outline" @click="reviewEditMode = !reviewEditMode">
                    <i class="fas fa-edit mr-1"></i>
                    {{ reviewEditMode ? $t('plugins.readonly', '只读') : $t('common.edit', '编辑') }}
                  </button>
                </div>
              </div>
              <div ref="reviewCodeEditorContainer" class="border border-base-300 rounded-lg overflow-hidden min-h-96"></div>
            </div>
          </div>
        </div>
        
        <!-- Fixed Footer -->
        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button class="btn btn-sm" @click="closeReviewDetailDialog">{{ $t('common.close', '关闭') }}</button>
          <button 
            v-if="reviewEditMode" 
            class="btn btn-primary btn-sm" 
            @click="saveReviewEdit"
            :disabled="savingReview"
          >
            <span v-if="savingReview" class="loading loading-spinner"></span>
            {{ savingReview ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
          </button>
          <button 
            class="btn btn-success btn-sm"
            @click="approvePlugin(selectedReviewPlugin)"
            :disabled="selectedReviewPlugin?.status === 'Approved'"
          >
            <i class="fas fa-check mr-1"></i>
            {{ $t('plugins.approve', '批准') }}
          </button>
          <button 
            class="btn btn-error btn-sm"
            @click="rejectPlugin(selectedReviewPlugin)"
            :disabled="selectedReviewPlugin?.status === 'Rejected'"
          >
            <i class="fas fa-times mr-1"></i>
            {{ $t('plugins.reject', '拒绝') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeReviewDetailDialog">close</button>
      </form>
    </dialog>
    
    <!-- Upload Plugin Dialog -->
    <dialog ref="uploadDialog" class="modal">
      <div class="modal-box max-h-[90vh] overflow-y-auto">
        <!-- Fixed Header -->
        <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
          <h3 class="font-bold text-lg">{{ $t('plugins.uploadPlugin', '上传插件') }}</h3>
          <button @click="closeUploadDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
        </div>
        
        <div class="form-control w-full">
          <label class="label">
            <span class="label-text">{{ $t('plugins.selectFile', '选择插件文件 (.ts / .js)') }}</span>
          </label>
          <input 
            type="file" 
            class="file-input file-input-bordered w-full"
            accept=".ts,.js"
            @change="handleFileSelect"
            ref="fileInput"
          />
        </div>
        
        <div v-if="uploadError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ uploadError }}</span>
        </div>
        
        <!-- Fixed Footer -->
        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button class="btn btn-sm" @click="closeUploadDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-primary btn-sm" 
            :disabled="!selectedFile || uploading"
            @click="uploadPlugin"
          >
            <span v-if="uploading" class="loading loading-spinner"></span>
            {{ uploading ? $t('plugins.uploading', '上传中...') : $t('plugins.upload', '上传') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeUploadDialog">close</button>
      </form>
    </dialog>
    
    <!-- Code Editor Dialog -->
    <dialog ref="codeEditorDialog" class="modal">
      <div class="modal-box w-11/12 max-w-5xl max-h-[90vh] overflow-y-auto">
        <!-- Fixed Header -->
        <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
          <h3 class="font-bold text-lg">
          {{ editingPlugin ? $t('plugins.codeEditor', '插件代码编辑器') : $t('plugins.newPlugin', '新增插件') }}
          <span v-if="editingPlugin" class="text-sm font-normal text-gray-500 ml-2">
            {{ editingPlugin.metadata.name }} ({{ editingPlugin.metadata.id }})
          </span>
        </h3>
          <button @click="closeCodeEditorDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
        </div>
        
        <!-- Plugin Metadata Form (for both new and editing) -->
        <div class="grid grid-cols-2 gap-4 mb-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginId', '插件ID') }} <span class="text-error">*</span></span>
            </label>
            <input 
              v-model="newPluginMetadata.id"
              type="text" 
              :placeholder="$t('plugins.pluginIdPlaceholder', '例如: sql_injection_scanner')"
              class="input input-bordered input-sm"
              :disabled="!!editingPlugin"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginName', '插件名称') }} <span class="text-error">*</span></span>
            </label>
            <input 
              v-model="newPluginMetadata.name"
              type="text" 
              :placeholder="$t('plugins.pluginNamePlaceholder', '例如: SQL注入扫描器')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.version', '版本') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.version"
              type="text" 
              placeholder="1.0.0"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.author', '作者') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.author"
              type="text" 
              :placeholder="$t('plugins.authorPlaceholder', '作者名称')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.mainCategory', '主分类') }} <span class="text-error">*</span></span>
            </label>
            <select 
              v-model="newPluginMetadata.mainCategory" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option 
                v-for="cat in mainCategories" 
                :key="cat.value" 
                :value="cat.value"
              >
                {{ cat.label }}
              </option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.subCategory', '子分类') }} <span class="text-error">*</span></span>
            </label>
            <select 
              v-model="newPluginMetadata.category" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option 
                v-for="cat in subCategories" 
                :key="cat.value" 
                :value="cat.value"
              >
                {{ cat.label }}
              </option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.defaultSeverity', '默认严重程度') }}</span>
            </label>
            <select 
              v-model="newPluginMetadata.default_severity" 
              class="select select-bordered select-sm"
              :disabled="editingPlugin && !isEditing"
            >
              <option value="info">{{ $t('common.info', '信息') }}</option>
              <option value="low">{{ $t('common.low', '低危') }}</option>
              <option value="medium">{{ $t('common.medium', '中危') }}</option>
              <option value="high">{{ $t('common.high', '高危') }}</option>
              <option value="critical">{{ $t('common.critical', '严重') }}</option>
            </select>
          </div>
          
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('common.description', '描述') }}</span>
            </label>
            <input 
              v-model="newPluginMetadata.description"
              type="text" 
              :placeholder="$t('plugins.descriptionPlaceholder', '插件功能描述')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
          
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.tags', '标签') }} ({{ $t('plugins.commaSeparated', '逗号分隔') }})</span>
            </label>
            <input 
              v-model="newPluginMetadata.tagsString"
              type="text" 
              :placeholder="$t('plugins.tagsPlaceholder', '例如: security, scanner, sql')"
              class="input input-bordered input-sm"
              :disabled="editingPlugin && !isEditing"
            />
          </div>
        </div>
        
        <!-- Code Editor -->
        <div class="form-control w-full">
          <div class="flex justify-between items-center mb-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.pluginCode', '插件代码') }}</span>
            </label>
            <div class="flex gap-2">
              <button 
                v-if="!editingPlugin"
                class="btn btn-xs btn-outline"
                @click="insertTemplate"
              >
                <i class="fas fa-file-code mr-1"></i>
                {{ $t('plugins.insertTemplate', '插入模板') }}
              </button>
              <button 
                class="btn btn-xs btn-outline"
                @click="formatCode"
              >
                <i class="fas fa-indent mr-1"></i>
                {{ $t('plugins.format', '格式化') }}
              </button>
            </div>
          </div>
          <div ref="codeEditorContainer" class="border border-base-300 rounded-lg overflow-hidden min-h-96"></div>
        </div>
        
        <div v-if="codeError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ codeError }}</span>
        </div>
        
        <!-- Fixed Footer -->
        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button class="btn btn-sm" @click="closeCodeEditorDialog">{{ $t('common.close', '关闭') }}</button>
          
          <!-- Edit Mode Buttons -->
          <template v-if="editingPlugin">
            <button 
              v-if="!isEditing"
              class="btn btn-primary btn-sm"
              @click="enableEditing"
            >
              <i class="fas fa-edit mr-2"></i>
              {{ $t('common.edit', '编辑') }}
            </button>
            <template v-else>
              <button class="btn btn-warning btn-sm" @click="cancelEditing">{{ $t('plugins.cancelEdit', '取消编辑') }}</button>
              <button 
                class="btn btn-success btn-sm"
                :disabled="saving"
                @click="savePluginCode"
              >
                <span v-if="saving" class="loading loading-spinner"></span>
                {{ saving ? $t('common.saving', '保存中...') : $t('common.save', '保存') }}
              </button>
            </template>
          </template>
          
          <!-- New Plugin Mode Buttons -->
          <template v-else>
            <button 
              class="btn btn-success btn-sm"
              :disabled="saving || !isNewPluginValid"
              @click="createNewPlugin"
            >
              <span v-if="saving" class="loading loading-spinner"></span>
              {{ saving ? $t('plugins.creating', '创建中...') : $t('plugins.createPlugin', '创建插件') }}
            </button>
          </template>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeCodeEditorDialog">close</button>
      </form>
    </dialog>
    
    <!-- Delete Confirmation Dialog -->
    <dialog ref="deleteDialog" class="modal">
      <div class="modal-box max-h-[90vh] overflow-y-auto">
        <!-- Fixed Header -->
        <div class="flex justify-between items-start mb-4 sticky top-0 bg-base-100 z-10 pb-2">
          <h3 class="font-bold text-lg">{{ $t('plugins.confirmDelete', '确认删除') }}</h3>
          <button @click="closeDeleteDialog" class="btn btn-sm btn-circle btn-ghost">✕</button>
        </div>
        
        <p class="py-4">
          {{ $t('plugins.deleteConfirmText', '确定要删除插件') }} <strong>{{ deletingPlugin?.metadata.name }}</strong> {{ $t('plugins.deleteWarning', '吗？此操作不可撤销。') }}
        </p>
        
        <!-- Fixed Footer -->
        <div class="modal-action sticky bottom-0 bg-base-100 pt-4">
          <button class="btn btn-sm" @click="closeDeleteDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-error btn-sm"
            :disabled="deleting"
            @click="deletePlugin"
          >
            <span v-if="deleting" class="loading loading-spinner"></span>
            {{ deleting ? $t('plugins.deleting', '删除中...') : $t('common.delete', '删除') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeDeleteDialog">close</button>
      </form>
    </dialog>
    
    <!-- AI Generate Plugin Dialog -->
    <dialog ref="aiGenerateDialog" class="modal">
      <div class="modal-box w-11/12 max-w-3xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-magic mr-2"></i>
          {{ $t('plugins.aiGenerate', 'AI生成插件') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.aiPrompt', '描述你想要的插件功能') }}</span>
            </label>
            <textarea 
              v-model="aiPrompt"
              class="textarea textarea-bordered h-32"
              :placeholder="$t('plugins.aiPromptPlaceholder', '例如：我需要一个检测SQL注入漏洞的插件，能够分析HTTP请求中的参数，识别潜在的SQL注入模式...')"
            ></textarea>
          </div>
          
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('plugins.pluginType', '插件类型') }}</span>
              </label>
              <select v-model="aiPluginType" class="select select-bordered select-sm">
                <option value="passiveScan">{{ $t('plugins.categories.passiveScan', '被动扫描插件') }}</option>
                <option value="agentTools">{{ $t('plugins.categories.agentTools', 'Agent工具插件') }}</option>
              </select>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('plugins.severity', '严重程度') }}</span>
              </label>
              <select v-model="aiSeverity" class="select select-bordered select-sm">
                <option value="info">{{ $t('common.info', '信息') }}</option>
                <option value="low">{{ $t('common.low', '低危') }}</option>
                <option value="medium">{{ $t('common.medium', '中危') }}</option>
                <option value="high">{{ $t('common.high', '高危') }}</option>
                <option value="critical">{{ $t('common.critical', '严重') }}</option>
              </select>
            </div>
          </div>
          
          <div v-if="aiGenerating" class="alert alert-info">
            <span class="loading loading-spinner"></span>
            <span>{{ $t('plugins.aiGenerating', 'AI正在生成插件代码，请稍候...') }}</span>
          </div>
          
          <div v-if="aiGenerateError" class="alert alert-error">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ aiGenerateError }}</span>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeAIGenerateDialog">{{ $t('common.cancel', '取消') }}</button>
          <button 
            class="btn btn-primary"
            :disabled="!aiPrompt.trim() || aiGenerating"
            @click="generatePluginWithAI"
          >
            <i class="fas fa-magic mr-2"></i>
            {{ $t('plugins.generatePlugin', '生成插件') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeAIGenerateDialog">close</button>
      </form>
    </dialog>
    
    <!-- Test Result Dialog -->
    <dialog ref="testResultDialog" class="modal">
      <div class="modal-box w-11/12 max-w-3xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-vial mr-2"></i>
          {{ $t('plugins.testResult', '插件测试结果') }}
          <span v-if="testResult" class="text-sm font-normal text-gray-500 ml-2">
            ({{ getTestTypeLabel() }})
          </span>
        </h3>
        
        <div v-if="testing" class="alert alert-info">
          <span class="loading loading-spinner"></span>
          <span>{{ $t('plugins.testing', '正在测试插件...') }}</span>
        </div>
        
        <div v-else-if="testResult" class="space-y-4">
          <div class="alert" :class="{
            'alert-success': testResult.success,
            'alert-error': !testResult.success
          }">
            <i :class="testResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
            <span>{{ testResult.success ? $t('plugins.testPassed', '测试通过') : $t('plugins.testFailed', '测试失败') }}</span>
          </div>
          
          <div v-if="testResult.message" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.testMessage', '测试消息') }}</h4>
              <pre class="text-sm whitespace-pre-wrap">{{ testResult.message }}</pre>
            </div>
          </div>
          
          <div v-if="testResult.findings && testResult.findings.length > 0" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.findings', '发现的问题') }} ({{ testResult.findings.length }})</h4>
              <div class="space-y-2">
                <div v-for="(finding, idx) in testResult.findings" :key="idx" class="card bg-base-100">
                  <div class="card-body p-3">
                    <div class="flex justify-between items-start">
                      <span class="font-medium">{{ finding.title }}</span>
                      <span class="badge" :class="{
                        'badge-error': finding.severity === 'critical' || finding.severity === 'high',
                        'badge-warning': finding.severity === 'medium',
                        'badge-info': finding.severity === 'low' || finding.severity === 'info'
                      }">{{ finding.severity }}</span>
                    </div>
                    <p class="text-sm text-base-content/70 mt-1">{{ finding.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <div v-if="testResult.error" class="alert alert-error">
            <i class="fas fa-exclamation-circle"></i>
            <span>{{ testResult.error }}</span>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeTestResultDialog">{{ $t('common.close', '关闭') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeTestResultDialog">close</button>
      </form>
    </dialog>

    <!-- Advanced Test Dialog -->
    <dialog ref="advancedDialog" class="modal">
      <div class="modal-box w-11/12 max-w-5xl">
        <h3 class="font-bold text-base mb-4">
          <i class="fas fa-gauge-high mr-2"></i>
          {{ $t('plugins.advancedTest', '高级测试') }}
          <span v-if="advancedPlugin" class="text-sm font-normal text-gray-500 ml-2">
            {{ advancedPlugin.metadata.name }} ({{ advancedPlugin.metadata.id }})
          </span>
        </h3>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.requestUrl', '请求 URL') }}</span>
            </label>
            <input v-model="advancedForm.url" type="text" class="input input-bordered input-sm w-full" placeholder="https://example.com/test" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.httpMethod', 'HTTP 方法') }}</span>
            </label>
            <select v-model="advancedForm.method" class="select select-bordered select-sm w-full">
              <option>GET</option>
              <option>POST</option>
              <option>PUT</option>
              <option>DELETE</option>
              <option>PATCH</option>
              <option>HEAD</option>
              <option>OPTIONS</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.runs', '运行次数') }}</span>
            </label>
            <input v-model.number="advancedForm.runs" type="number" min="1" class="input input-bordered input-sm w-full" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('plugins.concurrency', '并发数') }}</span>
            </label>
            <input v-model.number="advancedForm.concurrency" type="number" min="1" class="input input-bordered input-sm w-full" />
          </div>

          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.headersJson', '请求头 (JSON)') }}</span>
            </label>
            <textarea v-model="advancedForm.headersText" class="textarea textarea-bordered font-mono text-xs h-24" placeholder='{"User-Agent":"Sentinel-AdvTest/1.0"}'></textarea>
            <label class="label">
              <span class="label-text-alt">{{ $t('plugins.headersHint', '留空则使用默认 UA。必须是有效 JSON。') }}</span>
            </label>
          </div>

          <div class="form-control col-span-2">
            <label class="label">
              <span class="label-text">{{ $t('plugins.body', '请求体') }}</span>
            </label>
            <textarea v-model="advancedForm.bodyText" class="textarea textarea-bordered font-mono text-xs h-24" placeholder=""></textarea>
          </div>
        </div>

        <div v-if="advancedError" class="alert alert-error mt-4">
          <i class="fas fa-exclamation-circle"></i>
          <span>{{ advancedError }}</span>
        </div>

        <div class="mt-4">
          <button class="btn btn-primary" :disabled="advancedTesting" @click="runAdvancedTest">
            <span v-if="advancedTesting" class="loading loading-spinner"></span>
            {{ advancedTesting ? $t('plugins.testing', '正在测试...') : $t('plugins.startTest', '开始测试') }}
          </button>
        </div>

        <div v-if="advancedTesting" class="alert alert-info mt-4">
          <span class="loading loading-spinner"></span>
          <span class="ml-2">{{ $t('plugins.testing', '正在测试插件...') }}</span>
        </div>

        <div v-else-if="advancedResult" class="space-y-4 mt-4">
          <!-- Summary -->
          <div class="stats shadow w-full">
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.totalRuns', '总运行') }}</div>
              <div class="stat-value text-primary">{{ advancedResult.total_runs }}</div>
              <div class="stat-desc">{{ $t('plugins.concurrency', '并发') }}: {{ advancedResult.concurrency }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.totalDuration', '总耗时(ms)') }}</div>
              <div class="stat-value">{{ advancedResult.total_duration_ms }}</div>
              <div class="stat-desc">{{ $t('plugins.avgPerRun', '平均/次(ms)') }}: {{ advancedResult.avg_duration_ms.toFixed(1) }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ $t('plugins.findingsTotal', '发现数') }}</div>
              <div class="stat-value text-secondary">{{ advancedResult.total_findings }}</div>
              <div class="stat-desc">{{ $t('plugins.unique', '唯一') }}: {{ advancedResult.unique_findings }}</div>
            </div>
          </div>

          <!-- Per-run table -->
          <div class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.runDetails', '运行详情') }}</h4>
              <div class="overflow-x-auto">
                <table class="table table-zebra w-full">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>{{ $t('plugins.duration', '耗时(ms)') }}</th>
                      <th>{{ $t('plugins.findings', '发现') }}</th>
                      <th>{{ $t('plugins.error', '错误') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in sortedRuns" :key="r.run_index">
                      <td>{{ r.run_index }}</td>
                      <td>{{ r.duration_ms }}</td>
                      <td>{{ r.findings }}</td>
                      <td>
                        <span v-if="r.error" class="badge badge-error">{{ r.error }}</span>
                        <span v-else class="badge badge-success">OK</span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- Unique Findings List -->
          <div v-if="advancedResult.findings && advancedResult.findings.length > 0" class="card bg-base-200">
            <div class="card-body">
              <h4 class="font-semibold mb-2">{{ $t('plugins.uniqueFindings', '唯一发现') }} ({{ advancedResult.findings.length }})</h4>
              <div class="space-y-2">
                <div v-for="(finding, idx) in advancedResult.findings" :key="idx" class="card bg-base-100">
                  <div class="card-body p-3">
                    <div class="flex justify-between items-start">
                      <span class="font-medium">{{ finding.title }}</span>
                      <span class="badge" :class="{
                        'badge-error': finding.severity === 'critical' || finding.severity === 'high',
                        'badge-warning': finding.severity === 'medium',
                        'badge-info': finding.severity === 'low' || finding.severity === 'info'
                      }">{{ finding.severity }}</span>
                    </div>
                    <p class="text-sm text-base-content/70 mt-1">{{ finding.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn" @click="closeAdvancedDialog">{{ $t('common.close', '关闭') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeAdvancedDialog">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { readFile } from '@tauri-apps/plugin-fs'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState, Compartment } from '@codemirror/state'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'
import { keymap } from '@codemirror/view'
import { defaultKeymap, indentWithTab } from '@codemirror/commands'

const { t } = useI18n()

// Plugin Record Types
interface PluginMetadata {
  id: string
  name: string
  version: string
  author?: string
  main_category: string  // 主分类：passive 或 agent
  category: string       // 子分类：sqli, xss, scanner, analyzer 等
  default_severity: string
  tags: string[]
  description?: string
}

interface PluginRecord {
  metadata: PluginMetadata
  path: string
  status: 'Enabled' | 'Disabled' | 'Error'
  last_error?: string
  is_favorited?: boolean
}

// Review Plugin Types
interface ReviewPlugin {
  plugin_id: string
  plugin_name: string
  code: string
  description: string
  vuln_type: string
  quality_score: number
  quality_breakdown: {
    syntax_score: number
    logic_score: number
    security_score: number
    code_quality_score: number
  }
  validation: {
    is_valid: boolean
    syntax_valid: boolean
    has_required_functions: boolean
    security_check_passed: boolean
    errors: string[]
    warnings: string[]
  }
  status: string
  generated_at: string
  model: string
}

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

interface TestResult {
  success: boolean
  message?: string
  findings?: Array<{
    title: string
    description: string
    severity: string
  }>
  error?: string
}

interface AdvancedRunStat {
  run_index: number
  duration_ms: number
  findings: number
  error?: string | null
}

interface AdvancedTestResult {
  plugin_id: string
  success: boolean
  total_runs: number
  concurrency: number
  total_duration_ms: number
  avg_duration_ms: number
  total_findings: number
  unique_findings: number
  findings: Array<{ title: string; description: string; severity: string }>
  runs: AdvancedRunStat[]
  message?: string
  error?: string
}

// Component State
const selectedCategory = ref('all')
const plugins = ref<PluginRecord[]>([])
const uploadDialog = ref<HTMLDialogElement>()
const codeEditorDialog = ref<HTMLDialogElement>()
const deleteDialog = ref<HTMLDialogElement>()
const aiGenerateDialog = ref<HTMLDialogElement>()
const testResultDialog = ref<HTMLDialogElement>()
const advancedDialog = ref<HTMLDialogElement>()
const reviewDetailDialog = ref<HTMLDialogElement>()
const fileInput = ref<HTMLInputElement>()

// CodeMirror Editor Refs
const codeEditorContainer = ref<HTMLDivElement>()
const reviewCodeEditorContainer = ref<HTMLDivElement>()
let codeEditorView: EditorView | null = null
let reviewCodeEditorView: EditorView | null = null

// CodeMirror Compartments for dynamic configuration
const codeEditorReadOnly = new Compartment()
const reviewCodeEditorReadOnly = new Compartment()

// Review Plugin State
const reviewPlugins = ref<ReviewPlugin[]>([])
const selectedReviewPlugins = ref<ReviewPlugin[]>([])
const selectedReviewPlugin = ref<ReviewPlugin | null>(null)
const reviewSearchText = ref('')
const reviewEditMode = ref(false)
const editedReviewCode = ref('')
const savingReview = ref(false)

// Review Filter and Pagination State
const reviewStatusFilter = ref<string>('all') // 'all' | 'PendingReview' | 'Approved' | 'Rejected' | 'ValidationFailed'
const reviewCurrentPage = ref(1)
const reviewPageSize = ref(10)
const reviewTotalCount = ref(0)
const reviewTotalPagesCount = ref(0)

// Review Statistics (from backend)
const reviewStatsData = ref({
  total: 0,
  pending: 0,
  approved: 0,
  rejected: 0,
  failed: 0
})

// Plugin List Filter and Pagination State (被动扫描/Agent工具)
const pluginViewMode = ref<'favorited' | 'all'>('all') // 默认显示全部插件
const pluginCurrentPage = ref(1)
const pluginPageSize = ref(10)
const pluginTotalCount = ref(0)
const pluginTotalPagesCount = ref(0)
const pluginSearchText = ref('')

const selectedFile = ref<File | null>(null)
const uploading = ref(false)
const uploadError = ref('')

const editingPlugin = ref<PluginRecord | null>(null)
const pluginCode = ref('')
const originalCode = ref('')
const isEditing = ref(false)
const saving = ref(false)
const codeError = ref('')

const deletingPlugin = ref<PluginRecord | null>(null)
const deleting = ref(false)

// New Plugin Related State
const newPluginMetadata = ref({
  id: '',
  name: '',
  version: '1.0.0',
  author: '',
  mainCategory: 'passive', // 大分类: passive (被动扫描) 或 agent (Agent插件)
  category: 'vulnerability', // 子分类
  default_severity: 'medium',
  description: '',
  tagsString: ''
})

// 大分类定义
const mainCategories = [
  { value: 'passive', label: '被动扫描插件', icon: 'fas fa-shield-alt' },
  { value: 'agent', label: 'Agent插件', icon: 'fas fa-robot' }
]

// 被动扫描插件的所有子分类
const passiveScanCategories = [
  'sqli', 'command_injection', 'xss', 'idor', 'auth_bypass', 'csrf', 
  'info_leak', 'file_upload', 'file_inclusion', 'path_traversal', 
  'xxe', 'ssrf', 'report', 'custom'
]

// Agent 插件的所有子分类
const agentToolsCategories = [
  'scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility', 'custom', 'agentTools'
]

// 子分类定义（根据大分类动态变化）
const subCategories = computed(() => {
  if (newPluginMetadata.value.mainCategory === 'passive') {
    return [
      // 注入类
      { value: 'sqli', label: 'SQL注入', icon: 'fas fa-database' },
      { value: 'command_injection', label: '命令注入', icon: 'fas fa-terminal' },
      // XSS
      { value: 'xss', label: '跨站脚本', icon: 'fas fa-code' },
      // 访问控制
      { value: 'idor', label: '越权访问', icon: 'fas fa-user-lock' },
      { value: 'auth_bypass', label: '认证绕过', icon: 'fas fa-unlock' },
      // CSRF
      { value: 'csrf', label: 'CSRF', icon: 'fas fa-shield-alt' },
      // 信息泄露
      { value: 'info_leak', label: '信息泄露', icon: 'fas fa-eye-slash' },
      // 文件操作
      { value: 'file_upload', label: '文件上传', icon: 'fas fa-file-upload' },
      { value: 'file_inclusion', label: '文件包含', icon: 'fas fa-file-code' },
      { value: 'path_traversal', label: '目录穿越', icon: 'fas fa-folder-open' },
      // 服务端漏洞
      { value: 'xxe', label: 'XXE', icon: 'fas fa-file-code' },
      { value: 'ssrf', label: 'SSRF', icon: 'fas fa-server' },
      // 自定义
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  } else if (newPluginMetadata.value.mainCategory === 'agent') {
    return [
      { value: 'scanner', label: '扫描工具', icon: 'fas fa-radar' },
      { value: 'analyzer', label: '分析工具', icon: 'fas fa-microscope' },
      { value: 'reporter', label: '报告工具', icon: 'fas fa-file-alt' },
      { value: 'recon', label: '信息收集', icon: 'fas fa-search' },
      { value: 'exploit', label: '漏洞利用', icon: 'fas fa-bomb' },
      { value: 'utility', label: '实用工具', icon: 'fas fa-toolbox' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  }
  return []
})

// 根据旧的category值推断mainCategory（用于向后兼容）
const inferMainCategory = (category: string): string => {
  // agentTools 相关
  if (category === 'agentTools' || category === 'scanner' || category === 'analyzer' || 
      category === 'reporter' || category === 'recon' || category === 'exploit' || category === 'utility') {
    return 'agent'
  }
  
  // 根据分类名称推断（更宽松的匹配）
  const lowerCategory = category.toLowerCase()
  
  // Agent 相关关键词
  if (lowerCategory.includes('agent') || lowerCategory.includes('tool') || 
      lowerCategory.includes('scanner') || lowerCategory.includes('analyzer') ||
      lowerCategory.includes('reporter') || lowerCategory.includes('recon') ||
      lowerCategory.includes('exploit') || lowerCategory.includes('utility')) {
    return 'agent'
  }
  
  // 被动扫描相关（默认）
  return 'passive'
}

// 将旧的category值转换为新的子分类
const convertToSubCategory = (category: string): string => {
  // 如果是agentTools，转为scanner
  if (category === 'agentTools') return 'scanner'
  // 如果是passiveScan，转为vulnerability  
  if (category === 'passiveScan') return 'vulnerability'
  // 其他保持不变
  return category
}

// AI Generation Related State
const aiPrompt = ref('')
const aiPluginType = ref('vulnerability')
const aiSeverity = ref('medium')
const aiGenerating = ref(false)
const aiGenerateError = ref('')

// Test Related State
const testing = ref(false)
const testResult = ref<TestResult | null>(null)

// Advanced Test State
const advancedPlugin = ref<PluginRecord | null>(null)
const advancedTesting = ref(false)
const advancedError = ref('')
const advancedResult = ref<AdvancedTestResult | null>(null)
const advancedForm = ref({
  url: 'https://example.com/test',
  method: 'GET',
  headersText: '{"User-Agent":"Sentinel-AdvTest/1.0"}',
  bodyText: '',
  runs: 3,
  concurrency: 2,
})

let pluginChangedUnlisten: UnlistenFn | null = null

// Categories Definition
const categories = computed(() => [
  {
    value: 'all',
    label: t('plugins.categories.all', '全部'),
    icon: 'fas fa-th'
  },
  {
    value: 'passiveScan',
    label: t('plugins.categories.passiveScan', '被动扫描插件'),
    icon: 'fas fa-shield-alt'
  },
  {
    value: 'agentTools',
    label: t('plugins.categories.agentTools', 'Agent工具插件'),
    icon: 'fas fa-robot'
  },
//   {
//     value: 'builtinTools',
//     label: t('plugins.categories.builtinTools', '内置工具插件'),
//     icon: 'fas fa-toolbox'
//   },
//   {
//     value: 'mcpTools',
//     label: t('plugins.categories.mcpTools', 'MCP工具插件'),
//     icon: 'fas fa-plug'
//   },
//   {
//     value: 'vulnerability',
//     label: t('plugins.categories.vulnerability', '漏洞扫描'),
//     icon: 'fas fa-bug'
//   },
//   {
//     value: 'injection',
//     label: t('plugins.categories.injection', '注入检测'),
//     icon: 'fas fa-syringe'
//   },
//   {
//     value: 'xss',
//     label: t('plugins.categories.xss', '跨站脚本'),
//     icon: 'fas fa-code'
//   },
  // {
  //   value: 'custom',
  //   label: t('plugins.categories.custom', '自定义'),
  //   icon: 'fas fa-wrench'
  // }
])

// Filtered Plugins
const filteredPlugins = computed(() => {
  let filtered = plugins.value
  
  // 分类筛选
  if (selectedCategory.value === 'all') {
    filtered = plugins.value
  } else if (selectedCategory.value === 'passiveScan') {
    // 被动扫描插件：直接使用 main_category 字段
    filtered = plugins.value.filter(p => {
      // 优先使用 main_category 字段（后端已提供）
      if (p.metadata.main_category === 'passive') {
        return true
      }
      // 兼容旧数据：检查预定义的被动扫描子分类
      if (passiveScanCategories.includes(p.metadata.category)) {
        return true
      }
      // 兼容旧的 passiveScan 分类
      if (p.metadata.category === 'passiveScan') {
        return true
      }
      return false
    })
  } else if (selectedCategory.value === 'agentTools') {
    // Agent 工具插件：直接使用 main_category 字段
    filtered = plugins.value.filter(p => {
      // 优先使用 main_category 字段（后端已提供）
      if (p.metadata.main_category === 'agent') {
        return true
      }
      // 兼容旧数据：检查预定义的 Agent 子分类
      if (agentToolsCategories.includes(p.metadata.category)) {
        return true
      }
      return false
    })
  } else {
    // 其他：精确匹配
    filtered = plugins.value.filter(p => p.metadata.category === selectedCategory.value)
  }
  
  // 被动扫描插件的收藏筛选
  if (selectedCategory.value === 'passiveScan' && pluginViewMode.value === 'favorited') {
    filtered = filtered.filter(p => isPluginFavorited(p))
  }
  
  return filtered
})

// Review Plugins Computed (使用后端统计数据)
const reviewStats = computed(() => reviewStatsData.value)

// 审核插件列表（后端分页，直接使用）
const filteredReviewPlugins = computed(() => reviewPlugins.value)
const paginatedReviewPlugins = computed(() => reviewPlugins.value)

// 总页数（从后端获取）
const reviewTotalPages = computed(() => reviewTotalPagesCount.value)

// 分页信息
const reviewPaginationInfo = computed(() => {
  const start = (reviewCurrentPage.value - 1) * reviewPageSize.value + 1
  const end = Math.min(reviewCurrentPage.value * reviewPageSize.value, reviewTotalCount.value)
  const total = reviewTotalCount.value
  return { start, end, total }
})

const isAllSelected = computed(() => {
  return paginatedReviewPlugins.value.length > 0 && 
         paginatedReviewPlugins.value.every(p => isPluginSelected(p))
})

// 分页控制方法
const goToReviewPage = (page: number) => {
  if (page >= 1 && page <= reviewTotalPages.value) {
    reviewCurrentPage.value = page
    refreshReviewPlugins()
  }
}

const changeReviewPageSize = (size: number) => {
  reviewPageSize.value = size
  reviewCurrentPage.value = 1 // 重置到第一页
  refreshReviewPlugins()
}

// 切换状态筛选时重置到第一页
const changeReviewStatusFilter = (status: string) => {
  reviewStatusFilter.value = status
  reviewCurrentPage.value = 1
  refreshReviewPlugins()
}

// Get Category Count
const getCategoryCount = (category: string) => {
  if (category === 'all') return plugins.value.length
  // 被动扫描插件：统计所有被动扫描相关插件
  if (category === 'passiveScan') {
    return plugins.value.filter(p => {
      // 优先使用 main_category 字段
      if (p.metadata.main_category === 'passive') {
        return true
      }
      // 兼容旧数据：检查预定义的被动扫描子分类
      if (passiveScanCategories.includes(p.metadata.category)) {
        return true
      }
      // 兼容旧的 passiveScan 分类
      if (p.metadata.category === 'passiveScan') {
        return true
      }
      return false
    }).length
  }
  // Agent 工具插件：统计所有 Agent 相关插件
  if (category === 'agentTools') {
    return plugins.value.filter(p => {
      // 优先使用 main_category 字段
      if (p.metadata.main_category === 'agent') {
        return true
      }
      // 兼容旧数据：检查预定义的 Agent 子分类
      if (agentToolsCategories.includes(p.metadata.category)) {
        return true
      }
      return false
    }).length
  }
  // 其他：精确匹配
  return plugins.value.filter(p => p.metadata.category === category).length
}

// Sorted runs by index for stable display
const sortedRuns = computed(() => {
  if (!advancedResult.value) return [] as AdvancedRunStat[]
  return [...advancedResult.value.runs].sort((a, b) => a.run_index - b.run_index)
})

// Get Category Label
const getCategoryLabel = (category: string) => {
  const cat = categories.value.find(c => c.value === category)
  return cat ? cat.label : category
}

// Get Category Icon
const getCategoryIcon = (category: string) => {
  // 先尝试从主分类中查找
  const cat = categories.value.find(c => c.value === category)
  if (cat) return cat.icon
  
  // 再从子分类中查找（被动扫描和Agent工具）
  const allSubCategories = [
    ...subCategories.value,
    // Agent 子分类
    { value: 'scanner', icon: 'fas fa-radar' },
    { value: 'analyzer', icon: 'fas fa-microscope' },
    { value: 'reporter', icon: 'fas fa-file-alt' },
    { value: 'recon', icon: 'fas fa-search' },
    { value: 'exploit', icon: 'fas fa-bomb' },
    { value: 'utility', icon: 'fas fa-toolbox' }
  ]
  
  const subCat = allSubCategories.find(c => c.value === category)
  return subCat ? subCat.icon : 'fas fa-wrench'
}

// Check if plugin is Agent type
const isAgentPluginType = (plugin: PluginRecord): boolean => {
  return plugin.metadata.category === 'agentTools' || 
         ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
}

// Check if plugin is Passive Scan type
const isPassiveScanPluginType = (plugin: PluginRecord): boolean => {
  return passiveScanCategories.includes(plugin.metadata.category)
}

// Validate New Plugin
const isNewPluginValid = computed(() => {
  return newPluginMetadata.value.id.trim() !== '' &&
         newPluginMetadata.value.name.trim() !== '' &&
         newPluginMetadata.value.category.trim() !== '' &&
         pluginCode.value.trim() !== ''
})

// Get Status Text
const getStatusText = (status: string): string => {
  const statusMap: Record<string, string> = {
    'Enabled': t('plugins.enabled', '已启用'),
    'Disabled': t('plugins.disabled', '已禁用'),
    'Error': t('plugins.error', '错误')
  }
  return statusMap[status] || status
}

// Initialize CodeMirror for Code Editor Dialog
const initCodeEditor = async () => {
  await nextTick()
  if (!codeEditorContainer.value) return
  
  // Destroy existing editor if any
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  
  // Clear container
  codeEditorContainer.value.innerHTML = ''
  
  const readonly = editingPlugin.value && !isEditing.value
  
  const state = EditorState.create({
    doc: pluginCode.value,
    extensions: [
      basicSetup,
      javascript({ typescript: true }),
      oneDark,
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          pluginCode.value = update.state.doc.toString()
        }
      }),
      codeEditorReadOnly.of(EditorView.editable.of(!readonly)),
    ],
  })
  
  codeEditorView = new EditorView({
    state,
    parent: codeEditorContainer.value,
  })
}

// Initialize CodeMirror for Review Plugin Detail Dialog
const initReviewCodeEditor = async () => {
  await nextTick()
  if (!reviewCodeEditorContainer.value) return
  
  // Destroy existing editor if any
  if (reviewCodeEditorView) {
    reviewCodeEditorView.destroy()
    reviewCodeEditorView = null
  }
  
  // Clear container
  reviewCodeEditorContainer.value.innerHTML = ''
  
  const state = EditorState.create({
    doc: editedReviewCode.value,
    extensions: [
      basicSetup,
      javascript({ typescript: true }),
      oneDark,
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          editedReviewCode.value = update.state.doc.toString()
        }
      }),
      reviewCodeEditorReadOnly.of(EditorView.editable.of(reviewEditMode.value)),
    ],
  })
  
  reviewCodeEditorView = new EditorView({
    state,
    parent: reviewCodeEditorContainer.value,
  })
}

// Update CodeMirror editor content
const updateCodeEditorContent = (newContent: string) => {
  if (codeEditorView) {
    const transaction = codeEditorView.state.update({
      changes: {
        from: 0,
        to: codeEditorView.state.doc.length,
        insert: newContent
      }
    })
    codeEditorView.dispatch(transaction)
  }
}

// Update CodeMirror editor readonly state
const updateCodeEditorReadonly = (readonly: boolean) => {
  if (codeEditorView) {
    codeEditorView.dispatch({
      effects: codeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly))
    })
  }
}

// Update Review CodeMirror editor readonly state
const updateReviewCodeEditorReadonly = (readonly: boolean) => {
  if (reviewCodeEditorView) {
    reviewCodeEditorView.dispatch({
      effects: reviewCodeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly))
    })
  }
}

// Show Toast
const showToast = (message: string, type: 'success' | 'error' | 'info' | 'warning' = 'success') => {
  const toast = document.createElement('div')
  toast.className = 'toast toast-top toast-end z-50'
  toast.style.top = '5rem'
  
  const alertClass = {
    success: 'alert-success',
    error: 'alert-error',
    info: 'alert-info',
    warning: 'alert-warning'
  }[type]
  
  const icon = {
    success: 'fa-check-circle',
    error: 'fa-times-circle',
    info: 'fa-info-circle',
    warning: 'fa-exclamation-triangle'
  }[type]
  
  toast.innerHTML = `
    <div class="alert ${alertClass} shadow-lg">
      <i class="fas ${icon}"></i>
      <span>${message}</span>
    </div>
  `
  document.body.appendChild(toast)
  setTimeout(() => toast.remove(), 3000)
}

// Refresh Plugins List
const refreshPlugins = async () => {
  try {
    const response = await invoke<CommandResponse<PluginRecord[]>>('list_plugins')
    if (response.success && response.data) {
      plugins.value = response.data
    } else {
      console.error('Failed to refresh plugins:', response.error)
    }
  } catch (error) {
    console.error('Error refreshing plugins:', error)
  }
}

// Toggle plugin favorite
const togglePluginFavorite = async (plugin: PluginRecord) => {
  try {
    const response: any = await invoke('toggle_plugin_favorite', {
      pluginId: plugin.metadata.id,
      userId: null
    })
    
    if (response.success) {
      const isFavorited = response.data?.is_favorited || false
      showToast(
        isFavorited ? t('plugins.favoritedSuccess', '已收藏') : t('plugins.unfavoritedSuccess', '已取消收藏'),
        'success'
      )
      // 刷新插件列表
      await refreshPlugins()
    } else {
      showToast(t('plugins.favoriteError', '操作失败'), 'error')
    }
  } catch (error) {
    console.error('Error toggling favorite:', error)
    showToast(t('plugins.favoriteError', '操作失败'), 'error')
  }
}

// Check if plugin is favorited
const isPluginFavorited = (plugin: PluginRecord): boolean => {
  return plugin.is_favorited || false
}

// Scan Plugin Directory
// const scanPluginDirectory = async () => {
//   try {
//     const response = await invoke<CommandResponse<string[]>>('scan_plugin_directory')
//     if (response.success) {
//       await refreshPlugins()
//       const count = response.data?.length || 0
//       showToast(t('plugins.scanComplete', `扫描完成，发现 ${count} 个插件`), 'success')
//     } else {
//       showToast(t('plugins.scanFailed', `扫描失败: ${response.error || '未知错误'}`), 'error')
//     }
//   } catch (error) {
//     console.error('Error scanning plugin directory:', error)
//     showToast(t('plugins.scanError', '扫描插件目录时出错'), 'error')
//   }
// }

// Toggle Plugin Enable/Disable
const togglePlugin = async (plugin: PluginRecord) => {
  try {
    const command = plugin.status === 'Enabled' ? 'disable_plugin' : 'enable_plugin'
    const actionText = plugin.status === 'Enabled' ? t('plugins.disable', '禁用') : t('plugins.enable', '启用')
    
    const response = await invoke<CommandResponse<void>>(command, {
      pluginId: plugin.metadata.id
    })
    
    if (response.success) {
      await refreshPlugins()
      showToast(t('plugins.toggleSuccess', `插件已${actionText}: ${plugin.metadata.name}`), 'success')
    } else {
      showToast(t('plugins.toggleFailed', `${actionText}失败: ${response.error || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error toggling plugin:', error)
    showToast(t('plugins.toggleError', '操作失败'), 'error')
  }
}

// Open Create Plugin Dialog
const openCreateDialog = async () => {
  newPluginMetadata.value = {
    id: '',
    name: '',
    version: '1.0.0',
    author: '',
    mainCategory: 'passive',
    category: 'vulnerability',
    default_severity: 'medium',
    description: '',
    tagsString: ''
  }
  pluginCode.value = ''
  editingPlugin.value = null
  isEditing.value = false
  codeError.value = ''
  codeEditorDialog.value?.showModal()
  await initCodeEditor()
}

// Open Upload Dialog
const openUploadDialog = () => {
  uploadError.value = ''
  selectedFile.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
  uploadDialog.value?.showModal()
}

// Advanced Test Dialog Controls
const openAdvancedDialog = (plugin: PluginRecord) => {
  advancedPlugin.value = plugin
  advancedError.value = ''
  advancedResult.value = null
  
  // 检查是否为 Agent 插件
  const isAgentPlugin = plugin.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
  
  if (isAgentPlugin) {
    // Agent 插件显示提示信息
    advancedError.value = 'Agent 插件暂不支持高级并发测试功能。Agent 插件测试通过调用 analyze 函数进行，请使用普通测试按钮。'
  } else {
    // 被动扫描插件初始化测试参数
    if (!advancedForm.value.url) advancedForm.value.url = 'https://example.com/test'
    if (!advancedForm.value.method) advancedForm.value.method = 'GET'
    if (!advancedForm.value.runs || advancedForm.value.runs < 1) advancedForm.value.runs = 3
    if (!advancedForm.value.concurrency || advancedForm.value.concurrency < 1) advancedForm.value.concurrency = 2
  }
  
  advancedDialog.value?.showModal()
}

const formatDate = (dateString: string) => {
  if (!dateString) return '-'
  return new Date(dateString).toLocaleDateString()
}

const closeAdvancedDialog = () => {
  advancedDialog.value?.close()
  advancedTesting.value = false
  // do not reset form to preserve inputs between opens
}

// Close Upload Dialog
const closeUploadDialog = () => {
  uploadDialog.value?.close()
  selectedFile.value = null
  uploadError.value = ''
}

// Handle File Select
const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    selectedFile.value = target.files[0]
    uploadError.value = ''
  }
}

// Upload Plugin
const uploadPlugin = async () => {
  if (!selectedFile.value) return
  
  uploading.value = true
  uploadError.value = ''
  
  try {
    const fileContent = await selectedFile.value.text()
    
    // Parse plugin metadata from comments
    const metadataMatch = fileContent.match(/\/\*\*\s*([\s\S]*?)\s*\*\//)
    if (!metadataMatch) {
      uploadError.value = t('plugins.parseMetadataError', '无法从文件中解析插件元数据')
      uploading.value = false
      return
    }
    
    const metadataText = metadataMatch[1]
    const extractField = (field: string): string => {
      const match = metadataText.match(new RegExp(`@${field}\\s+(.+)`, 'i'))
      return match ? match[1].trim() : ''
    }
    
    const id = extractField('plugin')
    const name = extractField('name')
    const version = extractField('version')
    const author = extractField('author') || 'Unknown'
    const category = extractField('category')
    const defaultSeverity = extractField('default_severity')
    const tagsStr = extractField('tags')
    const description = extractField('description')
    
    if (!id || !name || !version || !category || !defaultSeverity) {
      uploadError.value = t('plugins.incompleteMetadata', '插件元数据不完整')
      uploading.value = false
      return
    }
    
    const tags = tagsStr.split(',').map(t => t.trim()).filter(t => t.length > 0)
    
    // 推断 main_category：agentTools 类别为 agent，其他为 passive
    const mainCategory = category === 'agentTools' ? 'agent' : 'passive'
    
    const metadata = {
      id, name, version, author,
      main_category: mainCategory, // 添加主分类字段
      category, description,
      default_severity: defaultSeverity,
      tags
    }
    
    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fileContent
    })
    
    if (response.success) {
      closeUploadDialog()
      await refreshPlugins()
      showToast(t('plugins.uploadSuccess', `插件上传成功: ${name}`), 'success')
    } else {
      uploadError.value = response.error || t('plugins.uploadFailed', '上传失败')
    }
  } catch (error) {
    uploadError.value = error instanceof Error ? error.message : t('plugins.uploadFailed', '上传失败')
    console.error('Error uploading plugin:', error)
  } finally {
    uploading.value = false
  }
}

// View Plugin Code
const viewPluginCode = async (plugin: PluginRecord) => {
  try {
    const response = await invoke<CommandResponse<string | null>>('get_plugin_code', {
      pluginId: plugin.metadata.id
    })
    
    if (response.success && response.data) {
      // Successfully got code from backend
      pluginCode.value = response.data
      originalCode.value = pluginCode.value
      editingPlugin.value = plugin
      
      // Load plugin metadata into form
      const oldCategory = plugin.metadata.category
      newPluginMetadata.value = {
        id: plugin.metadata.id,
        name: plugin.metadata.name,
        version: plugin.metadata.version,
        author: plugin.metadata.author || '',
        mainCategory: inferMainCategory(oldCategory),
        category: convertToSubCategory(oldCategory),
        default_severity: plugin.metadata.default_severity,
        description: plugin.metadata.description || '',
        tagsString: plugin.metadata.tags.join(', ')
      }
      
      isEditing.value = false
      codeError.value = ''
      codeEditorDialog.value?.showModal()
      await initCodeEditor()
    } else if (response.success && response.data === null && plugin.path) {
      // Fallback to reading from file only if path exists
      try {
        const content = await readFile(plugin.path)
        const decoder = new TextDecoder('utf-8')
        pluginCode.value = decoder.decode(content)
        originalCode.value = pluginCode.value
        editingPlugin.value = plugin
        
        // Load plugin metadata into form
        const oldCategory = plugin.metadata.category
        newPluginMetadata.value = {
          id: plugin.metadata.id,
          name: plugin.metadata.name,
          version: plugin.metadata.version,
          author: plugin.metadata.author || '',
          mainCategory: inferMainCategory(oldCategory),
          category: convertToSubCategory(oldCategory),
          default_severity: plugin.metadata.default_severity,
          description: plugin.metadata.description || '',
          tagsString: plugin.metadata.tags.join(', ')
        }
        
        isEditing.value = false
        codeError.value = ''
        codeEditorDialog.value?.showModal()
        await initCodeEditor()
      } catch (fileError) {
        console.error('Error reading plugin file:', fileError)
        codeError.value = t('plugins.readCodeError', '读取代码失败')
        showToast(codeError.value, 'error')
      }
    } else {
      codeError.value = response.error || t('plugins.readCodeError', '读取代码失败')
      showToast(codeError.value, 'error')
    }
  } catch (error) {
    console.error('Error reading plugin code:', error)
    codeError.value = error instanceof Error ? error.message : t('plugins.readCodeError', '读取代码失败')
    showToast(codeError.value, 'error')
  }
}

// Close Code Editor Dialog
const closeCodeEditorDialog = () => {
  codeEditorDialog.value?.close()
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  editingPlugin.value = null
  pluginCode.value = ''
  originalCode.value = ''
  isEditing.value = false
  codeError.value = ''
}

// Enable Editing
const enableEditing = () => {
  isEditing.value = true
  updateCodeEditorReadonly(false)
}

// Cancel Editing
const cancelEditing = () => {
  pluginCode.value = originalCode.value
  updateCodeEditorContent(originalCode.value)
  isEditing.value = false
  codeError.value = ''
  updateCodeEditorReadonly(true)
  
  // Restore original metadata if editing existing plugin
  if (editingPlugin.value) {
    const oldCategory = editingPlugin.value.metadata.category
    newPluginMetadata.value = {
      id: editingPlugin.value.metadata.id,
      name: editingPlugin.value.metadata.name,
      version: editingPlugin.value.metadata.version,
      author: editingPlugin.value.metadata.author || '',
      mainCategory: inferMainCategory(oldCategory),
      category: convertToSubCategory(oldCategory),
      default_severity: editingPlugin.value.metadata.default_severity,
      description: editingPlugin.value.metadata.description || '',
      tagsString: editingPlugin.value.metadata.tags.join(', ')
    }
  }
}

// Save Plugin Code
const savePluginCode = async () => {
  if (!editingPlugin.value) return
  
  saving.value = true
  codeError.value = ''
  
  try {
    // Parse tags
    const tags = newPluginMetadata.value.tagsString
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0)
    
    // Map hierarchical category to backend category
    // Agent插件统一映射为 agentTools，被动扫描插件保留子分类
    const backendCategory = newPluginMetadata.value.mainCategory === 'agent' 
      ? 'agentTools' 
      : newPluginMetadata.value.category
    
    // Build metadata comment
    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`
    
    // Remove old metadata comment if exists
    let codeWithoutMetadata = pluginCode.value
    const metadataMatch = codeWithoutMetadata.match(/\/\*\*\s*[\s\S]*?\*\/\s*/)
    if (metadataMatch) {
      codeWithoutMetadata = codeWithoutMetadata.substring(metadataMatch[0].length)
    }
    
    // Combine new metadata with code
    const fullCode = metadataComment + '\n' + codeWithoutMetadata
    
    // 判断是更新还是创建
    // 如果有 editingPlugin，说明是编辑现有插件，使用 update_plugin_code
    // 如果元数据被修改了，需要使用 create_plugin_in_db 来更新完整信息
    const metadataChanged = 
      editingPlugin.value.metadata.name !== newPluginMetadata.value.name ||
      editingPlugin.value.metadata.version !== newPluginMetadata.value.version ||
      editingPlugin.value.metadata.author !== (newPluginMetadata.value.author || 'Unknown') ||
      editingPlugin.value.metadata.description !== (newPluginMetadata.value.description || '') ||
      editingPlugin.value.metadata.default_severity !== newPluginMetadata.value.default_severity ||
      editingPlugin.value.metadata.tags.join(', ') !== tags.join(', ')
    
    let response: CommandResponse<string> | CommandResponse<void>
    
    if (metadataChanged) {
      // 元数据有变化，使用 create_plugin_in_db 完整更新
    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }
    
      response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata: metadata,
      pluginCode: fullCode
    })
    } else {
      // 仅代码有变化，使用 update_plugin_code
      response = await invoke<CommandResponse<void>>('update_plugin_code', {
        pluginId: newPluginMetadata.value.id,
        pluginCode: fullCode
      })
    }
    
    if (response.success) {
      originalCode.value = fullCode
      pluginCode.value = fullCode
      isEditing.value = false
      await refreshPlugins()
      showToast(t('plugins.saveSuccess', '插件保存成功'), 'success')
      
      // 如果被动扫描正在运行，重载插件
      try {
        const reloadResponse = await invoke<CommandResponse<string>>('reload_plugin_in_pipeline', {
          pluginId: newPluginMetadata.value.id
        })
        if (reloadResponse.success) {
          console.log('Plugin reloaded in pipeline:', reloadResponse.data)
          showToast('插件已在被动扫描中热更新', 'info')
        } else {
          console.warn('Failed to reload plugin:', reloadResponse.error)
        }
      } catch (reloadError) {
        console.warn('Plugin reload failed (passive scan may not be running):', reloadError)
      }
    } else {
      codeError.value = response.error || t('plugins.saveFailed', '保存失败')
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : t('plugins.saveFailed', '保存失败')
    console.error('Error saving plugin code:', error)
  } finally {
    saving.value = false
  }
}

// Create New Plugin
const createNewPlugin = async () => {
  saving.value = true
  codeError.value = ''
  
  try {
    const tags = newPluginMetadata.value.tagsString
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0)
    
    // Map hierarchical category to backend category
    // Agent插件统一映射为 agentTools，被动扫描插件保留子分类
    const backendCategory = newPluginMetadata.value.mainCategory === 'agent' 
      ? 'agentTools' 
      : newPluginMetadata.value.category
    
    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`
    
    const fullCode = metadataComment + '\n' + pluginCode.value
    
    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      main_category: newPluginMetadata.value.mainCategory, // 添加主分类字段
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }
    
    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fullCode
    })
    
    if (response.success) {
      closeCodeEditorDialog()
      await refreshPlugins()
      showToast(t('plugins.createSuccess', `插件创建成功: ${newPluginMetadata.value.name}`), 'success')
    } else {
      codeError.value = response.error || t('plugins.createFailed', '创建插件失败')
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : t('plugins.createFailed', '创建插件失败')
    console.error('Error creating plugin:', error)
  } finally {
    saving.value = false
  }
}

// Insert Code Template - 从后端获取模板
const insertTemplate = async () => {
  const isAgentPlugin = newPluginMetadata.value.mainCategory === 'agent'
  
  try {
    // 从后端获取组合模板（包含接口定义和输出格式）
    const templateType = isAgentPlugin ? 'agent' : 'passive'
    
    // 使用 get_combined_plugin_prompt_api 获取完整的模板内容
    const combinedTemplate = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: templateType,
      vulnType: newPluginMetadata.value.category || 'custom',
      severity: newPluginMetadata.value.default_severity || 'medium'
    })
    
    console.log('[Insert Template] Loaded template from backend:', {
      type: templateType,
      length: combinedTemplate.length
    })
    
    // 提取代码示例（如果模板中包含代码块）
    let codeTemplate = ''
    
    // 尝试从模板中提取 TypeScript 代码块（支持多种格式）
    const codeBlockPatterns = [
      /```typescript\n([\s\S]*?)\n```/,
      /```ts\n([\s\S]*?)\n```/,
      /```javascript\n([\s\S]*?)\n```/,
      /```js\n([\s\S]*?)\n```/
    ]
    
    for (const pattern of codeBlockPatterns) {
      const match = combinedTemplate.match(pattern)
      if (match) {
        codeTemplate = match[1].trim()
        break
      }
    }
    
    // 如果没有找到代码块，使用回退模板
    if (!codeTemplate) {
      console.warn('[Insert Template] No code block found in template, using fallback')
      if (isAgentPlugin) {
        codeTemplate = getAgentFallbackTemplate()
      } else {
        codeTemplate = getPassiveFallbackTemplate()
      }
    }
    
    pluginCode.value = codeTemplate
    updateCodeEditorContent(codeTemplate)
    showToast('已插入模板代码', 'success')
  } catch (error) {
    console.error('[Insert Template] Failed to load template from backend:', error)
    // 如果后端加载失败，使用回退模板
    const fallbackTemplate = isAgentPlugin ? getAgentFallbackTemplate() : getPassiveFallbackTemplate()
    pluginCode.value = fallbackTemplate
    updateCodeEditorContent(fallbackTemplate)
    showToast('使用内置模板', 'info')
  }
}

// Agent 插件回退模板
const getAgentFallbackTemplate = () => {
  return `export interface ToolInput {
  [key: string]: any;
}

export interface ToolOutput {
  success: boolean;
  data?: any;
  error?: string;
}

/**
 * Agent工具插件
 * @param input - 工具输入参数（根据需要自定义）
 * @returns 工具执行结果
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    // TODO: 实现你的Agent工具逻辑
    // 例如：
    // - 数据分析工具
    // - 扫描器工具
    // - 报告生成工具
    
    return {
      success: true,
      data: {}
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error'
    };
  }
}

// **CRITICAL**: Export function to globalThis
globalThis.analyze = analyze;`
}

// 被动扫描插件回退模板
const getPassiveFallbackTemplate = () => {
  return `export interface HttpRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body?: string;
}

export interface PluginContext {
  request: HttpRequest;
  response: HttpResponse;
}

export interface Finding {
  title: string;
  description: string;
  severity: 'info' | 'low' | 'medium' | 'high' | 'critical';
  evidence?: string;
  recommendation?: string;
}

/**
 * 被动扫描插件 - 分析HTTP流量
 * @param context - HTTP请求和响应上下文
 * @returns 检测发现列表
 */
export async function analyze(context: PluginContext): Promise<Finding[]> {
  const findings: Finding[] = [];
  
  // TODO: 实现你的被动扫描逻辑
  // 例如：
  // - 漏洞检测
  // - 注入检测
  // - 敏感信息泄露检测
  
  return findings;
}

// **CRITICAL**: Export function to globalThis
globalThis.analyze = analyze;`
}


// Format Code
const formatCode = () => {
  try {
    const lines = pluginCode.value.split('\n')
    let indentLevel = 0
    const formatted = lines.map(line => {
      const trimmed = line.trim()
      
      if (trimmed.startsWith('}') || trimmed.startsWith(']')) {
        indentLevel = Math.max(0, indentLevel - 1)
      }
      
      const indented = '  '.repeat(indentLevel) + trimmed
      
      if (trimmed.endsWith('{') || trimmed.endsWith('[')) {
        indentLevel++
      }
      
      return indented
    })
    
    pluginCode.value = formatted.join('\n')
  } catch (error) {
    console.error('Error formatting code:', error)
  }
}

// Open AI Generate Dialog
const openAIGenerateDialog = () => {
  aiPrompt.value = ''
  aiPluginType.value = 'vulnerability'
  aiSeverity.value = 'medium'
  aiGenerateError.value = ''
  aiGenerateDialog.value?.showModal()
}

// Close AI Generate Dialog
const closeAIGenerateDialog = () => {
  aiGenerateDialog.value?.close()
  aiPrompt.value = ''
  aiGenerateError.value = ''
}

// Fallback templates (used when database templates are not available)
const getFallbackAgentTemplate = () => {
  return `你是一个 Agent 工具插件开发专家。请根据用户的需求生成一个 TypeScript Agent 工具插件代码。

Agent 工具插件应该包含以下接口：
- ToolInput: 自定义输入参数接口 { [key: string]: any }
- ToolOutput: 输出结果接口 { success: boolean; data?: any; error?: string }
- analyze 函数: 接收 ToolInput，返回 Promise<ToolOutput>

插件类型: ${aiPluginType.value}
严重程度: ${aiSeverity.value}

注意：
1. Agent 工具插件不分析 HTTP 请求/响应，而是执行特定的工具任务
2. analyze 函数应该实现具体的工具逻辑（如扫描、分析、报告生成等）
3. 输入参数应该根据工具功能自定义设计
4. 返回结果应该包含工具执行的详细信息

请只返回完整的 TypeScript 代码，不要包含任何解释或 markdown 标记。代码应该可以直接使用。`
}

const getFallbackPassiveTemplate = () => {
  return `你是一个安全插件开发专家。请根据用户的需求生成一个 TypeScript 被动扫描插件代码。

被动扫描插件必须实现标准接口，包括 get_metadata、scan_request、scan_response 函数。

插件类型: ${aiPluginType.value}
严重程度: ${aiSeverity.value}

关键要求：
1. 必须导出函数到 globalThis
2. 包含错误处理
3. 仅在合理置信度时发送发现

请只返回完整的 TypeScript 代码，包含 get_metadata、scan_request、scan_response 函数，以及 globalThis 导出。`
}

// Generate Plugin with AI
const generatePluginWithAI = async () => {
  aiGenerating.value = true
  aiGenerateError.value = ''
  
  try {
    // 判断插件类型
    const isAgentPlugin = aiPluginType.value === 'agentTools' || 
                          ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(aiPluginType.value)
    
    let systemPrompt = ''
    
    // 从数据库获取模板
    try {
      const templateType = isAgentPlugin ? 'agent' : 'passive'
      systemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
        pluginType: templateType,
        vulnType: aiPluginType.value,
        severity: aiSeverity.value
      })
      
      console.log('[Plugin Gen] Loaded template from database:', {
        type: templateType,
        length: systemPrompt.length
      })
    } catch (error) {
      console.warn('[Plugin Gen] Failed to load template from database, using fallback:', error)
      // 如果数据库加载失败，使用回退模板
      if (isAgentPlugin) {
        systemPrompt = getFallbackAgentTemplate()
      } else {
        systemPrompt = getFallbackPassiveTemplate()
      }
    }
    
    const tempConversationId = `plugin_gen_${Date.now()}`
    const userPrompt = `${systemPrompt}\n\n用户需求：${aiPrompt.value}`
    
    let generatedCode = ''
    let streamCompleted = false
    let streamError: string | null = null
    
    const unlisten = await listen('message_chunk', (event: any) => {
      const payload = event.payload
      console.log('[Plugin Gen] Received chunk:', {
        conversation_id: payload.conversation_id,
        chunk_type: payload.chunk_type,
        is_final: payload.is_final,
        content_length: payload.content?.length || 0
      })
      
      if (payload.conversation_id === tempConversationId) {
        if (payload.chunk_type === 'Content' && payload.content) {
          generatedCode += payload.content
        }
        if (payload.is_final) {
          console.log('[Plugin Gen] Stream completed')
          streamCompleted = true
        }
      }
    })
    
    const unlistenError = await listen('ai_stream_error', (event: any) => {
      const payload = event.payload
      if (payload.conversation_id === tempConversationId) {
        streamError = payload.error || payload.message || t('plugins.aiGenerateFailed', 'AI生成失败')
        streamCompleted = true
      }
    })
    
    try {
      console.log('[Plugin Gen] Starting AI generation with conversation_id:', tempConversationId)
      
      await invoke('send_ai_stream_message', {
        request: {
          conversation_id: tempConversationId,
          message: userPrompt,
          service_name: 'default',
        }
      })
      
      console.log('[Plugin Gen] Waiting for stream to complete...')
      const maxWaitTime = 120000 // 增加到 120 秒
      const startTime = Date.now()
      while (!streamCompleted && (Date.now() - startTime < maxWaitTime)) {
        await new Promise(resolve => setTimeout(resolve, 100))
      }
      
      if (streamError) {
        throw new Error(streamError)
      }
      
      if (!streamCompleted) {
        // 如果有生成的代码，即使超时也继续处理
        if (generatedCode.trim()) {
          console.warn('[Plugin Gen] AI response timeout but code was generated, continuing...', {
            code_length: generatedCode.length,
            elapsed: Date.now() - startTime
          })
        } else {
          console.error('[Plugin Gen] Timeout with no code generated', {
            elapsed: Date.now() - startTime,
            streamCompleted,
            streamError
          })
          throw new Error(t('plugins.aiTimeout', 'AI响应超时，未收到任何代码'))
        }
      } else {
        console.log('[Plugin Gen] Stream completed successfully', {
          code_length: generatedCode.length,
          elapsed: Date.now() - startTime
        })
      }
      
      // Clean AI response
      generatedCode = generatedCode.trim()
      generatedCode = generatedCode.replace(/```typescript\n?/g, '')
      generatedCode = generatedCode.replace(/```ts\n?/g, '')
      generatedCode = generatedCode.replace(/```javascript\n?/g, '')
      generatedCode = generatedCode.replace(/```js\n?/g, '')
      generatedCode = generatedCode.replace(/```\n?/g, '')
      generatedCode = generatedCode.trim()
      
      if (!generatedCode) {
        throw new Error(t('plugins.aiNoCode', 'AI未返回任何代码'))
      }
      
      const pluginId = aiPrompt.value
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '_')
        .replace(/^_+|_+$/g, '')
        .substring(0, 50) || 'ai_generated_plugin'
      
      // 根据插件类型设置元数据
      const mainCategory = isAgentPlugin ? 'agent' : 'passive'
      const category = isAgentPlugin ? convertToSubCategory(aiPluginType.value) : aiPluginType.value
      
      newPluginMetadata.value = {
        id: pluginId,
        name: aiPrompt.value.substring(0, 50),
        version: '1.0.0',
        author: 'AI Generated',
        mainCategory: mainCategory,
        category: category,
        default_severity: aiSeverity.value,
        description: aiPrompt.value,
        tagsString: `ai-generated, ${aiPluginType.value}`
      }
      
      pluginCode.value = generatedCode
      editingPlugin.value = null
      
      closeAIGenerateDialog()
      codeEditorDialog.value?.showModal()
      await initCodeEditor()
    } finally {
      unlisten()
      unlistenError()
      
      try {
        await invoke('delete_ai_conversation', { conversationId: tempConversationId })
      } catch (e) {
        console.warn('Failed to cleanup temporary conversation:', e)
      }
    }
  } catch (error) {
    aiGenerateError.value = error instanceof Error ? error.message : t('plugins.aiGenerateFailed', 'AI生成失败')
    console.error('Error generating plugin with AI:', error)
  } finally {
    aiGenerating.value = false
  }
}

// Test Plugin - 根据插件类型选择不同的测试方式
const testPlugin = async (plugin: PluginRecord) => {
  if (!plugin || !plugin.metadata?.id) return
  
  // 判断插件类型
  const isAgentPlugin = plugin.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(plugin.metadata.category)
  
  testing.value = true
  testResult.value = null
  
  try {
    if (isAgentPlugin) {
      // Agent 插件测试：调用 analyze 函数
      await testAgentPlugin(plugin)
    } else {
      // 被动扫描插件测试：调用 scan_request/scan_response
      await testPassiveScanPlugin(plugin)
    }
  } catch (e) {
    console.error('Error testing plugin:', e)
    const msg = e instanceof Error ? e.message : '测试失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  } finally {
    testing.value = false
  }
}

// 测试被动扫描插件（HTTP请求/响应分析）
const testPassiveScanPlugin = async (plugin: PluginRecord) => {
  const resp = await invoke<CommandResponse<TestResult>>('test_plugin', {
    pluginId: plugin.metadata.id
  })
  
  if (resp.success && resp.data) {
    testResult.value = resp.data
    testResultDialog.value?.showModal()
    if (resp.data.success) {
      showToast(t('plugins.testSuccess', `测试完成，发现 ${resp.data.findings?.length || 0} 条结果`), 'success')
    } else {
      showToast(t('plugins.testFailed', resp.data.message || '测试失败'), 'error')
      testResultDialog.value?.showModal()
    }
  } else {
    const msg = resp.error || '测试命令执行失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  }
}

// 测试 Agent 插件（调用 analyze 函数）
const testAgentPlugin = async (plugin: PluginRecord) => {
  // 构造测试输入数据
  const testInput = {
    plugin_id: plugin.metadata.id,
    target: 'https://example.com/test',
    context: {
      test_mode: true,
      description: 'Plugin test execution'
    },
    data: {
      sample_key: 'sample_value'
    }
  }
  
  const resp = await invoke<CommandResponse<any>>('test_execute_plugin_tool', {
    request: testInput
  })
  
  if (resp.success && resp.data) {
    const result = resp.data
    testResult.value = {
      success: result.success,
      message: `插件 '${result.tool_name}' 执行完成\n执行时间: ${result.execution_time_ms}ms`,
      findings: result.output ? [{
        title: 'Agent 工具执行结果',
        description: JSON.stringify(result.output, null, 2),
        severity: result.success ? 'info' : 'error'
      }] : undefined,
      error: result.error
    }
    testResultDialog.value?.showModal()
    
    if (result.success) {
      showToast(t('plugins.testSuccess', `Agent 插件测试完成 (${result.execution_time_ms}ms)`), 'success')
    } else {
      showToast(t('plugins.testFailed', result.error || '测试失败'), 'error')
    }
  } else {
    const msg = resp.error || 'Agent 插件测试命令执行失败'
    showToast(t('plugins.testError', msg), 'error')
    testResult.value = { success: false, message: msg, error: msg }
    testResultDialog.value?.showModal()
  }
}

// Run Advanced Test - 根据插件类型选择不同的测试方式
const runAdvancedTest = async () => {
  if (!advancedPlugin.value) return
  
  // 判断插件类型
  const isAgentPlugin = advancedPlugin.value.metadata.category === 'agentTools' || 
                        ['scanner', 'analyzer', 'reporter', 'recon', 'exploit', 'utility'].includes(advancedPlugin.value.metadata.category)
  
  advancedTesting.value = true
  advancedError.value = ''
  advancedResult.value = null
  
  try {
    if (isAgentPlugin) {
      // Agent 插件不支持高级测试（多次并发测试），使用单次测试
      advancedError.value = 'Agent 插件暂不支持高级并发测试，请使用普通测试功能'
      showToast('Agent 插件暂不支持高级并发测试', 'warning')
      return
    }
    
    // 被动扫描插件的高级测试
    const headersStr = (advancedForm.value.headersText || '').trim()
    if (headersStr.length > 0) {
      try { JSON.parse(headersStr) } catch (e) {
        console.warn('Invalid headers JSON, proceeding with backend defaults:', e)
      }
    }

    const resp = await invoke<CommandResponse<AdvancedTestResult>>('test_plugin_advanced', {
      pluginId: advancedPlugin.value.metadata.id,
      url: advancedForm.value.url || undefined,
      method: advancedForm.value.method || undefined,
      headers: headersStr || undefined,
      body: advancedForm.value.bodyText || undefined,
      runs: Number(advancedForm.value.runs) || 1,
      concurrency: Number(advancedForm.value.concurrency) || 1,
    } as any)

    if (resp.success && resp.data) {
      advancedResult.value = resp.data
      if (resp.data.success) {
        showToast(t('plugins.testSuccess', `测试完成，唯一发现 ${resp.data.unique_findings} 条`), 'success')
      } else {
        showToast(t('plugins.testFailed', resp.data.message || '测试失败'), 'error')
      }
    } else {
      const msg = resp.error || '高级测试命令执行失败'
      advancedError.value = msg
      showToast(t('plugins.testError', msg), 'error')
    }
  } catch (e) {
    console.error('Error advanced testing plugin:', e)
    const msg = e instanceof Error ? e.message : '高级测试失败'
    advancedError.value = msg
    showToast(t('plugins.testError', msg), 'error')
  } finally {
    advancedTesting.value = false
  }
}

// Close Test Result Dialog
const closeTestResultDialog = () => {
  testResultDialog.value?.close()
  testResult.value = null
}

// Get test type label for current testing plugin
const getTestTypeLabel = (): string => {
  // 从 testResult 中推断插件类型（如果有 findings）
  if (testResult.value?.findings && testResult.value.findings.length > 0) {
    const firstFinding = testResult.value.findings[0]
    if (firstFinding.title === 'Agent 工具执行结果') {
      return 'Agent 工具测试'
    }
  }
  return '被动扫描测试'
}

// Confirm Delete Plugin
const confirmDeletePlugin = (plugin: PluginRecord) => {
  deletingPlugin.value = plugin
  deleteDialog.value?.showModal()
}

// Close Delete Dialog
const closeDeleteDialog = () => {
  deleteDialog.value?.close()
  deletingPlugin.value = null
}

// Delete Plugin
const deletePlugin = async () => {
  if (!deletingPlugin.value) return
  
  deleting.value = true
  
  try {
    // 调用后端删除插件命令
    const response = await invoke<CommandResponse<void>>('delete_plugin', {
      pluginId: deletingPlugin.value.metadata.id
    })
    
    if (response.success) {
      const pluginName = deletingPlugin.value.metadata.name
      closeDeleteDialog()
      await refreshPlugins()
      showToast(t('plugins.deleteSuccess', `插件已删除: ${pluginName}`), 'success')
    } else {
      throw new Error(response.error || t('plugins.deleteFailed', '删除失败'))
    }
  } catch (error) {
    console.error('Error deleting plugin:', error)
    showToast(t('plugins.deleteFailed', `删除失败: ${error instanceof Error ? error.message : '未知错误'}`), 'error')
  } finally {
    deleting.value = false
  }
}

// Setup Event Listeners
const setupEventListeners = async () => {
  pluginChangedUnlisten = await listen('plugin:changed', () => {
    refreshPlugins()
  })
}

// Refresh review statistics
const refreshReviewStats = async () => {
  try {
    const response: any = await invoke('get_plugin_review_statistics')
    if (response.success && response.data) {
      reviewStatsData.value = {
        total: response.data.total || 0,
        pending: response.data.pending || 0,
        approved: response.data.approved || 0,
        rejected: response.data.rejected || 0,
        failed: response.data.failed || 0
      }
    }
  } catch (error) {
    console.error('Error loading review statistics:', error)
  }
}

// Review Plugin Methods
const refreshReviewPlugins = async () => {
  try {
    const response: any = await invoke('get_plugins_paginated', {
      page: reviewCurrentPage.value,
      pageSize: reviewPageSize.value,
      statusFilter: reviewStatusFilter.value === 'all' ? null : reviewStatusFilter.value,
      searchText: reviewSearchText.value || null,
      userId: null
    })
    if (response.success && response.data) {
      reviewPlugins.value = Array.isArray(response.data.data) ? response.data.data : []
      reviewTotalCount.value = response.data.total || 0
      reviewTotalPagesCount.value = response.data.total_pages || 0
    } else {
      console.error('Failed to load review plugins:', response.message)
      reviewPlugins.value = []
      reviewTotalCount.value = 0
      reviewTotalPagesCount.value = 0
    }
    
    // 同时刷新统计数据
    await refreshReviewStats()
  } catch (error) {
    console.error('Error loading review plugins:', error)
    showToast(t('plugins.loadReviewError', '加载审核插件失败'), 'error')
    reviewPlugins.value = []
    reviewTotalCount.value = 0
    reviewTotalPagesCount.value = 0
  }
}

const viewReviewPluginDetail = async (plugin: ReviewPlugin) => {
  selectedReviewPlugin.value = plugin
  editedReviewCode.value = plugin.code
  reviewEditMode.value = false
  reviewDetailDialog.value?.showModal()
  await initReviewCodeEditor()
}

const closeReviewDetailDialog = () => {
  reviewDetailDialog.value?.close()
  if (reviewCodeEditorView) {
    reviewCodeEditorView.destroy()
    reviewCodeEditorView = null
  }
  selectedReviewPlugin.value = null
  reviewEditMode.value = false
  editedReviewCode.value = ''
}

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    selectedReviewPlugins.value = []
  } else {
    selectedReviewPlugins.value = [...filteredReviewPlugins.value]
  }
}

const togglePluginSelection = (plugin: ReviewPlugin) => {
  const index = selectedReviewPlugins.value.findIndex(p => p.plugin_id === plugin.plugin_id)
  if (index > -1) {
    selectedReviewPlugins.value.splice(index, 1)
  } else {
    selectedReviewPlugins.value.push(plugin)
  }
}

const isPluginSelected = (plugin: ReviewPlugin) => {
  return selectedReviewPlugins.value.some(p => p.plugin_id === plugin.plugin_id)
}

const approvePlugin = async (plugin: ReviewPlugin) => {
  if (!plugin) return
  
  try {
    const response: any = await invoke('approve_plugin', {
      pluginId: plugin.plugin_id
    })
    
    if (response.success) {
      plugin.status = 'Approved'
      await refreshReviewPlugins()
      await refreshPlugins() // 同时刷新主插件列表
      showToast(t('plugins.approveSuccess', `插件已批准: ${plugin.plugin_name}`), 'success')
      closeReviewDetailDialog()
    } else {
      showToast(t('plugins.approveFailed', `批准失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error approving plugin:', error)
    showToast(t('plugins.approveFailed', '批准失败'), 'error')
  }
}

const rejectPlugin = async (plugin: ReviewPlugin) => {
  if (!plugin) return
  
  try {
    const response: any = await invoke('reject_plugin', {
      pluginId: plugin.plugin_id,
      reason: 'Manual rejection'
    })
    
    if (response.success) {
      plugin.status = 'Rejected'
      await refreshReviewPlugins()
      await refreshPlugins() // 同时刷新主插件列表
      showToast(t('plugins.rejectSuccess', `插件已拒绝: ${plugin.plugin_name}`), 'success')
      closeReviewDetailDialog()
    } else {
      showToast(t('plugins.rejectFailed', `拒绝失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error rejecting plugin:', error)
    showToast(t('plugins.rejectFailed', '拒绝失败'), 'error')
  }
}

const approveSelected = async () => {
  if (selectedReviewPlugins.value.length === 0) return
  
  try {
    const pluginIds = selectedReviewPlugins.value.map(p => p.plugin_id)
    const response: any = await invoke('batch_approve_plugins', {
      pluginIds
    })
    
    if (response.success) {
      await refreshReviewPlugins()
      await refreshPlugins() // 同时刷新主插件列表
      selectedReviewPlugins.value = []
      showToast(t('plugins.batchApproveSuccess', `已批准 ${pluginIds.length} 个插件`), 'success')
    } else {
      showToast(t('plugins.batchApproveFailed', `批量批准失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error batch approving plugins:', error)
    showToast(t('plugins.batchApproveFailed', '批量批准失败'), 'error')
  }
}

const rejectSelected = async () => {
  if (selectedReviewPlugins.value.length === 0) return
  
  try {
    const pluginIds = selectedReviewPlugins.value.map(p => p.plugin_id)
    const response: any = await invoke('batch_reject_plugins', {
      pluginIds,
      reason: 'Batch rejection'
    })
    
    if (response.success) {
      await refreshReviewPlugins()
      await refreshPlugins() // 同时刷新主插件列表
      selectedReviewPlugins.value = []
      showToast(t('plugins.batchRejectSuccess', `已拒绝 ${pluginIds.length} 个插件`), 'success')
    } else {
      showToast(t('plugins.batchRejectFailed', `批量拒绝失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error batch rejecting plugins:', error)
    showToast(t('plugins.batchRejectFailed', '批量拒绝失败'), 'error')
  }
}

const deleteReviewPlugin = async (plugin: ReviewPlugin) => {
  if (!plugin) return
  
  try {
    const response: any = await invoke('review_delete_plugin', {
      pluginId: plugin.plugin_id
    })
    
    if (response.success) {
      await refreshReviewPlugins()
      await refreshPlugins() // 同时刷新主插件列表
      showToast(t('plugins.deleteSuccess', `插件已删除: ${plugin.plugin_name}`), 'success')
    } else {
      showToast(t('plugins.deleteFailed', `删除失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error deleting review plugin:', error)
    showToast(t('plugins.deleteFailed', '删除失败'), 'error')
  }
}

const copyReviewCode = () => {
  if (selectedReviewPlugin.value) {
    navigator.clipboard.writeText(selectedReviewPlugin.value.code)
    showToast(t('plugins.codeCopied', '代码已复制到剪贴板'), 'success')
  }
}

const saveReviewEdit = async () => {
  if (!selectedReviewPlugin.value) return
  
  savingReview.value = true
  
  try {
    const response: any = await invoke('review_update_plugin_code', {
      pluginId: selectedReviewPlugin.value.plugin_id,
      code: editedReviewCode.value
    })
    
    if (response.success) {
      selectedReviewPlugin.value.code = editedReviewCode.value
      reviewEditMode.value = false
      await refreshReviewPlugins()
      showToast(t('plugins.saveSuccess', '代码已保存'), 'success')
    } else {
      showToast(t('plugins.saveFailed', `保存失败: ${response.message || '未知错误'}`), 'error')
    }
  } catch (error) {
    console.error('Error saving review plugin code:', error)
    showToast(t('plugins.saveFailed', '保存失败'), 'error')
  } finally {
    savingReview.value = false
  }
}

const getReviewStatusText = (status: string): string => {
  const statusMap: Record<string, string> = {
    'PendingReview': t('plugins.pendingReview', '待审核'),
    'Approved': t('plugins.approved', '已批准'),
    'Rejected': t('plugins.rejected', '已拒绝'),
    'ValidationFailed': t('plugins.validationFailed', '验证失败')
  }
  return statusMap[status] || status
}

// Watch reviewEditMode to update editor readonly state
watch(reviewEditMode, (newValue) => {
  updateReviewCodeEditorReadonly(!newValue)
})

// Component Lifecycle
onMounted(async () => {
  await refreshPlugins()
  await refreshReviewPlugins()
  await setupEventListeners()
})

onUnmounted(() => {
  if (pluginChangedUnlisten) {
    pluginChangedUnlisten()
  }
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  if (reviewCodeEditorView) {
    reviewCodeEditorView.destroy()
    reviewCodeEditorView = null
  }
})
</script>

<style scoped>
.tabs {
  max-width: 100%;
}

.tab {
  flex-shrink: 0;
}

.table th {
  background-color: hsl(var(--b2));
}

textarea.textarea {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  line-height: 1.5;
  tab-size: 2;
}

.badge {
  font-weight: 600;
}

/* CodeMirror 样式 */
:deep(.cm-editor) {
  height: 600px;
  font-size: 12px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

:deep(.cm-scroller) {
  overflow: auto;
}

:deep(.cm-content) {
  padding: 10px 0;
}

:deep(.cm-line) {
  padding: 0 4px;
}

:deep(.cm-gutters) {
  background-color: #282c34;
  color: #5c6370;
  border: none;
}

:deep(.cm-activeLineGutter) {
  background-color: #2c313c;
}

:deep(.cm-activeLine) {
  background-color: #2c313c;
}
</style>
