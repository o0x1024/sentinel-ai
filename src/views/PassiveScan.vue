<template>
    <div class="page-content-padded flex flex-col h-[calc(100vh-4rem)]">
        <!-- Tab 切换 -->
        <div class="tabs tabs-boxed bg-base-200 flex-shrink-0 mb-4" role="tablist" :aria-label="$t('passiveScan.ariaLabels.passiveScanTabs')">
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'control'"
                :class="{ 'tab-active': activeTab === 'control' }" @click="activeTab = 'control'">
                <i class="fas fa-sliders-h mr-2"></i>
                {{ $t('passiveScan.tabs.control') }}
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'proxyhistory'"
                :class="{ 'tab-active': activeTab === 'proxyhistory' }" @click="activeTab = 'proxyhistory'">
                <i class="fas fa-history mr-2"></i>
                {{ $t('passiveScan.tabs.history') }}
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'repeater'"
                :class="{ 'tab-active': activeTab === 'repeater' }" @click="activeTab = 'repeater'">
                <i class="fas fa-redo mr-2"></i>
                {{ $t('passiveScan.tabs.repeater') }}
                <span v-if="repeaterCount > 0" class="badge badge-xs badge-primary ml-1">{{ repeaterCount }}</span>
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'proxifier'"
                :class="{ 'tab-active': activeTab === 'proxifier' }" @click="activeTab = 'proxifier'">
                <i class="fas fa-network-wired mr-2"></i>
                {{ $t('passiveScan.tabs.proxifier') }}
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'capture'"
                :class="{ 'tab-active': activeTab === 'capture' }" @click="activeTab = 'capture'">
                <i class="fas fa-broadcast-tower mr-2"></i>
                {{ $t('passiveScan.tabs.capture') }}
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'proxyconfig'"
                :class="{ 'tab-active': activeTab === 'proxyconfig' }" @click="activeTab = 'proxyconfig'">
                <i class="fas fa-cog mr-2"></i>
                {{ $t('passiveScan.tabs.proxyConfig') }}
            </button>
        </div>

        <!-- 内容区域：使用 v-show 避免组件销毁重建，保留临时数据 -->
        <div class="flex-1 min-h-0 relative">
            <PassiveControl 
                v-show="activeTab === 'control'" 
                @sendToRepeater="handleSendToRepeater"
                class="h-full absolute inset-0 overflow-auto"
            />
            <ProxyHistory 
                v-show="activeTab === 'proxyhistory'" 
                @sendToRepeater="handleSendToRepeater"
                class="h-full absolute inset-0 overflow-auto"
            />
            <ProxyRepeater 
                v-show="activeTab === 'repeater'" 
                ref="repeaterRef"
                :initialRequest="pendingRepeaterRequest"
                class="h-full absolute inset-0 overflow-auto"
            />
            <ProxifierPanel 
                v-show="activeTab === 'proxifier'" 
                class="h-full absolute inset-0"
            />
            <PacketCapture 
                v-show="activeTab === 'capture'" 
                class="h-full absolute inset-0"
            />
            <ProxyConfiguration v-show="activeTab === 'proxyconfig'" class="h-full absolute inset-0 overflow-auto" />
        </div>
    </div>

</template>

<script setup lang="ts">
import { ref, onMounted, onActivated, onDeactivated, onErrorCaptured, watch, provide } from 'vue'
import PassiveControl from '../components/passive/ProxyIntercept.vue'
import ProxyHistory from '../components/passive/ProxyHistory.vue'
import ProxyRepeater from '../components/passive/ProxyRepeater.vue'
import ProxyConfiguration from '../components/passive/ProxyConfiguration.vue'
import ProxifierPanel from '../components/passive/ProxifierPanel.vue'
import PacketCapture from '../components/passive/PacketCapture.vue'

// Types
interface RepeaterRequest {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
}

const activeTab = ref<'control' | 'proxyhistory' | 'repeater' | 'proxifier' | 'capture' | 'proxyconfig'>('proxyhistory')
const isDevelopment = ref(import.meta.env.DEV)
const componentError = ref<string | null>(null)
const refreshTrigger = ref(0)
const repeaterRef = ref<InstanceType<typeof ProxyRepeater> | null>(null)
const pendingRepeaterRequest = ref<RepeaterRequest | undefined>(undefined)
const repeaterCount = ref(0)

defineOptions({
  name: 'Passive'
});


// 提供刷新触发器给子组件
provide('refreshTrigger', refreshTrigger)

// 处理发送到 Repeater 的请求
function handleSendToRepeater(request: RepeaterRequest) {
    console.log('[PassiveScan] Sending to repeater:', request)
    
    // 如果当前在 Repeater 页面，直接调用方法
    if (activeTab.value === 'repeater' && repeaterRef.value) {
        repeaterRef.value.addRequestFromHistory(request)
    } else {
        // 否则先保存请求，然后切换到 Repeater 页面
        pendingRepeaterRequest.value = request
        activeTab.value = 'repeater'
    }
    
    repeaterCount.value++
}

// 监听 Tab 切换，清除待处理请求
watch(activeTab, (newTab) => {
    console.log('[PassiveScan] activeTab ->', newTab)
    if (newTab !== 'repeater') {
        // 切换离开 Repeater 时清除待处理请求
        pendingRepeaterRequest.value = undefined
    }
})

onMounted(() => {
    console.log('passive view mounted, activeTab:', activeTab.value)
})

// 当组件从缓存中激活时，触发刷新
onActivated(() => {
    console.log('passive view activated, triggering refresh')
    refreshTrigger.value++
})

// 当组件被缓存时
onDeactivated(() => {
    console.log('passive view deactivated')
})

// 捕获子组件错误
onErrorCaptured((err, instance, info) => {
    console.error('Component error caught:', err, info)
    const message = err instanceof Error ? err.message : String(err)
    componentError.value = `组件加载失败: ${message}`
    return false // 阻止错误继续传播
})
</script>

<style scoped>
.page-content-padded {
    padding: 1rem 1.5rem;
}

.tabs {
    border-radius: 0.5rem;
    padding: 0.25rem;
}

.tab {
    border-radius: 0.375rem;
    font-weight: 500;
}

.tab-active {
    background-color: hsl(var(--p));
    color: hsl(var(--pc));
}
</style>
