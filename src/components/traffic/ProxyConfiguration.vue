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
import { ref, computed, onMounted, onUnmounted, inject, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0))

// 保存状态
const isSaving = ref(false)
const saveTimeout = ref<ReturnType<typeof setTimeout> | null>(null)
const isInitialLoad = ref(true) // 标记是否是初始加载，避免初始加载时触发保存

// Upstream proxy configuration interface
interface UpstreamProxyConfig {
  enabled: boolean
  destination_host: string
  proxy_host: string
  proxy_port: number
  auth_type: string
  username?: string
  password?: string
}

// Proxy configuration
const proxyConfig = ref({
  start_port: 8080,
  max_port_attempts: 10,
  mitm_enabled: true,
  max_request_body_size: 2 * 1024 * 1024,
  max_response_body_size: 2 * 1024 * 1024,
  upstream_proxy: null as UpstreamProxyConfig | null,
  exclude_self_traffic: true,
})

// 辅助变量：请求/响应体大小（MB）
const requestBodySizeMB = ref(2)
const responseBodySizeMB = ref(2)

// 代理自动启动开关
const proxyAutoStart = ref(false)

// Proxy listeners
const proxyListeners = ref([
  {
    running: true,
    interface: '127.0.0.1:8080',
    invisible: false,
    redirect: false,
    certificate: 'Per-host',
    tlsProtocols: 'Default',
    supportHTTP2: true
  }
])

// 选中的监听器索引列表
const selectedListeners = ref<number[]>([])

// Interception settings
const masterInterceptionEnabled = ref(false)
const interceptRequests = ref(true)
const interceptResponses = ref(false)

// Request interception rules
const requestRules = ref([
  {
    enabled: true,
    operator: '',
    matchType: 'file_extension',
    relationship: 'does_not_match',
    condition: '(^gif$|^jpg$|^png$|^css$|^js$|^ico$|^woff$)'
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: 'request',
    relationship: 'contains_parameters',
    condition: ''
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: 'http_method',
    relationship: 'does_not_match',
    condition: '(get|post)'
  },
  {
    enabled: false,
    operator: 'And',
    matchType: 'url',
    relationship: 'is_in_target_scope',
    condition: ''
  }
])

// Response interception rules
const responseRules = ref([
  {
    enabled: true,
    operator: '',
    matchType: 'content_type_header',
    relationship: 'matches',
    condition: 'text'
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: 'request',
    relationship: 'was_modified',
    condition: ''
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: 'request',
    relationship: 'was_intercepted',
    condition: ''
  },
  {
    enabled: false,
    operator: 'And',
    matchType: 'status_code',
    relationship: 'does_not_match',
    condition: '^304$'
  },
  {
    enabled: false,
    operator: 'And',
    matchType: 'url',
    relationship: 'is_in_target_scope',
    condition: ''
  }
])

// Request settings
const autoFixNewlines = ref(false)
const autoUpdateContentLength = ref(true)

// Response settings
const autoUpdateResponseContentLength = ref(true)

// Rule selection and editing
const selectedRequestRuleIndex = ref(-1)
const selectedResponseRuleIndex = ref(-1)
const ruleDialogRef = ref<HTMLDialogElement | null>(null)
const editingRuleIsNew = ref(false)
const editingRuleType = ref<'request' | 'response'>('request')
const editingRuleIndex = ref(-1)

// Rule interface
interface InterceptionRule {
  enabled: boolean
  operator: string
  matchType: string
  relationship: string
  condition: string
}

const editingRule = ref<InterceptionRule>({
  enabled: true,
  operator: '',
  matchType: 'domain_name',
  relationship: 'matches',
  condition: ''
})

// Match type values for request rules
const requestMatchTypeValues = [
  'domain_name', 'ip_address', 'protocol', 'http_method', 'url', 
  'file_extension', 'request', 'cookie_name', 'cookie_value', 
  'any_header', 'body', 'param_name', 'param_value', 'listener_port'
]

// Match type values for response rules
const responseMatchTypeValues = [
  'domain_name', 'ip_address', 'protocol', 'http_method', 'url', 
  'file_extension', 'request', 'cookie_name', 'cookie_value', 
  'any_header', 'body', 'param_name', 'param_value', 
  'status_code', 'content_type_header'
]

// Relationship values
const relationshipValues = [
  'matches', 'does_not_match', 'contains_parameters', 
  'is_in_target_scope', 'was_modified', 'was_intercepted'
]

// Computed for current match types with i18n labels
const currentMatchTypes = computed(() => {
  const values = editingRuleType.value === 'request' ? requestMatchTypeValues : responseMatchTypeValues
  return values.map(value => ({
    value,
    label: t(`trafficAnalysis.proxyConfiguration.matchTypes.${value}`)
  }))
})

// Computed for relationship options with i18n labels
const relationshipOptions = computed(() => {
  return relationshipValues.map(value => ({
    value,
    label: t(`trafficAnalysis.proxyConfiguration.relationships.${value}`)
  }))
})

// Get match type label
const getMatchTypeLabel = (value: string) => {
  return t(`trafficAnalysis.proxyConfiguration.matchTypes.${value}`, value)
}

// Get relationship label
const getRelationshipLabel = (value: string) => {
  return t(`trafficAnalysis.proxyConfiguration.relationships.${value}`, value)
}

// Upstream proxy settings - now supports multiple proxies
const upstreamProxy = ref<UpstreamProxyConfig | null>(null)
const upstreamProxies = ref<UpstreamProxyConfig[]>([])
const selectedUpstreamIndex = ref(-1)
const upstreamDialogRef = ref<HTMLDialogElement | null>(null)
const editingUpstreamIsNew = ref(false)
const editingUpstreamIndex = ref(-1)
const editingUpstream = ref<UpstreamProxyConfig>({
  enabled: false,
  destination_host: '*',
  proxy_host: '',
  proxy_port: 8080,
  auth_type: '',
  username: '',
  password: ''
})

// Match and Replace Rules selection
const selectedMatchReplaceIndex = ref(-1)
const matchReplaceDialogRef = ref<HTMLDialogElement | null>(null)
const editingMatchReplaceIsNew = ref(false)
const editingMatchReplaceIndex = ref(-1)

interface MatchReplaceRule {
  enabled: boolean
  type: string
  match: string
  replace: string
  comment: string
}

const editingMatchReplace = ref<MatchReplaceRule>({
  enabled: true,
  type: 'Request header',
  match: '',
  replace: '',
  comment: ''
})

// TLS Pass Through selection
const selectedTlsPassThroughIndex = ref(-1)
const tlsPassThroughDialogRef = ref<HTMLDialogElement | null>(null)
const editingTlsIsNew = ref(false)
const editingTlsIndex = ref(-1)

interface TlsPassThroughRule {
  enabled: boolean
  host: string
  port: string
}

const editingTlsPassThrough = ref<TlsPassThroughRule>({
  enabled: true,
  host: '',
  port: '443'
})

// WebSocket settings
const interceptClientToServer = ref(true)
const interceptServerToClient = ref(true)
const onlyInterceptInScope = ref(false)

// Miscellaneous settings
const useHTTP1_1ToServer = ref(false)
const useHTTP1_1ToClient = ref(false)
const setConnectionClose = ref(false)
const setConnectionHeader = ref(true)
const stripProxyHeaders = ref(true)
const removeUnsupportedEncodings = ref(true)
const stripWebSocketExtensions = ref(true)
const unpackCompressedRequests = ref(false)
const unpackCompressedResponses = ref(true)

// 响应修改规则
const unhideHiddenFields = ref(false)
const prominentlyHighlightUnhidden = ref(false)
const enableDisabledFields = ref(false)
const removeInputFieldLengthLimits = ref(false)
const removeJavaScriptFormValidation = ref(false)
const removeAllJavaScript = ref(false)
const onlyApplyToInScope = ref(true)

// 匹配和替换规则
const matchReplaceRules = ref<Array<{
  enabled: boolean
  type: string
  match: string
  replace: string
  scope: string
  item: string
  comment: string
}>>([
  {
    enabled: true,
    type: 'Request header',
    match: 'User-Agent:.*',
    replace: 'User-Agent: Mozilla/5.0',
    scope: 'In scope',
    item: '1',
    comment: '修改 User-Agent'
  }
])
const tlsPassThroughRules = ref<Array<{
  enabled: boolean
  destination: string
  protocol: string
  host: string
  port: string
}>>([
  {
    enabled: true,
    destination: '*',
    protocol: 'TLS',
    host: '*.example.com',
    port: '443'
  }
])
const autoAddTLSOnFailure = ref(false)
const applyToOutOfScope = ref(false)

// 代理历史日志
const historyLogging = ref(true)

// 默认代理拦截状态
const interceptionState = ref<'intercept' | 'forward'>('forward')

// 杂项设置
const disableWebInterface = ref(false)
const suppressBurpErrorMessages = ref(false)
const dontSendToProxyHistory = ref(false)
const dontSendToProxyHistoryIfOutOfScope = ref(false)

// Certificate management states
const isDownloadingCert = ref(false)
const isRegeneratingCert = ref(false)
const isOpeningCertDir = ref(false)
const certDialogRef = ref<HTMLDialogElement | null>(null)
const certOperation = ref<string>('')
const isProcessingCert = ref(false)

// 编辑监听器相关状态
const editDialogRef = ref<HTMLDialogElement | null>(null)
const editingIndex = ref(-1)
const editingListener = ref({
  host: '127.0.0.1',
  port: 8080,
  certificate: 'Per-host',
  tlsProtocols: 'Default',
  supportHTTP2: true,
  invisible: false,
  redirect: false
})



const toggleListenerSelection = (index: number) => {
  const idx = selectedListeners.value.indexOf(index)
  if (idx > -1) {
    selectedListeners.value.splice(idx, 1)
  } else {
    selectedListeners.value.push(index)
  }
}

const toggleListenerRunning = async (listener: any, index: number) => {
  try {
    if (listener.running) {
      // 启动代理监听器
      const [host, port] = listener.interface.split(':')
      const response = await invoke<any>('start_proxy_listener', { 
        host,
        port: parseInt(port),
        index
      })
      
      // 检查返回结果
      if (response.success) {
        dialog.toast.success(`代理监听器 ${listener.interface} 已启动`)
      } else {
        // 启动失败，恢复状态
        listener.running = false
        dialog.toast.error(`启动失败: ${response.error || '端口可能被占用'}`)
      }
    } else {
      // 停止代理监听器
      const response = await invoke<any>('stop_proxy_listener', { index })
      
      if (response.success) {
        dialog.toast.success(`代理监听器 ${listener.interface} 已停止`)
      } else {
        // 停止失败，恢复状态
        listener.running = true
        dialog.toast.error(`停止失败: ${response.error || '未知错误'}`)
      }
    }
  } catch (error: any) {
    console.error('Failed to toggle listener:', error)
    // 操作失败时恢复状态
    listener.running = !listener.running
    dialog.toast.error(`操作失败: ${error}`)
  }
}

const addListener = () => {
  const newPort = 8080 + proxyListeners.value.length
  proxyListeners.value.push({
    running: false,
    interface: `127.0.0.1:${newPort}`,
    invisible: false,
    redirect: false,
    certificate: 'Per-host',
    tlsProtocols: 'Default',
    supportHTTP2: true
  })
  dialog.toast.success('已添加新的监听器')
}

const editListenerByIndex = (index: number) => {
  const listener = proxyListeners.value[index]
  
  // 解析接口字符串
  const [host, portStr] = listener.interface.split(':')
  const port = parseInt(portStr)
  
  // 填充编辑表单
  editingIndex.value = index
  editingListener.value = {
    host,
    port,
    certificate: listener.certificate,
    tlsProtocols: listener.tlsProtocols,
    supportHTTP2: listener.supportHTTP2,
    invisible: listener.invisible,
    redirect: listener.redirect
  }
  
  // 打开对话框
  editDialogRef.value?.showModal()
}

const editListener = () => {
  if (selectedListeners.value.length !== 1) {
    dialog.toast.warning('请选择一个监听器进行编辑')
    return
  }
  
  const index = selectedListeners.value[0]
  editListenerByIndex(index)
}

const saveEdit = () => {
  if (editingIndex.value === -1) return
  
  // 验证端口号
  if (editingListener.value.port < 1024 || editingListener.value.port > 65535) {
    dialog.toast.error('端口号必须在 1024-65535 之间')
    return
  }
  
  // 验证地址
  if (!editingListener.value.host.trim()) {
    dialog.toast.error('绑定地址不能为空')
    return
  }
  
  // 更新监听器配置
  const listener = proxyListeners.value[editingIndex.value]
  const wasRunning = listener.running
  
  listener.interface = `${editingListener.value.host}:${editingListener.value.port}`
  listener.certificate = editingListener.value.certificate
  listener.tlsProtocols = editingListener.value.tlsProtocols
  listener.supportHTTP2 = editingListener.value.supportHTTP2
  listener.invisible = editingListener.value.invisible
  listener.redirect = editingListener.value.redirect
  
  // 同步更新 proxyConfig 中的 start_port（如果编辑的是第一个监听器）
  if (editingIndex.value === 0) {
    proxyConfig.value.start_port = editingListener.value.port
    console.log('[ProxyConfiguration] Updated start_port to:', editingListener.value.port)
  }
  
  // 如果监听器正在运行，需要重启以应用新配置
  if (wasRunning) {
    dialog.toast.warning('监听器配置已更新，请重启以应用新配置')
    listener.running = false
  }
  
  // 关闭对话框
  editDialogRef.value?.close()
  dialog.toast.success('监听器配置已保存')
  editingIndex.value = -1
  
  // 触发自动保存
  debouncedSave()
}

const cancelEdit = () => {
  editDialogRef.value?.close()
  editingIndex.value = -1
}

const onUpstreamProxyChange = () => {
  // Sync changes to proxyConfig
  if (upstreamProxies.value.length > 0) {
    upstreamProxy.value = upstreamProxies.value[0]
    proxyConfig.value.upstream_proxy = upstreamProxy.value
    debouncedSave()
  }
}

// Request rule methods
const addRequestRule = () => {
  editingRuleType.value = 'request'
  editingRuleIsNew.value = true
  editingRuleIndex.value = -1
  editingRule.value = {
    enabled: true,
    operator: requestRules.value.length > 0 ? 'And' : '',
    matchType: 'domain_name',
    relationship: 'matches',
    condition: ''
  }
  ruleDialogRef.value?.showModal()
}

const editRequestRule = () => {
  if (selectedRequestRuleIndex.value === -1) return
  editRequestRuleByIndex(selectedRequestRuleIndex.value)
}

const editRequestRuleByIndex = (index: number) => {
  editingRuleType.value = 'request'
  editingRuleIsNew.value = false
  editingRuleIndex.value = index
  const rule = requestRules.value[index]
  editingRule.value = { ...rule }
  ruleDialogRef.value?.showModal()
}

const removeRequestRule = () => {
  if (selectedRequestRuleIndex.value === -1) return
  requestRules.value.splice(selectedRequestRuleIndex.value, 1)
  selectedRequestRuleIndex.value = -1
  debouncedSave()
}

const moveRequestRuleUp = () => {
  if (selectedRequestRuleIndex.value <= 0) return
  const index = selectedRequestRuleIndex.value
  const temp = requestRules.value[index]
  requestRules.value[index] = requestRules.value[index - 1]
  requestRules.value[index - 1] = temp
  selectedRequestRuleIndex.value = index - 1
  debouncedSave()
}

const moveRequestRuleDown = () => {
  if (selectedRequestRuleIndex.value === -1 || selectedRequestRuleIndex.value >= requestRules.value.length - 1) return
  const index = selectedRequestRuleIndex.value
  const temp = requestRules.value[index]
  requestRules.value[index] = requestRules.value[index + 1]
  requestRules.value[index + 1] = temp
  selectedRequestRuleIndex.value = index + 1
  debouncedSave()
}

// Response rule methods
const addResponseRule = () => {
  editingRuleType.value = 'response'
  editingRuleIsNew.value = true
  editingRuleIndex.value = -1
  editingRule.value = {
    enabled: true,
    operator: responseRules.value.length > 0 ? 'And' : '',
    matchType: 'domain_name',
    relationship: 'matches',
    condition: ''
  }
  ruleDialogRef.value?.showModal()
}

const editResponseRule = () => {
  if (selectedResponseRuleIndex.value === -1) return
  editResponseRuleByIndex(selectedResponseRuleIndex.value)
}

const editResponseRuleByIndex = (index: number) => {
  editingRuleType.value = 'response'
  editingRuleIsNew.value = false
  editingRuleIndex.value = index
  const rule = responseRules.value[index]
  editingRule.value = { ...rule }
  ruleDialogRef.value?.showModal()
}

const removeResponseRule = () => {
  if (selectedResponseRuleIndex.value === -1) return
  responseRules.value.splice(selectedResponseRuleIndex.value, 1)
  selectedResponseRuleIndex.value = -1
  debouncedSave()
}

const moveResponseRuleUp = () => {
  if (selectedResponseRuleIndex.value <= 0) return
  const index = selectedResponseRuleIndex.value
  const temp = responseRules.value[index]
  responseRules.value[index] = responseRules.value[index - 1]
  responseRules.value[index - 1] = temp
  selectedResponseRuleIndex.value = index - 1
  debouncedSave()
}

const moveResponseRuleDown = () => {
  if (selectedResponseRuleIndex.value === -1 || selectedResponseRuleIndex.value >= responseRules.value.length - 1) return
  const index = selectedResponseRuleIndex.value
  const temp = responseRules.value[index]
  responseRules.value[index] = responseRules.value[index + 1]
  responseRules.value[index + 1] = temp
  selectedResponseRuleIndex.value = index + 1
  debouncedSave()
}

// Save and cancel rule edit
const saveRuleEdit = () => {
  const rules = editingRuleType.value === 'request' ? requestRules.value : responseRules.value
  
  if (editingRuleIsNew.value) {
    rules.push({ ...editingRule.value })
    // Select the newly added rule
    if (editingRuleType.value === 'request') {
      selectedRequestRuleIndex.value = rules.length - 1
    } else {
      selectedResponseRuleIndex.value = rules.length - 1
    }
  } else {
    rules[editingRuleIndex.value] = { ...editingRule.value }
  }
  
  ruleDialogRef.value?.close()
  debouncedSave()
}

const cancelRuleEdit = () => {
  ruleDialogRef.value?.close()
}

// Upstream Proxy methods (updated for list support)
const addUpstreamProxy = () => {
  editingUpstreamIsNew.value = true
  editingUpstreamIndex.value = -1
  editingUpstream.value = {
    enabled: true,
    destination_host: '*',
    proxy_host: '127.0.0.1',
    proxy_port: 10809,
    auth_type: '',
    username: '',
    password: ''
  }
  upstreamDialogRef.value?.showModal()
}

const editUpstreamProxyByIndex = (index: number) => {
  editingUpstreamIsNew.value = false
  editingUpstreamIndex.value = index
  const proxy = upstreamProxies.value[index]
  editingUpstream.value = { ...proxy }
  upstreamDialogRef.value?.showModal()
}

const editUpstreamProxy = () => {
  if (selectedUpstreamIndex.value === -1) return
  editUpstreamProxyByIndex(selectedUpstreamIndex.value)
}

const removeUpstreamProxy = () => {
  if (selectedUpstreamIndex.value === -1) return
  upstreamProxies.value.splice(selectedUpstreamIndex.value, 1)
  selectedUpstreamIndex.value = -1
  // Sync with single proxy for backwards compatibility
  upstreamProxy.value = upstreamProxies.value.length > 0 ? upstreamProxies.value[0] : null
  proxyConfig.value.upstream_proxy = upstreamProxy.value
  debouncedSave()
}

const saveUpstreamEdit = () => {
  if (!editingUpstream.value.proxy_host.trim()) {
    dialog.toast.error('Proxy host is required')
    return
  }
  if (editingUpstream.value.proxy_port < 1 || editingUpstream.value.proxy_port > 65535) {
    dialog.toast.error('Port must be between 1 and 65535')
    return
  }
  
  if (editingUpstreamIsNew.value) {
    upstreamProxies.value.push({ ...editingUpstream.value })
    selectedUpstreamIndex.value = upstreamProxies.value.length - 1
  } else {
    upstreamProxies.value[editingUpstreamIndex.value] = { ...editingUpstream.value }
  }
  
  // Sync with single proxy for backwards compatibility
  upstreamProxy.value = upstreamProxies.value.length > 0 ? upstreamProxies.value[0] : null
  proxyConfig.value.upstream_proxy = upstreamProxy.value
  
  upstreamDialogRef.value?.close()
  dialog.toast.success('Upstream proxy saved')
  debouncedSave()
}

const cancelUpstreamEdit = () => {
  upstreamDialogRef.value?.close()
}

// Match and Replace Rule methods
const addMatchReplaceRule = () => {
  editingMatchReplaceIsNew.value = true
  editingMatchReplaceIndex.value = -1
  editingMatchReplace.value = {
    enabled: true,
    type: 'Request header',
    match: '',
    replace: '',
    comment: ''
  }
  matchReplaceDialogRef.value?.showModal()
}

const editMatchReplaceRuleByIndex = (index: number) => {
  editingMatchReplaceIsNew.value = false
  editingMatchReplaceIndex.value = index
  const rule = matchReplaceRules.value[index]
  editingMatchReplace.value = { 
    enabled: rule.enabled,
    type: rule.type,
    match: rule.match,
    replace: rule.replace,
    comment: rule.comment
  }
  matchReplaceDialogRef.value?.showModal()
}

const editMatchReplaceRule = () => {
  if (selectedMatchReplaceIndex.value === -1) return
  editMatchReplaceRuleByIndex(selectedMatchReplaceIndex.value)
}

const removeMatchReplaceRule = () => {
  if (selectedMatchReplaceIndex.value === -1) return
  matchReplaceRules.value.splice(selectedMatchReplaceIndex.value, 1)
  selectedMatchReplaceIndex.value = -1
  debouncedSave()
}

const moveMatchReplaceRuleUp = () => {
  if (selectedMatchReplaceIndex.value <= 0) return
  const index = selectedMatchReplaceIndex.value
  const temp = matchReplaceRules.value[index]
  matchReplaceRules.value[index] = matchReplaceRules.value[index - 1]
  matchReplaceRules.value[index - 1] = temp
  selectedMatchReplaceIndex.value = index - 1
  debouncedSave()
}

const moveMatchReplaceRuleDown = () => {
  if (selectedMatchReplaceIndex.value === -1 || selectedMatchReplaceIndex.value >= matchReplaceRules.value.length - 1) return
  const index = selectedMatchReplaceIndex.value
  const temp = matchReplaceRules.value[index]
  matchReplaceRules.value[index] = matchReplaceRules.value[index + 1]
  matchReplaceRules.value[index + 1] = temp
  selectedMatchReplaceIndex.value = index + 1
  debouncedSave()
}

const saveMatchReplaceEdit = () => {
  if (editingMatchReplaceIsNew.value) {
    matchReplaceRules.value.push({ 
      ...editingMatchReplace.value,
      scope: 'In scope',
      item: String(matchReplaceRules.value.length + 1)
    })
    selectedMatchReplaceIndex.value = matchReplaceRules.value.length - 1
  } else {
    const existing = matchReplaceRules.value[editingMatchReplaceIndex.value]
    matchReplaceRules.value[editingMatchReplaceIndex.value] = {
      ...existing,
      ...editingMatchReplace.value
    }
  }
  
  matchReplaceDialogRef.value?.close()
  debouncedSave()
}

const cancelMatchReplaceEdit = () => {
  matchReplaceDialogRef.value?.close()
}

// TLS Pass Through methods
const addTlsPassThroughRule = () => {
  editingTlsIsNew.value = true
  editingTlsIndex.value = -1
  editingTlsPassThrough.value = {
    enabled: true,
    host: '',
    port: '443'
  }
  tlsPassThroughDialogRef.value?.showModal()
}

const editTlsPassThroughRuleByIndex = (index: number) => {
  editingTlsIsNew.value = false
  editingTlsIndex.value = index
  const rule = tlsPassThroughRules.value[index]
  editingTlsPassThrough.value = { ...rule }
  tlsPassThroughDialogRef.value?.showModal()
}

const editTlsPassThroughRule = () => {
  if (selectedTlsPassThroughIndex.value === -1) return
  editTlsPassThroughRuleByIndex(selectedTlsPassThroughIndex.value)
}

const removeTlsPassThroughRule = () => {
  if (selectedTlsPassThroughIndex.value === -1) return
  tlsPassThroughRules.value.splice(selectedTlsPassThroughIndex.value, 1)
  selectedTlsPassThroughIndex.value = -1
  debouncedSave()
}

const saveTlsPassThroughEdit = () => {
  if (!editingTlsPassThrough.value.host.trim()) {
    dialog.toast.error('Host is required')
    return
  }
  
  if (editingTlsIsNew.value) {
    tlsPassThroughRules.value.push({ 
      ...editingTlsPassThrough.value,
      destination: '*',
      protocol: 'TLS'
    })
    selectedTlsPassThroughIndex.value = tlsPassThroughRules.value.length - 1
  } else {
    const existing = tlsPassThroughRules.value[editingTlsIndex.value]
    tlsPassThroughRules.value[editingTlsIndex.value] = {
      ...existing,
      ...editingTlsPassThrough.value
    }
  }
  
  tlsPassThroughDialogRef.value?.close()
  debouncedSave()
}

const cancelTlsPassThroughEdit = () => {
  tlsPassThroughDialogRef.value?.close()
}

const pasteUrlToTlsPassThrough = async () => {
  try {
    const text = await navigator.clipboard.readText()
    if (text) {
      const url = new URL(text)
      editingTlsPassThrough.value = {
        enabled: true,
        host: url.hostname,
        port: url.port || '443'
      }
      editingTlsIsNew.value = true
      tlsPassThroughDialogRef.value?.showModal()
    }
  } catch {
    dialog.toast.error('Failed to paste URL from clipboard')
  }
}

const removeListener = async () => {
  if (selectedListeners.value.length === 0) {
    dialog.toast.warning('请至少选择一个监听器')
    return
  }
  
  
  // 按索引降序排序，从后往前删除
  const sortedIndices = [...selectedListeners.value].sort((a, b) => b - a)
  
  for (const index of sortedIndices) {
    const listener = proxyListeners.value[index]
    // 如果监听器正在运行，先停止它
    if (listener.running) {
      try {
        await invoke('stop_proxy_listener', { index })
      } catch (error) {
        console.error('Failed to stop listener before removal:', error)
      }
    }
    proxyListeners.value.splice(index, 1)
  }
  
  selectedListeners.value = []
  dialog.toast.success('已删除选中的监听器')
}

// 更新请求体大小（MB -> 字节）
const updateRequestBodySize = () => {
  proxyConfig.value.max_request_body_size = requestBodySizeMB.value * 1024 * 1024
  debouncedSave()
}

// 更新响应体大小（MB -> 字节）
const updateResponseBodySize = () => {
  proxyConfig.value.max_response_body_size = responseBodySizeMB.value * 1024 * 1024
  debouncedSave()
}

// 防抖保存函数
const debouncedSave = () => {
  // 如果是初始加载阶段，不触发保存
  if (isInitialLoad.value) {
    return
  }
  
  // 清除之前的定时器
  if (saveTimeout.value) {
    clearTimeout(saveTimeout.value)
  }
  
  // 设置新的定时器（1秒后保存）
  saveTimeout.value = setTimeout(() => {
    saveConfiguration()
  }, 1000)
}

const saveConfiguration = async () => {
  try {
    isSaving.value = true
    console.log('[ProxyConfiguration] Saving configuration...', proxyConfig.value)
    
    // 保存配置到后端
    const response = await invoke<any>('save_proxy_config', { 
      config: proxyConfig.value 
    })
    
    if (response.success) {
      console.log('[ProxyConfiguration] Configuration saved successfully')
      // 静默保存，不显示提示
    } else {
      throw new Error(response.error || '保存失败')
    }
    
    // 保存请求和响应拦截规则到 localStorage
    try {
      localStorage.setItem('proxy_request_filter_rules', JSON.stringify(requestRules.value))
      localStorage.setItem('proxy_response_filter_rules', JSON.stringify(responseRules.value))
      console.log('[ProxyConfiguration] Filter rules saved to localStorage')
    } catch (e) {
      console.error('[ProxyConfiguration] Failed to save filter rules to localStorage:', e)
    }
  } catch (error: any) {
    console.error('[ProxyConfiguration] Failed to save configuration:', error)
    dialog.toast.error(`保存配置失败: ${error}`)
  } finally {
    isSaving.value = false
  }
}

const resetToDefaults = () => {
  proxyConfig.value = {
    start_port: 8080,
    max_port_attempts: 10,
    mitm_enabled: true,
    max_request_body_size: 2 * 1024 * 1024,
    max_response_body_size: 2 * 1024 * 1024,
    upstream_proxy: null,
    exclude_self_traffic: true,
  }
  requestBodySizeMB.value = 2
  responseBodySizeMB.value = 2
  upstreamProxy.value = null
  dialog.toast.info('已重置为默认配置')
}

// Certificate dialog methods
function openCertDialog() {
  certOperation.value = ''
  certDialogRef.value?.showModal()
}

function closeCertDialog() {
  certDialogRef.value?.close()
  certOperation.value = ''
}

async function executeCertOperation() {
  if (!certOperation.value) return
  
  isProcessingCert.value = true
  try {
    switch (certOperation.value) {
      case 'export_der_cert':
        await exportCertInDer()
        break
      case 'export_der_key':
        await exportKeyInDer()
        break
      case 'export_pkcs12':
        await exportPkcs12()
        break
      case 'import_der':
        await importDerCert()
        break
      case 'import_pkcs12':
        await importPkcs12()
        break
    }
    closeCertDialog()
  } catch (error: any) {
    console.error('Certificate operation failed:', error)
    dialog.toast.error(`${error}`)
  } finally {
    isProcessingCert.value = false
  }
}

async function exportCertInDer() {
  const response = await invoke<any>('export_ca_cert', { format: 'der' })
  if (response.success && response.data) {
    dialog.toast.success(`${t('trafficAnalysis.proxyConfiguration.certInDerFormat')}: ${response.data.path}`)
  } else {
    throw new Error(response.error || 'Export failed')
  }
}

async function exportKeyInDer() {
  const response = await invoke<any>('export_ca_key', { format: 'der' })
  if (response.success && response.data) {
    dialog.toast.success(`${t('trafficAnalysis.proxyConfiguration.privateKeyInDerFormat')}: ${response.data.path}`)
  } else {
    throw new Error(response.error || 'Export failed')
  }
}

async function exportPkcs12() {
  const response = await invoke<any>('export_ca_pkcs12', {})
  if (response.success && response.data) {
    dialog.toast.success(`${t('trafficAnalysis.proxyConfiguration.certAndKeyInPkcs12')}: ${response.data.path}`)
  } else {
    throw new Error(response.error || 'Export failed')
  }
}

async function importDerCert() {
  const response = await invoke<any>('import_ca_der', {})
  if (response.success) {
    dialog.toast.success('Certificate imported successfully')
  } else {
    throw new Error(response.error || 'Import failed')
  }
}

async function importPkcs12() {
  const response = await invoke<any>('import_ca_pkcs12', {})
  if (response.success) {
    dialog.toast.success('Certificate imported successfully')
  } else {
    throw new Error(response.error || 'Import failed')
  }
}

// Certificate management methods
async function downloadCACert() {
  isDownloadingCert.value = true
  try {
    const response = await invoke<any>('download_ca_cert')
    if (response.success && response.data) {
      dialog.toast.success(`证书已下载到: ${response.data.path}`)
    } else {
      dialog.toast.error(`下载证书失败: ${response.message || '未知错误'}`)
    }
  } catch (error: any) {
    console.error('Failed to download CA cert:', error)
    dialog.toast.error(`下载证书失败: ${error}`)
  } finally {
    isDownloadingCert.value = false
  }
}

async function regenerateCACert() {
  isRegeneratingCert.value = true
  try {
    await invoke('regenerate_ca_cert')
    dialog.toast.success('证书已重新生成，请重新安装到系统')
  } catch (error: any) {
    console.error('Failed to regenerate CA cert:', error)
    dialog.toast.error(`重新生成证书失败: ${error}`)
  } finally {
    isRegeneratingCert.value = false
  }
}

async function openCertDir() {
  isOpeningCertDir.value = true
  try {
    const response = await invoke<any>('open_ca_cert_dir')
    if (response.success) {
      dialog.toast.success(`已打开证书目录: ${response.data}`)
    } else {
      dialog.toast.error(`打开证书目录失败: ${response.error || '未知错误'}`)
    }
  } catch (error: any) {
    console.error('Failed to open cert directory:', error)
    dialog.toast.error(`打开证书目录失败: ${error}`)
  } finally {
    isOpeningCertDir.value = false
  }
}

// 保存代理自动启动配置
const saveProxyAutoStart = async () => {
  try {
    console.log('[ProxyConfiguration] Saving proxy auto-start:', proxyAutoStart.value)
    const response = await invoke<any>('set_proxy_auto_start', { 
      enabled: proxyAutoStart.value 
    })
    
    if (response.success) {
      console.log('[ProxyConfiguration] Proxy auto-start saved successfully')
      dialog.toast.success(proxyAutoStart.value ? '已启用代理自动启动' : '已禁用代理自动启动')
    } else {
      throw new Error(response.error || '保存失败')
    }
  } catch (error: any) {
    console.error('[ProxyConfiguration] Failed to save proxy auto-start:', error)
    dialog.toast.error(`保存配置失败: ${error}`)
    // 回滚状态
    proxyAutoStart.value = !proxyAutoStart.value
  }
}

// 加载配置的通用函数
const loadConfig = async () => {
  try {
    console.log('[ProxyConfiguration] Loading config...')
    // 加载代理配置
    const configResponse = await invoke<any>('get_proxy_config')
    if (configResponse.success && configResponse.data) {
      proxyConfig.value = configResponse.data
      requestBodySizeMB.value = Math.round(configResponse.data.max_request_body_size / (1024 * 1024))
      responseBodySizeMB.value = Math.round(configResponse.data.max_response_body_size / (1024 * 1024))
      proxyListeners.value[0].interface = `127.0.0.1:${configResponse.data.start_port}`
      
      // Load upstream proxy config
      if (configResponse.data.upstream_proxy) {
        upstreamProxy.value = configResponse.data.upstream_proxy
        console.log('[ProxyConfiguration] Loaded upstream proxy:', upstreamProxy.value)
      } else {
        upstreamProxy.value = null
      }
    }
    
    // 加载代理自动启动配置
    const autoStartResponse = await invoke<any>('get_proxy_auto_start')
    if (autoStartResponse.success) {
      proxyAutoStart.value = autoStartResponse.data || false
      console.log('[ProxyConfiguration] Loaded proxy auto-start:', proxyAutoStart.value)
    }
    
    // 检查代理实际运行状态
    const statusResponse = await invoke<any>('get_proxy_status')
    if (statusResponse.success && statusResponse.data) {
      const isRunning = statusResponse.data.running
      const actualPort = statusResponse.data.port
      
      // 同步运行状态到界面
      if (isRunning && actualPort > 0) {
        // 代理正在运行，更新界面状态
        const listenerIndex = proxyListeners.value.findIndex(
          l => l.interface === `127.0.0.1:${actualPort}`
        )
        if (listenerIndex !== -1) {
          proxyListeners.value[listenerIndex].running = true
        } else {
          // 如果找不到匹配的监听器，更新第一个
          proxyListeners.value[0].interface = `127.0.0.1:${actualPort}`
          proxyListeners.value[0].running = true
        }
        console.log(`[ProxyConfiguration] Proxy is running on port ${actualPort}`)
      } else {
        // 代理未运行，确保所有监听器的运行状态为 false
        proxyListeners.value.forEach(listener => {
          listener.running = false
        })
        console.log('[ProxyConfiguration] Proxy is not running')
      }
    }
    
    // 加载响应拦截状态
    const responseInterceptResponse = await invoke<any>('get_response_intercept_enabled')
    if (responseInterceptResponse.success) {
      interceptResponses.value = responseInterceptResponse.data
      console.log('[ProxyConfiguration] Response intercept:', responseInterceptResponse.data)
    }
    
    // 从 localStorage 加载请求和响应拦截规则
    try {
      const savedRequestRules = localStorage.getItem('proxy_request_filter_rules')
      if (savedRequestRules) {
        const parsed = JSON.parse(savedRequestRules)
        if (Array.isArray(parsed) && parsed.length > 0) {
          requestRules.value = parsed
          console.log('[ProxyConfiguration] Loaded request filter rules from localStorage:', parsed.length)
        }
      }
      
      const savedResponseRules = localStorage.getItem('proxy_response_filter_rules')
      if (savedResponseRules) {
        const parsed = JSON.parse(savedResponseRules)
        if (Array.isArray(parsed) && parsed.length > 0) {
          responseRules.value = parsed
          console.log('[ProxyConfiguration] Loaded response filter rules from localStorage:', parsed.length)
        }
      }
    } catch (e) {
      console.error('[ProxyConfiguration] Failed to load filter rules from localStorage:', e)
    }
  } catch (error) {
    console.error('[ProxyConfiguration] Failed to load config or status:', error)
    // 确保在出错时所有监听器的运行状态为 false
    proxyListeners.value.forEach(listener => {
      listener.running = false
    })
  }
}

// 自动启动代理监听器（已迁移到后端应用启动时自动执行）
// 前端保留此方法作为手动启动的备用选项
const autoStartProxy = async () => {
  // 检查第一个监听器是否已经在运行
  if (proxyListeners.value.length > 0 && !proxyListeners.value[0].running) {
    console.log('[ProxyConfiguration] Manually starting proxy listener...')
    const listener = proxyListeners.value[0]
    try {
      const [host, port] = listener.interface.split(':')
      const response = await invoke<any>('start_proxy_listener', { 
        host,
        port: parseInt(port),
        index: 0
      })
      
      if (response.success) {
        listener.running = true
        console.log(`[ProxyConfiguration] Proxy listener ${listener.interface} manually started`)
      } else {
        console.warn(`[ProxyConfiguration] Failed to start proxy: ${response.error || 'port may be in use'}`)
      }
    } catch (error: any) {
      console.warn('[ProxyConfiguration] Failed to start proxy:', error)
    }
  }
}

// Handle filter rule from ProxyIntercept component
interface FilterRulePayload {
  ruleType: 'request' | 'response'
  rule: InterceptionRule
}

function handleAddFilterRule(payload: FilterRulePayload) {
  console.log('[ProxyConfiguration] Received filter rule:', payload)
  
  const rules = payload.ruleType === 'request' ? requestRules.value : responseRules.value
  
  // Check if a similar rule already exists
  const existingIndex = rules.findIndex(r => 
    r.matchType === payload.rule.matchType && 
    r.condition === payload.rule.condition
  )
  
  if (existingIndex !== -1) {
    // Update existing rule
    rules[existingIndex] = { ...payload.rule }
    console.log('[ProxyConfiguration] Updated existing rule at index:', existingIndex)
  } else {
    // Add new rule
    rules.push({ ...payload.rule })
    console.log('[ProxyConfiguration] Added new rule')
  }
  
  // Trigger save
  debouncedSave()
}

// Store unlisten functions
let unlistenProxyStatus: (() => void) | null = null
let unlistenFilterRule: (() => void) | null = null

// 加载保存的配置
onMounted(async () => {
  await loadConfig()
  
  // 不再在前端自动启动代理，已迁移到后端应用初始化时执行
  // 前端仅加载配置和状态
  console.log('[ProxyConfiguration] Configuration loaded, proxy auto-start is now handled by backend')
  
  // 初始加载完成后，延迟启用自动保存
  setTimeout(() => {
    isInitialLoad.value = false
    console.log('[ProxyConfiguration] Auto-save enabled')
  }, 500)
  
  // 监听代理状态变化事件
  unlistenProxyStatus = await listen('proxy:status', (event: any) => {
    const payload = event.payload
    console.log('Received proxy status event:', payload)
    
    if (payload.running && payload.port > 0) {
      // 代理正在运行
      const listenerIndex = proxyListeners.value.findIndex(
        l => l.interface === `127.0.0.1:${payload.port}`
      )
      if (listenerIndex !== -1) {
        proxyListeners.value[listenerIndex].running = true
      } else {
        proxyListeners.value[0].interface = `127.0.0.1:${payload.port}`
        proxyListeners.value[0].running = true
      }
    } else {
      // 代理已停止
      proxyListeners.value.forEach(listener => {
        listener.running = false
      })
    }
  })
  
  // Listen for filter rules from ProxyIntercept
  unlistenFilterRule = await listen<FilterRulePayload>('intercept:add-filter-rule', (event) => {
    handleAddFilterRule(event.payload)
  })
})

// Cleanup on unmount
onUnmounted(() => {
  if (unlistenProxyStatus) unlistenProxyStatus()
  if (unlistenFilterRule) unlistenFilterRule()
  if (saveTimeout.value) {
    clearTimeout(saveTimeout.value)
  }
})

// 监听父组件的刷新触发器
watch(refreshTrigger, async () => {
  console.log('[ProxyConfiguration] Refresh triggered by parent')
  // 刷新时暂时禁用自动保存
  isInitialLoad.value = true
  await loadConfig()
  // 延迟重新启用自动保存
  setTimeout(() => {
    isInitialLoad.value = false
  }, 500)
})

// 监听 proxyConfig 的变化，自动保存
watch(proxyConfig, () => {
  console.log('[ProxyConfiguration] Config changed, triggering auto-save')
  debouncedSave()
}, { deep: true })

// 监听响应拦截开关变化
watch(interceptResponses, async (newValue) => {
  if (isInitialLoad.value) return
  
  console.log('[ProxyConfiguration] Response intercept changed:', newValue)
  try {
    await invoke('set_response_intercept_enabled', { enabled: newValue })
  } catch (error) {
    console.error('[ProxyConfiguration] Failed to set response intercept:', error)
  }
})

// Sync filter rules to backend
async function syncFilterRulesToBackend() {
  if (isInitialLoad.value) return
  
  try {
    // Sync request rules
    const reqRules = requestRules.value.map(r => ({
      enabled: r.enabled,
      operator: r.operator || '',
      match_type: r.matchType,
      relationship: r.relationship,
      condition: r.condition || ''
    }))
    await invoke('update_runtime_filter_rules', { 
      ruleType: 'request',
      rules: reqRules 
    })
    console.log('[ProxyConfiguration] Request filter rules synced to backend:', reqRules.length)
    
    // Sync response rules
    const respRules = responseRules.value.map(r => ({
      enabled: r.enabled,
      operator: r.operator || '',
      match_type: r.matchType,
      relationship: r.relationship,
      condition: r.condition || ''
    }))
    await invoke('update_runtime_filter_rules', { 
      ruleType: 'response',
      rules: respRules 
    })
    console.log('[ProxyConfiguration] Response filter rules synced to backend:', respRules.length)
  } catch (error) {
    console.error('[ProxyConfiguration] Failed to sync filter rules:', error)
  }
}

// Watch for rule changes and sync to backend
watch(requestRules, () => {
  if (!isInitialLoad.value) {
    console.log('[ProxyConfiguration] Request rules changed, syncing to backend')
    syncFilterRulesToBackend()
  }
}, { deep: true })

watch(responseRules, () => {
  if (!isInitialLoad.value) {
    console.log('[ProxyConfiguration] Response rules changed, syncing to backend')
    syncFilterRulesToBackend()
  }
}, { deep: true })

// Emit declaration
const emit = defineEmits<{
  (e: 'filterRuleAdded', rule: { matchType: string; condition: string; relationship: string }): void
}>()

// Expose method to add filter rule from external components
const addRequestFilterRule = (matchType: string, condition: string, relationship: string = 'matches') => {
  const newRule = {
    enabled: true,
    operator: requestRules.value.length > 0 ? 'And' : '',
    matchType,
    relationship,
    condition
  }
  requestRules.value.push(newRule)
  selectedRequestRuleIndex.value = requestRules.value.length - 1
  debouncedSave()
  
  // Emit event to notify parent component
  emit('filterRuleAdded', { matchType, condition, relationship })
  
  // Show success message
  dialog.toast.success(`Filter rule added: ${matchType} ${relationship} ${condition}`)
}

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
