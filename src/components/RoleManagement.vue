<template>
  <div class="modal modal-open">
    <div class="modal-box max-w-4xl">
      <h3 class="font-bold text-lg mb-4 flex items-center gap-2">
        <i class="fas fa-user-tie text-primary"></i>
        角色管理
      </h3>
      
      <!-- 角色列表 -->
      <div class="mb-6">
        <div class="flex items-center justify-between mb-4">
          <h4 class="font-semibold">已有角色</h4>
          <button @click="showCreateForm = true" class="btn btn-primary btn-sm gap-2">
            <i class="fas fa-plus"></i>
            新建角色
          </button>
        </div>
        
        <div v-if="isLoading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-md"></span>
        </div>
        
        <div v-else-if="roles.length === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-user-tie text-4xl opacity-30 mb-4"></i>
          <p>暂无自定义角色</p>
          <p class="text-sm">点击"新建角色"创建您的第一个角色</p>
        </div>
        
        <div v-else class="space-y-3">
          <div 
            v-for="role in roles" 
            :key="role.id"
            class="card bg-base-100 border border-base-300 hover:border-primary/50 transition-all duration-200"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0">
                  <h5 class="font-medium text-base mb-1 truncate">{{ role.title }}</h5>
                  <p class="text-sm text-base-content/70 mb-2 line-clamp-2">{{ role.description }}</p>
                  <div class="text-xs text-base-content/50">
                    创建时间: {{ new Date(role.created_at).toLocaleString() }}
                  </div>
                </div>
                <div class="flex gap-2 ml-4">
                  <button 
                    @click="editRole(role)" 
                    class="btn btn-ghost btn-sm gap-1"
                    title="编辑角色"
                  >
                    <i class="fas fa-edit"></i>
                  </button>
                  <button 
                    @click="confirmDeleteRole(role)" 
                    class="btn btn-ghost btn-sm gap-1 text-error hover:bg-error hover:text-error-content"
                    title="删除角色"
                  >
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 创建/编辑表单 -->
      <div v-if="showCreateForm || editingRole" class="border-t pt-6">
        <h4 class="font-semibold mb-4">
          {{ editingRole ? '编辑角色' : '新建角色' }}
        </h4>
        
        <form @submit.prevent="handleSubmit" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">角色名称 <span class="text-error">*</span></span>
            </label>
            <input 
              v-model="formData.title"
              type="text" 
              placeholder="请输入角色名称，如：技术专家、产品经理等"
              class="input input-bordered w-full"
              :class="{ 'input-error': formErrors.title }"
              required
            />
            <label v-if="formErrors.title" class="label">
              <span class="label-text-alt text-error">{{ formErrors.title }}</span>
            </label>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">角色描述</span>
            </label>
            <textarea 
              v-model="formData.description"
              placeholder="简要描述角色的特点和用途"
              class="textarea textarea-bordered h-20 w-full"
              :class="{ 'textarea-error': formErrors.description }"
            ></textarea>
            <label v-if="formErrors.description" class="label">
              <span class="label-text-alt text-error">{{ formErrors.description }}</span>
            </label>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">系统提示词 <span class="text-error">*</span></span>
            </label>
            <textarea 
              v-model="formData.prompt"
              placeholder="请输入详细的系统提示词，这将直接影响AI的回答方式和风格。例如：你是一个专业的网络安全专家，擅长漏洞分析和安全建议..."
              class="textarea textarea-bordered h-32 w-full"
              :class="{ 'textarea-error': formErrors.prompt }"
              required
            ></textarea>
            <label v-if="formErrors.prompt" class="label">
              <span class="label-text-alt text-error">{{ formErrors.prompt }}</span>
            </label>
            <label class="label">
              <span class="label-text-alt">提示词将直接作为AI的系统指令，请详细描述角色的专业背景、回答风格、注意事项等</span>
            </label>
          </div>
          
          <div class="flex gap-3 pt-4">
            <button 
              type="submit" 
              class="btn btn-primary"
              :disabled="isSubmitting"
            >
              <span v-if="isSubmitting" class="loading loading-spinner loading-sm"></span>
              {{ editingRole ? '更新角色' : '创建角色' }}
            </button>
            <button 
              type="button" 
              @click="cancelForm"
              class="btn btn-ghost"
            >
              取消
            </button>
          </div>
        </form>
      </div>
      
      <div class="modal-action">
        <button @click="$emit('close')" class="btn">关闭</button>
      </div>
    </div>
    <div class="modal-backdrop" @click="$emit('close')"></div>
    
    <!-- 删除确认对话框 -->
    <div v-if="roleToDelete" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg">确认删除</h3>
        <p class="py-4">
          确定要删除角色 <strong>{{ roleToDelete.title }}</strong> 吗？此操作不可撤销。
        </p>
        <div class="modal-action">
          <button 
            @click="handleDeleteRole" 
            class="btn btn-error"
            :disabled="isDeleting"
          >
            <span v-if="isDeleting" class="loading loading-spinner loading-sm"></span>
            确认删除
          </button>
          <button @click="roleToDelete = null" class="btn">取消</button>
        </div>
      </div>
      <div class="modal-backdrop" @click="roleToDelete = null"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRoleManagement } from '@/composables/useRoleManagement'
import type { Role } from '@/types/role'

// Emits
const emit = defineEmits(['close'])

// 使用角色管理composable
const {
  roles,
  isLoading,
  createRole,
  updateRole,
  deleteRole,
} = useRoleManagement()

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
    formErrors.title = '角色名称不能为空'
    return false
  }
  
  if (formData.title.trim().length > 50) {
    formErrors.title = '角色名称不能超过50个字符'
    return false
  }
  
  if (!formData.prompt.trim()) {
    formErrors.prompt = '系统提示词不能为空'
    return false
  }
  
  if (formData.prompt.trim().length > 2000) {
    formErrors.prompt = '系统提示词不能超过2000个字符'
    return false
  }
  
  if (formData.description.trim().length > 500) {
    formErrors.description = '角色描述不能超过500个字符'
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
    // TODO: 显示错误提示
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
    roleToDelete.value = null
  } catch (error) {
    console.error('Failed to delete role:', error)
    // TODO: 显示错误提示
  } finally {
    isDeleting.value = false
  }
}
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
