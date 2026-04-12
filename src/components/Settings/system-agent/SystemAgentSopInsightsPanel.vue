<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 flex items-start justify-between gap-3">
      <div>
        <div class="font-semibold text-sm">SOP 命中统计</div>
        <div class="text-xs text-base-content/60 mt-1">
          基于最近发现中的 Agent 上下文 evidence 聚合，并支持维护当前 Agent 的 SOP 清单。
        </div>
      </div>
      <button class="btn btn-sm btn-primary shrink-0" type="button" @click="openCreateDialog()">
        新增 SOP
      </button>
    </div>

    <div v-if="displayEntries.length === 0" class="p-4 text-sm text-base-content/60">
      当前还没有 SOP 定义，也没有命中数据。可以先新增 SOP，后续命中会按 SOP ID 聚合。
    </div>

    <div v-else class="p-4 space-y-3">
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">SOP 数量</div>
          <div class="stat-value text-lg">{{ definitions.length }}</div>
          <div class="stat-desc">当前 Agent</div>
        </div>
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">总命中次数</div>
          <div class="stat-value text-lg">{{ totalHits }}</div>
          <div class="stat-desc">来自 system_agent_context</div>
        </div>
        <div class="stat bg-base-200/60 rounded-lg border border-base-300">
          <div class="stat-title text-xs">Top SOP</div>
          <div class="stat-value text-sm">{{ topEntry?.name || topEntry?.id || '-' }}</div>
          <div class="stat-desc">{{ topEntry?.hits || 0 }} 次</div>
        </div>
      </div>

      <div class="space-y-2">
        <div
          v-for="entry in displayEntries"
          :key="entry.id"
          class="bg-base-200/60 border border-base-300 rounded-lg p-3"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <div class="font-medium text-sm break-words">{{ entry.name || entry.id }}</div>
                <div class="badge badge-sm" :class="entry.isRegistered ? 'badge-success' : 'badge-warning'">
                  {{ entry.isRegistered ? '已登记' : '待登记' }}
                </div>
              </div>
              <div class="text-xs text-base-content/50 break-all mt-1">ID: {{ entry.id }}</div>
              <div class="text-xs text-base-content/60 break-words mt-1">
                {{ entry.description || '无描述' }}
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <div class="badge badge-accent badge-sm">{{ entry.hits }} 次</div>
              <button
                v-if="entry.isRegistered"
                class="btn btn-xs btn-outline"
                type="button"
                @click="openEditDialog(entry.id)"
              >
                编辑
              </button>
              <button
                v-else
                class="btn btn-xs btn-outline"
                type="button"
                @click="openCreateDialog(entry)"
              >
                登记
              </button>
              <button
                v-if="entry.isRegistered"
                class="btn btn-xs btn-outline btn-error"
                type="button"
                @click="removeDefinition(entry.id)"
              >
                删除
              </button>
            </div>
          </div>

          <div v-if="entry.procedure" class="mt-3 rounded border border-base-300 bg-base-100/70 p-3">
            <div class="text-xs font-semibold text-base-content/70 mb-2">SOP 步骤</div>
            <pre class="text-xs whitespace-pre-wrap break-words font-mono">{{ entry.procedure }}</pre>
          </div>

          <div v-if="entry.sampleReasons.length > 0" class="text-xs text-base-content/70 mt-2">
            <span class="font-semibold">示例理由：</span>
            {{ entry.sampleReasons.join('；') }}
          </div>
        </div>
      </div>
    </div>

    <AppDialog ref="dialogRef" class="modal">
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-4">
          {{ editingIndex === -1 ? '新增 SOP' : '编辑 SOP' }}
        </h3>

        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">SOP ID</span>
            </label>
            <input
              v-model="editingDefinition.id"
              type="text"
              class="input input-bordered"
              placeholder="idor-triage"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">名称</span>
            </label>
            <input
              v-model="editingDefinition.name"
              type="text"
              class="input input-bordered"
              placeholder="IDOR 研判流程"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">描述</span>
            </label>
            <input
              v-model="editingDefinition.description"
              type="text"
              class="input input-bordered"
              placeholder="简述该 SOP 适用场景和目标"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">步骤</span>
            </label>
            <textarea
              v-model="editingDefinition.procedure"
              class="textarea textarea-bordered min-h-40"
              placeholder="1. 确认目标对象标识符&#10;2. 对比不同身份上下文&#10;3. 验证越权读写路径"
            ></textarea>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-ghost" type="button" @click="closeDialog">取消</button>
          <button class="btn btn-primary" type="button" @click="saveDialog">保存</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </AppDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { dialog } from '@/composables/useDialog'
import type {
  SystemAgentFindingSummary,
  SystemAgentSopDefinition,
} from '../systemAgentSettingsSupport'
import {
  createEmptySystemAgentSopDefinition,
  loadLegacySystemAgentSopDefinitions,
  normalizeSystemAgentSopDefinitions,
} from './systemAgentSopCatalog'

const props = defineProps<{
  profileId: string
  findings: SystemAgentFindingSummary[]
  windowStart: number
  definitions: SystemAgentSopDefinition[]
}>()

const emit = defineEmits<{
  'update:definitions': [value: SystemAgentSopDefinition[]]
}>()

interface SopHitEntry {
  id: string
  name?: string
  description?: string
  hits: number
  sampleReasons: string[]
}

interface DisplaySopEntry extends SopHitEntry {
  procedure: string
  isRegistered: boolean
  updatedAt?: string
}

const dialogRef = ref<HTMLDialogElement | null>(null)
const editingIndex = ref(-1)
const editingDefinition = ref<SystemAgentSopDefinition>(createEmptySystemAgentSopDefinition())
const migratedLegacyProfileIds = new Set<string>()

const definitions = computed(() =>
  normalizeSystemAgentSopDefinitions(props.definitions),
)

watch(
  () => [props.profileId, props.definitions] as const,
  profileId => {
    if (!profileId[0] || migratedLegacyProfileIds.has(profileId[0])) {
      return
    }
    if (normalizeSystemAgentSopDefinitions(profileId[1]).length > 0) {
      migratedLegacyProfileIds.add(profileId[0])
      return
    }

    const legacyDefinitions = loadLegacySystemAgentSopDefinitions(profileId[0])
    migratedLegacyProfileIds.add(profileId[0])
    if (legacyDefinitions.length === 0) {
      return
    }

    emitDefinitions(legacyDefinitions)
    dialog.toast.success('已导入当前 Agent 的本地 SOP 定义')
  },
  { immediate: true },
)

const windowedFindings = computed(() => {
  return props.findings.filter(finding => {
    const seenAt = new Date(finding.last_seen_at).getTime()
    return Number.isFinite(seenAt) && seenAt >= props.windowStart
  })
})

const sopHits = computed<SopHitEntry[]>(() => {
  const entries = new Map<string, SopHitEntry>()

  for (const finding of windowedFindings.value) {
    for (const evidence of finding.evidence ?? []) {
      if (evidence.location !== 'system_agent_context' || !evidence.request_body) {
        continue
      }
      const payload = parseJson(evidence.request_body)
      const items = Array.isArray(payload?.logicSopContext) && payload.logicSopContext.length > 0
        ? payload.logicSopContext
        : Array.isArray(payload?.logicSkillContext)
          ? payload.logicSkillContext
          : []
      for (const item of items) {
        const id = typeof item?.id === 'string' ? item.id.trim() : ''
        if (!id) continue
        const existing = entries.get(id) ?? {
          id,
          name: typeof item?.name === 'string' ? item.name : undefined,
          description: typeof item?.description === 'string' ? item.description : undefined,
          hits: 0,
          sampleReasons: [],
        }
        existing.hits += 1
        const reasons = Array.isArray(item?.reasons)
          ? item.reasons.filter((reason: unknown): reason is string => typeof reason === 'string')
          : []
        for (const reason of reasons) {
          if (!existing.sampleReasons.includes(reason) && existing.sampleReasons.length < 3) {
            existing.sampleReasons.push(reason)
          }
        }
        entries.set(id, existing)
      }
    }
  }

  return Array.from(entries.values()).sort((left, right) => right.hits - left.hits)
})

const displayEntries = computed<DisplaySopEntry[]>(() => {
  const hitMap = new Map(sopHits.value.map(item => [item.id, item]))

  const registered = definitions.value
    .map(definition => {
      const hit = hitMap.get(definition.id)
      return {
        id: definition.id,
        name: definition.name || hit?.name || definition.id,
        description: definition.description || hit?.description,
        procedure: definition.procedure,
        hits: hit?.hits || 0,
        sampleReasons: hit?.sampleReasons || [],
        isRegistered: true,
        updatedAt: definition.updatedAt,
      }
    })
    .sort((left, right) => {
      if (right.hits !== left.hits) return right.hits - left.hits
      return (right.updatedAt || '').localeCompare(left.updatedAt || '')
    })

  const discoveredOnly = sopHits.value
    .filter(item => !definitions.value.some(definition => definition.id === item.id))
    .map(item => ({
      ...item,
      procedure: '',
      isRegistered: false,
      updatedAt: undefined,
    }))

  return [...registered, ...discoveredOnly].sort((left, right) => {
    if (right.hits !== left.hits) return right.hits - left.hits
    if (left.isRegistered !== right.isRegistered) return left.isRegistered ? -1 : 1
    return (right.updatedAt || '').localeCompare(left.updatedAt || '')
  })
})

const totalHits = computed(() => displayEntries.value.reduce((sum, item) => sum + item.hits, 0))
const topEntry = computed(() => displayEntries.value[0] ?? null)

function openCreateDialog(seed?: Partial<DisplaySopEntry>) {
  editingIndex.value = -1
  editingDefinition.value = {
    ...createEmptySystemAgentSopDefinition(),
    id: seed?.id || '',
    name: seed?.name || '',
    description: seed?.description || '',
    procedure: seed?.procedure || '',
  }
  dialogRef.value?.showModal()
}

function openEditDialog(id: string) {
  const index = definitions.value.findIndex(item => item.id === id)
  if (index === -1) {
    return
  }
  editingIndex.value = index
  editingDefinition.value = { ...definitions.value[index] }
  dialogRef.value?.showModal()
}

function closeDialog() {
  dialogRef.value?.close()
}

async function removeDefinition(id: string) {
  const target = definitions.value.find(item => item.id === id)
  if (!target) {
    return
  }

  const confirmed = await dialog.confirm(`确认删除 SOP「${target.name || target.id}」吗？`)
  if (!confirmed) {
    return
  }

  emitDefinitions(definitions.value.filter(item => item.id !== id))
  dialog.toast.success('SOP 已删除')
}

function saveDialog() {
  const id = editingDefinition.value.id.trim()
  const name = editingDefinition.value.name.trim()

  if (!id || !name) {
    dialog.toast.warning('SOP ID 和名称不能为空')
    return
  }

  const duplicatedIndex = definitions.value.findIndex((item, index) => {
    if (index === editingIndex.value) {
      return false
    }
    return item.id === id
  })
  if (duplicatedIndex !== -1) {
    dialog.toast.warning('SOP ID 已存在，请使用唯一 ID')
    return
  }

  const nextItem: SystemAgentSopDefinition = {
    id,
    name,
    description: editingDefinition.value.description.trim(),
    procedure: editingDefinition.value.procedure.trim(),
    updatedAt: new Date().toISOString(),
  }

  if (editingIndex.value === -1) {
    emitDefinitions([...definitions.value, nextItem])
    dialog.toast.success('SOP 已新增')
  } else {
    const nextDefinitions = [...definitions.value]
    nextDefinitions.splice(editingIndex.value, 1, nextItem)
    emitDefinitions(nextDefinitions)
    dialog.toast.success('SOP 已更新')
  }

  closeDialog()
}

function emitDefinitions(nextDefinitions: SystemAgentSopDefinition[]) {
  emit('update:definitions', normalizeSystemAgentSopDefinitions(nextDefinitions))
}

function parseJson(raw?: string | null) {
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}
</script>
