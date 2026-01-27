<template>
  <div class="space-y-4">
    <!-- Scheduler Status Card -->
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-3">
            <div 
              class="w-12 h-12 rounded-lg flex items-center justify-center"
              :class="schedulerRunning ? 'bg-success/20 text-success' : 'bg-base-300 text-base-content/50'"
            >
              <i :class="schedulerRunning ? 'fas fa-play-circle' : 'fas fa-pause-circle'" class="text-2xl"></i>
            </div>
            <div>
              <h3 class="font-semibold">{{ t('bugBounty.monitor.schedulerTitle') }}</h3>
              <div class="text-sm text-base-content/60">
                <span v-if="schedulerRunning" class="text-success">
                  <i class="fas fa-circle text-xs mr-1 animate-pulse"></i>
                  {{ t('bugBounty.monitor.running') }}
                </span>
                <span v-else class="text-base-content/50">
                  <i class="fas fa-circle text-xs mr-1"></i>
                  {{ t('bugBounty.monitor.stopped') }}
                </span>
              </div>
            </div>
          </div>
          <div class="flex gap-2">
            <button 
              v-if="!schedulerRunning"
              class="btn btn-success btn-sm"
              @click="startScheduler"
              :disabled="starting"
            >
              <i class="fas fa-play mr-2"></i>
              {{ t('bugBounty.monitor.start') }}
            </button>
            <button 
              v-else
              class="btn btn-error btn-sm"
              @click="stopScheduler"
              :disabled="stopping"
            >
              <i class="fas fa-stop mr-2"></i>
              {{ t('bugBounty.monitor.stop') }}
            </button>
            <button class="btn btn-ghost btn-sm" @click="refreshStats">
              <i class="fas fa-sync-alt"></i>
            </button>
          </div>
        </div>

        <!-- Stats -->
        <div v-if="stats" class="grid grid-cols-4 gap-4 mt-4">
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.monitor.totalTasks') }}</div>
            <div class="stat-value text-lg">{{ stats.total_tasks }}</div>
            <div class="stat-desc">{{ stats.active_tasks }} {{ t('bugBounty.monitor.active') }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.monitor.totalRuns') }}</div>
            <div class="stat-value text-lg">{{ stats.total_runs }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.monitor.eventsDetected') }}</div>
            <div class="stat-value text-lg text-warning">{{ stats.total_events }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg p-3">
            <div class="stat-title text-xs">{{ t('bugBounty.monitor.uptime') }}</div>
            <div class="stat-value text-lg text-sm">{{ formatUptime(stats.scheduler_uptime_secs) }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Monitor Tasks -->
    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">{{ t('bugBounty.monitor.tasks') }}</h3>
          <div class="flex gap-2">
            <button 
              v-if="selectedProgram"
              class="btn btn-sm btn-outline"
              @click="createDefaultTasks"
            >
              <i class="fas fa-magic mr-2"></i>
              {{ t('bugBounty.monitor.createDefault') }}
            </button>
            <button class="btn btn-sm btn-primary" @click="openCreateModal">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.monitor.createTask') }}
            </button>
          </div>
        </div>

        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="tasks.length === 0" class="text-center py-8">
          <i class="fas fa-tasks text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.monitor.noTasks') }}</p>
          <button 
            v-if="selectedProgram"
            class="btn btn-primary btn-sm mt-4"
            @click="createDefaultTasks"
          >
            {{ t('bugBounty.monitor.createDefaultTasks') }}
          </button>
        </div>

        <div v-else class="space-y-3">
          <div 
            v-for="task in tasks" 
            :key="task.id"
            class="card bg-base-200 hover:bg-base-300 transition-colors"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div class="flex items-center gap-3 flex-1">
                  <input 
                    type="checkbox" 
                    class="toggle toggle-success" 
                    :checked="task.enabled"
                    @change="toggleTask(task)"
                  />
                  <div class="flex-1">
                    <div class="flex items-center gap-2">
                      <h4 class="font-medium">{{ task.name }}</h4>
                      <span v-if="task.enabled" class="badge badge-success badge-xs">
                        {{ t('bugBounty.monitor.enabled') }}
                      </span>
                      <span v-else class="badge badge-ghost badge-xs">
                        {{ t('bugBounty.monitor.disabled') }}
                      </span>
                    </div>
                    <div class="text-xs text-base-content/60 mt-1 space-y-1">
                      <div>
                        <i class="fas fa-clock mr-1"></i>
                        {{ t('bugBounty.monitor.interval') }}: {{ formatInterval(task.interval_secs) }}
                      </div>
                      <div v-if="task.next_run_at">
                        <i class="fas fa-calendar-alt mr-1"></i>
                        {{ t('bugBounty.monitor.nextRun') }}: {{ formatDateTime(task.next_run_at) }}
                      </div>
                      <div>
                        <i class="fas fa-chart-line mr-1"></i>
                        {{ task.run_count }} {{ t('bugBounty.monitor.runs') }}, 
                        {{ task.events_detected }} {{ t('bugBounty.monitor.events') }}
                      </div>
                    </div>
                  </div>
                </div>
                <div class="flex gap-1">
                  <button 
                    class="btn btn-primary btn-xs"
                    @click="discoverAssets(task)"
                    :title="t('bugBounty.monitor.discoverAssets')"
                  >
                    <i class="fas fa-search"></i>
                  </button>
                  <button 
                    class="btn btn-ghost btn-xs"
                    @click="triggerTask(task)"
                    :title="t('bugBounty.monitor.runNow')"
                  >
                    <i class="fas fa-play"></i>
                  </button>
                  <button 
                    class="btn btn-ghost btn-xs"
                    @click="editTask(task)"
                    :title="t('common.edit')"
                  >
                    <i class="fas fa-edit"></i>
                  </button>
                  <button 
                    class="btn btn-ghost btn-xs text-error"
                    @click="deleteTask(task)"
                    :title="t('common.delete')"
                  >
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>

              <!-- Monitor Config Summary -->
              <div class="flex flex-wrap gap-2 mt-2">
                <span v-if="task.config.enable_dns_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-network-wired mr-1"></i>DNS
                </span>
                <span v-if="task.config.enable_cert_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-certificate mr-1"></i>{{ t('bugBounty.monitor.cert') }}
                </span>
                <span v-if="task.config.enable_content_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-file-alt mr-1"></i>{{ t('bugBounty.monitor.content') }}
                </span>
                <span v-if="task.config.enable_api_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-plug mr-1"></i>API
                </span>
                <span v-if="task.config.enable_port_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-network-wired mr-1"></i>Port
                </span>
                <span v-if="task.config.enable_web_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-globe mr-1"></i>Web
                </span>
                <span v-if="task.config.enable_vuln_monitoring" class="badge badge-outline badge-xs">
                  <i class="fas fa-shield-alt mr-1"></i>Vuln
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit Task Modal -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showCreateModal || editingTask" class="modal modal-open">
          <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">
          {{ editingTask ? t('bugBounty.monitor.editTask') : t('bugBounty.monitor.createTask') }}
        </h3>

        <div class="space-y-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.program.selectedProgram') }} *</span></label>
            <select v-model="taskForm.program_id" class="select select-bordered" :disabled="!!editingTask">
              <option value="">{{ t('bugBounty.program.selectProgramPlaceholder') }}</option>
              <option v-for="p in programs" :key="p.id" :value="p.id">
                {{ p.name }} {{ p.organization ? `(${p.organization})` : '' }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.monitor.taskName') }} *</span></label>
            <input 
              v-model="taskForm.name" 
              type="text" 
              class="input input-bordered"
              :placeholder="t('bugBounty.monitor.taskNamePlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.monitor.checkInterval') }} *</span></label>
            <select v-model="taskForm.interval_secs" class="select select-bordered">
              <option :value="3600">{{ t('bugBounty.monitor.intervals.hourly') }}</option>
              <option :value="6 * 3600">{{ t('bugBounty.monitor.intervals.every6Hours') }}</option>
              <option :value="12 * 3600">{{ t('bugBounty.monitor.intervals.every12Hours') }}</option>
              <option :value="24 * 3600">{{ t('bugBounty.monitor.intervals.daily') }}</option>
              <option :value="7 * 24 * 3600">{{ t('bugBounty.monitor.intervals.weekly') }}</option>
            </select>
          </div>

          <div class="divider">{{ t('bugBounty.monitor.monitorTypes') }}</div>
          
          <!-- Plugin loading status -->
          <div v-if="loadingPlugins" class="alert alert-info mb-3">
            <span class="loading loading-spinner loading-sm"></span>
            <span>{{ t('bugBounty.monitor.pluginsLoading') }}</span>
          </div>
          
          <!-- Available plugins count -->
          <div v-if="!loadingPlugins && availablePlugins.length > 0" class="text-xs text-base-content/60 mb-2">
            <i class="fas fa-plug mr-1"></i>
            {{ availablePlugins.length }} {{ t('bugBounty.monitor.availablePlugins') }}
          </div>

          <!-- DNS Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_dns_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-network-wired mr-2"></i>
                  {{ t('bugBounty.monitor.dnsMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_dns_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('dns')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <!-- Empty state hint -->
            <div v-if="taskForm.config.enable_dns_monitoring && taskForm.config.dns_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_dns_monitoring && taskForm.config.dns_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.dns_plugins" :key="`dns-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('dns')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`dns-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('dns')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('dns', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('dns', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('dns', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Certificate Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_cert_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-certificate mr-2"></i>
                  {{ t('bugBounty.monitor.certMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_cert_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('cert')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_cert_monitoring && taskForm.config.cert_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_cert_monitoring && taskForm.config.cert_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.cert_plugins" :key="`cert-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('cert')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`cert-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('cert')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('cert', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('cert', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('cert', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Content Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_content_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-file-alt mr-2"></i>
                  {{ t('bugBounty.monitor.contentMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_content_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('content')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_content_monitoring && taskForm.config.content_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_content_monitoring && taskForm.config.content_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.content_plugins" :key="`content-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('content')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`content-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('content')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('content', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('content', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('content', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- API Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_api_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-plug mr-2"></i>
                  {{ t('bugBounty.monitor.apiMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_api_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('api')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_api_monitoring && taskForm.config.api_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_api_monitoring && taskForm.config.api_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.api_plugins" :key="`api-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('api')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`api-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('api')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('api', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('api', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('api', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
          <!-- Port Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_port_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-network-wired mr-2"></i>
                  {{ t('bugBounty.monitor.portMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_port_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('port')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_port_monitoring && taskForm.config.port_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_port_monitoring && taskForm.config.port_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.port_plugins" :key="`port-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('port')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`port-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('port')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('port', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('port', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('port', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Web Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_web_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-globe mr-2"></i>
                  {{ t('bugBounty.monitor.webMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_web_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('web')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_web_monitoring && taskForm.config.web_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_web_monitoring && taskForm.config.web_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.web_plugins" :key="`web-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('content')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`web-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('content')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('web', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('web', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('web', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Vulnerability Monitoring -->
          <div class="card bg-base-200 p-4 mb-3">
            <div class="flex items-center justify-between mb-2">
              <label class="label cursor-pointer gap-2">
                <input type="checkbox" v-model="taskForm.config.enable_vuln_monitoring" class="checkbox checkbox-primary" />
                <span class="label-text font-semibold">
                  <i class="fas fa-shield-alt mr-2"></i>
                  {{ t('bugBounty.monitor.vulnMonitoring') }}
                </span>
              </label>
              <button 
                v-if="taskForm.config.enable_vuln_monitoring"
                class="btn btn-xs btn-ghost"
                @click="addPluginConfig('vuln')"
              >
                <i class="fas fa-plus mr-1"></i>
                {{ t('bugBounty.monitor.addPlugin') }}
              </button>
            </div>
            
            <div v-if="taskForm.config.enable_vuln_monitoring && taskForm.config.vuln_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
              <i class="fas fa-info-circle mr-1"></i>
              {{ t('bugBounty.monitor.noPluginsConfigured') }}
            </div>
            
            <div v-if="taskForm.config.enable_vuln_monitoring && taskForm.config.vuln_plugins.length > 0" class="space-y-2 ml-6">
              <div v-for="(plugin, idx) in taskForm.config.vuln_plugins" :key="`vuln-${idx}`" class="card bg-base-100 p-3">
                <div class="flex items-start gap-2">
                  <div class="flex-1 space-y-2">
                    <div class="form-control">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                      </label>
                      <select v-model="plugin.plugin_id" class="select select-sm select-bordered">
                        <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                        <option v-for="p in getPluginsByType('vuln')" :key="p.id" :value="p.id">
                          {{ p.name }}
                        </option>
                      </select>
                    </div>
                    
                    <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                      <label class="label py-1">
                        <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                      </label>
                      <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`vuln-fb-${idx}-${fIdx}`" class="flex gap-1">
                        <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1">
                          <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                          <option v-for="p in getPluginsByType('vuln')" :key="p.id" :value="p.id">
                            {{ p.name }}
                          </option>
                        </select>
                        <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('vuln', idx, Number(fIdx))">
                          <i class="fas fa-times"></i>
                        </button>
                      </div>
                    </div>
                    
                    <button 
                      class="btn btn-xs btn-ghost"
                      @click="addFallbackPlugin('vuln', idx)"
                    >
                      <i class="fas fa-plus mr-1"></i>
                      {{ t('bugBounty.monitor.addFallback') }}
                    </button>
                  </div>
                  
                  <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('vuln', idx)">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
          <div class="divider">{{ t('bugBounty.monitor.autoTrigger') }}</div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('bugBounty.monitor.autoTriggerEnabled') }}</span>
              <input type="checkbox" v-model="taskForm.config.auto_trigger_enabled" class="checkbox checkbox-primary" />
            </label>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeModal">{{ t('common.cancel') }}</button>
          <button 
            class="btn btn-primary" 
            @click="saveTask" 
            :disabled="!taskForm.name || submitting"
          >
            <span v-if="submitting" class="loading loading-spinner loading-sm mr-2"></span>
            {{ editingTask ? t('common.save') : t('common.create') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeModal"></div>
    </div>
      </Transition>
    </Teleport>

    <!-- Discover Assets Modal -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showDiscoverModal" class="modal modal-open">
          <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg mb-4">
          <i class="fas fa-search mr-2"></i>
          {{ t('bugBounty.monitor.discoverAssets') }}
        </h3>

        <div v-if="!discoverResult" class="space-y-4">
          <div class="alert alert-info">
            <i class="fas fa-info-circle"></i>
            <span>{{ t('bugBounty.monitor.discoverAssetsHint') }}</span>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.monitor.selectPlugin') }} *</span></label>
            <select v-model="discoverForm.plugin_id" class="select select-bordered">
              <option value="">{{ t('bugBounty.monitor.selectPluginPlaceholder') }}</option>
              <option value="subdomain_enumerator">{{ t('bugBounty.monitor.plugins.subdomainEnum') }}</option>
              <option value="http_prober">{{ t('bugBounty.monitor.plugins.httpProber') }}</option>
              <option value="port_monitor">{{ t('bugBounty.monitor.plugins.portMonitor') }}</option>
            </select>
          </div>

          <div v-if="discoverForm.plugin_id === 'subdomain_enumerator'" class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.monitor.targetDomain') }} *</span></label>
            <input 
              v-model="discoverForm.domain" 
              type="text" 
              class="input input-bordered"
              placeholder="example.com"
            />
          </div>

          <div v-if="discoverForm.plugin_id === 'http_prober'" class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.monitor.targetUrls') }} *</span></label>
            <textarea 
              v-model="discoverForm.urls" 
              class="textarea textarea-bordered h-24"
              :placeholder="t('bugBounty.monitor.urlsPlaceholder')"
            ></textarea>
            <label class="label">
              <span class="label-text-alt">{{ t('bugBounty.monitor.urlsHint') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">
                <i class="fas fa-database mr-2"></i>
                {{ t('bugBounty.monitor.autoImportAssets') }}
              </span>
              <input type="checkbox" v-model="discoverForm.auto_import" class="checkbox checkbox-primary" />
            </label>
            <label class="label">
              <span class="label-text-alt">{{ t('bugBounty.monitor.autoImportHint') }}</span>
            </label>
          </div>
        </div>

        <div v-else class="space-y-4">
          <!-- Discovery Results -->
          <div class="alert" :class="discoverResult.success ? 'alert-success' : 'alert-error'">
            <i :class="discoverResult.success ? 'fas fa-check-circle' : 'fas fa-times-circle'"></i>
            <div>
              <div class="font-semibold">
                {{ discoverResult.success ? t('bugBounty.monitor.discoverySuccess') : t('bugBounty.monitor.discoveryFailed') }}
              </div>
              <div v-if="discoverResult.error" class="text-sm">{{ discoverResult.error }}</div>
            </div>
          </div>

          <div v-if="discoverResult.success" class="stats stats-vertical lg:stats-horizontal shadow w-full">
            <div class="stat">
              <div class="stat-title">{{ t('bugBounty.monitor.assetsDiscovered') }}</div>
              <div class="stat-value text-primary">{{ discoverResult.assets_discovered }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ t('bugBounty.monitor.assetsImported') }}</div>
              <div class="stat-value text-success">{{ discoverResult.assets_imported }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">{{ t('bugBounty.monitor.eventsCreated') }}</div>
              <div class="stat-value text-warning">{{ discoverResult.events_created }}</div>
            </div>
          </div>

          <div v-if="discoverResult.plugin_output" class="collapse collapse-arrow bg-base-200">
            <input type="checkbox" /> 
            <div class="collapse-title font-medium">
              <i class="fas fa-code mr-2"></i>
              {{ t('bugBounty.monitor.pluginOutput') }}
            </div>
            <div class="collapse-content">
              <pre class="text-xs overflow-auto max-h-96 bg-base-300 p-4 rounded">{{ JSON.stringify(discoverResult.plugin_output, null, 2) }}</pre>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeDiscoverModal">
            {{ discoverResult ? t('common.close') : t('common.cancel') }}
          </button>
          <button 
            v-if="!discoverResult"
            class="btn btn-primary" 
            @click="executeDiscovery" 
            :disabled="!isDiscoverFormValid || discovering"
          >
            <span v-if="discovering" class="loading loading-spinner loading-sm mr-2"></span>
            <i v-else class="fas fa-search mr-2"></i>
            {{ t('bugBounty.monitor.startDiscovery') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="closeDiscoverModal"></div>
    </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  selectedProgram?: any
  programs?: any[]
}>()

// State
const schedulerRunning = ref(false)
const starting = ref(false)
const stopping = ref(false)
const loading = ref(false)
const submitting = ref(false)
const stats = ref<any>(null)
const tasks = ref<any[]>([])
const showCreateModal = ref(false)
const editingTask = ref<any>(null)
const showDiscoverModal = ref(false)
const discovering = ref(false)
const discoverResult = ref<any>(null)
const currentDiscoverTask = ref<any>(null)
const availablePlugins = ref<any[]>([])
const loadingPlugins = ref(false)

const taskForm = reactive({
  name: '',
  program_id: '',
  interval_secs: 6 * 3600, // 6 hours default
  config: {
    enable_dns_monitoring: true,
    dns_plugins: [] as any[],
    enable_cert_monitoring: true,
    cert_plugins: [] as any[],
    enable_content_monitoring: false,
    content_plugins: [] as any[],
    enable_api_monitoring: false,
    api_plugins: [] as any[],
    enable_port_monitoring: false,
    port_plugins: [] as any[],
    enable_web_monitoring: false,
    web_plugins: [] as any[],
    enable_vuln_monitoring: false,
    vuln_plugins: [] as any[],
    auto_trigger_enabled: true,
  }
})

const discoverForm = reactive({
  plugin_id: '',
  domain: '',
  urls: '',
  auto_import: true,
})

const isDiscoverFormValid = computed(() => {
  if (!discoverForm.plugin_id) return false
  if (discoverForm.plugin_id === 'subdomain_enumerator' && !discoverForm.domain) return false
  if (discoverForm.plugin_id === 'http_prober' && !discoverForm.urls) return false
  return true
})

let unlistenChangeDetected: any = null
let unlistenSchedulerStarted: any = null
let unlistenSchedulerStopped: any = null

// Methods
const checkSchedulerStatus = async () => {
  try {
    schedulerRunning.value = await invoke('monitor_is_running')
  } catch (error) {
    console.error('Failed to check scheduler status:', error)
  }
}

const refreshStats = async () => {
  try {
    stats.value = await invoke('monitor_get_stats')
  } catch (error) {
    console.error('Failed to load stats:', error)
  }
}

const loadTasks = async (retryCount = 0) => {
  try {
    loading.value = true
    tasks.value = await invoke('monitor_list_tasks', {
      programId: props.selectedProgram?.id || null
    })
  } catch (error) {
    console.error('Failed to load tasks:', error)
    // Retry once after a short delay if this is the first attempt
    if (retryCount === 0) {
      await new Promise(resolve => setTimeout(resolve, 1000))
      return loadTasks(1)
    }
    toast.error(t('bugBounty.errors.loadFailed'))
    // Ensure tasks is set to empty array on error to prevent stuck loading state
    if (!tasks.value || tasks.value.length === 0) {
      tasks.value = []
    }
  } finally {
    loading.value = false
  }
}

const loadAvailablePlugins = async () => {
  try {
    loadingPlugins.value = true
    const plugins = await invoke('monitor_get_available_plugins') as any[]
    console.log('✅ Loaded available plugins from backend:', plugins)
    
    availablePlugins.value = plugins || []
  } catch (error) {
    console.error('❌ Failed to load available plugins:', error)
    availablePlugins.value = []
    toast.warning(t('bugBounty.monitor.pluginsLoadFailed'))
  } finally {
    loadingPlugins.value = false
  }
}

const getPluginsByType = (monitorType: string) => {
  const filtered = availablePlugins.value.filter((p: any) => p.monitor_type === monitorType)
  console.log(`Plugins for ${monitorType}:`, filtered)
  return filtered
}

const addPluginConfig = (monitorType: string) => {
  const newPlugin = {
    plugin_id: '',
    fallback_plugins: [],
    plugin_params: {}
  }
  
  switch (monitorType) {
    case 'dns':
      taskForm.config.dns_plugins.push(newPlugin)
      break
    case 'cert':
      taskForm.config.cert_plugins.push(newPlugin)
      break
    case 'content':
      taskForm.config.content_plugins.push(newPlugin)
      break
    case 'api':
      taskForm.config.api_plugins.push(newPlugin)
      break
    case 'port':
      taskForm.config.port_plugins.push(newPlugin)
      break
    case 'web':
      taskForm.config.web_plugins.push(newPlugin)
      break
    case 'vuln':
      taskForm.config.vuln_plugins.push(newPlugin)
      break
  }
}

const removePluginConfig = (monitorType: string, index: number) => {
  switch (monitorType) {
    case 'dns':
      taskForm.config.dns_plugins.splice(index, 1)
      break
    case 'cert':
      taskForm.config.cert_plugins.splice(index, 1)
      break
    case 'content':
      taskForm.config.content_plugins.splice(index, 1)
      break
    case 'api':
      taskForm.config.api_plugins.splice(index, 1)
      break
    case 'port':
      taskForm.config.port_plugins.splice(index, 1)
      break
    case 'web':
      taskForm.config.web_plugins.splice(index, 1)
      break
    case 'vuln':
      taskForm.config.vuln_plugins.splice(index, 1)
      break
  }
}

const addFallbackPlugin = (monitorType: string, pluginIndex: number) => {
  let plugins: any[] = []
  switch (monitorType) {
    case 'dns':
      plugins = taskForm.config.dns_plugins
      break
    case 'cert':
      plugins = taskForm.config.cert_plugins
      break
    case 'content':
      plugins = taskForm.config.content_plugins
      break
    case 'api':
      plugins = taskForm.config.api_plugins
      break
    case 'port':
      plugins = taskForm.config.port_plugins
      break
    case 'web':
      plugins = taskForm.config.web_plugins
      break
    case 'vuln':
      plugins = taskForm.config.vuln_plugins
      break
  }
  
  if (plugins[pluginIndex]) {
    plugins[pluginIndex].fallback_plugins.push('')
  }
}

const removeFallbackPlugin = (monitorType: string, pluginIndex: number, fallbackIndex: number) => {
  let plugins: any[] = []
  switch (monitorType) {
    case 'dns':
      plugins = taskForm.config.dns_plugins
      break
    case 'cert':
      plugins = taskForm.config.cert_plugins
      break
    case 'content':
      plugins = taskForm.config.content_plugins
      break
    case 'api':
      plugins = taskForm.config.api_plugins
      break
    case 'port':
      plugins = taskForm.config.port_plugins
      break
    case 'web':
      plugins = taskForm.config.web_plugins
      break
    case 'vuln':
      plugins = taskForm.config.vuln_plugins
      break
  }
  
  if (plugins[pluginIndex]) {
    plugins[pluginIndex].fallback_plugins.splice(fallbackIndex, 1)
  }
}

const startScheduler = async (options?: any) => {
  try {
    starting.value = true
    await invoke('monitor_start_scheduler')
    schedulerRunning.value = true
    toast.success(t('bugBounty.monitor.schedulerStarted'))
    await refreshStats()
  } catch (error: any) {
    const errorMsg = error?.toString() || ''
    // Handle case where scheduler is already running
    if (errorMsg.includes('Scheduler is already running')) {
      console.log('Scheduler is already running, syncing state.')
      schedulerRunning.value = true
      return
    }

    console.error('Failed to start scheduler:', error)
    toast.error(t('bugBounty.monitor.startFailed'))
    
    // Allow caller to handle error if requested
    if (options?.throwOnFail) {
      throw error
    }
  } finally {
    starting.value = false
  }
}

const stopScheduler = async () => {
  try {
    stopping.value = true
    await invoke('monitor_stop_scheduler')
    schedulerRunning.value = false
    toast.success(t('bugBounty.monitor.schedulerStopped'))
    await refreshStats()
  } catch (error) {
    console.error('Failed to stop scheduler:', error)
    toast.error(t('bugBounty.monitor.stopFailed'))
  } finally {
    stopping.value = false
  }
}

const createDefaultTasks = async () => {
  if (!props.selectedProgram) return
  try {
    const taskIds = await invoke('monitor_create_default_tasks', {
      programId: props.selectedProgram.id
    })
    toast.success(t('bugBounty.monitor.defaultTasksCreated', { count: (taskIds as any[]).length }))
    await loadTasks()
  } catch (error) {
    console.error('Failed to create default tasks:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  }
}

const saveTask = async () => {
  if (!taskForm.name.trim()) return
  if (!taskForm.program_id) {
    toast.error(t('bugBounty.monitor.selectProgramFirst'))
    return
  }

  try {
    submitting.value = true
    
    if (editingTask.value) {
      // Update existing task
      await invoke('monitor_update_task', {
        taskId: editingTask.value.id,
        request: {
          name: taskForm.name,
          interval_secs: taskForm.interval_secs,
          config: taskForm.config
        }
      })
      toast.success(t('bugBounty.monitor.taskUpdated'))
    } else {
      // Create new task
      await invoke('monitor_create_task', {
        request: {
          program_id: taskForm.program_id,
          name: taskForm.name,
          interval_secs: taskForm.interval_secs,
          config: taskForm.config
        }
      })
      toast.success(t('bugBounty.monitor.taskCreated'))
    }
    
    closeModal()
    await loadTasks()
  } catch (error) {
    console.error('Failed to save task:', error)
    toast.error(t('bugBounty.errors.saveFailed'))
  } finally {
    submitting.value = false
  }
}

const toggleTask = async (task: any) => {
  try {
    if (task.enabled) {
      await invoke('monitor_disable_task', { taskId: task.id })
      toast.success(t('bugBounty.monitor.taskDisabled'))
    } else {
      await invoke('monitor_enable_task', { taskId: task.id })
      toast.success(t('bugBounty.monitor.taskEnabled'))
    }
    await loadTasks()
  } catch (error) {
    console.error('Failed to toggle task:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const triggerTask = async (task: any) => {
  try {
    // Auto-start scheduler if not running
    if (!schedulerRunning.value) {
      toast.info(t('bugBounty.monitor.autoStartingScheduler'))
      await startScheduler({ throwOnFail: true })
    }
    
    await invoke('monitor_trigger_task', { taskId: task.id })
    toast.success(t('bugBounty.monitor.taskTriggered'))
    
    // Wait a bit for task to start, then reload tasks
    await new Promise(resolve => setTimeout(resolve, 500))
    await loadTasks()
  } catch (error) {
    console.error('Failed to trigger task:', error)
    toast.error(t('bugBounty.errors.operationFailed'))
    // Still try to reload tasks even if trigger failed
    try {
      await loadTasks()
    } catch (e) {
      console.error('Failed to reload tasks after trigger error:', e)
    }
  }
}

const editTask = (task: any) => {
  editingTask.value = task
  taskForm.name = task.name
  taskForm.program_id = task.program_id
  taskForm.interval_secs = task.interval_secs
  taskForm.config = { ...task.config }
}

const deleteTask = async (task: any) => {
  try {
    await invoke('monitor_delete_task', { taskId: task.id })
    toast.success(t('bugBounty.monitor.taskDeleted'))
    await loadTasks()
  } catch (error) {
    console.error('Failed to delete task:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  }
}

const discoverAssets = (task: any) => {
  const programId = task.program_id || props.selectedProgram?.id
  if (!programId) {
    toast.error(t('bugBounty.monitor.selectProgramFirst'))
    return
  }
  currentDiscoverTask.value = task
  showDiscoverModal.value = true
  discoverResult.value = null
  // Reset form
  discoverForm.plugin_id = ''
  discoverForm.domain = ''
  discoverForm.urls = ''
  discoverForm.auto_import = true
}

const executeDiscovery = async () => {
  const programId = currentDiscoverTask.value?.program_id || props.selectedProgram?.id
  
  if (!programId || !currentDiscoverTask.value) return
  
  try {
    discovering.value = true
    
    // Prepare plugin input based on plugin type
    let pluginInput: any = {}
    
    if (discoverForm.plugin_id === 'subdomain_enumerator') {
      pluginInput = {
        domain: discoverForm.domain,
        removeDuplicates: true,
      }
    } else if (discoverForm.plugin_id === 'http_prober') {
      const urls = discoverForm.urls.split('\n').map(u => u.trim()).filter(u => u.length > 0)
      pluginInput = {
        urls,
      }
    }
    
    const result = await invoke('monitor_discover_and_import_assets', {
      request: {
        program_id: programId,
        scope_id: null,
        plugin_id: discoverForm.plugin_id,
        plugin_input: pluginInput,
        auto_import: discoverForm.auto_import,
      }
    })
    
    discoverResult.value = result
    
    if ((result as any).success) {
      if ((result as any).assets_imported > 0) {
        toast.success(t('bugBounty.monitor.assetsImportedSuccess', { count: (result as any).assets_imported }))
      } else {
        toast.info(t('bugBounty.monitor.noNewAssets'))
      }
    } else {
      toast.error((result as any).error || t('bugBounty.errors.operationFailed'))
    }
  } catch (error: any) {
    console.error('Failed to discover assets:', error)
    toast.error(error || t('bugBounty.errors.operationFailed'))
    discoverResult.value = {
      success: false,
      error: error?.toString() || 'Unknown error',
      assets_discovered: 0,
      assets_imported: 0,
      events_created: 0,
    }
  } finally {
    discovering.value = false
  }
}

const closeDiscoverModal = () => {
  showDiscoverModal.value = false
  currentDiscoverTask.value = null
  discoverResult.value = null
}

const openCreateModal = () => {
  if (props.selectedProgram) {
    taskForm.program_id = props.selectedProgram.id
  }
  showCreateModal.value = true
}

const closeModal = () => {
  showCreateModal.value = false
  editingTask.value = null
  taskForm.name = ''
  taskForm.program_id = ''
  taskForm.interval_secs = 6 * 3600
  taskForm.config = {
    enable_dns_monitoring: true,
    dns_plugins: [],
    enable_cert_monitoring: true,
    cert_plugins: [],
    enable_content_monitoring: false,
    content_plugins: [],
    enable_api_monitoring: false,
    api_plugins: [],
    enable_port_monitoring: false,
    port_plugins: [],
    enable_web_monitoring: false,
    web_plugins: [],
    enable_vuln_monitoring: false,
    vuln_plugins: [],
    auto_trigger_enabled: true,
  }
}

// Formatters
const formatInterval = (secs: number) => {
  const hours = secs / 3600
  const days = hours / 24
  
  if (days >= 1) {
    return `${days} ${t('bugBounty.monitor.days')}`
  }
  return `${hours} ${t('bugBounty.monitor.hours')}`
}

const formatUptime = (secs: number) => {
  const hours = Math.floor(secs / 3600)
  const minutes = Math.floor((secs % 3600) / 60)
  
  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}

const formatDateTime = (dateStr: string) => {
  return new Date(dateStr).toLocaleString()
}

// Event listeners
const setupEventListeners = async () => {
  unlistenChangeDetected = await listen('monitor:change-detected', (event) => {
    console.log('Change detected:', event.payload)
    toast.info(t('bugBounty.monitor.changeDetected'))
    refreshStats()
  })

  unlistenSchedulerStarted = await listen('monitor:scheduler-started', () => {
    schedulerRunning.value = true
    refreshStats()
  })

  unlistenSchedulerStopped = await listen('monitor:scheduler-stopped', () => {
    schedulerRunning.value = false
    refreshStats()
  })
}

// Lifecycle
onMounted(async () => {
  await checkSchedulerStatus()
  await refreshStats()
  await loadTasks()
  await loadAvailablePlugins()
  await setupEventListeners()
  
  // Auto-refresh stats every 30 seconds
  const refreshInterval = setInterval(() => {
    if (schedulerRunning.value) {
      refreshStats()
      loadTasks()
    }
  }, 30000)
  
  onUnmounted(() => {
    clearInterval(refreshInterval)
    unlistenChangeDetected?.()
    unlistenSchedulerStarted?.()
    unlistenSchedulerStopped?.()
  })
})
</script>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .modal-box,
.modal-leave-active .modal-box {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.modal-enter-from .modal-box,
.modal-leave-to .modal-box {
  transform: scale(0.95);
  opacity: 0;
}
</style>
