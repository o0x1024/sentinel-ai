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
              <tr v-for="(listener, index) in proxyListeners" :key="index">
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="listener.running"
                  />
                </td>
                <td>{{ listener.interface }}</td>
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="listener.invisible"
                  />
                </td>
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="listener.redirect"
                  />
                </td>
                <td>{{ listener.certificate }}</td>
                <td>{{ listener.tlsProtocols }}</td>
                <td>
                  <input 
                    type="checkbox" 
                    class="checkbox checkbox-sm"
                    v-model="listener.supportHTTP2"
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
          <button class="btn btn-sm btn-outline" @click="editListener">
            <i class="fas fa-edit mr-1"></i>
            编辑
          </button>
          <button class="btn btn-sm btn-outline btn-error" @click="removeListener">
            <i class="fas fa-trash mr-1"></i>
            移除
          </button>
        </div>

        <div class="mt-4 space-y-2">
          <div class="flex gap-2">
            <button class="btn btn-sm btn-outline">
              <i class="fas fa-file-import mr-1"></i>
              导入/导出 CA 证书
            </button>
            <button class="btn btn-sm btn-outline">
              <i class="fas fa-sync-alt mr-1"></i>
              重新生成 CA 证书
            </button>
          </div>
          <p class="text-xs text-base-content/60">
            每个安装都会生成自己的 CA 证书，代理监听器在协商 TLS 连接时可以使用该证书。您可以导入或导出此证书以在其他工具或另一个安装中使用。
          </p>
        </div>
      </div>
    </div>

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

    <!-- Save Button -->
    <div class="flex justify-end gap-2">
      <button class="btn btn-outline" @click="resetToDefaults">
        <i class="fas fa-undo mr-2"></i>
        重置为默认
      </button>
      <button class="btn btn-primary" @click="saveConfiguration">
        <i class="fas fa-save mr-2"></i>
        保存配置
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

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

// Methods
const addListener = () => {
  console.log('Add listener')
  // TODO: Open dialog to add new listener
}

const editListener = () => {
  console.log('Edit listener')
  // TODO: Open dialog to edit selected listener
}

const removeListener = () => {
  console.log('Remove listener')
  // TODO: Remove selected listener
}

const saveConfiguration = () => {
  console.log('Save configuration')
  // TODO: Save configuration to backend
}

const resetToDefaults = () => {
  console.log('Reset to defaults')
  // TODO: Reset all settings to default values
}
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
