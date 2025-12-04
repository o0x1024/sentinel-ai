<template>
  <div v-if="isOpen" class="modal-overlay" @click.self="closeModal">
    <div class="modal-content">
      <div class="modal-header">
        <h2 class="text-xl font-semibold">{{ t('roles.roleManagement') }}</h2>
        <button @click="closeModal" class="close-button">&times;</button>
      </div>
      <div class="modal-body">
        <!-- Main view: Role list -->
        <div v-if="!editingRole">
          <div class="flex justify-between items-center mb-4">
            <div class="flex items-center gap-4">
              <span class="text-gray-600">{{ roles.length }} {{ t('roles.rolesCount') }}</span>
              <div class="view-toggle">
                <button @click="viewMode = 'grid'" class="view-toggle-button" :class="{ active: viewMode === 'grid' }" :title="t('roles.gridView')">
                  <i class="fas fa-th-large"></i>
                </button>
                <button @click="viewMode = 'list'" class="view-toggle-button" :class="{ active: viewMode === 'list' }" :title="t('roles.listView')">
                  <i class="fas fa-list"></i>
                </button>
              </div>
            </div>
            <button @click="startCreatingRole" class="new-role-button">
              <i class="fas fa-plus mr-2"></i>{{ t('roles.newRole') }}
            </button>
          </div>
          
          <!-- 角色列表 -->
          <div v-if="viewMode === 'grid'" class="roles-grid">
            <div v-for="role in roles" :key="role.id" class="role-card">
              <div class="role-card-header">
                <h3 class="font-semibold">{{ role.title }}</h3>
                <div class="role-card-actions">
                  <button @click="startEditingRole(role)" class="icon-button" :title="t('common.edit')">
                    <i class="fas fa-edit"></i>
                  </button>
                  <button @click="confirmDeleteRole(role.id)" class="icon-button delete-button" :title="t('common.delete')">
                    <i class="fas fa-trash"></i>
                  </button>
                </div>
              </div>
              <p class="role-card-description">{{ role.description }}</p>
              <div class="role-card-footer">
                <button @click="selectRole(role)" class="select-role-button">
                  <i class="fas fa-check mr-2"></i>{{ t('roles.selectThisRole') }}
                </button>
              </div>
            </div>
          </div>

          <div v-else class="roles-list">
            <div class="role-list-header">
              <div class="w-1/4 font-semibold">{{ t('roles.roleTitle') }}</div>
              <div class="flex-1 font-semibold">{{ t('roles.roleDescription') }}</div>
              <div class="w-48 text-right font-semibold">{{ t('common.actions') }}</div>
            </div>
            <div v-for="role in roles" :key="role.id" class="role-list-item">
              <div class="w-1/4 font-semibold text-gray-800 truncate" :title="role.title">{{ role.title }}</div>
              <div class="flex-1 text-sm text-gray-600 truncate" :title="role.description">{{ role.description }}</div>
              <div class="w-48 flex justify-end items-center gap-2">
                <button @click="selectRole(role)" class="list-select-button">
                  <i class="fas fa-check mr-1"></i>{{ t('roles.select') }}
                </button>
                <button @click="startEditingRole(role)" class="icon-button" :title="t('common.edit')">
                  <i class="fas fa-edit"></i>
                </button>
                <button @click="confirmDeleteRole(role.id)" class="icon-button delete-button" :title="t('common.delete')">
                  <i class="fas fa-trash"></i>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Form view: Create/Edit Role -->
        <div v-else>
          <div class="edit-role-header">
            <button @click="cancelEditing" class="back-button">
              <i class="fas fa-arrow-left mr-2"></i>{{ t('roles.backToList') }}
            </button>
            <h3 class="text-lg font-semibold">{{ isCreating ? t('roles.newRole') : t('roles.editRole') }}</h3>
          </div>
          
          <form @submit.prevent="saveRole" class="edit-role-form">
            <div class="form-group">
              <label for="role-title">{{ t('roles.roleTitle') }}</label>
              <input id="role-title" v-model="editingRole.title" type="text" required />
              <div class="character-count">{{ editingRole.title.length }}/50</div>
            </div>
            <div class="form-group">
              <label for="role-description">{{ t('roles.roleDescription') }}</label>
              <textarea id="role-description" v-model="editingRole.description" rows="5" required></textarea>
              <div class="character-count">{{ editingRole.description.length }}/100000</div>
            </div>
            <div class="form-group">
              <label for="role-prompt">{{ t('roles.rolePrompt') }}</label>
              <textarea id="role-prompt" v-model="editingRole.prompt" rows="5" required></textarea>
            </div>
            <div class="form-actions">
              <button type="button" @click="cancelEditing" class="cancel-button">{{ t('common.cancel') }}</button>
              <button type="submit" class="save-button">{{ t('common.save') }}</button>
            </div>
          </form>
        </div>
      </div>
      <div class="modal-footer">
        <button @click="closeModal" class="close-modal-button">{{ t('common.close') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { dialog } from '@/composables/useDialog';

const { t } = useI18n();

const props = defineProps<{
  isOpen: boolean
}>();

const emit = defineEmits(['close', 'select-role']);

interface Role {
  id: string;
  title: string;
  description: string;
  prompt: string;
  is_system?: boolean;
}

const roles = ref<Role[]>([]);
const editingRole = ref<Role | null>(null);
const viewMode = ref<'grid' | 'list'>('grid');

const isCreating = computed(() => editingRole.value && !editingRole.value.id);

const loadRoles = async () => {
  try {
    roles.value = await invoke('get_ai_roles');
  } catch (error) {
    console.error("Failed to load roles:", error);
    dialog.error(t('roles.loadError'));
  }
};

const startCreatingRole = () => {
  editingRole.value = { id: '', title: '', description: '', prompt: '' };
};

const startEditingRole = (role: Role) => {
  editingRole.value = { ...role };
};

const cancelEditing = () => {
  editingRole.value = null;
};

const saveRole = async () => {
  if (!editingRole.value) return;

  try {
    if (isCreating.value) {
      await invoke('create_ai_role', { payload: editingRole.value });
    } else {
      await invoke('update_ai_role', { payload: editingRole.value });
    }
    await loadRoles();
    cancelEditing();
    dialog.success(t('roles.saveSuccess'));
  } catch (error) {
    console.error("Failed to save role:", error);
    dialog.error(t('roles.saveError'));
  }
};

const confirmDeleteRole = async (roleId: string) => {
    console.log("Attempting to delete role with ID:", roleId);
    
    // 检查角色是否存在
    const roleToDelete = roles.value.find(r => r.id === roleId);
    if (!roleToDelete) {
      console.error("Role not found:", roleId);
      dialog.error("角色未找到");
      return;
    }
    
    // 直接删除角色，不使用对话框确认
    try {
      console.log("直接删除角色，ID:", roleId);
      await invoke('delete_ai_role', { id: roleId });
      console.log("角色删除成功:", roleId);
      
      // 重新加载角色列表
      await loadRoles();
      dialog.success(t('roles.deleteSuccess'));
    } catch (error) {
      console.error("删除角色失败:", error);
      dialog.error(t('roles.deleteError') + ": " + String(error));
    }
};

const closeModal = () => {
  emit('close');
};

const selectRole = (role: Role) => {
  emit('select-role', role);
  closeModal();
};

watch(() => props.isOpen, (newValue) => {
  if (newValue) {
    loadRoles();
    editingRole.value = null; // Reset view on open
  }
});
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 100;
}

.modal-content {
  background-color: #ffffff;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  width: 90%;
  max-width: 900px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  padding: 1rem;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: #ffffff;
}

.close-button {
  background: none;
  border: none;
  font-size: calc(var(--font-size-base, 14px) * 1.5);
  cursor: pointer;
  color: #9ca3af;
  padding: 0;
  line-height: 1;
}

.modal-body {
  padding: 1.5rem;
  overflow-y: auto;
  flex: 1;
}

.new-role-button {
  background-color: #6f0fb3;
  color: white;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  display: inline-flex;
  align-items: center;
  transition: background-color 0.2s;
}

.new-role-button:hover {
  background-color: #5a0c91;
}

.view-toggle {
  display: flex;
  align-items: center;
  background-color: #f0f0f0;
  border-radius: 6px;
  padding: 2px;
}

.view-toggle-button {
  background: none;
  border: none;
  padding: 0.3rem 0.6rem;
  cursor: pointer;
  color: #666;
  border-radius: 4px;
}

.view-toggle-button.active {
  background-color: #ffffff;
  color: #6f0fb3;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

.roles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
  gap: 1rem;
  margin-top: 1rem;
}

.role-card {
  background-color: #f9f9f9;
  border-radius: 8px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  transition: all 0.2s ease;
  border: 1px solid #e5e7eb;
  position: relative;
}

.role-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 0.75rem;
}

.role-card-actions {
  display: flex;
  gap: 0.5rem;
}

.icon-button {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  padding: 0.25rem;
  border-radius: 50%;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.icon-button:hover {
  background-color: #f0f0f0;
}

.delete-button {
  background-color: #f8d7da;
  color: #721c24;
}

.delete-button:hover {
  background-color: #f5c6cb;
}

.role-card-description {
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  color: #4b5563;
  line-height: 1.5;
  margin-bottom: 1.5rem;
  flex-grow: 1;
}

.role-card-footer {
  display: flex;
  justify-content: center;
  margin-top: auto;
}

.select-role-button {
  width: 100%;
  padding: 0.5rem;
  border: none;
  background-color: #f0f0f0;
  color: #333;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  display: flex;
  align-items: center;
  justify-content: center;
}

.select-role-button:hover {
  background-color: #e0e0e0;
}

.roles-list {
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
  margin-top: 1rem;
}

.role-list-header,
.role-list-item {
  display: flex;
  align-items: center;
  padding: 0.75rem 1.25rem;
  gap: 1rem;
}

.role-list-header {
  background-color: #f9fafb;
  color: #374151;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.role-list-item {
  border-top: 1px solid #e5e7eb;
}

.role-list-item:hover {
  background-color: #f9f9f9;
}

.list-select-button {
  background-color: #e5e7eb;
  color: #374151;
  padding: 0.25rem 0.75rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
  display: inline-flex;
  align-items: center;
  transition: background-color 0.2s;
  white-space: nowrap;
}

.list-select-button:hover {
  background-color: #d1d5db;
}

.modal-footer {
  padding: 1rem;
  border-top: 1px solid #e5e7eb;
  display: flex;
  justify-content: flex-end;
}

.close-modal-button {
  background-color: #f0f0f0;
  color: #333;
  padding: 0.5rem 1.5rem;
  border-radius: 4px;
  border: none;
  cursor: pointer;
}

.close-modal-button:hover {
  background-color: #e0e0e0;
}

/* 编辑角色表单样式 */
.edit-role-header {
  display: flex;
  align-items: center;
  margin-bottom: 1.5rem;
}

.back-button {
  display: flex;
  align-items: center;
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  margin-right: 1rem;
}

.edit-role-form {
  max-width: 800px;
}

.form-group {
  margin-bottom: 1.5rem;
  position: relative;
}

.form-group label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
  color: #333;
}

.form-group input,
.form-group textarea {
  width: 100%;
  padding: 0.75rem;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  background-color: #fff;
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.form-group textarea {
  min-height: 120px;
  resize: vertical;
}

.character-count {
  position: absolute;
  right: 0;
  bottom: -1.25rem;
  font-size: calc(var(--font-size-base, 14px) * 0.75);
  color: #6b7280;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 2rem;
}

.cancel-button {
  background-color: #f0f0f0;
  color: #333;
  padding: 0.5rem 1.5rem;
  border-radius: 4px;
  border: none;
  cursor: pointer;
}

.save-button {
  background-color: #6f0fb3;
  color: white;
  padding: 0.5rem 1.5rem;
  border-radius: 4px;
  border: none;
  cursor: pointer;
}

.save-button:hover {
  background-color: #5a0c91;
}
</style> 