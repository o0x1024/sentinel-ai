# Tenth Man 前端集成示例

## Vue 组件示例

### 1. 显示 Tenth Man 审查结果

```vue
<template>
  <div v-if="tenthManCritique" class="tenth-man-critique">
    <div class="critique-header">
      <AlertTriangle :class="getRiskClass(critique.risk_level)" />
      <h3>{{ $t('agent.tenthManCritique') }}</h3>
      <Badge :variant="getRiskVariant(critique.risk_level)">
        {{ getRiskLabel(critique.risk_level) }}
      </Badge>
    </div>
    
    <div class="critique-content">
      <div v-html="formatCritique(critique.critique)" />
    </div>
    
    <div class="critique-footer">
      <span class="trigger-info">
        {{ critique.mode === 'system_enforced' 
          ? $t('agent.tenthManSystemEnforced') 
          : $t('agent.tenthManToolCalled') 
        }}
      </span>
      <span class="timestamp">
        {{ formatTime(critique.timestamp) }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { AlertTriangle } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface TenthManCritique {
  execution_id: string;
  critique: string;
  message_id: string;
  trigger: string;
  mode: string;
  risk_level?: string;
  timestamp: number;
}

const tenthManCritique = ref<TenthManCritique | null>(null);
let unlistenCritique: (() => void) | null = null;

onMounted(async () => {
  // 监听 Tenth Man 审查事件
  unlistenCritique = await listen('agent:tenth_man_critique', (event) => {
    const data = event.payload as TenthManCritique;
    tenthManCritique.value = {
      ...data,
      timestamp: Date.now()
    };
  });
});

onUnmounted(() => {
  if (unlistenCritique) {
    unlistenCritique();
  }
});

function getRiskClass(level: string): string {
  const classes: Record<string, string> = {
    none: 'text-green-500',
    low: 'text-blue-500',
    medium: 'text-yellow-500',
    high: 'text-orange-500',
    critical: 'text-red-500'
  };
  return classes[level] || 'text-gray-500';
}

function getRiskVariant(level: string): string {
  const variants: Record<string, string> = {
    none: 'success',
    low: 'info',
    medium: 'warning',
    high: 'warning',
    critical: 'destructive'
  };
  return variants[level] || 'default';
}

function getRiskLabel(level: string): string {
  return t(`agent.riskLevel${level.charAt(0).toUpperCase() + level.slice(1)}`);
}

function formatCritique(critique: string): string {
  // 将 Markdown 格式转换为 HTML
  return critique
    .replace(/\*\*\[([^\]]+)\]\*\*/g, '<strong class="text-red-600">[$1]</strong>')
    .replace(/\*\*(\d+\. [^:]+):\*\*/g, '<strong class="text-blue-600">$1:</strong>')
    .replace(/\n/g, '<br>');
}

function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString();
}
</script>

<style scoped>
.tenth-man-critique {
  @apply border-l-4 border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20 p-4 rounded-lg my-4;
}

.critique-header {
  @apply flex items-center gap-2 mb-3;
}

.critique-header h3 {
  @apply text-lg font-semibold flex-1;
}

.critique-content {
  @apply prose dark:prose-invert max-w-none mb-3;
}

.critique-footer {
  @apply flex justify-between text-sm text-gray-500 dark:text-gray-400;
}
</style>
```

### 2. 工具管理界面

```vue
<template>
  <div class="tool-management">
    <h2>{{ $t('tools.management') }}</h2>
    
    <div class="tool-list">
      <div 
        v-for="tool in tools" 
        :key="tool.id"
        class="tool-item"
      >
        <div class="tool-info">
          <h3>{{ tool.name }}</h3>
          <p>{{ tool.description }}</p>
          <Badge>{{ tool.category }}</Badge>
        </div>
        
        <Switch
          :checked="tool.enabled"
          @update:checked="(val) => toggleTool(tool.id, val)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { useToast } from '@/components/ui/toast';

const { toast } = useToast();

interface BuiltinToolInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  version: string;
  enabled: boolean;
  input_schema?: any;
}

const tools = ref<BuiltinToolInfo[]>([]);

onMounted(async () => {
  await loadTools();
});

async function loadTools() {
  try {
    tools.value = await invoke('get_builtin_tools_with_status');
  } catch (error) {
    console.error('Failed to load tools:', error);
    toast({
      title: 'Error',
      description: 'Failed to load tools',
      variant: 'destructive'
    });
  }
}

async function toggleTool(toolId: string, enabled: boolean) {
  try {
    await invoke('toggle_builtin_tool', {
      tool_name: toolId,
      enabled
    });
    
    // 更新本地状态
    const tool = tools.value.find(t => t.id === toolId);
    if (tool) {
      tool.enabled = enabled;
    }
    
    toast({
      title: 'Success',
      description: `Tool ${toolId} ${enabled ? 'enabled' : 'disabled'}`,
      variant: 'success'
    });
  } catch (error) {
    console.error('Failed to toggle tool:', error);
    toast({
      title: 'Error',
      description: 'Failed to toggle tool',
      variant: 'destructive'
    });
  }
}
</script>

<style scoped>
.tool-management {
  @apply p-6;
}

.tool-list {
  @apply space-y-4 mt-4;
}

.tool-item {
  @apply flex items-center justify-between p-4 border rounded-lg;
}

.tool-info {
  @apply flex-1;
}

.tool-info h3 {
  @apply font-semibold mb-1;
}

.tool-info p {
  @apply text-sm text-gray-600 dark:text-gray-400 mb-2;
}
</style>
```

### 3. 手动触发 Tenth Man 审查

```vue
<template>
  <div class="manual-review">
    <Button @click="showDialog = true">
      <Shield class="w-4 h-4 mr-2" />
      {{ $t('agent.requestTenthManReview') }}
    </Button>
    
    <Dialog v-model:open="showDialog">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('agent.tenthManReview') }}</DialogTitle>
        </DialogHeader>
        
        <div class="space-y-4">
          <div>
            <Label>{{ $t('agent.contentToReview') }}</Label>
            <Textarea
              v-model="reviewContent"
              :placeholder="$t('agent.enterContentToReview')"
              rows="6"
            />
          </div>
          
          <div>
            <Label>{{ $t('agent.contextDescription') }}</Label>
            <Input
              v-model="contextDescription"
              :placeholder="$t('agent.optionalContext')"
            />
          </div>
          
          <div>
            <Label>{{ $t('agent.reviewType') }}</Label>
            <Select v-model="reviewType">
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="quick">Quick Review</SelectItem>
                <SelectItem value="full">Full Review</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        
        <DialogFooter>
          <Button variant="outline" @click="showDialog = false">
            {{ $t('common.cancel') }}
          </Button>
          <Button @click="performReview" :disabled="loading">
            <Loader2 v-if="loading" class="w-4 h-4 mr-2 animate-spin" />
            {{ $t('agent.performReview') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    
    <!-- 显示审查结果 -->
    <div v-if="reviewResult" class="mt-4">
      <Alert :variant="getAlertVariant(reviewResult.risk_level)">
        <AlertTriangle class="h-4 w-4" />
        <AlertTitle>
          {{ $t('agent.tenthManReview') }} - 
          {{ getRiskLabel(reviewResult.risk_level) }}
        </AlertTitle>
        <AlertDescription>
          <div v-html="formatCritique(reviewResult.critique)" />
        </AlertDescription>
      </Alert>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { Shield, Loader2, AlertTriangle } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select';
import { Alert, AlertTitle, AlertDescription } from '@/components/ui/alert';
import { useToast } from '@/components/ui/toast';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const { toast } = useToast();

const showDialog = ref(false);
const loading = ref(false);
const reviewContent = ref('');
const contextDescription = ref('');
const reviewType = ref('quick');
const reviewResult = ref<any>(null);

// 假设从父组件传入或从 store 获取
const props = defineProps<{
  executionId: string;
}>();

async function performReview() {
  if (!reviewContent.value.trim()) {
    toast({
      title: 'Error',
      description: 'Please enter content to review',
      variant: 'destructive'
    });
    return;
  }
  
  loading.value = true;
  reviewResult.value = null;
  
  try {
    const result = await invoke('unified_execute_tool', {
      tool_name: 'tenth_man_review',
      inputs: {
        execution_id: props.executionId,
        content_to_review: reviewContent.value,
        context_description: contextDescription.value || undefined,
        review_type: reviewType.value
      }
    });
    
    if (result.success && result.output) {
      reviewResult.value = result.output;
      showDialog.value = false;
      
      toast({
        title: 'Review Complete',
        description: `Risk level: ${result.output.risk_level}`,
        variant: result.output.risk_level === 'none' ? 'success' : 'warning'
      });
    } else {
      throw new Error(result.error || 'Review failed');
    }
  } catch (error) {
    console.error('Review failed:', error);
    toast({
      title: 'Error',
      description: 'Failed to perform review',
      variant: 'destructive'
    });
  } finally {
    loading.value = false;
  }
}

function getAlertVariant(level: string): string {
  if (level === 'none') return 'default';
  if (level === 'low') return 'default';
  if (level === 'medium') return 'warning';
  return 'destructive';
}

function getRiskLabel(level: string): string {
  return t(`agent.riskLevel${level.charAt(0).toUpperCase() + level.slice(1)}`);
}

function formatCritique(critique: string): string {
  return critique
    .replace(/\*\*\[([^\]]+)\]\*\*/g, '<strong class="text-red-600">[$1]</strong>')
    .replace(/\*\*(\d+\. [^:]+):\*\*/g, '<strong class="text-blue-600">$1:</strong>')
    .replace(/\n/g, '<br>');
}
</script>
```

## TypeScript 类型定义

```typescript
// types/tenth-man.ts

export interface TenthManToolInput {
  execution_id: string;
  content_to_review: string;
  context_description?: string;
  review_type: 'quick' | 'full';
}

export interface TenthManToolOutput {
  success: boolean;
  critique: string | null;
  risk_level: 'none' | 'low' | 'medium' | 'high' | 'critical';
  message: string;
}

export interface TenthManCritiqueEvent {
  execution_id: string;
  critique: string;
  message_id: string;
  trigger: 'final_review' | 'tool_call' | 'conclusion_detected';
  mode: 'system_enforced' | 'llm_requested';
  timestamp?: number;
}

export interface ToolExecutionResult {
  success: boolean;
  output: TenthManToolOutput | null;
  error: string | null;
  execution_time_ms: number;
}
```

## Composable 示例

```typescript
// composables/useTenthMan.ts

import { ref, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';
import type { TenthManCritiqueEvent, TenthManToolInput, ToolExecutionResult } from '@/types/tenth-man';

export function useTenthMan(executionId: string) {
  const critiques = ref<TenthManCritiqueEvent[]>([]);
  const latestCritique = ref<TenthManCritiqueEvent | null>(null);
  
  let unlistenCritique: UnlistenFn | null = null;
  let unlistenWarning: UnlistenFn | null = null;
  
  onMounted(async () => {
    // 监听系统强制审查
    unlistenCritique = await listen<TenthManCritiqueEvent>(
      'agent:tenth_man_critique',
      (event) => {
        if (event.payload.execution_id === executionId) {
          const critique = {
            ...event.payload,
            timestamp: Date.now()
          };
          critiques.value.push(critique);
          latestCritique.value = critique;
        }
      }
    );
    
    // 监听 LLM 主动调用
    unlistenWarning = await listen<any>(
      'agent:tenth_man_warning',
      (event) => {
        if (event.payload.execution_id === executionId) {
          // 解析工具结果
          try {
            const result = JSON.parse(event.payload.result);
            const critique: TenthManCritiqueEvent = {
              execution_id: executionId,
              critique: result.critique,
              message_id: event.payload.tool_call_id,
              trigger: 'tool_call',
              mode: 'llm_requested',
              timestamp: Date.now()
            };
            critiques.value.push(critique);
            latestCritique.value = critique;
          } catch (error) {
            console.error('Failed to parse Tenth Man warning:', error);
          }
        }
      }
    );
  });
  
  onUnmounted(() => {
    if (unlistenCritique) unlistenCritique();
    if (unlistenWarning) unlistenWarning();
  });
  
  async function requestReview(input: Omit<TenthManToolInput, 'execution_id'>) {
    try {
      const result = await invoke<ToolExecutionResult>('unified_execute_tool', {
        tool_name: 'tenth_man_review',
        inputs: {
          execution_id: executionId,
          ...input
        }
      });
      
      return result;
    } catch (error) {
      console.error('Failed to request Tenth Man review:', error);
      throw error;
    }
  }
  
  return {
    critiques,
    latestCritique,
    requestReview
  };
}
```

## 使用示例

```vue
<script setup lang="ts">
import { useTenthMan } from '@/composables/useTenthMan';

const props = defineProps<{
  executionId: string;
}>();

const { critiques, latestCritique, requestReview } = useTenthMan(props.executionId);

// 手动触发审查
async function handleManualReview() {
  await requestReview({
    content_to_review: 'My current plan is...',
    context_description: 'Security testing plan',
    review_type: 'full'
  });
}
</script>

<template>
  <div>
    <!-- 显示最新审查 -->
    <TenthManCritique v-if="latestCritique" :critique="latestCritique" />
    
    <!-- 显示所有审查历史 -->
    <div v-for="critique in critiques" :key="critique.message_id">
      <TenthManCritique :critique="critique" />
    </div>
    
    <!-- 手动触发按钮 -->
    <Button @click="handleManualReview">Request Review</Button>
  </div>
</template>
```
