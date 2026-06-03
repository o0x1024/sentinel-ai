<template>
  <div class="help-center-window h-screen overflow-y-auto overflow-x-hidden bg-base-100 text-base-content">
    <div class="help-center-backdrop"></div>

    <main
      class="relative mx-auto flex min-h-full w-full max-w-7xl flex-col gap-6 px-4 py-6 sm:px-6 lg:px-8"
    >
      <header
        class="rounded-[1.75rem] border border-base-300/70 bg-base-100/90 p-5 shadow-sm backdrop-blur sm:p-6"
      >
        <div class="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
          <div class="max-w-4xl space-y-4">
            <div
              class="inline-flex items-center rounded-full border border-primary/20 bg-primary/10 px-4 py-1 text-xs font-semibold uppercase tracking-[0.22em] text-primary"
            >
              {{ content.badge }}
            </div>
            <div class="space-y-2">
              <h1 class="text-3xl font-black leading-tight text-balance sm:text-4xl">
                {{ content.title }}
              </h1>
              <p class="max-w-3xl text-sm leading-7 text-base-content/72 sm:text-base">
                {{ content.description }}
              </p>
            </div>
          </div>

          <div class="flex shrink-0 items-start gap-3 lg:justify-end">
            <button class="btn btn-primary btn-sm sm:btn-md" type="button" @click="closeWindow">
              <i class="fas fa-xmark"></i>
              {{ t('common.close') }}
            </button>
          </div>
        </div>
      </header>

      <section class="grid gap-6 lg:grid-cols-[18rem_minmax(0,1fr)] lg:items-start">
        <aside class="lg:sticky lg:top-6">
          <div
            class="rounded-[1.5rem] border border-base-300 bg-base-100/92 p-4 shadow-sm backdrop-blur sm:p-5"
          >
            <div class="space-y-3 border-b border-base-300/80 pb-4">
              <div class="text-xs font-semibold uppercase tracking-[0.24em] text-base-content/45">
                {{ content.badge }}
              </div>
              <div class="text-lg font-semibold text-base-content">{{ content.overviewTitle }}</div>
              <p class="text-sm leading-6 text-base-content/65">{{ content.releaseNote }}</p>
            </div>

            <nav class="mt-4" aria-label="Help center navigation">
              <ul class="space-y-2">
                <li v-for="node in navTree" :key="node.id">
                  <div class="flex items-start gap-2">
                    <button
                      class="flex min-w-0 flex-1 items-center rounded-2xl px-3 py-2.5 text-left text-sm transition-colors"
                      :class="getParentNodeClass(node.id)"
                      type="button"
                      @click="handleParentNodeClick(node.id, node.children.length > 0)"
                    >
                      <span class="truncate font-medium">{{ node.label }}</span>
                    </button>

                    <button
                      v-if="node.children.length > 0"
                      class="mt-0.5 inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl border border-base-300 text-base-content/55 transition-colors hover:bg-base-200"
                      type="button"
                      @click="handleParentNodeClick(node.id, true)"
                    >
                      <i
                        class="fas fa-chevron-right text-[0.7rem] transition-transform"
                        :class="isNodeExpanded(node.id) ? 'rotate-90' : ''"
                      ></i>
                    </button>
                  </div>

                  <ul
                    v-if="node.children.length > 0 && isNodeExpanded(node.id)"
                    class="ml-5 mt-2 space-y-1.5 border-l border-base-300/80 pl-4"
                  >
                    <li v-for="child in node.children" :key="child.id">
                      <button
                        class="flex w-full items-center rounded-xl px-3 py-2 text-left text-sm transition-colors"
                        :class="
                          activeDocumentId === child.id
                            ? 'bg-base-content text-base-100'
                            : 'text-base-content/62 hover:bg-base-200'
                        "
                        type="button"
                        @click="selectDocument(child.id)"
                      >
                        <span class="truncate">{{ child.label }}</span>
                      </button>
                    </li>
                  </ul>
                </li>
              </ul>
            </nav>
          </div>
        </aside>

        <article
          class="min-w-0 rounded-[1.75rem] border border-base-300 bg-base-100/92 p-6 shadow-sm backdrop-blur sm:p-7"
        >
          <div class="space-y-6">
            <div class="flex flex-wrap items-center gap-3 text-sm text-base-content/72">
              <span
                class="inline-flex items-center rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em]"
                :class="getDocumentBadgeClass(activeDocument.id, activeDocument.parentId)"
              >
                {{ activeDocument.badge }}
              </span>

              <span class="rounded-full border border-base-300 bg-base-100 px-4 py-2">
                <i class="fas fa-folder-tree mr-2 text-base-content/45"></i>
                {{ getDocumentKindLabel(activeDocument.kind) }}
              </span>

              <span
                v-if="activeDocument.route"
                class="rounded-full border border-base-300 bg-base-100 px-4 py-2"
              >
                <i class="fas fa-signs-post mr-2 text-base-content/45"></i>
                {{ activeDocument.route }}
              </span>

              <button
                v-if="activeParentDocument"
                class="rounded-full border border-base-300 bg-base-100 px-4 py-2 transition-colors hover:bg-base-200"
                type="button"
                @click="selectDocument(activeParentDocument.id)"
              >
                <i class="fas fa-arrow-left mr-2 text-base-content/45"></i>
                {{ activeParentDocument.title }}
              </button>
            </div>

            <div class="space-y-3">
              <div class="flex items-start gap-4">
                <div
                  v-if="activeDocument.icon"
                  class="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary"
                >
                  <i :class="[activeDocument.icon, 'text-lg']"></i>
                </div>

                <div class="min-w-0 flex-1">
                  <h2 class="text-2xl font-bold text-base-content sm:text-3xl">
                    {{ activeDocument.title }}
                  </h2>
                  <p class="mt-3 max-w-4xl text-sm leading-7 text-base-content/72 sm:text-base">
                    {{ activeDocument.description }}
                  </p>
                </div>
              </div>
            </div>

            <section
              v-if="activeDocument.kind === 'overview'"
              class="space-y-5"
            >
              <div class="grid gap-3 sm:grid-cols-3">
                <article
                  v-for="stat in activeDocument.stats"
                  :key="stat.label"
                  class="rounded-[1.25rem] border border-base-300 bg-base-100 p-4"
                >
                  <div class="text-xs uppercase tracking-[0.22em] text-base-content/45">
                    {{ stat.label }}
                  </div>
                  <div class="mt-2 text-base font-semibold text-base-content">{{ stat.value }}</div>
                </article>
              </div>

              <div class="rounded-[1.4rem] border border-base-300 bg-base-100 p-5">
                <div class="text-sm font-semibold text-base-content">
                  {{ isZh ? '文档入口' : 'Document Sections' }}
                </div>
                <div class="mt-4 grid gap-3 xl:grid-cols-2">
                  <button
                    v-for="link in activeDocument.links"
                    :key="link.id"
                    class="rounded-[1.2rem] border border-base-300 bg-base-100 p-4 text-left transition-colors hover:bg-base-200"
                    type="button"
                    @click="selectDocument(link.id)"
                  >
                    <div class="flex items-center justify-between gap-3">
                      <div class="text-base font-semibold text-base-content">{{ link.title }}</div>
                      <span
                        v-if="link.meta"
                        class="rounded-full border border-base-300 px-3 py-1 text-xs text-base-content/55"
                      >
                        {{ link.meta }}
                      </span>
                    </div>
                    <p v-if="link.description" class="mt-2 text-sm leading-7 text-base-content/70">
                      {{ link.description }}
                    </p>
                  </button>
                </div>
              </div>
            </section>

            <section
              v-else-if="activeDocument.kind === 'workflow'"
              class="rounded-[1.4rem] border border-base-300 bg-base-100 p-5"
            >
              <div class="text-sm font-semibold text-base-content">
                {{ isZh ? '推荐步骤' : 'Recommended Steps' }}
              </div>
              <div class="mt-4 grid gap-4">
                <button
                  v-for="step in activeDocument.workflowSteps"
                  :key="step.id"
                  class="rounded-[1.2rem] border border-base-300 bg-base-100 p-5 text-left transition-colors hover:bg-base-200"
                  type="button"
                  @click="selectDocument(step.id)"
                >
                  <div class="text-base font-semibold text-base-content">{{ step.title }}</div>
                  <p class="mt-2 text-sm leading-7 text-base-content/70">{{ step.description }}</p>
                </button>
              </div>
            </section>

            <section
              v-else-if="activeDocument.kind === 'workflow-step'"
              class="rounded-[1.4rem] border border-base-300 bg-base-100 p-6"
            >
              <div class="text-sm leading-7 text-base-content/70">
                {{ isZh ? '当前步骤是完整主线中的单独阶段，先完成这一段，再进入下一段。' : 'This step is one stage in the main operating path. Finish it before moving forward.' }}
              </div>
            </section>

            <section
              v-else-if="activeDocument.kind === 'catalog-section'"
              class="rounded-[1.4rem] border border-base-300 bg-base-100 p-5"
            >
              <div class="text-sm font-semibold text-base-content">
                {{ isZh ? '目录内容' : 'Directory Content' }}
              </div>
              <div class="mt-4 grid gap-4 xl:grid-cols-2">
                <button
                  v-for="link in activeDocument.links"
                  :key="link.id"
                  class="rounded-[1.2rem] border border-base-300 bg-base-100 p-5 text-left transition-colors hover:bg-base-200"
                  type="button"
                  @click="selectDocument(link.id)"
                >
                  <div class="flex items-start gap-4">
                    <div
                      v-if="link.icon"
                      class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary"
                    >
                      <i :class="[link.icon, 'text-base']"></i>
                    </div>

                    <div class="min-w-0 flex-1">
                      <div class="flex flex-wrap items-center gap-2">
                        <div class="text-base font-semibold text-base-content">{{ link.title }}</div>
                        <span
                          v-if="link.meta"
                          class="rounded-full border border-base-300 px-3 py-1 text-xs text-base-content/55"
                        >
                          {{ link.meta }}
                        </span>
                      </div>
                      <p
                        v-if="link.description"
                        class="mt-2 text-sm leading-7 text-base-content/70"
                      >
                        {{ link.description }}
                      </p>
                    </div>
                  </div>
                </button>
              </div>
            </section>

            <section
              v-else-if="activeDocument.kind === 'feature-entry'"
              class="space-y-4"
            >
              <div class="grid gap-4 xl:grid-cols-[1fr_1fr]">
                <div class="rounded-[1.2rem] bg-base-200/70 p-4">
                  <div class="text-sm font-semibold text-base-content">
                    {{ featureDescriptionLabel }}
                  </div>
                  <ul class="mt-3 space-y-2 text-sm leading-7 text-base-content/72">
                    <li
                      v-for="capability in activeDocument.capabilities"
                      :key="capability"
                      class="flex gap-2"
                    >
                      <span class="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-primary"></span>
                      <span>{{ capability }}</span>
                    </li>
                  </ul>
                </div>

                <div class="rounded-[1.2rem] bg-base-200/70 p-4">
                  <div class="text-sm font-semibold text-base-content">
                    {{ operationGuideLabel }}
                  </div>
                  <ol class="mt-3 space-y-2 text-sm leading-7 text-base-content/72">
                    <li
                      v-for="(operation, index) in activeDocument.operations"
                      :key="operation"
                      class="flex gap-3"
                    >
                      <span
                        class="mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-base-content text-xs font-semibold text-base-100"
                      >
                        {{ index + 1 }}
                      </span>
                      <span>{{ operation }}</span>
                    </li>
                  </ol>
                </div>
              </div>

              <div
                v-for="section in activeDocument.detailSections"
                :key="section.id"
                class="rounded-[1.4rem] border border-base-300 bg-base-100 p-5"
              >
                <h3 class="text-base font-semibold text-base-content">{{ section.title }}</h3>
                <p
                  v-if="section.description"
                  class="mt-2 text-sm leading-7 text-base-content/72"
                >
                  {{ section.description }}
                </p>

                <ol
                  v-if="section.items?.length && section.ordered"
                  class="mt-4 space-y-2 text-sm leading-7 text-base-content/72"
                >
                  <li
                    v-for="(item, index) in section.items"
                    :key="item"
                    class="flex gap-3"
                  >
                    <span
                      class="mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-base-content text-xs font-semibold text-base-100"
                    >
                      {{ index + 1 }}
                    </span>
                    <span>{{ item }}</span>
                  </li>
                </ol>

                <ul
                  v-else-if="section.items?.length"
                  class="mt-4 space-y-2 text-sm leading-7 text-base-content/72"
                >
                  <li v-for="item in section.items" :key="item" class="flex gap-2">
                    <span class="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-primary"></span>
                    <span>{{ item }}</span>
                  </li>
                </ul>

                <div v-if="section.imageSrc" class="mt-4 flex flex-col items-start gap-2">
                  <img
                    :src="section.imageSrc"
                    :alt="section.imageAlt || ''"
                    class="w-44 h-44 rounded-xl border border-base-300 bg-base-100 object-contain"
                  />
                </div>

                <div v-if="section.codeBlocks?.length" class="mt-4 space-y-4">
                  <div
                    v-for="block in section.codeBlocks"
                    :key="block.id"
                    class="overflow-hidden rounded-[1.1rem] border border-base-300 bg-base-200/70"
                  >
                    <div
                      class="border-b border-base-300 px-4 py-2 text-xs font-semibold uppercase tracking-[0.16em] text-base-content/55"
                    >
                      {{ block.label }}
                    </div>
                    <pre class="overflow-x-auto px-4 py-4 text-xs leading-6 text-base-content"><code>{{ block.code }}</code></pre>
                  </div>
                </div>
              </div>
            </section>

            <section
              v-else-if="activeDocument.kind === 'faq'"
              class="rounded-[1.4rem] border border-base-300 bg-base-100 p-5"
            >
              <div class="text-sm font-semibold text-base-content">
                {{ isZh ? '常见问题' : 'Common Questions' }}
              </div>
              <div class="mt-4 grid gap-4 xl:grid-cols-2">
                <button
                  v-for="link in activeDocument.links"
                  :key="link.id"
                  class="rounded-[1.2rem] border border-base-300 bg-base-100 p-5 text-left transition-colors hover:bg-base-200"
                  type="button"
                  @click="selectDocument(link.id)"
                >
                  <div class="text-base font-semibold text-base-content">{{ link.title }}</div>
                  <p v-if="link.description" class="mt-2 text-sm leading-7 text-base-content/70">
                    {{ link.description }}
                  </p>
                </button>
              </div>
            </section>

            <section
              v-else
              class="rounded-[1.4rem] border border-base-300 bg-base-100 p-6"
            >
              <div class="text-sm leading-7 text-base-content/70">
                {{ isZh ? 'FAQ 详情按单条问题展示，方便按需查看，不再和其他内容混排。' : 'FAQ entries are displayed one at a time so they can be reviewed without mixing with other content.' }}
              </div>
            </section>

            <footer
              v-if="activeDocument.kind === 'overview'"
              class="border-t border-base-300 pt-5 text-sm text-base-content/60"
            >
              {{ content.closingNote }}
            </footer>
          </div>
        </article>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { getHelpCenterContent } from '@/views/help-center/helpCenterContent'
import {
  buildHelpCenterDirectory,
  type HelpCenterDirectoryDocument,
} from '@/views/help-center/helpCenterDirectory'

const { locale, t } = useI18n()

const activeDocumentId = ref('overview')
const expandedNodeIds = ref<string[]>([])
const content = computed(() => getHelpCenterContent(locale.value))
const helpCenter = computed(() => buildHelpCenterDirectory(content.value))
const navTree = computed(() => helpCenter.value.navTree)
const activeDocument = computed(() => helpCenter.value.documents[activeDocumentId.value])
const activeParentDocument = computed(() => {
  const parentId = activeDocument.value.parentId
  return parentId ? helpCenter.value.documents[parentId] : null
})
const isZh = computed(() => locale.value.startsWith('zh'))
const featureDescriptionLabel = computed(() => (isZh.value ? '功能说明' : 'What It Covers'))
const operationGuideLabel = computed(() => (isZh.value ? '操作说明' : 'How To Use It'))

function isNodeExpanded(id: string): boolean {
  return expandedNodeIds.value.includes(id)
}

function toggleNodeExpansion(id: string) {
  expandedNodeIds.value = isNodeExpanded(id)
    ? expandedNodeIds.value.filter(nodeId => nodeId !== id)
    : [...expandedNodeIds.value, id]
}

function expandNode(id: string) {
  if (!isNodeExpanded(id)) {
    expandedNodeIds.value = [...expandedNodeIds.value, id]
  }
}

function selectDocument(id: string) {
  activeDocumentId.value = id

  const parentId = helpCenter.value.parentById[id]
  if (parentId) {
    expandNode(parentId)
  }

  const node = navTree.value.find(item => item.id === id)
  if (node && node.children.length > 0) {
    expandNode(id)
  }
}

function handleParentNodeClick(id: string, hasChildren: boolean) {
  if (!hasChildren) {
    selectDocument(id)
    return
  }

  if (activeDocumentId.value === id && isNodeExpanded(id)) {
    toggleNodeExpansion(id)
    return
  }

  selectDocument(id)
}

function getParentNodeClass(id: string): string {
  const parentId = helpCenter.value.parentById[activeDocumentId.value]
  if (activeDocumentId.value === id) {
    return 'bg-primary text-primary-content shadow-sm'
  }

  if (parentId === id) {
    return 'bg-primary/12 text-primary'
  }

  return 'text-base-content/72 hover:bg-base-200'
}

function getDocumentKindLabel(kind: HelpCenterDirectoryDocument['kind']): string {
  if (isZh.value) {
    switch (kind) {
      case 'overview':
        return '总览'
      case 'workflow':
        return '路径'
      case 'workflow-step':
        return '步骤'
      case 'catalog-section':
        return '目录'
      case 'feature-entry':
        return '功能'
      case 'faq':
        return '问题集'
      case 'faq-entry':
        return '问题'
    }
  }

  switch (kind) {
    case 'overview':
      return 'Overview'
    case 'workflow':
      return 'Workflow'
    case 'workflow-step':
      return 'Step'
    case 'catalog-section':
      return 'Directory'
    case 'feature-entry':
      return 'Feature'
    case 'faq':
      return 'FAQ'
    case 'faq-entry':
      return 'Question'
  }
}

function getDocumentBadgeClass(id: string, parentId: string | null): string {
  const toneId = parentId === 'workflow' || parentId === 'faq' ? parentId : parentId ?? id

  switch (toneId) {
    case 'overview':
    case 'global-controls':
      return 'bg-primary/10 text-primary'
    case 'workflow':
    case 'core-features':
    case 'faq':
      return 'bg-secondary/10 text-secondary'
    case 'tools-and-content':
      return 'bg-accent/10 text-accent'
    case 'system-and-collaboration':
      return 'bg-info/10 text-info'
    default:
      return 'bg-base-200 text-base-content/70'
  }
}

watch(
  () => locale.value,
  () => {
    activeDocumentId.value = 'overview'
    expandedNodeIds.value = []
  }
)

async function closeWindow() {
  try {
    await getCurrentWebviewWindow().close()
  } catch {
    window.close()
  }
}
</script>

<style scoped>
.help-center-window {
  position: relative;
  scrollbar-gutter: stable;
  background:
    radial-gradient(circle at top left, rgb(14 165 233 / 0.14), transparent 26%),
    radial-gradient(circle at top right, rgb(251 191 36 / 0.12), transparent 24%),
    linear-gradient(180deg, rgb(248 250 252 / 0.96), transparent 24rem);
}

.help-center-backdrop {
  position: fixed;
  inset: 0;
  pointer-events: none;
  background-image:
    linear-gradient(rgb(15 23 42 / 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgb(15 23 42 / 0.03) 1px, transparent 1px);
  background-size: 28px 28px;
  mask-image: linear-gradient(180deg, rgb(0 0 0 / 0.95), transparent 90%);
}

@media (prefers-color-scheme: dark) {
  .help-center-window {
    background:
      radial-gradient(circle at top left, rgb(14 165 233 / 0.14), transparent 26%),
      radial-gradient(circle at top right, rgb(251 191 36 / 0.08), transparent 24%),
      linear-gradient(180deg, rgb(10 15 23 / 0.98), transparent 24rem);
  }

  .help-center-backdrop {
    background-image:
      linear-gradient(rgb(255 255 255 / 0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgb(255 255 255 / 0.03) 1px, transparent 1px);
  }
}
</style>
