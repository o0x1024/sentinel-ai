<template>
  <div v-if="contextMenu.visible" class="context-menu" :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }" @click.stop>
    <ul class="menu menu-sm bg-base-100 rounded-lg shadow-xl border border-base-300 p-1 w-44">
      <li><a class="text-xs" @click="onToggleMark"><i class="fas fa-bookmark w-3 mr-1"></i>{{ isCurrentPacketMarked ? $t('trafficAnalysis.packetCapture.contextMenu.unmark') : $t('trafficAnalysis.packetCapture.contextMenu.mark') }}</a></li>
      <li><a class="text-xs" @click="onToggleIgnore"><i class="fas fa-eye-slash w-3 mr-1"></i>{{ isCurrentPacketIgnored ? $t('trafficAnalysis.packetCapture.contextMenu.unignore') : $t('trafficAnalysis.packetCapture.contextMenu.ignore') }}</a></li>
      <div class="divider my-0.5"></div>
      <li class="submenu-parent">
        <a class="text-xs justify-between">
          <span><i class="fas fa-filter w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.filter') }}</span>
          <i class="fas fa-chevron-right text-xs"></i>
        </a>
        <ul class="submenu">
          <li><a class="text-xs" @click="onFilterByField('src')">{{ $t('trafficAnalysis.packetCapture.contextMenu.sourceAddress') }}</a></li>
          <li><a class="text-xs" @click="onFilterByField('dst')">{{ $t('trafficAnalysis.packetCapture.contextMenu.destinationAddress') }}</a></li>
          <li><a class="text-xs" @click="onFilterByField('protocol')">{{ $t('trafficAnalysis.packetCapture.contextMenu.protocol') }}</a></li>
          <li><a class="text-xs" @click="onFilterByConversation">{{ $t('trafficAnalysis.packetCapture.contextMenu.conversation') }}</a></li>
        </ul>
      </li>
      <li class="submenu-parent">
        <a class="text-xs justify-between">
          <span><i class="fas fa-stream w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.followStream') }}</span>
          <i class="fas fa-chevron-right text-xs"></i>
        </a>
        <ul class="submenu">
          <li><a class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowTcp }" @click="onFollowStream('tcp')">{{ $t('trafficAnalysis.packetCapture.contextMenu.tcpStream') }}</a></li>
          <li><a class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowUdp }" @click="onFollowStream('udp')">{{ $t('trafficAnalysis.packetCapture.contextMenu.udpStream') }}</a></li>
          <li><a class="text-xs" :class="{ 'opacity-40 pointer-events-none': !canFollowHttp }" @click="onFollowStream('http')">{{ $t('trafficAnalysis.packetCapture.contextMenu.httpStream') }}</a></li>
        </ul>
      </li>
      <div class="divider my-0.5"></div>
      <li class="submenu-parent">
        <a class="text-xs justify-between">
          <span><i class="fas fa-copy w-3 mr-1"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.copy') }}</span>
          <i class="fas fa-chevron-right text-xs"></i>
        </a>
        <ul class="submenu">
          <li><a class="text-xs" @click="onCopyPacketInfo">{{ $t('trafficAnalysis.packetCapture.contextMenu.summary') }}</a></li>
          <li><a class="text-xs" @click="onCopyPacketHex">{{ $t('trafficAnalysis.packetCapture.contextMenu.hex') }}</a></li>
          <li><a class="text-xs" @click="onCopyField('src')">{{ $t('trafficAnalysis.packetCapture.contextMenu.sourceAddress') }}</a></li>
          <li><a class="text-xs" @click="onCopyField('dst')">{{ $t('trafficAnalysis.packetCapture.contextMenu.destinationAddress') }}</a></li>
        </ul>
      </li>
    </ul>
  </div>

  <div v-if="fieldContextMenu.visible" class="context-menu" :style="{ left: fieldContextMenu.x + 'px', top: fieldContextMenu.y + 'px' }" @click.stop>
    <ul class="menu bg-base-100 rounded-box shadow-xl border border-base-300 p-1 w-64">
      <li><a @click="onFilterByFieldValue"><i class="fas fa-filter w-4"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.filterThisValue') }}</a></li>
      <li><a @click="onCopyFieldValue"><i class="fas fa-copy w-4"></i>{{ $t('trafficAnalysis.packetCapture.contextMenu.copy') }}: {{ fieldContextMenu.value }}</a></li>
    </ul>
  </div>

  <div v-if="showFilterDialog" class="modal modal-open">
    <div class="modal-box max-w-2xl">
      <h3 class="font-bold text-lg mb-4"><i class="fas fa-sliders-h mr-2"></i>{{ $t('trafficAnalysis.packetCapture.filterDialog.title') }}</h3>
      <div class="space-y-4">
        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.protocol') }}</span></label>
          <div class="flex flex-wrap gap-2">
            <label v-for="proto in ['TCP', 'UDP', 'HTTP', 'DNS', 'ICMP', 'ARP', 'TLS']" :key="proto" class="label cursor-pointer gap-2 bg-base-200 px-3 py-1 rounded-lg">
              <input v-model="advancedFilter.protocols" type="checkbox" class="checkbox checkbox-sm checkbox-primary" :value="proto" />
              <span class="label-text">{{ proto }}</span>
            </label>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.sourceIp') }}</span></label>
            <input v-model="advancedFilter.srcIp" type="text" class="input input-sm input-bordered" placeholder="192.168.1.1" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.destinationIp') }}</span></label>
            <input v-model="advancedFilter.dstIp" type="text" class="input input-sm input-bordered" placeholder="10.0.0.1" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.sourcePort') }}</span></label>
            <input v-model="advancedFilter.srcPort" type="text" class="input input-sm input-bordered" placeholder="80 或 1000-2000" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.destinationPort') }}</span></label>
            <input v-model="advancedFilter.dstPort" type="text" class="input input-sm input-bordered" placeholder="443 或 8000-9000" />
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.containsString') }}</span></label>
          <input v-model="advancedFilter.containsString" type="text" class="input input-sm input-bordered" placeholder="GET /api, password" />
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.containsHex') }}</span></label>
          <input v-model="advancedFilter.containsHex" type="text" class="input input-sm input-bordered" placeholder="48 54 54 50 (HTTP)" />
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.minLength') }}</span></label>
            <input v-model.number="advancedFilter.minLength" type="number" class="input input-sm input-bordered" placeholder="0" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.maxLength') }}</span></label>
            <input v-model.number="advancedFilter.maxLength" type="number" class="input input-sm input-bordered" placeholder="65535" />
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text font-medium">{{ $t('trafficAnalysis.packetCapture.filterDialog.tcpFlags') }}</span></label>
          <div class="flex flex-wrap gap-2">
            <label v-for="flag in ['SYN', 'ACK', 'FIN', 'RST', 'PSH', 'URG']" :key="flag" class="label cursor-pointer gap-2 bg-base-200 px-3 py-1 rounded-lg">
              <input v-model="advancedFilter.tcpFlags" type="checkbox" class="checkbox checkbox-sm checkbox-secondary" :value="flag" />
              <span class="label-text">{{ flag }}</span>
            </label>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="onResetAdvancedFilter">{{ $t('trafficAnalysis.packetCapture.filterDialog.reset') }}</button>
        <button class="btn btn-ghost" @click="$emit('update:showFilterDialog', false)">{{ $t('trafficAnalysis.packetCapture.filterDialog.cancel') }}</button>
        <button class="btn btn-primary" @click="onApplyAdvancedFilter">{{ $t('trafficAnalysis.packetCapture.filterDialog.apply') }}</button>
      </div>
    </div>
    <div class="modal-backdrop" @click="$emit('update:showFilterDialog', false)"></div>
  </div>

  <div v-if="streamDialog.visible" class="modal modal-open">
    <div class="modal-box max-w-4xl h-[80vh] flex flex-col">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-bold text-lg"><i class="fas fa-stream mr-2"></i>{{ streamDialog.title }}</h3>
        <div class="flex items-center gap-2">
          <select v-model="streamDialog.displayMode" class="select select-sm select-bordered">
            <option value="ascii">{{ $t('trafficAnalysis.packetCapture.streamDialog.ascii') }}</option>
            <option value="hex">{{ $t('trafficAnalysis.packetCapture.streamDialog.hex') }}</option>
            <option value="raw">{{ $t('trafficAnalysis.packetCapture.streamDialog.raw') }}</option>
          </select>
          <button class="btn btn-sm btn-ghost" @click="onCopyStreamContent"><i class="fas fa-copy"></i></button>
        </div>
      </div>

      <div class="flex-1 overflow-auto bg-base-200 rounded-lg p-2 stream-content">
        <template v-for="(segment, idx) in streamSegments" :key="idx">
          <div class="stream-segment font-mono text-sm p-2 mb-1 rounded" :class="segment.isClient ? 'stream-client' : 'stream-server'">
            <pre class="m-0 whitespace-pre-wrap break-all overflow-x-auto">{{ segment.content }}</pre>
          </div>
        </template>
        <div v-if="streamSegments.length === 0" class="text-center text-base-content/50 py-8">
          {{ $t('trafficAnalysis.packetCapture.streamDialog.noData') || '无有效数据' }}
        </div>
      </div>

      <div class="flex items-center justify-between mt-4">
        <div class="flex items-center gap-4 text-sm">
          <span class="flex items-center gap-1"><span class="w-3 h-3 rounded-full bg-error"></span> {{ $t('trafficAnalysis.packetCapture.streamDialog.clientToServer') }}</span>
          <span class="flex items-center gap-1"><span class="w-3 h-3 rounded-full bg-info"></span> {{ $t('trafficAnalysis.packetCapture.streamDialog.serverToClient') }}</span>
          <span class="badge badge-ghost">{{ streamDialog.packets.length }} {{ $t('trafficAnalysis.packetCapture.streamDialog.packets') }}</span>
        </div>
        <button class="btn" @click="onCloseStreamDialog">{{ $t('trafficAnalysis.packetCapture.streamDialog.close') }}</button>
      </div>
    </div>
    <div class="modal-backdrop" @click="onCloseStreamDialog"></div>
  </div>

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
        <div class="bg-base-200 rounded-lg p-3 mb-3">
          <div class="flex items-center gap-2 mb-2">
            <i class="fas fa-filter text-sm text-base-content/50"></i>
            <span class="text-sm font-medium">{{ $t('trafficAnalysis.packetCapture.extractDialog.filterConditions') }}</span>
            <button v-if="hasExtractFilter" class="btn btn-xs btn-ghost text-error" @click="onClearExtractFilter">
              <i class="fas fa-times mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.clearFilter') }}
            </button>
          </div>

          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div class="form-control">
              <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.filename') }}</span></label>
              <input v-model="extractFilter.filename" type="text" class="input input-xs input-bordered" :placeholder="$t('trafficAnalysis.packetCapture.extractDialog.searchFilename')" />
            </div>
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
            <div class="form-control">
              <label class="label py-0"><span class="label-text text-xs">{{ $t('trafficAnalysis.packetCapture.extractDialog.sourceProtocol') }}</span></label>
              <select v-model="extractFilter.sourceType" class="select select-xs select-bordered">
                <option value="">{{ $t('trafficAnalysis.packetCapture.extractDialog.allSources') }}</option>
                <option v-for="sourceType in availableSourceTypes" :key="sourceType" :value="sourceType">{{ sourceType }}</option>
              </select>
            </div>
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

          <div class="flex flex-wrap gap-1 mt-2">
            <button class="btn btn-xs" :class="extractFilter.fileType === 'image' ? 'btn-success' : 'btn-ghost'" @click="extractFilter.fileType = extractFilter.fileType === 'image' ? '' : 'image'"><i class="fas fa-image mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.image') }}</button>
            <button class="btn btn-xs" :class="extractFilter.fileType === 'archive' ? 'btn-info' : 'btn-ghost'" @click="extractFilter.fileType = extractFilter.fileType === 'archive' ? '' : 'archive'"><i class="fas fa-file-archive mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.archive') }}</button>
            <button class="btn btn-xs" :class="extractFilter.fileType === 'document' ? 'btn-error' : 'btn-ghost'" @click="extractFilter.fileType = extractFilter.fileType === 'document' ? '' : 'document'"><i class="fas fa-file-pdf mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.document') }}</button>
            <button class="btn btn-xs" :class="extractFilter.fileType === 'executable' ? 'btn-warning' : 'btn-ghost'" @click="extractFilter.fileType = extractFilter.fileType === 'executable' ? '' : 'executable'"><i class="fas fa-cog mr-1"></i>{{ $t('trafficAnalysis.packetCapture.extractDialog.executable') }}</button>
            <span class="divider divider-horizontal mx-0"></span>
            <button class="btn btn-xs" :class="extractFilter.sourceType === 'HTTP' ? 'btn-success' : 'btn-ghost'" @click="extractFilter.sourceType = extractFilter.sourceType === 'HTTP' ? '' : 'HTTP'">{{ $t('trafficAnalysis.packetCapture.extractDialog.http') }}</button>
            <button class="btn btn-xs" :class="extractFilter.sourceType === 'FTP' ? 'btn-info' : 'btn-ghost'" @click="extractFilter.sourceType = extractFilter.sourceType === 'FTP' ? '' : 'FTP'">{{ $t('trafficAnalysis.packetCapture.extractDialog.ftp') }}</button>
            <button class="btn btn-xs" :class="extractFilter.sourceType === 'EMAIL' ? 'btn-warning' : 'btn-ghost'" @click="extractFilter.sourceType = extractFilter.sourceType === 'EMAIL' ? '' : 'EMAIL'">{{ $t('trafficAnalysis.packetCapture.extractDialog.email') }}</button>
            <button class="btn btn-xs" :class="extractFilter.sourceType === 'DNS_TUNNEL' ? 'btn-error' : 'btn-ghost'" @click="extractFilter.sourceType = extractFilter.sourceType === 'DNS_TUNNEL' ? '' : 'DNS_TUNNEL'">{{ $t('trafficAnalysis.packetCapture.extractDialog.dnsTunnel') }}</button>
          </div>
        </div>

        <div class="overflow-x-auto max-h-[45vh]">
          <table class="table table-sm table-pin-rows">
            <thead>
              <tr>
                <th class="w-10">
                  <input type="checkbox" class="checkbox checkbox-sm" :checked="selectAllFilteredFiles" @change="onToggleSelectAllFilteredFiles" />
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
              <tr v-for="file in filteredExtractedFiles" :key="file.id" class="hover" :class="{ 'bg-base-200': selectedExtractFileIds.has(file.id) }">
                <td>
                  <input type="checkbox" class="checkbox checkbox-sm" :checked="selectedExtractFileIds.has(file.id)" @change="onToggleFileSelection(file.id)" />
                </td>
                <td class="font-mono text-sm max-w-48 truncate" :title="file.filename">{{ file.filename }}</td>
                <td><span class="badge badge-sm" :class="getFileTypeBadgeClass(file.content_type)">{{ getFileTypeLabel(file.content_type) }}</span></td>
                <td class="font-mono text-sm">{{ formatFileSize(file.size) }}</td>
                <td><span class="badge badge-sm" :class="getSourceTypeBadgeClass(file.source_type)">{{ file.source_type }}</span></td>
                <td class="text-xs text-base-content/70 truncate max-w-36" :title="`${file.src} → ${file.dst}`">{{ file.src.split(':')[0] }} → {{ file.dst.split(':')[0] }}</td>
                <td class="flex gap-1">
                  <button class="btn btn-xs btn-ghost" :title="$t('trafficAnalysis.packetCapture.extractDialog.downloadFile')" @click="onDownloadSingleFile(file)"><i class="fas fa-download"></i></button>
                  <button class="btn btn-xs btn-ghost" :title="$t('trafficAnalysis.packetCapture.extractDialog.traceTraffic')" @click="onFollowFileStream(file)"><i class="fas fa-stream"></i></button>
                  <button class="btn btn-xs btn-ghost" :title="$t('trafficAnalysis.packetCapture.extractDialog.locatePackets')" @click="onLocateFilePackets(file)"><i class="fas fa-crosshairs"></i></button>
                </td>
              </tr>
            </tbody>
          </table>

          <div v-if="filteredExtractedFiles.length === 0 && extractedFiles.length > 0" class="text-center py-6 text-base-content/50">
            <i class="fas fa-filter text-2xl mb-2"></i>
            <p>{{ $t('trafficAnalysis.packetCapture.extractDialog.noMatchingFiles') }}</p>
          </div>
        </div>

        <div class="flex items-center justify-between mt-3 pt-3 border-t border-base-300">
          <div class="flex items-center gap-4">
            <span class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.packetCapture.extractDialog.selectedFiles', { count: selectedExtractFileIds.size }) }}
              <span v-if="hasExtractFilter" class="text-xs">({{ $t('trafficAnalysis.packetCapture.extractDialog.displaying', { filtered: filteredExtractedFiles.length, total: extractedFiles.length }) }})</span>
            </span>
            <div class="flex flex-wrap gap-1 text-xs text-base-content/50">
              <template v-for="sourceTypeStat in sourceTypeStats" :key="sourceTypeStat.type">
                <span class="badge badge-xs" :class="getSourceTypeBadgeClass(sourceTypeStat.type)">{{ sourceTypeStat.type }}</span>
                <span class="mr-2">{{ sourceTypeStat.count }}</span>
              </template>
            </div>
          </div>
          <div class="text-sm text-base-content/50">{{ $t('trafficAnalysis.packetCapture.extractDialog.selectedSize') }}: {{ formatFileSize(selectedFilesTotalSize) }}</div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="onCloseExtractDialog">{{ $t('trafficAnalysis.packetCapture.extractDialog.close') }}</button>
        <button class="btn btn-primary" :disabled="selectedExtractFileIds.size === 0 || extractLoading" @click="onSaveExtractedFiles">
          <i class="fas fa-folder-open mr-1"></i>
          {{ $t('trafficAnalysis.packetCapture.extractDialog.saveSelectedFiles', { count: selectedExtractFileIds.size }) }}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" @click="onCloseExtractDialog"></div>
  </div>
</template>

<script setup lang="ts">
import type { AdvancedFilter, ExtractedFileInfo, Packet, StreamSegment } from './packetCaptureTypes'

defineProps<{
  contextMenu: { visible: boolean; x: number; y: number; packet: Packet | null }
  fieldContextMenu: { visible: boolean; x: number; y: number; key: string; value: string }
  isCurrentPacketMarked: boolean
  isCurrentPacketIgnored: boolean
  canFollowTcp: boolean | null
  canFollowUdp: boolean | null
  canFollowHttp: boolean | null
  showFilterDialog: boolean
  advancedFilter: AdvancedFilter
  streamDialog: { visible: boolean; title: string; packets: Packet[]; displayMode: 'ascii' | 'hex' | 'raw' }
  streamSegments: StreamSegment[]
  showExtractDialog: boolean
  extractLoading: boolean
  extractedFiles: ExtractedFileInfo[]
  extractFilter: { filename: string; fileType: string; sourceType: string; sizeRange: string }
  hasExtractFilter: boolean
  availableSourceTypes: string[]
  filteredExtractedFiles: ExtractedFileInfo[]
  selectedExtractFileIds: Set<string>
  selectAllFilteredFiles: boolean
  sourceTypeStats: Array<{ type: string; count: number }>
  selectedFilesTotalSize: number
  onToggleMark: () => void
  onToggleIgnore: () => void
  onFilterByField: (field: 'src' | 'dst' | 'protocol') => void
  onFilterByConversation: () => void
  onFollowStream: (type: string) => void
  onCopyPacketInfo: () => void
  onCopyPacketHex: () => void
  onCopyField: (field: 'src' | 'dst') => void
  onFilterByFieldValue: () => void
  onCopyFieldValue: () => void
  onResetAdvancedFilter: () => void
  onApplyAdvancedFilter: () => void
  onCopyStreamContent: () => void
  onCloseStreamDialog: () => void
  onToggleSelectAllFilteredFiles: () => void
  onToggleFileSelection: (fileId: string) => void
  onClearExtractFilter: () => void
  onCloseExtractDialog: () => void
  onSaveExtractedFiles: () => void
  onDownloadSingleFile: (file: ExtractedFileInfo) => void
  onFollowFileStream: (file: ExtractedFileInfo) => void
  onLocateFilePackets: (file: ExtractedFileInfo) => void
  getFileTypeBadgeClass: (contentType: string) => string
  getFileTypeLabel: (contentType: string) => string
  getSourceTypeBadgeClass: (sourceType: string) => string
  formatFileSize: (bytes: number) => string
}>()

defineEmits<{
  'update:showFilterDialog': [value: boolean]
}>()
</script>

<style scoped>
.context-menu { @apply fixed z-50; }

.submenu-parent {
  position: relative;
}

.submenu-parent > a {
  width: 100%;
  display: flex !important;
  align-items: center;
}

.submenu {
  position: absolute;
  visibility: hidden;
  opacity: 0;
  left: 100%;
  top: 0;
  margin-left: 2px;
  min-width: 140px;
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.1);
  border-radius: 0.5rem;
  box-shadow: var(--shadow-xl);
  padding: 0.25rem;
  z-index: 100;
  transition: opacity 0.15s ease-in-out, visibility 0.15s ease-in-out;
}

.submenu li {
  list-style: none;
}

.submenu li a {
  display: block;
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  border-radius: 0.25rem;
  cursor: pointer;
  white-space: nowrap;
}

.submenu li a:hover {
  background: oklch(var(--bc) / 0.1);
}

.submenu-parent:hover > .submenu {
  visibility: visible;
  opacity: 1;
}

.stream-content {
  background-color: #f5f5f5;
}

.stream-segment {
  border-left: 3px solid transparent;
  max-width: 100%;
  overflow: hidden;
}

.stream-segment pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
  word-break: break-all;
  max-width: 100%;
  overflow-wrap: break-word;
}

.stream-client {
  background-color: rgba(255, 190, 190, 0.6);
  border-left-color: #e57373;
}

.stream-server {
  background-color: rgba(187, 222, 251, 0.6);
  border-left-color: #64b5f6;
}

:global(.dark) .stream-content {
  background-color: #1a1a1a;
}

:global(.dark) .stream-client {
  background-color: rgba(180, 80, 80, 0.35);
  border-left-color: #ef5350;
}

:global(.dark) .stream-server {
  background-color: rgba(66, 135, 180, 0.35);
  border-left-color: #42a5f5;
}
</style>
