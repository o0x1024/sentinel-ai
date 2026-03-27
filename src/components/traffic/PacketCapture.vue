<template>
    <div class="h-full flex flex-col bg-base-100" @contextmenu.prevent>
        <PacketCaptureToolbar
            :selected-interface="selectedInterface"
            :interfaces="interfaces"
            :is-capturing="isCapturing"
            :packet-count="packets.length"
            :filtered-packet-count="filteredPackets.length"
            :filter-text="filterText"
            :filter-placeholder="filterPlaceholder"
            :has-advanced-filter="hasAdvancedFilter"
            :get-interface-display-name="getInterfaceDisplayName"
            :on-toggle-capture="toggleCapture"
            :on-clear-packets="clearPackets"
            :on-open-pcap-file="openPcapFile"
            :on-save-pcap-file="savePcapFile"
            :on-open-extract-dialog="() => { showExtractDialog = true }"
            :on-open-filter-dialog="() => { showFilterDialog = true }"
            :on-apply-filter="applyFilter"
            :on-clear-all-filters="clearAllFilters"
            @update:selected-interface="selectedInterface = $event"
            @update:filter-text="filterText = $event"
        />

        <div class="flex-1 flex flex-col min-h-0">
            <PacketCapturePacketList
                ref="scrollContainer"
                :filtered-packets="filteredPackets"
                :visible-items="visibleItems"
                :total-height="totalHeight"
                :header-height="headerHeight"
                :row-height="rowHeight"
                :list-height="listHeight"
                :column-widths="columnWidths"
                :selected-packet="selectedPacket"
                :marked-packets="markedPackets"
                :ignored-packets="ignoredPackets"
                :is-loading="isLoading"
                :load-error="loadError"
                :is-capturing="isCapturing"
                :format-time="formatTime"
                :get-protocol-row-class="getProtocolRowClass"
                :get-protocol-badge-class="getProtocolBadgeClass"
                :on-handle-scroll="handleScroll"
                :on-start-column-resize="startColumnResize"
                :on-select-packet="selectPacket"
                :on-show-context-menu="showContextMenu"
            />

            <div v-if="selectedPacket" class="resize-handle" @mousedown="startResize">
                <div class="resize-bar"></div>
            </div>

            <PacketCaptureDetails
                :selected-packet="selectedPacket"
                :expanded-layers="expandedLayers"
                :expanded-fields="expandedFields"
                :hex-view-mode="hexViewMode"
                :get-layer-bg-class="getLayerBgClass"
                :is-highlight-field="isHighlightField"
                :get-hex-view="getHexView"
                :on-toggle-layer="toggleLayer"
                :on-toggle-field="toggleField"
                :on-show-field-context-menu="showFieldContextMenu"
                @update:hex-view-mode="hexViewMode = $event"
            />
        </div>

        <div class="flex items-center justify-between px-3 py-1 bg-base-200 text-xs text-base-content/70 border-t border-base-300">
            <div class="flex items-center gap-4">
                <span v-if="selectedInterface">
                    <i class="fas fa-ethernet mr-1"></i>
                    {{ selectedInterfaceDisplayName }}
                </span>
                <span v-if="isCapturing" class="text-success">
                    <i class="fas fa-circle animate-pulse mr-1"></i>{{ $t('trafficAnalysis.packetCapture.statusBar.capturing') }}
                </span>
            </div>
            <div class="flex items-center gap-4">
                <span v-if="selectedPacket">{{ $t('trafficAnalysis.packetCapture.statusBar.selected') }}: #{{ selectedPacket.id }}</span>
                <span>{{ $t('trafficAnalysis.packetCapture.statusBar.captured') }}: {{ packets.length }} {{ $t('trafficAnalysis.packetCapture.statusBar.packets') }}</span>
            </div>
        </div>

        <PacketCaptureDialogs
            :context-menu="contextMenu"
            :field-context-menu="fieldContextMenu"
            :is-current-packet-marked="isCurrentPacketMarked"
            :is-current-packet-ignored="isCurrentPacketIgnored"
            :can-follow-tcp="canFollowTcp"
            :can-follow-udp="canFollowUdp"
            :can-follow-http="canFollowHttp"
            :show-filter-dialog="showFilterDialog"
            :advanced-filter="advancedFilter"
            :stream-dialog="streamDialog"
            :stream-segments="streamSegments"
            :show-extract-dialog="showExtractDialog"
            :extract-loading="extractLoading"
            :extracted-files="extractedFiles"
            :extract-filter="extractFilter"
            :has-extract-filter="hasExtractFilter"
            :available-source-types="availableSourceTypes"
            :filtered-extracted-files="filteredExtractedFiles"
            :selected-extract-file-ids="selectedExtractFileIds"
            :select-all-filtered-files="selectAllFilteredFiles"
            :source-type-stats="sourceTypeStats"
            :selected-files-total-size="selectedFilesTotalSize"
            :on-toggle-mark="toggleMark"
            :on-toggle-ignore="toggleIgnore"
            :on-filter-by-field="filterByField"
            :on-filter-by-conversation="filterByConversation"
            :on-follow-stream="followStream"
            :on-copy-packet-info="copyPacketInfo"
            :on-copy-packet-hex="copyPacketHex"
            :on-copy-field="copyField"
            :on-filter-by-field-value="filterByFieldValue"
            :on-copy-field-value="copyFieldValue"
            :on-reset-advanced-filter="resetAdvancedFilter"
            :on-apply-advanced-filter="applyAdvancedFilter"
            :on-copy-stream-content="copyStreamContent"
            :on-close-stream-dialog="closeStreamDialog"
            :on-toggle-select-all-filtered-files="toggleSelectAllFilteredFiles"
            :on-toggle-file-selection="toggleFileSelection"
            :on-clear-extract-filter="clearExtractFilter"
            :on-close-extract-dialog="closeExtractDialog"
            :on-save-extracted-files="saveExtractedFiles"
            :on-download-single-file="downloadSingleFile"
            :on-follow-file-stream="followFileStream"
            :on-locate-file-packets="locateFilePackets"
            :get-file-type-badge-class="getFileTypeBadgeClass"
            :get-file-type-label="getFileTypeLabel"
            :get-source-type-badge-class="getSourceTypeBadgeClass"
            :format-file-size="formatFileSize"
            @update:show-filter-dialog="showFilterDialog = $event"
        />
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import PacketCaptureDetails from './PacketCaptureDetails.vue'
import PacketCaptureDialogs from './PacketCaptureDialogs.vue'
import PacketCapturePacketList from './PacketCapturePacketList.vue'
import PacketCaptureToolbar from './PacketCaptureToolbar.vue'
import type { AdvancedFilter, ExtractedFileInfo, NetworkInterface, Packet, StreamSegment, VirtualItem } from './packetCaptureTypes'

const { t } = useI18n()

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
const showExtractDialog = ref(false)
const extractedFiles = ref<ExtractedFileInfo[]>([])
const selectedExtractFileIds = reactive(new Set<string>())
const selectAllFilteredFiles = ref(false)
const extractLoading = ref(false)

// Extract filter state
const extractFilter = reactive({
    filename: '',
    fileType: '',
    sourceType: '',
    sizeRange: ''
})

// 虚拟滚动相关
const scrollContainer = ref<{ scrollContainerEl: HTMLElement | null } | null>(null)
const rowHeight = 28 // 每行高度
const headerHeight = 32 // 表头高度
const scrollTop = ref(0)
const containerHeight = ref(300)
const bufferSize = 5 // 缓冲区大小
let scrollTimer: number | null = null

const getScrollContainerElement = () => scrollContainer.value?.scrollContainerEl ?? null

// 列宽状态（可拖动调整）
const columnWidths = reactive({
    mark: 48,      // 标记列
    no: 64,        // 序号
    time: 96,      // 时间
    source: 160,   // 源地址
    dest: 160,     // 目的地址
    protocol: 80,  // 协议
    length: 64,    // 长度
    // info 列是 flex-1，自动填充剩余空间
})

// 列拖拽调整状态
const columnResizing = reactive({
    active: false,
    column: '' as keyof typeof columnWidths | '',
    startX: 0,
    startWidth: 0,
})

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
const filterPlaceholder = computed(() => appliedFilter.value ? `${t('trafficAnalysis.packetCapture.toolbar.filtering')}: ${appliedFilter.value}` : t('trafficAnalysis.packetCapture.toolbar.filterPlaceholder'))
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

// 虚拟滚动 - 总高度
const totalHeight = computed(() => {
    return filteredPackets.value.length * rowHeight + headerHeight
})

// 虚拟滚动 - 可见项
const visibleItems = computed((): VirtualItem[] => {
    const startIndex = Math.max(0, Math.floor(scrollTop.value / rowHeight) - bufferSize)
    const visibleCount = Math.ceil(containerHeight.value / rowHeight)
    const endIndex = Math.min(filteredPackets.value.length, startIndex + visibleCount + bufferSize * 2)
    
    const items: VirtualItem[] = []
    for (let i = startIndex; i < endIndex; i++) {
        items.push({
            data: filteredPackets.value[i],
            offset: i * rowHeight
        })
    }
    return items
})

// 滚动处理
function handleScroll(e: Event) {
    const target = e.target as HTMLElement
    if (!target) return
    
    // 节流处理
    if (scrollTimer !== null) {
        window.clearTimeout(scrollTimer)
    }
    
    scrollTimer = window.setTimeout(() => {
        scrollTop.value = target.scrollTop
        scrollTimer = null
    }, 16) // ~60fps
}

// 更新容器高度
function updateContainerHeight() {
    const containerEl = getScrollContainerElement()
    if (containerEl) {
        containerHeight.value = containerEl.clientHeight
    }
}

// 端口匹配
function matchPort(addr: string, portFilter: string): boolean {
    const port = parseInt(addr.split(':')[1] || '0')
    if (portFilter.includes('-')) {
        const [min, max] = portFilter.split('-').map(Number)
        return port >= min && port <= max
    }
    return port === parseInt(portFilter)
}

// 列宽拖动调整
function startColumnResize(e: MouseEvent, column: keyof typeof columnWidths) {
    e.preventDefault()
    e.stopPropagation()
    columnResizing.active = true
    columnResizing.column = column
    columnResizing.startX = e.clientX
    columnResizing.startWidth = columnWidths[column]
    document.addEventListener('mousemove', doColumnResize)
    document.addEventListener('mouseup', stopColumnResize)
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
}

function doColumnResize(e: MouseEvent) {
    if (!columnResizing.active || !columnResizing.column) return
    const delta = e.clientX - columnResizing.startX
    const newWidth = Math.max(40, Math.min(columnResizing.startWidth + delta, 400))
    columnWidths[columnResizing.column as keyof typeof columnWidths] = newWidth
}

function stopColumnResize() {
    columnResizing.active = false
    columnResizing.column = ''
    document.removeEventListener('mousemove', doColumnResize)
    document.removeEventListener('mouseup', stopColumnResize)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
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
    scrollTop.value = 0
    const containerEl = getScrollContainerElement()
    if (containerEl) {
        containerEl.scrollTop = 0
    }
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
    // 重置滚动位置
    scrollTop.value = 0
    const containerEl = getScrollContainerElement()
    if (containerEl) {
        containerEl.scrollTop = 0
    }
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

const streamSegments = computed<StreamSegment[]>(() => {
    const segments: StreamSegment[] = []
    
    for (const p of streamDialog.packets) {
        const isClient = p.src === streamDialog.srcEndpoint
        let content = ''
        
        // 尝试提取应用层数据
        const payload = extractApplicationPayload(p)
        
        if (streamDialog.displayMode === 'ascii') {
            // 只显示可打印的 ASCII 字符和换行，并检测是否为有效文本
            const text = payload
                .map(b => (b >= 32 && b <= 126) || b === 10 || b === 13 || b === 9 ? String.fromCharCode(b) : '')
                .join('')
            
            // 只有当文本有足够可读内容时才显示
            if (text.trim().length > 0 && isLikelyText(payload)) {
                content = text
            }
        } else if (streamDialog.displayMode === 'hex') {
            if (payload.length > 0) {
                content = formatHex(payload)
            }
        } else {
            // raw 模式显示包信息
            content = `#${p.id} [${p.protocol}] Len=${p.length}`
        }
        
        if (content.trim()) {
            segments.push({ isClient, content, packetId: p.id })
        }
    }
    
    return segments
})

// 提取应用层数据（跳过协议头）
function extractApplicationPayload(p: Packet): number[] {
    const raw = p.raw
    
    // 尝试从 layers 中找到应用层数据的偏移
    // 首先检查是否有 HTTP/应用层数据
    const httpLayer = p.layers.find(l => l.name === 'HTTP' || l.name === 'http')
    const tcpLayer = p.layers.find(l => l.name === 'TCP' || l.name === 'tcp')
    const udpLayer = p.layers.find(l => l.name === 'UDP' || l.name === 'udp')
    
    // 简单的偏移计算：以太网头(14) + IP头(20-60) + TCP头(20-60) 或 UDP头(8)
    // 如果有应用层协议标识，尝试寻找应用层数据的起点
    
    let offset = 0
    
    // 检查是否为以太网帧：前 14 字节是以太网头
    if (raw.length > 14) {
        offset = 14
    }
    
    // 检查 IP 版本和头长度
    if (raw.length > offset) {
        const ipVersion = (raw[offset] >> 4) & 0xF
        if (ipVersion === 4) {
            const ipHeaderLen = (raw[offset] & 0xF) * 4
            offset += ipHeaderLen
        } else if (ipVersion === 6) {
            offset += 40 // IPv6 固定头长度
        }
    }
    
    // 检查传输层协议
    if (tcpLayer && raw.length > offset + 12) {
        // TCP 数据偏移在第 12-13 字节的高 4 位
        const dataOffset = ((raw[offset + 12] >> 4) & 0xF) * 4
        offset += dataOffset
    } else if (udpLayer && raw.length > offset + 8) {
        offset += 8 // UDP 头固定 8 字节
    }
    
    if (offset >= raw.length) {
        return []
    }
    
    return raw.slice(offset)
}

// 判断数据是否可能是文本内容
function isLikelyText(data: number[]): boolean {
    if (data.length === 0) return false
    
    let printableCount = 0
    const totalCount = Math.min(data.length, 200) // 只检查前 200 字节
    
    for (let i = 0; i < totalCount; i++) {
        const b = data[i]
        // 可打印字符、换行、回车、制表符
        if ((b >= 32 && b <= 126) || b === 10 || b === 13 || b === 9) {
            printableCount++
        }
    }
    
    // 如果超过 70% 是可打印字符，认为是文本
    return (printableCount / totalCount) > 0.7
}

function getStreamContent(): string {
    // 保留此函数用于复制功能
    const lines: string[] = []
    for (const segment of streamSegments.value) {
        const direction = segment.isClient ? '>>>' : '<<<'
        lines.push(`${direction}\n${segment.content}`)
    }
    return lines.join('\n\n') || '无数据'
}

async function copyStreamContent() {
    await navigator.clipboard.writeText(getStreamContent())
}

// PCAP file operations
async function openPcapFile() {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: '流量文件',
                extensions: ['pcap', 'pcapng', 'cap']
            }]
        })
        
        if (selected) {
            const filePath = typeof selected === 'string' ? selected : selected
            const loadedPackets = await invoke<Packet[]>('open_pcap_file', { filePath })
            
            // Clear existing and load new packets
            packets.value = []
            packetCounter = 0
            markedPackets.clear()
            ignoredPackets.clear()
            
            for (const pkt of loadedPackets) {
                packetCounter++
                packets.value.push({ ...pkt, id: packetCounter })
            }
            
            scrollTop.value = 0
            const containerEl = getScrollContainerElement()
            if (containerEl) {
                containerEl.scrollTop = 0
            }
        }
    } catch (e) {
        console.error('Failed to open pcap file:', e)
        alert('打开文件失败: ' + e)
    }
}

async function savePcapFile() {
    try {
        const selected = await save({
            filters: [{
                name: 'PCAP',
                extensions: ['pcap']
            }, {
                name: 'PCAPNG',
                extensions: ['pcapng']
            }],
            defaultPath: `capture_${Date.now()}.pcap`
        })
        
        if (selected) {
            await invoke('save_pcap_file', { 
                filePath: selected, 
                packets: packets.value 
            })
            alert('保存成功')
        }
    } catch (e) {
        console.error('Failed to save pcap file:', e)
        alert('保存文件失败: ' + e)
    }
}

// File extraction
async function showExtractDialogFn() {
    showExtractDialog.value = true
    extractLoading.value = true
    extractedFiles.value = []
    selectedExtractFileIds.clear()
    selectAllFilteredFiles.value = false
    clearExtractFilter()
    
    try {
        extractedFiles.value = await invoke<ExtractedFileInfo[]>('extract_files_preview', {
            packets: packets.value
        })
    } catch (e) {
        console.error('Failed to extract files:', e)
    } finally {
        extractLoading.value = false
    }
}

// Watch showExtractDialog to trigger file extraction
watch(showExtractDialog, (val) => {
    if (val && extractedFiles.value.length === 0 && !extractLoading.value) {
        showExtractDialogFn()
    }
})

// Watch selectAllFilteredFiles to update selection
watch(selectAllFilteredFiles, (val) => {
    if (val) {
        for (const file of filteredExtractedFiles.value) {
            selectedExtractFileIds.add(file.id)
        }
    }
})

function toggleSelectAllFilteredFiles() {
    if (selectAllFilteredFiles.value) {
        for (const file of filteredExtractedFiles.value) {
            selectedExtractFileIds.add(file.id)
        }
    } else {
        for (const file of filteredExtractedFiles.value) {
            selectedExtractFileIds.delete(file.id)
        }
    }
}

function toggleFileSelection(fileId: string) {
    if (selectedExtractFileIds.has(fileId)) {
        selectedExtractFileIds.delete(fileId)
    } else {
        selectedExtractFileIds.add(fileId)
    }
}

function clearExtractFilter() {
    extractFilter.filename = ''
    extractFilter.fileType = ''
    extractFilter.sourceType = ''
    extractFilter.sizeRange = ''
}

async function saveExtractedFiles() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: '选择保存目录'
        })
        
        if (selected) {
            const outputDir = typeof selected === 'string' ? selected : selected
            const fileIds = Array.from(selectedExtractFileIds)
            const result = await invoke<string[]>('save_selected_files', {
                fileIds,
                outputDir
            })
            
            alert(`成功导出 ${result.length} 个文件到:\n${outputDir}`)
        }
    } catch (e) {
        console.error('Failed to save extracted files:', e)
        alert('导出文件失败: ' + e)
    }
}

// Download a single file
async function downloadSingleFile(file: ExtractedFileInfo) {
    try {
        const selected = await save({
            defaultPath: file.filename,
            filters: [{
                name: '所有文件',
                extensions: ['*']
            }]
        })
        
        if (selected) {
            await invoke('save_extracted_file', {
                fileId: file.id,
                savePath: selected
            })
            alert('文件已保存')
        }
    } catch (e) {
        console.error('Failed to download file:', e)
        alert('下载失败: ' + e)
    }
}

// Follow the stream that contains the file
async function followFileStream(file: ExtractedFileInfo) {
    try {
        const streamPackets = await invoke<Packet[]>('get_file_stream_packets', {
            fileId: file.id,
            packets: packets.value
        })
        
        if (streamPackets.length === 0) {
            alert('未找到相关流量')
            return
        }
        
        // Show stream dialog
        const firstPkt = streamPackets[0]
        streamDialog.visible = true
        streamDialog.type = 'tcp'
        streamDialog.title = `文件流量 - ${file.filename}`
        streamDialog.packets = streamPackets
        streamDialog.displayMode = 'ascii'
        streamDialog.srcEndpoint = firstPkt.src
        
        closeExtractDialog()
    } catch (e) {
        console.error('Failed to get stream packets:', e)
        alert('获取流量失败: ' + e)
    }
}

// Locate and highlight packets related to the file
async function locateFilePackets(file: ExtractedFileInfo) {
    try {
        // Filter to show only packets related to this file
        const packetIdSet = new Set(file.packet_ids)
        
        // Find first related packet
        const firstPacketIdx = packets.value.findIndex(p => packetIdSet.has(p.id))
        
        if (firstPacketIdx === -1) {
            alert('未找到相关数据包')
            return
        }
        
        // Mark related packets
        for (const id of file.packet_ids) {
            markedPackets.add(id)
        }
        
        // Select first packet
        const firstPacket = packets.value[firstPacketIdx]
        selectPacket(firstPacket)
        
        // Scroll to the packet
        scrollTop.value = firstPacketIdx * rowHeight
        const containerEl = getScrollContainerElement()
        if (containerEl) {
            containerEl.scrollTop = scrollTop.value
        }
        
        closeExtractDialog()
        
    } catch (e) {
        console.error('Failed to locate packets:', e)
        alert('定位失败: ' + e)
    }
}

// Computed: filtered extracted files
const filteredExtractedFiles = computed(() => {
    return extractedFiles.value.filter(file => {
        // Filename filter
        if (extractFilter.filename && !file.filename.toLowerCase().includes(extractFilter.filename.toLowerCase())) {
            return false
        }
        
        // File type filter
        if (extractFilter.fileType) {
            const type = file.content_type.split(';')[0].trim().toLowerCase()
            const typeCategory = getFileTypeCategory(type)
            if (typeCategory !== extractFilter.fileType) return false
        }
        
        // Source type filter
        if (extractFilter.sourceType && file.source_type !== extractFilter.sourceType) {
            return false
        }
        
        // Size range filter
        if (extractFilter.sizeRange) {
            const size = file.size
            switch (extractFilter.sizeRange) {
                case 'tiny': if (size >= 1024) return false; break
                case 'small': if (size < 1024 || size >= 100 * 1024) return false; break
                case 'medium': if (size < 100 * 1024 || size >= 1024 * 1024) return false; break
                case 'large': if (size < 1024 * 1024 || size >= 10 * 1024 * 1024) return false; break
                case 'huge': if (size < 10 * 1024 * 1024) return false; break
            }
        }
        
        return true
    })
})

// Has active filter
const hasExtractFilter = computed(() => {
    return extractFilter.filename !== '' || extractFilter.fileType !== '' || 
           extractFilter.sourceType !== '' || extractFilter.sizeRange !== ''
})

// Available source types
const availableSourceTypes = computed(() => {
    const types = new Set<string>()
    for (const file of extractedFiles.value) {
        types.add(file.source_type)
    }
    return Array.from(types).sort()
})

// Computed: total size of selected files
const selectedFilesTotalSize = computed(() => {
    return extractedFiles.value
        .filter(f => selectedExtractFileIds.has(f.id))
        .reduce((sum, f) => sum + f.size, 0)
})

// Computed: source type statistics
const sourceTypeStats = computed(() => {
    const stats: Record<string, number> = {}
    for (const file of extractedFiles.value) {
        stats[file.source_type] = (stats[file.source_type] || 0) + 1
    }
    return Object.entries(stats).map(([type, count]) => ({ type, count }))
})

function getFileTypeCategory(mimeType: string): string {
    if (mimeType.startsWith('image/')) return 'image'
    if (mimeType.startsWith('video/')) return 'video'
    if (mimeType.startsWith('audio/')) return 'audio'
    if (mimeType.includes('zip') || mimeType.includes('rar') || mimeType.includes('7z') || 
        mimeType.includes('tar') || mimeType.includes('gzip') || mimeType.includes('bzip')) return 'archive'
    if (mimeType.includes('pdf') || mimeType.includes('msword') || mimeType.includes('document') ||
        mimeType.includes('rtf') || mimeType.includes('text/')) return 'document'
    if (mimeType.includes('executable') || mimeType.includes('msdownload') || 
        mimeType.includes('x-mach') || mimeType.includes('shellscript')) return 'executable'
    return 'other'
}

function closeExtractDialog() {
    showExtractDialog.value = false
    clearExtractFilter()
    selectedExtractFileIds.clear()
    selectAllFilteredFiles.value = false
}

function getFileTypeLabel(contentType: string): string {
    const type = contentType.split(';')[0].trim()
    const typeMap: Record<string, string> = {
        'image/jpeg': 'JPG',
        'image/png': 'PNG',
        'image/gif': 'GIF',
        'image/webp': 'WebP',
        'image/bmp': 'BMP',
        'image/x-icon': 'ICO',
        'application/pdf': 'PDF',
        'application/msword': 'DOC',
        'application/zip': 'ZIP',
        'application/x-gzip': 'GZ',
        'application/gzip': 'GZ',
        'application/x-rar-compressed': 'RAR',
        'application/x-7z-compressed': '7Z',
        'application/x-tar': 'TAR',
        'application/x-bzip2': 'BZ2',
        'application/json': 'JSON',
        'application/javascript': 'JS',
        'text/css': 'CSS',
        'video/mp4': 'MP4',
        'video/webm': 'WebM',
        'video/x-flv': 'FLV',
        'video/quicktime': 'MOV',
        'audio/mpeg': 'MP3',
        'audio/ogg': 'OGG',
        'audio/flac': 'FLAC',
        'application/x-msdownload': 'EXE',
        'application/x-executable': 'ELF',
        'font/ttf': 'TTF',
        'font/woff': 'WOFF',
        'font/woff2': 'WOFF2',
    }
    return typeMap[type] || type.split('/')[1]?.toUpperCase() || '文件'
}

function getFileTypeBadgeClass(contentType: string): string {
    const type = contentType.split(';')[0].trim()
    if (type.startsWith('image/')) return 'badge-success'
    if (type.startsWith('video/')) return 'badge-warning'
    if (type.startsWith('audio/')) return 'badge-accent'
    if (type.includes('zip') || type.includes('rar') || type.includes('7z') || type.includes('tar') || type.includes('gzip')) return 'badge-info'
    if (type.includes('pdf') || type.includes('msword')) return 'badge-error'
    if (type.includes('executable') || type.includes('msdownload')) return 'badge-neutral'
    return 'badge-ghost'
}

function getSourceTypeBadgeClass(sourceType: string): string {
    const classMap: Record<string, string> = {
        'HTTP': 'badge-success',
        'FTP': 'badge-info',
        'EMAIL': 'badge-warning',
        'STREAM': 'badge-secondary',
        'BASE64': 'badge-accent',
        'DNS_TUNNEL': 'badge-error',
        'ICMP_TUNNEL': 'badge-error',
        'TCP': 'badge-primary',
        'UDP': 'badge-info',
    }
    return classMap[sourceType] || 'badge-ghost'
}

function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B'
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
    return (bytes / 1024 / 1024).toFixed(1) + ' MB'
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
        'TLS': 'row-https', 'DNS': 'row-dns', 'ICMP': 'row-icmp', 'ICMPv6': 'row-icmp', 'ARP': 'row-arp'
    }
    return map[proto] || 'row-other'
}

function getProtocolBadgeClass(proto: string): string {
    const map: Record<string, string> = {
        'TCP': 'badge-secondary', 'UDP': 'badge-info', 'HTTP': 'badge-success', 
        'HTTPS': 'badge-warning', 'TLS': 'badge-warning', 'DNS': 'badge-accent', 
        'ICMP': 'badge-error', 'ICMPv6': 'badge-error', 'ARP': 'badge-neutral'
    }
    return map[proto] || 'badge-ghost'
}

function getLayerBgClass(name: string): string {
    const map: Record<string, string> = {
        'Frame': 'layer-frame', 'Ethernet': 'layer-eth', 
        'IPv4': 'layer-ip', 'IPv6': 'layer-ip',
        'TCP': 'layer-tcp', 'UDP': 'layer-udp', 
        'HTTP': 'layer-http', 'DNS': 'layer-dns', 
        'ICMP': 'layer-icmp', 'ICMPv6': 'layer-icmp', 'ARP': 'layer-arp'
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

let resizeObserver: ResizeObserver | null = null

// 监听列表高度变化
watch(listHeight, () => {
    nextTick(() => {
        updateContainerHeight()
    })
})

onMounted(() => {
    loadInterfaces()
    document.addEventListener('click', hideMenus)
    document.addEventListener('keydown', handleKeydown)
    
    // 监听容器大小变化
    const containerEl = getScrollContainerElement()
    if (containerEl) {
        updateContainerHeight()
        resizeObserver = new ResizeObserver(() => {
            updateContainerHeight()
        })
        resizeObserver.observe(containerEl)
    }
})

onUnmounted(() => {
    if (isCapturing.value) stopCapture()
    document.removeEventListener('click', hideMenus)
    document.removeEventListener('keydown', handleKeydown)
    if (resizeObserver) {
        resizeObserver.disconnect()
        resizeObserver = null
    }
    if (scrollTimer !== null) {
        window.clearTimeout(scrollTimer)
    }
})
</script>

<style scoped>
.resize-handle { @apply flex items-center justify-center cursor-ns-resize bg-base-200 hover:bg-base-300 h-1.5; }
.resize-bar { @apply bg-base-content/20 rounded-full w-16 h-1; }
.resize-handle:hover .resize-bar { @apply bg-base-content/40; }
</style>
