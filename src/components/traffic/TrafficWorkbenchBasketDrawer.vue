<template>
  <section
    v-show="open"
    class="basket-drawer rounded-[28px] border border-base-300/80 bg-base-100 shadow-[0_28px_72px_rgba(15,23,42,0.16)]"
  >
    <div class="flex items-center justify-between gap-3 border-b border-base-300/70 px-4 py-3">
      <div>
        <p class="text-[11px] font-semibold uppercase tracking-[0.22em] text-primary/80">
          Request Basket
        </p>
        <h3 class="mt-1 text-base font-semibold text-base-content">
          请求篮子
        </h3>
        <p class="mt-1 text-xs text-base-content/60">
          先收集高价值请求，再批量送入重放器或爆破器。
        </p>
      </div>
      <button type="button" class="btn btn-xs btn-ghost rounded-2xl" @click="$emit('close')">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <div v-if="items.length > 0" class="flex flex-wrap items-center gap-2 border-b border-base-300/70 px-4 py-3">
      <button type="button" class="btn btn-xs btn-primary" @click="$emit('sendAllToRepeater')">
        <i class="fas fa-redo mr-1"></i>
        全部发到重放器
      </button>
      <button type="button" class="btn btn-xs btn-outline" @click="$emit('sendAllToIntruder')">
        <i class="fas fa-crosshairs mr-1"></i>
        全部发到爆破器
      </button>
      <button type="button" class="btn btn-xs btn-ghost text-error" @click="$emit('clear')">
        <i class="fas fa-trash mr-1"></i>
        清空篮子
      </button>
    </div>

    <div v-if="items.length === 0" class="flex h-48 items-center justify-center px-6 text-center text-sm text-base-content/55">
      <div>
        <i class="fas fa-basket-shopping mb-3 text-3xl text-base-content/25"></i>
        <p>历史记录和拦截队列里的高价值请求，可以先放到这里再统一处理。</p>
      </div>
    </div>

    <div v-else class="max-h-[min(56vh,640px)] overflow-auto px-3 py-3">
      <div class="space-y-2">
        <article
          v-for="item in items"
          :key="item.id"
          class="rounded-2xl border border-base-300/70 bg-base-100 px-3 py-3"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="rounded-full bg-base-200 px-2 py-1 text-[11px] font-medium text-base-content/70">
                  {{ item.request.request.method }}
                </span>
                <span class="rounded-full bg-primary/10 px-2 py-1 text-[11px] font-medium text-primary">
                  {{ item.source.label }}
                </span>
                <span v-if="item.host" class="truncate text-xs text-base-content/55">
                  {{ item.host }}
                </span>
              </div>
              <p class="mt-2 truncate text-sm font-medium text-base-content">
                {{ item.name }}
              </p>
              <p class="mt-1 truncate font-mono text-[11px] text-base-content/50">
                {{ item.request.absoluteUrl }}
              </p>
            </div>

            <button
              type="button"
              class="btn btn-xs btn-ghost text-error"
              @click="$emit('remove', item.id)"
            >
              <i class="fas fa-trash"></i>
            </button>
          </div>

          <div class="mt-3 flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="btn btn-xs btn-primary"
              @click="$emit('sendToRepeater', item.id)"
            >
              <i class="fas fa-redo mr-1"></i>
              重放
            </button>
            <button
              type="button"
              class="btn btn-xs btn-outline"
              @click="$emit('sendToIntruder', item.id)"
            >
              <i class="fas fa-crosshairs mr-1"></i>
              爆破
            </button>
            <button
              v-if="item.requestId"
              type="button"
              class="btn btn-xs btn-ghost"
              @click="$emit('openHistoryRequest', item.requestId)"
            >
              <i class="fas fa-history mr-1"></i>
              查看源请求
            </button>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { TrafficWorkbenchBasketItem } from './trafficWorkbenchTypes'

defineProps<{
  open: boolean
  items: TrafficWorkbenchBasketItem[]
}>()

defineEmits<{
  close: []
  clear: []
  remove: [id: string]
  sendToRepeater: [id: string]
  sendToIntruder: [id: string]
  sendAllToRepeater: []
  sendAllToIntruder: []
  openHistoryRequest: [requestId: number]
}>()
</script>

<style scoped>
.basket-drawer {
  position: absolute;
  right: 1rem;
  bottom: 1rem;
  z-index: 35;
  width: min(28rem, calc(100vw - 2rem));
  display: flex;
  flex-direction: column;
}
</style>
