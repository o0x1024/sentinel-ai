# 提示词分组删除功能

**更新时间**: 2025-11-13  
**功能**: 为提示词管理界面添加分组删除功能

---

## 功能描述

在提示词管理界面的"分组管理"区域添加了删除按钮，允许用户删除自定义的提示词分组。

## 界面变更

### 删除按钮位置

```
提示词分组管理区域：
┌─────────────────────────────────┐
│ 分组                            │
│ [新建] [设为默认] [删除]        │ ← 新增删除按钮
│                                 │
│ ┌─────────────────────────┐   │
│ │ 分组1                    │   │
│ │ 分组2 [默认]             │   │
│ │ 分组3                    │   │
│ └─────────────────────────┘   │
└─────────────────────────────────┘
```

### 删除按钮特性

1. **样式**: 红色错误按钮 (`btn-error`)
2. **禁用条件**:
   - 未选中任何分组时禁用
   - 选中的是默认分组时禁用（防止误删）
3. **位置**: 位于"新建"和"设为默认"按钮右侧

## 功能行为

### 1. 删除前确认
```typescript
- 弹出确认对话框
- 显示将要删除的分组名称
- 警告用户：此操作将删除该分组的所有阶段映射
- 使用错误样式（红色）提醒用户操作的严重性
```

### 2. 安全检查
```typescript
// 防止删除默认分组
if (group?.is_default) {
  toast.error('不能删除默认分组')
  return
}
```

### 3. 删除流程
```
1. 用户点击删除按钮
2. 检查是否为默认分组 → 如果是，提示错误并返回
3. 显示确认对话框
4. 用户确认后调用后端 API
5. 删除成功后：
   - 清除当前选中的分组ID
   - 重新加载分组列表
   - 显示成功提示
6. 删除失败时显示错误信息
```

## 代码实现

### 前端代码

```typescript
async function deleteGroup() {
  if (!selectedGroupId.value) return
  
  // 防止删除默认分组
  const group = promptGroups.value.find(g => g.id === selectedGroupId.value)
  if (group?.is_default) {
    toast.error('不能删除默认分组')
    return
  }
  
  // 确认对话框
  const confirmed = await dialog.confirm({
    title: t('promptMgmt.groups') as unknown as string,
    message: `确定要删除分组"${group?.name}"吗？此操作将删除该分组的所有阶段映射。`,
    variant: 'error'
  })
  
  if (!confirmed) return
  
  try {
    // 调用后端 API
    await invoke('delete_prompt_group_api', { id: selectedGroupId.value } as any)
    selectedGroupId.value = null
    await loadGroups()
    toast.success('分组已删除')
  } catch (error) {
    console.error('Failed to delete group:', error)
    toast.error('删除分组失败: ' + (error as any).message)
  }
}
```

### UI 按钮

```vue
<button 
  class="btn btn-xs btn-error" 
  :disabled="!selectedGroupId || selectedGroup?.is_default" 
  @click="deleteGroup">
  {{ $t('common.delete') }}
</button>
```

## 后端 API

使用已有的 `delete_prompt_group_api` 命令：

```rust
#[tauri::command]
pub async fn delete_prompt_group_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.delete_group(id).await.map_err(|e| e.to_string())
}
```

**注意**: 后端 API 使用 `id` 作为参数名，前端调用时需要匹配。

## 用户体验优化

### 1. 安全性
- ✅ 防止删除默认分组
- ✅ 删除前确认对话框
- ✅ 显示分组名称，避免误删
- ✅ 明确提示将删除关联的阶段映射

### 2. 反馈
- ✅ 删除成功后显示成功提示
- ✅ 删除失败时显示具体错误信息
- ✅ 自动重新加载分组列表

### 3. 状态管理
- ✅ 删除后清除选中状态
- ✅ 自动刷新分组列表
- ✅ 保持界面一致性

## 使用场景

### 场景 1: 删除测试分组
```
1. 用户创建了测试用的提示词分组
2. 测试完成后想要清理
3. 选中测试分组
4. 点击删除按钮
5. 确认删除
6. 分组被移除
```

### 场景 2: 尝试删除默认分组（被阻止）
```
1. 用户选中标记为"默认"的分组
2. 删除按钮自动禁用（灰色）
3. 如果用户尝试其他方式删除
4. 系统提示"不能删除默认分组"
5. 操作被阻止
```

## 注意事项

### 1. 默认分组保护
默认分组是系统的基础配置，不能删除。如果需要更换默认分组：
```
1. 先将其他分组设为默认
2. 原默认分组会自动变为普通分组
3. 然后可以删除原默认分组
```

### 2. 关联数据
删除分组时会同时删除：
- 该分组的所有阶段映射
- 数据库中的相关记录

**不会删除**：
- 分组中引用的提示词模板本身
- 其他分组的配置

### 3. 无法撤销
删除操作无法撤销，用户需要谨慎操作。系统通过确认对话框提醒用户。

## 测试清单

- [ ] 删除普通分组成功
- [ ] 删除默认分组被阻止（按钮禁用）
- [ ] 删除前确认对话框正常显示
- [ ] 取消删除操作正常工作
- [ ] 删除后分组列表正确刷新
- [ ] 删除失败时错误提示正确显示
- [ ] 删除成功后成功提示正确显示

## 相关文件

### 已修改文件
- `src/views/PromptManagement.vue` - 添加删除按钮和删除函数

### 使用的后端 API
- `delete_prompt_group_api` - 删除提示词分组

---

**维护者**: Sentinel AI Team  
**最后更新**: 2025-11-13  
**状态**: ✅ 已实现，等待测试

