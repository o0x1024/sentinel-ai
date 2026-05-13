<template>
  <div class="space-y-4">
    <div class="card bg-base-100 shadow-md">
      <div class="card-body p-4 pt-3">
        <div class="flex flex-wrap justify-between items-center gap-3 mb-4">
          <h2 class="card-title">{{ t('bugBounty.programs.title') }}</h2>
          <div class="flex flex-wrap items-center gap-2">
            <input 
              v-model="searchQuery" 
              type="text" 
              class="input input-sm input-bordered w-64"
              :placeholder="t('bugBounty.search')"
            />
            <div class="join">
              <button
                type="button"
                class="btn btn-sm join-item"
                :class="viewMode === 'card' ? 'btn-primary' : 'btn-ghost'"
                :title="t('bugBounty.programs.cardView')"
                :aria-label="t('bugBounty.programs.cardView')"
                :aria-pressed="viewMode === 'card'"
                @click="viewMode = 'card'"
              >
                <i class="fas fa-grip"></i>
              </button>
              <button
                type="button"
                class="btn btn-sm join-item"
                :class="viewMode === 'list' ? 'btn-primary' : 'btn-ghost'"
                :title="t('bugBounty.programs.listView')"
                :aria-label="t('bugBounty.programs.listView')"
                :aria-pressed="viewMode === 'list'"
                @click="viewMode = 'list'"
              >
                <i class="fas fa-list"></i>
              </button>
            </div>
            <button class="btn btn-sm btn-primary" @click="$emit('create')">
              <i class="fas fa-plus mr-2"></i>
              {{ t('bugBounty.createProgram') }}
            </button>
          </div>
        </div>
        
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="filteredPrograms.length === 0" class="text-center py-8">
          <i class="fas fa-trophy text-4xl text-base-content/30 mb-4"></i>
          <p class="text-base-content/70">{{ t('bugBounty.programs.empty') }}</p>
          <button class="btn btn-primary btn-sm mt-4" @click="$emit('create')">
            {{ t('bugBounty.createFirstProgram') }}
          </button>
        </div>
        
        <template v-else>
          <div v-if="viewMode === 'card'" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div
              v-for="program in filteredPrograms"
              :key="program.id"
              class="card bg-base-200 hover:bg-base-300 cursor-pointer transition-colors"
              @click="$emit('select', program)"
            >
              <div class="card-body">
                <div class="flex justify-between items-start">
                  <h3 class="card-title text-base">{{ program.name }}</h3>
                  <div class="dropdown dropdown-end">
                    <label tabindex="0" class="btn btn-ghost btn-xs" @click.stop>
                      <i class="fas fa-ellipsis-v"></i>
                    </label>
                    <ul tabindex="0" class="dropdown-content z-10 menu p-2 shadow bg-base-100 rounded-box w-40">
                      <li><a @click.stop="$emit('export-assets', program)"><i class="fas fa-download mr-2"></i>{{ t('bugBounty.programs.exportAssets') }}</a></li>
                      <li><a @click.stop="$emit('edit', program)"><i class="fas fa-edit mr-2"></i>{{ t('common.edit') }}</a></li>
                      <li><a @click.stop="$emit('delete', program)" class="text-error"><i class="fas fa-trash mr-2"></i>{{ t('common.delete') }}</a></li>
                    </ul>
                  </div>
                </div>
                <p class="text-sm text-base-content/70">{{ program.organization }}</p>
                <div class="flex flex-wrap gap-2 mt-2">
                  <span class="badge badge-sm badge-primary">{{ program.platform }}</span>
                  <span class="badge badge-sm" :class="getStatusClass(program.status)">
                    {{ program.status }}
                  </span>
                </div>
                <div class="stats stats-vertical shadow mt-2 text-xs">
                  <div class="stat py-2">
                    <div class="stat-title text-xs">{{ t('bugBounty.programs.submissions') }}</div>
                    <div class="stat-value text-sm">{{ program.total_submissions }}</div>
                  </div>
                  <div class="stat py-2">
                    <div class="stat-title text-xs">{{ t('bugBounty.programs.earnings') }}</div>
                    <div class="stat-value text-sm">${{ program.total_earnings.toFixed(0) }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div v-else class="overflow-x-auto rounded-lg border border-base-300">
            <table class="table table-zebra table-sm">
              <thead>
                <tr>
                  <th>{{ t('common.name') }}</th>
                  <th>{{ t('bugBounty.programs.organization') }}</th>
                  <th>{{ t('bugBounty.programs.platform') }}</th>
                  <th>{{ t('common.status') }}</th>
                  <th class="text-right">{{ t('bugBounty.programs.submissions') }}</th>
                  <th class="text-right">{{ t('bugBounty.programs.earnings') }}</th>
                  <th class="w-12 text-right">{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="program in filteredPrograms"
                  :key="program.id"
                  class="cursor-pointer"
                  @click="$emit('select', program)"
                >
                  <td class="font-medium">{{ program.name }}</td>
                  <td>{{ program.organization }}</td>
                  <td>
                    <span class="badge badge-sm badge-primary">{{ program.platform }}</span>
                  </td>
                  <td>
                    <span class="badge badge-sm" :class="getStatusClass(program.status)">
                      {{ program.status }}
                    </span>
                  </td>
                  <td class="text-right">{{ program.total_submissions }}</td>
                  <td class="text-right">${{ program.total_earnings.toFixed(0) }}</td>
                  <td class="text-right">
                    <div class="dropdown dropdown-end">
                      <label tabindex="0" class="btn btn-ghost btn-xs" @click.stop>
                        <i class="fas fa-ellipsis-v"></i>
                      </label>
                      <ul tabindex="0" class="dropdown-content z-20 menu p-2 shadow bg-base-100 rounded-box w-40">
                        <li><a @click.stop="$emit('export-assets', program)"><i class="fas fa-download mr-2"></i>{{ t('bugBounty.programs.exportAssets') }}</a></li>
                        <li><a @click.stop="$emit('edit', program)"><i class="fas fa-edit mr-2"></i>{{ t('common.edit') }}</a></li>
                        <li><a @click.stop="$emit('delete', program)" class="text-error"><i class="fas fa-trash mr-2"></i>{{ t('common.delete') }}</a></li>
                      </ul>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  programs: any[]
  loading: boolean
}>()

defineEmits<{
  (e: 'create'): void
  (e: 'select', program: any): void
  (e: 'export-assets', program: any): void
  (e: 'edit', program: any): void
  (e: 'delete', program: any): void
}>()

const searchQuery = ref('')
const viewMode = ref<'card' | 'list'>('list')

const filteredPrograms = computed(() => {
  if (!searchQuery.value) return props.programs
  const search = searchQuery.value.toLowerCase()
  return props.programs.filter(p => 
    p.name.toLowerCase().includes(search) || 
    p.organization.toLowerCase().includes(search)
  )
})

const getStatusClass = (status: string) => {
  const classes: Record<string, string> = {
    active: 'badge-success',
    paused: 'badge-warning',
    ended: 'badge-neutral',
    archived: 'badge-ghost',
  }
  return classes[status.toLowerCase()] || 'badge-ghost'
}
</script>

<style scoped>
.stats-vertical {
  font-size: 0.75rem;
}
</style>
