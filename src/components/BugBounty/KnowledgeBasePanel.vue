<template>
  <div class="grid h-full min-w-0 gap-4 xl:grid-cols-[minmax(24rem,30rem)_minmax(0,1fr)]">
    <section class="card min-w-0 overflow-hidden bg-base-100 shadow-md">
      <div class="card-body min-w-0 gap-4 p-4">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="card-title">{{ t('bugBounty.knowledge.title') }}</h2>
            <p class="text-sm text-base-content/65">
              {{ t('bugBounty.knowledge.description') }}
            </p>
          </div>
          <button class="btn btn-sm btn-primary" type="button" @click="createNewNote">
            <i class="fas fa-plus mr-2"></i>
            {{ t('bugBounty.knowledge.newNote') }}
          </button>
        </div>

        <div class="grid gap-3 md:grid-cols-3">
          <div class="rounded-2xl bg-base-200 px-4 py-3">
            <div class="text-xs text-base-content/55">{{ t('bugBounty.knowledge.stats.total') }}</div>
            <div class="mt-1 text-2xl font-semibold">{{ stats.total_notes }}</div>
          </div>
          <div class="rounded-2xl bg-base-200 px-4 py-3">
            <div class="text-xs text-base-content/55">{{ t('bugBounty.knowledge.stats.bound') }}</div>
            <div class="mt-1 text-2xl font-semibold">{{ stats.bound_notes }}</div>
          </div>
          <div class="rounded-2xl bg-base-200 px-4 py-3">
            <div class="text-xs text-base-content/55">{{ t('bugBounty.knowledge.stats.unbound') }}</div>
            <div class="mt-1 text-2xl font-semibold">{{ stats.unbound_notes }}</div>
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <label class="input input-bordered input-sm flex flex-1 min-w-[14rem] items-center gap-2">
            <i class="fas fa-search text-base-content/45"></i>
            <input
              v-model="searchQuery"
              type="text"
              class="grow"
              :placeholder="t('bugBounty.knowledge.searchPlaceholder')"
              @input="scheduleLoadNotes"
            />
          </label>

          <select v-model="scopeFilter" class="select select-bordered select-sm" @change="loadNotes">
            <option value="all">{{ t('bugBounty.knowledge.filters.all') }}</option>
            <option value="current" :disabled="!selectedProgram?.id">
              {{ t('bugBounty.knowledge.filters.currentProgram') }}
            </option>
            <option value="unbound">{{ t('bugBounty.knowledge.filters.unbound') }}</option>
          </select>

          <button class="btn btn-sm btn-outline" type="button" @click="loadNotes">
            <i class="fas fa-rotate mr-2"></i>
            {{ t('common.refresh') }}
          </button>
        </div>

        <div v-if="loading" class="flex flex-1 items-center justify-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="notes.length === 0" class="flex flex-1 flex-col items-center justify-center rounded-2xl border border-dashed border-base-300 px-6 py-12 text-center">
          <i class="fas fa-book-open text-3xl text-base-content/30"></i>
          <div class="mt-3 text-sm text-base-content/70">{{ t('bugBounty.knowledge.empty') }}</div>
          <div class="mt-1 text-xs text-base-content/45">{{ t('bugBounty.knowledge.emptyHint') }}</div>
        </div>

        <div v-else class="space-y-3 overflow-auto pr-1">
          <button
            v-for="note in notes"
            :key="note.id"
            type="button"
            class="w-full rounded-2xl border px-4 py-3 text-left transition-colors"
            :class="selectedNoteId === note.id ? 'border-primary bg-primary/5' : 'border-base-300 hover:border-primary/35'"
            @click="openNote(note.id)"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="truncate font-semibold">{{ note.title || t('bugBounty.knowledge.untitled') }}</div>
                <div class="mt-1 line-clamp-3 text-sm text-base-content/65">
                  {{ formatSnippet(note.snippet || note.content) }}
                </div>
              </div>
              <span class="badge badge-outline badge-sm whitespace-nowrap">
                {{ note.program_name || t('bugBounty.knowledge.unboundProgram') }}
              </span>
            </div>
            <div class="mt-3 flex flex-wrap items-center gap-2 text-xs text-base-content/45">
              <span>{{ formatDate(note.updated_at) }}</span>
              <span
                v-for="tag in parseTags(note.tags_json).slice(0, 4)"
                :key="`${note.id}-${tag}`"
                class="badge badge-ghost badge-sm"
              >
                {{ tag }}
              </span>
            </div>
          </button>
        </div>
      </div>
    </section>

    <section class="card min-w-0 overflow-hidden bg-base-100 shadow-md">
      <div class="card-body min-h-0 min-w-0 gap-3 p-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div class="min-w-0">
            <h2 class="truncate text-base font-semibold">
              {{ draft.id ? t('bugBounty.knowledge.editNote') : t('bugBounty.knowledge.newNote') }}
            </h2>
          </div>
          <div class="flex gap-2">
            <button
              v-if="draft.id"
              class="btn btn-sm btn-error btn-outline"
              type="button"
              :disabled="saving || deleting"
              @click="deleteNote"
            >
              <span v-if="deleting" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-trash mr-2"></i>
              {{ t('common.delete') }}
            </button>
            <button
              class="btn btn-sm btn-primary"
              type="button"
              :disabled="saving"
              @click="saveNote"
            >
              <span v-if="saving" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-save mr-2"></i>
              {{ t('common.save') }}
            </button>
          </div>
        </div>

        <div class="grid gap-2 rounded-lg border border-base-300 bg-base-200/35 p-2 xl:grid-cols-[minmax(14rem,1.4fr)_minmax(10rem,0.8fr)_minmax(12rem,1fr)]">
          <label class="form-control gap-1">
            <span class="label-text text-xs">{{ t('bugBounty.knowledge.fields.title') }}</span>
            <input
              v-model="draft.title"
              type="text"
              class="input input-bordered input-sm"
              :placeholder="t('bugBounty.knowledge.fields.titlePlaceholder')"
            />
          </label>

          <label class="form-control gap-1">
            <span class="label-text text-xs">{{ t('bugBounty.knowledge.fields.program') }}</span>
            <select v-model="draft.program_id" class="select select-bordered select-sm">
              <option value="">{{ t('bugBounty.knowledge.unboundProgram') }}</option>
              <option
                v-for="program in programs"
                :key="program.id"
                :value="program.id"
              >
                {{ program.name }}
              </option>
            </select>
          </label>

          <label class="form-control gap-1">
            <span class="label-text text-xs">{{ t('bugBounty.knowledge.fields.tags') }}</span>
            <input
              v-model="draft.tagsInput"
              type="text"
              class="input input-bordered input-sm"
              :placeholder="t('bugBounty.knowledge.fields.tagsPlaceholder')"
            />
          </label>
        </div>

        <div class="form-control min-h-0 min-w-0 flex-1 gap-2">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <span class="label-text">{{ t('bugBounty.knowledge.fields.content') }}</span>
            <div class="join">
              <button
                type="button"
                class="btn join-item btn-xs"
                :class="contentMode === 'edit' ? 'btn-primary' : 'btn-outline'"
                @click="contentMode = 'edit'"
              >
                <i class="fas fa-pen-to-square mr-1"></i>
                {{ t('bugBounty.knowledge.markdown.edit') }}
              </button>
              <button
                type="button"
                class="btn join-item btn-xs"
                :class="contentMode === 'preview' ? 'btn-primary' : 'btn-outline'"
                @click="contentMode = 'preview'"
              >
                <i class="fas fa-eye mr-1"></i>
                {{ t('bugBounty.knowledge.markdown.preview') }}
              </button>
            </div>
          </div>
          <textarea
            v-if="contentMode === 'edit'"
            ref="contentTextarea"
            v-model="editorContent"
            class="textarea textarea-bordered min-h-[28rem] flex-1 resize-none font-mono text-sm"
            :placeholder="t('bugBounty.knowledge.fields.contentPlaceholder')"
            @input="syncDraftContentFromEditor"
            @paste="handleContentPaste"
          />
          <div
            v-else-if="fullDraftContent.trim()"
            class="knowledge-markdown-preview min-h-[28rem] min-w-0 flex-1 rounded-lg border border-base-300 bg-base-100 px-4 py-3"
            v-html="renderedDraftContent"
          ></div>
          <div
            v-else
            class="flex min-h-[28rem] flex-1 items-center justify-center rounded-lg border border-dashed border-base-300 bg-base-100 text-sm text-base-content/45"
          >
            {{ t('bugBounty.knowledge.markdown.emptyPreview') }}
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'
import { dialog } from '@/composables/useDialog'
import { emitBountyKnowledgeUpdated } from '@/services/bountyKnowledgeEvents'
import {
  formatBountyKnowledgeMarkdownSnippet,
  renderBountyKnowledgeMarkdown,
} from './bountyKnowledgeMarkdown'
import {
  buildPastedImageReferenceMarkdown,
  collapseBountyKnowledgeImageContent,
  expandBountyKnowledgeImageContent,
  findFirstClipboardImageFile,
  insertTextAtSelection,
  readImageFileAsDataUrl,
  type BountyKnowledgeImageReference,
} from './bountyKnowledgeImagePaste'

interface ProgramSummary {
  id: string
  name: string
}

interface KnowledgeNoteSummary {
  id: string
  title: string
  content: string
  snippet?: string
  program_id?: string | null
  program_name?: string | null
  tags_json?: string | null
  updated_at: string
}

const props = defineProps<{
  programs: ProgramSummary[]
  selectedProgram?: ProgramSummary | null
}>()

const { t } = useI18n()
const toast = useToast()
const route = useRoute()

type ScopeFilter = 'all' | 'current' | 'unbound'

const loading = ref(false)
const saving = ref(false)
const deleting = ref(false)
const notes = ref<KnowledgeNoteSummary[]>([])
const searchQuery = ref('')
const scopeFilter = ref<ScopeFilter>('all')
const contentMode = ref<'edit' | 'preview'>('edit')
const contentTextarea = ref<HTMLTextAreaElement | null>(null)
const editorContent = ref('')
const imageReferences = ref<BountyKnowledgeImageReference[]>([])
const pastedImageCount = ref(0)
const selectedNoteId = ref('')
const stats = ref({
  total_notes: 0,
  bound_notes: 0,
  unbound_notes: 0,
})
const draft = ref({
  id: '',
  title: '',
  content: '',
  program_id: '',
  tagsInput: '',
})

let searchTimer: ReturnType<typeof setTimeout> | null = null

const selectedProgramId = computed(() => props.selectedProgram?.id || '')
const fullDraftContent = computed(() =>
  expandBountyKnowledgeImageContent(editorContent.value, imageReferences.value),
)
const renderedDraftContent = computed(() => renderBountyKnowledgeMarkdown(fullDraftContent.value))

const syncEditorFromContent = (content: string) => {
  const collapsed = collapseBountyKnowledgeImageContent(content)
  editorContent.value = collapsed.content
  imageReferences.value = collapsed.references
  draft.value.content = expandBountyKnowledgeImageContent(editorContent.value, imageReferences.value)
}

const syncDraftContentFromEditor = () => {
  draft.value.content = fullDraftContent.value
}

const createEmptyDraft = () => {
  draft.value = {
    id: '',
    title: '',
    content: '',
    program_id: selectedProgramId.value,
    tagsInput: '',
  }
  editorContent.value = ''
  imageReferences.value = []
  selectedNoteId.value = ''
}

const parseTags = (tagsJson?: string | null) => {
  if (!tagsJson) {
    return []
  }

  try {
    const parsed = JSON.parse(tagsJson)
    return Array.isArray(parsed) ? parsed.filter(tag => typeof tag === 'string' && tag.trim()) : []
  } catch {
    return tagsJson
      .split(/[,\n]/g)
      .map(tag => tag.trim())
      .filter(Boolean)
  }
}

const formatSnippet = (value?: string | null) =>
  formatBountyKnowledgeMarkdownSnippet(String(value || ''))

const handleContentPaste = async (event: ClipboardEvent) => {
  const file = findFirstClipboardImageFile(event.clipboardData)
  if (!file) return

  event.preventDefault()

  try {
    const target = event.target instanceof HTMLTextAreaElement ? event.target : contentTextarea.value
    const selectionStart = target?.selectionStart ?? draft.value.content.length
    const selectionEnd = target?.selectionEnd ?? selectionStart
    const dataUrl = await readImageFileAsDataUrl(file)
    pastedImageCount.value += 1
    const id = `bounty-image-${Date.now()}-${pastedImageCount.value}`
    const markdown = buildPastedImageReferenceMarkdown(
      id,
      t('bugBounty.knowledge.markdown.pastedImageAlt', { index: pastedImageCount.value }),
    )
    const next = insertTextAtSelection(editorContent.value, markdown, selectionStart, selectionEnd)
    editorContent.value = next.value
    imageReferences.value = [...imageReferences.value, { id, dataUrl }]
    syncDraftContentFromEditor()

    await nextTick()
    const textarea = contentTextarea.value
    textarea?.focus()
    textarea?.setSelectionRange(next.cursor, next.cursor)
    toast.success(t('bugBounty.knowledge.markdown.imagePasted'))
  } catch (error) {
    console.error('Failed to paste bounty knowledge image:', error)
    toast.error(t('bugBounty.knowledge.markdown.imagePasteFailed'))
  }
}

const serializeTags = (raw: string) => {
  const tags = raw
    .split(/[,\n]/g)
    .map(tag => tag.trim())
    .filter(Boolean)

  return tags.length > 0 ? tags : null
}

const formatDate = (value?: string) => {
  if (!value) {
    return '--'
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString()
}

const buildFilterPayload = () => {
  if (scopeFilter.value === 'current' && selectedProgramId.value) {
    return {
      program_id: selectedProgramId.value,
      only_unbound: false,
      limit: 80,
      offset: 0,
    }
  }

  if (scopeFilter.value === 'unbound') {
    return {
      only_unbound: true,
      limit: 80,
      offset: 0,
    }
  }

  return {
    limit: 80,
    offset: 0,
  }
}

const loadStats = async () => {
  try {
    stats.value = await invoke('bounty_get_knowledge_note_stats', { programId: null })
  } catch (error) {
    console.error('Failed to load bounty knowledge stats:', error)
  }
}

const loadNotes = async () => {
  try {
    loading.value = true
    const trimmedQuery = searchQuery.value.trim()
    notes.value = trimmedQuery
      ? await invoke('bounty_search_knowledge_notes', {
          request: {
            query: trimmedQuery,
            ...buildFilterPayload(),
          },
        })
      : await invoke('bounty_list_knowledge_notes', {
          filter: buildFilterPayload(),
        })
  } catch (error) {
    console.error('Failed to load bounty knowledge notes:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
    notes.value = []
  } finally {
    loading.value = false
  }
}

const scheduleLoadNotes = () => {
  if (searchTimer) {
    clearTimeout(searchTimer)
  }

  searchTimer = setTimeout(() => {
    void loadNotes()
  }, 120)
}

const applyNoteToDraft = (note: any) => {
  selectedNoteId.value = note.id
  draft.value = {
    id: note.id,
    title: note.title || '',
    content: note.content || '',
    program_id: note.program_id || '',
    tagsInput: parseTags(note.tags_json).join(', '),
  }
  syncEditorFromContent(draft.value.content)
}

const openNote = async (noteId: string) => {
  try {
    const note = await invoke<any>('bounty_get_knowledge_note', { id: noteId })
    if (!note) {
      toast.warning(t('bugBounty.knowledge.notFound'))
      return
    }
    applyNoteToDraft(note)
  } catch (error) {
    console.error('Failed to open bounty knowledge note:', error)
    toast.error(t('bugBounty.errors.loadFailed'))
  }
}

const createNewNote = () => {
  createEmptyDraft()
}

const saveNote = async () => {
  const isEditing = Boolean(draft.value.id)
  const title = draft.value.title.trim()
  const content = fullDraftContent.value.trim()

  if (!title || !content) {
    toast.warning(t('bugBounty.knowledge.validation.required'))
    return
  }

  try {
    saving.value = true
    const request = {
      program_id: draft.value.program_id || null,
      title,
      content,
      tags: serializeTags(draft.value.tagsInput),
    }

    const saved = draft.value.id
      ? await invoke<any>('bounty_update_knowledge_note', {
          id: draft.value.id,
          request,
        })
      : await invoke<any>('bounty_create_knowledge_note', { request })

    applyNoteToDraft(saved)
    emitBountyKnowledgeUpdated()
    await Promise.all([loadNotes(), loadStats()])
    toast.success(
      isEditing
        ? t('bugBounty.knowledge.savedUpdated')
        : t('bugBounty.knowledge.savedCreated'),
    )
  } catch (error) {
    console.error('Failed to save bounty knowledge note:', error)
    toast.error(t('bugBounty.errors.saveFailed'))
  } finally {
    saving.value = false
  }
}

const deleteNote = async () => {
  if (!draft.value.id) {
    return
  }
  if (!(await dialog.confirm(t('bugBounty.knowledge.confirmDelete')))) {
    return
  }

  try {
    deleting.value = true
    await invoke('bounty_delete_knowledge_note', { id: draft.value.id })
    emitBountyKnowledgeUpdated()
    createEmptyDraft()
    await Promise.all([loadNotes(), loadStats()])
    toast.success(t('bugBounty.knowledge.deleted'))
  } catch (error) {
    console.error('Failed to delete bounty knowledge note:', error)
    toast.error(t('bugBounty.errors.deleteFailed'))
  } finally {
    deleting.value = false
  }
}

const syncFromRoute = async () => {
  const knowledgeId = typeof route.query.knowledgeId === 'string' ? route.query.knowledgeId.trim() : ''
  const routeSearch = typeof route.query.q === 'string' ? route.query.q.trim() : ''

  let shouldReloadNotes = false
  if (routeSearch && searchQuery.value !== routeSearch) {
    searchQuery.value = routeSearch
    shouldReloadNotes = true
  }

  if (shouldReloadNotes) {
    await loadNotes()
  }

  if (!knowledgeId || knowledgeId === selectedNoteId.value) {
    return
  }

  scopeFilter.value = 'all'
  await openNote(knowledgeId)
}

watch(selectedProgramId, value => {
  if (scopeFilter.value === 'current' && !value) {
    scopeFilter.value = 'all'
  }
  if (scopeFilter.value === 'current') {
    void loadNotes()
  }
  if (!draft.value.id && !draft.value.program_id) {
    draft.value.program_id = value
  }
})

watch(
  () => [route.query.knowledgeId, route.query.q],
  async () => {
    await syncFromRoute()
  },
)

onMounted(async () => {
  createEmptyDraft()
  await Promise.all([loadNotes(), loadStats()])
  await syncFromRoute()
})

onBeforeUnmount(() => {
  if (searchTimer) {
    clearTimeout(searchTimer)
    searchTimer = null
  }
})
</script>

<style scoped>
.knowledge-markdown-preview {
  width: 100%;
  max-width: 100%;
  overflow: auto;
  line-height: 1.65;
  color: hsl(var(--bc));
  overflow-wrap: anywhere;
}

.knowledge-markdown-preview :deep(h1),
.knowledge-markdown-preview :deep(h2),
.knowledge-markdown-preview :deep(h3) {
  margin: 0.9rem 0 0.45rem;
  font-weight: 700;
  line-height: 1.3;
}

.knowledge-markdown-preview :deep(h1) {
  font-size: 1.35rem;
}

.knowledge-markdown-preview :deep(h2) {
  font-size: 1.18rem;
}

.knowledge-markdown-preview :deep(h3) {
  font-size: 1.05rem;
}

.knowledge-markdown-preview :deep(p) {
  margin: 0.55rem 0;
}

.knowledge-markdown-preview :deep(ul),
.knowledge-markdown-preview :deep(ol) {
  margin: 0.55rem 0 0.55rem 1.35rem;
}

.knowledge-markdown-preview :deep(ul) {
  list-style: disc;
}

.knowledge-markdown-preview :deep(ol) {
  list-style: decimal;
}

.knowledge-markdown-preview :deep(li) {
  margin: 0.25rem 0;
}

.knowledge-markdown-preview :deep(blockquote) {
  margin: 0.75rem 0;
  padding: 0.45rem 0.75rem;
  border-left: 3px solid hsl(var(--p) / 0.55);
  background: hsl(var(--b2) / 0.55);
  color: hsl(var(--bc) / 0.72);
}

.knowledge-markdown-preview :deep(code) {
  white-space: break-spaces;
  border-radius: 0.35rem;
  background: hsl(var(--b2));
  padding: 0.1rem 0.35rem;
  font-size: 0.86em;
}

.knowledge-markdown-preview :deep(pre) {
  max-width: 100%;
  margin: 0.75rem 0;
  overflow-x: auto;
  border-radius: 0.5rem;
  background: hsl(var(--b2));
  padding: 0.75rem;
}

.knowledge-markdown-preview :deep(pre code) {
  white-space: pre;
  background: transparent;
  padding: 0;
}

.knowledge-markdown-preview :deep(a) {
  color: hsl(var(--p));
  text-decoration: underline;
  text-underline-offset: 2px;
}

.knowledge-markdown-preview :deep(img) {
  display: block;
  max-width: min(100%, 48rem);
  max-height: 28rem;
  height: auto;
  margin: 0.75rem auto;
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--b3));
  object-fit: contain;
}

.knowledge-markdown-preview :deep(table) {
  display: block;
  max-width: 100%;
  overflow-x: auto;
  width: max-content;
  margin: 0.75rem 0;
  border-collapse: collapse;
  font-size: 0.9rem;
}

.knowledge-markdown-preview :deep(th),
.knowledge-markdown-preview :deep(td) {
  border: 1px solid hsl(var(--b3));
  padding: 0.4rem 0.55rem;
  text-align: left;
}

.knowledge-markdown-preview :deep(th) {
  background: hsl(var(--b2));
}
</style>
