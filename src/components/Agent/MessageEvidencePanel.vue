<template>
  <div v-if="hasEvidence" class="evidence-panel">
    <div class="evidence-header">
      <span class="evidence-title">{{ t('agent.evidence') }}</span>
    </div>

    <div v-if="toolFact" class="evidence-section">
      <div class="evidence-section-label">{{ t('agent.evidenceTool') }}</div>
      <div class="tool-fact-card">
        <div class="tool-fact-top">
          <span class="tool-fact-name">{{ toolFact.toolName }}</span>
          <span v-if="toolFact.status" class="tool-fact-status">{{ toolFact.status }}</span>
        </div>
        <div class="tool-fact-meta">
          <span v-if="toolFact.toolCallId">ID {{ toolFact.toolCallId }}</span>
          <span v-if="toolFact.duration">{{ toolFact.duration }}</span>
        </div>
        <p class="tool-fact-copy">{{ t('agent.toolOutputCaptured') }}</p>
      </div>
    </div>

    <div v-if="visibleCitations.length > 0" class="evidence-section">
      <div class="evidence-section-label">{{ t('agent.evidenceSources') }}</div>
      <div class="evidence-list">
        <article v-for="citation in visibleCitations" :key="citation.id || citation.source_id" class="evidence-card">
          <div class="evidence-card-top">
            <span class="evidence-file" :title="citation.file_name">{{ citation.file_name }}</span>
            <span v-if="citation.page_number" class="evidence-meta-chip">
              {{ t('agent.pageShort') }} {{ citation.page_number }}
            </span>
            <span v-if="typeof citation.score === 'number'" class="evidence-meta-chip">
              {{ t('agent.scoreShort') }} {{ Math.round(citation.score * 100) }}%
            </span>
          </div>
          <p class="evidence-preview">{{ citation.content_preview }}</p>
        </article>
      </div>
      <div v-if="overflowCount > 0" class="evidence-overflow">
        {{ t('agent.evidenceOverflow', { count: overflowCount }) }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

interface CitationLike {
  id?: string
  source_id?: string
  file_name: string
  page_number?: number
  score?: number
  content_preview: string
}

interface ToolFact {
  toolName: string
  status?: string
  toolCallId?: string
  duration?: string
}

const props = defineProps<{
  citations?: CitationLike[]
  toolFact?: ToolFact | null
}>()

const { t } = useI18n()

const visibleCitations = computed(() => (props.citations || []).slice(0, 3))
const overflowCount = computed(() => Math.max(0, (props.citations?.length || 0) - visibleCitations.value.length))
const hasEvidence = computed(() => visibleCitations.value.length > 0 || Boolean(props.toolFact))
</script>

<style scoped>
.evidence-panel {
  margin-bottom: 0.95rem;
  padding: 0.8rem 0.85rem;
  border-radius: 15px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.65), rgba(248, 250, 252, 0.72));
}

.evidence-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.65rem;
}

.evidence-title {
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 58%, transparent);
}

.evidence-section + .evidence-section {
  margin-top: 0.8rem;
}

.evidence-section-label {
  margin-bottom: 0.45rem;
  font-size: 0.72rem;
  font-weight: 700;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 60%, transparent);
}

.tool-fact-card,
.evidence-card {
  border-radius: 12px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  background: rgba(255, 255, 255, 0.72);
  padding: 0.68rem 0.75rem;
}

.tool-fact-top,
.evidence-card-top {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.38rem;
}

.tool-fact-name,
.evidence-file {
  font-size: 0.82rem;
  font-weight: 700;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 86%, transparent);
}

.tool-fact-status,
.evidence-meta-chip {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 0.14rem 0.42rem;
  font-size: 0.68rem;
  background: rgba(59, 130, 246, 0.08);
  color: rgb(37, 99, 235);
}

.tool-fact-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 0.35rem;
  font-size: 0.72rem;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 55%, transparent);
}

.tool-fact-copy,
.evidence-preview,
.evidence-overflow {
  margin: 0.4rem 0 0;
  font-size: 0.76rem;
  line-height: 1.58;
  color: color-mix(in srgb, var(--fallback-bc, oklch(var(--bc) / 1)) 64%, transparent);
}

.evidence-list {
  display: grid;
  gap: 0.55rem;
}
</style>
