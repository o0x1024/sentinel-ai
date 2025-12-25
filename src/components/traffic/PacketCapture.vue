<template>
    <div class="h-full flex flex-col bg-base-100" @contextmenu.prevent>
        <!-- 工具栏 -->
        <div class="flex items-center gap-2 p-3 bg-base-200 border-b border-base-300">
            <!-- 网卡选择 -->
            <select v-model="selectedInterface" class="select select-sm select-bordered min-w-56"
                :disabled="isCapturing">
                <option value="">{{ $t('trafficAnalysis.packetCapture.toolbar.selectInterface') }}</option>
                <option v-for="iface in interfaces" :key="iface.name" :value="iface.name">
                    {{ getInterfaceDisplayName(iface) }}
                </option>
            </select>

            <!-- 开始/停止按钮 -->
            <button class="btn btn-sm" :class="isCapturing ? 'btn-error' : 'btn-success'"
                @click="toggleCapture" :disabled="!selectedInterface && !isCapturing">
                <i :class="isCapturing ? 'fas fa-stop' : 'fas fa-play'" class="mr-1"></i>
                {{ isCapturing ? $t('trafficAnalysis.packetCapture.toolbar.stop') : $t('trafficAnalysis.packetCapture.toolbar.start') }}
            </button>

            <!-- 清空按钮 -->
            <button class="btn btn-sm btn-ghost" @click="clearPackets" :disabled="packets.length === 0">
                <i class="fas fa-trash mr-1"></i>
                {{ $t('trafficAnalysis.packetCapture.toolbar.clear') }}
            </button>

            <div class="divider divider-horizontal mx-0"></div>

            <!-- 打开文件按钮 -->
            <button class="btn btn-sm btn-ghost" @click="openPcapFile" :disabled="isCapturing">
                <i class="fas fa-folder-open mr-1"></i>
                {{ $t('trafficAnalysis.packetCapture.toolbar.open') }}
            </button>

            <!-- 保存文件按钮 -->
            <button class="btn btn-sm btn-ghost" @click="savePcapFile" :disabled="packets.length === 0">
                <i class="fas fa-save mr-1"></i>
                {{ $t('trafficAnalysis.packetCapture.toolbar.save') }}
            </button>

            <!-- 导出文件按钮 -->
            <button class="btn btn-sm btn-ghost" @click="showExtractDialog = true" :disabled="packets.length === 0">
                <i class="fas fa-file-export mr-1"></i>
                {{ $t('trafficAnalysis.packetCapture.toolbar.export') }}
            </button>

            <div class="divider divider-horizontal mx-0"></div>

            <!-- 高级过滤按钮 -->
            <button class="btn btn-sm btn-ghost" @click="showFilterDialog = true">
                <i class="fas fa-sliders-h mr-1"></i>
                {{ $t('trafficAnalysis.packetCapture.toolbar.advancedFilter') }}
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
                <span v-if="hasAdvancedFilter" class="badge badge-info badge-sm">{{ $t('trafficAnalysis.packetCapture.toolbar.advancedFilterBadge') }}</span>
                <span class="badge badge-ghost">{{ filteredPackets.length }} / {{ packets.length }}</span>
            </div>
        </div>

        <!-- 主内容区 -->
        <div class="flex-1 flex flex-col min-h-0">
            <!-- 数据包列表 - 虚拟滚动 -->
            <div 
                ref="scrollContainer"
                class="min-h-0 overflow-auto border-b border-base-300 relative" 
                :style="{ flex: `0 0 ${listHeight}px` }"
                @scroll="handleScroll"
            >
                <!-- 虚拟滚动容器 -->
                <div 
                    v-if="filteredPackets.length > 0"
                    class="relative"
                    :style="{ height: totalHeight + 'px', minWidth: '900px' }"
                >
                    <!-- 表头 -->
                    <div class="sticky top-0 z-10 flex bg-base-200 border-b border-base-300" :style="{ height: headerHeight + 'px' }">
                        <div class="w-12 flex items-center justify-center px-2 border-r border-base-300"></div>
                        <div class="w-16 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.no') }}</div>
                        <div class="w-24 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.time') }}</div>
                        <div class="w-40 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.source') }}</div>
                        <div class="w-40 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.destination') }}</div>
                        <div class="w-20 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.protocol') }}</div>
                        <div class="w-16 flex items-center px-2 border-r border-base-300 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.length') }}</div>
                        <div class="flex-1 flex items-center px-2 text-xs font-semibold">{{ $t('trafficAnalysis.packetCapture.table.info') }}</div>
                    </div>

                    <!-- 数据行 - 虚拟渲染 -->
                    <div 
                        v-for="item in visibleItems" 
                        :key="item.data.id"
                        class="absolute left-0 right-0 flex cursor-pointer packet-row"
                        :class="[
                            getProtocolRowClass(item.data.protocol),
                            { 'selected-row': selectedPacket?.id === item.data.id },
                            { 'marked-row': markedPackets.has(item.data.id) },
                            { 'ignored-row': ignoredPackets.has(item.data.id) }
                        ]"
                        :style="{ 
                            top: (item.offset + headerHeight) + 'px', 
                            height: rowHeight + 'px'
                        }"
                        @click="selectPacket(item.data)"
                        @contextmenu.prevent="showContextMenu($event, item.data)"
                    >
                        <div class="w-12 flex items-center justify-center px-2 border-r border-base-300">
                            <i v-if="markedPackets.has(item.data.id)" class="fas fa-bookmark text-warning text-xs"></i>
                        </div>
                        <div class="w-16 flex items-center px-2 border-r border-base-300 font-mono text-xs">{{ item.data.id }}</div>
                        <div class="w-24 flex items-center px-2 border-r border-base-300 font-mono text-xs">{{ formatTime(item.data.timestamp) }}</div>
                        <div class="w-40 flex items-center px-2 border-r border-base-300 font-mono text-xs truncate">{{ item.data.src }}</div>
                        <div class="w-40 flex items-center px-2 border-r border-base-300 font-mono text-xs truncate">{{ item.data.dst }}</div>
                        <div class="w-20 flex items-center px-2 border-r border-base-300">
                            <span class="badge badge-sm" :class="getProtocolBadgeClass(item.data.protocol)">
                                {{ item.data.protocol }}
                            </span>
                        </div>
                        <div class="w-16 flex items-center px-2 border-r border-base-300 font-mono text-xs">{{ item.data.length }}</div>
                        <div class="flex-1 flex items-center px-2 text-xs truncate">{{ item.data.info }}</div>
                    </div>
                </div>

                <!-- 空状态 -->
                <div v-if="filteredPackets.length === 0" class="flex flex-col items-center justify-center h-full text-base-content/50 py-12">
                    <template v-if="isLoading">
                        <span class="loading loading-spinner loading-lg mb-4"></span>
                        <p>{{ $t('trafficAnalysis.packetCapture.emptyState.loadingInterfaces') }}</p>
                    </template>
                    <template v-else-if="loadError === 'no_interfaces'">
                        <i class="fas fa-exclamation-triangle text-4xl mb-4 text-warning"></i>
                        <p class="mb-2">{{ $t('trafficAnalysis.packetCapture.emptyState.noInterfaces') }}</p>
                        <p class="text-sm mb-4">{{ $t('trafficAnalysis.packetCapture.emptyState.npcapRequired') }}</p>
                        <a href="https://nmap.org/npcap/" target="_blank" class="btn btn-sm btn-primary">
                            <i class="fas fa-download mr-2"></i>{{ $t('trafficAnalysis.packetCapture.emptyState.downloadNpcap') }}
                        </a>
                    </template>
                    <template v-else-if="loadError">
                        <i class="fas fa-exclamation-circle text-4xl mb-4 text-error"></i>
                        <p class="text-sm text-error">{{ loadError }}</p>
                    </template>
                    <template v-else>
                        <i class="fas fa-broadcast-tower text-4xl mb-4"></i>
                        <p v-if="!isCapturing">{{ $t('trafficAnalysis.packetCapture.emptyState.selectAndStart') }}</p>
                        <p v-else>{{ $t('trafficAnalysis.packetCapture.emptyState.waitingForPackets') }}</p>
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
                    <i class="fas fa-circle animate-pulse mr-1"></i>{{ $t('trafficAnalysis.packetCapture.statusBar.capturing') }}
                </span>
            </div>
            <div class="flex items-center gap-4">
                <span v-if="selectedPacket">{{ $t('trafficAnalysis.packetCapture.statusBar.selected') }}: #{{ selectedPacket.id }}</span>
                <span>{{ $t('trafficAnalysis.packetCapture.statusBar.captured') }}: {{ packets.length }} {{ $t('trafficAnalysis.packetCapture.statusBar.packets') }}</span>
            </div>
        </div>

        <!-- 右键菜单 -->
        <div v-if="contextMenu.visible" class="context-menu"
             :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }" @click.stop>
            <ul class="menu menu-sm bg-base-100 rounded-lg shadow-xl border border-base-300 p-1 w-44">
                <li><a @click="toggleMark" class="text-xs"><i class="fas fa-bookmark w-3 mr-1"></i>{{ isCurrentPacketMarked ? $t('trafficAnalysis.packetCapture.contextMenu.unmark') : $t('trafficAnalysis.packetCapture.contextMenu.mark') }}</a></li>
                <li><a @click="toggleIgnore" class="text-xs"><i class="fas fa-eye-slash w-3 mr-1"></i>{{ isCurrentPacketIgnored ? $t('trafficAnalysis.packetCapture.contextMenu.unignore') : $t('trafficAnalysis.packetCapture.contextMenu.ignore') }}</a></li>
                <div class="divider my-0.5"></div>
                <!-- 过滤 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-filter w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.filter') }}</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="filterByField('src')" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.sourceAddress') }}</a></li>
                        <li><a @click="filterByField('dst')" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.destinationAddress') }}</a></li>
                        <li><a @click="filterByField('protocol')" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.protocol') }}</a></li>
                        <li><a @click="filterByConversation" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.conversation') }}</a></li>
                    </ul>
                </li>
                <!-- 追踪流 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-stream w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.followStream') }}</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="followStream('tcp')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowTcp }">{{ $t('trafficAnalysis.packetCapture.contextMenu.tcpStream') }}</a></li>
                        <li><a @click="followStream('udp')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowUdp }">{{ $t('trafficAnalysis.packetCapture.contextMenu.udpStream') }}</a></li>
                        <li><a @click="followStream('http')" class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowHttp }">{{ $t('trafficAnalysis.packetCapture.contextMenu.httpStream') }}</a></li>
                    </ul>
                </li>
                <div class="divider my-0.5"></div>
                <!-- 复制 - 右侧弹出 -->
                <li class="submenu-parent">
                    <a class="text-xs justify-between">
                        <span><i class="fas fa-copy w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.copy') }}</span>
                        <i class="fas fa-chevron-right text-xs"></i>
                    </a>
                    <ul class="submenu">
                        <li><a @click="copyPacketInfo" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.summary') }}</a></li>
                        <li><a @click="copyPacketHex" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.hex') }}</a></li>
                        <li><a @click="copyField('src')" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.sourceAddress') }}</a></li>
                        <li><a @click="copyField('dst')" class="text-xs">{{ $t('trafficAnalysis.packetCapture.contextMenu.destinationAddress') }}</a></li>
                    </ul>
                </li>
            </ul>
        </div>

        <!-- 字段右键菜单 -->
        <div v-if="fieldContextMenu.visible" class="context-menu"
             :style="{ left: fieldContextMenu.x + 'px', top: fieldContextMenu.y + 'px' }" @click.stop>
            <ul class="menu bg-base-100 rounded-box shadow-xl border border-base-300 p-1 w-64">
                <li><a @click="filterByFieldValue"><i class="fas fa-filter w-4"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.filterThisValue') }}</a></li>
                <li><a @click="copyFieldValue"><i class="fas fa-copy w-4"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.copy') }}: {{ fieldContextMenu.value }}</a></li>
            </ul>
        </div>

        <!-- 高级过滤对话框 -->
        <div v-if="showFilterDialog" class="modal modal-open">
            <div class="modal-box max-w-2xl">
                <h3 class="font-bold text-lg mb-4">
                    <i class="fas fa-sliders-h mr-2"></i>{{ $t('trafficAnalysis.packetCapture.filterDialog.title') }}
                </h3>
                
                <div class="space-y-4">
                    <!-- 协议过滤 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.protocol') }}</span></label>
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
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.sourceIp') }}</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.srcIp" placeholder="192.168.1.1" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.destinationIp') }}</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.dstIp" placeholder="10.0.0.1" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.sourcePort') }}</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.srcPort" placeholder="80 或 1000-2000" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.destinationPort') }}</span></label>
                            <input type="text" class="input input-sm input-bordered" 
                                   v-model="advancedFilter.dstPort" placeholder="443 或 8000-9000" />
                        </div>
                    </div>

                    <!-- 内容过滤 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.containsString') }}</span></label>
                        <input type="text" class="input input-sm input-bordered" 
                               v-model="advancedFilter.containsString" placeholder="GET /api, password" />
                    </div>

                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.containsHex') }}</span></label>
                        <input type="text" class="input input-sm input-bordered" 
                               v-model="advancedFilter.containsHex" placeholder="48 54 54 50 (HTTP)" />
                    </div>

                    <!-- 大小过滤 -->
                    <div class="grid grid-cols-2 gap-4">
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.minLength') }}</span></label>
                            <input type="number" class="input input-sm input-bordered" 
                                   v-model.number="advancedFilter.minLength" placeholder="0" />
                        </div>
                        <div class="form-control">
                            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.maxLength') }}</span></label>
                            <input type="number" class="input input-sm input-bordered" 
                                   v-model.number="advancedFilter.maxLength" placeholder="65535" />
                        </div>
                    </div>

                    <!-- TCP 标志 -->
                    <div class="form-control">
                        <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.tcpFlags') }}</span></label>
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
                    <button class="btn btn-ghost" @click="resetAdvancedFilter">{{ $t('trafficAnalysis.packetCapture.filterDialog.reset') }}</button>
                    <button class="btn btn-ghost" @click="showFilterDialog = false">{{ $t('trafficAnalysis.packetCapture.filterDialog.cancel') }}</button>
                    <button class="btn btn-primary" @click="applyAdvancedFilter">{{ $t('trafficAnalysis.packetCapture.filterDialog.apply') }}</button>
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
                            <option value="ascii">{{ $t('trafficAnalysis.packetCapture.streamDialog.ascii') }}</option>
                            <option value="hex">{{ $t('trafficAnalysis.packetCapture.streamDialog.hex') }}</option>
                            <option value="raw">{{ $t('trafficAnalysis.packetCapture.streamDialog.raw') }}</option>
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
                            <span class="w-3 h-3 rounded-full bg-error"></span> {{ $t('trafficAnalysis.packetCapture.streamDialog.clientToServer') }}
                        </span>
                        <span class="flex items-center gap-1">
                            <span class="w-3 h-3 rounded-full bg-info"></span> {{ $t('trafficAnalysis.packetCapture.streamDialog.serverToClient') }}
                        </span>
                        <span class="badge badge-ghost">{{ streamDialog.packets.length }} {{ $t('trafficAnalysis.packetCapture.streamDialog.packets') }}</span>
                    </div>
                    <button class="btn" @click="closeStreamDialog">{{ $t('trafficAnalysis.packetCapture.streamDialog.close') }}</button>
                </div>
            </div>
            <div class="modal-backdrop" @click="closeStreamDialog"></div>
        </div>

        <!-- 导出文件对话框 -->
        <div v-if="showExtractDialog" class="modal modal-open">
            <div class="modal-box max-w-5xl max-h-[90vh]">
                <h3 class="font-bold text-lg mb-4 flex items-center justify-between">
                    <span><i class="fas fa-file-export mr-2"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.title') }}</span>
                    <span v-if="!extractLoading && extractedFiles.length > 0" class="text-sm font-normal text-base-content/70">
                        {{ $t('trafficAnalysis.packetCapture.extractDialog.foundFiles', { count: extractedFiles.length }) }}
                    </span>
                </h3>
                
                <div v-if="extractLoading" class="flex flex-col items-center py-8">
                    <span class="loading loading-spinner loading-lg mb-4"></span>
                    <p>{{ $t('trafficAnalysis.packetCapture.extractDialog.analyzing') }}</p>
                    <p class="text-sm text-base-content/50 mt-2">{{ $t('trafficAnalysis.packetCapture.extractDialog.supportedProtocols') }}</p>
                </div>
                
                <div v-else-if="extractedFiles.length === 0" class="text-center py-8 text-base-content/50">
                    <i class="fas fa-inbox text-4xl mb-4"></i>
                    <p>{{ $t('trafficAnalysis.packetCapture.extractDialog.noFilesFound') }}</p>
                    <p class="text-sm mt-2">{{ $t('trafficAnalysis.packetCapture.extractDialog.supportedProtocols') }}</p>
                    <p class="text-xs mt-1">{{ $t('trafficAnalysis.packetCapture.extractDialog.protocolExamples') }}</p>
                </div>
                
                <div v-else>
                    <!-- 过滤器面板 -->
                    <div class="bg-base-200 rounded-lg p-3 mb-3">
                        <div class="flex items-center gap-2 mb-2">
                            <i class="fas fa-filter text-sm text-base-content/50"></i>
                            <span class="text-sm font-medium">{{ $t('trafficAnalysis.packetCapture.extractDialog.filterConditions') }}</span>
                            <button v-if="hasExtractFilter" class="btn btn-xs btn-ghost text-error" @click="clearExtractFilter">
                                <i class="fas fa-times mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.clearFilter') }}
                            </button>
                        </div>
                        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                            <!-- 文件名搜索 -->
                            <div class="form-control">
                                <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.filename') }}</span></label>
                                <input type="text" v-model="extractFilter.filename" 
                                       class="input input-xs input-bordered" placeholder="{{ $t('trafficAnalysis.packetCapture.extractDialog.searchFilename') }}" />
                            </div>
                            
                            <!-- 文件类型 -->
                            <div class="form-control">
                                <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.fileType') }}</span></label>
                                <select v-model="extractFilter.fileType" class="select select-xs select-bordered">
                                    <option value="">{{ $t('trafficAnalysis.packetCapture.extractDialog.allTypes') }}</option>
                                    <option value="image">{{ $t('trafficAnalysis.packetCapture.extractDialog.image') }}</option>
                                    <option value="video">{{ $t('trafficAnalysis.packetCapture.extractDialog.video') }}</option>
                                    <option value="audio">{{ $t('trafficAnalysis.packetCapture.extractDialog.audio') }}</option>
                                    <option value="archive">{{ $t('trafficAnalysis.packetCapture.extractDialog.archive') }}</option>
                                    <option value="document">{{ $t('trafficAnalysis.packetCapture.extractDialog.document') }}</option>
                                    <option value="executable">{{ $t('trafficAnalysis.packetCapture.extractDialog.executable') }}</option>
                                    <option value="other">{{ $t('trafficAnalysis.packetCapture.extractDialog.other') }}</option>
                                </select>
                            </div>
                            
                            <!-- 来源协议 -->
                            <div class="form-control">
                                <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.sourceProtocol') }}</span></label>
                                <select v-model="extractFilter.sourceType" class="select select-xs select-bordered">
                                    <option value="">{{ $t('trafficAnalysis.packetCapture.extractDialog.allSources') }}</option>
                                    <option v-for="st in availableSourceTypes" :key="st" :value="st">{{ st }}</option>
                                </select>
                            </div>
                            
                            <!-- 文件大小 -->
                            <div class="form-control">
                                <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.fileSize') }}</span></label>
                                <select v-model="extractFilter.sizeRange" class="select select-xs select-bordered">
                                    <option value="">{{ $t('trafficAnalysis.packetCapture.extractDialog.anySize') }}</option>
                                    <option value="tiny">{{ $t('trafficAnalysis.packetCapture.extractDialog.sizeTiny') }}</option>
                                    <option value="small">{{ $t('trafficAnalysis.packetCapture.extractDialog.sizeSmall') }}</option>
                                    <option value="medium">{{ $t('trafficAnalysis.packetCapture.extractDialog.sizeMedium') }}</option>
                                    <option value="large">{{ $t('trafficAnalysis.packetCapture.extractDialog.sizeLarge') }}</option>
                                    <option value="huge">{{ $t('trafficAnalysis.packetCapture.extractDialog.sizeHuge') }}</option>
                                </select>
                            </div>
                        </div>
                        
                        <!-- 快捷过滤按钮 -->
                        <div class="flex flex-wrap gap-1 mt-2">
                            <button class="btn btn-xs" :class="extractFilter.fileType === 'image' ? 'btn-success' : 'btn-ghost'" 
                                    @click="extractFilter.fileType = extractFilter.fileType === 'image' ? '' : 'image'">
                                <i class="fas fa-image mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.image') }}
                            </button>
                            <button class="btn btn-xs" :class="extractFilter.fileType === 'archive' ? 'btn-info' : 'btn-ghost'" 
                                    @click="extractFilter.fileType = extractFilter.fileType === 'archive' ? '' : 'archive'">
                                <i class="fas fa-file-archive mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.archive') }}
                            </button>
                            <button class="btn btn-xs" :class="extractFilter.fileType === 'document' ? 'btn-error' : 'btn-ghost'" 
                                    @click="extractFilter.fileType = extractFilter.fileType === 'document' ? '' : 'document'">
                                <i class="fas fa-file-pdf mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.document') }}
                            </button>
                            <button class="btn btn-xs" :class="extractFilter.fileType === 'executable' ? 'btn-warning' : 'btn-ghost'" 
                                    @click="extractFilter.fileType = extractFilter.fileType === 'executable' ? '' : 'executable'">
                                <i class="fas fa-cog mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.executable') }}
                            </button>
                            <span class="divider divider-horizontal mx-0"></span>
                            <button class="btn btn-xs" :class="extractFilter.sourceType === 'HTTP' ? 'btn-success' : 'btn-ghost'" 
                                    @click="extractFilter.sourceType = extractFilter.sourceType === 'HTTP' ? '' : 'HTTP'">{{ $t('trafficAnalysis.packetCapture.extractDialog.http') }}</button>
                            <button class="btn btn-xs" :class="extractFilter.sourceType === 'FTP' ? 'btn-info' : 'btn-ghost'" 
                                    @click="extractFilter.sourceType = extractFilter.sourceType === 'FTP' ? '' : 'FTP'">{{ $t('trafficAnalysis.packetCapture.extractDialog.ftp') }}</button>
                            <button class="btn btn-xs" :class="extractFilter.sourceType === 'EMAIL' ? 'btn-warning' : 'btn-ghost'" 
                                    @click="extractFilter.sourceType = extractFilter.sourceType === 'EMAIL' ? '' : 'EMAIL'">{{ $t('trafficAnalysis.packetCapture.extractDialog.email') }}</button>
                            <button class="btn btn-xs" :class="extractFilter.sourceType === 'DNS_TUNNEL' ? 'btn-error' : 'btn-ghost'" 
                                    @click="extractFilter.sourceType = extractFilter.sourceType === 'DNS_TUNNEL' ? '' : 'DNS_TUNNEL'">{{ $t('trafficAnalysis.packetCapture.extractDialog.dnsTunnel') }}</button>
                        </div>
                    </div>

                    <!-- 文件列表 -->
                    <div class="overflow-x-auto max-h-[45vh]">
                        <table class="table table-sm table-pin-rows">
                            <thead>
                                <tr>
                                    <th class="w-10">
                                        <input type="checkbox" class="checkbox checkbox-sm" 
                                               v-model="selectAllFilteredFiles" @change="toggleSelectAllFilteredFiles" />
                                    </th>
                                    <th>{{ $t('trafficAnalysis.packetCapture.extractDialog.filename') }}</th>
                                    <th class="w-20">{{ $t('trafficAnalysis.packetCapture.extractDialog.type') }}</th>
                                    <th class="w-20">{{ $t('trafficAnalysis.packetCapture.extractDialog.size') }}</th>
                                    <th class="w-20">{{ $t('trafficAnalysis.packetCapture.extractDialog.source') }}</th>
                                    <th class="w-36">{{ $t('trafficAnalysis.packetCapture.extractDialog.traffic') }}</th>
                                    <th class="w-28">{{ $t('trafficAnalysis.packetCapture.extractDialog.actions') }}</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="file in filteredExtractedFiles" :key="file.id" 
                                    class="hover" :class="{ 'bg-base-200': selectedExtractFileIds.has(file.id) }">
                                    <td>
                                        <input type="checkbox" class="checkbox checkbox-sm" 
                                               :checked="selectedExtractFileIds.has(file.id)"
                                               @change="toggleFileSelection(file.id)" />
                                    </td>
                                    <td class="font-mono text-sm max-w-48 truncate" :title="file.filename">
                                        {{ file.filename }}
                                    </td>
                                    <td>
                                        <span class="badge badge-sm" :class="getFileTypeBadgeClass(file.content_type)">
                                            {{ getFileTypeLabel(file.content_type) }}
                                        </span>
                                    </td>
                                    <td class="font-mono text-sm">{{ formatFileSize(file.size) }}</td>
                                    <td>
                                        <span class="badge badge-sm" :class="getSourceTypeBadgeClass(file.source_type)">
                                            {{ file.source_type }}
                                        </span>
                                    </td>
                                    <td class="text-xs text-base-content/70 truncate max-w-36" :title="`${file.src} → ${file.dst}`">
                                        {{ file.src.split(':')[0] }} → {{ file.dst.split(':')[0] }}
                                    </td>
                                    <td class="flex gap-1">
                                        <button class="btn btn-xs btn-ghost" @click="downloadSingleFile(file)" :title="$t('trafficAnalysis.packetCapture.extractDialog.downloadFile')">
                                            <i class="fas fa-download"></i>
                                        </button>
                                        <button class="btn btn-xs btn-ghost" @click="followFileStream(file)" :title="$t('trafficAnalysis.packetCapture.extractDialog.traceTraffic')">
                                            <i class="fas fa-stream"></i>
                                        </button>
                                        <button class="btn btn-xs btn-ghost" @click="locateFilePackets(file)" :title="$t('trafficAnalysis.packetCapture.extractDialog.locatePackets')">
                                            <i class="fas fa-crosshairs"></i>
                                        </button>
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                        
                        <!-- 过滤后无结果 -->
                        <div v-if="filteredExtractedFiles.length === 0 && extractedFiles.length > 0" 
                             class="text-center py-6 text-base-content/50">
                            <i class="fas fa-filter text-2xl mb-2"></i>
                            <p>{{ $t('trafficAnalysis.packetCapture.extractDialog.noMatchingFiles') }}</p>
                        </div>
                    </div>
                    
                    <!-- 状态栏 -->
                    <div class="flex items-center justify-between mt-3 pt-3 border-t border-base-300">
                        <div class="flex items-center gap-4">
                            <span class="text-sm text-base-content/70">
                                {{ $t('trafficAnalysis.packetCapture.extractDialog.selectedFiles', { count: selectedExtractFileIds.size }) }}
                                <span v-if="hasExtractFilter" class="text-xs">({{ $t('trafficAnalysis.packetCapture.extractDialog.displaying', { filtered: filteredExtractedFiles.length, total: extractedFiles.length }) }})</span>
                            </span>
                            <div class="flex flex-wrap gap-1 text-xs text-base-content/50">
                                <template v-for="st in sourceTypeStats" :key="st.type">
                                    <span class="badge badge-xs" :class="getSourceTypeBadgeClass(st.type)">{{ st.type }}</span>
                                    <span class="mr-2">{{ st.count }}</span>
                                </template>
                            </div>
                        </div>
                        <div class="text-sm text-base-content/50">
                            {{ $t('trafficAnalysis.packetCapture.extractDialog.selectedSize') }}: {{ formatFileSize(selectedFilesTotalSize) }}
                        </div>
                    </div>
                </div>

                <div class="modal-action">
                    <button class="btn btn-ghost" @click="closeExtractDialog">{{ $t('trafficAnalysis.packetCapture.extractDialog.close') }}</button>
                    <button class="btn btn-primary" @click="saveExtractedFiles" 
                            :disabled="selectedExtractFileIds.size === 0 || extractLoading">
                        <i class="fas fa-folder-open mr-1"></i>
                        {{ $t('trafficAnalysis.packetCapture.extractDialog.saveSelectedFiles', { count: selectedExtractFileIds.size }) }}
                    </button>
                </div>
            </div>
            <div class="modal-backdrop" @click="closeExtractDialog"></div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'

const { t } = useI18n()

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

interface ExtractedFileInfo {
    id: string
    filename: string
    content_type: string
    size: number
    src: string
    dst: string
    packet_ids: number[]
    stream_key: string
    source_type: string
}

// 虚拟列表类型
interface VirtualItem {
    data: Packet
    offset: number
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
const scrollContainer = ref<HTMLElement | null>(null)
const rowHeight = 28 // 每行高度
const headerHeight = 32 // 表头高度
const scrollTop = ref(0)
const containerHeight = ref(300)
const bufferSize = 5 // 缓冲区大小
let scrollTimer: number | null = null

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
    if (scrollContainer.value) {
        containerHeight.value = scrollContainer.value.clientHeight
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
    if (scrollContainer.value) {
        scrollContainer.value.scrollTop = 0
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
    if (scrollContainer.value) {
        scrollContainer.value.scrollTop = 0
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
            if (scrollContainer.value) {
                scrollContainer.value.scrollTop = 0
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
        if (scrollContainer.value) {
            scrollContainer.value.scrollTop = scrollTop.value
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
    if (scrollContainer.value) {
        updateContainerHeight()
        resizeObserver = new ResizeObserver(() => {
            updateContainerHeight()
        })
        resizeObserver.observe(scrollContainer.value)
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
.packet-row { 
    transition: background-color 0.1s;
    border-bottom: 1px solid oklch(var(--bc) / 0.1);
}
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
