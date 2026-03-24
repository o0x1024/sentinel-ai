<template>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
    <div
      v-for="dictionary in dictionaries"
      :key="dictionary.id"
      class="card bg-base-200 shadow-xl hover:shadow-2xl transition-shadow overflow-visible relative"
    >
      <div class="card-body">
        <div class="flex justify-between items-start mb-2">
          <h3 class="card-title text-lg">{{ dictionary.name }}</h3>
          <div class="dropdown dropdown-end z-50 relative">
            <label tabindex="0" class="btn btn-ghost btn-sm">
              <i class="fas fa-ellipsis-v"></i>
            </label>
            <ul tabindex="0" class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52 z-50">
              <li><a @click="emit('edit', dictionary)"><i class="fas fa-edit mr-2"></i>编辑</a></li>
              <li><a @click="emit('export', dictionary)"><i class="fas fa-download mr-2"></i>导出</a></li>
              <li><a @click="emit('duplicate', dictionary)"><i class="fas fa-copy mr-2"></i>复制</a></li>
              <li><a @click="emit('mark-default', dictionary)"><i class="fas fa-star mr-2"></i>设为默认</a></li>
              <li v-if="defaultMap[dictionary.dict_type] === dictionary.id">
                <a @click="emit('clear-default', dictionary)"><i class="fas fa-ban mr-2"></i>取消默认</a>
              </li>
              <li v-if="!dictionary.is_builtin">
                <a class="text-error" @click="emit('delete', dictionary)"><i class="fas fa-trash mr-2"></i>删除</a>
              </li>
            </ul>
          </div>
        </div>

        <p class="text-sm opacity-70 mb-3">{{ dictionary.description }}</p>

        <div class="flex flex-wrap gap-2 mb-3">
          <div class="badge badge-primary">{{ getDictionaryTypeLabel(dictionary.dict_type) }}</div>
          <div v-if="dictionary.service_type" class="badge badge-secondary">{{ getServiceTypeLabel(dictionary.service_type) }}</div>
          <div v-if="dictionary.is_builtin" class="badge badge-accent">内置</div>
          <div v-if="defaultMap[dictionary.dict_type] === dictionary.id" class="badge badge-success">默认</div>
        </div>

        <div class="stats stats-horizontal bg-base-100 rounded-lg">
          <div class="stat py-2">
            <div class="stat-title text-xs">词条数</div>
            <div class="stat-value text-lg">{{ dictionary.word_count || 0 }}</div>
          </div>
          <div class="stat py-2">
            <div class="stat-title text-xs">最后更新</div>
            <div class="stat-value text-xs">{{ formatDate(dictionary.updated_at) }}</div>
          </div>
        </div>

        <div class="card-actions justify-end mt-4">
          <button class="btn btn-sm btn-outline" @click="emit('view-words', dictionary)">
            <i class="fas fa-eye mr-1"></i>
            查看词条
          </button>
          <button class="btn btn-sm btn-primary" @click="emit('manage-words', dictionary)">
            <i class="fas fa-edit mr-1"></i>
            管理
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface DictionaryCard {
  id: string
  name: string
  description?: string
  dict_type: string
  service_type?: string
  is_builtin: boolean
  word_count?: number
  updated_at: string
}

defineProps<{
  dictionaries: DictionaryCard[]
  defaultMap: Record<string, string>
  getDictionaryTypeLabel: (type: string) => string
  getServiceTypeLabel: (type: string) => string
  formatDate: (dateString: string) => string
}>()

const emit = defineEmits<{
  (e: 'edit', dictionary: DictionaryCard): void
  (e: 'export', dictionary: DictionaryCard): void
  (e: 'duplicate', dictionary: DictionaryCard): void
  (e: 'mark-default', dictionary: DictionaryCard): void
  (e: 'clear-default', dictionary: DictionaryCard): void
  (e: 'delete', dictionary: DictionaryCard): void
  (e: 'view-words', dictionary: DictionaryCard): void
  (e: 'manage-words', dictionary: DictionaryCard): void
}>()
</script>
