<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 rounded-lg flex items-center justify-center" :class="schedulerRunning ? 'bg-success/20 text-success' : 'bg-base-300 text-base-content/50'">
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
            <button v-if="!schedulerRunning" class="btn btn-success btn-sm" @click="startScheduler" :disabled="starting">
              <i class="fas fa-play mr-2"></i>
              {{ t('bugBounty.monitor.start') }}
            </button>
            <button v-else class="btn btn-error btn-sm" @click="stopScheduler" :disabled="stopping">
              <i class="fas fa-stop mr-2"></i>
              {{ t('bugBounty.monitor.stop') }}
            </button>
            <button class="btn btn-ghost btn-sm" @click="refreshStats">
              <i class="fas fa-sync-alt"></i>
            </button>
          </div>
        </div>
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
            <div class="stat-value text-lg text-sm">
              {{ formatUptime(stats.scheduler_uptime_secs) }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card bg-base-100 shadow-md">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">{{ t('bugBounty.monitor.tasks') }}</h3>
          <div class="flex gap-2">
            <button v-if="selectedProgram" class="btn btn-sm btn-outline" @click="createDefaultTasks">
              <i class="fas fa-magic mr-2"></i>
              {{ t('bugBounty.monitor.createDefault') }}
            </button>
            <button type="button" class="btn btn-sm btn-primary" @click="openCreateModal">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.monitor.createTask') }}
            </button>
            <button type="button" class="btn btn-sm btn-outline" @click="showRunHistoryDrawer = true">
              <i class="fas fa-history mr-2"></i>
              {{ t('bugBounty.monitor.runHistory') }}
            </button>
          </div>
        </div>

        <div v-if="loading && tasks.length === 0" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="tasks.length === 0" class="text-center py-8">
          <i class="fas fa-tasks text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.monitor.noTasks') }}</p>
          <button v-if="selectedProgram" class="btn btn-primary btn-sm mt-4" @click="createDefaultTasks">
            {{ t('bugBounty.monitor.createDefaultTasks') }}
          </button>
        </div>

        <div v-else class="space-y-3">
          <MonitorTaskCard v-for="task in tasks" :key="task.id" :task="task" :is-running="isTaskRunning(task.id)" :stopping="stoppingTaskIds.has(task.id)" :progress="getTaskProgress(task.id)" @toggle="toggleTask" @discover="discoverAssets" @stop="stopTask" @trigger="triggerTask" @edit="editTask" @delete="deleteTask" />
        </div>
      </div>
    </div>

    <MonitorRunHistoryDrawer :open="showRunHistoryDrawer" :programs="props.programs" @close="showRunHistoryDrawer = false" />

    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showCreateModal || editingTask" class="modal modal-open">
          <div class="modal-box max-w-2xl">
            <h3 class="font-bold text-lg mb-4">
              {{ editingTask ? t('bugBounty.monitor.editTask') : t('bugBounty.monitor.createTask') }}
            </h3>

            <div class="space-y-4">
              <div class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.program.selectedProgram') }} *</span></label
                >
                <select v-model="taskForm.program_id" class="select select-bordered" :disabled="!!editingTask">
                  <option value="">{{ t('bugBounty.program.selectProgramPlaceholder') }}</option>
                  <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }} {{ p.organization ? `(${p.organization})` : '' }}</option>
                </select>
              </div>

              <div class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.taskName') }} *</span></label
                >
                <input v-model="taskForm.name" type="text" class="input input-bordered" :placeholder="t('bugBounty.monitor.taskNamePlaceholder')" />
              </div>

              <div class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.checkInterval') }} *</span></label
                >
                <select v-model="taskForm.interval_secs" class="select select-bordered">
                  <option :value="3600">{{ t('bugBounty.monitor.intervals.hourly') }}</option>
                  <option :value="6 * 3600">
                    {{ t('bugBounty.monitor.intervals.every6Hours') }}
                  </option>
                  <option :value="12 * 3600">
                    {{ t('bugBounty.monitor.intervals.every12Hours') }}
                  </option>
                  <option :value="24 * 3600">{{ t('bugBounty.monitor.intervals.daily') }}</option>
                  <option :value="7 * 24 * 3600">
                    {{ t('bugBounty.monitor.intervals.weekly') }}
                  </option>
                </select>
              </div>

              <div class="divider">{{ t('bugBounty.monitor.monitorTypes') }}</div>

              <div v-if="loadingPlugins" class="alert alert-info mb-3">
                <span class="loading loading-spinner loading-sm"></span>
                <span>{{ t('bugBounty.monitor.pluginsLoading') }}</span>
              </div>

              <div v-if="!loadingPlugins && availablePlugins.length > 0" class="text-xs text-base-content/60 mb-2">
                <i class="fas fa-plug mr-1"></i>
                {{ availablePlugins.length }} {{ t('bugBounty.monitor.availablePlugins') }}
              </div>

              <div class="card bg-base-200 p-4 mb-3">
                <div class="flex items-center justify-between mb-2">
                  <label class="label cursor-pointer gap-2">
                    <input type="checkbox" v-model="taskForm.config.enable_dns_monitoring" class="checkbox checkbox-primary" />
                    <span class="label-text font-semibold">
                      <i class="fas fa-network-wired mr-2"></i>
                      {{ t('bugBounty.monitor.dnsMonitoring') }}
                    </span>
                  </label>
                  <button v-if="taskForm.config.enable_dns_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('dns')">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('bugBounty.monitor.addPlugin') }}
                  </button>
                </div>

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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('dns')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('dns', plugin.plugin_id)" />
                        <MonitorSubdomainBruteConfig v-if="plugin.plugin_id === 'subdomain_brute'" :plugin="plugin" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`dns-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
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

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('dns', idx)">
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

              <!-- IP Monitoring -->
              <div class="card bg-base-200 p-4 mb-3">
                <div class="flex items-center justify-between mb-2">
                  <label class="label cursor-pointer gap-2">
                    <input type="checkbox" v-model="taskForm.config.enable_ip_monitoring" class="checkbox checkbox-primary" />
                    <span class="label-text font-semibold">
                      <i class="fas fa-diagram-project mr-2"></i>
                      {{ t('bugBounty.monitor.ipMonitoring') }}
                    </span>
                  </label>
                  <button v-if="taskForm.config.enable_ip_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('ip')">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('bugBounty.monitor.addPlugin') }}
                  </button>
                </div>

                <div v-if="taskForm.config.enable_ip_monitoring && taskForm.config.ip_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
                  <i class="fas fa-info-circle mr-1"></i>
                  {{ t('bugBounty.monitor.noPluginsConfigured') }}
                </div>

                <div v-if="taskForm.config.enable_ip_monitoring && taskForm.config.ip_plugins.length > 0" class="space-y-2 ml-6">
                  <div v-for="(plugin, idx) in taskForm.config.ip_plugins" :key="`ip-${idx}`" class="card bg-base-100 p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-1 space-y-2">
                        <div class="text-xs text-base-content/60">
                          目标资产应为域名，监控的是域名解析结果中的 IP 变化。
                        </div>
                        <div class="form-control">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                          </label>
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('ip')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('ip', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`ip-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
                              <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                              <option v-for="p in getPluginsByType('ip')" :key="p.id" :value="p.id">
                                {{ p.name }}
                              </option>
                            </select>
                            <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('ip', idx, Number(fIdx))">
                              <i class="fas fa-times"></i>
                            </button>
                          </div>
                        </div>

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('ip', idx)">
                          <i class="fas fa-plus mr-1"></i>
                          {{ t('bugBounty.monitor.addFallback') }}
                        </button>
                      </div>

                      <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('ip', idx)">
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
                  <button v-if="taskForm.config.enable_port_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('port')">
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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('port')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('port', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`port-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
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

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('port', idx)">
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

              <!-- Service Monitoring -->
              <div class="card bg-base-200 p-4 mb-3">
                <div class="flex items-center justify-between mb-2">
                  <label class="label cursor-pointer gap-2">
                    <input type="checkbox" v-model="taskForm.config.enable_service_monitoring" class="checkbox checkbox-primary" />
                    <span class="label-text font-semibold">
                      <i class="fas fa-server mr-2"></i>
                      {{ t('bugBounty.monitor.serviceMonitoring') }}
                    </span>
                  </label>
                  <button v-if="taskForm.config.enable_service_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('service')">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('bugBounty.monitor.addPlugin') }}
                  </button>
                </div>

                <div v-if="taskForm.config.enable_service_monitoring && taskForm.config.service_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
                  <i class="fas fa-info-circle mr-1"></i>
                  {{ t('bugBounty.monitor.noPluginsConfigured') }}
                </div>

                <div v-if="taskForm.config.enable_service_monitoring && taskForm.config.service_plugins.length > 0" class="space-y-2 ml-6">
                  <div v-for="(plugin, idx) in taskForm.config.service_plugins" :key="`service-${idx}`" class="card bg-base-100 p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-1 space-y-2">
                        <div class="form-control">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                          </label>
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('service')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <div v-if="supportsServiceProbeEngine(plugin)" class="form-control">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.serviceProbeEngine') }}</span>
                          </label>
                          <select :value="getServiceProbeEngine(plugin)" class="select select-sm select-bordered" @change="handleServiceProbeEngineChange(plugin, $event)">
                            <option value="native">
                              {{ t('bugBounty.monitor.serviceProbeEngineNative') }}
                            </option>
                          </select>
                          <div class="text-xs text-base-content/60 mt-1">
                            {{ t('bugBounty.monitor.serviceProbeEngineHint') }}
                          </div>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('service', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`service-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
                              <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                              <option v-for="p in getPluginsByType('service')" :key="p.id" :value="p.id">
                                {{ p.name }}
                              </option>
                            </select>
                            <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('service', idx, Number(fIdx))">
                              <i class="fas fa-times"></i>
                            </button>
                          </div>
                        </div>

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('service', idx)">
                          <i class="fas fa-plus mr-1"></i>
                          {{ t('bugBounty.monitor.addFallback') }}
                        </button>
                      </div>

                      <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('service', idx)">
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
                  <button v-if="taskForm.config.enable_cert_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('cert')">
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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('cert')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('cert', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`cert-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
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

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('cert', idx)">
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
                  <button v-if="taskForm.config.enable_web_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('web')">
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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('web')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('web', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`web-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
                              <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                              <option v-for="p in getPluginsByType('web')" :key="p.id" :value="p.id">
                                {{ p.name }}
                              </option>
                            </select>
                            <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('web', idx, Number(fIdx))">
                              <i class="fas fa-times"></i>
                            </button>
                          </div>
                        </div>

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('web', idx)">
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
                  <button v-if="taskForm.config.enable_api_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('api')">
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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('api')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('api', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`api-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
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

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('api', idx)">
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
                  <button v-if="taskForm.config.enable_content_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('content')">
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
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('content')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('content', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`content-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
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

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('content', idx)">
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

              <!-- Vulnerability Monitoring -->
              <div class="card bg-base-200 p-4 mb-3">
                <div class="flex items-center justify-between mb-2">
                  <label class="label cursor-pointer gap-2">
                    <input type="checkbox" v-model="taskForm.config.enable_risk_monitoring" class="checkbox checkbox-primary" />
                    <span class="label-text font-semibold">
                      <i class="fas fa-shield-alt mr-2"></i>
                      {{ t('bugBounty.monitor.vulnMonitoring') }}
                    </span>
                  </label>
                  <button v-if="taskForm.config.enable_risk_monitoring" class="btn btn-xs btn-ghost" @click="addPluginConfig('risk')">
                    <i class="fas fa-plus mr-1"></i>
                    {{ t('bugBounty.monitor.addPlugin') }}
                  </button>
                </div>

                <div v-if="taskForm.config.enable_risk_monitoring && taskForm.config.risk_plugins.length === 0" class="text-center py-4 text-sm text-base-content/60 ml-6">
                  <i class="fas fa-info-circle mr-1"></i>
                  {{ t('bugBounty.monitor.noPluginsConfigured') }}
                </div>

                <div v-if="taskForm.config.enable_risk_monitoring && taskForm.config.risk_plugins.length > 0" class="space-y-2 ml-6">
                  <div v-for="(plugin, idx) in taskForm.config.risk_plugins" :key="`risk-${idx}`" class="card bg-base-100 p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-1 space-y-2">
                        <div class="form-control">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.primaryPlugin') }}</span>
                          </label>
                          <select v-model="plugin.plugin_id" class="select select-sm select-bordered" @focus="refreshAvailablePluginsOnDropdownOpen" @change="handleServicePluginChanged(plugin)">
                            <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                            <option v-for="p in getPluginsByType('risk')" :key="p.id" :value="p.id">
                              {{ p.name }}
                            </option>
                          </select>
                        </div>

                        <MonitorTargetAssetSelector v-model="plugin.target_asset_types" :allowed-values="getAllowedTargetAssetTypes('risk', plugin.plugin_id)" />

                        <div v-if="plugin.fallback_plugins.length > 0" class="space-y-1">
                          <label class="label py-1">
                            <span class="label-text-alt">{{ t('bugBounty.monitor.fallbackPlugins') }}</span>
                          </label>
                          <div v-for="(fallback, fIdx) in plugin.fallback_plugins" :key="`risk-fb-${idx}-${fIdx}`" class="flex gap-1">
                            <select v-model="plugin.fallback_plugins[fIdx]" class="select select-xs select-bordered flex-1" @focus="refreshAvailablePluginsOnDropdownOpen">
                              <option value="">{{ t('bugBounty.monitor.selectPlugin') }}</option>
                              <option v-for="p in getPluginsByType('risk')" :key="p.id" :value="p.id">
                                {{ p.name }}
                              </option>
                            </select>
                            <button class="btn btn-xs btn-ghost" @click="removeFallbackPlugin('risk', idx, Number(fIdx))">
                              <i class="fas fa-times"></i>
                            </button>
                          </div>
                        </div>

                        <button class="btn btn-xs btn-ghost" @click="addFallbackPlugin('risk', idx)">
                          <i class="fas fa-plus mr-1"></i>
                          {{ t('bugBounty.monitor.addFallback') }}
                        </button>
                      </div>

                      <button class="btn btn-xs btn-ghost text-error" @click="removePluginConfig('risk', idx)">
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
              <button class="btn btn-primary" @click="saveTask" :disabled="!taskForm.name || submitting">
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
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.selectPlugin') }} *</span></label
                >
                <select v-model="discoverForm.plugin_id" class="select select-bordered">
                  <option value="">{{ t('bugBounty.monitor.selectPluginPlaceholder') }}</option>
                  <option value="subdomain_enumerator">
                    {{ t('bugBounty.monitor.plugins.subdomainEnum') }}
                  </option>
                  <option value="subdomain_brute">
                    {{ t('bugBounty.monitor.plugins.subdomainBrute') }}
                  </option>
                  <option value="http_prober">
                    {{ t('bugBounty.monitor.plugins.httpProber') }}
                  </option>
                  <option value="port_monitor">
                    {{ t('bugBounty.monitor.plugins.portMonitor') }}
                  </option>
                </select>
              </div>

              <div v-if="['subdomain_enumerator', 'subdomain_brute'].includes(discoverForm.plugin_id)" class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.targetDomain') }} *</span></label
                >
                <input v-model="discoverForm.domain" type="text" class="input input-bordered" placeholder="example.com" />
              </div>

              <div v-if="discoverForm.plugin_id === 'http_prober'" class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.targetUrls') }} *</span></label
                >
                <textarea v-model="discoverForm.urls" class="textarea textarea-bordered h-24" :placeholder="t('bugBounty.monitor.urlsPlaceholder')"></textarea>
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
              <button v-if="!discoverResult" class="btn btn-primary" @click="executeDiscovery" :disabled="!isDiscoverFormValid || discovering">
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
import { useMonitorTaskProgress } from '../../composables/useMonitorTaskProgress'
import MonitorRunHistoryDrawer from './MonitorRunHistoryDrawer.vue'
import MonitorSubdomainBruteConfig from './MonitorSubdomainBruteConfig.vue'
import MonitorTaskCard from './MonitorTaskCard.vue'
import MonitorTargetAssetSelector from './MonitorTargetAssetSelector.vue'
import { formatInvokeError, formatUptime } from './monitorPanelUtils'
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
const showRunHistoryDrawer = ref(false)
const showCreateModal = ref(false)
const editingTask = ref<any>(null)
const showDiscoverModal = ref(false)
const discovering = ref(false)
const discoverResult = ref<any>(null)
const currentDiscoverTask = ref<any>(null)
const availablePlugins = ref<any[]>([])
const loadingPlugins = ref(false)
const stoppingTaskIds = ref<Set<string>>(new Set())
const { isTaskRunning, getTaskProgress, markTaskQueued, pruneTaskProgress, loadRunningTasks, setupTaskProgressListener } = useMonitorTaskProgress()
const createEmptyPluginConfig = () => ({
  plugin_id: '',
  fallback_plugins: [] as string[],
  plugin_params: {},
  target_asset_types: [] as string[],
})

const createEmptyTaskConfig = () => ({
  enable_dns_monitoring: false,
  dns_plugins: [] as any[],
  enable_ip_monitoring: false,
  ip_plugins: [] as any[],
  enable_cert_monitoring: false,
  cert_plugins: [] as any[],
  enable_content_monitoring: false,
  content_plugins: [] as any[],
  enable_api_monitoring: false,
  api_plugins: [] as any[],
  enable_port_monitoring: false,
  port_plugins: [] as any[],
  enable_service_monitoring: false,
  service_plugins: [] as any[],
  enable_web_monitoring: false,
  web_plugins: [] as any[],
  enable_risk_monitoring: false,
  risk_plugins: [] as any[],
  auto_trigger_enabled: true,
})
const normalizeTargetAssetTypes = (value: unknown) =>
  Array.from(
    new Set(
      (Array.isArray(value) ? value : [])
        .map(assetType =>
          String(assetType || '')
            .trim()
            .toLowerCase()
        )
        .filter(Boolean)
    )
  )
const DOMAIN_HIERARCHY_TARGET_ASSET_TYPES = [
  'domain_root',
  'domain_level_1',
  'domain_level_2',
  'domain_level_3_plus',
]
const ALL_MONITOR_TARGET_ASSET_TYPES = ['web', 'domain', ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES, 'host', 'ip', 'service']

const inferDefaultTargetAssetTypes = (pluginId: string): string[] => {
  const normalizedPluginId = String(pluginId || '')
    .trim()
    .replace(/^plugin__/, '')

  switch (normalizedPluginId) {
    case 'sensitive_file_scanner':
    case 'tech_fingerprinter':
    case 'favicon_fingerprinter':
    case 'content_monitor':
    case 'api_monitor':
    case 'js_analyzer':
    case 'js_link_finder':
    case 'risk_scanner':
      return ['web']
    case 'http_prober':
      return ['web', 'domain', 'service']
    case 'subdomain_enumerator':
    case 'subdomain_brute':
      return ['domain']
    case 'dns_resolver':
      return ['domain']
    case 'cert_monitor':
    case 'ssl_scanner':
      return ['domain', 'service']
    case 'port_monitor':
    case 'cidr_mapper':
      return ['ip']
    case 'service_monitor':
    case 'service_probe':
      return ['service']
    default:
      return []
  }
}

const inferAllowedTargetAssetTypes = (monitorType: string, pluginId: string): string[] => {
  const inferred = inferDefaultTargetAssetTypes(pluginId)
  if (inferred.length > 0) {
    if (monitorType === 'dns' && inferred.includes('domain')) {
      return ['domain', ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES]
    }
    return inferred
  }

  switch (monitorType) {
    case 'dns':
      return ['domain', ...DOMAIN_HIERARCHY_TARGET_ASSET_TYPES]
    case 'cert':
      return ['domain', 'service']
    case 'ip':
      return ['domain']
    case 'port':
      return ['ip']
    case 'service':
      return ['service']
    case 'content':
    case 'api':
    case 'risk':
      return ['web']
    default:
      return ALL_MONITOR_TARGET_ASSET_TYPES
  }
}

const getAllowedTargetAssetTypes = (monitorType: string, pluginId: string) => inferAllowedTargetAssetTypes(monitorType, pluginId)

const normalizeAllowedTargetAssetTypes = (monitorType: string, pluginId: string, value: unknown) => {
  const normalized = normalizeTargetAssetTypes(value)
  const allowed = inferAllowedTargetAssetTypes(monitorType, pluginId)
  const allowedSet = new Set(allowed)
  return normalized.filter(assetType => allowedSet.has(assetType))
}

const sanitizeMonitorPluginParams = (value: unknown) => {
  const params = value && typeof value === 'object' && !Array.isArray(value) ? { ...(value as Record<string, unknown>) } : {}

  delete params.targets
  delete params.target_objects
  delete params.service_targets
  delete params.urls
  delete params.url
  delete params.domains
  delete params.domain
  delete params.__monitorExecution

  return params
}

const normalizePluginConfig = (plugin: any, monitorType = '') => {
  const pluginId = plugin?.plugin_id === 'service_fingerprinter' ? 'service_probe' : plugin?.plugin_id || ''
  return {
    plugin_id: pluginId,
    fallback_plugins: Array.isArray(plugin?.fallback_plugins) ? plugin.fallback_plugins : [],
    plugin_params: sanitizeMonitorPluginParams(plugin?.plugin_params),
    target_asset_types: normalizeAllowedTargetAssetTypes(monitorType, pluginId, plugin?.target_asset_types),
  }
}

const applyDefaultTargetAssetTypes = (plugin: any) => {
  if (!plugin) return
  plugin.target_asset_types = inferDefaultTargetAssetTypes(plugin.plugin_id)
}
const serviceProbeEnginePluginIds = new Set(['service_monitor', 'service_probe'])
const supportsServiceProbeEngine = (plugin: any) => serviceProbeEnginePluginIds.has(String(plugin?.plugin_id || '').trim())

const getServiceProbeEngine = (plugin: any) => {
  if (!supportsServiceProbeEngine(plugin)) {
    return 'native'
  }

  return 'native'
}

const setServiceProbeEngine = (plugin: any, engine: string) => {
  if (!plugin) return

  if (!plugin.plugin_params || typeof plugin.plugin_params !== 'object') {
    plugin.plugin_params = {}
  }

  plugin.plugin_params.serviceProbeEngine = 'native'
}

const handleServiceProbeEngineChange = (plugin: any, event: Event) => {
  const target = event.target as HTMLSelectElement | null
  setServiceProbeEngine(plugin, target?.value || 'native')
}

const handleServicePluginChanged = (plugin: any) => {
  if (!plugin) return

  applyDefaultTargetAssetTypes(plugin)

  if (supportsServiceProbeEngine(plugin)) {
    setServiceProbeEngine(plugin, getServiceProbeEngine(plugin))
    return
  }

  if (plugin.plugin_params && typeof plugin.plugin_params === 'object') {
    delete plugin.plugin_params.serviceProbeEngine
    if (plugin.plugin_id !== 'subdomain_brute') {
      delete plugin.plugin_params.dictionary_id
      delete plugin.plugin_params.dictionary
    }
  }
}

const normalizePluginConfigList = (plugins: unknown, monitorType = '') =>
  Array.isArray(plugins)
    ? plugins.map(plugin => {
        const normalizedPlugin = normalizePluginConfig(plugin, monitorType)
        if (supportsServiceProbeEngine(normalizedPlugin)) {
          setServiceProbeEngine(normalizedPlugin, getServiceProbeEngine(normalizedPlugin))
        }
        return normalizedPlugin
      })
    : []

const splitLegacyServicePlugins = (plugins: any[]) => {
  const portPlugins: any[] = []
  const servicePlugins: any[] = []

  for (const rawPlugin of plugins) {
    const plugin = normalizePluginConfig(rawPlugin)
    if (plugin.plugin_id === 'service_fingerprinter') {
      servicePlugins.push({
        ...plugin,
        plugin_id: 'service_probe',
      })
      continue
    }

    const fallbackPlugins = Array.isArray(plugin.fallback_plugins) ? plugin.fallback_plugins : []
    const serviceFallbacks = fallbackPlugins.filter((fallback: string) => fallback === 'service_fingerprinter' || fallback === 'service_probe')
    const retainedFallbacks = fallbackPlugins.filter((fallback: string) => fallback !== 'service_fingerprinter' && fallback !== 'service_probe')

    if (serviceFallbacks.length > 0) {
      servicePlugins.push({
        ...createEmptyPluginConfig(),
        plugin_id: 'service_probe',
        fallback_plugins: serviceFallbacks.map((fallback: string) => (fallback === 'service_fingerprinter' ? 'service_probe' : fallback)),
      })
    }

    portPlugins.push({
      ...plugin,
      fallback_plugins: retainedFallbacks,
    })
  }

  return { portPlugins, servicePlugins }
}

const splitLegacyIpPlugins = (plugins: any[]) => {
  const dnsPlugins: any[] = []
  const ipPlugins: any[] = []

  for (const rawPlugin of plugins) {
    const plugin = normalizePluginConfig(rawPlugin)
    if (plugin.plugin_id === 'dns_resolver') {
      ipPlugins.push(plugin)
      continue
    }

    const fallbackPlugins = Array.isArray(plugin.fallback_plugins) ? plugin.fallback_plugins : []
    const ipFallbacks = fallbackPlugins.filter((fallback: string) => fallback === 'dns_resolver')
    const retainedFallbacks = fallbackPlugins.filter((fallback: string) => fallback !== 'dns_resolver')

    if (ipFallbacks.length > 0) {
      ipPlugins.push({
        ...createEmptyPluginConfig(),
        plugin_id: 'dns_resolver',
        fallback_plugins: ipFallbacks,
      })
    }

    dnsPlugins.push({
      ...plugin,
      fallback_plugins: retainedFallbacks,
    })
  }

  return { dnsPlugins, ipPlugins }
}

const normalizeTaskConfig = (config: any) => {
  const normalizedDnsPlugins = normalizePluginConfigList(config?.dns_plugins, 'dns')
  const normalizedIpPlugins = normalizePluginConfigList(config?.ip_plugins, 'ip')
  const dnsLegacyMigration = normalizedIpPlugins.length === 0 ? splitLegacyIpPlugins(normalizedDnsPlugins) : { dnsPlugins: normalizedDnsPlugins, ipPlugins: normalizedIpPlugins }
  const normalizedPortPlugins = normalizePluginConfigList(config?.port_plugins, 'port')
  const normalizedServicePlugins = normalizePluginConfigList(config?.service_plugins, 'service')
  const serviceLegacyMigration = normalizedServicePlugins.length === 0 ? splitLegacyServicePlugins(normalizedPortPlugins) : { portPlugins: normalizedPortPlugins, servicePlugins: normalizedServicePlugins }

  return {
    ...createEmptyTaskConfig(),
    ...(config || {}),
    dns_plugins: dnsLegacyMigration.dnsPlugins,
    enable_ip_monitoring: Boolean(config?.enable_ip_monitoring || dnsLegacyMigration.ipPlugins.length > 0),
    ip_plugins: dnsLegacyMigration.ipPlugins,
    cert_plugins: normalizePluginConfigList(config?.cert_plugins, 'cert'),
    content_plugins: normalizePluginConfigList(config?.content_plugins, 'content'),
    api_plugins: normalizePluginConfigList(config?.api_plugins, 'api'),
    port_plugins: serviceLegacyMigration.portPlugins,
    enable_service_monitoring: Boolean(config?.enable_service_monitoring || serviceLegacyMigration.servicePlugins.length > 0),
    service_plugins: serviceLegacyMigration.servicePlugins,
    web_plugins: normalizePluginConfigList(config?.web_plugins, 'web'),
    risk_plugins: normalizePluginConfigList(config?.risk_plugins, 'risk'),
  }
}

const buildTaskConfigForSave = (config: any) => {
  const normalized = normalizeTaskConfig(config)
  const sectionMappings: Array<[string, string]> = [
    ['enable_dns_monitoring', 'dns_plugins'],
    ['enable_ip_monitoring', 'ip_plugins'],
    ['enable_cert_monitoring', 'cert_plugins'],
    ['enable_content_monitoring', 'content_plugins'],
    ['enable_api_monitoring', 'api_plugins'],
    ['enable_port_monitoring', 'port_plugins'],
    ['enable_service_monitoring', 'service_plugins'],
    ['enable_web_monitoring', 'web_plugins'],
    ['enable_risk_monitoring', 'risk_plugins'],
  ]

  for (const [enabledKey, pluginsKey] of sectionMappings) {
    if (config?.[enabledKey] === false) {
      normalized[enabledKey] = false
      normalized[pluginsKey] = []
    }
  }

  return normalized
}

const getPluginConfigs = (monitorType: string) => {
  switch (monitorType) {
    case 'dns':
      return taskForm.config.dns_plugins
    case 'ip':
      return taskForm.config.ip_plugins
    case 'cert':
      return taskForm.config.cert_plugins
    case 'content':
      return taskForm.config.content_plugins
    case 'api':
      return taskForm.config.api_plugins
    case 'port':
      return taskForm.config.port_plugins
    case 'service':
      return taskForm.config.service_plugins
    case 'web':
      return taskForm.config.web_plugins
    case 'risk':
      return taskForm.config.risk_plugins
    default:
      return []
  }
}

const taskForm = reactive({
  name: '',
  program_id: '',
  interval_secs: 6 * 3600, // 6 hours default
  config: createEmptyTaskConfig(),
})

const discoverForm = reactive({
  plugin_id: '',
  domain: '',
  urls: '',
  auto_import: true,
})

const isDiscoverFormValid = computed(() => {
  if (!discoverForm.plugin_id) return false
  if (['subdomain_enumerator', 'subdomain_brute'].includes(discoverForm.plugin_id) && !discoverForm.domain) return false
  if (discoverForm.plugin_id === 'http_prober' && !discoverForm.urls) return false
  return true
})

let unlistenChangeDetected: any = null
let unlistenSchedulerStarted: any = null
let unlistenSchedulerStopped: any = null
let unlistenTaskProgress: any = null
let unlistenPluginChanged: any = null
let lastPluginListRefreshAt = 0

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

interface LoadTasksOptions {
  retryCount?: number
  showLoading?: boolean
}

const loadTasks = async (options: LoadTasksOptions = {}) => {
  const { retryCount = 0, showLoading = tasks.value.length === 0 } = options
  try {
    if (showLoading) {
      loading.value = true
    }
    tasks.value = await invoke('monitor_list_tasks', {
      programId: props.selectedProgram?.id || null,
    })
    pruneTaskProgress(tasks.value)
  } catch (error) {
    console.error('Failed to load tasks:', error)
    // Retry once after a short delay if this is the first attempt
    if (retryCount === 0) {
      await new Promise(resolve => setTimeout(resolve, 1000))
      return loadTasks({
        retryCount: 1,
        showLoading,
      })
    }
    toast.error(t('bugBounty.errors.loadFailed'))
    // Ensure tasks is set to empty array on error to prevent stuck loading state
    if (!tasks.value || tasks.value.length === 0) {
      tasks.value = []
    }
  } finally {
    if (showLoading) {
      loading.value = false
    }
  }
}

const loadAvailablePlugins = async () => {
  try {
    loadingPlugins.value = true
    const plugins = (await invoke('monitor_get_available_plugins')) as any[]
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

const refreshAvailablePluginsOnDropdownOpen = async () => {
  const now = Date.now()
  if (loadingPlugins.value || now - lastPluginListRefreshAt < 1000) {
    return
  }

  lastPluginListRefreshAt = now
  await loadAvailablePlugins()
}

const getPluginsByType = (monitorType: string) => {
  const filtered = availablePlugins.value.filter((p: any) => p.monitor_type === monitorType)
  console.log(`Plugins for ${monitorType}:`, filtered)
  return filtered
}

const addPluginConfig = (monitorType: string) => {
  getPluginConfigs(monitorType).push(createEmptyPluginConfig())
}

const removePluginConfig = (monitorType: string, index: number) => {
  getPluginConfigs(monitorType).splice(index, 1)
}

const addFallbackPlugin = (monitorType: string, pluginIndex: number) => {
  const plugins = getPluginConfigs(monitorType)
  if (plugins[pluginIndex]) {
    plugins[pluginIndex].fallback_plugins.push('')
  }
}

const removeFallbackPlugin = (monitorType: string, pluginIndex: number, fallbackIndex: number) => {
  const plugins = getPluginConfigs(monitorType)
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
      programId: props.selectedProgram.id,
    })
    toast.success(t('bugBounty.monitor.defaultTasksCreated', { count: (taskIds as any[]).length }))
    await loadTasks({ showLoading: false })
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
    const normalizedConfig = buildTaskConfigForSave(taskForm.config)

    if (editingTask.value) {
      // Update existing task
      await invoke('monitor_update_task', {
        taskId: editingTask.value.id,
        request: {
          name: taskForm.name,
          interval_secs: taskForm.interval_secs,
          config: normalizedConfig,
        },
      })
      toast.success(t('bugBounty.monitor.taskUpdated'))
    } else {
      // Create new task
      await invoke('monitor_create_task', {
        request: {
          program_id: taskForm.program_id,
          name: taskForm.name,
          interval_secs: taskForm.interval_secs,
          config: normalizedConfig,
        },
      })
      toast.success(t('bugBounty.monitor.taskCreated'))
    }

    closeModal()
    await loadTasks({ showLoading: false })
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
    await loadTasks({ showLoading: false })
  } catch (error) {
    console.error('Failed to toggle task:', error)
    toast.error(t('bugBounty.errors.updateFailed'))
  }
}

const triggerTask = async (task: any) => {
  try {
    await invoke('monitor_trigger_task', { taskId: task.id })
    markTaskQueued(task, t('bugBounty.monitor.progressPreparing'))
    toast.success(t('bugBounty.monitor.taskTriggered'))

    // Wait a bit for task to start, then reload tasks
    await new Promise(resolve => setTimeout(resolve, 500))
    await loadTasks({ showLoading: false })
    await loadRunningTasks()
  } catch (error) {
    console.error('Failed to trigger task:', error)
    toast.error(formatInvokeError(error, t('bugBounty.errors.operationFailed')))
    // Still try to reload tasks even if trigger failed
    try {
      await loadTasks({ showLoading: false })
      await loadRunningTasks()
    } catch (e) {
      console.error('Failed to reload tasks after trigger error:', e)
    }
  }
}

const stopTask = async (task: any) => {
  try {
    stoppingTaskIds.value = new Set(stoppingTaskIds.value).add(task.id)
    await invoke('monitor_stop_task', { taskId: task.id })
    toast.success(t('bugBounty.monitor.taskStopRequested'))
    await loadRunningTasks(tasks.value)
  } catch (error) {
    console.error('Failed to stop task:', error)
    toast.error(formatInvokeError(error, t('bugBounty.errors.operationFailed')))
  } finally {
    const next = new Set(stoppingTaskIds.value)
    next.delete(task.id)
    stoppingTaskIds.value = next
  }
}

const editTask = (task: any) => {
  editingTask.value = task
  taskForm.name = task.name
  taskForm.program_id = task.program_id
  taskForm.interval_secs = task.interval_secs
  taskForm.config = normalizeTaskConfig(task.config)
}

const deleteTask = async (task: any) => {
  try {
    await invoke('monitor_delete_task', { taskId: task.id })
    toast.success(t('bugBounty.monitor.taskDeleted'))
    await loadTasks({ showLoading: false })
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

    if (['subdomain_enumerator', 'subdomain_brute'].includes(discoverForm.plugin_id)) {
      pluginInput = {
        domain: discoverForm.domain,
        removeDuplicates: true,
      }
    } else if (discoverForm.plugin_id === 'http_prober') {
      const urls = discoverForm.urls
        .split('\n')
        .map(u => u.trim())
        .filter(u => u.length > 0)
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
      },
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
  taskForm.config = createEmptyTaskConfig()
}

// Event listeners
const setupEventListeners = async () => {
  unlistenChangeDetected = await listen('monitor:change-detected', event => {
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
  unlistenPluginChanged = await listen('plugin:changed', async () => {
    await loadAvailablePlugins()
  })
  unlistenTaskProgress = await setupTaskProgressListener(() => {
    refreshStats()
    loadTasks({ showLoading: false })
  })
}

// Store refresh interval for cleanup
let refreshInterval: ReturnType<typeof setInterval> | null = null

// Lifecycle
onMounted(async () => {
  await checkSchedulerStatus()
  await refreshStats()
  await loadTasks({ showLoading: true })
  await loadRunningTasks(tasks.value)
  await loadAvailablePlugins()
  await setupEventListeners()

  // Auto-refresh stats every 30 seconds
  refreshInterval = setInterval(() => {
    if (schedulerRunning.value) {
      refreshStats()
      loadTasks()
    }
    loadRunningTasks(tasks.value)
  }, 30000)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
  unlistenChangeDetected?.()
  unlistenSchedulerStarted?.()
  unlistenSchedulerStopped?.()
  unlistenPluginChanged?.()
  unlistenTaskProgress?.()
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
  transition:
    transform 0.2s ease,
    opacity 0.2s ease;
}

.modal-enter-from .modal-box,
.modal-leave-to .modal-box {
  transform: scale(0.95);
  opacity: 0;
}
</style>
