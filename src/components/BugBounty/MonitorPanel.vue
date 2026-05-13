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
            <button type="button" class="btn btn-sm btn-outline" @click="showSeedsModal = true">
              <i class="fas fa-seedling mr-2"></i>
              {{ t('bugBounty.monitor.seeds.title') }}
            </button>
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
    <MonitorSeedsPanel
      :open="showSeedsModal"
      :selected-program="selectedProgram"
      :programs="programs"
      @close="showSeedsModal = false"
    />

    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showCreateModal || editingTask" class="modal modal-open">
          <div class="modal-box max-w-2xl max-h-[90vh] overflow-y-auto overflow-x-hidden">
            <h3 class="font-bold text-lg mb-4">
              {{ editingTask ? t('bugBounty.monitor.editTask') : t('bugBounty.monitor.createTask') }}
            </h3>

            <div class="space-y-4">
              <div class="form-control">
                <label class="label"
                  ><span class="label-text">{{ t('bugBounty.monitor.targetPrograms') }} *</span></label
                >
                <div v-if="!editingTask" class="dropdown dropdown-bottom w-full">
                  <button
                    type="button"
                    tabindex="0"
                    class="btn btn-outline w-full justify-between font-normal"
                  >
                    <span class="truncate text-left">{{ selectedProgramSummary }}</span>
                    <i class="fas fa-chevron-down text-xs opacity-60"></i>
                  </button>
                  <div
                    tabindex="0"
                    class="dropdown-content z-[60] mt-1 w-full rounded-lg border border-base-300 bg-base-100 p-2 shadow-xl"
                  >
                    <div v-if="programOptions.length === 0" class="px-3 py-4 text-sm text-base-content/60">
                      {{ t('bugBounty.monitor.noProgramsAvailable') }}
                    </div>
                    <div v-else class="max-h-64 overflow-y-auto">
                      <label
                        v-for="p in programOptions"
                        :key="p.id"
                        class="flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 hover:bg-base-200"
                      >
                        <input
                          type="checkbox"
                          class="checkbox checkbox-primary checkbox-sm"
                          :checked="taskForm.program_ids.includes(p.id)"
                          @change="toggleProgramSelection(p.id)"
                        />
                        <span class="min-w-0 flex-1 truncate text-sm font-medium">
                          {{ formatProgramOption(p) }}
                        </span>
                      </label>
                    </div>
                    <div class="mt-2 flex items-center justify-between border-t border-base-200 pt-2">
                      <span class="text-xs text-base-content/60">
                        {{ t('bugBounty.monitor.selectedProgramCount', { count: taskForm.program_ids.length }) }}
                      </span>
                      <div class="flex items-center gap-1">
                        <button type="button" class="btn btn-ghost btn-xs" @click="selectAllPrograms">
                          {{ t('bugBounty.monitor.selectAllPrograms') }}
                        </button>
                        <button type="button" class="btn btn-ghost btn-xs" @click="clearProgramSelection">
                          {{ t('bugBounty.monitor.clearSelectedPrograms') }}
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
                <select v-else v-model="taskForm.program_id" class="select select-bordered" disabled>
                  <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }} {{ p.organization ? `(${p.organization})` : '' }}</option>
                </select>
                <label v-if="!editingTask" class="label">
                  <span class="label-text-alt text-base-content/60">
                    {{ t('bugBounty.monitor.selectedProgramCount', { count: taskForm.program_ids.length }) }}
                  </span>
                </label>
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
                <div class="join w-full">
                  <input
                    v-model.number="taskForm.interval_value"
                    type="number"
                    min="1"
                    step="1"
                    class="input input-bordered join-item w-full"
                    :placeholder="t('bugBounty.monitor.intervalValuePlaceholder')"
                  />
                  <select v-model="taskForm.interval_unit" class="select select-bordered join-item w-32">
                    <option value="minutes">{{ t('bugBounty.monitor.minutes') }}</option>
                    <option value="hours">{{ t('bugBounty.monitor.hours') }}</option>
                  </select>
                </div>
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

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_dns_monitoring"
                :title="t('bugBounty.monitor.dnsMonitoring')"
                icon-class="fas fa-network-wired"
                monitor-type="dns"
                :program-id="configProgramId"
                :plugins="taskForm.config.dns_plugins"
                :plugin-options="getPluginsByType('dns')"
                @add-plugin="addPluginConfig('dns')"
                @remove-plugin="idx => removePluginConfig('dns', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_ip_monitoring"
                :title="t('bugBounty.monitor.ipMonitoring')"
                icon-class="fas fa-diagram-project"
                monitor-type="ip"
                :program-id="configProgramId"
                :plugins="taskForm.config.ip_plugins"
                :plugin-options="getPluginsByType('ip')"
                intro-text="目标资产应为域名，监控的是域名解析结果中的 IP 变化。"
                @add-plugin="addPluginConfig('ip')"
                @remove-plugin="idx => removePluginConfig('ip', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_port_monitoring"
                :title="t('bugBounty.monitor.portMonitoring')"
                icon-class="fas fa-network-wired"
                monitor-type="port"
                :program-id="configProgramId"
                :plugins="taskForm.config.port_plugins"
                :plugin-options="getPluginsByType('port')"
                @add-plugin="addPluginConfig('port')"
                @remove-plugin="idx => removePluginConfig('port', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_service_monitoring"
                :title="t('bugBounty.monitor.serviceMonitoring')"
                icon-class="fas fa-server"
                monitor-type="service"
                :program-id="configProgramId"
                :plugins="taskForm.config.service_plugins"
                :plugin-options="getPluginsByType('service')"
                @add-plugin="addPluginConfig('service')"
                @remove-plugin="idx => removePluginConfig('service', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_cert_monitoring"
                :title="t('bugBounty.monitor.certMonitoring')"
                icon-class="fas fa-certificate"
                monitor-type="cert"
                :program-id="configProgramId"
                :plugins="taskForm.config.cert_plugins"
                :plugin-options="getPluginsByType('cert')"
                @add-plugin="addPluginConfig('cert')"
                @remove-plugin="idx => removePluginConfig('cert', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_web_monitoring"
                :title="t('bugBounty.monitor.webMonitoring')"
                icon-class="fas fa-globe"
                monitor-type="web"
                :program-id="configProgramId"
                :plugins="taskForm.config.web_plugins"
                :plugin-options="getPluginsByType('web')"
                @add-plugin="addPluginConfig('web')"
                @remove-plugin="idx => removePluginConfig('web', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_api_monitoring"
                :title="t('bugBounty.monitor.apiMonitoring')"
                icon-class="fas fa-plug"
                monitor-type="api"
                :program-id="configProgramId"
                :plugins="taskForm.config.api_plugins"
                :plugin-options="getPluginsByType('api')"
                @add-plugin="addPluginConfig('api')"
                @remove-plugin="idx => removePluginConfig('api', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_content_monitoring"
                :title="t('bugBounty.monitor.contentMonitoring')"
                icon-class="fas fa-file-alt"
                monitor-type="content"
                :program-id="configProgramId"
                :plugins="taskForm.config.content_plugins"
                :plugin-options="getPluginsByType('content')"
                @add-plugin="addPluginConfig('content')"
                @remove-plugin="idx => removePluginConfig('content', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />

              <MonitorTypeConfigSection
                v-model:enabled="taskForm.config.enable_risk_monitoring"
                :title="t('bugBounty.monitor.vulnMonitoring')"
                icon-class="fas fa-shield-alt"
                monitor-type="risk"
                :program-id="configProgramId"
                :plugins="taskForm.config.risk_plugins"
                :plugin-options="getPluginsByType('risk')"
                @add-plugin="addPluginConfig('risk')"
                @remove-plugin="idx => removePluginConfig('risk', idx)"
                @refresh-plugins="refreshAvailablePluginsOnDropdownOpen"
              />
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
import MonitorSeedsPanel from './MonitorSeedsPanel.vue'
import MonitorTaskCard from './MonitorTaskCard.vue'
import MonitorTypeConfigSection from './MonitorTypeConfigSection.vue'
import {
  createEmptyPluginConfig,
  mapPluginTree,
  normalizePluginConfig,
  normalizePluginConfigList,
  normalizeMonitorPluginId,
  sanitizeMonitorPluginParamsBySchema,
} from './monitorPluginConfigSupport'
import { PARAM_EDITOR_ERROR_KEY, PARAM_EDITOR_INVALID_KEY } from './monitorPluginParamsSupport'
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
const showSeedsModal = ref(false)
const showCreateModal = ref(false)
const editingTask = ref<any>(null)
const showDiscoverModal = ref(false)
const discovering = ref(false)
const discoverResult = ref<any>(null)
const currentDiscoverTask = ref<any>(null)
const availablePlugins = ref<any[]>([])
const loadingPlugins = ref(false)
const stoppingTaskIds = ref<Set<string>>(new Set())
const pluginInputSchemaCache = new Map<string, any>()
const { isTaskRunning, getTaskProgress, markTaskQueued, markTaskStopped, pruneTaskProgress, loadRunningTasks, setupTaskProgressListener } = useMonitorTaskProgress()

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

const normalizeFallbackPluginId = (fallback: any) =>
  typeof fallback === 'string'
    ? fallback
    : String(fallback?.plugin_id || '')

const splitLegacyServicePlugins = (plugins: any[]) => {
  const portPlugins: any[] = []
  const servicePlugins: any[] = []

  for (const rawPlugin of plugins) {
    const plugin = {
      ...rawPlugin,
      fallback_plugins: Array.isArray(rawPlugin?.fallback_plugins) ? [...rawPlugin.fallback_plugins] : [],
      target_asset_types: Array.isArray(rawPlugin?.target_asset_types) ? [...rawPlugin.target_asset_types] : [],
    }
    if (plugin.plugin_id === 'service_fingerprinter') {
      servicePlugins.push({
        ...plugin,
        plugin_id: 'service_probe',
      })
      continue
    }

    const fallbackPlugins = Array.isArray(plugin.fallback_plugins) ? plugin.fallback_plugins : []
    const serviceFallbacks = fallbackPlugins.filter(
      fallback => {
        const fallbackId = normalizeFallbackPluginId(fallback)
        return fallbackId === 'service_fingerprinter' || fallbackId === 'service_probe'
      }
    )
    const retainedFallbacks = fallbackPlugins.filter(
      fallback => {
        const fallbackId = normalizeFallbackPluginId(fallback)
        return fallbackId !== 'service_fingerprinter' && fallbackId !== 'service_probe'
      }
    )

    if (serviceFallbacks.length > 0) {
      servicePlugins.push({
        ...createEmptyPluginConfig(),
        plugin_id: 'service_probe',
        fallback_plugins: serviceFallbacks.map(fallback => {
          const fallbackId = normalizeFallbackPluginId(fallback)
          const normalizedId = fallbackId === 'service_fingerprinter' ? 'service_probe' : fallbackId
          return typeof fallback === 'string'
            ? createEmptyPluginConfig()
            : {
                ...fallback,
                plugin_id: normalizedId,
              }
        }).map((fallback, index) =>
          typeof serviceFallbacks[index] === 'string'
            ? { ...createEmptyPluginConfig(), plugin_id: fallback.plugin_id }
            : fallback
        ),
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
    const plugin = {
      ...rawPlugin,
      fallback_plugins: Array.isArray(rawPlugin?.fallback_plugins) ? [...rawPlugin.fallback_plugins] : [],
      target_asset_types: Array.isArray(rawPlugin?.target_asset_types) ? [...rawPlugin.target_asset_types] : [],
    }
    if (plugin.plugin_id === 'dns_resolver') {
      ipPlugins.push(plugin)
      continue
    }

    const fallbackPlugins = Array.isArray(plugin.fallback_plugins) ? plugin.fallback_plugins : []
    const ipFallbacks = fallbackPlugins.filter(
      fallback => normalizeFallbackPluginId(fallback) === 'dns_resolver'
    )
    const retainedFallbacks = fallbackPlugins.filter(
      fallback => normalizeFallbackPluginId(fallback) !== 'dns_resolver'
    )

    if (ipFallbacks.length > 0) {
      ipPlugins.push({
        ...createEmptyPluginConfig(),
        plugin_id: 'dns_resolver',
        fallback_plugins: ipFallbacks.map(fallback =>
          typeof fallback === 'string'
            ? { ...createEmptyPluginConfig(), plugin_id: fallback }
            : fallback
        ),
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

const loadPluginInputSchema = async (pluginId: string) => {
  const normalizedPluginId = normalizeMonitorPluginId(pluginId)
  if (!normalizedPluginId) {
    return { type: 'object', properties: {} }
  }

  if (pluginInputSchemaCache.has(normalizedPluginId)) {
    return pluginInputSchemaCache.get(normalizedPluginId)
  }

  const response = await invoke<any>('get_plugin_input_schema', { pluginId: normalizedPluginId })
  if (response?.success === false) {
    throw new Error(response?.error || `Failed to load input schema for ${normalizedPluginId}`)
  }
  const resolvedSchema =
    response?.success && response?.data && typeof response.data === 'object'
      ? response.data
      : response && typeof response === 'object'
        ? response
        : { type: 'object', properties: {} }

  pluginInputSchemaCache.set(normalizedPluginId, resolvedSchema)
  return resolvedSchema
}

const sanitizePluginTreesBySchema = async (config: any) => {
  const pluginIds = new Set<string>()
  const pluginCollections = [
    config?.dns_plugins,
    config?.ip_plugins,
    config?.cert_plugins,
    config?.content_plugins,
    config?.api_plugins,
    config?.port_plugins,
    config?.service_plugins,
    config?.web_plugins,
    config?.risk_plugins,
  ]

  for (const plugins of pluginCollections) {
    mapPluginTree(Array.isArray(plugins) ? plugins : [], plugin => {
      const normalizedPluginId = normalizeMonitorPluginId(plugin?.plugin_id || '')
      if (normalizedPluginId) {
        pluginIds.add(normalizedPluginId)
      }
    })
  }

  const schemaEntries = await Promise.all(
    Array.from(pluginIds).map(async pluginId => [pluginId, await loadPluginInputSchema(pluginId)] as const)
  )
  const schemaMap = new Map<string, any>(schemaEntries)

  for (const plugins of pluginCollections) {
    mapPluginTree(Array.isArray(plugins) ? plugins : [], plugin => {
      const normalizedPluginId = normalizeMonitorPluginId(plugin?.plugin_id || '')
      plugin.plugin_params = sanitizeMonitorPluginParamsBySchema(
        plugin?.plugin_params,
        schemaMap.get(normalizedPluginId),
      )
    })
  }
}

const buildTaskConfigForSave = async (config: any) => {
  const normalized = normalizeTaskConfig(config)
  await sanitizePluginTreesBySchema(normalized)
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

const collectParamValidationErrors = (config: any) => {
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

  const errors: string[] = []

  for (const [enabledKey, pluginsKey] of sectionMappings) {
    if (!config?.[enabledKey]) {
      continue
    }

    mapPluginTree(config?.[pluginsKey] || [], plugin => {
      if (!plugin?.[PARAM_EDITOR_INVALID_KEY]) {
        return
      }

      const pluginLabel = String(plugin.plugin_id || t('bugBounty.monitor.primaryPlugin'))
      const pluginErrors = Array.isArray(plugin[PARAM_EDITOR_ERROR_KEY]) ? plugin[PARAM_EDITOR_ERROR_KEY] : []
      if (pluginErrors.length === 0) {
        errors.push(pluginLabel)
        return
      }

      errors.push(`${pluginLabel}: ${pluginErrors[0]}`)
    })
  }

  return errors
}

const collectSeedValidationErrors = (config: any) => {
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

  const errors: string[] = []

  for (const [enabledKey, pluginsKey] of sectionMappings) {
    if (!config?.[enabledKey]) {
      continue
    }

    mapPluginTree(config?.[pluginsKey] || [], plugin => {
      const pluginMeta = availablePlugins.value.find(item => item.id === plugin?.plugin_id)
      const declaredBindings = Array.isArray(pluginMeta?.seed_bindings) ? pluginMeta.seed_bindings : []
      if (declaredBindings.length === 0) {
        return
      }

      const configuredBindings = Array.isArray(plugin?.seed_config?.bindings)
        ? plugin.seed_config.bindings
        : []

      const hasAnySeedValue = configuredBindings.some((binding: any) => {
        const selected = Array.isArray(binding?.selected_project_values) ? binding.selected_project_values : []
        const manual = Array.isArray(binding?.manual_values) ? binding.manual_values : []
        return selected.length > 0 || manual.length > 0
      })

      if (!hasAnySeedValue) {
        errors.push(`${pluginMeta?.name || plugin?.plugin_id}: 未配置任何发现种子`)
      }
    })
  }

  return errors
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

type IntervalUnit = 'minutes' | 'hours'

const INTERVAL_UNIT_SECONDS: Record<IntervalUnit, number> = {
  minutes: 60,
  hours: 3600,
}

const taskForm = reactive({
  name: '',
  program_id: '',
  program_ids: [] as string[],
  interval_value: 6,
  interval_unit: 'hours' as IntervalUnit,
  config: createEmptyTaskConfig(),
})

const resolveTaskIntervalSecs = () => {
  const intervalValue = Number(taskForm.interval_value)
  if (!Number.isInteger(intervalValue) || intervalValue < 1) {
    return null
  }
  return intervalValue * INTERVAL_UNIT_SECONDS[taskForm.interval_unit]
}

const setTaskIntervalFields = (intervalSecs: number) => {
  if (intervalSecs % INTERVAL_UNIT_SECONDS.hours === 0) {
    taskForm.interval_value = intervalSecs / INTERVAL_UNIT_SECONDS.hours
    taskForm.interval_unit = 'hours'
    return
  }

  taskForm.interval_value = intervalSecs / INTERVAL_UNIT_SECONDS.minutes
  taskForm.interval_unit = 'minutes'
}

const programOptions = computed(() => props.programs || [])

const formatProgramOption = (program: any) =>
  `${program.name}${program.organization ? ` (${program.organization})` : ''}`

const selectedProgramNames = computed(() =>
  programOptions.value
    .filter(program => taskForm.program_ids.includes(program.id))
    .map(formatProgramOption),
)

const selectedProgramSummary = computed(() => {
  if (selectedProgramNames.value.length === 0) {
    return t('bugBounty.monitor.selectProgramsPlaceholder')
  }
  if (selectedProgramNames.value.length === 1) {
    return selectedProgramNames.value[0]
  }
  return t('bugBounty.monitor.selectedProgramCount', { count: selectedProgramNames.value.length })
})

const configProgramId = computed(() => {
  if (editingTask.value) {
    return taskForm.program_id
  }
  return taskForm.program_ids.length === 1 ? taskForm.program_ids[0] : ''
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
  const filtered = availablePlugins.value.filter((p: any) => p.monitor_type === monitorType && p.is_available !== false)
  console.log(`Plugins for ${monitorType}:`, filtered)
  return filtered
}

const addPluginConfig = (monitorType: string) => {
  getPluginConfigs(monitorType).push(createEmptyPluginConfig())
}

const removePluginConfig = (monitorType: string, index: number) => {
  getPluginConfigs(monitorType).splice(index, 1)
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
    await loadRunningTasks(tasks.value)
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

const toggleProgramSelection = (programId: string) => {
  if (taskForm.program_ids.includes(programId)) {
    taskForm.program_ids = taskForm.program_ids.filter(id => id !== programId)
    return
  }
  taskForm.program_ids = [...taskForm.program_ids, programId]
}

const clearProgramSelection = () => {
  taskForm.program_ids = []
}

const selectAllPrograms = () => {
  taskForm.program_ids = programOptions.value.map(program => program.id)
}

const saveTask = async () => {
  if (!taskForm.name.trim()) return
  const selectedProgramIds = Array.from(new Set(taskForm.program_ids.filter(Boolean)))
  if (editingTask.value && !taskForm.program_id) {
    toast.error(t('bugBounty.monitor.selectProgramFirst'))
    return
  }
  if (!editingTask.value && selectedProgramIds.length === 0) {
    toast.error(t('bugBounty.monitor.selectProgramsFirst'))
    return
  }
  const intervalSecs = resolveTaskIntervalSecs()
  if (!intervalSecs) {
    toast.error(t('bugBounty.monitor.invalidInterval'))
    return
  }

  try {
    const paramValidationErrors = collectParamValidationErrors(taskForm.config)
    if (paramValidationErrors.length > 0) {
      toast.error(paramValidationErrors[0])
      return
    }
    const seedValidationErrors = collectSeedValidationErrors(taskForm.config)
    if (seedValidationErrors.length > 0) {
      toast.error(seedValidationErrors[0])
      return
    }
    submitting.value = true
    const normalizedConfig = await buildTaskConfigForSave(taskForm.config)

    if (editingTask.value) {
      // Update existing task
      await invoke('monitor_update_task', {
        taskId: editingTask.value.id,
        request: {
          name: taskForm.name,
          interval_secs: intervalSecs,
          config: normalizedConfig,
        },
      })
      toast.success(t('bugBounty.monitor.taskUpdated'))
    } else {
      const createdTaskIds = await invoke('monitor_create_tasks_for_programs', {
        request: {
          program_ids: selectedProgramIds,
          name: taskForm.name,
          interval_secs: intervalSecs,
          config: normalizedConfig,
        },
      })
      toast.success(t('bugBounty.monitor.tasksCreated', { count: (createdTaskIds as any[]).length }))
    }

    closeModal()
    await loadTasks({ showLoading: false })
  } catch (error) {
    console.error('Failed to save task:', error)
    toast.error(formatInvokeError(error, t('bugBounty.errors.saveFailed')))
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
    markTaskStopped(task.id)
    await invoke('monitor_stop_task', { taskId: task.id })
    toast.success(t('bugBounty.monitor.taskStopRequested'))
    await loadRunningTasks(tasks.value)
  } catch (error) {
    console.error('Failed to stop task:', error)
    toast.error(formatInvokeError(error, t('bugBounty.errors.operationFailed')))
    await loadRunningTasks(tasks.value)
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
  taskForm.program_ids = [task.program_id]
  setTaskIntervalFields(task.interval_secs)
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
    taskForm.program_ids = [props.selectedProgram.id]
  } else {
    taskForm.program_id = ''
    taskForm.program_ids = []
  }
  showCreateModal.value = true
}

const closeModal = () => {
  showCreateModal.value = false
  editingTask.value = null
  taskForm.name = ''
  taskForm.program_id = ''
  taskForm.program_ids = []
  taskForm.interval_value = 6
  taskForm.interval_unit = 'hours'
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
