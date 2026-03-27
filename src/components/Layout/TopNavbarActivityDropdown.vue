<template>
  <div class="dropdown dropdown-end">
    <div tabindex="0" role="button" class="btn btn-ghost btn-circle btn-sm sm:btn-md indicator">
      <i :class="iconClass" class="text-lg sm:text-xl"></i>
      <span v-if="unreadCount > 0" class="badge badge-xs badge-primary indicator-item">
        {{ unreadBadge }}
      </span>
    </div>

    <div tabindex="0" class="dropdown-content z-[60] card card-compact w-96 max-w-[calc(100vw-2rem)] p-2 shadow bg-base-100">
      <div class="card-body gap-3">
        <div class="flex items-center justify-between gap-3">
          <h3 class="card-title text-sm">{{ title }}</h3>
          <div class="flex items-center gap-2">
            <button
              v-if="items.length > 0"
              class="btn btn-ghost btn-xs"
              @click.stop="$emit('mark-all-read')"
            >
              {{ markAllLabel }}
            </button>
            <button
              v-if="items.length > 0"
              class="btn btn-ghost btn-xs text-error"
              @click.stop="$emit('clear-all')"
            >
              {{ clearAllLabel }}
            </button>
          </div>
        </div>

        <div class="space-y-2 max-h-80 overflow-y-auto">
          <div
            v-for="item in items"
            :key="item.id"
            role="button"
            tabindex="0"
            class="w-full text-left rounded-xl border border-base-300 bg-base-100 hover:bg-base-200 transition-colors"
            :class="{ 'opacity-70': item.read }"
            @click="$emit('open', item)"
          >
            <div class="p-3 flex items-start gap-3">
              <div class="pt-0.5">
                <i :class="item.icon" class="text-sm"></i>
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex items-start justify-between gap-2">
                  <div class="font-semibold text-xs leading-5 truncate">
                    {{ item.title }}
                  </div>
                  <span v-if="!item.read" class="badge badge-primary badge-xs">
                    {{ unreadLabel }}
                  </span>
                </div>
                <div class="text-xs text-base-content/70 mt-1 leading-5 break-words">
                  {{ item.message }}
                </div>
                <div class="text-[11px] text-base-content/50 mt-2">
                  {{ formatRelativeTime(item.createdAt) }}
                </div>
              </div>
              <button
                class="btn btn-ghost btn-xs text-base-content/50"
                @click.stop="$emit('remove', item.id)"
              >
                <i class="fas fa-xmark"></i>
              </button>
            </div>
          </div>

          <div v-if="items.length === 0" class="text-center text-sm opacity-70 py-6">
            {{ emptyText }}
          </div>
        </div>

        <button
          class="btn btn-ghost btn-sm w-full justify-between"
          @click.stop="$emit('view-all')"
        >
          <span>{{ viewAllLabel }}</span>
          <i class="fas fa-arrow-right text-xs"></i>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AppNotificationItem } from '@/types/notification'

const props = defineProps<{
  title: string
  iconClass: string
  emptyText: string
  items: AppNotificationItem[]
  unreadCount: number
}>()

defineEmits<{
  (e: 'open', item: AppNotificationItem): void
  (e: 'remove', id: string): void
  (e: 'mark-all-read'): void
  (e: 'clear-all'): void
  (e: 'view-all'): void
}>()

const { t } = useI18n()

const unreadBadge = computed(() => {
  return props.unreadCount > 99 ? '99+' : String(props.unreadCount)
})

const unreadLabel = computed(() => t('notifications.center.unread'))
const markAllLabel = computed(() => t('notifications.center.markAllRead'))
const clearAllLabel = computed(() => t('notifications.center.clearAll'))
const viewAllLabel = computed(() => t('notifications.center.viewAll'))

const formatRelativeTime = (value: string) => {
  const timestamp = new Date(value).getTime()
  const deltaMs = Date.now() - timestamp

  if (Number.isNaN(timestamp) || deltaMs < 60_000) {
    return t('notifications.center.justNow')
  }

  const minutes = Math.floor(deltaMs / 60_000)
  if (minutes < 60) {
    return t('notifications.center.minutesAgo', { count: minutes })
  }

  const hours = Math.floor(minutes / 60)
  if (hours < 24) {
    return t('notifications.center.hoursAgo', { count: hours })
  }

  const days = Math.floor(hours / 24)
  return t('notifications.center.daysAgo', { count: days })
}
</script>
