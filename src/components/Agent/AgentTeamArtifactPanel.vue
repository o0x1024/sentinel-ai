<template>
  <div class="artifact-panel flex flex-col h-full overflow-hidden bg-base-100">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-base-300 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <i class="fas fa-file-code text-primary"></i>
        <h3 class="font-bold text-sm text-base-content">产物文档</h3>
        <span class="badge badge-xs badge-primary">{{ artifacts.length }}</span>
      </div>
      <div class="flex items-center gap-2">
        <!-- Export all -->
        <button
          v-if="artifacts.length > 0"
          class="btn btn-xs btn-ghost gap-1 text-base-content/60"
          @click="exportAll"
          title="导出全部产物"
        >
          <i class="fas fa-download text-xs"></i>
          导出全部
        </button>
        <button v-if="showClose" class="btn btn-xs btn-ghost" @click="emit('close')">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- Artifact list or empty -->
    <div v-if="artifacts.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3 text-base-content/30">
      <i class="fas fa-file-circle-plus text-3xl"></i>
      <p class="text-sm">团队会话结束后自动生成产物文档</p>
    </div>

    <div v-else class="flex-1 flex overflow-hidden min-h-0">
      <!-- Left: artifact list -->
      <div class="w-48 flex-shrink-0 border-r border-base-200 overflow-y-auto">
        <div
          v-for="art in artifacts"
          :key="art.id"
          class="group px-3 py-2.5 border-b border-base-100 cursor-pointer transition-colors"
          :class="selectedArtifact?.id === art.id
            ? 'bg-primary/10 border-l-2 border-l-primary'
            : 'hover:bg-base-200/50'"
          @click="selectArtifact(art)"
          :id="`artifact-item-${art.id}`"
        >
          <div class="flex items-center gap-2">
            <i :class="artifactIcon(art.artifact_type)" class="text-sm flex-shrink-0"></i>
            <div class="min-w-0">
              <div class="text-xs font-medium text-base-content/85 truncate leading-tight">
                {{ art.title }}
              </div>
              <div class="text-[10px] text-base-content/40 mt-0.5">
                v{{ art.version }} · {{ formatDate(art.created_at) }}
              </div>
            </div>
          </div>
          <!-- Judge score badge -->
          <div v-if="getJudgeScore(art)" class="mt-1">
            <span class="badge badge-xs" :class="judgeScoreBadgeClass(getJudgeScore(art)!)">
              Judge: {{ (getJudgeScore(art)! * 10).toFixed(1) }}/10
            </span>
          </div>
        </div>
      </div>

      <!-- Right: artifact preview -->
      <div class="flex-1 overflow-hidden flex flex-col min-h-0" v-if="selectedArtifact">
        <!-- Artifact header -->
        <div class="px-4 py-2.5 border-b border-base-200 flex items-center gap-2 bg-base-50/50">
          <i :class="artifactIcon(selectedArtifact.artifact_type)" class="text-primary"></i>
          <div class="flex-1 min-w-0">
            <div class="text-sm font-semibold text-base-content truncate">{{ selectedArtifact.title }}</div>
            <div class="text-xs text-base-content/40">
              版本 {{ selectedArtifact.version }} · {{ formatTime(selectedArtifact.created_at) }}
            </div>
          </div>
          <!-- Actions -->
          <div class="flex items-center gap-1.5 flex-shrink-0">
            <!-- Version history -->
            <button
              class="btn btn-xs btn-ghost gap-1"
              :class="showVersionHistory ? 'text-primary' : 'text-base-content/50'"
              @click="showVersionHistory = !showVersionHistory"
              title="版本历史"
            >
              <i class="fas fa-history text-xs"></i>
              <span class="text-xs">v{{ selectedArtifact.version }}</span>
            </button>

            <!-- View mode toggle -->
            <div class="flex rounded-lg overflow-hidden border border-base-300">
              <button
                class="px-2 py-1 text-xs transition-colors"
                :class="viewMode === 'preview' ? 'bg-primary text-white' : 'bg-base-100 text-base-content/50'"
                @click="viewMode = 'preview'"
              >预览</button>
              <button
                class="px-2 py-1 text-xs transition-colors"
                :class="viewMode === 'source' ? 'bg-primary text-white' : 'bg-base-100 text-base-content/50'"
                @click="viewMode = 'source'"
              >源码</button>
            </div>

            <!-- Export single -->
            <button
              class="btn btn-xs btn-primary gap-1"
              @click="exportArtifact(selectedArtifact)"
              title="导出此文档"
            >
              <i class="fas fa-download text-xs"></i>
              导出
            </button>
          </div>
        </div>

        <!-- Version history panel -->
        <div v-if="showVersionHistory" class="border-b border-base-200 bg-base-50/80 px-4 py-2">
          <div class="text-xs font-medium text-base-content/60 mb-1.5">版本历史</div>
          <div class="flex gap-2 overflow-x-auto">
            <div
              v-for="v in artifactVersions"
              :key="v.version"
              class="flex-shrink-0 px-2.5 py-1.5 rounded-lg border cursor-pointer transition-all text-xs"
              :class="selectedVersion === v.version
                ? 'border-primary bg-primary/10 text-primary font-medium'
                : 'border-base-300 text-base-content/60 hover:border-primary/40'"
              @click="selectedVersion = v.version"
            >
              <div class="font-mono">v{{ v.version }}</div>
              <div class="text-base-content/40 text-[10px]">{{ v.generated_at }}</div>
              <div v-if="v.diff_summary" class="text-[10px] text-warning mt-0.5 max-w-[120px] truncate">
                {{ v.diff_summary }}
              </div>
            </div>
          </div>
        </div>

        <!-- Judge panel (if applicable) -->
        <div
          v-if="judgeReview && showJudge"
          class="border-b border-base-200 px-4 py-2.5"
          :class="judgeReview.verdict === 'approve' ? 'bg-success/5' : 'bg-warning/5'"
        >
          <div class="flex items-start gap-3">
            <div class="flex-shrink-0 w-8 h-8 rounded-full bg-neutral/10 flex items-center justify-center">
              <i class="fas fa-gavel text-sm" :class="judgeReview.verdict === 'approve' ? 'text-success' : 'text-warning'"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs font-bold text-base-content/70">Tenth-Man Judge</span>
                <span class="badge badge-xs" :class="judgeBadgeClass">{{ judgeVerdictLabel }}</span>
                <span class="text-xs text-base-content/40">评分: {{ (judgeReview.score * 10).toFixed(1) }}/10</span>
              </div>
              <div v-if="judgeReview.blind_spots?.length" class="text-xs text-base-content/60">
                <span class="font-medium text-warning">盲点: </span>{{ judgeReview.blind_spots[0] }}
                <span v-if="judgeReview.blind_spots.length > 1" class="text-base-content/40"> 等{{ judgeReview.blind_spots.length }}项</span>
              </div>
            </div>
            <button class="btn btn-xs btn-ghost text-base-content/30 flex-shrink-0" @click="showJudge = false">
              <i class="fas fa-times text-xs"></i>
            </button>
          </div>
        </div>

        <!-- Content area -->
        <div class="flex-1 overflow-y-auto">
          <!-- Preview mode (rendered markdown) -->
          <div
            v-if="viewMode === 'preview'"
            class="prose prose-sm max-w-none p-4 text-base-content/85"
            v-html="renderedContent"
          ></div>

          <!-- Source mode -->
          <div v-else class="p-4">
            <pre class="text-xs font-mono text-base-content/75 whitespace-pre-wrap break-words leading-relaxed bg-base-200/30 rounded-xl p-4">{{ selectedArtifact.content }}</pre>
          </div>
        </div>
      </div>

      <!-- Empty selection -->
      <div v-else class="flex-1 flex items-center justify-center text-base-content/30 text-sm">
        <div class="text-center">
          <i class="fas fa-mouse-pointer text-xl mb-2 block"></i>
          点击左侧选择产物文档
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AgentTeamArtifact } from '@/types/agentTeam'

// ==================== Types ====================

interface ArtifactVersion {
  version: number
  generated_at: string
  diff_summary?: string
}

interface JudgeReview {
  score: number
  blind_spots: string[]
  risks: string[]
  verdict: 'approve' | 'request_revision' | 'reject'
}

// ==================== Props / Emits ====================

const props = defineProps<{
  artifacts: AgentTeamArtifact[]
  judgeReviews?: Record<string, JudgeReview>
  showClose?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'export', artifact: AgentTeamArtifact): void
}>()

// ==================== State ====================

const selectedArtifact = ref<AgentTeamArtifact | null>(null)
const viewMode = ref<'preview' | 'source'>('preview')
const showVersionHistory = ref(false)
const showJudge = ref(true)
const selectedVersion = ref<number | null>(null)

// Simulated version history (in real app, fetched from backend)
const artifactVersions = computed<ArtifactVersion[]>(() => {
  if (!selectedArtifact.value) return []
  const versions: ArtifactVersion[] = []
  for (let v = 1; v <= selectedArtifact.value.version; v++) {
    versions.push({
      version: v,
      generated_at: new Date().toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }),
      diff_summary: v > 1 ? '更新了安全章节' : undefined,
    })
  }
  return versions
})

// ==================== Computed ====================

const judgeReview = computed<JudgeReview | null>(() => {
  if (!selectedArtifact.value || !props.judgeReviews) return null
  return props.judgeReviews[selectedArtifact.value.id] ?? null
})

const judgeBadgeClass = computed(() => {
  if (!judgeReview.value) return 'badge-ghost'
  if (judgeReview.value.verdict === 'approve') return 'badge-success'
  if (judgeReview.value.verdict === 'reject') return 'badge-error'
  return 'badge-warning'
})

const judgeVerdictLabel = computed(() => {
  if (!judgeReview.value) return ''
  if (judgeReview.value.verdict === 'approve') return '通过'
  if (judgeReview.value.verdict === 'reject') return '否决'
  return '建议修订'
})

const renderedContent = computed(() => {
  if (!selectedArtifact.value) return ''
  // Simple Markdown rendering (could use marked.js in production)
  return markdownToHtml(selectedArtifact.value.content)
})

// ==================== Actions ====================

function selectArtifact(art: AgentTeamArtifact) {
  selectedArtifact.value = art
  selectedVersion.value = art.version
  showVersionHistory.value = false
  showJudge.value = true
}

function exportArtifact(art: AgentTeamArtifact) {
  emit('export', art)
  const blob = new Blob([art.content], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${art.title.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_')}_v${art.version}.md`
  a.click()
  URL.revokeObjectURL(url)
}

function exportAll() {
  for (const art of props.artifacts) {
    exportArtifact(art)
  }
}

function getJudgeScore(art: AgentTeamArtifact): number | null {
  if (!props.judgeReviews) return null
  return props.judgeReviews[art.id]?.score ?? null
}

// ==================== Display helpers ====================

const ARTIFACT_ICONS: Record<string, string> = {
  prd: 'fas fa-clipboard-list text-primary',
  architecture: 'fas fa-sitemap text-info',
  detailed_design: 'fas fa-drafting-compass text-secondary',
  test_plan: 'fas fa-vial text-warning',
  workflow_tasks: 'fas fa-tasks text-success',
  vulnerability_report: 'fas fa-bug text-error',
  incident_review: 'fas fa-shield-halved text-error',
  audit_summary: 'fas fa-search text-warning',
  remediation_plan: 'fas fa-tools text-success',
  red_team_report: 'fas fa-crosshairs text-error',
  blue_team_response: 'fas fa-shield text-info',
  lessons_learned: 'fas fa-graduation-cap text-accent',
  change_plan: 'fas fa-code-commit text-warning',
  rollback_plan: 'fas fa-rotate-left text-secondary',
}

function artifactIcon(type: string): string {
  return ARTIFACT_ICONS[type] ?? 'fas fa-file-alt text-base-content/50'
}

function judgeScoreBadgeClass(score: number): string {
  if (score >= 0.75) return 'badge-success'
  if (score >= 0.5) return 'badge-warning'
  return 'badge-error'
}

function formatDate(ts: string): string {
  try {
    return new Date(ts).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
  } catch { return '' }
}

function formatTime(ts: string): string {
  try {
    return new Date(ts).toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
  } catch { return '' }
}

// Simple Markdown → HTML (no external dep needed for basic rendering)
function markdownToHtml(md: string): string {
  return md
    .replace(/^#{6}\s(.+)$/gm, '<h6 class="text-xs font-bold mt-2">$1</h6>')
    .replace(/^#{5}\s(.+)$/gm, '<h5 class="text-xs font-bold mt-2">$1</h5>')
    .replace(/^#{4}\s(.+)$/gm, '<h4 class="text-sm font-bold mt-3">$1</h4>')
    .replace(/^###\s(.+)$/gm, '<h3 class="text-sm font-bold text-base-content mt-3 mb-1">$1</h3>')
    .replace(/^##\s(.+)$/gm, '<h2 class="text-base font-bold text-primary mt-4 mb-1 pb-1 border-b border-base-300">$1</h2>')
    .replace(/^#\s(.+)$/gm, '<h1 class="text-lg font-bold text-base-content mt-2 mb-3">$1</h1>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code class="bg-base-300 rounded px-1 text-xs font-mono">$1</code>')
    .replace(/^-\s(.+)$/gm, '<li class="ml-4 text-xs list-disc">$1</li>')
    .replace(/^\d+\.\s(.+)$/gm, '<li class="ml-4 text-xs list-decimal">$1</li>')
    .replace(/^---$/gm, '<hr class="border-base-300 my-3" />')
    .replace(/\n\n/g, '</p><p class="text-xs leading-relaxed mb-2 text-base-content/75">')
    .replace(/^(.+)$/m, '<p class="text-xs leading-relaxed mb-2 text-base-content/75">$1')
}
</script>

<style scoped>
.prose h1 { margin-top: 0.5rem; }
.prose li + li { margin-top: 0.125rem; }
</style>
