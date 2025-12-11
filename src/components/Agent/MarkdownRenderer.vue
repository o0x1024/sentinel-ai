<template>
  <div class="markdown-body leading-relaxed text-sm text-base-content" v-html="renderedHtml"></div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'

const props = defineProps<{
  content: string
}>()

// Configure marked
marked.setOptions({
  gfm: true,
  breaks: true,
})

const renderedHtml = computed(() => {
  try {
    let content = props.content
    
    // 高亮知识库引用 [SOURCE n]
    content = content.replace(
      /\[SOURCE\s+(\d+)\]/gi,
      '<span class="source-citation" title="知识库引用 #$1">[SOURCE $1]</span>'
    )
    
    return marked(content)
  } catch (e) {
    console.error('Markdown parsing error:', e)
    return props.content
  }
})
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
</style>
