<template>
  <div class="overflow-x-auto mb-6">
    <table class="table table-zebra">
      <thead>
        <tr>
          <th>名称</th>
          <th>类型</th>
          <th>服务</th>
          <th class="text-right">词条数</th>
          <th>状态</th>
          <th>更新时间</th>
          <th class="text-right">操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="dictionary in dictionaries" :key="dictionary.id">
          <td class="min-w-[16rem]">
            <div class="flex flex-col gap-1">
              <div class="font-semibold">{{ dictionary.name }}</div>
              <div class="text-xs font-mono text-base-content/60">{{ dictionary.dict_type }}</div>
              <div v-if="dictionary.description" class="text-xs text-base-content/70 line-clamp-2">
                {{ dictionary.description }}
              </div>
            </div>
          </td>
          <td class="min-w-[14rem]">
            <div class="flex flex-wrap gap-2">
              <div class="badge badge-primary">{{ getDictionaryTypeLabel(dictionary.dict_type) }}</div>
              <div
                v-if="getDictionarySubtypeLabel(dictionary)"
                class="badge"
                :class="getDictionarySubtypeBadgeClass(dictionary)"
              >
                {{ getDictionarySubtypeLabel(dictionary) }}
              </div>
            </div>
          </td>
          <td>
            <span v-if="dictionary.service_type">{{ getServiceTypeLabel(dictionary.service_type) }}</span>
            <span v-else class="text-base-content/50">-</span>
          </td>
          <td class="text-right font-medium">{{ dictionary.word_count || 0 }}</td>
          <td>
            <div class="flex flex-wrap gap-2">
              <div v-if="dictionary.is_builtin" class="badge badge-accent">内置</div>
              <div v-if="defaultMap[dictionary.dict_type] === dictionary.id" class="badge badge-success">默认</div>
            </div>
          </td>
          <td class="whitespace-nowrap">{{ formatDate(dictionary.updated_at) }}</td>
          <td>
            <div class="flex justify-end items-center gap-2">
              <button class="btn btn-ghost btn-sm" @click="emit('view-words', dictionary)">
                查看词条
              </button>
              <button class="btn btn-primary btn-sm" @click="emit('manage-words', dictionary)">
                管理
              </button>
              <button
                class="btn btn-ghost btn-sm"
                @click.stop="toggleMenu($event, dictionary)"
              >
                <i class="fas fa-ellipsis-v"></i>
              </button>
            </div>
          </td>
        </tr>
        <tr v-if="dictionaries.length === 0">
          <td colspan="7" class="text-center py-10 text-base-content/60">暂无字典</td>
        </tr>
      </tbody>
    </table>
  </div>

  <Teleport to="body">
    <div
      v-if="menuDictionary"
      class="fixed inset-0 z-[99998]"
      @click="closeMenu"
    >
      <div
        ref="menuRef"
        class="fixed z-[99999] w-52 rounded-box border border-base-300 bg-base-100 p-2 shadow-2xl"
        :style="menuStyle"
        @click.stop
      >
        <ul class="menu p-0">
          <li>
            <a @click="handleMenuAction('edit')"><i class="fas fa-edit mr-2"></i>编辑</a>
          </li>
          <li>
            <a @click="handleMenuAction('export')"><i class="fas fa-download mr-2"></i>导出</a>
          </li>
          <li>
            <a @click="handleMenuAction('duplicate')"><i class="fas fa-copy mr-2"></i>复制</a>
          </li>
          <li>
            <a @click="handleMenuAction('mark-default')"><i class="fas fa-star mr-2"></i>设为默认</a>
          </li>
          <li v-if="menuDictionary && defaultMap[menuDictionary.dict_type] === menuDictionary.id">
            <a @click="handleMenuAction('clear-default')"><i class="fas fa-ban mr-2"></i>取消默认</a>
          </li>
          <li v-if="menuDictionary && !menuDictionary.is_builtin">
            <a class="text-error" @click="handleMenuAction('delete')"><i class="fas fa-trash mr-2"></i>删除</a>
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

interface DictionaryListItem {
  id: string
  name: string
  description?: string
  dict_type: string
  service_type?: string
  is_builtin: boolean
  word_count?: number
  updated_at: string
  category?: string
  tags?: string[] | string
}

interface MenuAnchorRect {
  left: number
  top: number
  right: number
  bottom: number
}

const props = defineProps<{
  dictionaries: DictionaryListItem[]
  defaultMap: Record<string, string>
  getDictionaryTypeLabel: (type: string) => string
  getDictionarySubtypeLabel: (dictionary: DictionaryListItem) => string | null
  getDictionarySubtypeBadgeClass: (dictionary: DictionaryListItem) => string
  getServiceTypeLabel: (type: string) => string
  formatDate: (dateString: string) => string
}>()

const emit = defineEmits<{
  (e: 'edit', dictionary: DictionaryListItem): void
  (e: 'export', dictionary: DictionaryListItem): void
  (e: 'duplicate', dictionary: DictionaryListItem): void
  (e: 'mark-default', dictionary: DictionaryListItem): void
  (e: 'clear-default', dictionary: DictionaryListItem): void
  (e: 'delete', dictionary: DictionaryListItem): void
  (e: 'view-words', dictionary: DictionaryListItem): void
  (e: 'manage-words', dictionary: DictionaryListItem): void
}>()

const MENU_WIDTH = 208
const VIEWPORT_PADDING = 12

const menuRef = ref<HTMLElement | null>(null)
const menuDictionary = ref<DictionaryListItem | null>(null)
const menuAnchorRect = ref<MenuAnchorRect | null>(null)
const menuPosition = ref({ left: 0, top: 0 })

const menuStyle = computed(() => ({
  left: `${menuPosition.value.left}px`,
  top: `${menuPosition.value.top}px`,
}))

const syncMenuPosition = () => {
  if (!menuAnchorRect.value) return

  const viewportWidth = window.innerWidth
  const viewportHeight = window.innerHeight
  const menuWidth = menuRef.value?.offsetWidth || MENU_WIDTH
  const menuHeight = menuRef.value?.offsetHeight || 0

  let left = menuAnchorRect.value.right - menuWidth
  let top = menuAnchorRect.value.bottom + 8

  if (left + menuWidth > viewportWidth - VIEWPORT_PADDING) {
    left = viewportWidth - menuWidth - VIEWPORT_PADDING
  }
  if (left < VIEWPORT_PADDING) {
    left = VIEWPORT_PADDING
  }
  if (top + menuHeight > viewportHeight - VIEWPORT_PADDING) {
    top = Math.max(VIEWPORT_PADDING, menuAnchorRect.value.top - menuHeight - 8)
  }

  menuPosition.value = { left, top }
}

const closeMenu = () => {
  menuDictionary.value = null
  menuAnchorRect.value = null
}

const handleViewportChange = () => {
  if (!menuDictionary.value) return
  closeMenu()
}

const toggleMenu = async (event: MouseEvent, dictionary: DictionaryListItem) => {
  const target = event.currentTarget as HTMLElement | null
  if (!target) return

  if (menuDictionary.value?.id === dictionary.id) {
    closeMenu()
    return
  }

  const rect = target.getBoundingClientRect()
  menuDictionary.value = dictionary
  menuAnchorRect.value = {
    left: rect.left,
    top: rect.top,
    right: rect.right,
    bottom: rect.bottom,
  }
  menuPosition.value = {
    left: Math.max(VIEWPORT_PADDING, rect.right - MENU_WIDTH),
    top: rect.bottom + 8,
  }

  await nextTick()
  syncMenuPosition()
}

const handleMenuAction = (
  action: 'edit' | 'export' | 'duplicate' | 'mark-default' | 'clear-default' | 'delete',
) => {
  if (!menuDictionary.value) return
  const dictionary = menuDictionary.value
  closeMenu()
  emit(action, dictionary)
}

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    closeMenu()
  }
}

onMounted(() => {
  if (typeof window === 'undefined') return
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('scroll', handleViewportChange, true)
  window.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  if (typeof window === 'undefined') return
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  window.removeEventListener('keydown', handleKeydown)
})
</script>
