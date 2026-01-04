import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Role, CreateRoleRequest, UpdateRoleRequest } from '@/types/role'

const roles = ref<Role[]>([])
const selectedRole = ref<Role | null>(null)
const isLoading = ref(false)

// 持久化选中角色的key
const SELECTED_ROLE_KEY = 'ai:selectedRoleId'

export function useRoleManagement() {
  // 加载所有角色
  const loadRoles = async () => {
    isLoading.value = true
    try {
      const result = await invoke<Role[]>('get_ai_roles')
      roles.value = result || []
      
      // 尝试恢复之前选中的角色
      await restoreSelectedRole()
    } catch (error) {
      console.error('Failed to load roles:', error)
      roles.value = []
    } finally {
      isLoading.value = false
    }
  }

  // 创建新角色
  const createRole = async (request: CreateRoleRequest): Promise<Role> => {
    try {
      const newRole = await invoke<Role>('create_ai_role', { payload: request })
      roles.value.push(newRole)
      return newRole
    } catch (error) {
      console.error('Failed to create role:', error)
      throw error
    }
  }

  // 更新角色
  const updateRole = async (request: UpdateRoleRequest): Promise<void> => {
    try {
      await invoke('update_ai_role', { payload: request })
      const index = roles.value.findIndex(r => r.id === request.id)
      if (index !== -1) {
        roles.value[index] = { ...roles.value[index], ...request, updated_at: new Date() }
      }
    } catch (error) {
      console.error('Failed to update role:', error)
      throw error
    }
  }

  // 删除角色
  const deleteRole = async (id: string) => {
    try {
      await invoke('delete_ai_role', { id })
      roles.value = roles.value.filter(r => r.id !== id)
      
      // 如果删除的是当前选中的角色，清除选择
      if (selectedRole.value?.id === id) {
        selectedRole.value = null
        localStorage.removeItem(SELECTED_ROLE_KEY)
      }
    } catch (error) {
      console.error('Failed to delete role:', error)
      throw error
    }
  }

  // 选择角色
  const selectRole = async (role: Role | null) => {
    try {
      // 调用后端API设置当前角色
      await invoke('set_current_ai_role', { roleId: role?.id || null })
      
      // 更新本地状态
      selectedRole.value = role
      
      // 同步到localStorage（用于UI状态恢复）
      if (role) {
        localStorage.setItem(SELECTED_ROLE_KEY, role.id)
      } else {
        localStorage.removeItem(SELECTED_ROLE_KEY)
      }
    } catch (error) {
      console.error('Failed to set current role:', error)
      throw error
    }
  }

  // 恢复选中的角色（从后端获取）
  const restoreSelectedRole = async () => {
    try {
      const currentRole = await invoke<Role | null>('get_current_ai_role')
      selectedRole.value = currentRole
      
      // 同步到localStorage
      if (currentRole) {
        localStorage.setItem(SELECTED_ROLE_KEY, currentRole.id)
      } else {
        localStorage.removeItem(SELECTED_ROLE_KEY)
      }
    } catch (error) {
      console.error('Failed to restore selected role:', error)
      // 降级到localStorage恢复
      try {
        const savedId = localStorage.getItem(SELECTED_ROLE_KEY)
        if (savedId && roles.value.length > 0) {
          const found = roles.value.find(r => r.id === savedId)
          if (found) {
            selectedRole.value = found
          }
        }
      } catch (fallbackError) {
        console.error('Failed to fallback restore selected role:', fallbackError)
      }
    }
  }

  // 获取角色的system prompt
  const getRoleSystemPrompt = (role: Role | null): string => {
    if (!role || !role.prompt.trim()) {
      return ''
    }
    return role.prompt
  }

  // AI生成角色
  const generateRole = async (prompt: string): Promise<CreateRoleRequest> => {
    try {
      const result = await invoke<CreateRoleRequest>('generate_ai_role', { prompt })
      return result
    } catch (error) {
      console.error('Failed to generate role with AI:', error)
      throw error
    }
  }

  return {
    roles: computed(() => roles.value),
    selectedRole: computed(() => selectedRole.value),
    isLoading: computed(() => isLoading.value),
    loadRoles,
    createRole,
    updateRole,
    deleteRole,
    selectRole,
    getRoleSystemPrompt,
    generateRole,
  }
}
