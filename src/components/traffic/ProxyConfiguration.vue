<template>
  <div class="space-y-4">
    <!-- Proxy Listeners Section -->
    <div class="card bg-base-100 shadow-xl">
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

        <div class="flex gap-4">
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
    <dialog ref="editDialogRef" class="modal">
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
    </dialog>

    <!-- Traffic Analysis Settings -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-chart-line mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.trafficAnalysisSettings') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.trafficAnalysisSettingsDesc') }}
        </p>

        <!-- Exclude self traffic from scanning -->
        <div class="form-control mb-4">
          <label class="label cursor-pointer justify-start gap-3 py-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-primary"
              v-model="proxyConfig.exclude_self_traffic"
              @change="debouncedSave"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.excludeSelfTraffic') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.excludeSelfTrafficDesc') }}</p>
            </div>
          </label>
        </div>

        <!-- Enable traffic analysis plugin scanning -->
        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-3 py-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-primary"
              v-model="trafficAnalysisPluginEnabled"
              @change="saveTrafficAnalysisPluginEnabled"
            />
            <div>
              <span class="label-text font-medium">{{ $t('trafficAnalysis.proxyConfiguration.enableTrafficAnalysisPlugin') }}</span>
              <p class="text-xs text-base-content/60 mt-1">{{ $t('trafficAnalysis.proxyConfiguration.enableTrafficAnalysisPluginDesc') }}</p>
            </div>
          </label>
        </div>
      </div>
    </div>

    <!-- Request Interception Rules -->
    <div class="card bg-base-100 shadow-xl">
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
            <span v-if="!masterInterceptionEnabled" class="text-warning text-sm italic">{{ $t('trafficAnalysis.proxyConfiguration.masterInterceptionDisabled') }}</span>
          </label>
        </div>

        <div class="flex gap-4 mt-2">
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
    <div class="card bg-base-100 shadow-xl">
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
            <span v-if="!masterInterceptionEnabled" class="text-warning text-sm italic">{{ $t('trafficAnalysis.proxyConfiguration.masterInterceptionDisabled') }}</span>
          </label>
        </div>

        <div class="flex gap-4 mt-2">
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
    <dialog ref="ruleDialogRef" class="modal">
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
    </dialog>

    <!-- upstream proxy servers -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-server mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.upstreamProxyServers') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.upstreamProxyServersDesc') }}
        </p>

        <div class="flex gap-4">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addUpstreamProxy">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editUpstreamProxy"
              :disabled="selectedUpstreamIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeUpstreamProxy"
              :disabled="selectedUpstreamIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
          </div>
          
          <!-- Right side: table -->
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
                  @click="selectedUpstreamIndex = index"
                  @dblclick="editUpstreamProxyByIndex(index)"
                  class="cursor-pointer hover:bg-base-200"
                >
                  <td>
                    <input 
                      type="checkbox" 
                      class="checkbox checkbox-sm"
                      v-model="proxy.enabled"
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
    <!-- WebSocket Interception -->
    <div class="card bg-base-100 shadow-xl">
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

    <!-- Response Modification Rules -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-edit mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.responseModificationRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.responseModificationRulesDesc') }}
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unhideHiddenFields" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unhideHiddenFields') }}</span>
            </label>
          </div>

          <div class="form-control ml-6">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="prominentlyHighlightUnhidden" disabled />
              <span class="label-text text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.prominentlyHighlightUnhidden') }}</span>  
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="enableDisabledFields" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.enableDisabledFields') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeInputFieldLengthLimits" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeInputFieldLengthLimits') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeJavaScriptFormValidation" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeJavaScriptFormValidation') }}</span> 
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeAllJavaScript" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeAllJavaScript') }}</span> 
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- Match and Replace Rules -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-search-plus mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.matchReplaceRules') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.matchReplaceRulesDesc') }}
        </p>

        <div class="form-control mb-3">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="onlyApplyToInScope" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.onlyApplyToInScope') }}</span>
          </label>
        </div>

        <div class="flex gap-4">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addMatchReplaceRule">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editMatchReplaceRule"
              :disabled="selectedMatchReplaceIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeMatchReplaceRule"
              :disabled="selectedMatchReplaceIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveMatchReplaceRuleUp"
              :disabled="selectedMatchReplaceIndex <= 0"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveUp') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="moveMatchReplaceRuleDown"
              :disabled="selectedMatchReplaceIndex === -1 || selectedMatchReplaceIndex >= matchReplaceRules.length - 1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.moveDown') }}
            </button>
          </div>
          
          <!-- Right side: table -->
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
                  @click="selectedMatchReplaceIndex = index"
                  @dblclick="editMatchReplaceRuleByIndex(index)"
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

    <!-- Upstream Proxy Servers -->


    <!-- Upstream Proxy Edit Dialog -->
    <dialog ref="upstreamDialogRef" class="modal">
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
    </dialog>

    <!-- Match and Replace Rule Edit Dialog -->
    <dialog ref="matchReplaceDialogRef" class="modal">
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
              class="input input-bordered w-full"
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
    </dialog>

    <!-- TLS Pass Through Edit Dialog -->
    <dialog ref="tlsPassThroughDialogRef" class="modal">
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
    </dialog>

    <!-- CA Certificate Import/Export Dialog -->
    <dialog ref="certDialogRef" class="modal">
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
    </dialog>

    <!-- TLS Pass Through -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-lock mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.tlsPassThrough') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.tlsPassThroughDesc') }}
        </p>

        <div class="flex gap-4">
          <!-- Left side: buttons -->
          <div class="flex flex-col gap-2 shrink-0">
            <button class="btn btn-sm btn-outline w-24" @click="addTlsPassThroughRule">
              {{ $t('trafficAnalysis.proxyConfiguration.add') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="editTlsPassThroughRule"
              :disabled="selectedTlsPassThroughIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.edit') }}
            </button>
            <button 
              class="btn btn-sm btn-outline w-24" 
              @click="removeTlsPassThroughRule"
              :disabled="selectedTlsPassThroughIndex === -1"
            >
              {{ $t('trafficAnalysis.proxyConfiguration.remove') }}
            </button>
            <button class="btn btn-sm btn-outline w-24" @click="pasteUrlToTlsPassThrough">
              {{ $t('trafficAnalysis.proxyConfiguration.pasteURL') }}
            </button>
          </div>
          
          <!-- Right side: table -->
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
                  @click="selectedTlsPassThroughIndex = index"
                  @dblclick="editTlsPassThroughRuleByIndex(index)"
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
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoAddTLSOnFailure" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.autoAddTLSOnFailure') }}</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="applyToOutOfScope" disabled />
            <span class="label-text text-base-content/50">{{ $t('trafficAnalysis.proxyConfiguration.applyToOutOfScope') }}</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Proxy History Logging -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-history mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.proxyHistoryLogging') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.proxyHistoryLoggingDesc') }}
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="historyLogging" 
                class="radio radio-sm"
                value="stop"
                v-model="historyLogging"
                checked
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stopLoggingOutOfScope') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="historyLogging" 
                class="radio radio-sm"
                value="ask"
                v-model="historyLogging"
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.askUser') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input 
                type="radio" 
                name="historyLogging" 
                class="radio radio-sm"
                value="nothing"
                v-model="historyLogging"
              />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.doNothing') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- Default Proxy Interception State -->
    <div class="card bg-base-100 shadow-xl">
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

    <!-- Miscellaneous Settings -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-cogs mr-2"></i>
          {{ $t('trafficAnalysis.proxyConfiguration.miscellaneousSettings') }}
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          {{ $t('trafficAnalysis.proxyConfiguration.miscellaneousSettingsDesc') }}
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="useHTTP1_1ToServer" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.useHTTP1_1ToServer') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="useHTTP1_1ToClient" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.useHTTP1_1ToClient') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="setConnectionClose" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.setConnectionClose') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="setConnectionHeader" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.setConnectionHeader') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="stripProxyHeaders" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stripProxyHeaders') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeUnsupportedEncodings" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.removeUnsupportedEncodings') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="stripWebSocketExtensions" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.stripWebSocketExtensions') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unpackCompressedRequests" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unpackCompressedRequests') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unpackCompressedResponses" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.unpackCompressedResponses') }}</span>
            </label>
          </div>


          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="suppressBurpErrorMessages" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.suppressBurpErrorMessages') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="dontSendToProxyHistory" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.dontSendToProxyHistory') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="dontSendToProxyHistoryIfOutOfScope" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.dontSendToProxyHistoryIfOutOfScope') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>


    <!-- Reset Button -->
    <div class="flex justify-end gap-2">
      <button class="btn btn-outline" @click="resetToDefaults">
        <i class="fas fa-undo mr-2"></i>
        {{ $t('trafficAnalysis.proxyConfiguration.resetToDefaults') }}
      </button>
      <div v-if="isSaving" class="flex items-center gap-2 text-sm text-base-content/70">
        <i class="fas fa-spinner fa-spin"></i>
        <span>{{ $t('trafficAnalysis.proxyConfiguration.saving') }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useProxyConfiguration } from './useProxyConfiguration'

const { t } = useI18n()

// Emit declaration
const emit = defineEmits<{
  (e: 'filterRuleAdded', rule: { matchType: string; condition: string; relationship: string }): void
}>()

const {
  isSaving,
  proxyConfig,
  requestBodySizeMB,
  responseBodySizeMB,
  proxyAutoStart,
  trafficAnalysisPluginEnabled,
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
  loadConfig,
  autoStartProxy,
  addRequestFilterRule,
} = useProxyConfiguration({
  t,
  emitFilterRuleAdded: rule => emit('filterRuleAdded', rule),
})

defineExpose({
  addRequestFilterRule
})
</script>

<style scoped>
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
</style>
