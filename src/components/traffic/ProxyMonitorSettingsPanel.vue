<template>
  <div
    v-if="showAnalysis"
    class="proxy-monitor-settings-panel proxy-monitor-settings-panel-analysis"
  >
    <div class="proxy-settings-card proxy-settings-card-analysis-main card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-chart-line mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.trafficAnalysisSettings') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.trafficAnalysisSettingsDesc') }}
        </p>

        <div class="form-control mb-4">
          <label class="label cursor-pointer justify-start gap-3 py-2">
            <input
              v-model="proxyConfig.exclude_self_traffic"
              type="checkbox"
              class="checkbox checkbox-primary"
              @change="debouncedSave"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.excludeSelfTraffic') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.excludeSelfTrafficDesc') }}</p>
            </div>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-3 py-2">
            <input
              v-model="trafficAnalysisPluginEnabled"
              type="checkbox"
              class="checkbox checkbox-primary"
              @change="saveTrafficAnalysisPluginEnabled"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.enableTrafficAnalysisPlugin') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.enableTrafficAnalysisPluginDesc') }}</p>
            </div>
          </label>
        </div>

        <div class="mt-4 rounded-lg border border-base-300 p-4 space-y-4">
          <div>
            <h3 class="font-medium">{{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeTitle') }}</h3>
            <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeDesc') }}</p>
          </div>

          <div class="grid gap-4 md:grid-cols-2">
            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.activeProbeMinHostCooldownMs') }}</span>
              <input
                v-model.number="trafficPluginRuntimeSettings.activeProbe.minHostCooldownMs"
                type="number"
                min="0"
                max="60000"
                class="input input-bordered"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.activeProbeTimeoutMs') }}</span>
              <input
                v-model.number="trafficPluginRuntimeSettings.activeProbe.timeoutMs"
                type="number"
                min="1000"
                max="60000"
                class="input input-bordered"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.activeProbeMaxConcurrentPerHost') }}</span>
              <input
                v-model.number="trafficPluginRuntimeSettings.activeProbe.maxConcurrentPerHost"
                type="number"
                min="1"
                max="16"
                class="input input-bordered"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.activeProbeJitterMinMs') }}</span>
              <input
                v-model.number="trafficPluginRuntimeSettings.activeProbe.jitterRange[0]"
                type="number"
                min="0"
                max="30000"
                class="input input-bordered"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.activeProbeJitterMaxMs') }}</span>
              <input
                v-model.number="trafficPluginRuntimeSettings.activeProbe.jitterRange[1]"
                type="number"
                min="0"
                max="30000"
                class="input input-bordered"
              />
            </label>
          </div>

          <p class="text-xs text-base-content/60">{{ $t('trafficAnalysis.proxyConfiguration.pluginRuntimeActiveProbeHint') }}</p>

          <div class="flex flex-wrap gap-2">
            <button class="btn btn-xs btn-outline" type="button" :disabled="isSavingTrafficPluginRuntimeSettings" @click="applyTrafficPluginRuntimePreset('local_fast')">
              {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetLocalFast') }}
            </button>
            <button class="btn btn-xs btn-outline" type="button" :disabled="isSavingTrafficPluginRuntimeSettings" @click="applyTrafficPluginRuntimePreset('balanced')">
              {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetBalanced') }}
            </button>
            <button class="btn btn-xs btn-outline" type="button" :disabled="isSavingTrafficPluginRuntimeSettings" @click="applyTrafficPluginRuntimePreset('conservative')">
              {{ $t('trafficAnalysis.proxyConfiguration.activeProbePresetConservative') }}
            </button>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="btn btn-sm btn-primary" type="button" :disabled="isSavingTrafficPluginRuntimeSettings" @click="saveTrafficPluginRuntimeSettings">
              <i :class="isSavingTrafficPluginRuntimeSettings ? 'fas fa-spinner fa-spin' : 'fas fa-save'"></i>
              <span>{{ $t('trafficAnalysis.proxyConfiguration.save') }}</span>
            </button>
            <button class="btn btn-sm btn-outline" type="button" :disabled="isSavingTrafficPluginRuntimeSettings" @click="resetTrafficPluginRuntimeSettings">
              {{ $t('trafficAnalysis.proxyConfiguration.resetToDefaults') }}
            </button>
          </div>
        </div>

        <div class="mt-4">
          <ProxyScopeRulesPanel
            v-model:include-rules="proxyConfig.scope_include_rules"
            v-model:exclude-rules="proxyConfig.scope_exclude_rules"
          />
        </div>

        <div class="mt-4 rounded-lg border border-base-300 p-4 space-y-3">
          <div>
            <h3 class="font-medium">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSignalSource') }}</h3>
            <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSignalSourceDesc') }}</p>
          </div>

          <label class="label cursor-pointer justify-start gap-3 py-2 items-start">
            <input
              v-model="behaviorSignalSettings.mode"
              type="radio"
              class="radio radio-primary mt-1"
              value="proxy_inferred"
              @change="saveTrafficBehaviorSignalSettings"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSourceProxyInferred') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSourceProxyInferredDesc') }}</p>
            </div>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-2 items-start">
            <input
              v-model="behaviorSignalSettings.mode"
              type="radio"
              class="radio radio-primary mt-1"
              value="browser_extension"
              @change="saveTrafficBehaviorSignalSettings"
            />
            <div class="flex-1">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSourceBrowserExtension') }}</span>
                <span class="badge badge-sm" :class="behaviorSignalSettings.browserExtensionConnected ? 'badge-success' : 'badge-ghost'">
                  {{
                    behaviorSignalSettings.browserExtensionConnected
                      ? $t('trafficAnalysis.proxyConfiguration.browserExtensionConnected')
                      : $t('trafficAnalysis.proxyConfiguration.browserExtensionDisconnected')
                  }}
                </span>
              </div>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.behaviorSourceBrowserExtensionDesc') }}</p>
              <div class="mt-2 text-[11px] text-base-content/50 space-y-1">
                <div>
                  {{ $t('trafficAnalysis.proxyConfiguration.browserExtensionBridgeUrl') }}:
                  <code class="font-mono">{{ browserExtensionBridgeUrl }}</code>
                </div>
                <div>
                  {{ $t('trafficAnalysis.proxyConfiguration.browserExtensionDirectory') }}:
                  <code class="font-mono break-all">{{ browserExtensionDirectoryPath }}</code>
                  <span class="badge badge-xs ml-2" :class="browserExtensionBundledWithApp ? 'badge-success' : 'badge-ghost'">
                    {{
                      browserExtensionBundledWithApp
                        ? $t('trafficAnalysis.proxyConfiguration.browserExtensionBundledWithApp')
                        : $t('trafficAnalysis.proxyConfiguration.browserExtensionFromWorkspace')
                    }}
                  </span>
                </div>
                <div v-if="behaviorSignalSettings.browserExtensionLastSeenAt">
                  {{ $t('trafficAnalysis.proxyConfiguration.browserExtensionLastSeenAt') }}:
                  {{ new Date(behaviorSignalSettings.browserExtensionLastSeenAt).toLocaleString() }}
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-2">
                <button class="btn btn-xs btn-outline" type="button" @click.stop="copyBrowserExtensionBridgeUrl">
                  {{ $t('trafficAnalysis.proxyConfiguration.copyBridgeUrl', '复制 Bridge 地址') }}
                </button>
                <button class="btn btn-xs btn-outline" type="button" @click.stop="copyBrowserExtensionDirectory">
                  {{ $t('trafficAnalysis.proxyConfiguration.copyExtensionDirectory', '复制扩展目录') }}
                </button>
                <button class="btn btn-xs btn-outline" type="button" :disabled="isCopyingBrowserExtension" @click.stop="copyBrowserExtensionToDirectory">
                  <i :class="isCopyingBrowserExtension ? 'fas fa-spinner fa-spin' : 'fas fa-folder-plus'"></i>
                  <span>{{ $t('trafficAnalysis.proxyConfiguration.copyExtensionToSpecificDirectory') }}</span>
                </button>
              </div>
            </div>
          </label>
        </div>

        <div class="mt-4 rounded-lg border border-base-300 p-4 space-y-4">
          <div>
            <h3 class="font-medium">{{ $t('trafficAnalysis.proxyConfiguration.oastTitle') }}</h3>
            <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.oastDesc') }}</p>
          </div>

          <label class="label cursor-pointer justify-start gap-3 py-0">
            <input v-model="trafficOastConfig.enabled" type="checkbox" class="checkbox checkbox-primary" />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.oastEnabled') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.oastEnabledDesc') }}</p>
            </div>
          </label>

          <div class="grid gap-4 md:grid-cols-2">
            <label class="form-control md:col-span-2">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.oastServerBaseUrl') }}</span>
              <input
                v-model.trim="trafficOastConfig.serverBaseUrl"
                type="text"
                class="input input-bordered"
                :placeholder="$t('trafficAnalysis.proxyConfiguration.oastServerBaseUrlPlaceholder')"
              />
            </label>

            <label class="form-control md:col-span-2">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.oastApiKey') }}</span>
              <input
                v-model.trim="trafficOastConfig.apiKey"
                type="password"
                class="input input-bordered"
                :placeholder="$t('trafficAnalysis.proxyConfiguration.oastApiKeyPlaceholder')"
              />
              <span class="label-text-alt mt-1 text-xs text-base-content/60">{{ $t('trafficAnalysis.proxyConfiguration.oastApiKeyDesc') }}</span>
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.oastPollIntervalSecs') }}</span>
              <input
                v-model.number="trafficOastConfig.pollIntervalSecs"
                type="number"
                min="5"
                max="300"
                class="input input-bordered"
              />
            </label>

            <label class="form-control">
              <span class="label-text text-xs">{{ $t('trafficAnalysis.proxyConfiguration.oastRequestTimeoutSecs') }}</span>
              <input
                v-model.number="trafficOastConfig.requestTimeoutSecs"
                type="number"
                min="3"
                max="60"
                class="input input-bordered"
              />
            </label>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="btn btn-sm btn-outline" type="button" :disabled="testingTrafficOastConfig" @click="testTrafficOastConfig">
              <i :class="testingTrafficOastConfig ? 'fas fa-spinner fa-spin' : 'fas fa-plug'"></i>
              <span>{{ $t('trafficAnalysis.proxyConfiguration.oastTestConnection') }}</span>
            </button>
          </div>
          <div class="space-y-1">
            <p class="text-xs text-base-content/60">{{ $t('trafficAnalysis.proxyConfiguration.oastAutoSaveHint') }}</p>
            <p class="flex items-center gap-2 text-xs" :class="trafficOastAutoSaveStatusClass">
              <i :class="trafficOastAutoSaveStatusIcon"></i>
              <span>{{ trafficOastAutoSaveStatusText }}</span>
            </p>
          </div>

          <div
            v-if="lastTrafficOastTestResult"
            class="rounded-lg border px-3 py-3 text-sm"
            :class="lastTrafficOastTestResult.reachable ? 'border-success/30 bg-success/10' : 'border-error/30 bg-error/10'"
          >
            <div class="font-medium">
              {{
                lastTrafficOastTestResult.reachable
                  ? $t('trafficAnalysis.proxyConfiguration.oastTestSuccess')
                  : $t('trafficAnalysis.proxyConfiguration.oastTestFailed')
              }}
            </div>
            <p class="mt-1 break-all text-xs text-base-content/80">{{ lastTrafficOastTestResult.message }}</p>
            <p v-if="lastTrafficOastTestResult.generatedToken" class="mt-2 text-xs">
              {{ $t('trafficAnalysis.proxyConfiguration.oastGeneratedToken') }}:
              <code class="font-mono">{{ lastTrafficOastTestResult.generatedToken }}</code>
            </p>
            <p v-if="lastTrafficOastTestResult.generatedFqdn" class="mt-1 text-xs">
              {{ $t('trafficAnalysis.proxyConfiguration.oastGeneratedFqdn') }}:
              <code class="font-mono">{{ lastTrafficOastTestResult.generatedFqdn }}</code>
            </p>
          </div>
        </div>
      </div>
    </div>

    <div class="proxy-settings-card proxy-settings-card-compact card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-history mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.proxyHistoryLogging') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.proxyHistoryLoggingDesc') }}</p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="historyLogging" type="radio" name="historyLogging" class="radio radio-sm" value="stop" checked />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stopLoggingOutOfScope') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="historyLogging" type="radio" name="historyLogging" class="radio radio-sm" value="ask" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.askUser') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="historyLogging" type="radio" name="historyLogging" class="radio radio-sm" value="nothing" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.doNothing') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>

  <div
    v-if="showAdvanced"
    class="proxy-monitor-settings-panel proxy-monitor-settings-panel-advanced"
  >
    <div class="proxy-settings-card card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-server mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.upstreamProxyServers') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.upstreamProxyServersDesc') }}</p>

        <div class="proxy-settings-rule-editor flex gap-4">
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addUpstreamProxy">{{ $t('trafficAnalysis.proxyConfiguration.add') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedUpstreamIndex === -1" @click="editUpstreamProxy">{{ $t('trafficAnalysis.proxyConfiguration.edit') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedUpstreamIndex === -1" @click="removeUpstreamProxy">{{ $t('trafficAnalysis.proxyConfiguration.remove') }}</button>
          </div>

          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.destinationHost') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.proxyHost') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.proxyPort') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.authType') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.username') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(proxy, index) in upstreamProxies"
                  :key="index"
                  :class="{ 'bg-primary/10': selectedUpstreamIndex === index }"
                  class="cursor-pointer hover:bg-base-200"
                  @click="selectedUpstreamIndex = index"
                  @dblclick="editUpstreamProxyByIndex(index)"
                >
                  <td>
                    <input
                      v-model="proxy.enabled"
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      @click.stop
                      @change="onUpstreamProxyChange"
                    />
                  </td>
                  <td>{{ proxy.destination_host || '*' }}</td>
                  <td>{{ proxy.proxy_host || '-' }}</td>
                  <td>{{ proxy.proxy_port || '-' }}</td>
                  <td>{{ proxy.auth_type || '-' }}</td>
                  <td>{{ proxy.username || '-' }}</td>
                </tr>
                <tr v-if="upstreamProxies.length === 0">
                  <td colspan="6" class="text-center text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.noUpstreamProxy') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <div class="proxy-settings-card proxy-settings-card-compact card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-edit mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.responseModificationRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.responseModificationRulesDesc') }}</p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="unhideHiddenFields" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unhideHiddenFields') }}</span>
            </label>
          </div>
          <div class="form-control ml-6">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="prominentlyHighlightUnhidden" type="checkbox" class="checkbox checkbox-sm" disabled />
              <span class="label-text text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.prominentlyHighlightUnhidden') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="enableDisabledFields" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.enableDisabledFields') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="removeInputFieldLengthLimits" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeInputFieldLengthLimits') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="removeJavaScriptFormValidation" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeJavaScriptFormValidation') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="removeAllJavaScript" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeAllJavaScript') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div class="proxy-settings-card proxy-settings-card-wide card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-search-plus mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.matchReplaceRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceRulesDesc') }}</p>

        <div class="form-control mb-3">
          <label class="label cursor-pointer justify-start gap-2">
            <input v-model="onlyApplyToInScope" type="checkbox" class="checkbox checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.onlyApplyToInScope') }}</span>
          </label>
        </div>

        <div class="proxy-settings-rule-editor flex gap-4">
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addMatchReplaceRule">{{ $t('trafficAnalysis.proxyConfiguration.add') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedMatchReplaceIndex === -1" @click="editMatchReplaceRule">{{ $t('trafficAnalysis.proxyConfiguration.edit') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedMatchReplaceIndex === -1" @click="removeMatchReplaceRule">{{ $t('trafficAnalysis.proxyConfiguration.remove') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedMatchReplaceIndex <= 0" @click="moveMatchReplaceRuleUp">{{ $t('trafficAnalysis.proxyConfiguration.moveUp') }}</button>
            <button
              class="btn btn-sm btn-outline w-24"
              :disabled="selectedMatchReplaceIndex === -1 || selectedMatchReplaceIndex >= matchReplaceRules.length - 1"
              @click="moveMatchReplaceRuleDown"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveDown') }}
            </button>
          </div>

          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.type') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.match') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.replace') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.comment') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(rule, index) in matchReplaceRules"
                  :key="index"
                  :class="{ 'bg-primary/10': selectedMatchReplaceIndex === index }"
                  class="cursor-pointer hover:bg-base-200"
                  @click="selectedMatchReplaceIndex = index"
                  @dblclick="editMatchReplaceRuleByIndex(index)"
                >
                  <td>
                    <input v-model="rule.enabled" type="checkbox" class="checkbox checkbox-sm" @click.stop />
                  </td>
                  <td class="text-xs">{{ rule.type }}</td>
                  <td class="font-mono text-xs max-w-xs truncate" :title="rule.match">{{ rule.match }}</td>
                  <td class="text-xs max-w-xs truncate" :title="rule.replace">{{ rule.replace }}</td>
                  <td class="text-xs">{{ rule.comment }}</td>
                </tr>
                <tr v-if="matchReplaceRules.length === 0">
                  <td colspan="5" class="text-center text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.noRules') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>

    <div class="proxy-settings-card card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-lock mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.tlsPassThrough') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.tlsPassThroughDesc') }}</p>

        <div class="proxy-settings-rule-editor flex gap-4">
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addTlsPassThroughRule">{{ $t('trafficAnalysis.proxyConfiguration.add') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedTlsPassThroughIndex === -1" @click="editTlsPassThroughRule">{{ $t('trafficAnalysis.proxyConfiguration.edit') }}</button>
            <button class="btn btn-sm btn-outline w-24" :disabled="selectedTlsPassThroughIndex === -1" @click="removeTlsPassThroughRule">{{ $t('trafficAnalysis.proxyConfiguration.remove') }}</button>
            <button class="btn btn-sm btn-outline w-24" @click="pasteUrlToTlsPassThrough">{{ $t('trafficAnalysis.proxyConfiguration.pasteURL') }}</button>
          </div>

          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enabled') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.hostIPRange') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.port') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(rule, index) in tlsPassThroughRules"
                  :key="index"
                  :class="{ 'bg-primary/10': selectedTlsPassThroughIndex === index }"
                  class="cursor-pointer hover:bg-base-200"
                  @click="selectedTlsPassThroughIndex = index"
                  @dblclick="editTlsPassThroughRuleByIndex(index)"
                >
                  <td>
                    <input v-model="rule.enabled" type="checkbox" class="checkbox checkbox-sm" @click.stop />
                  </td>
                  <td>{{ rule.host }}</td>
                  <td>{{ rule.port }}</td>
                </tr>
                <tr v-if="tlsPassThroughRules.length === 0">
                  <td colspan="3" class="text-center text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.noRules') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input v-model="autoAddTLSOnFailure" type="checkbox" class="checkbox checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.autoAddTLSOnFailure') }}</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input v-model="applyToOutOfScope" type="checkbox" class="checkbox checkbox-sm" disabled />
            <span class="label-text text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.applyToOutOfScope') }}</span>
          </label>
        </div>
      </div>
    </div>

    <div class="proxy-settings-card proxy-settings-card-compact card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-cogs mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.miscellaneousSettings') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('trafficAnalysis.proxyConfiguration.miscellaneousSettingsDesc') }}</p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="useHTTP1_1ToServer" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.useHTTP1_1ToServer') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="useHTTP1_1ToClient" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.useHTTP1_1ToClient') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="setConnectionClose" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.setConnectionClose') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="setConnectionHeader" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.setConnectionHeader') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="stripProxyHeaders" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stripProxyHeaders') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="removeUnsupportedEncodings" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeUnsupportedEncodings') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="stripWebSocketExtensions" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stripWebSocketExtensions') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="unpackCompressedRequests" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unpackCompressedRequests') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="unpackCompressedResponses" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unpackCompressedResponses') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="suppressBurpErrorMessages" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.suppressBurpErrorMessages') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="dontSendToProxyHistory" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.dontSendToProxyHistory') }}</span>
            </label>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input v-model="dontSendToProxyHistoryIfOutOfScope" type="checkbox" class="checkbox checkbox-sm" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.dontSendToProxyHistoryIfOutOfScope') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import ProxyScopeRulesPanel from './ProxyScopeRulesPanel.vue'

type Action = (...args: any[]) => void | Promise<void>

const props = withDefaults(defineProps<{
  showAnalysis?: boolean
  showAdvanced?: boolean
  proxyConfig: any
  debouncedSave: Action
  trafficAnalysisPluginEnabled: any
  saveTrafficAnalysisPluginEnabled: Action
  trafficPluginRuntimeSettings: any
  isSavingTrafficPluginRuntimeSettings: any
  applyTrafficPluginRuntimePreset: Action
  saveTrafficPluginRuntimeSettings: Action
  resetTrafficPluginRuntimeSettings: Action
  behaviorSignalSettings: any
  saveTrafficBehaviorSignalSettings: Action
  browserExtensionBridgeUrl: any
  browserExtensionDirectoryPath: any
  browserExtensionBundledWithApp: any
  copyBrowserExtensionBridgeUrl: Action
  copyBrowserExtensionDirectory: Action
  copyBrowserExtensionToDirectory: Action
  isCopyingBrowserExtension: any
  trafficOastConfig: any
  testingTrafficOastConfig: any
  testTrafficOastConfig: Action
  lastTrafficOastTestResult: any
  trafficOastAutoSaveStatusClass: any
  trafficOastAutoSaveStatusIcon: any
  trafficOastAutoSaveStatusText: any
  historyLogging: any
  upstreamProxies: any
  selectedUpstreamIndex: any
  addUpstreamProxy: Action
  editUpstreamProxy: Action
  removeUpstreamProxy: Action
  editUpstreamProxyByIndex: Action
  onUpstreamProxyChange: Action
  unhideHiddenFields: any
  prominentlyHighlightUnhidden: any
  enableDisabledFields: any
  removeInputFieldLengthLimits: any
  removeJavaScriptFormValidation: any
  removeAllJavaScript: any
  onlyApplyToInScope: any
  matchReplaceRules: any
  selectedMatchReplaceIndex: any
  addMatchReplaceRule: Action
  editMatchReplaceRule: Action
  removeMatchReplaceRule: Action
  moveMatchReplaceRuleUp: Action
  moveMatchReplaceRuleDown: Action
  editMatchReplaceRuleByIndex: Action
  tlsPassThroughRules: any
  selectedTlsPassThroughIndex: any
  addTlsPassThroughRule: Action
  editTlsPassThroughRule: Action
  removeTlsPassThroughRule: Action
  pasteUrlToTlsPassThrough: Action
  editTlsPassThroughRuleByIndex: Action
  autoAddTLSOnFailure: any
  applyToOutOfScope: any
  useHTTP1_1ToServer: any
  useHTTP1_1ToClient: any
  setConnectionClose: any
  setConnectionHeader: any
  stripProxyHeaders: any
  removeUnsupportedEncodings: any
  stripWebSocketExtensions: any
  unpackCompressedRequests: any
  unpackCompressedResponses: any
  suppressBurpErrorMessages: any
  dontSendToProxyHistory: any
  dontSendToProxyHistoryIfOutOfScope: any
}>(), {
  showAnalysis: true,
  showAdvanced: true,
})

const {
  showAnalysis,
  showAdvanced,
  proxyConfig,
  debouncedSave,
  trafficAnalysisPluginEnabled,
  saveTrafficAnalysisPluginEnabled,
  trafficPluginRuntimeSettings,
  isSavingTrafficPluginRuntimeSettings,
  applyTrafficPluginRuntimePreset,
  saveTrafficPluginRuntimeSettings,
  resetTrafficPluginRuntimeSettings,
  behaviorSignalSettings,
  saveTrafficBehaviorSignalSettings,
  browserExtensionBridgeUrl,
  browserExtensionDirectoryPath,
  browserExtensionBundledWithApp,
  copyBrowserExtensionBridgeUrl,
  copyBrowserExtensionDirectory,
  copyBrowserExtensionToDirectory,
  isCopyingBrowserExtension,
  trafficOastConfig,
  testingTrafficOastConfig,
  testTrafficOastConfig,
  lastTrafficOastTestResult,
  trafficOastAutoSaveStatusClass,
  trafficOastAutoSaveStatusIcon,
  trafficOastAutoSaveStatusText,
  historyLogging,
  upstreamProxies,
  selectedUpstreamIndex,
  addUpstreamProxy,
  editUpstreamProxy,
  removeUpstreamProxy,
  editUpstreamProxyByIndex,
  onUpstreamProxyChange,
  unhideHiddenFields,
  prominentlyHighlightUnhidden,
  enableDisabledFields,
  removeInputFieldLengthLimits,
  removeJavaScriptFormValidation,
  removeAllJavaScript,
  onlyApplyToInScope,
  matchReplaceRules,
  selectedMatchReplaceIndex,
  addMatchReplaceRule,
  editMatchReplaceRule,
  removeMatchReplaceRule,
  moveMatchReplaceRuleUp,
  moveMatchReplaceRuleDown,
  editMatchReplaceRuleByIndex,
  tlsPassThroughRules,
  selectedTlsPassThroughIndex,
  addTlsPassThroughRule,
  editTlsPassThroughRule,
  removeTlsPassThroughRule,
  pasteUrlToTlsPassThrough,
  editTlsPassThroughRuleByIndex,
  autoAddTLSOnFailure,
  applyToOutOfScope,
  useHTTP1_1ToServer,
  useHTTP1_1ToClient,
  setConnectionClose,
  setConnectionHeader,
  stripProxyHeaders,
  removeUnsupportedEncodings,
  stripWebSocketExtensions,
  unpackCompressedRequests,
  unpackCompressedResponses,
  suppressBurpErrorMessages,
  dontSendToProxyHistory,
  dontSendToProxyHistoryIfOutOfScope,
} = props
</script>

<style scoped>
.proxy-monitor-settings-panel {
  display: grid;
  align-content: start;
  align-items: start;
  gap: 0.875rem;
  min-width: 0;
}

.proxy-settings-card {
  align-self: start;
}

.proxy-settings-rule-editor {
  align-items: flex-start;
}

.proxy-monitor-settings-panel-analysis {
  grid-template-columns: minmax(0, 1fr);
}

.proxy-monitor-settings-panel-analysis > .proxy-settings-card-analysis-main {
  grid-column: auto;
}

.proxy-monitor-settings-panel-analysis > .proxy-settings-card-compact {
  grid-column: auto;
}

.proxy-monitor-settings-panel-advanced {
  grid-template-columns: minmax(0, 1fr);
}

.proxy-monitor-settings-panel-advanced > .proxy-settings-card-wide {
  grid-column: auto;
}

.proxy-monitor-settings-panel-advanced > .proxy-settings-card {
  grid-column: auto;
}

.proxy-monitor-settings-panel-advanced > .proxy-settings-card-compact {
  grid-column: auto;
}

@media (max-width: 1440px) {
  .proxy-monitor-settings-panel-analysis > .proxy-settings-card-analysis-main,
  .proxy-monitor-settings-panel-analysis > .proxy-settings-card-compact,
  .proxy-monitor-settings-panel-advanced > .proxy-settings-card,
  .proxy-monitor-settings-panel-advanced > .proxy-settings-card-compact {
    grid-column: 1 / -1;
  }
}

@media (max-width: 960px) {
  .proxy-monitor-settings-panel-analysis > .proxy-settings-card-analysis-main,
  .proxy-monitor-settings-panel-analysis > .proxy-settings-card-compact,
  .proxy-monitor-settings-panel-advanced > .proxy-settings-card,
  .proxy-monitor-settings-panel-advanced > .proxy-settings-card-compact {
    grid-column: 1 / -1;
  }
}
</style>
