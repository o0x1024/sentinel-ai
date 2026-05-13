<template>
  <div
    class="bounty-markdown rounded-lg bg-base-200 p-4 text-sm"
    :class="{ 'text-base-content/50': !content }"
    v-html="renderedContent"
  ></div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { renderBountyKnowledgeMarkdown } from './bountyKnowledgeMarkdown'

const props = defineProps<{
  content?: string | null
  emptyText?: string
}>()

const renderedContent = computed(() => (
  props.content
    ? renderBountyKnowledgeMarkdown(props.content)
    : renderBountyKnowledgeMarkdown(props.emptyText || '-')
))
</script>

<style scoped>
.bounty-markdown :deep(:first-child) {
  margin-top: 0;
}

.bounty-markdown :deep(:last-child) {
  margin-bottom: 0;
}

.bounty-markdown :deep(h1),
.bounty-markdown :deep(h2),
.bounty-markdown :deep(h3),
.bounty-markdown :deep(h4) {
  margin: 0.75rem 0 0.375rem;
  font-weight: 650;
  line-height: 1.25;
}

.bounty-markdown :deep(h1) { font-size: 1.25rem; }
.bounty-markdown :deep(h2) { font-size: 1.125rem; }
.bounty-markdown :deep(h3) { font-size: 1rem; }
.bounty-markdown :deep(h4) { font-size: 0.875rem; }

.bounty-markdown :deep(p),
.bounty-markdown :deep(ul),
.bounty-markdown :deep(ol),
.bounty-markdown :deep(pre),
.bounty-markdown :deep(blockquote),
.bounty-markdown :deep(table) {
  margin: 0.5rem 0;
}

.bounty-markdown :deep(ul),
.bounty-markdown :deep(ol) {
  padding-left: 1.25rem;
}

.bounty-markdown :deep(ul) {
  list-style: disc;
}

.bounty-markdown :deep(ol) {
  list-style: decimal;
}

.bounty-markdown :deep(code) {
  border-radius: 0.25rem;
  background: hsl(var(--b3));
  padding: 0.125rem 0.25rem;
  font-size: 0.85em;
}

.bounty-markdown :deep(pre) {
  overflow-x: auto;
  border-radius: 0.5rem;
  background: hsl(var(--b3));
  padding: 0.75rem;
}

.bounty-markdown :deep(pre code) {
  background: transparent;
  padding: 0;
}

.bounty-markdown :deep(blockquote) {
  border-left: 3px solid hsl(var(--p) / 0.45);
  padding-left: 0.75rem;
  color: hsl(var(--bc) / 0.72);
}

.bounty-markdown :deep(a) {
  color: hsl(var(--p));
  text-decoration: underline;
  text-underline-offset: 2px;
}

.bounty-markdown :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
  margin: 0.5rem 0;
}

.bounty-markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.875rem;
}

.bounty-markdown :deep(th),
.bounty-markdown :deep(td) {
  border: 1px solid hsl(var(--b3));
  padding: 0.375rem 0.5rem;
}
</style>

