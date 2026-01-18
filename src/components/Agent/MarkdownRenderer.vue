<template>
  <div 
    class="markdown-body leading-relaxed text-sm text-base-content" 
    ref="markdownBodyRef"
    v-html="renderedHtml"
    @click="handleClick"
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
import { ref, computed, watch, nextTick, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { marked } from 'marked'
import hljs from 'highlight.js'

const props = defineProps<{
  content: string
  citations?: any[]
  showTableDownload?: boolean
}>()

const emit = defineEmits<{
  (e: 'downloadTable', tableIndex: number): void
  (e: 'renderHtml', htmlContent: string): void
}>()

const { t } = useI18n()

// Handle citation click/hover to show details
const showCitationDetails = ref(false)
const selectedCitation = ref<any>(null)
const citationPosition = ref({ x: 0, y: 0 })
const markdownBodyRef = ref<HTMLElement | null>(null)

// Configure marked with highlight.js for code highlighting
const markedOptions = {
  gfm: true,
  breaks: true,
  async: false as const,
}

// Store code blocks for copy/render functionality
const codeBlocks = ref<{ code: string; lang: string }[]>([])

// Custom renderer for code highlighting with action buttons
const renderer = new marked.Renderer()
renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
  const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext'
  const highlighted = hljs.highlight(text, { language }).value
  const blockIndex = codeBlocks.value.length
  codeBlocks.value.push({ code: text, lang: language })
  
  // Show render button only for html/svg/xml
  const isRenderable = ['html', 'svg', 'xml'].includes(language.toLowerCase())
  const renderBtn = isRenderable 
    ? `<button class="code-render-btn" data-code-index="${blockIndex}" title="${t('agent.renderHtml')}"><i class="fas fa-play"></i></button>`
    : ''
  
  return `<div class="code-block-wrapper">
    <div class="code-block-header">
      <span class="code-lang">${language}</span>
      <div class="code-actions">
        ${renderBtn}
        <button class="code-copy-btn" data-code-index="${blockIndex}" title="${t('agent.copyCode')}"><i class="fas fa-copy"></i></button>
      </div>
    </div>
    <pre><code class="hljs language-${language}">${highlighted}</code></pre>
  </div>`
}

marked.use({ renderer })

const renderedHtml = computed(() => {
  try {
    // Reset code blocks for each render
    codeBlocks.value = []
    
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
    
    // Add blinking cursor
    content = content.replace(
      /▍/g,
      '<span class="typing-cursor">▍</span>'
    )
    
    // Use synchronous parsing with async: false
    return marked.parse(content, markedOptions) as string
  } catch (e) {
    console.error('Markdown parsing error:', e)
    return props.content
  }
})

// Wrap tables with download button after DOM update
const wrapTablesWithDownloadButton = () => {
  if (!markdownBodyRef.value || !props.showTableDownload) return
  
  const tables = markdownBodyRef.value.querySelectorAll('table')
  tables.forEach((table, index) => {
    // Skip if already wrapped
    if (table.parentElement?.classList.contains('table-wrapper')) return
    
    // Create wrapper
    const wrapper = document.createElement('div')
    wrapper.className = 'table-wrapper'
    
    // Create download button
    const btn = document.createElement('button')
    btn.className = 'table-download-btn'
    btn.type = 'button'
    btn.setAttribute('data-table-index', String(index))
    btn.innerHTML = `<i class="fas fa-download"></i><span>${t('agent.download')}</span>`
    btn.title = t('agent.download')
    btn.addEventListener('click', (event) => {
      event.preventDefault()
      event.stopPropagation()
      emit('downloadTable', index)
    })
    
    // Insert wrapper before table
    table.parentNode?.insertBefore(wrapper, table)
    wrapper.appendChild(btn)
    wrapper.appendChild(table)
  })
}

// Watch for rendered HTML changes and wrap tables
watch(renderedHtml, () => {
  nextTick(() => {
    wrapTablesWithDownloadButton()
  })
})

onMounted(() => {
  nextTick(() => {
    wrapTablesWithDownloadButton()
  })
})

// Copy code to clipboard
const copyCode = async (index: number) => {
  const block = codeBlocks.value[index]
  if (!block) return
  
  try {
    await navigator.clipboard.writeText(block.code)
    console.log('[MarkdownRenderer] Code copied to clipboard')
  } catch (e) {
    console.error('[MarkdownRenderer] Failed to copy code:', e)
  }
}

// Render HTML code
const renderHtmlCode = (index: number) => {
  const block = codeBlocks.value[index]
  if (!block) return
  
  emit('renderHtml', block.code)
}

const handleClick = (event: MouseEvent) => {
  const target = event.target as HTMLElement
  
  // Handle citation click
  if (target.classList.contains('source-citation')) {
    const index = parseInt(target.getAttribute('data-index') || '-1')
    if (props.citations && props.citations[index]) {
      selectedCitation.value = props.citations[index]
      citationPosition.value = { x: event.clientX, y: event.clientY }
      showCitationDetails.value = true
    }
    return
  }
  
  // Handle code copy button click
  const copyBtn = target.closest('.code-copy-btn') as HTMLElement
  if (copyBtn) {
    event.preventDefault()
    event.stopPropagation()
    const idx = parseInt(copyBtn.getAttribute('data-code-index') || '0')
    copyCode(idx)
    
    // Visual feedback
    const icon = copyBtn.querySelector('i')
    if (icon) {
      icon.className = 'fas fa-check'
      setTimeout(() => {
        icon.className = 'fas fa-copy'
      }, 1500)
    }
    return
  }
  
  // Handle code render button click
  const renderBtn = target.closest('.code-render-btn') as HTMLElement
  if (renderBtn) {
    event.preventDefault()
    event.stopPropagation()
    const idx = parseInt(renderBtn.getAttribute('data-code-index') || '0')
    renderHtmlCode(idx)
    return
  }
  
  // Handle table download button click
  const btn = target.closest('.table-download-btn') as HTMLElement
  if (btn) {
    event.preventDefault()
    event.stopPropagation()
    const idx = parseInt(btn.getAttribute('data-table-index') || '0')
    console.log('[MarkdownRenderer] Download table clicked, index:', idx)
    emit('downloadTable', idx)
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

/* Code block wrapper */
.markdown-body :deep(.code-block-wrapper) {
  position: relative;
  margin: 0.75rem 0;
  border-radius: 0.5rem;
  overflow: hidden;
  background: #1e1e2e;
}

.markdown-body :deep(.code-block-header) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  background: #181825;
  border-bottom: 1px solid #313244;
}

.markdown-body :deep(.code-lang) {
  font-size: 0.75rem;
  color: #6c7086;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  text-transform: lowercase;
}

.markdown-body :deep(.code-actions) {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.markdown-body :deep(.code-copy-btn),
.markdown-body :deep(.code-render-btn) {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 0.25rem;
  color: #6c7086;
  cursor: pointer;
  transition: all 0.2s;
}

.markdown-body :deep(.code-copy-btn:hover),
.markdown-body :deep(.code-render-btn:hover) {
  background: #313244;
  color: #cdd6f4;
}

.markdown-body :deep(.code-render-btn) {
  color: #a6e3a1;
}

.markdown-body :deep(.code-render-btn:hover) {
  background: rgba(166, 227, 161, 0.2);
  color: #a6e3a1;
}

.markdown-body :deep(.code-copy-btn i),
.markdown-body :deep(.code-render-btn i) {
  font-size: 0.875rem;
}

/* Code blocks */
.markdown-body :deep(.code-block-wrapper pre) {
  margin: 0;
  padding: 1rem;
  background: #1e1e2e;
  border-radius: 0;
  overflow-x: auto;
}

.markdown-body :deep(pre) {
  margin: 0.75rem 0;
  padding: 1rem;
  background: #1e1e2e;
  border-radius: 0.5rem;
  overflow-x: auto;
}

.markdown-body :deep(pre code) {
  padding: 0;
  background: none;
  font-size: 0.8125rem;
  line-height: 1.5;
}

/* Highlight.js theme - Catppuccin Mocha inspired */
.markdown-body :deep(.hljs) {
  color: #cdd6f4;
  background: #1e1e2e;
}

.markdown-body :deep(.hljs-keyword),
.markdown-body :deep(.hljs-selector-tag),
.markdown-body :deep(.hljs-built_in),
.markdown-body :deep(.hljs-name) {
  color: #cba6f7;
}

.markdown-body :deep(.hljs-string),
.markdown-body :deep(.hljs-title),
.markdown-body :deep(.hljs-section),
.markdown-body :deep(.hljs-attribute),
.markdown-body :deep(.hljs-literal),
.markdown-body :deep(.hljs-template-tag),
.markdown-body :deep(.hljs-template-variable),
.markdown-body :deep(.hljs-type) {
  color: #a6e3a1;
}

.markdown-body :deep(.hljs-number),
.markdown-body :deep(.hljs-symbol),
.markdown-body :deep(.hljs-bullet),
.markdown-body :deep(.hljs-link),
.markdown-body :deep(.hljs-meta),
.markdown-body :deep(.hljs-selector-id),
.markdown-body :deep(.hljs-title.class_) {
  color: #fab387;
}

.markdown-body :deep(.hljs-emphasis) {
  font-style: italic;
}

.markdown-body :deep(.hljs-strong) {
  font-weight: bold;
}

.markdown-body :deep(.hljs-comment),
.markdown-body :deep(.hljs-quote) {
  color: #6c7086;
  font-style: italic;
}

.markdown-body :deep(.hljs-doctag) {
  color: #f38ba8;
}

.markdown-body :deep(.hljs-formula) {
  color: #94e2d5;
}

.markdown-body :deep(.hljs-variable),
.markdown-body :deep(.hljs-params) {
  color: #f5c2e7;
}

.markdown-body :deep(.hljs-function) {
  color: #89b4fa;
}

.markdown-body :deep(.hljs-class .hljs-title) {
  color: #f9e2af;
}

.markdown-body :deep(.hljs-tag) {
  color: #89dceb;
}

.markdown-body :deep(.hljs-attr) {
  color: #f9e2af;
}

.markdown-body :deep(.hljs-regexp) {
  color: #f38ba8;
}

.markdown-body :deep(.hljs-deletion) {
  color: #f38ba8;
  background: rgba(243, 139, 168, 0.1);
}

.markdown-body :deep(.hljs-addition) {
  color: #a6e3a1;
  background: rgba(166, 227, 161, 0.1);
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

/* Table wrapper with download button */
.markdown-body :deep(.table-wrapper) {
  position: relative;
  margin: 1rem 0;
}

.markdown-body :deep(.table-download-btn) {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: hsl(var(--b2));
  border: 1px solid hsl(var(--b3));
  border-radius: 0.25rem;
  color: hsl(var(--bc) / 0.7);
  cursor: pointer;
  transition: all 0.2s;
}

.markdown-body :deep(.table-download-btn:hover) {
  background: hsl(var(--b3));
  color: hsl(var(--bc));
}

.markdown-body :deep(.table-download-btn i) {
  font-size: 0.625rem;
}

/* Tables - use explicit border properties to override Tailwind reset */
.markdown-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  border-spacing: 0;
  margin: 0;
  margin-top: 1.5rem;
  font-size: 0.875rem;
  border-width: 1px !important;
  border-style: solid !important;
  border-color: rgba(128, 128, 128, 0.3) !important;
}

.markdown-body :deep(.table-wrapper table) {
  margin-top: 1.75rem;
}

.markdown-body :deep(thead) {
  border-bottom-width: 2px;
  border-bottom-style: solid;
  border-bottom-color: rgba(128, 128, 128, 0.4);
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 0.5rem 0.75rem;
  text-align: left;
  border-width: 1px !important;
  border-style: solid !important;
  border-color: rgba(128, 128, 128, 0.25) !important;
}

.markdown-body :deep(th) {
  background: var(--fallback-b3, oklch(var(--b3) / 1));
  font-weight: 600;
}

.markdown-body :deep(tr:nth-child(even)) {
  background: var(--fallback-b2, oklch(var(--b2) / 0.5));
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

/* Typing cursor */
@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.markdown-body :deep(.typing-cursor) {
  display: inline-block;
  width: 0.5em;
  animation: blink 1s infinite;
  color: hsl(var(--p));
  font-weight: bold;
  vertical-align: middle;
  margin-left: 2px;
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
