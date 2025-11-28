<template>
  <div class="p-4 space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">运行历史</h1>
      <div class="flex gap-2">
        <button class="btn btn-primary btn-sm" @click="refresh_runs">刷新</button>
      </div>
    </div>

    <div class="card bg-base-100 shadow-xl">
      <div class="card-body p-4">
        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>执行ID</th>
                <th>工作流</th>
                <th>版本</th>
                <th>状态</th>
                <th>开始时间</th>
                <th>耗时</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in runs" :key="r.execution_id">
                <td class="font-mono">{{ r.execution_id }}</td>
                <td>{{ r.workflow_name }}</td>
                <td>{{ r.version }}</td>
                <td>
                  <span class="badge" :class="status_badge(r.status)">{{ r.status }}</span>
                </td>
                <td>{{ r.started_at }}</td>
                <td>{{ r.duration }}</td>
                <td>
                  <button class="btn btn-ghost btn-xs">详情</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowEvents } from '@/composables/useWorkflowEvents'

const runs = ref<Array<any>>([])

const refresh_runs = async () => {
  try {
    const list = await invoke<Array<any>>('list_workflow_runs')
    runs.value = list.map(item => ({
      execution_id: item.execution_id,
      workflow_name: item.workflow_name,
      version: item.version,
      status: item.status,
      started_at: format_dt(item.started_at),
      duration: format_duration(item.completed_at, item.started_at),
      progress: item.progress
    }))
  } catch (e) {
    console.error('list_workflow_runs error', e)
  }
}

const status_badge = (s: string) => {
  if (s === 'completed') return 'badge-success'
  if (s === 'running') return 'badge-warning'
  if (s === 'failed') return 'badge-error'
  return 'badge-ghost'
}

const format_dt = (d: any) => {
  try { return new Date(d).toLocaleString() } catch { return '' }
}
const format_duration = (end: any, start: any) => {
  try { const ms = new Date(end).getTime() - new Date(start).getTime(); if (!isFinite(ms) || ms <= 0) return ''; return Math.round(ms/1000) + 's' } catch { return '' }
}

const wf_events = useWorkflowEvents()
const setup_event_listeners = async () => {
  await wf_events.on_run_start(async () => { await refresh_runs() })
  await wf_events.on_run_complete(async () => { await refresh_runs() })
  await wf_events.on_progress(async () => { await refresh_runs() })
}

onMounted(async () => { await refresh_runs(); await setup_event_listeners() })
onUnmounted(() => { wf_events.unsubscribe_all() })
</script>

<style scoped>
</style>
