<template>
  <div class="container  p-6 ">
    <!-- 头部控制栏 -->
    <div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-8">
      <div class="flex items-center gap-4">
        <h1 class="text-3xl font-bold text-base-content">性能监控</h1>
        <div class="badge badge-lg" :class="isMonitoring ? 'badge-success' : 'badge-neutral'">
          <div v-if="isMonitoring" class="w-2 h-2 bg-success rounded-full mr-2 animate-pulse"></div>
          {{ isMonitoring ? '监控中' : '未启动' }}
        </div>
      </div>
      <div class="flex flex-wrap gap-2">
        <button 
          class="btn btn-outline btn-sm"
          @click="toggleAutoRefresh"
          :class="{ 'btn-active': autoRefresh }"
        >
          <svg v-if="autoRefresh" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          自动刷新
        </button>
        <button 
          class="btn btn-outline btn-sm" 
          @click="refreshAll" 
          :disabled="loading"
        >
          <svg v-if="loading" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          刷新
        </button>
        <button 
          class="btn btn-primary btn-sm" 
          @click="startMonitoring" 
          :disabled="isMonitoring || loading"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
          </svg>
          启动监控
        </button>
      </div>
    </div>

    <!-- 加载状态 -->
    <div v-if="!metrics || !config" class="flex justify-center items-center min-h-[400px]">
      <div class="text-center">
        <span class="loading loading-spinner loading-lg text-primary"></span>
        <p class="mt-4 text-base-content/70">加载性能数据中...</p>
      </div>
    </div>

    <!-- 主要内容 -->
    <div v-else class="space-y-6">
      <!-- 标签页 -->
      <div class="tabs tabs-boxed bg-base-200 p-1">
        <button 
          v-for="tab in tabs" 
          :key="tab.id"
          class="tab tab-lg"
          :class="{ 'tab-active': activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <svg v-if="tab.id === 'overview'" class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
          </svg>
          <svg v-else-if="tab.id === 'details'" class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
          </svg>
          <svg v-else-if="tab.id === 'suggestions'" class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
          </svg>
          <svg v-else-if="tab.id === 'report'" class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
          </svg>
          {{ tab.label }}
        </button>
      </div>

      <!-- 概览标签页 -->
      <div v-if="activeTab === 'overview'" class="space-y-6">
        <!-- 系统资源指标 -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
          <!-- 内存使用 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">内存使用</h3>
                <div class="w-8 h-8 bg-primary/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.memory_usage_mb.toFixed(1) }} MB</div>
                <progress 
                  class="progress w-full" 
                  :class="getProgressClass(metrics.memory_usage_mb, config.memory_threshold_mb)"
                  :value="Math.min((metrics.memory_usage_mb / config.memory_threshold_mb) * 100, 100)" 
                  max="100"
                ></progress>
                <div class="text-xs text-base-content/60">阈值: {{ config.memory_threshold_mb }} MB</div>
              </div>
            </div>
          </div>

          <!-- CPU使用率 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">CPU使用率</h3>
                <div class="w-8 h-8 bg-secondary/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.cpu_usage_percent.toFixed(1) }}%</div>
                <progress 
                  class="progress w-full" 
                  :class="getProgressClass(metrics.cpu_usage_percent, config.cpu_threshold_percent)"
                  :value="Math.min(metrics.cpu_usage_percent, 100)" 
                  max="100"
                ></progress>
                <div class="text-xs text-base-content/60">阈值: {{ config.cpu_threshold_percent }}%</div>
              </div>
            </div>
          </div>

          <!-- 活跃任务 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">活跃任务</h3>
                <div class="w-8 h-8 bg-accent/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.active_tasks }}</div>
                <div class="text-xs text-base-content/60">最大并发: {{ config.max_concurrent_scans }}</div>
              </div>
            </div>
          </div>

          <!-- 响应时间 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">响应时间</h3>
                <div class="w-8 h-8 bg-info/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-info" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4h16v12a1 1 0 01-1 1H5a1 1 0 01-1-1V4z"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.avg_response_time_ms.toFixed(1) }} ms</div>
                <div class="text-xs text-base-content/60">吞吐量: {{ metrics.throughput_rps.toFixed(2) }} req/s</div>
              </div>
            </div>
          </div>

          <!-- 磁盘使用 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">磁盘使用</h3>
                <div class="w-8 h-8 bg-warning/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-warning" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.disk_usage_mb.toFixed(1) }} MB</div>
              </div>
            </div>
          </div>

          <!-- 网络IO -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">网络IO</h3>
                <div class="w-8 h-8 bg-success/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9v-9m0-9v9m0 9c-5 0-9-4-9-9s4-9 9-9"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ formatBytes(metrics.network_io_bps) }}/s</div>
              </div>
            </div>
          </div>

          <!-- 数据库连接 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">数据库连接</h3>
                <div class="w-8 h-8 bg-error/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-error" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.db_connections }}</div>
                <div class="text-xs text-base-content/60">连接池: {{ config.connection_pool_size }}</div>
              </div>
            </div>
          </div>

          <!-- 缓存命中率 -->
          <div class="card bg-base-100 shadow-lg">
            <div class="card-body p-6">
              <div class="flex items-center justify-between mb-4">
                <h3 class="card-title text-sm font-medium text-base-content/70">缓存命中率</h3>
                <div class="w-8 h-8 bg-success/10 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                  </svg>
                </div>
              </div>
              <div class="space-y-3">
                <div class="text-2xl font-bold text-base-content">{{ metrics.cache_hit_rate.toFixed(1) }}%</div>
                <progress 
                  class="progress progress-success w-full" 
                  :value="Math.min(metrics.cache_hit_rate, 100)" 
                  max="100"
                ></progress>
              </div>
            </div>
          </div>
        </div>

        <!-- 错误率 -->
        <div class="card bg-base-100 shadow-lg">
          <div class="card-body p-6">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 bg-error/10 rounded-lg flex items-center justify-center">
                <svg class="w-6 h-6 text-error" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                </svg>
              </div>
              <h3 class="text-xl font-bold text-base-content">错误率</h3>
            </div>
            <div class="space-y-4">
              <div class="text-3xl font-bold text-error">{{ metrics.error_rate_percent.toFixed(2) }}%</div>
              <progress 
                class="progress progress-error w-full" 
                :value="Math.min(metrics.error_rate_percent, 100)" 
                max="100"
              ></progress>
              <div class="text-sm text-base-content/60">系统错误率监控</div>
            </div>
          </div>
        </div>
      </div>

      <!-- 详细信息标签页 -->
      <div v-if="activeTab === 'details'" class="space-y-6">
        <div class="card bg-base-100 shadow-lg">
          <div class="card-body">
            <div class="flex items-center gap-3 mb-6">
              <div class="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center">
                <svg class="w-6 h-6 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                </svg>
              </div>
              <div>
                <h2 class="text-2xl font-bold text-base-content">详细配置</h2>
                <p class="text-base-content/60">当前性能监控配置参数</p>
              </div>
            </div>
            
            <div class="overflow-x-auto">
              <table class="table table-zebra w-full">
                <thead>
                  <tr>
                    <th class="text-base-content/70">配置项</th>
                    <th class="text-base-content/70">当前值</th>
                    <th class="text-base-content/70">描述</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td class="font-medium">最大并发扫描数</td>
                    <td><span class="badge badge-primary">{{ config.max_concurrent_scans }}</span></td>
                    <td class="text-base-content/60">同时运行的最大扫描任务数</td>
                  </tr>
                  <tr>
                    <td class="font-medium">内存阈值</td>
                    <td><span class="badge badge-secondary">{{ config.memory_threshold_mb }} MB</span></td>
                    <td class="text-base-content/60">内存使用警告阈值</td>
                  </tr>
                  <tr>
                    <td class="font-medium">CPU阈值</td>
                    <td><span class="badge badge-accent">{{ config.cpu_threshold_percent }}%</span></td>
                    <td class="text-base-content/60">CPU使用率警告阈值</td>
                  </tr>
                  <tr>
                    <td class="font-medium">自动优化</td>
                    <td>
                      <div class="badge" :class="config.auto_optimization ? 'badge-success' : 'badge-error'">
                        {{ config.auto_optimization ? '启用' : '禁用' }}
                      </div>
                    </td>
                    <td class="text-base-content/60">是否启用自动性能优化</td>
                  </tr>
                  <tr>
                    <td class="font-medium">缓存大小</td>
                    <td><span class="badge badge-info">{{ config.cache_size_mb }} MB</span></td>
                    <td class="text-base-content/60">系统缓存大小限制</td>
                  </tr>
                  <tr>
                    <td class="font-medium">连接池大小</td>
                    <td><span class="badge badge-warning">{{ config.connection_pool_size }}</span></td>
                    <td class="text-base-content/60">数据库连接池大小</td>
                  </tr>
                  <tr>
                    <td class="font-medium">监控间隔</td>
                    <td><span class="badge badge-neutral">{{ config.monitoring_interval_secs }} 秒</span></td>
                    <td class="text-base-content/60">性能数据采集间隔</td>
                  </tr>
                </tbody>
              </table>
            </div>
            
            <div class="flex flex-wrap gap-3 mt-6">
              <button class="btn btn-outline btn-sm" @click="resetStats">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                </svg>
                重置统计
              </button>
              <button class="btn btn-outline btn-sm">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                </svg>
                配置设置
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 优化建议标签页 -->
      <div v-if="activeTab === 'suggestions'" class="space-y-6">
        <div class="card bg-base-100 shadow-lg">
          <div class="card-body">
            <div class="flex items-center gap-3 mb-6">
              <div class="w-10 h-10 bg-warning/10 rounded-lg flex items-center justify-center">
                <svg class="w-6 h-6 text-warning" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                </svg>
              </div>
              <div>
                <h2 class="text-2xl font-bold text-base-content">性能优化建议</h2>
                <p class="text-base-content/60">基于当前性能数据的智能优化建议</p>
              </div>
            </div>
            
            <div v-if="suggestions.length > 0" class="space-y-4">
              <div v-for="(suggestion, index) in suggestions" :key="index" class="alert alert-info shadow-lg">
                <div class="flex items-start gap-3">
                  <div class="badge badge-info badge-lg">{{ index + 1 }}</div>
                  <div class="flex-1">
                    <div class="flex items-center gap-2 mb-2">
                      <svg class="w-5 h-5 text-info" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                      </svg>
                      <h4 class="font-bold text-info">优化建议</h4>
                    </div>
                    <p class="text-info/80">{{ suggestion }}</p>
                    <div class="mt-3">
                      <button class="btn btn-info btn-sm">
                        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                        </svg>
                        应用建议
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            
            <div v-else class="text-center py-12">
              <div class="w-16 h-16 bg-success/10 rounded-full flex items-center justify-center mx-auto mb-4">
                <svg class="w-8 h-8 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
              </div>
              <h3 class="text-xl font-bold text-base-content mb-2">系统性能良好</h3>
              <p class="text-base-content/60">当前系统运行状态良好，暂无优化建议</p>
            </div>
            
            <!-- 自动优化状态 -->
            <div class="card bg-base-200 border border-base-300 mt-6">
              <div class="card-body p-4">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div class="w-8 h-8 bg-success/10 rounded-lg flex items-center justify-center">
                      <svg class="w-5 h-5 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                    </div>
                    <div>
                      <h4 class="font-bold text-base-content">自动优化</h4>
                      <p class="text-base-content/60 text-sm">系统将自动应用优化建议</p>
                    </div>
                  </div>
                  <input type="checkbox" class="toggle toggle-success" :checked="config?.auto_optimization" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 性能报告标签页 -->
      <div v-if="activeTab === 'report'" class="space-y-6">
        <div class="card bg-base-100 shadow-lg">
          <div class="card-body">
            <div class="flex items-center gap-3 mb-6">
              <div class="w-10 h-10 bg-info/10 rounded-lg flex items-center justify-center">
                <svg class="w-6 h-6 text-info" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                </svg>
              </div>
              <div>
                <h2 class="text-2xl font-bold text-base-content">性能报告</h2>
                <p class="text-base-content/60">系统性能详细分析报告</p>
              </div>
            </div>
            
            <!-- 报告统计 -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
              <div class="stat bg-primary/5 rounded-lg">
                <div class="stat-figure text-primary">
                  <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"></path>
                  </svg>
                </div>
                <div class="stat-title text-primary/70">系统状态</div>
                <div class="stat-value text-primary text-lg">良好</div>
                <div class="stat-desc text-primary/60">运行稳定</div>
              </div>
              
              <div class="stat bg-secondary/5 rounded-lg">
                <div class="stat-figure text-secondary">
                  <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                  </svg>
                </div>
                <div class="stat-title text-secondary/70">运行时间</div>
                <div class="stat-value text-secondary text-lg">24h</div>
                <div class="stat-desc text-secondary/60">持续监控</div>
              </div>
              
              <div class="stat bg-accent/5 rounded-lg">
                <div class="stat-figure text-accent">
                  <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                  </svg>
                </div>
                <div class="stat-title text-accent/70">处理任务</div>
                <div class="stat-value text-accent text-lg">{{ metrics.active_tasks }}</div>
                <div class="stat-desc text-accent/60">当前活跃</div>
              </div>
              
              <div class="stat bg-success/5 rounded-lg">
                <div class="stat-figure text-success">
                  <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                  </svg>
                </div>
                <div class="stat-title text-success/70">成功率</div>
                <div class="stat-value text-success text-lg">{{ (100 - metrics.error_rate_percent).toFixed(1) }}%</div>
                <div class="stat-desc text-success/60">处理成功</div>
              </div>
            </div>
            
            <!-- 详细报告内容 -->
            <div class="space-y-6">
              <!-- 系统概览 -->
              <div class="card bg-base-200 border border-base-300">
                <div class="card-body p-6">
                  <div class="flex items-center gap-3 mb-4">
                    <div class="w-8 h-8 bg-primary/10 rounded-lg flex items-center justify-center">
                      <svg class="w-5 h-5 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                      </svg>
                    </div>
                    <h3 class="text-lg font-bold text-base-content">系统概览</h3>
                  </div>
                  <p class="text-base-content/70">当前系统运行状态良好，各项指标均在正常范围内。内存使用率为 {{ metrics.memory_usage_mb.toFixed(1) }} MB，CPU使用率为 {{ metrics.cpu_usage_percent.toFixed(1) }}%，系统响应时间平均为 {{ metrics.avg_response_time_ms.toFixed(1) }}ms。</p>
                </div>
              </div>
              
              <!-- 性能趋势 -->
              <div class="card bg-base-200 border border-base-300">
                <div class="card-body p-6">
                  <div class="flex items-center gap-3 mb-4">
                    <div class="w-8 h-8 bg-secondary/10 rounded-lg flex items-center justify-center">
                      <svg class="w-5 h-5 text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"></path>
                      </svg>
                    </div>
                    <h3 class="text-lg font-bold text-base-content">性能趋势</h3>
                  </div>
                  <p class="text-base-content/70">过去24小时内，系统性能保持稳定，无明显波动。吞吐量维持在 {{ metrics.throughput_rps.toFixed(2) }} 请求/秒，错误率控制在 {{ metrics.error_rate_percent.toFixed(2) }}% 以下。</p>
                </div>
              </div>
              
              <!-- 资源使用详情 -->
              <div class="card bg-base-200 border border-base-300">
                <div class="card-body p-6">
                  <div class="flex items-center gap-3 mb-4">
                    <div class="w-8 h-8 bg-accent/10 rounded-lg flex items-center justify-center">
                      <svg class="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
                      </svg>
                    </div>
                    <h3 class="text-lg font-bold text-base-content">资源使用详情</h3>
                  </div>
                  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div class="bg-base-100 p-4 rounded-lg">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-base-content/70">内存使用</span>
                        <span class="font-bold">{{ metrics.memory_usage_mb.toFixed(1) }} MB</span>
                      </div>
                      <progress class="progress progress-primary w-full" :value="Math.min((metrics.memory_usage_mb / config.memory_threshold_mb) * 100, 100)" max="100"></progress>
                    </div>
                    <div class="bg-base-100 p-4 rounded-lg">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-base-content/70">CPU使用率</span>
                        <span class="font-bold">{{ metrics.cpu_usage_percent.toFixed(1) }}%</span>
                      </div>
                      <progress class="progress progress-secondary w-full" :value="metrics.cpu_usage_percent" max="100"></progress>
                    </div>
                    <div class="bg-base-100 p-4 rounded-lg">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-base-content/70">磁盘使用</span>
                        <span class="font-bold">{{ metrics.disk_usage_mb.toFixed(1) }} MB</span>
                      </div>
                      <progress class="progress progress-accent w-full" :value="50" max="100"></progress>
                    </div>
                  </div>
                </div>
              </div>
              
              <!-- 建议措施 -->
              <div class="card bg-base-200 border border-base-300">
                <div class="card-body p-6">
                  <div class="flex items-center gap-3 mb-4">
                    <div class="w-8 h-8 bg-warning/10 rounded-lg flex items-center justify-center">
                      <svg class="w-5 h-5 text-warning" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                      </svg>
                    </div>
                    <h3 class="text-lg font-bold text-base-content">建议措施</h3>
                  </div>
                  <div class="space-y-3">
                    <div class="flex items-start gap-3">
                      <div class="w-2 h-2 bg-warning rounded-full mt-2"></div>
                      <p class="text-base-content/70">建议定期清理缓存，当前缓存命中率为 {{ metrics.cache_hit_rate.toFixed(1) }}%</p>
                    </div>
                    <div class="flex items-start gap-3">
                      <div class="w-2 h-2 bg-warning rounded-full mt-2"></div>
                      <p class="text-base-content/70">优化数据库查询，当前数据库连接数为 {{ metrics.db_connections }}</p>
                    </div>
                    <div class="flex items-start gap-3">
                      <div class="w-2 h-2 bg-warning rounded-full mt-2"></div>
                      <p class="text-base-content/70">持续监控系统资源使用情况，确保服务稳定性</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- 操作按钮 -->
            <div class="flex flex-wrap gap-3 mt-6">
              <button class="btn btn-primary btn-sm" @click="fetchReport">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                </svg>
                刷新报告
              </button>
              <button class="btn btn-outline btn-sm">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                </svg>
                导出报告
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface PerformanceMetrics {
  memory_usage_mb: number
  cpu_usage_percent: number
  active_tasks: number
  avg_response_time_ms: number
  error_rate_percent: number
  throughput_rps: number
  disk_usage_mb: number
  network_io_bps: number
  db_connections: number
  cache_hit_rate: number
}

interface PerformanceConfig {
  max_concurrent_scans: number
  memory_threshold_mb: number
  cpu_threshold_percent: number
  auto_optimization: boolean
  cache_size_mb: number
  connection_pool_size: number
  monitoring_interval_secs: number
}

// 响应式数据
const metrics = ref<PerformanceMetrics | null>(null)
const config = ref<PerformanceConfig | null>(null)
const suggestions = ref<string[]>([])
const report = ref<string>('')
const isMonitoring = ref(false)
const loading = ref(false)
const autoRefresh = ref(true)
const activeTab = ref('overview')

// 标签页配置
const tabs = [
  { id: 'overview', label: '概览' },
  { id: 'details', label: '详细信息' },
  { id: 'suggestions', label: '优化建议' },
  { id: 'report', label: '性能报告' }
]

// 自动刷新定时器
let refreshInterval: number | null = null

// 获取性能指标
const fetchMetrics = async () => {
  try {
    const data = await invoke<PerformanceMetrics>('get_performance_metrics')
    metrics.value = data
  } catch (error) {
    console.error('Failed to fetch performance metrics:', error)
  }
}

// 获取性能配置
const fetchConfig = async () => {
  try {
    const data = await invoke<PerformanceConfig>('get_performance_config')
    config.value = data
  } catch (error) {
    console.error('Failed to fetch performance config:', error)
  }
}

// 获取优化建议
const fetchSuggestions = async () => {
  try {
    const data = await invoke<string[]>('get_optimization_suggestions')
    suggestions.value = data
  } catch (error) {
    console.error('Failed to fetch optimization suggestions:', error)
  }
}

// 获取性能报告
const fetchReport = async () => {
  try {
    const data = await invoke<string>('get_performance_report')
    report.value = data
  } catch (error) {
    console.error('Failed to fetch performance report:', error)
  }
}

// 启动性能监控
const startMonitoring = async () => {
  loading.value = true
  try {
    await invoke('start_performance_monitoring')
    isMonitoring.value = true
    console.log('性能监控已启动')
  } catch (error) {
    console.error('Failed to start performance monitoring:', error)
  } finally {
    loading.value = false
  }
}

// 重置统计数据
const resetStats = async () => {
  try {
    await invoke('reset_performance_stats')
    await fetchMetrics()
    console.log('统计数据已重置')
  } catch (error) {
    console.error('Failed to reset performance stats:', error)
  }
}

// 刷新所有数据
const refreshAll = async () => {
  loading.value = true
  try {
    await Promise.all([
      fetchMetrics(),
      fetchConfig(),
      fetchSuggestions(),
      fetchReport()
    ])
  } finally {
    loading.value = false
  }
}

// 切换自动刷新
const toggleAutoRefresh = () => {
  autoRefresh.value = !autoRefresh.value
  if (autoRefresh.value) {
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
}

// 启动自动刷新
const startAutoRefresh = () => {
  if (refreshInterval) return
  refreshInterval = window.setInterval(() => {
    fetchMetrics()
    fetchSuggestions()
  }, 5000) // 每5秒刷新一次
}

// 停止自动刷新
const stopAutoRefresh = () => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
    refreshInterval = null
  }
}

// 获取进度条样式类
const getProgressClass = (value: number, threshold: number) => {
  const percentage = (value / threshold) * 100
  if (percentage <= 70) return 'progress-success'
  if (percentage <= 90) return 'progress-warning'
  return 'progress-error'
}

// 格式化字节数
const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// 组件挂载时初始化
onMounted(() => {
  refreshAll()
  if (autoRefresh.value) {
    startAutoRefresh()
  }
})

// 组件卸载时清理
onUnmounted(() => {
  stopAutoRefresh()
})
</script>

<style scoped>
/* 保留一些必要的自定义样式 */
.tab-content {
  animation: fadeIn 0.3s ease-in-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 确保图标正确显示 */
.icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>