<template>
  <div v-if="useStructuredLayout" class="structured-message">
    <section v-if="summarySection" class="structured-panel structured-panel-summary">
      <div class="structured-label">{{ summarySection.label }}</div>
      <MarkdownRenderer
        :content="summarySection.content"
        :citations="citations"
        @render-html="(html: string) => emit('renderHtml', html)"
      />
    </section>

    <div v-if="detailSections.length > 0" class="structured-grid">
      <section
        v-for="section in detailSections"
        :key="section.key"
        :class="['structured-panel', section.panelClass]"
      >
        <div class="structured-label">{{ section.label }}</div>
        <MarkdownRenderer
          :content="section.content"
          :citations="citations"
          @render-html="(html: string) => emit('renderHtml', html)"
        />
      </section>
    </div>

    <section v-if="remainingSection" class="structured-panel structured-panel-supporting">
      <div class="structured-label">{{ remainingSection.label }}</div>
      <MarkdownRenderer
        :content="remainingSection.content"
        :citations="citations"
        @render-html="(html: string) => emit('renderHtml', html)"
      />
    </section>
  </div>

  <MarkdownRenderer
    v-else
    :content="content"
    :citations="citations"
    :show-table-download="showTableDownload"
    @download-table="(tableIndex: number) => emit('downloadTable', tableIndex)"
    @render-html="(html: string) => emit('renderHtml', html)"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import MarkdownRenderer from './MarkdownRenderer.vue'

type StructuredSection = {
  key: string
  label: string
  content: string
  panelClass: string
}

const props = defineProps<{
  content: string
  rawContent: string
  messageType: string
  citations?: any[]
  showTableDownload?: boolean
}>()

const emit = defineEmits<{
  (e: 'downloadTable', tableIndex: number): void
  (e: 'renderHtml', htmlContent: string): void
}>()

const STRUCTURED_TYPES = new Set(['final', 'thinking', 'progress', 'error'])

const sectionTitleMap: Array<{
  kind: 'summary' | 'evidence' | 'next' | 'details'
  patterns: RegExp[]
}> = [
  {
    kind: 'summary',
    patterns: [/^(结论|总结|摘要|概述|overview|summary|conclusion)\s*[:：]?$/i],
  },
  {
    kind: 'evidence',
    patterns: [/^(证据|依据|发现|分析|observations|evidence|findings|analysis)\s*[:：]?$/i],
  },
  {
    kind: 'next',
    patterns: [/^(下一步|建议|行动|后续|next steps?|actions?|recommendations?)\s*[:：]?$/i],
  },
  {
    kind: 'details',
    patterns: [/^(补充|细节|说明|details?|notes?)\s*[:：]?$/i],
  },
]

const splitMarkdownSections = (raw: string) => {
  const sections: Array<{ title: string; body: string }> = []
  const lines = raw.split('\n')
  let currentTitle = ''
  let currentLines: string[] = []

  const pushSection = () => {
    const body = currentLines.join('\n').trim()
    if (!body) return
    sections.push({ title: currentTitle.trim(), body })
  }

  for (const line of lines) {
    const headingMatch = line.match(/^\s{0,3}#{1,6}\s+(.+?)\s*$/)
    if (headingMatch) {
      pushSection()
      currentTitle = headingMatch[1]
      currentLines = []
      continue
    }
    currentLines.push(line)
  }

  pushSection()
  return sections
}

const classifySection = (title: string): 'summary' | 'evidence' | 'next' | 'details' | null => {
  const normalized = title.trim()
  if (!normalized) return null
  for (const rule of sectionTitleMap) {
    if (rule.patterns.some(pattern => pattern.test(normalized))) {
      return rule.kind
    }
  }
  return null
}

const firstParagraph = (raw: string) => {
  return raw
    .split(/\n\s*\n/)
    .map(part => part.trim())
    .find(Boolean) || ''
}

const listLikeChunks = (raw: string) => {
  return raw
    .split(/\n(?=\s*(?:[-*+]|\d+\.)\s+)/)
    .map(part => part.trim())
    .filter(Boolean)
}

const structuredSections = computed(() => {
  if (!STRUCTURED_TYPES.has(props.messageType)) return null
  if (props.showTableDownload) return null

  const raw = props.rawContent.trim()
  if (!raw || raw.includes('```')) return null

  const explicitSections = splitMarkdownSections(raw)
  const sectionMap = new Map<string, StructuredSection>()
  const remainingBodies: string[] = []

  for (const section of explicitSections) {
    const kind = classifySection(section.title)
    if (!kind) {
      remainingBodies.push(`### ${section.title}\n\n${section.body}`)
      continue
    }
    const panelClass =
      kind === 'summary'
        ? 'structured-panel-summary'
        : kind === 'evidence'
          ? 'structured-panel-evidence'
          : kind === 'next'
            ? 'structured-panel-next'
            : 'structured-panel-supporting'

    const label =
      kind === 'summary'
        ? '结论'
        : kind === 'evidence'
          ? '证据'
          : kind === 'next'
            ? '下一步'
            : '补充'

    sectionMap.set(kind, {
      key: kind,
      label,
      content: section.body,
      panelClass,
    })
  }

  if (sectionMap.size > 0) {
    return {
      summary: sectionMap.get('summary') || null,
      details: ['evidence', 'next', 'details']
        .map(key => sectionMap.get(key))
        .filter((item): item is StructuredSection => Boolean(item)),
      remaining: remainingBodies.length
        ? {
            key: 'remaining',
            label: '补充说明',
            content: remainingBodies.join('\n\n'),
            panelClass: 'structured-panel-supporting',
          }
        : null,
    }
  }

  if (props.messageType !== 'final') return null

  const summary = firstParagraph(raw)
  if (!summary || summary.length < 24) return null

  const remainder = raw.slice(raw.indexOf(summary) + summary.length).trim()
  const listChunks = listLikeChunks(remainder)

  let evidenceContent = ''
  let nextContent = ''
  let restContent = remainder

  if (listChunks.length >= 2) {
    evidenceContent = listChunks[0]
    nextContent = listChunks[1]
    restContent = remainder
      .replace(listChunks[0], '')
      .replace(listChunks[1], '')
      .trim()
  }

  return {
    summary: {
      key: 'summary',
      label: '结论',
      content: summary,
      panelClass: 'structured-panel-summary',
    },
    details: [
      evidenceContent
        ? {
            key: 'evidence',
            label: '证据',
            content: evidenceContent,
            panelClass: 'structured-panel-evidence',
          }
        : null,
      nextContent
        ? {
            key: 'next',
            label: '下一步',
            content: nextContent,
            panelClass: 'structured-panel-next',
          }
        : null,
    ].filter((item): item is StructuredSection => Boolean(item)),
    remaining: restContent
      ? {
          key: 'remaining',
          label: '补充说明',
          content: restContent,
          panelClass: 'structured-panel-supporting',
        }
      : null,
  }
})

const useStructuredLayout = computed(() => Boolean(structuredSections.value))
const summarySection = computed(() => structuredSections.value?.summary || null)
const detailSections = computed(() => structuredSections.value?.details || [])
const remainingSection = computed(() => structuredSections.value?.remaining || null)
</script>

<style scoped>
.structured-message {
  display: grid;
  gap: 0.75rem;
}

.structured-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.structured-panel {
  border-radius: 14px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  padding: 0.8rem 0.85rem;
  background: rgba(255, 255, 255, 0.58);
}

.structured-panel-summary {
  background: linear-gradient(180deg, rgba(16, 185, 129, 0.09), rgba(255, 255, 255, 0.72));
}

.structured-panel-evidence {
  background: linear-gradient(180deg, rgba(59, 130, 246, 0.08), rgba(255, 255, 255, 0.72));
}

.structured-panel-next {
  background: linear-gradient(180deg, rgba(245, 158, 11, 0.09), rgba(255, 255, 255, 0.72));
}

.structured-panel-supporting {
  background: rgba(255, 255, 255, 0.5);
}

.structured-label {
  margin-bottom: 0.45rem;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 55%, transparent);
}

.structured-panel :deep(.markdown-body) {
  font-size: 0.88rem;
}

.structured-panel :deep(.markdown-body p:last-child) {
  margin-bottom: 0;
}

@media (max-width: 768px) {
  .structured-grid {
    grid-template-columns: 1fr;
  }
}
</style>
