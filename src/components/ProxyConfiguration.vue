<template>
  <div class="space-y-4">
    <!-- Proxy Listeners Section -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-network-wired mr-2"></i>
          代理监听器
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          配置代理监听器以接收来自浏览器的 HTTP 请求。需要配置浏览器使用其中一个监听器作为代理服务器。
        </p>

        <div class="overflow-x-auto">
          <table class="table table-sm w-full">
            <thead>
              <tr>
                <th class="w-12">
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    @change="toggleAllListeners"
                    :checked="selectedListeners.length === proxyListeners.length && proxyListeners.length > 0"
                  />
                </th>
                <th class="w-20">运行中</th>
                <th>接口</th>
                <th>不可见</th>
                <th>重定向</th>
                <th>证书</th>
                <th>TLS协议</th>
                <th>支持 HTTP/2</th>
              </tr>
            </thead>
            <tbody>
              <tr 
                v-for="(listener, index) in proxyListeners" 
                :key="index"
                :class="{ 'bg-base-200': selectedListeners.includes(index) }"
                @dblclick="editListenerByIndex(index)"
                class="cursor-pointer hover:bg-base-300 transition-colors"
              >
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    :checked="selectedListeners.includes(index)"
                    @change="toggleListenerSelection(index)"
                  />
                </td>
                <td>
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

        <div class="flex gap-2 mt-4">
          <button class="btn btn-sm btn-primary" @click="addListener">
            <i class="fas fa-plus mr-1"></i>
            添加
          </button>
          <button 
            class="btn btn-sm btn-outline" 
            @click="editListener"
            :disabled="selectedListeners.length !== 1"
          >
            <i class="fas fa-edit mr-1"></i>
            编辑
          </button>
          <button 
            class="btn btn-sm btn-outline btn-error" 
            @click="removeListener"
            :disabled="selectedListeners.length === 0"
          >
            <i class="fas fa-trash mr-1"></i>
            移除
          </button>
        </div>

        <div class="mt-4 space-y-2">
          <div class="flex gap-2">
            <button 
              class="btn btn-sm btn-outline"
              @click="downloadCACert"
              :disabled="isDownloadingCert"
            >
              <i :class="['fas fa-download mr-1', { 'fa-spin': isDownloadingCert }]"></i>
              导出 CA 证书
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="regenerateCACert"
              :disabled="isRegeneratingCert"
            >
              <i :class="['fas fa-sync-alt mr-1', { 'fa-spin': isRegeneratingCert }]"></i>
              重新生成 CA 证书
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="openCertDir"
              :disabled="isOpeningCertDir"
            >
              <i :class="['fas fa-folder-open mr-1', { 'fa-spin': isOpeningCertDir }]"></i>
              打开证书目录
            </button>
          </div>
          <p class="text-xs text-base-content/60">
            每个安装都会生成自己的 CA 证书，代理监听器在协商 TLS 连接时可以使用该证书。导出证书后，请手动将其安装到系统信任列表中。
          </p>
        </div>
      </div>
    </div>

    <!-- 编辑监听器对话框 -->
    <dialog ref="editDialogRef" class="modal">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">编辑监听器</h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">绑定地址</span>
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
              <span class="label-text">端口</span>
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
              <span class="label-text">证书模式</span>
            </label>
            <select v-model="editingListener.certificate" class="select select-bordered">
              <option value="Per-host">Per-host（每个主机独立证书）</option>
              <option value="Wildcard">Wildcard（通配符证书）</option>
              <option value="Custom">Custom（自定义证书）</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">TLS 协议</span>
            </label>
            <select v-model="editingListener.tlsProtocols" class="select select-bordered">
              <option value="Default">Default（默认）</option>
              <option value="TLS 1.2">TLS 1.2</option>
              <option value="TLS 1.3">TLS 1.3</option>
              <option value="TLS 1.2+1.3">TLS 1.2+1.3</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">支持 HTTP/2</span>
              <input 
                type="checkbox" 
                v-model="editingListener.supportHTTP2"
                class="checkbox"
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">不可见模式</span>
              <input 
                type="checkbox" 
                v-model="editingListener.invisible"
                class="checkbox"
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用重定向</span>
              <input 
                type="checkbox" 
                v-model="editingListener.redirect"
                class="checkbox"
              />
            </label>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" @click="cancelEdit">取消</button>
          <button class="btn btn-primary" @click="saveEdit">保存</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>

    <!-- Request Interception Rules -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-filter mr-2"></i>
          请求拦截规则
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来控制在拦截选项卡中暂停哪些请求以供查看和编辑。
        </p>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-sm"
              v-model="interceptRequests"
            />
            <span class="label-text">根据以下规则拦截请求：</span>
            <span v-if="!masterInterceptionEnabled" class="text-warning text-sm">主拦截已关闭</span>
          </label>
        </div>

        <div class="overflow-x-auto mt-2">
          <table class="table table-sm w-full">
            <thead>
              <tr>
                <th class="w-20">启用</th>
                <th>操作符</th>
                <th>匹配类型</th>
                <th>关系</th>
                <th>条件</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(rule, index) in requestRules" :key="index">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="rule.enabled"
                  />
                </td>
                <td>{{ rule.operator }}</td>
                <td>{{ rule.matchType }}</td>
                <td>{{ rule.relationship }}</td>
                <td class="font-mono text-xs">{{ rule.condition }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="flex gap-2 mt-4">
          <button class="btn btn-sm btn-primary">添加</button>
          <button class="btn btn-sm btn-outline">编辑</button>
          <button class="btn btn-sm btn-outline btn-error">移除</button>
          <button class="btn btn-sm btn-outline">上移</button>
          <button class="btn btn-sm btn-outline">下移</button>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoFixNewlines" />
            <span class="label-text">自动修复请求末尾缺失或多余的换行符</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoUpdateContentLength" />
            <span class="label-text">编辑请求时自动更新 Content-Length 标头</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Response Interception Rules -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-reply mr-2"></i>
          响应拦截规则
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来控制在拦截选项卡中暂停哪些响应以供查看和编辑。
        </p>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input 
              type="checkbox" 
              class="checkbox checkbox-sm"
              v-model="interceptResponses"
            />
            <span class="label-text">根据以下规则拦截响应：</span>
            <span v-if="!masterInterceptionEnabled" class="text-warning text-sm">主拦截已关闭</span>
          </label>
        </div>

        <div class="overflow-x-auto mt-2">
          <table class="table table-sm w-full">
            <thead>
              <tr>
                <th class="w-20">启用</th>
                <th>操作符</th>
                <th>匹配类型</th>
                <th>关系</th>
                <th>条件</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(rule, index) in responseRules" :key="index">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="rule.enabled"
                  />
                </td>
                <td>{{ rule.operator }}</td>
                <td>{{ rule.matchType }}</td>
                <td>{{ rule.relationship }}</td>
                <td class="font-mono text-xs">{{ rule.condition }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoUpdateResponseContentLength" />
            <span class="label-text">编辑响应时自动更新 Content-Length 标头</span>
          </label>
        </div>
      </div>
    </div>

    <!-- WebSocket Interception -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-exchange-alt mr-2"></i>
          WebSocket 拦截规则
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来控制在拦截选项卡中暂停哪些 WebSocket 消息以供查看和编辑。
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="interceptClientToServer" />
              <span class="label-text">拦截客户端到服务器的消息</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="interceptServerToClient" />
              <span class="label-text">拦截服务器到客户端的消息</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="onlyInterceptInScope" />
              <span class="label-text">仅拦截范围内的消息</span>
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
          响应修改规则
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来控制 Burp 自动修改响应的方式。
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unhideHiddenFields" />
              <span class="label-text">取消隐藏表单字段</span>
            </label>
          </div>

          <div class="form-control ml-6">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="prominentlyHighlightUnhidden" disabled />
              <span class="label-text text-base-content/50">突出显示取消隐藏的字段</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="enableDisabledFields" />
              <span class="label-text">启用禁用的表单字段</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeInputFieldLengthLimits" />
              <span class="label-text">删除输入字段长度限制</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeJavaScriptFormValidation" />
              <span class="label-text">删除 JavaScript 表单验证</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeAllJavaScript" />
              <span class="label-text">删除所有 JavaScript</span>
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
          匹配和替换规则
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来自动替换通过代理的请求和响应的部分内容。
        </p>

        <div class="form-control mb-3">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="onlyApplyToInScope" />
            <span class="label-text">仅应用于范围内的项目</span>
          </label>
        </div>

        <div class="overflow-x-auto">
          <table class="table table-sm w-full">
            <thead>
              <tr>
                <th class="w-20">启用</th>
                <th>项目</th>
                <th>匹配</th>
                <th>替换</th>
                <th>类型</th>
                <th>备注</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(rule, index) in matchReplaceRules" :key="index">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="rule.enabled"
                  />
                </td>
                <td class="text-xs">{{ rule.item }}</td>
                <td class="font-mono text-xs">{{ rule.match }}</td>
                <td class="text-xs">{{ rule.replace }}</td>
                <td class="text-xs">{{ rule.type }}</td>
                <td class="text-xs">{{ rule.comment }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="flex gap-2 mt-4">
          <button class="btn btn-sm btn-primary">添加</button>
          <button class="btn btn-sm btn-outline">编辑</button>
          <button class="btn btn-sm btn-outline btn-error">移除</button>
          <button class="btn btn-sm btn-outline">上移</button>
          <button class="btn btn-sm btn-outline">下移</button>
        </div>
      </div>
    </div>

    <!-- TLS Pass Through -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-lock mr-2"></i>
          TLS 直通
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来指定目标 Web 服务器，Burp 将直接通过 TLS 连接。通过这些连接将无法在代理拦截视图或历史记录中查看有关请求或响应的详细信息。
        </p>

        <div class="overflow-x-auto">
          <table class="table table-sm w-full">
            <thead>
              <tr>
                <th class="w-20">启用</th>
                <th>主机 / IP 范围</th>
                <th>端口</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="tlsPassThroughRules.length === 0">
                <td colspan="3" class="text-center text-base-content/50">暂无规则</td>
              </tr>
              <tr v-for="(rule, index) in tlsPassThroughRules" :key="index">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="rule.enabled"
                  />
                </td>
                <td>{{ rule.host }}</td>
                <td>{{ rule.port }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="flex gap-2 mt-4">
          <button class="btn btn-sm btn-primary">添加</button>
          <button class="btn btn-sm btn-outline">编辑</button>
          <button class="btn btn-sm btn-outline btn-error">移除</button>
          <button class="btn btn-sm btn-outline">粘贴 URL</button>
          <button class="btn btn-sm btn-outline">加载...</button>
        </div>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="autoAddTLSOnFailure" />
            <span class="label-text">在客户端 TLS 协商失败时自动添加条目</span>
          </label>
        </div>

        <div class="form-control">
          <label class="label cursor-pointer justify-start gap-2">
            <input type="checkbox" class="checkbox checkbox-sm" v-model="applyToOutOfScope" disabled />
            <span class="label-text text-base-content/50">应用于范围外的项目</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Proxy History Logging -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-base mb-3">
          <i class="fas fa-history mr-2"></i>
          代理历史记录日志
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用此设置来选择当您将项目添加到目标范围时，代理是否自动停止将范围外的项目发送到历史记录和其他工具。
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
              <span class="label-text">停止记录范围外的项目</span>
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
              <span class="label-text">每次询问我该怎么做</span>
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
              <span class="label-text">不执行任何操作</span>
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
          默认代理拦截状态
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用此设置来选择启动 Burp 时是否启用代理拦截。
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
              <span class="label-text">启用拦截</span>
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
              <span class="label-text">禁用拦截</span>
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
              <span class="label-text">恢复关闭 Burp 时在 <strong>代理 > 拦截</strong> 选项卡中选择的设置</span>
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
          其他设置
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          使用这些设置来更改 Burp 代理的默认行为。
        </p>

        <div class="space-y-2">
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="useHTTP1_1ToServer" />
              <span class="label-text">对服务器的请求使用 HTTP/1.0</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="useHTTP1_1ToClient" />
              <span class="label-text">对客户端的响应使用 HTTP/1.0</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="setConnectionClose" />
              <span class="label-text">设置响应头 "Connection: close"</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="setConnectionHeader" />
              <span class="label-text">使用 HTTP/1 时在传入请求上设置 "Connection" 标头</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="stripProxyHeaders" />
              <span class="label-text">剥离传入请求中的 Proxy-* 标头</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="removeUnsupportedEncodings" />
              <span class="label-text">从传入请求的 Accept-Encoding 标头中删除不支持的编码</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="stripWebSocketExtensions" />
              <span class="label-text">剥离传入请求中的 Sec-WebSocket-Extensions 标头</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unpackCompressedRequests" />
              <span class="label-text">解压缩请求</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="unpackCompressedResponses" />
              <span class="label-text">解压缩响应</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="disableWebInterface" />
              <span class="label-text">禁用 http://burpsuite 的 Web 界面</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="suppressBurpErrorMessages" />
              <span class="label-text">在浏览器中抑制 Burp 错误消息</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="dontSendToProxyHistory" />
              <span class="label-text">不将项目发送到代理历史记录或实时任务</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-2">
              <input type="checkbox" class="checkbox checkbox-sm" v-model="dontSendToProxyHistoryIfOutOfScope" />
              <span class="label-text">如果超出范围，不将项目发送到代理历史记录或实时任务</span>
            </label>
          </div>
        </div>
      </div>
    </div>


    <!-- Reset Button -->
    <div class="flex justify-end gap-2">
      <button class="btn btn-outline" @click="resetToDefaults">
        <i class="fas fa-undo mr-2"></i>
        重置为默认
      </button>
      <div v-if="isSaving" class="flex items-center gap-2 text-sm text-base-content/70">
        <i class="fas fa-spinner fa-spin"></i>
        <span>正在保存...</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, inject, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { dialog } from '@/composables/useDialog'

// 注入父组件的刷新触发器
const refreshTrigger = inject<any>('refreshTrigger', ref(0))

// 保存状态
const isSaving = ref(false)
const saveTimeout = ref<ReturnType<typeof setTimeout> | null>(null)
const isInitialLoad = ref(true) // 标记是否是初始加载，避免初始加载时触发保存

// Proxy configuration
const proxyConfig = ref({
  start_port: 8080,
  max_port_attempts: 10,
  mitm_enabled: true,
  max_request_body_size: 2 * 1024 * 1024,
  max_response_body_size: 2 * 1024 * 1024,
})

// 辅助变量：请求/响应体大小（MB）
const requestBodySizeMB = ref(2)
const responseBodySizeMB = ref(2)

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
    matchType: '文件扩展名',
    relationship: '不匹配',
    condition: '(^gif$|^jpg$|^png$|^css$|^js$|^ico$|^woff$)'
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: '请求',
    relationship: '包含参数',
    condition: ''
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: 'HTTP 方法',
    relationship: '不匹配',
    condition: '(get|post)'
  },
  {
    enabled: false,
    operator: 'And',
    matchType: 'URL',
    relationship: '在目标范围内',
    condition: ''
  }
])

// Response interception rules
const responseRules = ref([
  {
    enabled: true,
    operator: '',
    matchType: 'Content-type 标头',
    relationship: '匹配',
    condition: 'text'
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: '请求',
    relationship: '已修改',
    condition: ''
  },
  {
    enabled: false,
    operator: 'Or',
    matchType: '请求',
    relationship: '已被拦截',
    condition: ''
  },
  {
    enabled: false,
    operator: 'And',
    matchType: '状态码',
    relationship: '不匹配',
    condition: '^304$'
  },
  {
    enabled: false,
    operator: 'And',
    matchType: 'URL',
    relationship: '在目标范围内',
    condition: ''
  }
])

// Request settings
const autoFixNewlines = ref(false)
const autoUpdateContentLength = ref(true)

// Response settings
const autoUpdateResponseContentLength = ref(true)

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

// Methods
const toggleAllListeners = (event: Event) => {
  const checked = (event.target as HTMLInputElement).checked
  if (checked) {
    selectedListeners.value = proxyListeners.value.map((_, index) => index)
  } else {
    selectedListeners.value = []
  }
}

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
      await invoke('start_proxy_listener', { 
        host,
        port: parseInt(port),
        index
      })
      // 注意：被动扫描代理与全局出站代理是独立配置，不互相影响
      dialog.toast.success(`代理监听器 ${listener.interface} 已启动`)
    } else {
      // 停止代理监听器
      await invoke('stop_proxy_listener', { index })
      dialog.toast.success(`代理监听器 ${listener.interface} 已停止`)
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

const removeListener = async () => {
  if (selectedListeners.value.length === 0) {
    dialog.toast.warning('请至少选择一个监听器')
    return
  }
  
  if (!confirm(`确定要删除选中的 ${selectedListeners.value.length} 个监听器吗？`)) {
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
  }
  requestBodySizeMB.value = 2
  responseBodySizeMB.value = 2
  dialog.toast.info('已重置为默认配置')
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
  if (!confirm('重新生成证书将使旧证书失效，需要重新安装。确定继续吗？')) {
    return
  }
  
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
  } catch (error) {
    console.error('[ProxyConfiguration] Failed to load config or status:', error)
    // 确保在出错时所有监听器的运行状态为 false
    proxyListeners.value.forEach(listener => {
      listener.running = false
    })
  }
}

// 加载保存的配置
onMounted(async () => {
  await loadConfig()
  
  // 初始加载完成后，延迟启用自动保存
  setTimeout(() => {
    isInitialLoad.value = false
    console.log('[ProxyConfiguration] Auto-save enabled')
  }, 500)
  
  // 监听代理状态变化事件
  const unlisten = await listen('proxy:status', (event: any) => {
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
  
  // 保存取消监听函数，用于组件卸载时清理
  onUnmounted(() => {
    unlisten()
    // 清理定时器
    if (saveTimeout.value) {
      clearTimeout(saveTimeout.value)
    }
  })
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
