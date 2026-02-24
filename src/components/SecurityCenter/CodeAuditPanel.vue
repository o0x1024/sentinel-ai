<template>
  <div class="space-y-6">
    <!-- Sub-tabs: Findings / Rules -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 p-3">
      <div class="flex items-center justify-between">
        <div class="tabs tabs-boxed bg-base-200/60">
          <a
            class="tab gap-1"
            :class="{ 'tab-active': activeSubTab === 'findings' }"
            @click="activeSubTab = 'findings'"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            {{ $t('agent.findingsTab') }}
            <span v-if="findingsCount > 0" class="badge badge-sm badge-error">{{ findingsCount }}</span>
          </a>
          <a
            class="tab gap-1"
            :class="{ 'tab-active': activeSubTab === 'rules' }"
            @click="activeSubTab = 'rules'; loadRulesIfNeeded()"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 104 0M9 5a2 2 0 014 0m-6 9l2 2 4-4" />
            </svg>
            {{ $t('agent.securityRulesTab') }}
            <span v-if="rulesCount > 0" class="badge badge-sm badge-ghost">{{ rulesCount }}</span>
          </a>
        </div>
      </div>
    </div>

    <!-- Findings Sub-tab -->
    <CodeAuditFindingsPanel v-if="activeSubTab === 'findings'" @count-updated="findingsCount = $event" />

    <!-- Rules Sub-tab -->
    <div v-if="activeSubTab === 'rules'" class="space-y-4">
      <!-- Rules Toolbar -->
      <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 p-4">
        <div class="flex flex-wrap items-center gap-3">
          <button class="btn btn-sm btn-primary gap-1" @click="startNewRule">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            {{ $t('agent.addRule') }}
          </button>
          <button class="btn btn-sm btn-outline gap-1" @click="seedBuiltinRules" :disabled="seedingRules">
            <svg v-if="seedingRules" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            {{ $t('agent.seedBuiltinRules') }}
          </button>
          <div class="flex-1"></div>
          <div class="form-control">
            <select v-model="ruleFilters.severity" class="select select-bordered select-sm" @change="loadRules">
              <option value="">{{ $t('agent.ruleSeverity') }}</option>
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
              <option value="info">Info</option>
            </select>
          </div>
          <div class="form-control">
            <select v-model="ruleFilterBuiltin" class="select select-bordered select-sm" @change="loadRules">
              <option value="">全部类型</option>
              <option value="true">{{ $t('agent.ruleBuiltinBadge') }}</option>
              <option value="false">{{ $t('agent.ruleCustomBadge') }}</option>
            </select>
          </div>
          <input
            v-model="ruleFilters.search"
            type="text"
            class="input input-bordered input-sm w-48"
            placeholder="搜索规则..."
            @input="debouncedLoadRules"
          />
        </div>
      </div>

      <!-- Rule Editor -->
      <div v-if="editingRule" class="bg-base-100 rounded-lg shadow-sm border border-primary/30 p-5 space-y-4">
        <div class="flex items-center justify-between">
          <h4 class="text-lg font-semibold flex items-center gap-2">
            <svg class="w-5 h-5 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
            {{ editingRule.id ? $t('agent.editRule') : $t('agent.addRule') }}
          </h4>
          <button class="btn btn-sm btn-ghost" @click="editingRule = null">
            ✕
          </button>
        </div>

        <!-- Basic Info -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('agent.ruleName') }}</span></label>
            <input v-model="editingRule.name" type="text" class="input input-bordered" :placeholder="$t('agent.ruleNamePlaceholder')" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('agent.ruleCwe') }}</span></label>
            <input v-model="editingRule.cwe" type="text" class="input input-bordered" :placeholder="$t('agent.ruleCwePlaceholder')" />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">{{ $t('agent.ruleSeverity') }}</span></label>
            <select v-model="editingRule.severity" class="select select-bordered">
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
              <option value="info">Info</option>
            </select>
          </div>
          <div class="form-control">
            <label class="label cursor-pointer justify-start gap-3">
              <input type="checkbox" v-model="editingRule.enabled" class="checkbox checkbox-primary" />
              <span class="label-text">{{ $t('agent.ruleEnabled') }}</span>
            </label>
          </div>
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">{{ $t('agent.ruleDescription') }}</span></label>
          <textarea v-model="editingRule.description" class="textarea textarea-bordered h-20" :placeholder="$t('agent.ruleDescriptionPlaceholder')"></textarea>
        </div>

        <!-- Pattern Sections -->
        <div class="divider text-xs opacity-50">Source / Sink / Sanitizer</div>

        <PatternEditor
          :title="$t('agent.ruleSources')"
          :hint="$t('agent.ruleSourcesHint')"
          :patterns="editingRule.sources"
          badge-color="badge-info"
          icon-class="text-info"
          @add="addPattern(editingRule.sources)"
          @remove="removePattern(editingRule.sources, $event)"
        />

        <PatternEditor
          :title="$t('agent.ruleSinks')"
          :hint="$t('agent.ruleSinksHint')"
          :patterns="editingRule.sinks"
          badge-color="badge-error"
          icon-class="text-error"
          @add="addPattern(editingRule.sinks)"
          @remove="removePattern(editingRule.sinks, $event)"
        />

        <PatternEditor
          :title="$t('agent.ruleSanitizers')"
          :hint="$t('agent.ruleSanitizersHint')"
          :patterns="editingRule.sanitizers"
          badge-color="badge-success"
          icon-class="text-success"
          @add="addPattern(editingRule.sanitizers)"
          @remove="removePattern(editingRule.sanitizers, $event)"
        />

        <!-- Actions -->
        <div class="flex justify-end gap-2 pt-3 border-t border-base-300">
          <button class="btn btn-ghost" @click="editingRule = null">{{ $t('agent.cancelEdit') }}</button>
          <button class="btn btn-primary gap-1" @click="saveRule" :disabled="savingRule">
            <span v-if="savingRule" class="loading loading-spinner loading-xs"></span>
            {{ $t('agent.saveRule') }}
          </button>
        </div>
      </div>

      <!-- Rules List -->
      <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
        <div v-if="loadingRules" class="text-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else-if="rules.length === 0" class="text-center py-12 text-base-content/50">
          <svg class="w-12 h-12 mx-auto mb-3 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m5.618-4.764A9 9 0 1112 3a9 9 0 017.618 4.236z" />
          </svg>
          <p class="text-sm">{{ $t('agent.noSecurityRules') }}</p>
          <p class="text-xs mt-1 opacity-60">点击「{{ $t('agent.seedBuiltinRules') }}」导入默认规则</p>
        </div>

        <table v-else class="table table-zebra">
          <thead>
            <tr>
              <th>{{ $t('agent.ruleSeverity') }}</th>
              <th>{{ $t('agent.ruleName') }}</th>
              <th>{{ $t('agent.ruleCwe') }}</th>
              <th>Patterns</th>
              <th>类型</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="rule in rules" :key="rule.id">
              <td>
                <span class="badge badge-sm" :class="getSeverityBadgeClass(rule.severity)">{{ rule.severity }}</span>
              </td>
              <td class="font-medium max-w-xs">
                <div class="line-clamp-1">{{ rule.name }}</div>
                <div v-if="rule.description" class="text-xs text-base-content/50 line-clamp-1 mt-0.5">{{ rule.description }}</div>
              </td>
              <td>
                <span class="badge badge-outline badge-xs">{{ rule.cwe || '-' }}</span>
              </td>
              <td>
                <div class="flex gap-2 text-xs">
                  <span class="badge badge-xs badge-info badge-outline" :title="$t('agent.ruleSources')">S:{{ rule.sources.length }}</span>
                  <span class="badge badge-xs badge-error badge-outline" :title="$t('agent.ruleSinks')">K:{{ rule.sinks.length }}</span>
                  <span class="badge badge-xs badge-success badge-outline" :title="$t('agent.ruleSanitizers')">Z:{{ rule.sanitizers.length }}</span>
                </div>
              </td>
              <td>
                <span
                  class="badge badge-xs"
                  :class="rule.is_builtin ? 'badge-info badge-outline' : 'badge-accent badge-outline'"
                >
                  {{ rule.is_builtin ? $t('agent.ruleBuiltinBadge') : $t('agent.ruleCustomBadge') }}
                </span>
              </td>
              <td>
                <input
                  type="checkbox"
                  :checked="rule.enabled"
                  class="toggle toggle-sm toggle-primary"
                  @change="toggleRule(rule)"
                />
              </td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-xs btn-outline" @click="editRule(rule)" :title="$t('agent.editRule')">
                    编辑
                  </button>
                  <button
                    v-if="!rule.is_builtin"
                    class="btn btn-xs btn-error btn-outline"
                    @click="deleteRule(rule)"
                    :title="$t('agent.deleteRule')"
                  >
                    删除
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import CodeAuditFindingsPanel from './CodeAuditFindingsPanel.vue'
import PatternEditor from './PatternEditor.vue'

// ── Types ──────────────────────────────────────────────────

interface PatternSpec {
  name_pattern: string
  arg_pattern?: string
  languages: string[]
  description?: string
}

interface SecurityRule {
  id: string
  name: string
  cwe: string
  severity: string
  description: string
  sources: PatternSpec[]
  sinks: PatternSpec[]
  sanitizers: PatternSpec[]
  is_builtin: boolean
  enabled: boolean
  created_at: string
  updated_at: string
}

interface RuleCommandResponse<T> {
  success: boolean
  data: T | null
  error: string | null
}

interface EditableRule {
  id: string | null
  name: string
  cwe: string
  severity: string
  description: string
  sources: PatternSpec[]
  sinks: PatternSpec[]
  sanitizers: PatternSpec[]
  enabled: boolean
}

const { t } = useI18n()

// ── State ──────────────────────────────────────────────────

const activeSubTab = ref<'findings' | 'rules'>('findings')
const findingsCount = ref(0)

// Rules state
const rules = ref<SecurityRule[]>([])
const rulesCount = ref(0)
const loadingRules = ref(false)
const editingRule = ref<EditableRule | null>(null)
const savingRule = ref(false)
const seedingRules = ref(false)
let rulesLoaded = false

const ruleFilters = ref({
  severity: '',
  search: '',
})
const ruleFilterBuiltin = ref('')

let searchDebounce: ReturnType<typeof setTimeout> | null = null

// ── Severity Badge ─────────────────────────────────────────

const getSeverityBadgeClass = (severity?: string) => {
  switch ((severity || '').toLowerCase()) {
    case 'critical': return 'badge-error'
    case 'high': return 'badge-warning'
    case 'medium': return 'badge-info'
    case 'low': return 'badge-success'
    default: return 'badge-ghost'
  }
}

// ── Rules Loading ──────────────────────────────────────────

function loadRulesIfNeeded() {
  if (!rulesLoaded) loadRules()
}

async function loadRules() {
  loadingRules.value = true
  try {
    const isBuiltin = ruleFilterBuiltin.value === ''
      ? null
      : ruleFilterBuiltin.value === 'true'

    const resp = await invoke<RuleCommandResponse<SecurityRule[]>>('list_cpg_security_rules', {
      severity: ruleFilters.value.severity || null,
      isBuiltin,
      search: ruleFilters.value.search || null,
    })
    if (resp.success && resp.data) {
      rules.value = resp.data
      rulesCount.value = resp.data.length
      rulesLoaded = true
    } else {
      console.error('Failed to load rules:', resp.error)
    }
  } catch (e) {
    console.error('Failed to load rules:', e)
  } finally {
    loadingRules.value = false
  }
}

function debouncedLoadRules() {
  if (searchDebounce) clearTimeout(searchDebounce)
  searchDebounce = setTimeout(() => loadRules(), 300)
}

// ── CRUD Operations ────────────────────────────────────────

function startNewRule() {
  editingRule.value = {
    id: null,
    name: '',
    cwe: '',
    severity: 'medium',
    description: '',
    sources: [],
    sinks: [],
    sanitizers: [],
    enabled: true,
  }
}

function editRule(rule: SecurityRule) {
  editingRule.value = {
    id: rule.id,
    name: rule.name,
    cwe: rule.cwe,
    severity: rule.severity,
    description: rule.description,
    sources: JSON.parse(JSON.stringify(rule.sources)),
    sinks: JSON.parse(JSON.stringify(rule.sinks)),
    sanitizers: JSON.parse(JSON.stringify(rule.sanitizers)),
    enabled: rule.enabled,
  }
  // Scroll to editor
  window.scrollTo({ top: 0, behavior: 'smooth' })
}

async function saveRule() {
  if (!editingRule.value) return
  savingRule.value = true
  try {
    const resp = await invoke<RuleCommandResponse<SecurityRule>>('save_cpg_security_rule', {
      rule: {
        id: editingRule.value.id,
        name: editingRule.value.name,
        cwe: editingRule.value.cwe,
        severity: editingRule.value.severity,
        description: editingRule.value.description,
        sources: editingRule.value.sources,
        sinks: editingRule.value.sinks,
        sanitizers: editingRule.value.sanitizers,
        enabled: editingRule.value.enabled,
      },
    })
    if (resp.success) {
      editingRule.value = null
      await loadRules()
    } else {
      console.error('Failed to save rule:', resp.error)
    }
  } catch (e) {
    console.error('Failed to save rule:', e)
  } finally {
    savingRule.value = false
  }
}

async function toggleRule(rule: SecurityRule) {
  try {
    await invoke('toggle_cpg_security_rule', {
      ruleId: rule.id,
      enabled: !rule.enabled,
    })
    rule.enabled = !rule.enabled
  } catch (e) {
    console.error('Failed to toggle rule:', e)
  }
}

async function deleteRule(rule: SecurityRule) {
  if (!confirm(t('agent.deleteRuleConfirm'))) return
  try {
    await invoke('delete_cpg_security_rule', { ruleId: rule.id })
    await loadRules()
  } catch (e) {
    console.error('Failed to delete rule:', e)
  }
}

async function seedBuiltinRules() {
  if (!confirm(t('agent.seedBuiltinRulesConfirm'))) return
  seedingRules.value = true
  try {
    const resp = await invoke<RuleCommandResponse<number>>('seed_builtin_cpg_rules')
    if (resp.success && resp.data != null) {
      alert(t('agent.seedSuccess', { count: resp.data }))
      await loadRules()
    } else {
      console.error('Failed to seed rules:', resp.error)
    }
  } catch (e) {
    console.error('Failed to seed rules:', e)
  } finally {
    seedingRules.value = false
  }
}

// ── Pattern Helpers ────────────────────────────────────────

function addPattern(patterns: PatternSpec[]) {
  patterns.push({
    name_pattern: '',
    languages: [],
    description: undefined,
  })
}

function removePattern(patterns: PatternSpec[], index: number) {
  patterns.splice(index, 1)
}
</script>
