import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import type { VulnerabilityFilterState } from './vulnerabilitiesFilterPresets'

const STORAGE_KEY = 'security-center:vulnerability-filter-views:v1'

export interface SavedVulnerabilityFilterView {
  id: string
  name: string
  filters: VulnerabilityFilterState
  updatedAt: string
}

export function loadSavedVulnerabilityFilterViews(): SavedVulnerabilityFilterView[] {
  if (typeof localStorage === 'undefined') return []
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .map(item => normalizeSavedView(item))
      .filter((item): item is SavedVulnerabilityFilterView => !!item)
      .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
  } catch {
    return []
  }
}

export function saveSavedVulnerabilityFilterViews(views: SavedVulnerabilityFilterView[]) {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(STORAGE_KEY, JSON.stringify(views))
}

export function promptAndSaveVulnerabilityFilterView(params: {
  views: SavedVulnerabilityFilterView[]
  activeViewId: string | null
  filters: VulnerabilityFilterState
}): SavedVulnerabilityFilterView[] {
  const suggestedName =
    params.views.find(view => view.id === params.activeViewId)?.name || '新筛选视图'
  const nextName = window.prompt('输入视图名称', suggestedName)?.trim()
  if (!nextName) return params.views

  const updated = params.activeViewId
    ? params.views.map(view => {
        if (view.id !== params.activeViewId) return view
        return { ...updateSavedVulnerabilityFilterView(view, params.filters), name: nextName }
      })
    : [createSavedVulnerabilityFilterView(nextName, params.filters), ...params.views]

  return sortSavedViews(updated)
}

export function confirmAndDeleteSavedVulnerabilityFilterView(params: {
  views: SavedVulnerabilityFilterView[]
  viewId: string | null
}): SavedVulnerabilityFilterView[] {
  if (!params.viewId) return params.views
  const target = params.views.find(view => view.id === params.viewId)
  if (!target) return params.views
  if (!window.confirm(`删除已保存视图“${target.name}”？`)) return params.views
  return params.views.filter(view => view.id !== params.viewId)
}

export async function exportSavedVulnerabilityFilterViews(views: SavedVulnerabilityFilterView[]) {
  const selected = await save({
    title: '导出已保存筛选视图',
    defaultPath: `vulnerability-filter-views-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.json`,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  if (!selected) return false

  await writeTextFile(
    selected,
    JSON.stringify(
      {
        format_version: '1.0',
        exported_at: new Date().toISOString(),
        views: sortSavedViews(views),
      },
      null,
      2,
    ),
  )
  return true
}

export async function importSavedVulnerabilityFilterViews(): Promise<SavedVulnerabilityFilterView[]> {
  const selected = await open({
    directory: false,
    multiple: false,
    title: '导入已保存筛选视图',
    filters: [{ name: 'JSON', extensions: ['json'] }],
  })
  if (!selected) return []

  const content = await readTextFile(selected as string)
  const parsed = JSON.parse(content)
  return decodeSavedViewsImportPayload(parsed)
}

export function mergeSavedVulnerabilityFilterViews(
  existing: SavedVulnerabilityFilterView[],
  incoming: SavedVulnerabilityFilterView[],
) {
  const merged = new Map(existing.map(view => [view.id, view] as const))
  for (const view of incoming) {
    merged.set(view.id, view)
  }
  return sortSavedViews(Array.from(merged.values()))
}

export function createSavedVulnerabilityFilterView(
  name: string,
  filters: VulnerabilityFilterState,
): SavedVulnerabilityFilterView {
  const now = new Date().toISOString()
  return {
    id: `vf-${Math.random().toString(36).slice(2, 10)}`,
    name: name.trim(),
    filters: cloneFilterState(filters),
    updatedAt: now,
  }
}

export function updateSavedVulnerabilityFilterView(
  view: SavedVulnerabilityFilterView,
  filters: VulnerabilityFilterState,
): SavedVulnerabilityFilterView {
  return {
    ...view,
    filters: cloneFilterState(filters),
    updatedAt: new Date().toISOString(),
  }
}

export function cloneFilterState(filters: VulnerabilityFilterState): VulnerabilityFilterState {
  return {
    severity: filters.severity,
    status: filters.status,
    lifecycleView: filters.lifecycleView,
    search: filters.search,
    semanticSource: filters.semanticSource,
    hypothesisRiskType: filters.hypothesisRiskType,
    hypothesisRiskTypes: [...filters.hypothesisRiskTypes],
  }
}

export function detectActiveSavedViewId(
  filters: VulnerabilityFilterState,
  views: SavedVulnerabilityFilterView[],
): string | null {
  const target = JSON.stringify(cloneFilterState(filters))
  const matched = views.find(view => JSON.stringify(cloneFilterState(view.filters)) === target)
  return matched?.id || null
}

function decodeSavedViewsImportPayload(value: unknown): SavedVulnerabilityFilterView[] {
  const items = Array.isArray(value)
    ? value
    : value && typeof value === 'object' && Array.isArray((value as { views?: unknown[] }).views)
      ? (value as { views: unknown[] }).views
      : []
  return sortSavedViews(
    items.map(item => normalizeSavedView(item)).filter((item): item is SavedVulnerabilityFilterView => !!item),
  )
}

function sortSavedViews(views: SavedVulnerabilityFilterView[]) {
  return [...views].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
}

function normalizeSavedView(value: unknown): SavedVulnerabilityFilterView | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const item = value as Partial<SavedVulnerabilityFilterView>
  const id = typeof item.id === 'string' ? item.id.trim() : ''
  const name = typeof item.name === 'string' ? item.name.trim() : ''
  if (!id || !name) return null
  return {
    id,
    name,
    filters: normalizeFilterState(item.filters),
    updatedAt:
      typeof item.updatedAt === 'string' && item.updatedAt ? item.updatedAt : new Date().toISOString(),
  }
}

function normalizeFilterState(value: unknown): VulnerabilityFilterState {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {
      severity: '',
      status: '',
      lifecycleView: 'formal',
      search: '',
      semanticSource: '',
      hypothesisRiskType: '',
      hypothesisRiskTypes: [],
    }
  }
  const item = value as Partial<VulnerabilityFilterState>
  return {
    severity: typeof item.severity === 'string' ? item.severity : '',
    status: typeof item.status === 'string' ? item.status : '',
    lifecycleView: isLifecycleView(item.lifecycleView) ? item.lifecycleView : 'formal',
    search: typeof item.search === 'string' ? item.search : '',
    semanticSource: typeof item.semanticSource === 'string' ? item.semanticSource : '',
    hypothesisRiskType: typeof item.hypothesisRiskType === 'string' ? item.hypothesisRiskType : '',
    hypothesisRiskTypes: Array.isArray(item.hypothesisRiskTypes)
      ? item.hypothesisRiskTypes.filter((entry): entry is string => typeof entry === 'string' && !!entry)
      : [],
  }
}

function isLifecycleView(value: unknown): value is VulnerabilityFilterState['lifecycleView'] {
  return (
    value === 'formal' ||
    value === 'candidate' ||
    value === 'verified' ||
    value === 'false_positive' ||
    value === 'all'
  )
}
