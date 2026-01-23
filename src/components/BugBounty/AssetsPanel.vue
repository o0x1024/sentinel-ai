<template>
  <div class="space-y-6">
    <!-- 操作栏 -->
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">{{ $t('assetManagement.title') }}</h2>
      <div class="flex space-x-2">
        <button @click="showColumnsModal = true" class="btn btn-outline btn-sm">
          <i class="fas fa-columns mr-2"></i>
          {{ $t('assetManagement.customizeColumns') }}
        </button>
        <button @click="$emit('discover-assets')" class="btn btn-primary btn-sm">
          <i class="fas fa-search mr-2"></i>
          {{ $t('bugBounty.monitor.discoverAssets') }}
        </button>
        <button @click="refreshAssets" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 高级筛选 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-medium">{{ $t('assetManagement.filters') }}</h3>
        <button @click="resetFilters" class="btn btn-ghost btn-xs">
          <i class="fas fa-redo mr-1"></i>
          {{ $t('common.reset') }}
        </button>
      </div>
      
      <!-- 第一行筛选 -->
      <div class="grid grid-cols-1 md:grid-cols-6 gap-3 mb-3">
        <div class="form-control">
          <input 
            v-model="filters.search" 
            type="text" 
            :placeholder="$t('common.search') + '...'"
            class="input input-bordered input-sm"
            @input="applyFilters"
          />
        </div>
        <div class="form-control">
          <select v-model="filters.programId" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('common.allPrograms') }}</option>
            <option v-for="program in programs" :key="program.id" :value="program.id">
              {{ program.name }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.assetType" class="select select-bordered select-sm" @change="onAssetTypeChange">
            <option value="">{{ $t('assetManagement.allTypes') }}</option>
            <option v-for="type in assetTypes" :key="type" :value="type">
              {{ type }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.riskLevel" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allRiskLevels') }}</option>
            <option value="High">{{ $t('assetManagement.riskLevel.high') }}</option>
            <option value="Medium">{{ $t('assetManagement.riskLevel.medium') }}</option>
            <option value="Low">{{ $t('assetManagement.riskLevel.low') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.status" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allStatuses') }}</option>
            <option value="Active">{{ $t('assetManagement.status.active') }}</option>
            <option value="Inactive">{{ $t('assetManagement.status.inactive') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.exposureLevel" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('common.allExposureLevels') }}</option>
            <option value="internet">{{ $t('common.exposureLevel.internet') }}</option>
            <option value="intranet">{{ $t('common.exposureLevel.intranet') }}</option>
            <option value="private">{{ $t('common.exposureLevel.private') }}</option>
          </select>
        </div>
      </div>

      <!-- 第二行筛选 (ASM特有) -->
      <div class="grid grid-cols-1 md:grid-cols-5 gap-3">
        <div class="form-control">
          <select v-model="filters.country" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allCountries') }}</option>
            <option v-for="country in countries" :key="country" :value="country">
              {{ country }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.cloudProvider" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allCloudProviders') }}</option>
            <option v-for="provider in cloudProviders" :key="provider" :value="provider">
              {{ provider }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.serviceName" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allServices') }}</option>
            <option v-for="service in serviceNames" :key="service" :value="service">
              {{ service }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.cdnDetected" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allCDNs') }}</option>
            <option v-for="cdn in cdns" :key="cdn" :value="cdn">
              {{ cdn }}
            </option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="filters.wafDetected" class="select select-bordered select-sm" @change="applyFilters">
            <option value="">{{ $t('assetManagement.allWAFs') }}</option>
            <option v-for="waf in wafs" :key="waf" :value="waf">
              {{ waf }}
            </option>
          </select>
        </div>
      </div>
    </div>

    <!-- 资产列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div v-if="loading" class="flex justify-center py-8">
        <span class="loading loading-spinner loading-lg"></span>
      </div>

      <div v-else-if="filteredAssets.length > 0" class="overflow-x-auto">
        <table class="table table-zebra table-xs">
          <thead>
            <tr>
              <th v-if="visibleColumns.name">{{ $t('assetManagement.name') }}</th>
              <th v-if="visibleColumns.type">{{ $t('assetManagement.type') }}</th>
              <th v-if="visibleColumns.value">{{ $t('assetManagement.value') }}</th>
              
              <!-- IP Asset Columns -->
              <th v-if="visibleColumns.ip_version && showIpColumns">{{ $t('assetManagement.columns.ipVersion') }}</th>
              <th v-if="visibleColumns.asn && showIpColumns">{{ $t('assetManagement.columns.asn') }}</th>
              <th v-if="visibleColumns.country && showIpColumns">{{ $t('assetManagement.columns.country') }}</th>
              <th v-if="visibleColumns.cloud_provider && showIpColumns">{{ $t('assetManagement.columns.cloudProvider') }}</th>
              
              <!-- Port/Service Columns -->
              <th v-if="visibleColumns.port && showPortColumns">{{ $t('assetManagement.columns.port') }}</th>
              <th v-if="visibleColumns.service_name && showPortColumns">{{ $t('assetManagement.columns.serviceName') }}</th>
              <th v-if="visibleColumns.service_version && showPortColumns">{{ $t('assetManagement.columns.serviceVersion') }}</th>
              <th v-if="visibleColumns.transport_protocol && showPortColumns">{{ $t('assetManagement.columns.transportProtocol') }}</th>
              
              <!-- Domain Columns -->
              <th v-if="visibleColumns.domain_registrar && showDomainColumns">{{ $t('assetManagement.columns.domainRegistrar') }}</th>
              <th v-if="visibleColumns.expiration_date && showDomainColumns">{{ $t('assetManagement.columns.expirationDate') }}</th>
              <th v-if="visibleColumns.parent_domain && showDomainColumns">{{ $t('assetManagement.columns.parentDomain') }}</th>
              
              <!-- Web/URL Columns -->
              <th v-if="visibleColumns.http_status && showWebColumns">{{ $t('assetManagement.columns.httpStatus') }}</th>
              <th v-if="visibleColumns.title && showWebColumns">{{ $t('assetManagement.columns.title') }}</th>
              <th v-if="visibleColumns.cdn && showWebColumns">{{ $t('assetManagement.columns.cdn') }}</th>
              <th v-if="visibleColumns.waf && showWebColumns">{{ $t('assetManagement.columns.waf') }}</th>
              <th v-if="visibleColumns.techStack && showWebColumns">{{ $t('assetManagement.columns.techStack') }}</th>
              
              <!-- Certificate Columns -->
              <th v-if="visibleColumns.ssl && showCertColumns">{{ $t('assetManagement.columns.ssl') }}</th>
              <th v-if="visibleColumns.certIssuer && showCertColumns">{{ $t('assetManagement.columns.certIssuer') }}</th>
              <th v-if="visibleColumns.certExpiry && showCertColumns">{{ $t('assetManagement.columns.certExpiry') }}</th>
              
              <!-- Common Columns -->
              <th v-if="visibleColumns.attack_surface_score">{{ $t('bugBounty.assets.attackSurface') }}</th>
              <th v-if="visibleColumns.exposure_level">{{ $t('bugBounty.assets.exposure') }}</th>
              <th v-if="visibleColumns.risk_level">{{ $t('assetManagement.riskLevel.title') }}</th>
              <th v-if="visibleColumns.status">{{ $t('common.status') }}</th>
              <th v-if="visibleColumns.last_seen">{{ $t('assetManagement.lastSeen') }}</th>
              <th v-if="visibleColumns.discovery_method">{{ $t('assetManagement.columns.discoveryMethod') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="asset in paginatedAssets" :key="asset.id">
              <td v-if="visibleColumns.name">
                <div class="flex items-center space-x-2">
                  <i :class="getAssetIcon(asset.asset_type)" class="text-xs"></i>
                  <span class="font-medium text-xs">{{ asset.name }}</span>
                </div>
              </td>
              <td v-if="visibleColumns.type">
                <span class="badge badge-outline badge-xs">{{ asset.asset_type }}</span>
              </td>
              <td v-if="visibleColumns.value">
                <code class="text-xs bg-base-200 px-1 py-0.5 rounded">{{ truncate(asset.value, 40) }}</code>
              </td>
              
              <!-- IP Asset Columns -->
              <td v-if="visibleColumns.ip_version && showIpColumns">
                <span class="text-xs">{{ asset.ip_version || '-' }}</span>
              </td>
              <td v-if="visibleColumns.asn && showIpColumns">
                <span class="text-xs">{{ asset.asn ? `AS${asset.asn}` : '-' }}</span>
              </td>
              <td v-if="visibleColumns.country && showIpColumns">
                <span class="text-xs">{{ asset.country || '-' }}</span>
              </td>
              <td v-if="visibleColumns.cloud_provider && showIpColumns">
                <span v-if="asset.cloud_provider" class="badge badge-info badge-xs">{{ asset.cloud_provider }}</span>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              
              <!-- Port/Service Columns -->
              <td v-if="visibleColumns.port && showPortColumns">
                <span class="text-xs font-mono">{{ asset.port || '-' }}</span>
              </td>
              <td v-if="visibleColumns.service_name && showPortColumns">
                <span class="badge badge-ghost badge-xs">{{ asset.service_name || '-' }}</span>
              </td>
              <td v-if="visibleColumns.service_version && showPortColumns">
                <span class="text-xs">{{ asset.service_version || '-' }}</span>
              </td>
              <td v-if="visibleColumns.transport_protocol && showPortColumns">
                <span class="text-xs">{{ asset.transport_protocol || '-' }}</span>
              </td>
              
              <!-- Domain Columns -->
              <td v-if="visibleColumns.domain_registrar && showDomainColumns">
                <span class="text-xs">{{ asset.domain_registrar || '-' }}</span>
              </td>
              <td v-if="visibleColumns.expiration_date && showDomainColumns">
                <span class="text-xs" :class="{ 'text-error': isExpiringSoon(asset.expiration_date) }">
                  {{ formatDate(asset.expiration_date) }}
                </span>
              </td>
              <td v-if="visibleColumns.parent_domain && showDomainColumns">
                <span class="text-xs">{{ asset.parent_domain || '-' }}</span>
              </td>
              
              <!-- Web/URL Columns -->
              <td v-if="visibleColumns.http_status && showWebColumns">
                <span class="badge badge-xs" :class="getHttpStatusClass(asset.http_status)">
                  {{ asset.http_status || '-' }}
                </span>
              </td>
              <td v-if="visibleColumns.title && showWebColumns">
                <span class="text-xs">{{ truncate(asset.title, 30) }}</span>
              </td>
              <td v-if="visibleColumns.cdn && showWebColumns">
                <span v-if="asset.cdn_detected" class="badge badge-primary badge-xs">{{ asset.cdn_detected }}</span>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              <td v-if="visibleColumns.waf && showWebColumns">
                <span v-if="asset.waf_detected" class="badge badge-warning badge-xs">{{ asset.waf_detected }}</span>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              <td v-if="visibleColumns.techStack && showWebColumns">
                <div v-if="asset.tech_stack && asset.tech_stack.length > 0" class="flex gap-1 flex-wrap">
                  <span v-for="(tech, idx) in asset.tech_stack.slice(0, 2)" :key="idx" class="badge badge-ghost badge-xs">
                    {{ tech }}
                  </span>
                  <span v-if="asset.tech_stack.length > 2" class="text-xs opacity-50">+{{ asset.tech_stack.length - 2 }}</span>
                </div>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              
              <!-- Certificate Columns -->
              <td v-if="visibleColumns.ssl && showCertColumns">
                <span class="badge badge-xs" :class="asset.ssl_enabled ? 'badge-success' : 'badge-ghost'">
                  {{ asset.ssl_enabled ? 'Yes' : 'No' }}
                </span>
              </td>
              <td v-if="visibleColumns.certIssuer && showCertColumns">
                <span class="text-xs">{{ truncate(asset.certificate_issuer, 20) }}</span>
              </td>
              <td v-if="visibleColumns.certExpiry && showCertColumns">
                <span class="text-xs" :class="{ 'text-error': isExpiringSoon(asset.certificate_valid_to) }">
                  {{ formatDate(asset.certificate_valid_to) }}
                </span>
              </td>
              
              <!-- Common Columns -->
              <td v-if="visibleColumns.attack_surface_score">
                <div v-if="asset.attack_surface_score !== null && asset.attack_surface_score !== undefined" class="flex items-center space-x-1">
                  <div class="w-12 h-1.5 bg-base-300 rounded-full overflow-hidden">
                    <div 
                      class="h-full transition-all" 
                      :class="getAttackSurfaceScoreColor(asset.attack_surface_score)"
                      :style="{ width: asset.attack_surface_score + '%' }"
                    ></div>
                  </div>
                  <span class="text-xs font-mono">{{ Math.round(asset.attack_surface_score) }}</span>
                </div>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              <td v-if="visibleColumns.exposure_level">
                <span v-if="asset.exposure_level" class="badge badge-xs" :class="getExposureLevelClass(asset.exposure_level)">
                  {{ asset.exposure_level }}
                </span>
                <span v-else class="text-xs opacity-50">-</span>
              </td>
              <td v-if="visibleColumns.risk_level">
                <span class="badge badge-xs" :class="getRiskLevelClass(asset.risk_level)">
                  {{ asset.risk_level }}
                </span>
              </td>
              <td v-if="visibleColumns.status">
                <span class="badge badge-xs" :class="getStatusClass(asset.status)">
                  {{ asset.status }}
                </span>
              </td>
              <td v-if="visibleColumns.last_seen" class="text-xs opacity-70">
                {{ asset.last_seen ? formatTime(asset.last_seen) : '-' }}
              </td>
              <td v-if="visibleColumns.discovery_method">
                <span class="badge badge-ghost badge-xs">{{ asset.discovery_method || '-' }}</span>
              </td>
              <td>
                <button @click="viewAssetDetail(asset)" class="btn btn-ghost btn-xs">
                  <i class="fas fa-eye text-xs"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>

        <!-- 分页 -->
        <div class="flex justify-center py-4">
          <div class="join">
            <button @click="currentPage--" :disabled="currentPage === 1" class="join-item btn btn-sm">«</button>
            <button class="join-item btn btn-sm">{{ currentPage }} / {{ totalPages }}</button>
            <button @click="currentPage++" :disabled="currentPage === totalPages" class="join-item btn btn-sm">»</button>
          </div>
        </div>
      </div>

      <div v-else class="text-center py-8 text-base-content/50">
        <svg class="w-16 h-16 mx-auto mb-2 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"></path>
        </svg>
        <p class="text-sm">{{ $t('assetManagement.noAssets') }}</p>
      </div>
    </div>

    <!-- 自定义列模态框 -->
    <div v-if="showColumnsModal" class="modal modal-open">
      <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('assetManagement.customizeColumns') }}</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <!-- 基础列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.basic') }}</h4>
            <div class="space-y-1">
              <label v-for="col in basicColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
          
          <!-- IP资产列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.ip') }}</h4>
            <div class="space-y-1">
              <label v-for="col in ipColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
          
          <!-- 端口/服务列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.port') }}</h4>
            <div class="space-y-1">
              <label v-for="col in portColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
          
          <!-- 域名列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.domain') }}</h4>
            <div class="space-y-1">
              <label v-for="col in domainColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
          
          <!-- Web/URL列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.web') }}</h4>
            <div class="space-y-1">
              <label v-for="col in webColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
          
          <!-- 证书列 -->
          <div>
            <h4 class="font-semibold mb-2 text-sm">{{ $t('assetManagement.columnGroups.certificate') }}</h4>
            <div class="space-y-1">
              <label v-for="col in certColumns" :key="col" class="flex items-center space-x-2 cursor-pointer">
                <input type="checkbox" v-model="visibleColumns[col]" class="checkbox checkbox-sm" />
                <span class="text-xs">{{ $t(`assetManagement.columns.${col}`) }}</span>
              </label>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="resetColumns" class="btn btn-ghost btn-sm">{{ $t('common.reset') }}</button>
          <button @click="saveColumnSettings" class="btn btn-primary btn-sm">{{ $t('common.save') }}</button>
          <button @click="showColumnsModal = false" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>

    <!-- 资产详情模态框 -->
    <div v-if="showDetailModal && selectedAsset" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('assetManagement.assetDetail') }}</h3>
        
        <div class="grid grid-cols-2 gap-4">
          <!-- 基础信息 -->
          <div class="col-span-2">
            <h4 class="font-semibold mb-2">{{ $t('assetManagement.columnGroups.basic') }}</h4>
            <div class="bg-base-200 p-3 rounded space-y-1 text-sm">
              <p><strong>{{ $t('assetManagement.name') }}:</strong> {{ selectedAsset.name }}</p>
              <p><strong>{{ $t('assetManagement.type') }}:</strong> {{ selectedAsset.asset_type }}</p>
              <p><strong>{{ $t('assetManagement.value') }}:</strong> {{ selectedAsset.value }}</p>
              <p><strong>{{ $t('assetManagement.riskLevel.title') }}:</strong> 
                <span class="badge badge-sm" :class="getRiskLevelClass(selectedAsset.risk_level)">
                  {{ selectedAsset.risk_level }}
                </span>
              </p>
              <p><strong>{{ $t('common.status') }}:</strong> 
                <span class="badge badge-sm" :class="getStatusClass(selectedAsset.status)">
                  {{ selectedAsset.status }}
                </span>
              </p>
            </div>
          </div>
          
          <!-- IP信息 -->
          <div v-if="selectedAsset.asn || selectedAsset.country">
            <h4 class="font-semibold mb-2">{{ $t('assetManagement.columnGroups.ip') }}</h4>
            <div class="bg-base-200 p-3 rounded space-y-1 text-sm">
              <p v-if="selectedAsset.ip_version"><strong>{{ $t('assetManagement.columns.ipVersion') }}:</strong> {{ selectedAsset.ip_version }}</p>
              <p v-if="selectedAsset.asn"><strong>{{ $t('assetManagement.columns.asn') }}:</strong> AS{{ selectedAsset.asn }}</p>
              <p v-if="selectedAsset.asn_org"><strong>{{ $t('assetManagement.columns.asnOrg') }}:</strong> {{ selectedAsset.asn_org }}</p>
              <p v-if="selectedAsset.country"><strong>{{ $t('assetManagement.columns.country') }}:</strong> {{ selectedAsset.country }}</p>
              <p v-if="selectedAsset.city"><strong>{{ $t('assetManagement.columns.city') }}:</strong> {{ selectedAsset.city }}</p>
              <p v-if="selectedAsset.cloud_provider"><strong>{{ $t('assetManagement.columns.cloudProvider') }}:</strong> {{ selectedAsset.cloud_provider }}</p>
            </div>
          </div>
          
          <!-- 服务信息 -->
          <div v-if="selectedAsset.service_name || selectedAsset.port">
            <h4 class="font-semibold mb-2">{{ $t('assetManagement.columnGroups.port') }}</h4>
            <div class="bg-base-200 p-3 rounded space-y-1 text-sm">
              <p v-if="selectedAsset.port"><strong>{{ $t('assetManagement.columns.port') }}:</strong> {{ selectedAsset.port }}</p>
              <p v-if="selectedAsset.service_name"><strong>{{ $t('assetManagement.columns.serviceName') }}:</strong> {{ selectedAsset.service_name }}</p>
              <p v-if="selectedAsset.service_version"><strong>{{ $t('assetManagement.columns.serviceVersion') }}:</strong> {{ selectedAsset.service_version }}</p>
              <p v-if="selectedAsset.transport_protocol"><strong>{{ $t('assetManagement.columns.transportProtocol') }}:</strong> {{ selectedAsset.transport_protocol }}</p>
            </div>
          </div>
          
          <!-- Web信息 -->
          <div v-if="selectedAsset.http_status || selectedAsset.title" class="col-span-2">
            <h4 class="font-semibold mb-2">{{ $t('assetManagement.columnGroups.web') }}</h4>
            <div class="bg-base-200 p-3 rounded space-y-1 text-sm">
              <p v-if="selectedAsset.http_status"><strong>{{ $t('assetManagement.columns.httpStatus') }}:</strong> {{ selectedAsset.http_status }}</p>
              <p v-if="selectedAsset.title"><strong>{{ $t('assetManagement.columns.title') }}:</strong> {{ selectedAsset.title }}</p>
              <p v-if="selectedAsset.cdn_detected"><strong>{{ $t('assetManagement.columns.cdn') }}:</strong> {{ selectedAsset.cdn_detected }}</p>
              <p v-if="selectedAsset.waf_detected"><strong>{{ $t('assetManagement.columns.waf') }}:</strong> {{ selectedAsset.waf_detected }}</p>
              <p v-if="selectedAsset.tech_stack && selectedAsset.tech_stack.length > 0">
                <strong>{{ $t('assetManagement.columns.techStack') }}:</strong> 
                <span v-for="(tech, idx) in selectedAsset.tech_stack" :key="idx" class="badge badge-ghost badge-xs ml-1">
                  {{ tech }}
                </span>
              </p>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showDetailModal = false" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted ,watch} from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

const props = defineProps<{
  selectedProgram?: { id: string; name: string } | null
  programs?: Array<{ id: string; name: string }>
}>();

const emit = defineEmits<{
  (e: 'stats-updated', stats: { total: number; active: number }): void
  (e: 'discover-assets'): void
}>();

interface Asset {
  id: string;
  name: string;
  value: string;
  asset_type: string;
  risk_level: string;
  status: string;
  last_seen?: string;
  
  // IP Asset fields
  ip_version?: string;
  asn?: number;
  asn_org?: string;
  isp?: string;
  country?: string;
  city?: string;
  latitude?: number;
  longitude?: number;
  is_cloud?: boolean;
  cloud_provider?: string;
  
  // Port/Service fields
  port?: number;
  service_name?: string;
  service_version?: string;
  service_product?: string;
  banner?: string;
  transport_protocol?: string;
  cpe?: string;
  
  // Domain fields
  domain_registrar?: string;
  registration_date?: string;
  expiration_date?: string;
  nameservers?: string[];
  is_wildcard?: boolean;
  parent_domain?: string;
  
  // Web/URL fields
  http_status?: number;
  response_time_ms?: number;
  content_length?: number;
  content_type?: string;
  title?: string;
  favicon_hash?: string;
  waf_detected?: string;
  cdn_detected?: string;
  screenshot_path?: string;
  body_hash?: string;
  tech_stack?: string[];
  
  // Certificate fields
  ssl_enabled?: boolean;
  certificate_subject?: string;
  certificate_issuer?: string;
  certificate_valid_from?: string;
  certificate_valid_to?: string;
  certificate_san?: string[];
  
  // Attack Surface & Risk
  exposure_level?: string;
  attack_surface_score?: number;
  vulnerability_count?: number;
  cvss_max_score?: number;
  exploit_available?: boolean;
  
  // Discovery
  discovery_method?: string;
  data_sources?: string[];
}

// Column definitions
const basicColumns = ['name', 'type', 'value', 'risk_level', 'status', 'last_seen', 'attack_surface_score', 'exposure_level', 'discovery_method'];
const ipColumns = ['ip_version', 'asn', 'country', 'city', 'cloud_provider'];
const portColumns = ['port', 'service_name', 'service_version', 'transport_protocol'];
const domainColumns = ['domain_registrar', 'expiration_date', 'parent_domain'];
const webColumns = ['http_status', 'title', 'cdn', 'waf', 'techStack'];
const certColumns = ['ssl', 'certIssuer', 'certExpiry'];

const loading = ref(false);
const assets = ref<Asset[]>([]);
const currentPage = ref(1);
const pageSize = ref(20);
const showDetailModal = ref(false);
const showColumnsModal = ref(false);
const selectedAsset = ref<Asset | null>(null);

// Filters
const filters = ref({
  search: '',
  programId: '',
  assetType: '',
  riskLevel: '',
  status: '',
  exposureLevel: '',
  country: '',
  cloudProvider: '',
  serviceName: '',
  cdnDetected: '',
  wafDetected: ''
});

// Filter options
const assetTypes = ref<string[]>([]);
const countries = ref<string[]>([]);
const cloudProviders = ref<string[]>([]);
const serviceNames = ref<string[]>([]);
const cdns = ref<string[]>([]);
const wafs = ref<string[]>([]);

// Column visibility - default visible columns
const visibleColumns = ref<Record<string, boolean>>({
  name: true,
  type: true,
  value: true,
  risk_level: true,
  status: true,
  last_seen: true,
  attack_surface_score: true,
  exposure_level: true,
  
  // IP columns
  ip_version: false,
  asn: false,
  country: false,
  cloud_provider: false,
  
  // Port columns
  port: false,
  service_name: false,
  service_version: false,
  transport_protocol: false,
  
  // Domain columns
  domain_registrar: false,
  expiration_date: false,
  parent_domain: false,
  
  // Web columns
  http_status: false,
  title: false,
  cdn: false,
  waf: false,
  techStack: false,
  
  // Certificate columns
  ssl: false,
  certIssuer: false,
  certExpiry: false,
  
  discovery_method: false
});

// Computed: Show type-specific columns based on selected asset type
const showIpColumns = computed(() => !filters.value.assetType || ['ip', 'host'].includes(filters.value.assetType.toLowerCase()));
const showPortColumns = computed(() => !filters.value.assetType || ['port', 'service'].includes(filters.value.assetType.toLowerCase()));
const showDomainColumns = computed(() => !filters.value.assetType || ['domain', 'subdomain'].includes(filters.value.assetType.toLowerCase()));
const showWebColumns = computed(() => !filters.value.assetType || ['url', 'web', 'http', 'https'].includes(filters.value.assetType.toLowerCase()));
const showCertColumns = computed(() => !filters.value.assetType || ['certificate', 'ssl'].includes(filters.value.assetType.toLowerCase()));

const filteredAssets = computed(() => {
  let filtered = assets.value;
  
  if (filters.value.search) {
    const query = filters.value.search.toLowerCase();
    filtered = filtered.filter(asset => 
      asset.name.toLowerCase().includes(query) ||
      asset.value.toLowerCase().includes(query) ||
      asset.service_name?.toLowerCase().includes(query) ||
      asset.title?.toLowerCase().includes(query)
    );
  }
  
  if (filters.value.assetType) {
    filtered = filtered.filter(asset => asset.asset_type === filters.value.assetType);
  }
  
  if (filters.value.riskLevel) {
    filtered = filtered.filter(asset => asset.risk_level === filters.value.riskLevel);
  }
  
  if (filters.value.status) {
    filtered = filtered.filter(asset => asset.status === filters.value.status);
  }
  
  if (filters.value.exposureLevel) {
    filtered = filtered.filter(asset => asset.exposure_level === filters.value.exposureLevel);
  }
  
  if (filters.value.country) {
    filtered = filtered.filter(asset => asset.country === filters.value.country);
  }
  
  if (filters.value.cloudProvider) {
    filtered = filtered.filter(asset => asset.cloud_provider === filters.value.cloudProvider);
  }
  
  if (filters.value.serviceName) {
    filtered = filtered.filter(asset => asset.service_name === filters.value.serviceName);
  }
  
  if (filters.value.cdnDetected) {
    filtered = filtered.filter(asset => asset.cdn_detected === filters.value.cdnDetected);
  }
  
  if (filters.value.wafDetected) {
    filtered = filtered.filter(asset => asset.waf_detected === filters.value.wafDetected);
  }
  
  return filtered;
});

const totalPages = computed(() => {
  return Math.ceil(filteredAssets.value.length / pageSize.value);
});

const paginatedAssets = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  const end = start + pageSize.value;
  return filteredAssets.value.slice(start, end);
});

const loadAssets = async () => {
  try {
    loading.value = true;
    
    // Use filter programId if set, otherwise use selectedProgram
    const programId = filters.value.programId || props.selectedProgram?.id || null;
    
    const result = await invoke<any[]>('bounty_list_assets', {
      filter: {
        program_id: programId,
        scope_id: null,
        asset_type: null,
        is_alive: null,
        has_findings: null,
        limit: null,
        offset: null
      }
    });
    
    // Map BountyAssetRow to Asset interface
    assets.value = result.map((row: any) => ({
      id: row.id,
      name: row.hostname || row.canonical_url,
      value: row.canonical_url,
      asset_type: row.asset_type,
      risk_level: row.risk_score > 0.7 ? 'High' : row.risk_score > 0.4 ? 'Medium' : 'Low',
      status: row.is_alive ? 'Active' : 'Inactive',
      last_seen: row.last_seen_at,
      
      // IP Asset fields
      ip_version: row.ip_version,
      asn: row.asn,
      asn_org: row.asn_org,
      isp: row.isp,
      country: row.country,
      city: row.city,
      latitude: row.latitude,
      longitude: row.longitude,
      is_cloud: row.is_cloud,
      cloud_provider: row.cloud_provider,
      
      // Port/Service fields
      port: row.port,
      service_name: row.service_name,
      service_version: row.service_version,
      service_product: row.service_product,
      banner: row.banner,
      transport_protocol: row.transport_protocol,
      cpe: row.cpe,
      
      // Domain fields
      domain_registrar: row.domain_registrar,
      registration_date: row.registration_date,
      expiration_date: row.expiration_date,
      nameservers: row.nameservers_json ? JSON.parse(row.nameservers_json) : null,
      is_wildcard: row.is_wildcard,
      parent_domain: row.parent_domain,
      
      // Web/URL fields
      http_status: row.http_status,
      response_time_ms: row.response_time_ms,
      content_length: row.content_length,
      content_type: row.content_type,
      title: row.title,
      favicon_hash: row.favicon_hash,
      waf_detected: row.waf_detected,
      cdn_detected: row.cdn_detected,
      screenshot_path: row.screenshot_path,
      body_hash: row.body_hash,
      tech_stack: row.tech_stack_json ? JSON.parse(row.tech_stack_json) : null,
      
      // Certificate fields
      ssl_enabled: row.ssl_enabled,
      certificate_subject: row.certificate_subject,
      certificate_issuer: row.certificate_issuer,
      certificate_valid_from: row.certificate_valid_from,
      certificate_valid_to: row.certificate_valid_to,
      certificate_san: row.certificate_san_json ? JSON.parse(row.certificate_san_json) : null,
      
      // Attack Surface & Risk
      exposure_level: row.exposure_level,
      attack_surface_score: row.attack_surface_score,
      vulnerability_count: row.vulnerability_count,
      cvss_max_score: row.cvss_max_score,
      exploit_available: row.exploit_available,
      
      // Discovery
      discovery_method: row.discovery_method,
      data_sources: row.data_sources_json ? JSON.parse(row.data_sources_json) : null,
    }));
    
    updateStats();
    extractFilterOptions();
    loadColumnSettings();
  } catch (error) {
    console.error('Failed to load assets:', error);
    assets.value = [];
  } finally {
    loading.value = false;
  }
};

const extractFilterOptions = () => {
  // Extract unique values for filter dropdowns
  assetTypes.value = Array.from(new Set(assets.value.map(a => a.asset_type).filter(Boolean)));
  countries.value = Array.from(new Set(assets.value.map(a => a.country).filter(Boolean)));
  cloudProviders.value = Array.from(new Set(assets.value.map(a => a.cloud_provider).filter(Boolean)));
  serviceNames.value = Array.from(new Set(assets.value.map(a => a.service_name).filter(Boolean)));
  cdns.value = Array.from(new Set(assets.value.map(a => a.cdn_detected).filter(Boolean)));
  wafs.value = Array.from(new Set(assets.value.map(a => a.waf_detected).filter(Boolean)));
};

const updateStats = () => {
  const stats = {
    total: assets.value.length,
    active: assets.value.filter(a => a.status === 'Active').length
  };
  emit('stats-updated', stats);
};

const refreshAssets = () => {
  loadAssets();
};

const applyFilters = () => {
  currentPage.value = 1;
};

const resetFilters = () => {
  filters.value = {
    search: '',
    programId: '',
    assetType: '',
    riskLevel: '',
    status: '',
    exposureLevel: '',
    country: '',
    cloudProvider: '',
    serviceName: '',
    cdnDetected: '',
    wafDetected: ''
  };
  applyFilters();
  loadAssets(); // Reload assets when program filter is reset
};

const onAssetTypeChange = () => {
  // When asset type changes, auto-show relevant columns
  const type = filters.value.assetType.toLowerCase();
  
  if (type.includes('ip') || type.includes('host')) {
    visibleColumns.value.asn = true;
    visibleColumns.value.country = true;
    visibleColumns.value.cloud_provider = true;
  }
  
  if (type.includes('port') || type.includes('service')) {
    visibleColumns.value.port = true;
    visibleColumns.value.service_name = true;
    visibleColumns.value.service_version = true;
  }
  
  if (type.includes('domain') || type.includes('subdomain')) {
    visibleColumns.value.domain_registrar = true;
    visibleColumns.value.expiration_date = true;
  }
  
  if (type.includes('url') || type.includes('web') || type.includes('http')) {
    visibleColumns.value.http_status = true;
    visibleColumns.value.title = true;
    visibleColumns.value.cdn = true;
    visibleColumns.value.waf = true;
  }
  
  applyFilters();
};

const viewAssetDetail = (asset: Asset) => {
  selectedAsset.value = asset;
  showDetailModal.value = true;
};

const loadColumnSettings = () => {
  const saved = localStorage.getItem('asset-column-settings');
  if (saved) {
    try {
      const settings = JSON.parse(saved);
      Object.assign(visibleColumns.value, settings);
    } catch (e) {
      console.error('Failed to load column settings:', e);
    }
  }
};

const saveColumnSettings = () => {
  localStorage.setItem('asset-column-settings', JSON.stringify(visibleColumns.value));
  showColumnsModal.value = false;
};

const resetColumns = () => {
  visibleColumns.value = {
    name: true,
    type: true,
    value: true,
    risk_level: true,
    status: true,
    last_seen: true,
    attack_surface_score: true,
    exposure_level: true,
    
    ip_version: false,
    asn: false,
    country: false,
    cloud_provider: false,
    
    port: false,
    service_name: false,
    service_version: false,
    transport_protocol: false,
    
    domain_registrar: false,
    expiration_date: false,
    parent_domain: false,
    
    http_status: false,
    title: false,
    cdn: false,
    waf: false,
    techStack: false,
    
    ssl: false,
    certIssuer: false,
    certExpiry: false,
    
    discovery_method: false
  };
};

const getRiskLevelClass = (level: string) => {
  const classes: Record<string, string> = {
    High: 'badge-error',
    Medium: 'badge-warning',
    Low: 'badge-success',
    Unknown: 'badge-ghost'
  };
  return classes[level] || 'badge-ghost';
};

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    Active: 'badge-success',
    Inactive: 'badge-ghost',
    Archived: 'badge-neutral'
  };
  return classes[status] || 'badge-ghost';
};

const getAttackSurfaceScoreColor = (score: number) => {
  if (score >= 70) return 'bg-error';
  if (score >= 40) return 'bg-warning';
  return 'bg-success';
};

const getExposureLevelClass = (level: string) => {
  const classes: Record<string, string> = {
    internet: 'badge-error',
    intranet: 'badge-warning',
    private: 'badge-success',
    unknown: 'badge-ghost'
  };
  return classes[level] || 'badge-ghost';
};

const formatTime = (dateStr: string) => {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  const now = new Date();
  const diffInMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60));
  
  if (diffInMinutes < 60) return `${diffInMinutes}${t('common.minutesAgo')}`;
  if (diffInMinutes < 1440) return `${Math.floor(diffInMinutes / 60)}${t('common.hoursAgo')}`;
  return `${Math.floor(diffInMinutes / 1440)}${t('common.daysAgo')}`;
};

const formatDate = (dateStr?: string) => {
  if (!dateStr) return '-';
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString();
  } catch {
    return '-';
  }
};

const isExpiringSoon = (dateStr?: string) => {
  if (!dateStr) return false;
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const daysUntilExpiry = Math.floor((date.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    return daysUntilExpiry < 30 && daysUntilExpiry > 0;
  } catch {
    return false;
  }
};

const truncate = (text: string | undefined, length: number) => {
  if (!text) return '-';
  return text.length > length ? text.substring(0, length) + '...' : text;
};

const getAssetIcon = (type: string) => {
  const icons: Record<string, string> = {
    ip: 'fas fa-network-wired',
    host: 'fas fa-server',
    domain: 'fas fa-globe',
    subdomain: 'fas fa-sitemap',
    url: 'fas fa-link',
    port: 'fas fa-plug',
    service: 'fas fa-cogs',
    certificate: 'fas fa-certificate'
  };
  return icons[type.toLowerCase()] || 'fas fa-cube';
};

const getHttpStatusClass = (status?: number) => {
  if (!status) return 'badge-ghost';
  if (status >= 200 && status < 300) return 'badge-success';
  if (status >= 300 && status < 400) return 'badge-info';
  if (status >= 400 && status < 500) return 'badge-warning';
  if (status >= 500) return 'badge-error';
  return 'badge-ghost';
};

const handleRefresh = () => {
  refreshAssets();
};

// Watch for program changes
watch(() => props.selectedProgram?.id, () => {
  loadAssets();
});

// Watch for filter programId changes
watch(() => filters.value.programId, () => {
  loadAssets();
});

onMounted(() => {
  loadAssets();
  window.addEventListener('security-center-refresh', handleRefresh);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
});

// Expose methods to parent component
defineExpose({
  refreshAssets,
  loadAssets
});
</script>
