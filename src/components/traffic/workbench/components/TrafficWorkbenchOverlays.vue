<template>
  <Transition name="intercept-drawer-backdrop">
    <div
      v-show="interceptDrawerOpen"
      class="intercept-drawer-backdrop absolute inset-0 z-30"
      aria-hidden="true"
      @click="$emit('closeInterceptDrawer')"
    ></div>
  </Transition>

  <Transition name="intercept-right-drawer">
    <section
      v-show="interceptDrawerOpen"
      class="intercept-drawer workbench-solid-surface absolute bottom-0 right-0 top-0 z-40 flex h-full w-full flex-col overflow-hidden rounded-l-[28px] border border-warning/20 bg-base-100 shadow-[0_28px_72px_rgba(15,23,42,0.18)]"
      :style="interceptDrawerStyle"
    >
      <div
        class="drawer-width-resizer drawer-width-resizer-warning absolute bottom-0 left-0 top-0 z-[2]"
        @mousedown="startDrawerWidthResize('intercept', $event)"
      ></div>
      <div class="border-b border-warning/10 px-4 py-3">
        <div class="flex items-start justify-between gap-3">
          <div>
            <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-warning">
              Intercept Queue
            </p>
            <h3 class="mt-1 text-lg font-semibold text-base-content">
              {{ controlTitle }}
            </h3>
          </div>
          <div class="flex items-center gap-2">
            <button type="button" class="btn btn-xs btn-outline rounded-2xl" @click="$emit('openProxySettings')">
              <i class="fas fa-cog mr-1"></i>
              代理设置
            </button>
            <button type="button" class="btn btn-sm btn-ghost rounded-2xl" @click="$emit('closeInterceptDrawer')">
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>
      </div>

      <TrafficControl
        class="min-h-0 flex-1 overflow-auto"
        @openResponseInterceptionSettings="$emit('openResponseInterceptionSettings')"
        @interceptQueueChanged="$emit('interceptQueueChanged', $event)"
        @createDraft="$emit('createDraftFromIntercept', $event)"
        @openDraftCompare="$emit('openDraftCompareFromIntercept', $event)"
        @createAttackWorkspace="$emit('createAttackWorkspaceFromIntercept', $event)"
        @addToBasket="$emit('addToBasketFromIntercept', $event)"
      />
    </section>
  </Transition>

    <AppModal
      :open="proxySettingsOpen"
      box-class="traffic-proxy-config-modal-box"
      resizable
      resize-storage-key="traffic-proxy-config-modal-size"
      :min-width="1040"
      :min-height="720"
      @close="$emit('closeProxySettings')"
    >
      <div class="flex h-full min-h-0 flex-col overflow-hidden">
        <div class="border-b border-base-300/70 px-6 py-4">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
                Proxy Settings
              </p>
              <h3 class="mt-1 text-xl font-semibold text-base-content">
                代理设置
              </h3>
            </div>
            <button
              type="button"
              class="btn btn-sm btn-ghost rounded-2xl"
              @click="$emit('closeProxySettings')"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>

        <div class="min-h-0 flex-1 bg-base-200/35 p-4">
          <ProxyConfiguration
            v-if="proxySettingsOpen"
            ref="proxyConfigRef"
            class="h-full min-h-0"
            @filterRuleAdded="$emit('filterRuleAdded', $event)"
          />
        </div>
      </div>
    </AppModal>

    <AppModal
      :open="trafficPluginsOpen"
      box-class="traffic-plugin-modal-box"
      resizable
      resize-storage-key="traffic-plugin-modal-size-v3"
      :min-width="1008"
      :min-height="640"
      :default-width="1120"
      :default-height="900"
      @close="$emit('closeTrafficPlugins')"
    >
      <div class="flex h-full min-h-0 flex-col overflow-hidden">
        <div class="border-b border-base-300/70 px-6 py-4">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
                Traffic Plugins
              </p>
              <h3 class="mt-1 text-xl font-semibold text-base-content">
                {{ pluginsTitle }}
              </h3>
            </div>
            <button
              type="button"
              class="btn btn-sm btn-ghost rounded-2xl"
              @click="$emit('closeTrafficPlugins')"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>

        <div class="min-h-0 flex-1 bg-base-200/35 p-4">
          <ImmersiveTrafficPluginPanel
            v-if="trafficPluginsOpen"
            class="h-full min-h-0"
          />
        </div>
      </div>
    </AppModal>

    <TrafficWorkbenchBasketDrawer
      :open="basketOpen"
      :items="basketItems"
      @close="$emit('closeBasket')"
      @remove="$emit('removeBasketItem', $event)"
      @clear="$emit('clearBasket')"
      @createDraft="$emit('createDraftFromBasketItem', $event)"
      @createAttackWorkspace="$emit('createAttackWorkspaceFromBasketItem', $event)"
      @createDraftsForAll="$emit('createDraftsForAllBasketItems')"
      @createAttackWorkspacesForAll="$emit('createAttackWorkspacesForAllBasketItems')"
      @openHistoryRequest="$emit('openHistoryRequestFromBasket', $event)"
    />

    <TrafficActiveProbePanel
      :shell-style="activeProbeShellStyle"
      :pending-entries="pendingActiveProbeEntries"
      :running-entries="runningActiveProbeEntries"
      :recent-entries="recentActiveProbeEntries"
      :queued-count="activeProbeQueuedCount"
      :collapsed="activeProbeCollapsed"
      :preview-request-map="activeProbePreviewRequests"
      :loading-traffic-request-id="activeProbePreviewLoadingId"
      @toggle-collapsed="$emit('toggleActiveProbeCollapsed')"
      @ensure-preview="$emit('ensureActiveProbePreview', $event)"
      @open-history="$emit('openHistoryRequestById', $event)"
      @open-history-by-traffic-request-id="$emit('openHistoryRequestByTrafficRequestId', $event)"
    />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import AppModal from '@/components/AppModal.vue'
import ImmersiveTrafficPluginPanel from '../../ImmersiveTrafficPluginPanel.vue'
import TrafficActiveProbePanel from '../../TrafficActiveProbePanel.vue'
import TrafficWorkbenchBasketDrawer from '../../TrafficWorkbenchBasketDrawer.vue'
import TrafficControl from '../../ProxyIntercept.vue'
import ProxyConfiguration from '../../ProxyConfiguration.vue'
import type { HttpExchangeRequest } from '../../http/model'
import type { ProxyRequest } from '../../proxyHistoryTypes'
import type { ActiveProbeQueueEntry } from '../../trafficActiveProbeTypes'
import type { TrafficComparerDraftRequestInput, TrafficComparePayload } from '../../transfers'
import type { TrafficWorkbenchBasketCandidateInput, TrafficWorkbenchBasketItem } from '../../trafficWorkbenchTypes'

defineProps<{
  interceptDrawerOpen: boolean
  interceptDrawerStyle: Record<string, string>
  proxySettingsOpen: boolean
  trafficPluginsOpen: boolean
  basketOpen: boolean
  basketItems: TrafficWorkbenchBasketItem[]
  controlTitle: string
  pluginsTitle: string
  activeProbeShellStyle: Record<string, string>
  pendingActiveProbeEntries: ActiveProbeQueueEntry[]
  runningActiveProbeEntries: ActiveProbeQueueEntry[]
  recentActiveProbeEntries: ActiveProbeQueueEntry[]
  activeProbeQueuedCount: number
  activeProbeCollapsed: boolean
  activeProbePreviewRequests: Record<string, ProxyRequest | undefined>
  activeProbePreviewLoadingId: string | null
  startDrawerWidthResize: (drawer: 'intercept', event: MouseEvent) => void
}>()

defineEmits<{
  (e: 'openProxySettings'): void
  (e: 'closeInterceptDrawer'): void
  (e: 'openResponseInterceptionSettings'): void
  (e: 'interceptQueueChanged', count: number): void
  (e: 'createDraftFromIntercept', request: HttpExchangeRequest): void
  (e: 'openDraftCompareFromIntercept', payload: TrafficComparerDraftRequestInput): void
  (e: 'createAttackWorkspaceFromIntercept', request: HttpExchangeRequest): void
  (e: 'addToBasketFromIntercept', payload: TrafficWorkbenchBasketCandidateInput): void
  (e: 'closeProxySettings'): void
  (e: 'filterRuleAdded', payload: { matchType: string; condition: string; relationship?: string }): void
  (e: 'closeTrafficPlugins'): void
  (e: 'closeBasket'): void
  (e: 'removeBasketItem', id: string): void
  (e: 'clearBasket'): void
  (e: 'createDraftFromBasketItem', id: string): void
  (e: 'createAttackWorkspaceFromBasketItem', id: string): void
  (e: 'createDraftsForAllBasketItems'): void
  (e: 'createAttackWorkspacesForAllBasketItems'): void
  (e: 'openHistoryRequestFromBasket', requestId: number): void
  (e: 'toggleActiveProbeCollapsed'): void
  (e: 'ensureActiveProbePreview', trafficRequestId: string): void
  (e: 'openHistoryRequestById', requestId: number): void
  (e: 'openHistoryRequestByTrafficRequestId', trafficRequestId: string): void
}>()

const proxyConfigRef = ref<InstanceType<typeof ProxyConfiguration> | null>(null)

function addRequestFilterRule(matchType: string, condition: string, relationship: string) {
  proxyConfigRef.value?.addRequestFilterRule(matchType, condition, relationship)
}

async function openResponseInterceptionRules() {
  await proxyConfigRef.value?.openResponseInterceptionRules?.()
}

defineExpose({
  addRequestFilterRule,
  openResponseInterceptionRules,
})
</script>

<style scoped>
.intercept-right-drawer-enter-active,
.intercept-right-drawer-leave-active {
  transition:
    transform 180ms ease,
    opacity 180ms ease;
}

.intercept-right-drawer-enter-from,
.intercept-right-drawer-leave-to {
  opacity: 0;
  transform: translateX(100%);
}

.intercept-right-drawer-enter-to,
.intercept-right-drawer-leave-from {
  opacity: 1;
  transform: translateX(0);
}

.intercept-drawer-backdrop {
  background: transparent;
}

.intercept-drawer-backdrop-enter-active,
.intercept-drawer-backdrop-leave-active {
  transition: opacity 180ms ease;
}

.intercept-drawer-backdrop-enter-from,
.intercept-drawer-backdrop-leave-to {
  opacity: 0;
}

.intercept-drawer-backdrop-enter-to,
.intercept-drawer-backdrop-leave-from {
  opacity: 1;
}

.drawer-width-resizer {
  width: 10px;
  cursor: col-resize;
  background: linear-gradient(180deg, transparent 0%, rgb(148 163 184 / 0.12) 50%, transparent 100%);
  opacity: 0.4;
  transition:
    background-color 160ms ease,
    opacity 160ms ease;
}

.drawer-width-resizer:hover {
  opacity: 1;
  background: linear-gradient(180deg, transparent 0%, rgb(59 130 246 / 0.3) 50%, transparent 100%);
}

.drawer-width-resizer-warning:hover {
  background: linear-gradient(180deg, transparent 0%, rgb(245 158 11 / 0.28) 50%, transparent 100%);
}

@media (max-width: 1024px) {
  .intercept-drawer {
    border-radius: 0;
  }
}
</style>
