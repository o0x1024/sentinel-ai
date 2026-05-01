<template>
  <div class="proxy-configuration-layout flex h-full min-h-0 gap-3">
    <aside class="proxy-configuration-sidebar flex min-h-0 w-[13rem] shrink-0 flex-col rounded-[24px] border border-base-300/80 bg-base-100/95 p-2.5 shadow-sm">
      <div class="px-2 pb-3">

        <h2 class="mt-1 text-lg font-semibold text-base-content">
          代理配置
        </h2>
        <p class="mt-1 text-xs leading-5 text-base-content/60">
          左侧切换配置域，右侧集中编辑当前配置。
        </p>
      </div>

      <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-1 pb-3">
        <button
          v-for="tab in settingsTabs"
          :key="tab.id"
          type="button"
          class="proxy-settings-tab"
          :class="{ 'proxy-settings-tab-active': activeSettingsTab === tab.id }"
          @click="activeSettingsTab = tab.id"
        >
          <div class="flex items-center gap-3">
            <span class="proxy-settings-tab-icon">
              <i :class="tab.icon"></i>
            </span>
            <div class="min-w-0 text-left">
              <div class="truncate text-sm font-semibold">
                {{ tab.label }}
              </div>
              <div class="mt-1 text-xs text-base-content/55">
                {{ tab.description }}
              </div>
            </div>
          </div>
        </button>
      </div>

      <div class="rounded-[20px] border border-base-300/80 bg-base-200/60 p-3">
        <button class="btn btn-outline btn-sm w-full justify-start" @click="resetToDefaults">
          <i class="fas fa-undo mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.resetToDefaults') }}
        </button>
        <div v-if="isSaving" class="mt-3 flex items-center gap-2 text-sm text-base-content/70">
          <i class="fas fa-spinner fa-spin"></i>
          <span>{{ $t('trafficAnalysis.proxyConfiguration.saving') }}</span>
        </div>
      </div>
    </aside>

    <div
      ref="settingsContentRef"
      class="min-h-0 min-w-0 flex-1 overflow-y-auto pr-1"
      @scroll.passive="rememberSettingsScroll"
    >
      <div class="proxy-settings-grid" :class="activeSettingsGridClass">

    <!-- Proxy Listeners Section -->
    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-card proxy-settings-card-wide card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-network-wired mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.proxyListenersTitle') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.proxyListenersDescription') }}
        </p>

        <!-- Auto-start proxy on app launch -->
        <div class="form-control mb-4">
          <label class="label cursor-pointer justify-start gap-3 py-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-primary"
              v-model="proxyAutoStart"
              @change="saveProxyAutoStart"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.autoStartProxy') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.autoStartProxyDesc') }}</p>
            </div>
          </label>
        </div>

        <div class="proxy-settings-table-editor flex gap-4">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addListener">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editListener"
              :disabled="selectedListeners.length !== 1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeListener"
              :disabled="selectedListeners.length === 0"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
          </div>
          
          <!-- Right side: table -->
          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">
                    {{ $t('trafficAnalysis.proxyConfiguration.running') }}
                  </th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.interface') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.invisible') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.redirect') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.certificate') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.tlsProtocols') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.http2Support') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr 
                  v-for="(listener, index) in proxyListeners" 
                  :key="index"
                  :class="{ 'bg-primary/10': selectedListeners.includes(index) }"
                  @click="toggleListenerSelection(index)"
                  @dblclick="editListenerByIndex(index)"
                  class="cursor-pointer hover:bg-base-200"
                >
                  <td @click.stop>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="listener.running"
                      @change="toggleListenerRunning(listener, index)"
                    />
                  </td>
                  <td>{{ listener.interface }}</td>
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="listener.invisible"
                      disabled
                    />
                  </td>
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="listener.redirect"
                      disabled
                    />
                  </td>
                  <td>{{ listener.certificate }}</td>
                  <td>{{ listener.tlsProtocols }}</td>
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="listener.supportHTTP2"
                      disabled
                    />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="mt-4 space-y-2">
          <div class="flex gap-2">
            <button 
              class="btn btn-sm btn-outline"
              @click="openCertDialog"
            >
              <i class="fas fa-certificate mr-1"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.exportCACert') }}
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="regenerateCACert"
              :disabled="isRegeneratingCert"
            >
              <i :class="['fas fa-sync-alt mr-1', { 'fa-spin': isRegeneratingCert }]"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.regenerateCACert') }}
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="openCertDir"
              :disabled="isOpeningCertDir"
            >
              <i :class="['fas fa-folder-open mr-1', { 'fa-spin': isOpeningCertDir }]"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.openCertDir') }}
            </button>
          </div>
          <p class="text-xs text-base-content/60">
            {{ $t('trafficAnalysis.proxyConfiguration.certInfo') }}
          </p>
        </div>
      </div>
    </div>

    <!-- 编辑监听器对话框 -->
    <AppDialog ref="editDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.editListener') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.bindAddress') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingListener.host"
              class="input input-bordered"
              placeholder="127.0.0.1"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.port') }}</span>
            </label>
            <input 
              type="number" 
              v-model.number="editingListener.port"
              class="input input-bordered"
              placeholder="8080"
              min="1024"
              max="65535"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.certMode') }}</span>
            </label>
            <select v-model="editingListener.certificate" class="select select-bordered">
              <option value="Per-host">{{ $t('trafficAnalysis.proxyConfiguration.perHostCert') }}</option>
              <option value="Wildcard">{{ $t('trafficAnalysis.proxyConfiguration.wildcardCert') }}</option>
              <option value="Custom">{{ $t('trafficAnalysis.proxyConfiguration.customCert') }}</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.tlsProtocols') }}</span>
            </label>
            <select v-model="editingListener.tlsProtocols" class="select select-bordered">
              <option value="Default">{{ $t('trafficAnalysis.proxyConfiguration.defaultTLS') }}</option>
              <option value="TLS 1.2">{{ $t('trafficAnalysis.proxyConfiguration.tls12') }}</option>
              <option value="TLS 1.3">{{ $t('trafficAnalysis.proxyConfiguration.tls13') }}</option>
              <option value="TLS 1.2+1.3">{{ $t('trafficAnalysis.proxyConfiguration.tls12Plus13') }}</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.supportHTTP2') }}</span>
              <input 
                type="checkbox" 
                v-model="editingListener.supportHTTP2"
                class="checkbox"
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.invisibleMode') }}</span>
              <input 
                type="checkbox" 
                v-model="editingListener.invisible"
                class="checkbox"
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.enableRedirect') }}</span>
              <input 
                type="checkbox" 
                v-model="editingListener.redirect"
                class="checkbox"
              />
            </label>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelEdit">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button class="btn btn-primary" @click="saveEdit">{{ $t('trafficAnalysis.proxyConfiguration.save') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <div v-if="activeSettingsTab === 'analysis'" class="proxy-settings-panel-section">
      <ProxyMonitorSettingsPanel
        v-bind="settingsPanelBindings"
        :show-analysis="true"
        :show-advanced="false"
      />
    </div>

    <!-- Request Interception Rules -->
    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-card card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-filter mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.requestInterceptionRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.requestInterceptionRulesDesc') }}
        </p>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input
              type="checkbox"
              class="checkbox checkbox-sm"
              v-model="interceptRequests"
            />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.interceptRequests') }}</span>
            <span
              v-if="!masterInterceptionEnabled"
              class="text-warning text-sm italic"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.masterInterceptionDisabled') }}
            </span>
          </label>
        </div>

        <div class="mb-2 text-xs text-base-content/60">
          {{ $t('trafficAnalysis.proxyConfiguration.masterInterceptionManagedInInterceptTab') }}
        </div>

        <div class="proxy-settings-table-editor flex gap-4 mt-2">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addRequestRule">
              {{ $t('trafficAnalysis.proxyConfiguration.addRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editRequestRule"
              :disabled="selectedRequestRuleIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.editRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeRequestRule"
              :disabled="selectedRequestRuleIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.removeRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveRequestRuleUp"
              :disabled="selectedRequestRuleIndex <= 0"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveUp') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveRequestRuleDown"
              :disabled="selectedRequestRuleIndex === -1 || selectedRequestRuleIndex >= requestRules.length - 1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveDown') }}
            </button>
          </div>
          
          <!-- Right side: table -->
          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enable') }}</th>
                  <th class="w-20">{{ $t('trafficAnalysis.proxyConfiguration.operator') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.matchType') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.relationship') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.condition') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr 
                  v-for="(rule, index) in requestRules" 
                  :key="index"
                  :class="{ 'bg-primary/10': selectedRequestRuleIndex === index }"
                  @click="selectedRequestRuleIndex = index"
                  @dblclick="editRequestRuleByIndex(index)"
                  class="cursor-pointer hover:bg-base-200"
                >
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="rule.enabled"
                      @click.stop
                    />
                  </td>
                  <td>{{ rule.operator || '-' }}</td>
                  <td>{{ getMatchTypeLabel(rule.matchType) }}</td>
                  <td>{{ getRelationshipLabel(rule.relationship) }}</td>
                  <td class="font-mono text-xs max-w-xs truncate" :title="rule.condition">{{ rule.condition || '-' }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoFixNewlines" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.autoFixNewlines') }}</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoUpdateContentLength" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.autoUpdateContentLength') }}</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Response Interception Rules -->
    <div v-if="activeSettingsTab === 'listeners'" ref="responseInterceptionRulesRef" class="proxy-settings-card card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-reply mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.responseInterceptionRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.responseInterceptionRulesDesc') }}
        </p>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-sm"
              v-model="interceptResponses"
            />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.interceptResponses') }}</span>
            <span
              v-if="!masterInterceptionEnabled"
              class="text-warning text-sm italic"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.masterInterceptionManagedInInterceptTab') }}
            </span>
          </label>
        </div>
        <div class="mb-2 text-xs text-base-content/60">
          {{ $t('trafficAnalysis.proxyConfiguration.responseInterceptionDependsOnMaster') }}
        </div>

        <div class="proxy-settings-table-editor flex gap-4 mt-2">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addResponseRule">
              {{ $t('trafficAnalysis.proxyConfiguration.addRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editResponseRule"
              :disabled="selectedResponseRuleIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.editRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeResponseRule"
              :disabled="selectedResponseRuleIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.removeRule') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveResponseRuleUp"
              :disabled="selectedResponseRuleIndex <= 0"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveUp') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveResponseRuleDown"
              :disabled="selectedResponseRuleIndex === -1 || selectedResponseRuleIndex >= responseRules.length - 1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveDown') }}
            </button>
          </div>
          
          <!-- Right side: table -->
          <div class="flex-1 overflow-x-auto border border-base-300 rounded">
            <table class="table table-sm w-full">
              <thead>
                <tr>
                  <th class="w-16">{{ $t('trafficAnalysis.proxyConfiguration.enable') }}</th>
                  <th class="w-20">{{ $t('trafficAnalysis.proxyConfiguration.operator') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.matchType') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.relationship') }}</th>
                  <th>{{ $t('trafficAnalysis.proxyConfiguration.condition') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr 
                  v-for="(rule, index) in responseRules" 
                  :key="index"
                  :class="{ 'bg-primary/10': selectedResponseRuleIndex === index }"
                  @click="selectedResponseRuleIndex = index"
                  @dblclick="editResponseRuleByIndex(index)"
                  class="cursor-pointer hover:bg-base-200"
                >
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="rule.enabled"
                      @click.stop
                    />
                  </td>
                  <td>{{ rule.operator || '-' }}</td>
                  <td>{{ getMatchTypeLabel(rule.matchType) }}</td>
                  <td>{{ getRelationshipLabel(rule.relationship) }}</td>
                  <td class="font-mono text-xs max-w-xs truncate" :title="rule.condition">{{ rule.condition || '-' }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoUpdateResponseContentLength" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.autoUpdateResponseContentLength') }}</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Rule Edit Dialog -->
    <AppDialog ref="ruleDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ editingRuleIsNew ? $t('trafficAnalysis.proxyConfiguration.addInterceptionRule') : $t('trafficAnalysis.proxyConfiguration.editInterceptionRule') }}
        </h3>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.specifyRuleDetails') }}
        </p>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.booleanOperator') }}</span>
            </label>
            <select v-model="editingRule.operator" class="select select-bordered w-full">
              <option value="">-</option>
              <option value="Or">Or</option>
              <option value="And">And</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.matchType') }}</span>
            </label>
            <select v-model="editingRule.matchType" class="select select-bordered w-full">
              <option v-for="type in currentMatchTypes" :key="type.value" :value="type.value">
                {{ type.label }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.matchRelationship') }}</span>
            </label>
            <select v-model="editingRule.relationship" class="select select-bordered w-full">
              <option v-for="rel in relationshipOptions" :key="rel.value" :value="rel.value">
                {{ rel.label }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.matchCondition') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingRule.condition"
              class="input input-bordered w-full"
              :placeholder="$t('trafficAnalysis.proxyConfiguration.conditionPlaceholder')"
            />
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelRuleEdit">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button class="btn btn-primary" @click="saveRuleEdit">{{ $t('trafficAnalysis.proxyConfiguration.ok') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-card proxy-settings-card-wide card bg-base-100 shadow-xl">
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

    <!-- WebSocket Interception -->
    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-card proxy-settings-card-compact card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-exchange-alt mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.websocketInterceptionRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.websocketInterceptionRulesDesc') }}
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="interceptClientToServer" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.interceptClientToServer') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="interceptServerToClient" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.interceptServerToClient') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="onlyInterceptInScope" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.onlyInterceptInScope') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- Upstream Proxy Edit Dialog -->
    <AppDialog ref="upstreamDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ editingUpstreamIsNew ? $t('trafficAnalysis.proxyConfiguration.addUpstreamProxy') : $t('trafficAnalysis.proxyConfiguration.editUpstreamProxy') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.destinationHost') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingUpstream.destination_host"
              class="input input-bordered"
              placeholder="*"
            />
            <label class="label">
              <span class="label-text-alt text-base-content/60">{{ $t('trafficAnalysis.proxyConfiguration.destinationHostHelp') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.proxyHost') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingUpstream.proxy_host"
              class="input input-bordered"
              placeholder="127.0.0.1"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.proxyPort') }}</span>
            </label>
            <input 
              type="number" 
              v-model.number="editingUpstream.proxy_port"
              class="input input-bordered"
              placeholder="8080"
              min="1"
              max="65535"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.authType') }}</span>
            </label>
            <select v-model="editingUpstream.auth_type" class="select select-bordered">
              <option value="">{{ $t('trafficAnalysis.proxyConfiguration.authNone') }}</option>
              <option value="Basic">{{ $t('trafficAnalysis.proxyConfiguration.authBasic') }}</option>
            </select>
          </div>

          <div v-if="editingUpstream.auth_type === 'Basic'" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.username') }}</span>
              </label>
              <input 
                type="text" 
                v-model="editingUpstream.username"
                class="input input-bordered"
                placeholder=""
              />
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.password') }}</span>
              </label>
              <input 
                type="password" 
                v-model="editingUpstream.password"
                class="input input-bordered"
                placeholder=""
              />
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelUpstreamEdit">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button class="btn btn-primary" @click="saveUpstreamEdit">{{ $t('trafficAnalysis.proxyConfiguration.ok') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <!-- Match and Replace Rule Edit Dialog -->
    <AppDialog ref="matchReplaceDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ editingMatchReplaceIsNew ? $t('trafficAnalysis.proxyConfiguration.addMatchReplaceRule') : $t('trafficAnalysis.proxyConfiguration.editMatchReplaceRule') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.type') }}</span>
            </label>
            <select v-model="editingMatchReplace.type" class="select select-bordered w-full">
              <option value="Request header">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.requestHeader') }}</option>
              <option value="Request body">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.requestBody') }}</option>
              <option value="Request param name">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.requestParamName') }}</option>
              <option value="Request param value">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.requestParamValue') }}</option>
              <option value="Request first line">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.requestFirstLine') }}</option>
              <option value="Response header">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.responseHeader') }}</option>
              <option value="Response body">{{ $t('trafficAnalysis.proxyConfiguration.matchReplaceTypes.responseBody') }}</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.match') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingMatchReplace.match"
              class="input input-bordered w-full font-mono"
              :placeholder="$t('trafficAnalysis.proxyConfiguration.matchPlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.replace') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingMatchReplace.replace"
              class="input input-bordered w-full font-mono"
              :placeholder="$t('trafficAnalysis.proxyConfiguration.replacePlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.comment') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingMatchReplace.comment"
              class="input input-bordered w-full"
            />
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelMatchReplaceEdit">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button class="btn btn-primary" @click="saveMatchReplaceEdit">{{ $t('trafficAnalysis.proxyConfiguration.ok') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <!-- TLS Pass Through Edit Dialog -->
    <AppDialog ref="tlsPassThroughDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">
          {{ editingTlsIsNew ? $t('trafficAnalysis.proxyConfiguration.addTlsPassThrough') : $t('trafficAnalysis.proxyConfiguration.editTlsPassThrough') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.hostIPRange') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingTlsPassThrough.host"
              class="input input-bordered w-full"
              :placeholder="$t('trafficAnalysis.proxyConfiguration.hostPlaceholder')"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.port') }}</span>
            </label>
            <input 
              type="text" 
              v-model="editingTlsPassThrough.port"
              class="input input-bordered w-full"
              placeholder="443"
            />
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelTlsPassThroughEdit">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button class="btn btn-primary" @click="saveTlsPassThroughEdit">{{ $t('trafficAnalysis.proxyConfiguration.ok') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <!-- CA Certificate Import/Export Dialog -->
    <AppDialog ref="certDialogRef" class="modal">
      <div class="modal-box max-w-lg">
        <h3 class="font-bold text-lg mb-2">
          {{ $t('trafficAnalysis.proxyConfiguration.caCertDialogTitle') }}
        </h3>
        
        <div class="flex items-start gap-3 mb-6">
          <div class="text-info mt-1">
            <i class="fas fa-question-circle text-lg"></i>
          </div>
          <p class="text-sm text-base-content/70">
            {{ $t('trafficAnalysis.proxyConfiguration.caCertDialogDesc') }}
          </p>
        </div>
        
        <div class="space-y-4">
          <!-- Export Section -->
          <div>
            <h4 class="font-semibold mb-2">{{ $t('trafficAnalysis.proxyConfiguration.exportSection') }}</h4>
            <div class="space-y-2">
              <label class="flex items-center gap-3 cursor-pointer p-2 rounded hover:bg-base-200">
                <input 
                  type="radio" 
                  name="certOperation" 
                  value="export_der_cert"
                  v-model="certOperation"
                  class="radio radio-sm"
                />
                <span class="text-sm">{{ $t('trafficAnalysis.proxyConfiguration.certInDerFormat') }}</span>
              </label>
              <label class="flex items-center gap-3 cursor-pointer p-2 rounded hover:bg-base-200">
                <input 
                  type="radio" 
                  name="certOperation" 
                  value="export_der_key"
                  v-model="certOperation"
                  class="radio radio-sm"
                />
                <span class="text-sm">{{ $t('trafficAnalysis.proxyConfiguration.privateKeyInDerFormat') }}</span>
              </label>
              <label class="flex items-center gap-3 cursor-pointer p-2 rounded hover:bg-base-200">
                <input 
                  type="radio" 
                  name="certOperation" 
                  value="export_pkcs12"
                  v-model="certOperation"
                  class="radio radio-sm"
                />
                <span class="text-sm">{{ $t('trafficAnalysis.proxyConfiguration.certAndKeyInPkcs12') }}</span>
              </label>
            </div>
          </div>

          <!-- Import Section -->
          <div>
            <h4 class="font-semibold mb-2">{{ $t('trafficAnalysis.proxyConfiguration.importSection') }}</h4>
            <div class="space-y-2">
              <label class="flex items-center gap-3 cursor-pointer p-2 rounded hover:bg-base-200">
                <input 
                  type="radio" 
                  name="certOperation" 
                  value="import_der"
                  v-model="certOperation"
                  class="radio radio-sm"
                />
                <span class="text-sm">{{ $t('trafficAnalysis.proxyConfiguration.certAndKeyInDerFormat') }}</span>
              </label>
              <label class="flex items-center gap-3 cursor-pointer p-2 rounded hover:bg-base-200">
                <input 
                  type="radio" 
                  name="certOperation" 
                  value="import_pkcs12"
                  v-model="certOperation"
                  class="radio radio-sm"
                />
                <span class="text-sm">{{ $t('trafficAnalysis.proxyConfiguration.certAndKeyFromPkcs12') }}</span>
              </label>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeCertDialog">{{ $t('trafficAnalysis.proxyConfiguration.cancel') }}</button>
          <button 
            class="btn btn-primary" 
            @click="executeCertOperation"
            :disabled="!certOperation || isProcessingCert"
          >
            <i v-if="isProcessingCert" class="fas fa-spinner fa-spin mr-1"></i>
            {{ $t('trafficAnalysis.proxyConfiguration.next') }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>{{ $t('trafficAnalysis.proxyConfiguration.close') }}</button>
      </form>
    </AppDialog>

    <!-- Default Proxy Interception State -->
    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-card proxy-settings-card-compact card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-power-off mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.defaultInterceptionState') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.defaultInterceptionStateDesc') }}
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="interceptionState" 
                class="radio radio-sm"
                value="enable"
                v-model="interceptionState"
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.enableInterception') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="interceptionState" 
                class="radio radio-sm"
                value="disable"
                v-model="interceptionState"
                checked
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.disableInterception') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="interceptionState" 
                class="radio radio-sm"
                value="restore"
                v-model="interceptionState"
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.restoreInterceptionState') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <div v-if="activeSettingsTab === 'listeners'" class="proxy-settings-panel-section">
      <ProxyMonitorSettingsPanel
        v-bind="settingsPanelBindings"
        :show-analysis="false"
        :show-advanced="true"
      />
    </div>

        <TrafficDisplaySettingsPanel v-if="activeSettingsTab === 'display'" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import ProxyMonitorSettingsPanel from './ProxyMonitorSettingsPanel.vue'
import TrafficDisplaySettingsPanel from './TrafficDisplaySettingsPanel.vue'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProxyConfiguration } from './useProxyConfiguration'
import { setLocalStorageItem } from '@/utils/browserStorage'

const { t } = useI18n()

type SettingsTabId = 'listeners' | 'analysis' | 'display'

type SettingsTab = {
  id: SettingsTabId
  label: string
  icon: string
  description: string
}

type SettingsPanelState = {
  activeTab: SettingsTabId
  scrollTops: Record<SettingsTabId, number>
}

const settingsStateStorageKey = 'trafficAnalysis.proxyConfiguration.panelState.v1'
const defaultSettingsScrollTops: Record<SettingsTabId, number> = {
  listeners: 0,
  analysis: 0,
  display: 0,
}

const isSettingsTabId = (value: unknown): value is SettingsTabId =>
  value === 'listeners' || value === 'analysis' || value === 'display'

const normalizeSettingsPanelState = (state?: {
  activeTab?: unknown
  scrollTops?: Partial<Record<SettingsTabId, number>>
} | null): SettingsPanelState => ({
  activeTab: isSettingsTabId(state?.activeTab) ? state.activeTab : 'listeners',
  scrollTops: {
    ...defaultSettingsScrollTops,
    ...(state?.scrollTops || {}),
  },
})

const loadSettingsPanelState = () => {
  try {
    const raw = localStorage.getItem(settingsStateStorageKey)
    if (!raw) {
      return normalizeSettingsPanelState()
    }

    const parsed = JSON.parse(raw) as {
      activeTab?: unknown
      scrollTops?: Partial<Record<SettingsTabId, number>>
    }

    return normalizeSettingsPanelState({
      activeTab: isSettingsTabId(parsed.activeTab) ? parsed.activeTab : 'listeners',
      scrollTops: parsed.scrollTops,
    })
  } catch {
    return normalizeSettingsPanelState()
  }
}

const props = defineProps<{
  initialPanelState?: SettingsPanelState | null
}>()

// Emit declaration
const emit = defineEmits<{
  (e: 'filterRuleAdded', rule: { matchType: string; condition: string; relationship: string }): void
  (e: 'panelStateChanged', state: SettingsPanelState): void
}>()

const initialSettingsPanelState = normalizeSettingsPanelState(
  props.initialPanelState || loadSettingsPanelState(),
)
const activeSettingsTab = ref<SettingsTabId>(initialSettingsPanelState.activeTab)
const settingsContentRef = ref<HTMLElement | null>(null)
const settingsScrollTops = ref<Record<SettingsTabId, number>>({
  ...defaultSettingsScrollTops,
  ...initialSettingsPanelState.scrollTops,
})
const responseInterceptionRulesRef = ref<HTMLElement | null>(null)

const getSettingsPanelSnapshot = (): SettingsPanelState => {
  const currentScrollTop = settingsContentRef.value?.scrollTop
  return {
    activeTab: activeSettingsTab.value,
    scrollTops: {
      ...settingsScrollTops.value,
      [activeSettingsTab.value]: Number.isFinite(currentScrollTop)
        ? currentScrollTop || 0
        : settingsScrollTops.value[activeSettingsTab.value] || 0,
    },
  }
}

const persistSettingsPanelState = () => {
  const snapshot = getSettingsPanelSnapshot()
  settingsScrollTops.value = snapshot.scrollTops
  setLocalStorageItem(
    settingsStateStorageKey,
    JSON.stringify(snapshot),
  )
  emit('panelStateChanged', snapshot)
}

const rememberSettingsScroll = () => {
  settingsScrollTops.value[activeSettingsTab.value] = settingsContentRef.value?.scrollTop || 0
  persistSettingsPanelState()
}

const restoreSettingsScroll = async () => {
  await nextTick()
  const scrollTop = settingsScrollTops.value[activeSettingsTab.value] || 0
  requestAnimationFrame(() => {
    if (!settingsContentRef.value) return
    settingsContentRef.value.scrollTop = scrollTop
  })
}

watch(activeSettingsTab, () => {
  persistSettingsPanelState()
  void restoreSettingsScroll()
})

onMounted(() => {
  void restoreSettingsScroll()
  window.setTimeout(() => void restoreSettingsScroll(), 50)
})

const activeSettingsGridClass = computed(() => {
  if (activeSettingsTab.value === 'listeners') {
    return 'proxy-settings-grid-listeners'
  }

  if (activeSettingsTab.value === 'analysis') {
    return 'proxy-settings-grid-analysis'
  }

  return 'proxy-settings-grid-display'
})

const settingsTabs = computed<SettingsTab[]>(() => [
  {
    id: 'listeners',
    label: t('trafficAnalysis.proxyConfiguration.settingsTabListeners'),
    icon: 'fas fa-network-wired',
    description: '监听器、证书与拦截规则',
  },
  {
    id: 'analysis',
    label: t('trafficAnalysis.proxyConfiguration.settingsTabAnalysis'),
    icon: 'fas fa-chart-line',
    description: '扫描范围、行为信号与 OAST',
  },
  {
    id: 'display',
    label: t('trafficAnalysis.proxyConfiguration.settingsTabDisplay'),
    icon: 'fas fa-font',
    description: '消息展示、编码与发送入口',
  },
])

const {
  isSaving,
  proxyConfig,
  requestBodySizeMB,
  responseBodySizeMB,
  proxyAutoStart,
  trafficAnalysisPluginEnabled,
  browserExtensionBridgeUrl,
  browserExtensionDirectoryPath,
  browserExtensionBundledWithApp,
  isCopyingBrowserExtension,
  behaviorSignalSettings,
  trafficPluginRuntimeSettings,
  isSavingTrafficPluginRuntimeSettings,
  trafficOastConfig,
  trafficOastAutoSaveState,
  trafficOastLastSavedAt,
  testingTrafficOastConfig,
  lastTrafficOastTestResult,
  proxyListeners,
  selectedListeners,
  masterInterceptionEnabled,
  interceptRequests,
  interceptResponses,
  requestRules,
  responseRules,
  autoFixNewlines,
  autoUpdateContentLength,
  autoUpdateResponseContentLength,
  selectedRequestRuleIndex,
  selectedResponseRuleIndex,
  ruleDialogRef,
  editingRuleIsNew,
  editingRuleType,
  editingRule,
  currentMatchTypes,
  relationshipOptions,
  getMatchTypeLabel,
  getRelationshipLabel,
  upstreamProxy,
  upstreamProxies,
  selectedUpstreamIndex,
  upstreamDialogRef,
  editingUpstreamIsNew,
  editingUpstream,
  selectedMatchReplaceIndex,
  matchReplaceDialogRef,
  editingMatchReplaceIsNew,
  editingMatchReplace,
  selectedTlsPassThroughIndex,
  tlsPassThroughDialogRef,
  editingTlsIsNew,
  editingTlsPassThrough,
  interceptClientToServer,
  interceptServerToClient,
  onlyInterceptInScope,
  useHTTP1_1ToServer,
  useHTTP1_1ToClient,
  setConnectionClose,
  setConnectionHeader,
  stripProxyHeaders,
  removeUnsupportedEncodings,
  stripWebSocketExtensions,
  unpackCompressedRequests,
  unpackCompressedResponses,
  unhideHiddenFields,
  prominentlyHighlightUnhidden,
  enableDisabledFields,
  removeInputFieldLengthLimits,
  removeJavaScriptFormValidation,
  removeAllJavaScript,
  onlyApplyToInScope,
  matchReplaceRules,
  tlsPassThroughRules,
  autoAddTLSOnFailure,
  applyToOutOfScope,
  historyLogging,
  interceptionState,
  disableWebInterface,
  suppressBurpErrorMessages,
  dontSendToProxyHistory,
  dontSendToProxyHistoryIfOutOfScope,
  isDownloadingCert,
  isRegeneratingCert,
  isOpeningCertDir,
  certDialogRef,
  certOperation,
  isProcessingCert,
  editDialogRef,
  editingListener,
  toggleListenerSelection,
  toggleListenerRunning,
  addListener,
  editListenerByIndex,
  editListener,
  saveEdit,
  cancelEdit,
  onUpstreamProxyChange,
  addRequestRule,
  editRequestRule,
  editRequestRuleByIndex,
  removeRequestRule,
  moveRequestRuleUp,
  moveRequestRuleDown,
  addResponseRule,
  editResponseRule,
  editResponseRuleByIndex,
  removeResponseRule,
  moveResponseRuleUp,
  moveResponseRuleDown,
  saveRuleEdit,
  cancelRuleEdit,
  addUpstreamProxy,
  editUpstreamProxyByIndex,
  editUpstreamProxy,
  removeUpstreamProxy,
  saveUpstreamEdit,
  cancelUpstreamEdit,
  addMatchReplaceRule,
  editMatchReplaceRuleByIndex,
  editMatchReplaceRule,
  removeMatchReplaceRule,
  moveMatchReplaceRuleUp,
  moveMatchReplaceRuleDown,
  saveMatchReplaceEdit,
  cancelMatchReplaceEdit,
  addTlsPassThroughRule,
  editTlsPassThroughRuleByIndex,
  editTlsPassThroughRule,
  removeTlsPassThroughRule,
  saveTlsPassThroughEdit,
  cancelTlsPassThroughEdit,
  pasteUrlToTlsPassThrough,
  removeListener,
  updateRequestBodySize,
  updateResponseBodySize,
  debouncedSave,
  resetToDefaults,
  openCertDialog,
  closeCertDialog,
  executeCertOperation,
  downloadCACert,
  regenerateCACert,
  openCertDir,
  saveProxyAutoStart,
  saveTrafficAnalysisPluginEnabled,
  saveTrafficBehaviorSignalSettings,
  saveTrafficPluginRuntimeSettings,
  applyTrafficPluginRuntimePreset,
  resetTrafficPluginRuntimeSettings,
  testTrafficOastConfig,
  copyBrowserExtensionBridgeUrl,
  copyBrowserExtensionDirectory,
  copyBrowserExtensionToDirectory,
  loadConfig,
  autoStartProxy,
  addRequestFilterRule,
} = useProxyConfiguration({
  t,
  emitFilterRuleAdded: rule => emit('filterRuleAdded', rule),
})

const formatTrafficOastSavedTime = (value: string | null) => {
  if (!value) {
    return ''
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleTimeString()
}

const trafficOastAutoSaveStatusText = computed(() => {
  switch (trafficOastAutoSaveState.value) {
    case 'dirty':
      return t('trafficAnalysis.proxyConfiguration.oastAutoSavePending')
    case 'saving':
      return t('trafficAnalysis.proxyConfiguration.oastAutoSaveSaving')
    case 'saved':
      return t('trafficAnalysis.proxyConfiguration.oastAutoSaveSavedAt', {
        time: formatTrafficOastSavedTime(trafficOastLastSavedAt.value),
      })
    case 'error':
      return t('trafficAnalysis.proxyConfiguration.oastAutoSaveFailed')
    default:
      return t('trafficAnalysis.proxyConfiguration.oastAutoSaveSaved')
  }
})

const trafficOastAutoSaveStatusClass = computed(() => {
  switch (trafficOastAutoSaveState.value) {
    case 'saving':
      return 'text-info'
    case 'saved':
      return 'text-success'
    case 'error':
      return 'text-error'
    default:
      return 'text-base-content/55'
  }
})

const trafficOastAutoSaveStatusIcon = computed(() => {
  switch (trafficOastAutoSaveState.value) {
    case 'saving':
      return 'fas fa-spinner fa-spin'
    case 'saved':
      return 'fas fa-check-circle'
    case 'error':
      return 'fas fa-circle-exclamation'
    default:
      return 'fas fa-clock'
  }
})

const settingsPanelBindings = {
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
}

defineExpose({
  addRequestFilterRule,
  getSettingsPanelSnapshot,
  persistSettingsPanelState,
  async openResponseInterceptionRules() {
    activeSettingsTab.value = 'listeners'
    await nextTick()
    responseInterceptionRulesRef.value?.scrollIntoView({
      behavior: 'smooth',
      block: 'start',
    })
  },
})
</script>

<style scoped>
.proxy-settings-grid {
  display: grid;
  align-items: start;
  gap: 0.875rem;
}

.proxy-settings-grid > * {
  min-width: 0;
}

.proxy-settings-card {
  align-self: start;
  min-height: 0;
}

.proxy-settings-card :deep(.card-body) {
  padding: 1rem 1rem 1.05rem;
}

.proxy-settings-card :deep(.card-title) {
  margin-bottom: 0.4rem;
}

.proxy-settings-card :deep(.form-control .label) {
  min-height: 0;
  padding-top: 0.35rem;
  padding-bottom: 0.35rem;
}

.proxy-settings-card :deep(.table th) {
  white-space: nowrap;
}

.proxy-settings-card :deep(.btn.btn-sm) {
  min-height: 2.1rem;
}

.proxy-settings-table-editor {
  align-items: flex-start;
}

.proxy-settings-grid-display {
  display: block;
}

.proxy-settings-grid-listeners,
.proxy-settings-grid-analysis {
  grid-template-columns: minmax(0, 1fr);
}

.proxy-settings-grid-listeners > .proxy-settings-panel-section,
.proxy-settings-grid-analysis > .proxy-settings-panel-section {
  min-width: 0;
}

.proxy-settings-grid-listeners > .proxy-settings-card-wide {
  grid-column: auto;
}

.proxy-settings-grid-listeners > .proxy-settings-card {
  grid-column: auto;
}

.proxy-settings-grid-listeners > .proxy-settings-card-compact {
  grid-column: auto;
}

.proxy-settings-tab {
  width: 100%;
  border-radius: 1.25rem;
  border: 1px solid hsl(var(--b3) / 0.9);
  background: hsl(var(--b1) / 0.85);
  padding: 0.85rem 0.9rem;
  transition:
    border-color 0.18s ease,
    background-color 0.18s ease,
    transform 0.18s ease,
    box-shadow 0.18s ease;
}

.proxy-settings-tab:hover {
  border-color: hsl(var(--p) / 0.3);
  background: hsl(var(--b1));
  transform: translateX(2px);
}

.proxy-settings-tab-active {
  border-color: hsl(var(--p) / 0.5);
  background:
    linear-gradient(135deg, hsl(var(--p) / 0.12), transparent 62%),
    hsl(var(--b1));
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.08);
}

.proxy-settings-tab-icon {
  display: inline-flex;
  height: 2.25rem;
  width: 2.25rem;
  align-items: center;
  justify-content: center;
  border-radius: 0.9rem;
  background: hsl(var(--b2));
  color: hsl(var(--bc) / 0.75);
}

.proxy-settings-tab-active .proxy-settings-tab-icon {
  background: hsl(var(--p) / 0.14);
  color: hsl(var(--p));
}

.table th {
  background-color: hsl(var(--b2));
  font-weight: 600;
}

.table-sm td {
  padding: 0.5rem;
}

.font-mono {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

@media (max-width: 1440px) {
  .proxy-settings-grid-listeners > .proxy-settings-card,
  .proxy-settings-grid-listeners > .proxy-settings-card-compact {
    grid-column: 1 / -1;
  }
}

@media (max-width: 960px) {
  .proxy-configuration-layout {
    flex-direction: column;
    gap: 0.75rem;
  }

  .proxy-configuration-sidebar {
    width: 100%;
    padding: 0.75rem;
  }

  .proxy-settings-grid-listeners > .proxy-settings-card,
  .proxy-settings-grid-listeners > .proxy-settings-card-compact {
    grid-column: 1 / -1;
  }
}
</style>
