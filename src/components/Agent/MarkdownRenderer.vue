<template>
  <div 
    class="markdown-body leading-relaxed text-sm text-base-content" 
    v-html="renderedHtml"
    @click="handleCitationClick"
  ></div>

  <!-- Citation Detail Popover -->
  <Teleport to="body">
    <div 
      v-if="showCitationDetails" 
      class="citation-popover fixed z-[9999] bg-base-100 shadow-2xl rounded-xl border border-base-300 p-4 w-80 md:w-96 animate-in fade-in zoom-in duration-200"
      :style="{ left: `${citationPosition.x}px`, top: `${citationPosition.y}px`, transform: 'translate(-50%, -100%) translateY(-10px)' }"
    >
      <div class="flex justify-between items-start mb-2">
        <div class="flex items-center gap-2">
          <i class="fas fa-file-alt text-primary"></i>
          <span class="font-bold text-sm truncate max-w-[200px]">{{ selectedCitation?.file_name }}</span>
        </div>
        <button @click="showCitationDetails = false" class="btn btn-ghost btn-xs btn-circle">
          <i class="fas fa-times"></i>
        </button>
      </div>
      
      <div class="space-y-2">
        <div class="flex items-center gap-4 text-[10px] text-base-content/60">
          <span v-if="selectedCitation?.page_number">Page: {{ selectedCitation.page_number }}</span>
          <span v-if="selectedCitation?.score">Score: {{ (selectedCitation.score * 100).toFixed(1) }}%</span>
        </div>
        
        <div class="bg-base-200/50 rounded-lg p-3 text-xs leading-relaxed max-h-48 overflow-y-auto italic border-l-2 border-primary/30">
          "{{ selectedCitation?.content_preview }}"
        </div>
        
        <div v-if="selectedCitation?.file_path" class="text-[10px] text-base-content/40 truncate mt-2">
          Path: {{ selectedCitation.file_path }}
        </div>
      </div>
      
      <div class="mt-3 flex justify-end">
        <button class="btn btn-primary btn-xs" @click="showCitationDetails = false">
          {{ t('agent.close') }}
        </button>
      </div>
    </div>
    
    <!-- Backdrop to close popover -->
    <div 
      v-if="showCitationDetails" 
      class="fixed inset-0 z-[9998] bg-transparent" 
      @click="showCitationDetails = false"
    ></div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { marked } from 'marked'

const props = defineProps<{
  content: string
  citations?: any[]
}>()

const { t } = useI18n()

// Handle citation click/hover to show details
const showCitationDetails = ref(false)
const selectedCitation = ref<any>(null)
const citationPosition = ref({ x: 0, y: 0 })

// Configure marked
marked.setOptions({
  gfm: true,
  breaks: true,
})

const renderedHtml = computed(() => {
  try {
    let content = props.content
    
    // Highlight knowledge base citations [SOURCE n]
    // Use data-index to associate with citations array
    content = content.replace(
      /\[SOURCE\s+(\d+)\]/gi,
      (match, p1) => {
        const index = parseInt(p1) - 1
        const citation = props.citations && props.citations[index]
        const title = citation ? `${citation.file_name} (Page ${citation.page_number || 1})` : t('agent.sourceCitation', { number: p1 })
        return `<span class="source-citation" data-index="${index}" title="${title}">[SOURCE ${p1}]</span>`
      }
    )
    
    return marked(content)
  } catch (e) {
    console.error('Markdown parsing error:', e)
    return props.content
  }
})

const handleCitationClick = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  if (target.classList.contains('source-citation')) {
    const index = parseInt(target.getAttribute('data-index') || '-1')
    if (props.citations && props.citations[index]) {
      selectedCitation.value = props.citations[index]
      citationPosition.value = { x: event.clientX, y: event.clientY }
      showCitationDetails.value = true
    }
  }
}
</script>

<style scoped>
/* Headings */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  font-weight: 600;
  line-height: 1.3;
  color: hsl(var(--bc));
}

.markdown-body :deep(h1) { font-size: 1.5rem; }
.markdown-body :deep(h2) { font-size: 1.25rem; }
.markdown-body :deep(h3) { font-size: 1.125rem; }
.markdown-body :deep(h4) { font-size: 1rem; }

/* Paragraphs */
.markdown-body :deep(p) {
  margin: 0.5rem 0;
}

/* Inline code */
.markdown-body :deep(code) {
  padding: 0.125rem 0.375rem;
  background: hsl(var(--b3));
  border-radius: 0.25rem;
  font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
  font-size: 0.875em;
  color: hsl(var(--p));
}

/* Code blocks */
.markdown-body :deep(pre) {
  margin: 0.75rem 0;
  padding: 1rem;
  background: hsl(var(--b3));
  border-radius: 0.5rem;
  overflow-x: auto;
}

.markdown-body :deep(pre code) {
  padding: 0;
  background: none;
  color: hsl(var(--bc));
  font-size: 0.8125rem;
  line-height: 1.5;
}

/* Blockquotes */
.markdown-body :deep(blockquote) {
  margin: 0.75rem 0;
  padding: 0.5rem 1rem;
  border-left: 3px solid hsl(var(--p));
  background: hsl(var(--b2));
  color: hsl(var(--bc) / 0.7);
}

.markdown-body :deep(blockquote p) {
  margin: 0;
}

/* Tables */
.markdown-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 1rem 0;
  font-size: 0.875rem;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid hsl(var(--b3));
  text-align: left;
}

.markdown-body :deep(th) {
  background: hsl(var(--b3));
  font-weight: 600;
}

.markdown-body :deep(tr:nth-child(even)) {
  background: hsl(var(--b2));
}

/* Lists */
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 1.5rem;
  margin: 0.5rem 0;
}

.markdown-body :deep(li) {
  margin: 0.25rem 0;
}

/* Task lists */
.markdown-body :deep(li.task-list-item) {
  list-style: none;
  margin-left: -1.25rem;
}

.markdown-body :deep(input[type="checkbox"]) {
  margin-right: 0.5rem;
  vertical-align: middle;
}

/* Links */
.markdown-body :deep(a) {
  color: hsl(var(--p));
  text-decoration: none;
}

.markdown-body :deep(a:hover) {
  text-decoration: underline;
}

/* Horizontal rules */
.markdown-body :deep(hr) {
  margin: 1rem 0;
  border: none;
  border-top: 1px solid hsl(var(--b3));
}

/* Strong and emphasis */
.markdown-body :deep(strong) {
  font-weight: 600;
  color: hsl(var(--bc));
}

.markdown-body :deep(em) {
  font-style: italic;
}

/* Images */
.markdown-body :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
  margin: 0.5rem 0;
}

/* Knowledge base source citations */
.markdown-body :deep(.source-citation) {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.5rem;
  margin: 0 0.125rem;
  background: hsl(var(--in) / 0.15);
  color: hsl(var(--in));
  border: 1px solid hsl(var(--in) / 0.3);
  border-radius: 0.375rem;
  font-size: 0.75rem;
  font-weight: 600;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  cursor: help;
  transition: all 0.2s ease;
}

.markdown-body :deep(.source-citation:hover) {
  background: hsl(var(--in) / 0.25);
  border-color: hsl(var(--in) / 0.5);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px hsl(var(--in) / 0.2);
}

.citation-popover {
  filter: drop-shadow(0 10px 15px rgba(0, 0, 0, 0.1));
}

.citation-popover::after {
  content: '';
  position: absolute;
  bottom: -6px;
  left: 50%;
  transform: translateX(-50%) rotate(45deg);
  width: 12px;
  height: 12px;
  background: hsl(var(--b1));
  border-right: 1px solid hsl(var(--b3));
  border-bottom: 1px solid hsl(var(--b3));
}
</style>
