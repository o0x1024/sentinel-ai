<template>
    <div class="h-full flex flex-col bg-base-100" @contextmenu.prevent>
        <!-- 工具栏 -->
        <div class="flex items-center gap-2 p-3 bg-base-200 border-b border-base-300">
            <!-- 网卡选择 -->
            <select v-model="selectedInterface" class="select select-sm select-bordered min-w-56"
                :disabled="isCapturing">
                <option value="">选择网卡</option>
                <option v-for="iface in interfaces" :key="iface.name" :value="iface.name">
                    {{ getInterfaceDisplayName(iface) }}
                </option>
            </select>

            <!-- 开始/停止按钮 -->
            <button class="btn btn-sm" :class="isCapturing ? 'btn-error' : 'btn-success'"
                @click="toggleCapture" :disabled="!selectedInterface && !isCapturing">
                <i :class="isCapturing ? 'fas fa-stop' : 'fas fa-play'" class="mr-1"></i>
                {{ isCapturing ? '停止' : '开始' }}
            </button>

            <!-- 清空按钮 -->
            <button class="btn btn-sm btn-ghost" @click="clearPackets" :disabled="packets.length === 0">
                <i class="fas fa-trash mr-1"></i>
                清空
            </button>

            <div class="divider divider-horizontal mx-0"></div>

            <!-- 高级过滤按钮 -->
            <button class="btn btn-sm btn-ghost" @click="showFilterDialog = true">
                <i class="fas fa-sliders-h mr-1"></i>
                高级过滤
            </button>

            <!-- 过滤器输入 -->
            <div class="flex-1 flex items-center gap-2">
                <div class="relative flex-1">
                    <input v-model="filterText" type="text" 
                        :placeholder="filterPlaceholder"
                        class="input input-sm input-bordered w-full pr-20" 
                        @keyup.enter="applyFilter" />
                    <div class="absolute right-1 top-1/2 -translate-y-1/2 flex gap-1">
                        <button v-if="filterText || hasAdvancedFilter" class="btn btn-xs btn-ghost btn-circle" @click="clearAllFilters">
                            <i class="fas fa-times"></i>
                        </button>
                        <button class="btn btn-xs btn-primary" @click="applyFilter">
                            <i class="fas fa-filter"></i>
                        </button>
                    </div>
                </div>
            </div>

            <!-- 统计信息 -->
            <div class="flex items-center gap-2">
                <span v-if="hasAdvancedFilter" class="badge badge-info badge-sm">高级过滤</span>
                <span class="badge badge-ghost">{{ filteredPackets.length }} / {{ packets.length }}</span>
            </div>
        </div>

        <!-- 主内容区 -->
        <div class="flex-1 flex flex-col min-h-0">
            <!-- 数据包列表 -->
            <div class="min-h-0 overflow-auto border-b border-base-300" 
                 :style="{ flex: `0 0 ${listHeight}px` }">
                <table class="table table-xs table-pin-rows packet-table">
                    <thead>
                        <tr class="bg-base-200">
                            <th class="w-12"></th>
                            <th class="w-16">No.</th>
                            <th class="w-24">时间</th>
                            <th class="w-40">源地址</th>
                            <th class="w-40">目标地址</th>
                            <th class="w-20">协议</th>
                            <th class="w-16">长度</th>
                            <th>信息</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="packet in filteredPackets" :key="packet.id"
                            class="cursor-pointer packet-row"
                            :class="[
                                getProtocolRowClass(packet.protocol),
                                { 'selected-row': selectedPacket?.id === packet.id },
                                { 'marked-row': markedPackets.has(packet.id) },
                                { 'ignored-row': ignoredPackets.has(packet.id) }
                            ]"
                            @click="selectPacket(packet)"
                            @contextmenu.prevent="showContextMenu($event, packet)">
                            <td class="text-center">
                                <i v-if="markedPackets.has(packet.id)" class="fas fa-bookmark text-warning text-xs"></i>
                            </td>
                            <td class="font-mono text-xs">{{ packet.id }}</td>
                            <td class="font-mono text-xs">{{ formatTime(packet.timestamp) }}</td>
                            <td class="font-mono text-xs">{{ packet.src }}</td>
                            <td class="font-mono text-xs">{{ packet.dst }}</td>
                            <td>
                                <span class="badge badge-sm" :class="getProtocolBadgeClass(packet.protocol)">
                                    {{ packet.protocol }}
                                </span>
                            </td>
                            <td class="font-mono text-xs">{{ packet.length }}</td>
                            <td class="text-xs truncate max-w-md">{{ packet.info }}</td>
                        </tr>
                    </tbody>
                </table>

                <!-- 空状态 -->
                <div v-if="filteredPackets.length === 0" class="flex flex-col items-center justify-center h-full text-base-content/50 py-12">
                    <template v-if="isLoading">
                        <span class="loading loading-spinner loading-lg mb-4"></span>
                        <p>正在获取网卡列表...</p>
                    </template>
                    <template v-else-if="loadError === 'no_interfaces'">
                        <i class="fas fa-exclamation-triangle text-4xl mb-4 text-warning"></i>
                        <p class="mb-2">未检测到可用网卡</p>
                        <p class="text-sm mb-4">Windows 系统需要安装 Npcap 驱动才能进行网络抓包</p>
                        <a href="https://nmap.org/npcap/" target="_blank" class="btn btn-sm btn-primary">
                            <i class="fas fa-download mr-2"></i>下载 Npcap
                        </a>
                    </template>
                    <template v-else-if="loadError">
                        <i class="fas fa-exclamation-circle text-4xl mb-4 text-error"></i>
                        <p class="text-sm text-error">{{ loadError }}</p>
                    </template>
                    <template v-else>
                        <i class="fas fa-broadcast-tower text-4xl mb-4"></i>
                        <p v-if="!isCapturing">选择网卡并点击开始抓包</p>
                        <p v-else>等待数据包...</p>
                    </template>
                </div>
            </div>

            <!-- 拖动条 -->
            <div v-if="selectedPacket" class="resize-handle" @mousedown="startResize">
                <div class="resize-bar"></div>
            </div>

            <!-- 数据包详情 - 树形展示 -->
            <div v-if="selectedPacket" class="flex-1 flex min-h-0 overflow-hidden">
                <!-- 协议详情 -->
                <div class="w-1/2 overflow-auto border-r border-base-300 protocol-tree-panel">
                    <!-- 协议层 -->
                    <div v-for="(layer, idx) in selectedPacket.layers" :key="idx" class="protocol-layer">
                        <div class="layer-header" :class="getLayerBgClass(layer.name)" @click="toggleLayer(`layer-${idx}`)">
                            <i class="fas fa-caret-right layer-toggle" :class="{ 'expanded': expandedLayers[`layer-${idx}`] }"></i>
                            <span class="layer-title">{{ layer.display }}</span>
                        </div>
                        <div v-show="expandedLayers[`layer-${idx}`]" class="layer-content">
                            <template v-for="(field, fidx) in layer.fields" :key="fidx">
                                <!-- 有子字段的字段 -->
                                <div v-if="field.children && field.children.length > 0" class="field-group">
                                    <div class="field-row field-parent" @click="toggleField(`layer-${idx}-field-${fidx}`)">
                                        <i class="fas fa-caret-right field-toggle" :class="{ 'expanded': expandedFields[`layer-${idx}-field-${fidx}`] }"></i>
                                        <span class="field-name">{{ field.name }}:</span>
                                        <span class="field-value">{{ field.value }}</span>
                                    </div>
                                    <div v-show="expandedFields[`layer-${idx}-field-${fidx}`]" class="field-children">
                                        <div v-for="(child, cidx) in field.children" :key="cidx" 
                                             class="field-row field-child"
                                             @contextmenu.prevent="showFieldContextMenu($event, child.name, child.value)">
                                            <span class="field-name">{{ child.name }}:</span>
                                            <span class="field-value">{{ child.value }}</span>
                                        </div>
                                    </div>
                                </div>
                                <!-- 普通字段 -->
                                <div v-else class="field-row"
                                     @contextmenu.prevent="showFieldContextMenu($event, field.name, field.value)">
                                    <span class="field-name">{{ field.name }}:</span>
                                    <span class="field-value" :class="{ 'highlight': isHighlightField(field.name) }">{{ field.value }}</span>
                                </div>
                            </template>
                        </div>
                    </div>
                </div>

                <!-- Hex 视图 -->
                <div class="w-1/2 overflow-auto p-2 bg-base-200/30">
                    <div class="tabs tabs-boxed bg-base-200 mb-2">
                        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'hex' }" @click="hexViewMode = 'hex'">Hex</a>
                        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'ascii' }" @click="hexViewMode = 'ascii'">ASCII</a>
                        <a class="tab tab-sm" :class="{ 'tab-active': hexViewMode === 'raw' }" @click="hexViewMode = 'raw'">Raw</a>
                    </div>
                    <pre class="text-xs font-mono bg-base-100 p-2 rounded-lg overflow-auto">{{ getHexView() }}</pre>
                </div>
            </div>
        </div>

        <!-- 状态栏 -->
        <div class="flex items-center justify-between px-3 py-1 bg-base-200 text-xs text-base-content/70 border-t border-base-300">
            <div class="flex items-center gap-4">
                <span v-if="selectedInterface">
                    <i class="fas fa-ethernet mr-1"></i>
                    {{ selectedInterfaceDisplayName }}
                </span>
                <span v-if="isCapturing" class="text-success">
                    <i class="fas fa-circle animate-pulse mr-1"></i>抓包中
                </span>
            </div>
            <div class="flex items-center gap-4">
                <span v-if="selectedPacket">选中: #{{ selectedPacket.id }}</span>
                <span>已捕获: {{ packets.length }} 包</span>
            </div>
        </div>

        <!-- 右键菜单 -->
        <div v-if="contextMenu.visible" class="context-menu"
             :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }" @click.stop>
            <ul class="menu menu-sm bg-base-100 rounded-lg shadow-xl border border-base-300 p-1 w-44">
                <li><a @click="toggleMark" class="text-xs"><i class="fas fa-bookmark w-3 mr-1"></i>{{ isCurrentPacketMarked ? '取消标记' : '标记' }}</a></li>
                <li><a @click="toggleIgnore" class="text-xs"><i class="fas fa-eye-slash w-3 mr-1"></i>{{ isCurrentPacketIgnored ? '取消忽略' : '忽略' }}</a></li>
                <div class="divider my-0.5"></div>
                <!-- 过滤 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-filter w-3 mr-1"></i>过滤</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="filterByField('src')" class="text-xs">源地址</a></li>
                        <li><a @click="filterByField('dst')" class="text-xs">目标地址</a></li>
                        <li><a @click="filterByField('protocol')" class="text-xs">协议</a></li>
                        <li><a @click="filterByConversation" class="text-xs">会话</a></li>
                    </ul>
                </li>
                <!-- 追踪流 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-stream w-3 mr-1"></i>追踪流</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="followStream('tcp')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowTcp }">TCP 流</a></li>
                        <li><a @click="followStream('udp')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowUdp }">UDP 流</a></li>
                        <li><a @click="followStream('http')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowHttp }">HTTP 流</a></li>
                    </ul>
                </li>
                <div class="divider my-0.5"></div>
                <!-- 复制 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-copy w-3 mr-1"></i>复制</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="copyPacketInfo" class="text-xs">摘要</a></li>
                        <li><a @click="copyPacketHex" class="text-xs">Hex</a></li>
                        <li><a @click="copyField('src')" class="text-xs">源地址</a></li>
                        <li><a @click="copyField('dst')" class="text-xs">目标地址</a></li>
                    </ul>
                </li>
            </ul>
        </div>

        <!-- 字段右键菜单 -->
        <div v-if="fieldContextMenu.visible" class="context-menu"
             :style="{ left: fieldContextMenu.x + 'px', top: fieldContextMenu.y + 'px' }" @click.stop>
            <ul class="menu bg-base-100 rounded-box shadow-xl border border-base-300 p-1 w-64">
                <li><a @click="filterByFieldValue"><i class="fas fa-filter w-4"></i>过滤此值</a></li>
                <li><a @click="copyFieldValue"><i class="fas fa-copy w-4"></i>复制: {{ fieldContextMenu.value }}</a></li>
            </ul>
        </div>

        <!-- 高级过滤对话框 -->
        <div v-if="showFilterDialog" class="modal modal-open">
            <div class="modal-box max-w-2xl">
                <h3 class="font-bold text-lg mb-4">
                    <i class="fas fa-sliders-h mr-2"></i>高级过滤设置
                </h3>
                
                <div class="space-y-4">
                    <!-- 协议过滤 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">协议</span></label>
                        <div class="flex flex-wrap gap-2">
                            <label v-for="proto in ['TCP', 'UDP', 'HTTP', 'DNS', 'ICMP', 'ARP', 'TLS']" :key="proto"
                                   class="label cursor-pointer gap-2 bg-base-200 px-3 py-1 rounded-lg">
                                <input type="checkbox" class="checkbox checkbox-sm checkbox-primary" 
                                       v-model="advancedFilter.protocols" :value="proto" />
                                <span class="label-text">{{ proto }}</span>
                            </label>
                        </div>
                    </div>

                    <!-- IP/端口过滤 -->
                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">源IP</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.srcIp" placeholder="192.168.1.1" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">目标IP</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.dstIp" placeholder="10.0.0.1" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">源端口</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.srcPort" placeholder="80 或 1000-2000" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">目标端口</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.dstPort" placeholder="443 或 8000-9000" />
                        </div>
                    </div>

                    <!-- 内容过滤 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">包含字符串 (ASCII)</span></label>
                        <input type="text" class="input input-sm input-bordered" 
                               v-model="advancedFilter.containsString" placeholder="GET /api, password" />
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">包含十六进制</span></label>
                        <input type="text" class="input input-sm input-bordered" 
                               v-model="advancedFilter.containsHex" placeholder="48 54 54 50 (HTTP)" />
                    </div>

                    <!-- 大小过滤 -->
                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">最小长度 (bytes)</span></label>
                            <input type="number" class="input input-sm input-bordered" 
                                   v-model.number="advancedFilter.minLength" placeholder="0" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">最大长度 (bytes)</span></label>
                            <input type="number" class="input input-sm input-bordered" 
                                   v-model.number="advancedFilter.maxLength" placeholder="65535" />
                        </div>
                    </div>

                    <!-- TCP 标志 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">TCP 标志</span></label>
                        <div class="flex flex-wrap gap-2">
                            <label v-for="flag in ['SYN', 'ACK', 'FIN', 'RST', 'PSH', 'URG']" :key="flag"
                                   class="label cursor-pointer gap-2 bg-base-200 px-3 py-1 rounded-lg">
                                <input type="checkbox" class="checkbox checkbox-sm checkbox-secondary" 
                                       v-model="advancedFilter.tcpFlags" :value="flag" />
                                <span class="label-text">{{ flag }}</span>
                            </label>
                        </div>
                    </div>
                </div>

                <div class="modal-action">
                    <button class="btn btn-ghost" @click="resetAdvancedFilter">重置</button>
                    <button class="btn btn-ghost" @click="showFilterDialog = false">取消</button>
                    <button class="btn btn-primary" @click="applyAdvancedFilter">应用</button>
                </div>
            </div>
            <div class="modal-backdrop" @click="showFilterDialog = false"></div>
        </div>

        <!-- 追踪流对话框 -->
        <div v-if="streamDialog.visible" class="modal modal-open">
            <div class="modal-box max-w-4xl h-[80vh] flex flex-col">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="font-bold text-lg">
                        <i class="fas fa-stream mr-2"></i>{{ streamDialog.title }}
                    </h3>
                    <div class="flex items-center gap-2">
                        <select v-model="streamDialog.displayMode" class="select select-sm select-bordered">
                            <option value="ascii">ASCII</option>
                            <option value="hex">十六进制</option>
                            <option value="raw">原始摘要</option>
                        </select>
                        <button class="btn btn-sm btn-ghost" @click="copyStreamContent">
                            <i class="fas fa-copy"></i>
                        </button>
                    </div>
                </div>
                
                <div class="flex-1 overflow-auto bg-base-200 rounded-lg p-4">
                    <pre class="text-sm font-mono whitespace-pre-wrap">{{ getStreamContent() }}</pre>
                </div>

                <div class="flex items-center justify-between mt-4">
                    <div class="flex items-center gap-4 text-sm">
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 rounded-full bg-error"></span> 客户端 →
                        </span>
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 rounded-full bg-info"></span> ← 服务端
                        </span>
                        <span class="badge badge-ghost">{{ streamDialog.packets.length }} 包</span>
                    </div>
                    <button class="btn" @click="closeStreamDialog">关闭</button>
                </div>
            </div>
            <div class="modal-backdrop" @click="closeStreamDialog"></div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

interface NetworkInterface {
    name: string
    description?: string
    mac?: string
    ipv4?: string
}

interface ProtocolField {
    name: string
    value: string
    children?: ProtocolField[]
}

interface ProtocolLayer {
    name: string
    display: string
    fields: ProtocolField[]
}

interface Packet {
    id: number
    timestamp: number
    src: string
    dst: string
    protocol: string
    length: number
    info: string
    layers: ProtocolLayer[]
    raw: number[]
}

interface AdvancedFilter {
    protocols: string[]
    srcIp: string
    dstIp: string
    srcPort: string
    dstPort: string
    containsString: string
    containsHex: string
    minLength: number | null
    maxLength: number | null
    tcpFlags: string[]
}

// 状态
const interfaces = ref<NetworkInterface[]>([])
const selectedInterface = ref('')
const isCapturing = ref(false)
const packets = ref<Packet[]>([])
const selectedPacket = ref<Packet | null>(null)
const filterText = ref('')
const appliedFilter = ref('')
const loadError = ref<string | null>(null)
const isLoading = ref(false)
const listHeight = ref(300)
const expandedLayers = reactive<Record<string, boolean>>({})
const expandedFields = reactive<Record<string, boolean>>({})
const markedPackets = reactive(new Set<number>())
const ignoredPackets = reactive(new Set<number>())
const hexViewMode = ref<'hex' | 'ascii' | 'raw'>('hex')
const showFilterDialog = ref(false)

// 高级过滤
const advancedFilter = reactive<AdvancedFilter>({
    protocols: [],
    srcIp: '',
    dstIp: '',
    srcPort: '',
    dstPort: '',
    containsString: '',
    containsHex: '',
    minLength: null,
    maxLength: null,
    tcpFlags: []
})
const advancedFilterApplied = ref(false)

// 追踪流对话框
const streamDialog = reactive({
    visible: false,
    type: 'tcp' as 'tcp' | 'udp' | 'http',
    title: '',
    packets: [] as Packet[],
    displayMode: 'ascii' as 'ascii' | 'hex' | 'raw',
    srcEndpoint: ''
})

// 右键菜单
const contextMenu = reactive({ visible: false, x: 0, y: 0, packet: null as Packet | null })
const fieldContextMenu = reactive({ visible: false, x: 0, y: 0, key: '', value: '' })

let unlistenPacket: UnlistenFn | null = null
let packetCounter = 0

// 计算属性
const filterPlaceholder = computed(() => appliedFilter.value ? `过滤: ${appliedFilter.value}` : '输入关键词过滤...')
const hasAdvancedFilter = computed(() => advancedFilterApplied.value)
const selectedInterfaceDisplayName = computed(() => {
    const iface = interfaces.value.find(i => i.name === selectedInterface.value)
    return iface ? (iface.description || iface.name) : ''
})
const isCurrentPacketMarked = computed(() => contextMenu.packet ? markedPackets.has(contextMenu.packet.id) : false)
const isCurrentPacketIgnored = computed(() => contextMenu.packet ? ignoredPackets.has(contextMenu.packet.id) : false)

// 追踪流条件
const canFollowTcp = computed(() => {
    const p = contextMenu.packet
    return p && ['TCP', 'HTTP', 'HTTPS', 'TLS'].includes(p.protocol)
})
const canFollowUdp = computed(() => {
    const p = contextMenu.packet
    return p && ['UDP', 'DNS', 'DHCP', 'NTP', 'QUIC'].includes(p.protocol)
})
const canFollowHttp = computed(() => {
    const p = contextMenu.packet
    return p && ['HTTP', 'HTTPS'].includes(p.protocol)
})

// 过滤后的数据包
const filteredPackets = computed(() => {
    let result = packets.value.filter(p => !ignoredPackets.has(p.id))
    
    // 简单文本过滤
    if (appliedFilter.value) {
        const filter = appliedFilter.value.toLowerCase()
        result = result.filter(p => {
            const text = `${p.src} ${p.dst} ${p.protocol} ${p.info}`.toLowerCase()
            return text.includes(filter)
        })
    }
    
    // 高级过滤
    if (advancedFilterApplied.value) {
        result = result.filter(p => {
            // 协议过滤
            if (advancedFilter.protocols.length > 0 && !advancedFilter.protocols.includes(p.protocol)) {
                return false
            }
            
            // IP过滤
            if (advancedFilter.srcIp && !p.src.startsWith(advancedFilter.srcIp)) return false
            if (advancedFilter.dstIp && !p.dst.startsWith(advancedFilter.dstIp)) return false
            
            // 端口过滤
            if (advancedFilter.srcPort && !matchPort(p.src, advancedFilter.srcPort)) return false
            if (advancedFilter.dstPort && !matchPort(p.dst, advancedFilter.dstPort)) return false
            
            // 长度过滤
            if (advancedFilter.minLength !== null && p.length < advancedFilter.minLength) return false
            if (advancedFilter.maxLength !== null && p.length > advancedFilter.maxLength) return false
            
            // 字符串过滤
            if (advancedFilter.containsString) {
                const ascii = p.raw.map(b => (b >= 32 && b <= 126) ? String.fromCharCode(b) : '').join('')
                if (!ascii.toLowerCase().includes(advancedFilter.containsString.toLowerCase())) return false
            }
            
            // 十六进制过滤
            if (advancedFilter.containsHex) {
                const hexPattern = advancedFilter.containsHex.replace(/\s/g, '').toLowerCase()
                const packetHex = p.raw.map(b => b.toString(16).padStart(2, '0')).join('')
                if (!packetHex.includes(hexPattern)) return false
            }
            
            // TCP标志过滤
            if (advancedFilter.tcpFlags.length > 0) {
                const flagsStr = p.info.toLowerCase()
                if (!advancedFilter.tcpFlags.some(f => flagsStr.includes(f.toLowerCase()))) return false
            }
            
            return true
        })
    }
    
    return result
})

// 端口匹配
function matchPort(addr: string, portFilter: string): boolean {
    const port = parseInt(addr.split(':')[1] || '0')
    if (portFilter.includes('-')) {
        const [min, max] = portFilter.split('-').map(Number)
        return port >= min && port <= max
    }
    return port === parseInt(portFilter)
}

// 拖动调整大小
let isResizing = false, startY = 0, startHeight = 0

function startResize(e: MouseEvent) {
    isResizing = true
    startY = e.clientY
    startHeight = listHeight.value
    document.addEventListener('mousemove', doResize)
    document.addEventListener('mouseup', stopResize)
    document.body.style.cursor = 'ns-resize'
    document.body.style.userSelect = 'none'
}

function doResize(e: MouseEvent) {
    if (!isResizing) return
    listHeight.value = Math.max(100, Math.min(startHeight + e.clientY - startY, window.innerHeight - 250))
}

function stopResize() {
    isResizing = false
    document.removeEventListener('mousemove', doResize)
    document.removeEventListener('mouseup', stopResize)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
}

// 加载网卡
async function loadInterfaces() {
    isLoading.value = true
    loadError.value = null
    try {
        interfaces.value = await invoke<NetworkInterface[]>('get_network_interfaces')
        if (interfaces.value.length === 0) loadError.value = 'no_interfaces'
    } catch (e) {
        loadError.value = String(e)
    } finally {
        isLoading.value = false
    }
}

// 抓包控制
async function toggleCapture() {
    if (isCapturing.value) await stopCapture()
    else await startCapture()
}

async function startCapture() {
    if (!selectedInterface.value) return
    try {
        unlistenPacket = await listen<Packet>('packet-captured', (event) => {
            packetCounter++
            packets.value.push({ ...event.payload, id: packetCounter })
            if (packets.value.length > 10000) packets.value = packets.value.slice(-5000)
        })
        await invoke('start_packet_capture', { interfaceName: selectedInterface.value })
        isCapturing.value = true
    } catch (e) {
        console.error('Failed to start capture:', e)
    }
}

async function stopCapture() {
    try {
        await invoke('stop_packet_capture')
        isCapturing.value = false
        if (unlistenPacket) { unlistenPacket(); unlistenPacket = null }
    } catch (e) {
        console.error('Failed to stop capture:', e)
    }
}

function clearPackets() {
    packets.value = []
    selectedPacket.value = null
    packetCounter = 0
    markedPackets.clear()
    ignoredPackets.clear()
}

function selectPacket(packet: Packet) {
    selectedPacket.value = packet
    // 清空展开状态，默认全部折叠
    Object.keys(expandedLayers).forEach(k => delete expandedLayers[k])
    Object.keys(expandedFields).forEach(k => delete expandedFields[k])
}

function toggleLayer(key: string) {
    expandedLayers[key] = !expandedLayers[key]
}

function toggleField(key: string) {
    expandedFields[key] = !expandedFields[key]
}

// 过滤
function applyFilter() {
    appliedFilter.value = filterText.value
}

function clearAllFilters() {
    filterText.value = ''
    appliedFilter.value = ''
    resetAdvancedFilter()
}

function applyAdvancedFilter() {
    advancedFilterApplied.value = true
    showFilterDialog.value = false
}

function resetAdvancedFilter() {
    advancedFilter.protocols = []
    advancedFilter.srcIp = ''
    advancedFilter.dstIp = ''
    advancedFilter.srcPort = ''
    advancedFilter.dstPort = ''
    advancedFilter.containsString = ''
    advancedFilter.containsHex = ''
    advancedFilter.minLength = null
    advancedFilter.maxLength = null
    advancedFilter.tcpFlags = []
    advancedFilterApplied.value = false
}

// 右键菜单
function showContextMenu(e: MouseEvent, packet: Packet) {
    contextMenu.visible = true
    contextMenu.x = Math.min(e.clientX, window.innerWidth - 250)
    contextMenu.y = Math.min(e.clientY, window.innerHeight - 300)
    contextMenu.packet = packet
    fieldContextMenu.visible = false
}

function showFieldContextMenu(e: MouseEvent, key: string, value: string) {
    fieldContextMenu.visible = true
    fieldContextMenu.x = e.clientX
    fieldContextMenu.y = e.clientY
    fieldContextMenu.key = key
    fieldContextMenu.value = value
    contextMenu.visible = false
}

function hideMenus() {
    contextMenu.visible = false
    fieldContextMenu.visible = false
}

function toggleMark() {
    if (!contextMenu.packet) return
    const id = contextMenu.packet.id
    markedPackets.has(id) ? markedPackets.delete(id) : markedPackets.add(id)
    hideMenus()
}

function toggleIgnore() {
    if (!contextMenu.packet) return
    const id = contextMenu.packet.id
    ignoredPackets.has(id) ? ignoredPackets.delete(id) : ignoredPackets.add(id)
    hideMenus()
}

function filterByField(field: 'src' | 'dst' | 'protocol') {
    if (!contextMenu.packet) return
    filterText.value = contextMenu.packet[field]
    applyFilter()
    hideMenus()
}

function filterByConversation() {
    if (!contextMenu.packet) return
    const srcIp = contextMenu.packet.src.split(':')[0]
    const dstIp = contextMenu.packet.dst.split(':')[0]
    filterText.value = `${srcIp} ${dstIp}`
    applyFilter()
    hideMenus()
}

function filterByFieldValue() {
    filterText.value = fieldContextMenu.value
    applyFilter()
    hideMenus()
}

async function copyFieldValue() {
    await navigator.clipboard.writeText(fieldContextMenu.value)
    hideMenus()
}

async function copyPacketInfo() {
    if (!contextMenu.packet) return
    const p = contextMenu.packet
    await navigator.clipboard.writeText(`#${p.id} | ${p.protocol} | ${p.src} → ${p.dst} | ${p.length} bytes | ${p.info}`)
    hideMenus()
}

async function copyPacketHex() {
    if (!contextMenu.packet) return
    await navigator.clipboard.writeText(formatHex(contextMenu.packet.raw))
    hideMenus()
}

async function copyField(field: 'src' | 'dst') {
    if (!contextMenu.packet) return
    await navigator.clipboard.writeText(contextMenu.packet[field])
    hideMenus()
}

// 追踪流
function followStream(type: string) {
    if (!contextMenu.packet) return
    const p = contextMenu.packet
    const srcIp = p.src.split(':')[0]
    const dstIp = p.dst.split(':')[0]
    const srcPort = p.src.split(':')[1] || ''
    const dstPort = p.dst.split(':')[1] || ''
    
    // 根据类型确定要追踪的协议
    let protocolFilter: string[] = []
    if (type === 'tcp') protocolFilter = ['TCP', 'HTTP', 'HTTPS', 'TLS']
    else if (type === 'udp') protocolFilter = ['UDP', 'DNS', 'DHCP', 'NTP', 'QUIC', 'mDNS', 'LLMNR']
    else if (type === 'http') protocolFilter = ['HTTP', 'HTTPS']
    
    const streamPackets = packets.value.filter(pk => {
        const pSrcIp = pk.src.split(':')[0]
        const pDstIp = pk.dst.split(':')[0]
        const pSrcPort = pk.src.split(':')[1] || ''
        const pDstPort = pk.dst.split(':')[1] || ''
        
        // 检查协议
        if (!protocolFilter.some(proto => pk.protocol.includes(proto) || proto.includes(pk.protocol))) {
            return false
        }
        
        // 双向匹配会话
        const match1 = pSrcIp === srcIp && pDstIp === dstIp && pSrcPort === srcPort && pDstPort === dstPort
        const match2 = pSrcIp === dstIp && pDstIp === srcIp && pSrcPort === dstPort && pDstPort === srcPort
        return match1 || match2
    })
    
    streamDialog.visible = true
    streamDialog.type = type as 'tcp' | 'udp' | 'http'
    streamDialog.title = `${type.toUpperCase()} 流 - ${p.src} ↔ ${p.dst}`
    streamDialog.packets = streamPackets
    streamDialog.displayMode = 'ascii'
    streamDialog.srcEndpoint = p.src
    
    hideMenus()
}

function closeStreamDialog() {
    streamDialog.visible = false
    streamDialog.packets = []
}

function getStreamContent(): string {
    const lines: string[] = []
    for (const p of streamDialog.packets) {
        const isClient = p.src === streamDialog.srcEndpoint
        const prefix = isClient ? `[→ ${p.src}]` : `[← ${p.src}]`
        
        if (streamDialog.displayMode === 'ascii') {
            const text = p.raw.map(b => (b >= 32 && b <= 126) || b === 10 || b === 13 ? String.fromCharCode(b) : '').join('')
            if (text.trim()) lines.push(`${prefix}\n${text}`)
        } else if (streamDialog.displayMode === 'hex') {
            lines.push(`${prefix}\n${formatHex(p.raw)}`)
        } else {
            lines.push(`#${p.id} ${p.src} → ${p.dst} [${p.protocol}] Len=${p.length}`)
        }
    }
    return lines.join('\n\n') || '无数据'
}

async function copyStreamContent() {
    await navigator.clipboard.writeText(getStreamContent())
}

// 格式化
function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString('zh-CN', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 } as Intl.DateTimeFormatOptions)
}

function formatFullTime(ts: number): string {
    return new Date(ts).toLocaleString('zh-CN', { hour12: false })
}

function formatHex(raw: number[]): string {
    if (!raw?.length) return ''
    const lines: string[] = []
    for (let i = 0; i < raw.length; i += 16) {
        const bytes = raw.slice(i, i + 16)
        const hex = bytes.map(b => b.toString(16).padStart(2, '0')).join(' ')
        const ascii = bytes.map(b => (b >= 32 && b <= 126) ? String.fromCharCode(b) : '.').join('')
        lines.push(`${i.toString(16).padStart(8, '0')}  ${hex.padEnd(48)}  ${ascii}`)
    }
    return lines.join('\n')
}

function getHexView(): string {
    if (!selectedPacket.value) return ''
    const raw = selectedPacket.value.raw
    if (hexViewMode.value === 'hex') return formatHex(raw)
    if (hexViewMode.value === 'ascii') return raw.map(b => (b >= 32 && b <= 126) || b === 10 || b === 13 ? String.fromCharCode(b) : '.').join('')
    return raw.map(b => b.toString(16).padStart(2, '0')).join(' ')
}

function getInterfaceDisplayName(iface: NetworkInterface): string {
    return iface.ipv4 ? `${iface.description || iface.name} (${iface.ipv4})` : (iface.description || iface.name)
}

// 样式
function getProtocolRowClass(proto: string): string {
    const map: Record<string, string> = {
        'TCP': 'row-tcp', 'UDP': 'row-udp', 'HTTP': 'row-http', 'HTTPS': 'row-https',
        'TLS': 'row-https', 'DNS': 'row-dns', 'ICMP': 'row-icmp', 'ARP': 'row-arp'
    }
    return map[proto] || 'row-other'
}

function getProtocolBadgeClass(proto: string): string {
    const map: Record<string, string> = {
        'TCP': 'badge-secondary', 'UDP': 'badge-info', 'HTTP': 'badge-success', 
        'HTTPS': 'badge-warning', 'TLS': 'badge-warning', 'DNS': 'badge-accent', 
        'ICMP': 'badge-error', 'ARP': 'badge-neutral'
    }
    return map[proto] || 'badge-ghost'
}

function getLayerBgClass(name: string): string {
    const map: Record<string, string> = {
        'Frame': 'layer-frame', 'Ethernet': 'layer-eth', 
        'IPv4': 'layer-ip', 'IPv6': 'layer-ip',
        'TCP': 'layer-tcp', 'UDP': 'layer-udp', 
        'HTTP': 'layer-http', 'DNS': 'layer-dns', 
        'ICMP': 'layer-icmp', 'ARP': 'layer-arp'
    }
    return map[name] || ''
}

function isHighlightField(key: string): boolean {
    return ['Source', 'Destination', 'Source Port', 'Destination Port', 'Flags', 'Query Name', 'Resolved IPs', 'URI', 'Status Code'].includes(key)
}

// 键盘快捷键
function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'm' && selectedPacket.value) {
        contextMenu.packet = selectedPacket.value
        toggleMark()
    }
    if (e.key === 'Escape') hideMenus()
}

onMounted(() => {
    loadInterfaces()
    document.addEventListener('click', hideMenus)
    document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
    if (isCapturing.value) stopCapture()
    document.removeEventListener('click', hideMenus)
    document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.packet-table th { @apply sticky top-0 z-10 bg-base-200; }
.packet-row { transition: background-color 0.1s; }
.row-tcp { background-color: rgba(168, 162, 217, 0.15); }
.row-udp { background-color: rgba(125, 211, 252, 0.15); }
.row-http { background-color: rgba(134, 239, 172, 0.2); }
.row-https { background-color: rgba(253, 224, 71, 0.15); }
.row-dns { background-color: rgba(125, 211, 252, 0.2); }
.row-icmp { background-color: rgba(251, 146, 150, 0.15); }
.row-arp { background-color: rgba(253, 186, 116, 0.2); }
.row-other { background-color: transparent; }
.packet-row:hover { filter: brightness(0.95); }
.selected-row { @apply bg-primary/20 outline outline-1 outline-primary/50; }
.marked-row { @apply border-l-4 border-warning; }
.ignored-row { @apply opacity-40; }

.resize-handle { @apply flex items-center justify-center cursor-ns-resize bg-base-200 hover:bg-base-300 h-1.5; }
.resize-bar { @apply bg-base-content/20 rounded-full w-16 h-1; }
.resize-handle:hover .resize-bar { @apply bg-base-content/40; }

.context-menu { @apply fixed z-50; }

/* 协议树形面板 - Wireshark风格 */
.protocol-tree-panel {
    @apply text-xs font-mono;
    background: var(--fallback-b1, oklch(var(--b1)));
}

.protocol-layer {
    border-bottom: 1px solid oklch(var(--bc) / 0.1);
}

.layer-header {
    @apply flex items-center gap-1 px-1 py-0.5 cursor-pointer select-none;
    min-height: 20px;
}
.layer-header:hover { filter: brightness(0.95); }

.layer-toggle {
    @apply w-3 text-base-content/50 transition-transform duration-100;
    font-size: 10px;
}
.layer-toggle.expanded { transform: rotate(90deg); }

.layer-title {
    @apply flex-1 truncate;
}

.layer-content {
    @apply pl-3;
}

.field-row {
    @apply flex gap-1 px-1 py-px hover:bg-base-200/50;
    min-height: 18px;
    line-height: 18px;
}

.field-parent {
    @apply cursor-pointer;
}

.field-toggle {
    @apply w-3 text-base-content/50 transition-transform duration-100;
    font-size: 10px;
}
.field-toggle.expanded { transform: rotate(90deg); }

.field-children {
    @apply pl-4;
}

.field-child {
    @apply text-base-content/80;
}

.field-name {
    @apply text-base-content/60 whitespace-nowrap;
}

.field-value {
    @apply text-base-content flex-1;
}
.field-value.highlight {
    @apply text-primary font-semibold;
}

/* 协议层颜色 */
.layer-frame { background-color: #f5f5f5; }
.layer-eth { background-color: #e8f4e8; }
.layer-ip { background-color: #e8f0f8; }
.layer-tcp { background-color: #f0e8f8; }
.layer-udp { background-color: #e8f8f8; }
.layer-http { background-color: #f0f8e8; }
.layer-dns { background-color: #f8f0e8; }
.layer-icmp { background-color: #f8e8e8; }
.layer-arp { background-color: #f8f8e8; }

:global(.dark) .layer-frame { background-color: #2a2a2a; }
:global(.dark) .layer-eth { background-color: #1a2a1a; }
:global(.dark) .layer-ip { background-color: #1a1a2a; }
:global(.dark) .layer-tcp { background-color: #2a1a2a; }
:global(.dark) .layer-udp { background-color: #1a2a2a; }
:global(.dark) .layer-http { background-color: #2a2a1a; }
:global(.dark) .layer-dns { background-color: #2a1a1a; }
:global(.dark) .layer-icmp { background-color: #2a1a1a; }
:global(.dark) .layer-arp { background-color: #2a2a1a; }

/* 右键子菜单 - 右侧弹出 */
.submenu-parent { @apply relative; }
.submenu-parent > a { @apply flex; }
.submenu { 
    @apply absolute invisible opacity-0 menu menu-sm bg-base-100 rounded-lg shadow-xl border border-base-300 p-1 w-32;
    left: 100%;
    top: 0;
    margin-left: 2px;
    transition: opacity 0.1s, visibility 0.1s;
}
.submenu-parent:hover > .submenu { @apply visible opacity-100; }

:global(.dark) .row-tcp { background-color: rgba(100, 100, 160, 0.25); }
:global(.dark) .row-udp { background-color: rgba(80, 120, 160, 0.25); }
:global(.dark) .row-http { background-color: rgba(80, 160, 80, 0.25); }
:global(.dark) .row-https { background-color: rgba(160, 160, 80, 0.25); }
:global(.dark) .row-dns { background-color: rgba(80, 120, 160, 0.3); }
:global(.dark) .row-icmp { background-color: rgba(160, 100, 100, 0.25); }
:global(.dark) .row-arp { background-color: rgba(160, 140, 80, 0.25); }
</style>
