# 资产发现功能完整总结

## ✅ 所有问题已修复

### 1. 按钮灰色问题 ✅
**问题**：选择插件后"开始发现"按钮灰色无法点击
**原因**：`ToolInfo` 结构体没有 `id` 字段，只有 `name` 字段
**解决**：将所有使用 `plugin.id` 的地方改为 `plugin.name`

### 2. 项目关联问题 ✅
**问题**：从"变更监控"页面点击"发现资产"时，没有选择项目
**原因**：`selectedProgram` 只在点击项目时设置，从其他页面进入时为 `null`
**解决**：在"发现资产"对话框中添加项目选择下拉框

---

## 🎯 两种使用场景

### 场景 1：从项目列表进入（已有项目）

```
BugBounty → 项目列表 → 点击项目 → 发现资产
```

**流程**：
1. 用户在项目列表中点击某个项目
2. `selectedProgram` 被设置
3. 打开"发现资产"对话框
4. **自动显示已选择的项目**（绿色提示框）
5. 用户选择插件并填写参数
6. 资产自动关联到该项目

**UI 显示**：
```
┌─────────────────────────────────────────┐
│ 发现资产                                 │
├─────────────────────────────────────────┤
│ ℹ️ 执行插件来发现新资产，并可选择自动导入 │
├─────────────────────────────────────────┤
│ ✅ 已选择项目                            │
│    Example Bug Bounty Program           │
├─────────────────────────────────────────┤
│ 选择插件 *                               │
│ [plugin__subdomain_enumerator     ▼]    │
├─────────────────────────────────────────┤
│ Target domain... *                      │
│ [example.com                        ]   │
└─────────────────────────────────────────┘
```

### 场景 2：从变更监控/其他页面进入（无项目）

```
BugBounty → 变更监控 → 发现资产
```

**流程**：
1. 用户在变更监控页面点击"发现资产"
2. `selectedProgram` 为 `null`
3. 打开"发现资产"对话框
4. **显示项目选择下拉框**
5. 用户先选择项目
6. 然后选择插件并填写参数
7. 资产关联到选择的项目

**UI 显示**：
```
┌─────────────────────────────────────────┐
│ 发现资产                                 │
├─────────────────────────────────────────┤
│ ℹ️ 执行插件来发现新资产，并可选择自动导入 │
├─────────────────────────────────────────┤
│ 选择项目 *                               │
│ [请选择一个项目                    ▼]    │
│   - Example Bug Bounty Program          │
│   - Test Program                        │
│   - Production Program                  │
├─────────────────────────────────────────┤
│ 选择插件 *                               │
│ [plugin__subdomain_enumerator     ▼]    │
├─────────────────────────────────────────┤
│ Target domain... *                      │
│ [example.com                        ]   │
└─────────────────────────────────────────┘
```

---

## 🔧 技术实现

### 前端代码

```vue
<template>
  <!-- Program Selection (if not pre-selected) -->
  <div v-if="!selectedProgram" class="form-control">
    <label class="label">
      <span class="label-text">{{ t('bugBounty.selectProgram') }} *</span>
    </label>
    <select v-model="form.program_id" class="select select-bordered">
      <option value="">{{ t('bugBounty.selectProgramPlaceholder') }}</option>
      <option v-for="program in availablePrograms" :key="program.id" :value="program.id">
        {{ program.name }}
      </option>
    </select>
  </div>
  
  <!-- Show selected program info -->
  <div v-else class="alert alert-success text-sm">
    <i class="fas fa-trophy"></i>
    <div>
      <div class="font-semibold">{{ t('bugBounty.selectedProgram') }}</div>
      <div>{{ selectedProgram.name }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
// Load programs if not pre-selected
const loadPrograms = async () => {
  const response = await invoke('bounty_list_programs')
  availablePrograms.value = response.programs || []
}

// Validate form
const isFormValid = computed(() => {
  // Check if program is selected (either from props or form)
  const programId = props.selectedProgram?.id || form.program_id
  if (!programId) return false
  
  if (!form.plugin_id) return false
  // ... other validations
})

// Execute discovery
const executeDiscovery = async () => {
  // Get program ID from props or form
  const programId = props.selectedProgram?.id || form.program_id
  if (!programId) {
    toast.error(t('bugBounty.monitor.selectProgramFirst'))
    return
  }
  
  const result = await invoke('monitor_discover_and_import_assets', {
    request: {
      program_id: programId,  // ✅ 使用选择的项目ID
      // ...
    }
  })
}
</script>
```

### 验证逻辑

```typescript
const isFormValid = computed(() => {
  // 1. 检查项目是否选择
  const programId = props.selectedProgram?.id || form.program_id
  if (!programId) return false  // ❌ 没有项目，按钮禁用
  
  // 2. 检查插件是否选择
  if (!form.plugin_id) return false
  
  // 3. 检查必填字段（基于 Schema）
  if (pluginInputSchema.value?.required) {
    for (const field of required) {
      if (field === 'domain' && !form.domain) return false
      if (field === 'targets' && !form.urls) return false
    }
  }
  
  return true  // ✅ 所有验证通过，按钮可点击
})
```

---

## 📊 数据流

### 完整流程

```
用户操作
  ↓
场景判断
  ├─ 有 selectedProgram → 显示项目信息
  └─ 无 selectedProgram → 显示项目选择框
  ↓
用户选择/确认项目
  ↓
选择插件
  ↓
加载插件 Schema
  ↓
显示输入字段（基于 Schema）
  ↓
填写参数
  ↓
验证表单（项目 + 插件 + 参数）
  ↓
点击"开始发现"
  ↓
执行插件
  ↓
解析输出
  ↓
创建资产并关联到项目
  ↓
显示结果
```

### 项目ID传递

```typescript
// 方式 1：从 props 获取（已选择项目）
program_id: props.selectedProgram?.id

// 方式 2：从 form 获取（手动选择项目）
program_id: form.program_id

// 最终使用（优先 props）
program_id: props.selectedProgram?.id || form.program_id
```

---

## 🎨 UI 状态

### 按钮状态逻辑

```typescript
:disabled="!isFormValid || discovering"
```

**禁用条件**：
- ❌ 没有选择项目
- ❌ 没有选择插件
- ❌ 必填字段未填写
- ❌ 正在执行中

**可点击条件**：
- ✅ 已选择项目（props 或 form）
- ✅ 已选择插件
- ✅ 所有必填字段已填写
- ✅ 未在执行中

### 视觉反馈

1. **项目已选择**（绿色提示）：
   ```
   ✅ 已选择项目
      Example Bug Bounty Program
   ```

2. **项目未选择**（下拉框）：
   ```
   选择项目 *
   [请选择一个项目        ▼]
   ```

3. **加载中**：
   ```
   选择项目 *
   [加载中...             ▼]
   ```

---

## 📝 国际化

### 中文

```typescript
{
  selectProgram: '选择项目',
  selectProgramPlaceholder: '请选择一个项目',
  selectedProgram: '已选择项目',
}
```

### 英文

```typescript
{
  selectProgram: 'Select Program',
  selectProgramPlaceholder: 'Please select a program',
  selectedProgram: 'Selected Program',
}
```

---

## 🧪 测试场景

### 测试 1：从项目列表进入

1. 进入 BugBounty → 项目列表
2. 点击某个项目
3. 点击"发现资产"按钮
4. **预期**：显示绿色"已选择项目"提示
5. 选择插件并填写参数
6. **预期**：按钮可点击
7. 点击"开始发现"
8. **预期**：资产关联到该项目

### 测试 2：从变更监控进入

1. 进入 BugBounty → 变更监控
2. 点击"发现资产"按钮
3. **预期**：显示"选择项目"下拉框
4. 不选择项目，直接选择插件
5. **预期**：按钮仍然禁用（灰色）
6. 选择一个项目
7. 选择插件并填写参数
8. **预期**：按钮可点击
9. 点击"开始发现"
10. **预期**：资产关联到选择的项目

### 测试 3：验证逻辑

1. 打开"发现资产"对话框
2. **预期**：按钮禁用（没有项目）
3. 选择项目
4. **预期**：按钮仍禁用（没有插件）
5. 选择插件
6. **预期**：按钮仍禁用（没有填写必填字段）
7. 填写 domain
8. **预期**：按钮可点击

---

## 🎯 最终效果

### ✅ 已实现的功能

1. **灵活的项目选择**
   - 从项目列表进入：自动使用选择的项目
   - 从其他页面进入：手动选择项目

2. **智能表单验证**
   - 基于 Schema 的动态验证
   - 清晰的必填字段标记
   - 实时的按钮状态更新

3. **完整的用户反馈**
   - 项目选择状态提示
   - 插件输入要求说明
   - Schema 加载状态
   - 执行结果展示

4. **自动项目关联**
   - 发现的资产自动关联到选择的项目
   - 自动去重
   - 自动添加标签

### 📊 使用统计

```
支持的入口：
├─ 项目列表 → 发现资产 ✅
├─ 变更监控 → 发现资产 ✅
├─ 监控任务 → 发现资产 ✅
└─ 资产表面 → 发现资产 ✅

支持的插件：
├─ plugin__subdomain_enumerator ✅
├─ plugin__http_prober ✅
├─ plugin__port_monitor ✅
└─ ... 其他插件 ✅

支持的资产类型：
├─ domain（子域名）✅
├─ url（URL）✅
├─ port（端口）✅
└─ ... 其他类型 ✅
```

---

## 🚀 后续优化建议

### 1. 记住上次选择的项目

```typescript
// 保存到 localStorage
localStorage.setItem('lastSelectedProgramId', programId)

// 下次打开时自动选择
const lastProgramId = localStorage.getItem('lastSelectedProgramId')
if (lastProgramId && !props.selectedProgram) {
  form.program_id = lastProgramId
}
```

### 2. 支持批量项目

```vue
<select v-model="form.program_ids" multiple>
  <option v-for="program in availablePrograms" :value="program.id">
    {{ program.name }}
  </option>
</select>
```

### 3. 项目快速创建

```vue
<div class="form-control">
  <select v-model="form.program_id">
    <!-- ... options ... -->
  </select>
  <button @click="createNewProgram" class="btn btn-sm btn-ghost">
    <i class="fas fa-plus"></i> 创建新项目
  </button>
</div>
```

---

## 📋 总结

### 问题 1：按钮灰色
- ✅ **已修复**：使用 `plugin.name` 代替 `plugin.id`

### 问题 2：项目关联
- ✅ **已修复**：添加项目选择功能
- ✅ **支持两种场景**：预选项目 + 手动选择

### 最终状态
- ✅ 所有入口都能正常使用
- ✅ 资产正确关联到项目
- ✅ 完整的用户体验
- ✅ 清晰的视觉反馈

**现在可以从任何页面使用"发现资产"功能了！** 🎉
