<template>
  <div class="flex h-full min-h-0 flex-col bg-base-100">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex items-center justify-between gap-2">
        <div class="min-w-0">
          <div class="flex items-center gap-2 text-sm font-semibold">
            <i class="fas fa-folder-tree text-primary"></i>
            <span>工作目录</span>
          </div>
          <div class="mt-1 truncate font-mono text-[11px] text-base-content/55" :title="rootLabel">
            {{ rootLabel }}
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-1">
          <button class="btn btn-ghost btn-xs" title="刷新" @click="reloadCurrentDirectory">
            <i class="fas fa-rotate-right" :class="{ 'animate-spin': loading }"></i>
          </button>
          <button class="btn btn-ghost btn-xs" title="关闭" @click="$emit('close')">
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>
      <div class="mt-3">
        <div class="flex gap-2">
          <input
            v-model.trim="filterText"
            class="input input-bordered input-xs min-w-0 flex-1"
            type="search"
            placeholder="过滤当前目录"
          />
          <button class="btn btn-outline btn-xs" title="新建文件" @click="openCreateDialog(false)">
            <i class="fas fa-file-circle-plus"></i>
          </button>
          <button class="btn btn-outline btn-xs" title="新建目录" @click="openCreateDialog(true)">
            <i class="fas fa-folder-plus"></i>
          </button>
        </div>
      </div>
    </div>

    <div v-if="!workingDirectory.trim()" class="p-4 text-sm text-base-content/60">
      当前会话未配置工作目录。
    </div>

    <div v-else class="flex min-h-0 flex-1 flex-col">
      <div class="border-b border-base-300 px-3 py-2">
        <div class="flex min-w-0 items-center gap-2">
          <button
            class="btn btn-ghost btn-xs"
            :disabled="!canGoParent || loading"
            title="上级目录"
            @click="goParent"
          >
            <i class="fas fa-arrow-up"></i>
          </button>
          <span class="truncate font-mono text-[11px] text-base-content/65" :title="displayPath">
            {{ displayPath }}
          </span>
        </div>
      </div>

      <div v-if="error" class="border-b border-error/20 bg-error/10 px-3 py-2 text-xs text-error">
        {{ error }}
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto">
        <button
          v-for="row in visibleRows"
          :key="row.entry.relative_path"
          class="group flex w-full items-center gap-2 border-b border-base-300/50 px-3 py-2 text-left hover:bg-base-200/70"
          :class="selectedPath === row.entry.relative_path ? 'bg-primary/10 text-primary' : 'text-base-content'"
          :style="{ paddingLeft: `${12 + row.depth * 16}px` }"
          @click="openEntry(row.entry)"
        >
          <span
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 p-0"
            :class="row.entry.is_directory ? '' : 'invisible'"
            @click.stop="toggleDirectory(row.entry)"
          >
            <i
              class="fas text-[10px]"
              :class="isDirectoryLoading(row.entry.relative_path) ? 'fa-spinner fa-spin' : (isExpanded(row.entry.relative_path) ? 'fa-chevron-down' : 'fa-chevron-right')"
            ></i>
          </span>
          <i
            class="fas w-4 shrink-0 text-center text-xs"
            :class="row.entry.is_directory ? 'fa-folder text-warning' : 'fa-file-lines text-base-content/55'"
          ></i>
          <div class="min-w-0 flex-1">
            <div class="truncate text-xs font-medium" :title="row.entry.name">{{ row.entry.name }}</div>
            <div class="mt-0.5 flex min-w-0 items-center gap-2 text-[10px] text-base-content/45">
              <span v-if="!row.entry.is_directory">{{ formatBytes(row.entry.size) }}</span>
              <span v-if="row.entry.modified_ms">{{ formatModified(row.entry.modified_ms) }}</span>
            </div>
          </div>
          <button class="btn btn-ghost btn-xs h-6 min-h-6 px-1 opacity-0 focus:opacity-100 group-hover:opacity-100" title="复制相对路径" @click.stop="copyRelativePath(row.entry)">
            <i class="fas fa-copy"></i>
          </button>
          <button class="btn btn-ghost btn-xs h-6 min-h-6 px-1 opacity-0 focus:opacity-100 group-hover:opacity-100" title="重命名" @click.stop="openRenameDialog(row.entry)">
            <i class="fas fa-pen-to-square"></i>
          </button>
          <button class="btn btn-ghost btn-xs h-6 min-h-6 px-1 text-error opacity-0 focus:opacity-100 group-hover:opacity-100" title="删除" @click.stop="openDeleteDialog(row.entry)">
            <i class="fas fa-trash"></i>
          </button>
        </button>

        <div v-if="loading" class="p-4 text-xs text-base-content/50">正在加载...</div>
        <div v-else-if="visibleRows.length === 0" class="p-4 text-xs text-base-content/50">
          当前目录没有可显示的文件。
        </div>
        <div v-if="truncated" class="border-t border-warning/20 bg-warning/10 px-3 py-2 text-xs text-warning">
          当前目录内容较多，仅显示前 500 项。
        </div>
      </div>

      <div class="flex shrink-0 flex-col border-t border-base-300" :style="previewHeightStyle">
        <div
          class="group flex h-2 shrink-0 cursor-row-resize items-center justify-center bg-base-100 hover:bg-primary/10"
          title="拖动调整预览区高度"
          @mousedown="startPreviewResize"
        >
          <div class="h-0.5 w-12 rounded-full bg-base-300 group-hover:bg-primary/50"></div>
        </div>
        <div class="flex min-h-9 items-center justify-between gap-2 px-3 py-2">
          <div class="min-w-0 truncate font-mono text-[11px] text-base-content/65" :title="selectedPath">
            {{ selectedPath || '选择文件后预览' }}
          </div>
          <div class="flex shrink-0 items-center gap-1">
            <button
              v-if="isMarkdownPreview && !editMode"
              class="btn btn-ghost btn-xs"
              :title="previewRenderMode === 'rendered' ? '查看源码' : '渲染 Markdown'"
              @click="previewRenderMode = previewRenderMode === 'rendered' ? 'source' : 'rendered'"
            >
              <i class="fas" :class="previewRenderMode === 'rendered' ? 'fa-code' : 'fa-eye'"></i>
            </button>
            <span v-if="externalFileChanged" class="badge badge-info badge-xs">已更新</span>
            <span v-if="preview?.truncated" class="badge badge-warning badge-xs">截断</span>
            <button
              v-if="selectedPath && !editMode"
              class="btn btn-ghost btn-xs"
              :disabled="openingFile"
              title="用系统默认应用打开"
              @click="openSelectedFile"
            >
              <i class="fas" :class="openingFile ? 'fa-spinner fa-spin' : 'fa-up-right-from-square'"></i>
            </button>
            <button
              v-if="preview && !editMode"
              class="btn btn-ghost btn-xs"
              :disabled="!canEditPreview"
              title="编辑文件"
              @click="beginEdit"
            >
              <i class="fas fa-pen"></i>
            </button>
            <template v-if="editMode">
              <button class="btn btn-ghost btn-xs" :disabled="saving" title="取消编辑" @click="cancelEdit">
                <i class="fas fa-xmark"></i>
              </button>
              <button
                class="btn btn-primary btn-xs"
                :disabled="!canSaveDraft"
                title="保存文件"
                @click="saveEdit"
              >
                <i class="fas" :class="saving ? 'fa-spinner fa-spin' : 'fa-floppy-disk'"></i>
              </button>
            </template>
          </div>
        </div>
        <div v-if="previewError" class="px-3 pb-2 text-xs text-error">{{ previewError }}</div>
        <div
          v-if="externalFileChanged"
          class="mx-3 mb-2 flex items-center justify-between gap-2 rounded border border-info/20 bg-info/10 px-2 py-1.5 text-xs text-info"
        >
          <span>当前文件已被外部修改。</span>
          <button class="btn btn-info btn-xs" :disabled="previewLoading || hasDraftChange" @click="refreshSelectedFile">
            刷新
          </button>
        </div>
        <WorkspaceCodeEditor
          v-if="preview && editMode"
          v-model="draftContent"
          class="min-h-0 flex-1"
          :file-path="selectedPath"
        />
        <img
          v-else-if="imagePreviewUrl"
          :src="imagePreviewUrl"
          class="min-h-0 flex-1 object-contain bg-base-200/60"
          alt=""
        />
        <div
          v-else-if="preview && markdownPreviewHtml && previewRenderMode === 'rendered'"
          class="prose prose-sm min-h-0 max-w-none flex-1 overflow-auto bg-base-200/60 px-4 py-3 text-base-content"
          v-html="markdownPreviewHtml"
        ></div>
        <pre
          v-else-if="preview"
          class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words bg-base-200/60 px-3 py-2 font-mono text-sm leading-6 text-base-content/80"
        >{{ previewText }}</pre>
        <div v-else class="flex min-h-0 flex-1 items-center justify-center px-3 text-xs text-base-content/45">
          {{ previewLoading ? '正在读取...' : '暂未选择文件' }}
        </div>
      </div>
    </div>

    <div v-if="createDialog.open" class="modal modal-open">
      <div class="modal-box max-w-md">
        <h3 class="text-base font-semibold">{{ createDialog.isDirectory ? '新建目录' : '新建文件' }}</h3>
        <input v-model.trim="createDialog.name" class="input input-bordered mt-4 w-full" placeholder="名称" />
        <div v-if="operationError" class="mt-3 text-sm text-error">{{ operationError }}</div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeOperationDialogs">取消</button>
          <button class="btn btn-primary" :disabled="operationBusy || !createDialog.name" @click="submitCreateEntry">创建</button>
        </div>
      </div>
    </div>

    <div v-if="renameDialog.open" class="modal modal-open">
      <div class="modal-box max-w-md">
        <h3 class="text-base font-semibold">重命名</h3>
        <div class="mt-2 truncate font-mono text-xs text-base-content/60">{{ renameDialog.relativePath }}</div>
        <input v-model.trim="renameDialog.name" class="input input-bordered mt-4 w-full" placeholder="新名称" />
        <div v-if="operationError" class="mt-3 text-sm text-error">{{ operationError }}</div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeOperationDialogs">取消</button>
          <button class="btn btn-primary" :disabled="operationBusy || !renameDialog.name" @click="submitRenameEntry">保存</button>
        </div>
      </div>
    </div>

    <div v-if="deleteDialog.open" class="modal modal-open">
      <div class="modal-box max-w-md border border-error/20">
        <h3 class="text-base font-semibold text-error">删除条目</h3>
        <p class="mt-3 text-sm text-base-content/70">确认删除这个文件或空目录？</p>
        <div class="mt-2 break-all rounded bg-base-200 px-3 py-2 font-mono text-xs">{{ deleteDialog.relativePath }}</div>
        <div v-if="operationError" class="mt-3 text-sm text-error">{{ operationError }}</div>
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeOperationDialogs">取消</button>
          <button class="btn btn-error" :disabled="operationBusy" @click="submitDeleteEntry">删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import WorkspaceCodeEditor from './WorkspaceCodeEditor.vue'

interface WorkspaceEntry {
  name: string
  relative_path: string
  is_directory: boolean
  size?: number | null
  modified_ms?: number | null
}

interface WorkspaceEntriesResponse {
  root: string
  relative_path: string
  parent_relative_path?: string | null
  entries: WorkspaceEntry[]
  truncated: boolean
}

interface WorkingDirectoryFilePreview {
  id: string
  path: string
  relative_path: string
  preview: string
  truncated: boolean
  size: number
  sha256: string
  modified_ms?: number | null
}

interface WorkingDirectoryWatchResponse {
  watcher_id: string
  root: string
  relative_path: string
  is_directory: boolean
}

interface WorkingDirectoryChangeEvent {
  watcher_id: string
  root: string
  relative_path: string
  is_directory: boolean
}

interface WorkingDirectoryOpenTarget {
  path: string
  relative_path: string
}

interface WorkingDirectoryMutationResponse {
  relative_path: string
}

interface TreeRow {
  entry: WorkspaceEntry
  depth: number
}

const props = defineProps<{
  conversationId: string | null
  workingDirectory: string
}>()

defineEmits<{
  (e: 'close'): void
}>()

const currentRelativePath = ref('')
const parentRelativePath = ref<string | null>(null)
const entries = ref<WorkspaceEntry[]>([])
const filterText = ref('')
const loading = ref(false)
const error = ref('')
const root = ref('')
const truncated = ref(false)
const selectedPath = ref('')
const selectedOpenTargetPath = ref('')
const preview = ref<WorkingDirectoryFilePreview | null>(null)
const previewLoading = ref(false)
const previewError = ref('')
const previewRenderMode = ref<'rendered' | 'source'>('source')
const editMode = ref(false)
const draftContent = ref('')
const saving = ref(false)
const openingFile = ref(false)
const externalFileChanged = ref(false)
const childEntriesByDir = ref<Record<string, WorkspaceEntry[]>>({})
const expandedDirectories = ref<Set<string>>(new Set())
const loadingDirectories = ref<Set<string>>(new Set())
const operationBusy = ref(false)
const operationError = ref('')
const createDialog = ref({
  open: false,
  isDirectory: false,
  name: '',
})
const renameDialog = ref({
  open: false,
  relativePath: '',
  name: '',
})
const deleteDialog = ref({
  open: false,
  relativePath: '',
})
const directoryWatcherId = ref('')
const directoryWatcherPath = ref<string | null>(null)
const fileWatcherId = ref('')
const fileWatcherPath = ref<string | null>(null)
let loadSeq = 0
let previewSeq = 0
let unlistenWorkspaceChange: UnlistenFn | null = null
let directoryRefreshTimer: ReturnType<typeof setTimeout> | null = null
let localWriteGuardUntil = 0
let previewResizeStartY = 0
let previewResizeStartHeight = 0
let previousBodyUserSelect = ''

const PREVIEW_HEIGHT_STORAGE_KEY = 'sentinel:agent-workspace-preview-height'
const PREVIEW_MIN_HEIGHT = 180
const PREVIEW_DEFAULT_HEIGHT = 300
const PREVIEW_MAX_HEIGHT = 720

const rootLabel = computed(() => root.value || props.workingDirectory.trim() || '未配置工作目录')
const displayPath = computed(() => currentRelativePath.value || '.')
const canGoParent = computed(() => currentRelativePath.value.length > 0)
const entryMatchesFilter = (entry: WorkspaceEntry, query: string) => {
  return `${entry.name} ${entry.relative_path}`.toLowerCase().includes(query)
}
const visibleRows = computed<TreeRow[]>(() => {
  const query = filterText.value.trim().toLowerCase()
  const rows: TreeRow[] = []
  const appendRows = (items: WorkspaceEntry[], depth: number) => {
    items.forEach((entry) => {
      if (!query || entryMatchesFilter(entry, query)) {
        rows.push({ entry, depth })
      }
      if (entry.is_directory && expandedDirectories.value.has(entry.relative_path)) {
        appendRows(childEntriesByDir.value[entry.relative_path] || [], depth + 1)
      }
    })
  }
  appendRows(entries.value, 0)
  return rows
})
const canEditPreview = computed(() => !!preview.value && !preview.value.truncated && !previewLoading.value && !saving.value && !externalFileChanged.value)
const hasDraftChange = computed(() => editMode.value && draftContent.value !== (preview.value?.preview ?? ''))
const canSaveDraft = computed(() => hasDraftChange.value && !saving.value && !externalFileChanged.value)
const selectedExtension = computed(() => {
  const path = selectedPath.value.toLowerCase()
  const index = path.lastIndexOf('.')
  return index >= 0 ? path.slice(index + 1) : ''
})
const isMarkdownPreview = computed(() => !!preview.value && ['md', 'markdown', 'mdown'].includes(selectedExtension.value))
const imagePreviewUrl = computed(() => {
  if (!selectedOpenTargetPath.value || !['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico', 'avif'].includes(selectedExtension.value)) {
    return ''
  }
  return convertFileSrc(selectedOpenTargetPath.value)
})
const markdownPreviewHtml = computed(() => {
  if (!isMarkdownPreview.value || !preview.value) return ''
  const rendered = marked.parse(preview.value.preview, { async: false }) as string
  return DOMPurify.sanitize(rendered)
})
const jsonPreviewText = computed(() => {
  if (!preview.value || selectedExtension.value !== 'json') return ''
  try {
    return JSON.stringify(JSON.parse(preview.value.preview), null, 2)
  } catch {
    return ''
  }
})
const previewText = computed(() => jsonPreviewText.value || preview.value?.preview || '')
const previewHeight = ref(PREVIEW_DEFAULT_HEIGHT)
const previewHeightStyle = computed(() => ({
  height: `${previewHeight.value}px`,
}))

const getPreviewMaxHeight = () => {
  if (typeof window === 'undefined') return PREVIEW_MAX_HEIGHT
  return Math.max(PREVIEW_MIN_HEIGHT, Math.min(PREVIEW_MAX_HEIGHT, window.innerHeight - 260))
}

const clampPreviewHeight = (height: number) => {
  return Math.max(PREVIEW_MIN_HEIGHT, Math.min(getPreviewMaxHeight(), height))
}

const loadPreviewHeight = () => {
  if (typeof localStorage === 'undefined') return
  const raw = localStorage.getItem(PREVIEW_HEIGHT_STORAGE_KEY)
  const parsed = raw ? Number.parseInt(raw, 10) : PREVIEW_DEFAULT_HEIGHT
  previewHeight.value = clampPreviewHeight(Number.isFinite(parsed) ? parsed : PREVIEW_DEFAULT_HEIGHT)
}

const savePreviewHeight = () => {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(PREVIEW_HEIGHT_STORAGE_KEY, String(previewHeight.value))
}

const handlePreviewResizeMove = (event: MouseEvent) => {
  event.preventDefault()
  const delta = previewResizeStartY - event.clientY
  previewHeight.value = clampPreviewHeight(previewResizeStartHeight + delta)
}

const stopPreviewResize = () => {
  document.removeEventListener('mousemove', handlePreviewResizeMove)
  document.removeEventListener('mouseup', stopPreviewResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = previousBodyUserSelect
  savePreviewHeight()
}

const startPreviewResize = (event: MouseEvent) => {
  event.preventDefault()
  previewResizeStartY = event.clientY
  previewResizeStartHeight = previewHeight.value
  previousBodyUserSelect = document.body.style.userSelect
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', handlePreviewResizeMove)
  document.addEventListener('mouseup', stopPreviewResize)
}

const formatBytes = (size?: number | null) => {
  if (typeof size !== 'number') return ''
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

const formatModified = (value: number) => {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleString()
}

const clearPreview = () => {
  selectedPath.value = ''
  selectedOpenTargetPath.value = ''
  preview.value = null
  previewError.value = ''
  previewLoading.value = false
  previewRenderMode.value = 'source'
  editMode.value = false
  draftContent.value = ''
  saving.value = false
  externalFileChanged.value = false
  void stopFileWatcher()
}

const stopWatcher = async (watcherId: string) => {
  if (!watcherId) return
  try {
    await invoke('stop_working_directory_watch', { watcherId })
  } catch (err) {
    console.warn('[WorkspaceFilesPanel] Failed to stop watcher:', err)
  }
}

const stopDirectoryWatcher = async () => {
  const watcherId = directoryWatcherId.value
  directoryWatcherId.value = ''
  directoryWatcherPath.value = null
  await stopWatcher(watcherId)
}

const stopFileWatcher = async () => {
  const watcherId = fileWatcherId.value
  fileWatcherId.value = ''
  fileWatcherPath.value = null
  await stopWatcher(watcherId)
}

const ensureDirectoryWatcher = async (relativePath: string) => {
  const directory = props.workingDirectory.trim()
  if (!directory || directoryWatcherPath.value === relativePath) return
  await stopDirectoryWatcher()
  try {
    const response = await invoke<WorkingDirectoryWatchResponse>('watch_working_directory_path', {
      conversationId: props.conversationId,
      relativePath: relativePath || null,
      workingDirectory: directory,
      isDirectory: true,
    })
    directoryWatcherId.value = response.watcher_id
    directoryWatcherPath.value = response.relative_path
  } catch (err) {
    console.warn('[WorkspaceFilesPanel] Failed to watch directory:', err)
  }
}

const ensureFileWatcher = async (relativePath: string) => {
  const directory = props.workingDirectory.trim()
  if (!directory || fileWatcherPath.value === relativePath) return
  await stopFileWatcher()
  try {
    const response = await invoke<WorkingDirectoryWatchResponse>('watch_working_directory_path', {
      conversationId: props.conversationId,
      relativePath,
      workingDirectory: directory,
      isDirectory: false,
    })
    fileWatcherId.value = response.watcher_id
    fileWatcherPath.value = response.relative_path
  } catch (err) {
    console.warn('[WorkspaceFilesPanel] Failed to watch file:', err)
  }
}

const loadDirectory = async (
  relativePath: string,
  options: { preservePreview?: boolean; preserveFilter?: boolean } = {},
) => {
  const directory = props.workingDirectory.trim()
  if (!directory) return

  const currentSeq = ++loadSeq
  loading.value = true
  error.value = ''

  try {
    const response = await invoke<WorkspaceEntriesResponse>('list_working_directory_entries', {
      conversationId: props.conversationId,
      relativePath: relativePath || null,
      workingDirectory: directory,
    })
    if (currentSeq !== loadSeq) return
    root.value = response.root
    currentRelativePath.value = response.relative_path
    parentRelativePath.value = response.parent_relative_path || null
    entries.value = response.entries
    truncated.value = response.truncated
    if (!options.preserveFilter) filterText.value = ''
    if (!options.preservePreview) {
      childEntriesByDir.value = {}
      expandedDirectories.value = new Set()
      loadingDirectories.value = new Set()
      clearPreview()
    }
    void ensureDirectoryWatcher(response.relative_path)
  } catch (err) {
    if (currentSeq !== loadSeq) return
    error.value = err instanceof Error ? err.message : String(err)
    entries.value = []
    truncated.value = false
    clearPreview()
  } finally {
    if (currentSeq === loadSeq) loading.value = false
  }
}

const reloadCurrentDirectory = () => {
  if (!canLeaveCurrentEdit()) return
  void loadDirectory(currentRelativePath.value)
}

const goParent = () => {
  if (!canLeaveCurrentEdit()) return
  void loadDirectory(parentRelativePath.value || '')
}

const isExpanded = (relativePath: string) => expandedDirectories.value.has(relativePath)

const isDirectoryLoading = (relativePath: string) => loadingDirectories.value.has(relativePath)

const loadChildEntries = async (relativePath: string) => {
  if (childEntriesByDir.value[relativePath] || isDirectoryLoading(relativePath)) return
  const directory = props.workingDirectory.trim()
  if (!directory) return

  loadingDirectories.value = new Set([...loadingDirectories.value, relativePath])
  error.value = ''

  try {
    const response = await invoke<WorkspaceEntriesResponse>('list_working_directory_entries', {
      conversationId: props.conversationId,
      relativePath,
      workingDirectory: directory,
    })
    childEntriesByDir.value = {
      ...childEntriesByDir.value,
      [relativePath]: response.entries,
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    const nextLoading = new Set(loadingDirectories.value)
    nextLoading.delete(relativePath)
    loadingDirectories.value = nextLoading
  }
}

const toggleDirectory = (entry: WorkspaceEntry) => {
  if (!entry.is_directory) return
  const nextExpanded = new Set(expandedDirectories.value)
  if (nextExpanded.has(entry.relative_path)) {
    nextExpanded.delete(entry.relative_path)
    expandedDirectories.value = nextExpanded
    return
  }
  nextExpanded.add(entry.relative_path)
  expandedDirectories.value = nextExpanded
  void loadChildEntries(entry.relative_path)
}

const resolveOpenTarget = async (relativePath: string) => {
  const directory = props.workingDirectory.trim()
  if (!directory) return null
  return invoke<WorkingDirectoryOpenTarget>('resolve_working_directory_file_open_target', {
    conversationId: props.conversationId,
    relativePath,
    workingDirectory: directory,
  })
}

const readFilePreview = async (entry: WorkspaceEntry) => {
  const currentSeq = ++previewSeq
  selectedPath.value = entry.relative_path
  selectedOpenTargetPath.value = ''
  preview.value = null
  previewError.value = ''
  previewLoading.value = true
  externalFileChanged.value = false
  previewRenderMode.value = 'source'

  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico', 'avif'].includes(entry.relative_path.toLowerCase().split('.').pop() || '')) {
    try {
      const target = await resolveOpenTarget(entry.relative_path)
      if (currentSeq !== previewSeq) return
      selectedOpenTargetPath.value = target?.path || ''
      editMode.value = false
      draftContent.value = ''
      void ensureFileWatcher(entry.relative_path)
    } catch (err) {
      if (currentSeq !== previewSeq) return
      previewError.value = err instanceof Error ? err.message : String(err)
      void ensureFileWatcher(entry.relative_path)
    } finally {
      if (currentSeq === previewSeq) previewLoading.value = false
    }
    return
  }

  try {
    const result = await invoke<WorkingDirectoryFilePreview>('read_working_directory_file_preview', {
      conversationId: props.conversationId,
      relativePath: entry.relative_path,
      workingDirectory: props.workingDirectory.trim(),
      maxChars: 12000,
    })
    if (currentSeq !== previewSeq) return
    preview.value = result
    previewRenderMode.value = ['md', 'markdown', 'mdown'].includes(result.relative_path.toLowerCase().split('.').pop() || '') ? 'rendered' : 'source'
    editMode.value = false
    draftContent.value = ''
    void ensureFileWatcher(result.relative_path)
  } catch (err) {
    if (currentSeq !== previewSeq) return
    previewError.value = err instanceof Error ? err.message : String(err)
    try {
      const target = await resolveOpenTarget(entry.relative_path)
      if (currentSeq === previewSeq) selectedOpenTargetPath.value = target?.path || ''
    } catch {
      selectedOpenTargetPath.value = ''
    }
    void ensureFileWatcher(entry.relative_path)
  } finally {
    if (currentSeq === previewSeq) previewLoading.value = false
  }
}

const openEntry = (entry: WorkspaceEntry) => {
  if (!canLeaveCurrentEdit()) return
  if (entry.is_directory) {
    toggleDirectory(entry)
    return
  }
  void readFilePreview(entry)
}

const canLeaveCurrentEdit = () => {
  if (!hasDraftChange.value) return true
  previewError.value = '当前文件有未保存修改，请先保存或取消。'
  return false
}

const refreshSelectedFile = () => {
  if (!selectedPath.value || hasDraftChange.value) {
    previewError.value = '当前文件有未保存修改，请先取消后再刷新。'
    return
  }
  void readFilePreview({
    name: selectedPath.value.split('/').pop() || selectedPath.value,
    relative_path: selectedPath.value,
    is_directory: false,
  })
}

const openSelectedFile = async () => {
  const directory = props.workingDirectory.trim()
  if (!selectedPath.value || !directory || openingFile.value) return

  openingFile.value = true
  previewError.value = ''

  try {
    const target = await resolveOpenTarget(selectedPath.value)
    if (!target) return
    await invoke('plugin:opener|open_path', { path: target.path, with: null })
  } catch (err) {
    previewError.value = err instanceof Error ? err.message : String(err)
  } finally {
    openingFile.value = false
  }
}

const beginEdit = () => {
  if (!preview.value || preview.value.truncated) return
  draftContent.value = preview.value.preview
  previewError.value = ''
  editMode.value = true
}

const closeOperationDialogs = () => {
  if (operationBusy.value) return
  operationError.value = ''
  createDialog.value.open = false
  renameDialog.value.open = false
  deleteDialog.value.open = false
}

const openCreateDialog = (isDirectory: boolean) => {
  operationError.value = ''
  createDialog.value = {
    open: true,
    isDirectory,
    name: '',
  }
}

const openRenameDialog = (entry: WorkspaceEntry) => {
  operationError.value = ''
  renameDialog.value = {
    open: true,
    relativePath: entry.relative_path,
    name: entry.name,
  }
}

const openDeleteDialog = (entry: WorkspaceEntry) => {
  operationError.value = ''
  deleteDialog.value = {
    open: true,
    relativePath: entry.relative_path,
  }
}

const resetTreeAndReload = async (preservePreview = true) => {
  childEntriesByDir.value = {}
  expandedDirectories.value = new Set()
  loadingDirectories.value = new Set()
  await loadDirectory(currentRelativePath.value, {
    preserveFilter: true,
    preservePreview,
  })
}

const submitCreateEntry = async () => {
  const directory = props.workingDirectory.trim()
  const name = createDialog.value.name.trim()
  if (!directory || !name || operationBusy.value) return

  operationBusy.value = true
  operationError.value = ''

  try {
    await invoke<WorkingDirectoryMutationResponse>('create_working_directory_entry', {
      conversationId: props.conversationId,
      parentRelativePath: currentRelativePath.value || null,
      workingDirectory: directory,
      name,
      isDirectory: createDialog.value.isDirectory,
    })
    createDialog.value.open = false
    await resetTreeAndReload(true)
  } catch (err) {
    operationError.value = err instanceof Error ? err.message : String(err)
  } finally {
    operationBusy.value = false
  }
}

const submitRenameEntry = async () => {
  const directory = props.workingDirectory.trim()
  const relativePath = renameDialog.value.relativePath
  const newName = renameDialog.value.name.trim()
  if (!directory || !relativePath || !newName || operationBusy.value) return

  operationBusy.value = true
  operationError.value = ''

  try {
    const currentPreviewPath = preview.value?.relative_path || ''
    const result = await invoke<WorkingDirectoryMutationResponse>('rename_working_directory_entry', {
      conversationId: props.conversationId,
      relativePath,
      workingDirectory: directory,
      newName,
    })
    renameDialog.value.open = false
    const renamedSelection = selectedPath.value === relativePath || selectedPath.value.startsWith(`${relativePath}/`)
    const renamedPreview = currentPreviewPath === relativePath || currentPreviewPath.startsWith(`${relativePath}/`)
    let nextPreviewPath = ''
    if (renamedPreview) {
      nextPreviewPath = currentPreviewPath === relativePath
        ? result.relative_path
        : `${result.relative_path}${currentPreviewPath.slice(relativePath.length)}`
    }
    if (renamedSelection) {
      clearPreview()
    }
    await resetTreeAndReload(true)
    if (nextPreviewPath) {
      void readFilePreview({
        name: nextPreviewPath.split('/').pop() || nextPreviewPath,
        relative_path: nextPreviewPath,
        is_directory: false,
      })
    }
  } catch (err) {
    operationError.value = err instanceof Error ? err.message : String(err)
  } finally {
    operationBusy.value = false
  }
}

const submitDeleteEntry = async () => {
  const directory = props.workingDirectory.trim()
  const relativePath = deleteDialog.value.relativePath
  if (!directory || !relativePath || operationBusy.value) return

  operationBusy.value = true
  operationError.value = ''

  try {
    await invoke<WorkingDirectoryMutationResponse>('delete_working_directory_entry', {
      conversationId: props.conversationId,
      relativePath,
      workingDirectory: directory,
    })
    deleteDialog.value.open = false
    const deletedSelection = selectedPath.value === relativePath || selectedPath.value.startsWith(`${relativePath}/`)
    if (deletedSelection) clearPreview()
    await resetTreeAndReload(!deletedSelection)
  } catch (err) {
    operationError.value = err instanceof Error ? err.message : String(err)
  } finally {
    operationBusy.value = false
  }
}

const copyRelativePath = async (entry: WorkspaceEntry) => {
  previewError.value = ''
  try {
    await navigator.clipboard.writeText(entry.relative_path)
  } catch (err) {
    previewError.value = err instanceof Error ? err.message : String(err)
  }
}

const cancelEdit = () => {
  editMode.value = false
  draftContent.value = ''
  previewError.value = ''
}

const updateSelectedEntryMetadata = (result: WorkingDirectoryFilePreview) => {
  entries.value = entries.value.map((entry) => {
    if (entry.relative_path !== result.relative_path) return entry
    return {
      ...entry,
      modified_ms: result.modified_ms ?? entry.modified_ms,
      size: result.size,
    }
  })
}

const saveEdit = async () => {
  const currentPreview = preview.value
  const directory = props.workingDirectory.trim()
  if (!currentPreview || !directory || saving.value || !hasDraftChange.value) return

  saving.value = true
  previewError.value = ''

  try {
    const result = await invoke<WorkingDirectoryFilePreview>('write_working_directory_file', {
      conversationId: props.conversationId,
      relativePath: currentPreview.relative_path,
      workingDirectory: directory,
      expectedSha256: currentPreview.sha256,
      content: draftContent.value,
    })
    preview.value = result
    selectedPath.value = result.relative_path
    updateSelectedEntryMetadata(result)
    editMode.value = false
    draftContent.value = ''
    externalFileChanged.value = false
    localWriteGuardUntil = Date.now() + 1500
    void ensureFileWatcher(result.relative_path)
  } catch (err) {
    previewError.value = err instanceof Error ? err.message : String(err)
  } finally {
    saving.value = false
  }
}

const scheduleDirectoryRefresh = () => {
  if (directoryRefreshTimer) {
    clearTimeout(directoryRefreshTimer)
  }
  directoryRefreshTimer = setTimeout(() => {
    directoryRefreshTimer = null
    void loadDirectory(currentRelativePath.value, {
      preserveFilter: true,
      preservePreview: true,
    })
  }, 250)
}

const handleWorkspacePathChanged = (payload: WorkingDirectoryChangeEvent) => {
  if (payload.watcher_id === directoryWatcherId.value) {
    scheduleDirectoryRefresh()
    return
  }
  if (payload.watcher_id === fileWatcherId.value && payload.relative_path === selectedPath.value) {
    if (Date.now() < localWriteGuardUntil || saving.value) return
    externalFileChanged.value = true
    if (editMode.value) {
      previewError.value = '当前文件已被外部修改，请取消编辑并刷新后再继续。'
    }
  }
}

watch(
  () => [props.conversationId, props.workingDirectory],
  () => {
    void stopDirectoryWatcher()
    void stopFileWatcher()
    void loadDirectory('')
  },
  { immediate: true },
)

onMounted(async () => {
  loadPreviewHeight()
  unlistenWorkspaceChange = await listen<WorkingDirectoryChangeEvent>(
    'agent:workspace-path-changed',
    (event) => handleWorkspacePathChanged(event.payload),
  )
})

onUnmounted(() => {
  if (directoryRefreshTimer) {
    clearTimeout(directoryRefreshTimer)
    directoryRefreshTimer = null
  }
  if (unlistenWorkspaceChange) {
    unlistenWorkspaceChange()
    unlistenWorkspaceChange = null
  }
  stopPreviewResize()
  void stopDirectoryWatcher()
  void stopFileWatcher()
})
</script>
