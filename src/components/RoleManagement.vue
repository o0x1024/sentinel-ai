<template>
  <div class="modal modal-open">
    <div class="modal-box max-w-6xl w-11/12 h-[85vh] p-0 flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="p-4 border-b flex items-center justify-between bg-base-200/50">
        <h3 class="font-bold text-lg flex items-center gap-2">
          <i class="fas fa-user-tie text-primary"></i>
          {{ t('roles.roleManagement') }}
        </h3>
        <button @click="$emit('close')" class="btn btn-ghost btn-sm btn-circle">
          <i class="fas fa-times"></i>
        </button>
      </div>

      <div class="flex-1 flex overflow-hidden">
        <!-- Left Sidebar: Role List -->
        <div class="w-80 border-r flex flex-col bg-base-100">
          <div class="p-4 space-y-4 border-b">
            <div class="relative">
              <i class="fas fa-search absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40"></i>
              <input 
                v-model="searchQuery"
                type="text" 
                :placeholder="t('common.search')" 
                class="input input-bordered input-sm w-full pl-9"
              />
            </div>
            <button @click="startCreate" class="btn btn-primary btn-sm w-full gap-2">
              <i class="fas fa-plus"></i>
              {{ t('roles.newRole') }}
            </button>
          </div>

          <div class="flex-1 overflow-y-auto p-2 space-y-2">
            <div v-if="isLoading" class="flex justify-center py-8">
              <span class="loading loading-spinner loading-md"></span>
            </div>
            
            <template v-else>
              <div 
                v-for="role in filteredRoles" 
                :key="role.id"
                @click="editRole(role)"
                class="card bg-base-100 border transition-all cursor-pointer hover:bg-base-200"
                :class="editingRole?.id === role.id ? 'border-primary bg-primary/5 shadow-sm' : 'border-base-300'"
              >
                <div class="p-3">
                  <div class="flex items-start justify-between gap-2">
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1">
                        <span v-if="role.is_system" class="badge badge-ghost badge-xs shrink-0">System</span>
                        <h5 class="font-medium text-sm truncate">{{ role.title }}</h5>
                      </div>
                      <p class="text-xs text-base-content/60 line-clamp-1">{{ role.description }}</p>
                    </div>
                    <button 
                      v-if="!role.is_system"
                      @click.stop="confirmDeleteRole(role)" 
                      class="btn btn-ghost btn-xs text-error p-0 h-6 w-6 min-h-0 opacity-0 group-hover:opacity-100"
                    >
                      <i class="fas fa-trash text-[10px]"></i>
                    </button>
                  </div>
                </div>
              </div>

              <div v-if="filteredRoles.length === 0" class="text-center py-12 text-base-content/40">
                <i class="fas fa-folder-open text-4xl mb-3 opacity-20"></i>
                <p class="text-sm">{{ t('common.noMore') }}</p>
              </div>
            </template>
          </div>
        </div>

        <!-- Right Content: Editor & Preview -->
        <div class="flex-1 flex flex-col overflow-hidden bg-base-200/20">
          <div v-if="showCreateForm || editingRole" class="flex-1 flex flex-col overflow-hidden">
            <!-- Toolbar -->
            <div class="px-6 py-3 border-b bg-base-100 flex items-center justify-between shadow-sm z-10">
              <div class="flex items-center gap-2">
                <i class="fas" :class="editingRole ? 'fa-edit' : 'fa-plus'"></i>
                <h4 class="font-semibold text-sm">
                  {{ editingRole ? t('roles.editRole') : t('roles.newRole') }}
                </h4>
              </div>
              <div class="flex gap-2">
                <button 
                  @click="cancelForm"
                  class="btn btn-ghost btn-sm"
                >
                  {{ t('common.cancel') }}
                </button>
                <button 
                  @click="handleSubmit" 
                  class="btn btn-primary btn-sm gap-2"
                  :disabled="isSubmitting"
                >
                  <span v-if="isSubmitting" class="loading loading-spinner loading-xs"></span>
                  <i class="fas fa-save" v-else></i>
                  {{ t('common.save') }}
                </button>
              </div>
            </div>

            <!-- Content Area -->
            <div class="flex-1 overflow-y-auto p-6">
              <div class="grid grid-cols-1 xl:grid-cols-2 gap-8 h-full min-h-[500px]">
                <!-- Editor -->
                <div class="flex flex-col gap-6">
                  <div class="form-control">
                    <label class="label pt-0">
                      <span class="label-text font-bold text-base-content/70">{{ t('roles.roleTitle') }} <span class="text-error">*</span></span>
                    </label>
                    <input 
                      v-model="formData.title"
                      type="text" 
                      :placeholder="t('roles.roleTitle')"
                      class="input input-bordered w-full focus:input-primary transition-all"
                      :class="{ 'input-error': formErrors.title }"
                      required
                    />
                    <label v-if="formErrors.title" class="label">
                      <span class="label-text-alt text-error">{{ formErrors.title }}</span>
                    </label>
                  </div>
                  
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text font-bold text-base-content/70">{{ t('roles.roleDescription') }}</span>
                    </label>
                    <textarea 
                      v-model="formData.description"
                      :placeholder="t('roles.roleDescription')"
                      class="textarea textarea-bordered h-24 w-full focus:textarea-primary transition-all resize-none"
                      :class="{ 'textarea-error': formErrors.description }"
                    ></textarea>
                    <label v-if="formErrors.description" class="label">
                      <span class="label-text-alt text-error">{{ formErrors.description }}</span>
                    </label>
                  </div>
                  
                  <div class="form-control flex-1 flex flex-col min-h-[300px]">
                    <label class="label">
                      <span class="label-text font-bold text-base-content/70">{{ t('roles.rolePrompt') }} <span class="text-error">*</span></span>
                    </label>
                    <textarea 
                      v-model="formData.prompt"
                      :placeholder="t('roles.rolePrompt')"
                      class="textarea textarea-bordered flex-1 w-full font-mono text-sm leading-relaxed focus:textarea-primary transition-all"
                      :class="{ 'textarea-error': formErrors.prompt }"
                      required
                    ></textarea>
                    <label v-if="formErrors.prompt" class="label">
                      <span class="label-text-alt text-error">{{ formErrors.prompt }}</span>
                    </label>
                  </div>
                </div>

                <!-- Preview -->
                <div class="flex flex-col h-full">
                  <label class="label pt-0">
                    <span class="label-text font-bold text-base-content/40 uppercase text-xs tracking-widest">{{ t('common.actionsMap.preview') }}</span>
                  </label>
                  <div class="flex-1 rounded-2xl border border-base-300 bg-base-100 shadow-inner p-6 overflow-y-auto">
                    <div class="prose prose-sm max-w-none">
                      <div class="flex items-center gap-4 mb-8">
                        <div class="avatar placeholder">
                          <div class="bg-primary/10 text-primary rounded-full w-12 h-12">
                            <i class="fas fa-robot text-2xl"></i>
                          </div>
                        </div>
                        <div>
                          <div class="font-black text-lg">{{ formData.title || 'Untitled Role' }}</div>
                          <div class="text-xs text-base-content/40 italic">{{ formData.description || 'No description provided' }}</div>
                        </div>
                      </div>
                      
                      <div class="space-y-6">
                        <div class="chat chat-start">
                          <div class="chat-header opacity-50 text-[10px] mb-1">AI Assistant</div>
                          <div class="chat-bubble bg-base-200 text-base-content border-none shadow-sm text-sm">
                            Hello! I am ready to assist you as <span class="font-bold text-primary">{{ formData.title || 'a professional assistant' }}</span>. How can I help today?
                          </div>
                        </div>
                        
                        <div class="divider text-[10px] opacity-20 uppercase tracking-widest">System Prompt Reference</div>

                        <div v-if="formData.prompt" class="p-5 rounded-xl bg-base-200/30 border border-base-300/50 relative">
                          <i class="fas fa-quote-left absolute -top-2 -left-2 text-base-content/10 text-3xl"></i>
                          <div class="text-xs whitespace-pre-wrap leading-relaxed text-base-content/70">{{ formData.prompt }}</div>
                        </div>
                        <div v-else class="flex flex-col items-center justify-center py-12 text-base-content/20 border-2 border-dashed border-base-300 rounded-xl">
                          <i class="fas fa-terminal text-2xl mb-2"></i>
                          <p class="text-xs">Prompt text will appear here</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Empty State -->
          <div v-else class="flex-1 flex flex-col items-center justify-center text-base-content/20 p-12">
            <div class="w-32 h-32 rounded-full bg-base-100 flex items-center justify-center mb-8 shadow-xl border border-base-300">
              <i class="fas fa-user-tie text-6xl text-base-content/10"></i>
            </div>
            <h3 class="text-2xl font-black mb-3 text-base-content/40">Select a Role to Manage</h3>
            <p class="max-w-xs text-center text-sm leading-relaxed">
              Choose a role from the sidebar to view details, or create a new one to define a custom AI persona.
            </p>
          </div>
        </div>
      </div>
    </div>
    
    <div class="modal-backdrop" @click="$emit('close')"></div>
    
    <!-- Delete Confirmation -->
    <div v-if="roleToDelete" class="modal modal-open">
      <div class="modal-box max-w-sm">
        <h3 class="font-bold text-lg text-error flex items-center gap-2">
          <i class="fas fa-exclamation-triangle"></i>
          {{ t('common.confirm') }}
        </h3>
        <p class="py-4">
          {{ t('roles.deleteConfirm') }}
          <br/>
          <strong class="mt-2 block text-xl">{{ roleToDelete.title }}</strong>
        </p>
        <div class="modal-action">
          <button @click="roleToDelete = null" class="btn btn-ghost">{{ t('common.cancel') }}</button>
          <button 
            @click="handleDeleteRole" 
            class="btn btn-error"
            :disabled="isDeleting"
          >
            <span v-if="isDeleting" class="loading loading-spinner loading-sm"></span>
            {{ t('common.delete') }}
          </button>
        </div>
      </div>
      <div class="modal-backdrop" @click="roleToDelete = null"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useRoleManagement } from '@/composables/useRoleManagement'
import type { Role } from '@/types/role'
import { useI18n } from 'vue-i18n'

// Emits
const emit = defineEmits(['close'])
const { t } = useI18n()

// 使用角色管理composable
const {
  roles,
  isLoading,
  createRole,
  updateRole,
  deleteRole,
  loadRoles,
} = useRoleManagement()

// 搜索
const searchQuery = ref('')

// 过滤后的角色列表
const filteredRoles = computed(() => {
  if (!searchQuery.value.trim()) return roles.value
  const query = searchQuery.value.toLowerCase()
  return roles.value.filter(role => 
    role.title.toLowerCase().includes(query) || 
    role.description.toLowerCase().includes(query)
  )
})

// 表单状态
const showCreateForm = ref(false)
const editingRole = ref<Role | null>(null)
const isSubmitting = ref(false)
const isDeleting = ref(false)
const roleToDelete = ref<Role | null>(null)

// 表单数据
const formData = reactive({
  title: '',
  description: '',
  prompt: '',
})

// 表单错误
const formErrors = reactive({
  title: '',
  description: '',
  prompt: '',
})

// 验证表单
const validateForm = () => {
  formErrors.title = ''
  formErrors.description = ''
  formErrors.prompt = ''
  
  if (!formData.title.trim()) {
    formErrors.title = t('common.validation.required')
    return false
  }
  
  if (formData.title.trim().length > 50) {
    formErrors.title = t('common.validation.maxLength', { max: 50 })
    return false
  }
  
  if (!formData.prompt.trim()) {
    formErrors.prompt = t('common.validation.required')
    return false
  }
  
  if (formData.prompt.trim().length > 20000) {
    formErrors.prompt = t('common.validation.maxLength', { max: 20000 })
    return false
  }
  
  if (formData.description.trim().length > 500) {
    formErrors.description = t('common.validation.maxLength', { max: 500 })
    return false
  }
  
  return true
}

// 重置表单
const resetForm = () => {
  formData.title = ''
  formData.description = ''
  formData.prompt = ''
  formErrors.title = ''
  formErrors.description = ''
  formErrors.prompt = ''
}

// 编辑角色
const editRole = (role: Role) => {
  editingRole.value = role
  formData.title = role.title
  formData.description = role.description
  formData.prompt = role.prompt
  showCreateForm.value = false
}

// 切换到创建模式
const startCreate = () => {
  editingRole.value = null
  showCreateForm.value = true
  resetForm()
}

// 取消表单
const cancelForm = () => {
  showCreateForm.value = false
  editingRole.value = null
  resetForm()
}

// 提交表单
const handleSubmit = async () => {
  if (!validateForm()) return
  
  isSubmitting.value = true
  
  try {
    if (editingRole.value) {
      // 更新角色
      await updateRole({
        id: editingRole.value.id,
        title: formData.title.trim(),
        description: formData.description.trim(),
        prompt: formData.prompt.trim(),
      })
    } else {
      // 创建角色
      await createRole({
        title: formData.title.trim(),
        description: formData.description.trim(),
        prompt: formData.prompt.trim(),
      })
    }
    
    cancelForm()
  } catch (error) {
    console.error('Failed to save role:', error)
  } finally {
    isSubmitting.value = false
  }
}

// 确认删除角色
const confirmDeleteRole = (role: Role) => {
  roleToDelete.value = role
}

// 删除角色
const handleDeleteRole = async () => {
  if (!roleToDelete.value) return
  
  isDeleting.value = true
  
  try {
    await deleteRole(roleToDelete.value.id)
    if (editingRole.value?.id === roleToDelete.value.id) {
      cancelForm()
    }
    roleToDelete.value = null
  } catch (error) {
    console.error('Failed to delete role:', error)
  } finally {
    isDeleting.value = false
  }
}

onMounted(() => {
  loadRoles()
})
</script>

<style scoped>
.line-clamp-1 {
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-clamp: 1;
}

.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-clamp: 2;
}

/* 自定义滚动条样式，使其更细一点 */
.overflow-y-auto::-webkit-scrollbar {
  width: 4px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.1);
  border-radius: 10px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.2);
}
</style>
