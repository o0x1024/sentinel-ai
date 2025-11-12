<template>
    <div class="page-content-padded space-y-6">
        <!-- 页面标题 -->
        <!-- <div class="flex items-center justify-between">
            <h1 class="text-2xl font-bold">
                <i class="fas fa-shield-alt mr-3"></i>
                被动扫描
            </h1>
            <div class="text-sm text-gray-500">
                通过 MITM 代理拦截流量，使用插件自动检测漏洞
            </div>
        </div> -->

        <!-- Tab 切换 -->
        <div class="tabs tabs-boxed bg-base-200" role="tablist" aria-label="Passive scan tabs">
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'control'"
                :class="{ 'tab-active': activeTab === 'control' }" @click="activeTab = 'control'">
                <i class="fas fa-sliders-h mr-2"></i>
                代理控制
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'proxyhistory'"
                :class="{ 'tab-active': activeTab === 'proxyhistory' }" @click="activeTab = 'proxyhistory'">
                <i class="fas fa-bug mr-2"></i>
                历史记录
            </button>
            <button type="button" class="tab" role="tab" :aria-selected="activeTab === 'proxyconfig'"
                :class="{ 'tab-active': activeTab === 'proxyconfig' }" @click="activeTab = 'proxyconfig'">
                <i class="fas fa-cog mr-2"></i>
                代理配置
            </button>
        </div>


        <PassiveScanControl v-if="activeTab === 'control'" />
        <ProxyHistory v-if="activeTab === 'proxyhistory'" />
        <ProxyConfiguration v-if="activeTab === 'proxyconfig'" />
    </div>

</template>

<script setup lang="ts">
import { ref, onMounted, onErrorCaptured, watch } from 'vue'
import PassiveScanControl from '../components/ProxyIntercept.vue'
import ProxyHistory from '../components/ProxyHistory.vue'
import ProxyConfiguration from '../components/ProxyConfiguration.vue'

const activeTab = ref<'control' | 'proxyhistory' | 'proxyconfig'>('control')
const isDevelopment = ref(import.meta.env.DEV)
const componentError = ref<string | null>(null)

onMounted(() => {
    console.log('PassiveScan view mounted, activeTab:', activeTab.value)
})

// 捕获子组件错误
onErrorCaptured((err, instance, info) => {
    console.error('Component error caught:', err, info)
    const message = err instanceof Error ? err.message : String(err)
    componentError.value = `组件加载失败: ${message}`
    return false // 阻止错误继续传播
})

// 调试：监听 tab 切换
watch(activeTab, (v) => {
    console.log('[PassiveScan] activeTab ->', v)
})
</script>

<style scoped>
.page-content-padded {
    padding: 1.5rem;
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
