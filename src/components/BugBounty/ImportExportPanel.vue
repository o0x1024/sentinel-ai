<template>
  <div class="space-y-6">
    <!-- Import Section -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-file-import text-primary mr-2"></i>
          {{ t('bugBounty.importExport.importTitle') }}
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mt-4">
          <!-- Import from Platform -->
          <div class="space-y-4">
            <h4 class="font-medium text-sm">{{ t('bugBounty.importExport.importFromPlatform') }}</h4>
            
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('bugBounty.importExport.selectPlatform') }}</span></label>
              <select v-model="importPlatform" class="select select-bordered">
                <option value="hackerone">HackerOne</option>
                <option value="bugcrowd">Bugcrowd</option>
                <option value="intigriti">Intigriti</option>
                <option value="yeswehack">YesWeHack</option>
                <option value="other">{{ t('bugBounty.importExport.otherPlatform') }}</option>
              </select>
            </div>

            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('bugBounty.importExport.programUrl') }}</span></label>
              <input 
                v-model="importUrl" 
                type="text" 
                class="input input-bordered"
                :placeholder="getUrlPlaceholder()"
              />
            </div>

            <button class="btn btn-primary btn-sm" @click="importFromUrl" :disabled="!importUrl || importing">
              <span v-if="importing" class="loading loading-spinner loading-sm mr-2"></span>
              <i v-else class="fas fa-download mr-2"></i>
              {{ t('bugBounty.importExport.fetchProgram') }}
            </button>
          </div>

          <!-- Import from JSON -->
          <div class="space-y-4">
            <h4 class="font-medium text-sm">{{ t('bugBounty.importExport.importFromJson') }}</h4>
            
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('bugBounty.importExport.pasteJson') }}</span></label>
              <textarea 
                v-model="importJson" 
                class="textarea textarea-bordered font-mono text-sm h-32"
                :placeholder="jsonPlaceholder"
              ></textarea>
            </div>

            <div class="flex gap-2">
              <button class="btn btn-primary btn-sm" @click="importFromJson" :disabled="!importJson || importing">
                <span v-if="importing" class="loading loading-spinner loading-sm mr-2"></span>
                <i v-else class="fas fa-file-import mr-2"></i>
                {{ t('bugBounty.importExport.importJson') }}
              </button>
              <label class="btn btn-ghost btn-sm">
                <i class="fas fa-upload mr-2"></i>
                {{ t('bugBounty.importExport.uploadFile') }}
                <input type="file" accept=".json" class="hidden" @change="handleFileUpload" />
              </label>
            </div>
          </div>
        </div>

        <!-- Import Preview -->
        <div v-if="importPreview" class="mt-4 p-4 bg-base-200 rounded-lg">
          <h4 class="font-medium text-sm mb-2">{{ t('bugBounty.importExport.preview') }}</h4>
          <div class="grid grid-cols-2 gap-4 text-sm">
            <div><span class="text-base-content/60">{{ t('bugBounty.form.programName') }}:</span> {{ importPreview.name }}</div>
            <div><span class="text-base-content/60">{{ t('bugBounty.form.platform') }}:</span> {{ importPreview.platform }}</div>
            <div><span class="text-base-content/60">{{ t('bugBounty.programDetail.scopes') }}:</span> {{ importPreview.scopes?.length || 0 }}</div>
            <div><span class="text-base-content/60">{{ t('bugBounty.form.organization') }}:</span> {{ importPreview.organization }}</div>
          </div>
          <div class="flex gap-2 mt-4">
            <button class="btn btn-success btn-sm" @click="confirmImport" :disabled="importing">
              <i class="fas fa-check mr-2"></i>
              {{ t('bugBounty.importExport.confirmImport') }}
            </button>
            <button class="btn btn-ghost btn-sm" @click="importPreview = null">
              {{ t('common.cancel') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Export Section -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-file-export text-success mr-2"></i>
          {{ t('bugBounty.importExport.exportTitle') }}
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
          <!-- Export Programs -->
          <div class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-medium flex items-center gap-2">
                <i class="fas fa-trophy text-primary"></i>
                {{ t('bugBounty.importExport.exportPrograms') }}
              </h4>
              <p class="text-sm text-base-content/60">{{ t('bugBounty.importExport.exportProgramsDesc') }}</p>
              <div class="card-actions mt-2">
                <button class="btn btn-sm btn-outline" @click="exportData('programs', 'json')">
                  <i class="fas fa-code mr-1"></i> JSON
                </button>
                <button class="btn btn-sm btn-outline" @click="exportData('programs', 'csv')">
                  <i class="fas fa-table mr-1"></i> CSV
                </button>
              </div>
            </div>
          </div>

          <!-- Export Findings -->
          <div class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-medium flex items-center gap-2">
                <i class="fas fa-bug text-error"></i>
                {{ t('bugBounty.importExport.exportFindings') }}
              </h4>
              <p class="text-sm text-base-content/60">{{ t('bugBounty.importExport.exportFindingsDesc') }}</p>
              <div class="card-actions mt-2">
                <button class="btn btn-sm btn-outline" @click="exportData('findings', 'json')">
                  <i class="fas fa-code mr-1"></i> JSON
                </button>
                <button class="btn btn-sm btn-outline" @click="exportData('findings', 'csv')">
                  <i class="fas fa-table mr-1"></i> CSV
                </button>
              </div>
            </div>
          </div>

          <!-- Export Submissions -->
          <div class="card bg-base-200">
            <div class="card-body p-4">
              <h4 class="font-medium flex items-center gap-2">
                <i class="fas fa-paper-plane text-success"></i>
                {{ t('bugBounty.importExport.exportSubmissions') }}
              </h4>
              <p class="text-sm text-base-content/60">{{ t('bugBounty.importExport.exportSubmissionsDesc') }}</p>
              <div class="card-actions mt-2">
                <button class="btn btn-sm btn-outline" @click="exportData('submissions', 'json')">
                  <i class="fas fa-code mr-1"></i> JSON
                </button>
                <button class="btn btn-sm btn-outline" @click="exportData('submissions', 'csv')">
                  <i class="fas fa-table mr-1"></i> CSV
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Export All -->
        <div class="divider">{{ t('bugBounty.importExport.or') }}</div>
        
        <div class="flex flex-wrap gap-2 justify-center">
          <button class="btn btn-success" @click="exportAll('json')">
            <i class="fas fa-download mr-2"></i>
            {{ t('bugBounty.importExport.exportAllJson') }}
          </button>
          <button class="btn btn-outline" @click="exportAll('csv')">
            <i class="fas fa-file-csv mr-2"></i>
            {{ t('bugBounty.importExport.exportAllCsv') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Report Generation -->
    <div class="card bg-base-100 shadow-lg border border-base-200">
      <div class="card-body">
        <h3 class="card-title text-base">
          <i class="fas fa-file-alt text-warning mr-2"></i>
          {{ t('bugBounty.importExport.reportTitle') }}
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.importExport.selectProgram') }}</span></label>
            <select v-model="reportProgramId" class="select select-bordered">
              <option value="">{{ t('bugBounty.importExport.allPrograms') }}</option>
              <option v-for="program in programs" :key="program.id" :value="program.id">
                {{ program.name }}
              </option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.importExport.reportFormat') }}</span></label>
            <select v-model="reportFormat" class="select select-bordered">
              <option value="markdown">Markdown</option>
              <option value="html">HTML</option>
              <option value="pdf">PDF ({{ t('bugBounty.importExport.comingSoon') }})</option>
            </select>
          </div>
        </div>

        <div class="flex gap-2 mt-4">
          <button class="btn btn-warning" @click="generateReport" :disabled="generatingReport">
            <span v-if="generatingReport" class="loading loading-spinner loading-sm mr-2"></span>
            <i v-else class="fas fa-file-alt mr-2"></i>
            {{ t('bugBounty.importExport.generateReport') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const props = defineProps<{
  programs: any[]
  findings: any[]
  submissions: any[]
}>()

const emit = defineEmits<{
  (e: 'imported'): void
}>()

// Import state
const importPlatform = ref('hackerone')
const importUrl = ref('')
const importJson = ref('')
const importPreview = ref<any>(null)
const importing = ref(false)

// Export state
const reportProgramId = ref('')
const reportFormat = ref('markdown')
const generatingReport = ref(false)

const jsonPlaceholder = `{
  "name": "Example Program",
  "organization": "Example Corp",
  "platform": "hackerone",
  "url": "https://hackerone.com/example",
  "scopes": [
    { "target": "*.example.com", "type": "wildcard_domain", "in_scope": true }
  ]
}`

const getUrlPlaceholder = () => {
  const placeholders: Record<string, string> = {
    hackerone: 'https://hackerone.com/company-name',
    bugcrowd: 'https://bugcrowd.com/company-name',
    intigriti: 'https://app.intigriti.com/programs/company',
    yeswehack: 'https://yeswehack.com/programs/company',
    other: 'https://...',
  }
  return placeholders[importPlatform.value] || placeholders.other
}

const importFromUrl = async () => {
  // For now, just parse the URL and create a basic program
  try {
    importing.value = true
    
    const url = new URL(importUrl.value)
    const pathParts = url.pathname.split('/').filter(Boolean)
    const programHandle = pathParts[pathParts.length - 1] || 'unknown'
    
    importPreview.value = {
      name: programHandle.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
      organization: programHandle,
      platform: importPlatform.value,
      url: importUrl.value,
      scopes: [],
    }
    
    toast.info(t('bugBounty.importExport.urlParsed'))
  } catch (error) {
    console.error('Failed to parse URL:', error)
    toast.error(t('bugBounty.importExport.invalidUrl'))
  } finally {
    importing.value = false
  }
}

const importFromJson = async () => {
  try {
    importing.value = true
    const data = JSON.parse(importJson.value)
    
    if (!data.name) {
      toast.error(t('bugBounty.importExport.missingName'))
      return
    }
    
    importPreview.value = {
      name: data.name,
      organization: data.organization || data.name,
      platform: data.platform || importPlatform.value,
      url: data.url || '',
      description: data.description || '',
      scopes: data.scopes || [],
    }
  } catch (error) {
    console.error('Failed to parse JSON:', error)
    toast.error(t('bugBounty.importExport.invalidJson'))
  } finally {
    importing.value = false
  }
}

const handleFileUpload = async (event: Event) => {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  
  try {
    const text = await file.text()
    importJson.value = text
    await importFromJson()
  } catch (error) {
    console.error('Failed to read file:', error)
    toast.error(t('bugBounty.importExport.fileReadError'))
  }
}

const confirmImport = async () => {
  if (!importPreview.value) return
  
  try {
    importing.value = true
    
    // Create program
    const request = {
      name: importPreview.value.name,
      organization: importPreview.value.organization,
      platform: importPreview.value.platform,
      url: importPreview.value.url || null,
      description: importPreview.value.description || null,
      platform_handle: null,
      program_type: null,
      rewards: null,
      rules: null,
      tags: null,
    }
    
    const program: any = await invoke('bounty_create_program', { request })
    
    // Create scopes if any
    if (importPreview.value.scopes?.length > 0) {
      for (const scope of importPreview.value.scopes) {
        const scopeRequest = {
          program_id: program.id,
          scope_type: scope.in_scope !== false ? 'in_scope' : 'out_of_scope',
          target_type: scope.type || 'domain',
          target: scope.target,
          description: scope.description || null,
          allowed_tests: null,
          instructions: null,
          requires_auth: null,
          priority: null,
        }
        await invoke('bounty_create_scope', { request: scopeRequest })
      }
    }
    
    toast.success(t('bugBounty.success.programCreated'))
    importPreview.value = null
    importUrl.value = ''
    importJson.value = ''
    emit('imported')
  } catch (error) {
    console.error('Failed to import:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    importing.value = false
  }
}

const exportData = async (type: 'programs' | 'findings' | 'submissions', format: 'json' | 'csv') => {
  try {
    let data: any[]
    let filename: string
    
    switch (type) {
      case 'programs':
        data = props.programs
        filename = `bounty-programs-${Date.now()}`
        break
      case 'findings':
        data = props.findings
        filename = `bounty-findings-${Date.now()}`
        break
      case 'submissions':
        data = props.submissions
        filename = `bounty-submissions-${Date.now()}`
        break
    }
    
    let content: string
    let ext: string
    
    if (format === 'json') {
      content = JSON.stringify(data, null, 2)
      ext = 'json'
    } else {
      content = convertToCsv(data)
      ext = 'csv'
    }
    
    const filePath = await save({
      defaultPath: `${filename}.${ext}`,
      filters: [{ name: format.toUpperCase(), extensions: [ext] }],
    })
    
    if (filePath) {
      await writeTextFile(filePath, content)
      toast.success(t('bugBounty.importExport.exportSuccess'))
    }
  } catch (error) {
    console.error('Failed to export:', error)
    toast.error(t('bugBounty.importExport.exportError'))
  }
}

const exportAll = async (format: 'json' | 'csv') => {
  try {
    const data = {
      programs: props.programs,
      findings: props.findings,
      submissions: props.submissions,
      exported_at: new Date().toISOString(),
    }
    
    let content: string
    let ext: string
    
    if (format === 'json') {
      content = JSON.stringify(data, null, 2)
      ext = 'json'
    } else {
      // For CSV, export each type separately in a combined file
      content = '# Programs\n' + convertToCsv(props.programs) + '\n\n# Findings\n' + convertToCsv(props.findings) + '\n\n# Submissions\n' + convertToCsv(props.submissions)
      ext = 'csv'
    }
    
    const filePath = await save({
      defaultPath: `bounty-export-${Date.now()}.${ext}`,
      filters: [{ name: format.toUpperCase(), extensions: [ext] }],
    })
    
    if (filePath) {
      await writeTextFile(filePath, content)
      toast.success(t('bugBounty.importExport.exportSuccess'))
    }
  } catch (error) {
    console.error('Failed to export:', error)
    toast.error(t('bugBounty.importExport.exportError'))
  }
}

const convertToCsv = (data: any[]): string => {
  if (!data.length) return ''
  
  const headers = Object.keys(data[0])
  const rows = data.map(item => 
    headers.map(h => {
      const val = item[h]
      if (val === null || val === undefined) return ''
      if (typeof val === 'object') return JSON.stringify(val).replace(/"/g, '""')
      return String(val).replace(/"/g, '""')
    }).map(v => `"${v}"`).join(',')
  )
  
  return [headers.join(','), ...rows].join('\n')
}

const generateReport = async () => {
  try {
    generatingReport.value = true
    
    const filteredFindings = reportProgramId.value 
      ? props.findings.filter(f => f.program_id === reportProgramId.value)
      : props.findings
    
    const filteredSubmissions = reportProgramId.value
      ? props.submissions.filter(s => s.program_id === reportProgramId.value)
      : props.submissions
    
    const program = reportProgramId.value 
      ? props.programs.find(p => p.id === reportProgramId.value)
      : null
    
    let content: string
    
    if (reportFormat.value === 'markdown') {
      content = generateMarkdownReport(program, filteredFindings, filteredSubmissions)
    } else {
      content = generateHtmlReport(program, filteredFindings, filteredSubmissions)
    }
    
    const ext = reportFormat.value === 'markdown' ? 'md' : 'html'
    const filename = program ? `report-${program.name.toLowerCase().replace(/\s+/g, '-')}` : 'bounty-report'
    
    const filePath = await save({
      defaultPath: `${filename}-${Date.now()}.${ext}`,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    })
    
    if (filePath) {
      await writeTextFile(filePath, content)
      toast.success(t('bugBounty.importExport.reportGenerated'))
    }
  } catch (error) {
    console.error('Failed to generate report:', error)
    toast.error(t('bugBounty.importExport.reportError'))
  } finally {
    generatingReport.value = false
  }
}

const generateMarkdownReport = (program: any, findings: any[], submissions: any[]): string => {
  const title = program ? `Bug Bounty Report: ${program.name}` : 'Bug Bounty Report'
  const date = new Date().toLocaleDateString()
  
  let md = `# ${title}\n\n`
  md += `**Generated:** ${date}\n\n`
  
  if (program) {
    md += `## Program Information\n\n`
    md += `- **Organization:** ${program.organization}\n`
    md += `- **Platform:** ${program.platform}\n`
    md += `- **Status:** ${program.status}\n`
    if (program.url) md += `- **URL:** ${program.url}\n`
    md += '\n'
  }
  
  md += `## Summary\n\n`
  md += `- **Total Findings:** ${findings.length}\n`
  md += `- **Total Submissions:** ${submissions.length}\n`
  
  const accepted = submissions.filter(s => ['accepted', 'resolved'].includes(s.status?.toLowerCase()))
  md += `- **Accepted Submissions:** ${accepted.length}\n`
  
  const totalReward = submissions.reduce((sum, s) => sum + (s.reward_amount || 0) + (s.bonus_amount || 0), 0)
  md += `- **Total Earnings:** $${totalReward.toFixed(2)}\n\n`
  
  if (findings.length > 0) {
    md += `## Findings\n\n`
    md += `| Title | Type | Severity | Status |\n`
    md += `|-------|------|----------|--------|\n`
    findings.forEach(f => {
      md += `| ${f.title} | ${f.finding_type} | ${f.severity} | ${f.status} |\n`
    })
    md += '\n'
  }
  
  if (submissions.length > 0) {
    md += `## Submissions\n\n`
    md += `| Title | Status | Reward | Submitted |\n`
    md += `|-------|--------|--------|----------|\n`
    submissions.forEach(s => {
      const reward = (s.reward_amount || 0) + (s.bonus_amount || 0)
      const submitted = s.submitted_at ? new Date(s.submitted_at).toLocaleDateString() : '-'
      md += `| ${s.title} | ${s.status} | $${reward.toFixed(0)} | ${submitted} |\n`
    })
  }
  
  return md
}

const generateHtmlReport = (program: any, findings: any[], submissions: any[]): string => {
  const md = generateMarkdownReport(program, findings, submissions)
  
  // Simple markdown to HTML conversion
  let html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>Bug Bounty Report</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 2rem; }
    h1 { color: #1f2937; border-bottom: 2px solid #3b82f6; padding-bottom: 0.5rem; }
    h2 { color: #374151; margin-top: 2rem; }
    table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
    th, td { padding: 0.75rem; text-align: left; border-bottom: 1px solid #e5e7eb; }
    th { background: #f3f4f6; font-weight: 600; }
    tr:hover { background: #f9fafb; }
  </style>
</head>
<body>
`
  
  // Convert markdown to HTML (simple conversion)
  html += md
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^\*\*(.+?):\*\* (.+)$/gm, '<p><strong>$1:</strong> $2</p>')
    .replace(/^- \*\*(.+?):\*\* (.+)$/gm, '<p>• <strong>$1:</strong> $2</p>')
    .replace(/\| (.+) \|/g, (match) => {
      const cells = match.split('|').filter(Boolean).map(c => c.trim())
      return '<tr>' + cells.map(c => `<td>${c}</td>`).join('') + '</tr>'
    })
    .replace(/\|[-\s|]+\|/g, '')
    .replace(/<tr>(<td>[^<]+<\/td>)+<\/tr>/g, (match, _, offset, str) => {
      if (str.indexOf(match) === str.indexOf('<tr>')) {
        return match.replace(/<td>/g, '<th>').replace(/<\/td>/g, '</th>')
      }
      return match
    })
  
  html += `
</body>
</html>`
  
  return html
}
</script>
