<template>
  <div v-if="visible" class="modal modal-open">
    <div class="modal-box max-w-4xl max-h-[90vh] flex flex-col">
      <h3 class="font-bold text-lg mb-4">{{ t('bugBounty.createFinding') }}</h3>
      
      <!-- Template Selector -->
      <div class="bg-base-200 rounded-lg p-3 mb-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <i class="fas fa-file-alt text-primary"></i>
            <span class="text-sm font-medium">{{ t('bugBounty.createFinding.useTemplate') }}</span>
          </div>
          <div class="flex items-center gap-2">
            <select v-model="selectedTemplateId" class="select select-sm select-bordered" @change="applyTemplate">
              <option value="">{{ t('bugBounty.createFinding.noTemplate') }}</option>
              <optgroup :label="t('bugBounty.templates.builtIn')">
                <option v-for="t in builtInTemplates" :key="t.id" :value="t.id">{{ t.name }}</option>
              </optgroup>
              <optgroup v-if="customTemplates.length > 0" :label="t('bugBounty.templates.custom')">
                <option v-for="t in customTemplates" :key="t.id" :value="t.id">{{ t.name }}</option>
              </optgroup>
            </select>
            <button v-if="selectedTemplateId" class="btn btn-xs btn-ghost" @click="clearTemplate">
              <i class="fas fa-times"></i>
            </button>
          </div>
        </div>
        <div v-if="activeTemplate" class="mt-2 text-xs text-base-content/60">
          {{ activeTemplate.description }}
        </div>
      </div>
      
      <div class="flex-1 overflow-auto space-y-4">
        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.program') }} *</span>
            </label>
            <select v-model="form.program_id" class="select select-bordered">
              <option value="">{{ t('bugBounty.form.selectProgram') }}</option>
              <option v-for="p in programs" :key="p.id" :value="p.id">{{ p.name }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.type') }} *</span>
            </label>
            <select v-model="form.finding_type" class="select select-bordered" @change="onTypeChange">
              <option value="xss">XSS</option>
              <option value="sqli">SQL Injection</option>
              <option value="ssrf">SSRF</option>
              <option value="idor">IDOR</option>
              <option value="rce">RCE</option>
              <option value="auth_bypass">Auth Bypass</option>
              <option value="info_disclosure">Info Disclosure</option>
              <option value="csrf">CSRF</option>
              <option value="xxe">XXE</option>
              <option value="lfi">LFI/RFI</option>
              <option value="open_redirect">Open Redirect</option>
              <option value="business_logic">Business Logic</option>
              <option value="other">Other</option>
            </select>
          </div>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.title') }} *</span>
            <span v-if="activeTemplate?.title_template" class="label-text-alt text-primary cursor-pointer" @click="generateTitle">
              <i class="fas fa-magic mr-1"></i>{{ t('bugBounty.createFinding.generateFromTemplate') }}
            </span>
          </label>
          <input 
            v-model="form.title" 
            type="text" 
            class="input input-bordered"
            :placeholder="t('bugBounty.form.findingTitlePlaceholder')"
          />
        </div>
        
        <div class="grid grid-cols-4 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.severity') }}</span>
            </label>
            <select v-model="form.severity" class="select select-bordered">
              <option value="critical">{{ t('bugBounty.severity.critical') }}</option>
              <option value="high">{{ t('bugBounty.severity.high') }}</option>
              <option value="medium">{{ t('bugBounty.severity.medium') }}</option>
              <option value="low">{{ t('bugBounty.severity.low') }}</option>
              <option value="info">{{ t('bugBounty.severity.info') }}</option>
            </select>
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">CVSS</span>
            </label>
            <input 
              v-model.number="form.cvss_score" 
              type="number" 
              step="0.1"
              min="0"
              max="10"
              class="input input-bordered"
              placeholder="0.0 - 10.0"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">CWE ID</span>
            </label>
            <input 
              v-model="form.cwe_id" 
              type="text" 
              class="input input-bordered"
              placeholder="79"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.affectedParameter') }}</span>
            </label>
            <input 
              v-model="form.affected_parameter" 
              type="text" 
              class="input input-bordered"
              placeholder="q, id, name..."
            />
          </div>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.affectedUrl') }}</span>
            </label>
            <input 
              v-model="form.affected_url" 
              type="text" 
              class="input input-bordered"
              placeholder="https://example.com/vulnerable/path"
            />
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('bugBounty.form.affectedEndpoint') }}</span>
            </label>
            <input 
              v-model="form.affected_endpoint" 
              type="text" 
              class="input input-bordered"
              placeholder="/api/users/{id}"
            />
          </div>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.description') }} *</span>
            <span v-if="activeTemplate?.description_template" class="label-text-alt text-primary cursor-pointer" @click="fillFromTemplate('description')">
              <i class="fas fa-magic mr-1"></i>{{ t('bugBounty.createFinding.fillFromTemplate') }}
            </span>
          </label>
          <textarea 
            v-model="form.description" 
            class="textarea textarea-bordered font-mono text-sm"
            rows="5"
            :placeholder="t('bugBounty.form.findingDescriptionPlaceholder')"
          ></textarea>
        </div>
        
        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.form.impact') }}</span>
            <span v-if="activeTemplate?.impact_template" class="label-text-alt text-primary cursor-pointer" @click="fillFromTemplate('impact')">
              <i class="fas fa-magic mr-1"></i>{{ t('bugBounty.createFinding.fillFromTemplate') }}
            </span>
          </label>
          <textarea 
            v-model="form.impact" 
            class="textarea textarea-bordered font-mono text-sm"
            rows="3"
            :placeholder="t('bugBounty.form.impactPlaceholder')"
          ></textarea>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.findingDetail.reproductionSteps') }}</span>
            <span v-if="activeTemplate?.steps_template" class="label-text-alt text-primary cursor-pointer" @click="fillFromTemplate('steps')">
              <i class="fas fa-magic mr-1"></i>{{ t('bugBounty.createFinding.fillFromTemplate') }}
            </span>
          </label>
          <textarea 
            v-model="form.reproduction_steps" 
            class="textarea textarea-bordered font-mono text-sm"
            rows="4"
            :placeholder="stepsPlaceholder"
          ></textarea>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">{{ t('bugBounty.findingDetail.remediation') }}</span>
            <span v-if="activeTemplate?.remediation_template" class="label-text-alt text-primary cursor-pointer" @click="fillFromTemplate('remediation')">
              <i class="fas fa-magic mr-1"></i>{{ t('bugBounty.createFinding.fillFromTemplate') }}
            </span>
          </label>
          <textarea 
            v-model="form.remediation" 
            class="textarea textarea-bordered font-mono text-sm"
            rows="3"
            :placeholder="t('bugBounty.form.remediationPlaceholder')"
          ></textarea>
        </div>
      </div>
      
      <div class="modal-action">
        <button @click="$emit('close')" class="btn">
          {{ t('common.cancel') }}
        </button>
        <button 
          @click="submit" 
          class="btn btn-primary" 
          :disabled="!isValid || submitting"
        >
          <span v-if="submitting" class="loading loading-spinner loading-sm mr-2"></span>
          {{ t('common.create') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  submitting: boolean
  programs: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'submit', data: any): void
}>()

const form = reactive({
  program_id: '',
  title: '',
  finding_type: 'xss',
  severity: 'medium',
  cvss_score: null as number | null,
  cwe_id: '',
  affected_url: '',
  affected_endpoint: '',
  affected_parameter: '',
  description: '',
  impact: '',
  reproduction_steps: '',
  remediation: '',
})

const selectedTemplateId = ref('')
const activeTemplate = ref<any>(null)
const customTemplates = ref<any[]>([])

const stepsPlaceholder = `1. Navigate to the vulnerable URL
2. Enter the payload
3. Observe the behavior
4. Verify the vulnerability`

// Built-in templates (same as ReportTemplatesPanel)
const builtInTemplates = [
  {
    id: 'xss-reflected',
    name: 'Reflected XSS',
    description: 'Template for reflected cross-site scripting vulnerabilities',
    category: 'web',
    finding_type: 'xss',
    cwe_id: '79',
    title_template: '[{{severity}}] Reflected XSS in {{endpoint}}',
    description_template: `## Summary
A reflected cross-site scripting (XSS) vulnerability was discovered in {{endpoint}}.

## Technical Details
The application reflects user input in the HTTP response without proper sanitization, allowing injection of arbitrary JavaScript code.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-79`,
    impact_template: `An attacker could exploit this vulnerability to:
- Steal session cookies and hijack user accounts
- Perform actions on behalf of authenticated users
- Redirect users to malicious websites
- Deface the web application`,
    steps_template: `1. Navigate to {{url}}
2. Insert the following payload in the {{parameter}} parameter:
   \`<scr` + `ipt>alert(document.domain)</scr` + `ipt>\`
3. Submit the request
4. Observe the JavaScript alert box executing`,
    remediation_template: `To fix this vulnerability:
1. Implement context-aware output encoding
2. Use Content-Security-Policy headers
3. Enable HttpOnly flag on session cookies
4. Consider using a web application firewall (WAF)`,
  },
  {
    id: 'sqli',
    name: 'SQL Injection',
    description: 'Template for SQL injection vulnerabilities',
    category: 'web',
    finding_type: 'sqli',
    cwe_id: '89',
    title_template: '[{{severity}}] SQL Injection in {{endpoint}}',
    description_template: `## Summary
A SQL injection vulnerability was discovered in {{endpoint}}.

## Technical Details
The application constructs SQL queries using unsanitized user input, allowing an attacker to manipulate database queries.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-89`,
    impact_template: `An attacker could exploit this vulnerability to:
- Extract sensitive data from the database
- Modify or delete database records
- Bypass authentication mechanisms
- Execute administrative operations`,
    steps_template: `1. Navigate to {{url}}
2. Insert a single quote (') in the {{parameter}} parameter
3. Observe the SQL error in the response
4. Use the following payload to confirm:
   \`' OR '1'='1\``,
    remediation_template: `To fix this vulnerability:
1. Use parameterized queries (prepared statements)
2. Implement input validation and whitelisting
3. Apply the principle of least privilege to database accounts`,
  },
  {
    id: 'idor',
    name: 'IDOR',
    description: 'Template for Insecure Direct Object Reference vulnerabilities',
    category: 'api',
    finding_type: 'idor',
    cwe_id: '639',
    title_template: '[{{severity}}] IDOR in {{endpoint}}',
    description_template: `## Summary
An Insecure Direct Object Reference (IDOR) vulnerability was discovered in {{endpoint}}.

## Technical Details
The application exposes internal object references without proper authorization checks.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-639`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access other users' private data
- Modify or delete other users' resources
- Escalate privileges horizontally`,
    steps_template: `1. Login as User A and navigate to {{url}}
2. Note the resource ID in the {{parameter}} parameter
3. Change the ID to another user's resource ID
4. Observe unauthorized access to User B's data`,
    remediation_template: `To fix this vulnerability:
1. Implement proper authorization checks for all object access
2. Use indirect reference maps instead of direct IDs
3. Validate that the current user owns the requested resource`,
  },
  {
    id: 'ssrf',
    name: 'SSRF',
    description: 'Template for Server-Side Request Forgery vulnerabilities',
    category: 'web',
    finding_type: 'ssrf',
    cwe_id: '918',
    title_template: '[{{severity}}] SSRF in {{endpoint}}',
    description_template: `## Summary
A Server-Side Request Forgery (SSRF) vulnerability was discovered in {{endpoint}}.

## Technical Details
The application makes HTTP requests to user-controlled URLs without proper validation.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-918`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access internal services and APIs
- Scan internal network infrastructure
- Access cloud metadata endpoints`,
    steps_template: `1. Navigate to {{url}}
2. Set the {{parameter}} parameter to: http://127.0.0.1:80
3. Observe the server making a request to localhost`,
    remediation_template: `To fix this vulnerability:
1. Implement URL validation and whitelist allowed hosts
2. Block requests to private IP ranges
3. Disable unnecessary URL schemes`,
  },
  {
    id: 'auth-bypass',
    name: 'Auth Bypass',
    description: 'Template for authentication bypass vulnerabilities',
    category: 'web',
    finding_type: 'auth_bypass',
    cwe_id: '287',
    title_template: '[{{severity}}] Authentication Bypass in {{endpoint}}',
    description_template: `## Summary
An authentication bypass vulnerability was discovered in {{endpoint}}.

## Technical Details
The application fails to properly verify user authentication.

## Affected Component
- URL: {{url}}
- CWE: CWE-287`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access protected resources without authentication
- Impersonate other users
- Gain administrative access`,
    steps_template: `1. Attempt to access {{url}} without authentication
2. Observe that the resource is accessible
3. Verify access to protected functionality`,
    remediation_template: `To fix this vulnerability:
1. Implement proper authentication checks on all protected endpoints
2. Use a centralized authentication middleware
3. Validate session tokens on every request`,
  },
  {
    id: 'info-disclosure',
    name: 'Info Disclosure',
    description: 'Template for information disclosure vulnerabilities',
    category: 'general',
    finding_type: 'info_disclosure',
    cwe_id: '200',
    title_template: '[{{severity}}] Information Disclosure in {{endpoint}}',
    description_template: `## Summary
An information disclosure vulnerability was discovered in {{endpoint}}.

## Technical Details
The application exposes sensitive information that could aid an attacker.

## Affected Component
- URL: {{url}}
- CWE: CWE-200`,
    impact_template: `An attacker could use this information to:
- Gather intelligence for further attacks
- Identify software versions with known vulnerabilities
- Discover internal system architecture`,
    steps_template: `1. Navigate to {{url}}
2. Observe the sensitive information in the response
3. Document the exposed data`,
    remediation_template: `To fix this vulnerability:
1. Remove sensitive information from responses
2. Implement proper error handling
3. Configure proper access controls`,
  },
]

const isValid = computed(() => 
  form.program_id && form.title && form.finding_type && form.description
)

// Load custom templates from localStorage
const loadCustomTemplates = () => {
  const saved = localStorage.getItem('bounty-report-templates')
  if (saved) {
    try {
      customTemplates.value = JSON.parse(saved)
    } catch (e) {
      console.error('Failed to load templates:', e)
    }
  }
}

// Check for active template from session storage (from Templates tab)
const checkSessionTemplate = () => {
  const sessionTemplate = sessionStorage.getItem('bounty-active-template')
  if (sessionTemplate) {
    try {
      const template = JSON.parse(sessionTemplate)
      activeTemplate.value = template
      selectedTemplateId.value = template.id
      applyTemplateData(template)
      sessionStorage.removeItem('bounty-active-template')
    } catch (e) {
      console.error('Failed to load session template:', e)
    }
  }
}

const applyTemplate = () => {
  if (!selectedTemplateId.value) {
    activeTemplate.value = null
    return
  }
  
  const allTemplates = [...builtInTemplates, ...customTemplates.value]
  const template = allTemplates.find(t => t.id === selectedTemplateId.value)
  
  if (template) {
    activeTemplate.value = template
    applyTemplateData(template)
  }
}

const applyTemplateData = (template: any) => {
  // Auto-fill type and CWE if template has them
  if (template.finding_type) {
    form.finding_type = template.finding_type
  }
  if (template.cwe_id) {
    form.cwe_id = template.cwe_id
  }
}

const clearTemplate = () => {
  selectedTemplateId.value = ''
  activeTemplate.value = null
}

const onTypeChange = () => {
  // Auto-suggest template based on type
  if (!selectedTemplateId.value) {
    const typeTemplateMap: Record<string, string> = {
      xss: 'xss-reflected',
      sqli: 'sqli',
      idor: 'idor',
      ssrf: 'ssrf',
      auth_bypass: 'auth-bypass',
      info_disclosure: 'info-disclosure',
    }
    const suggestedId = typeTemplateMap[form.finding_type]
    if (suggestedId) {
      selectedTemplateId.value = suggestedId
      applyTemplate()
    }
  }
}

const replaceVariables = (text: string): string => {
  const program = props.programs.find(p => p.id === form.program_id)
  
  return text
    .replace(/\{\{vuln_type\}\}/g, form.finding_type || '')
    .replace(/\{\{severity\}\}/g, form.severity || '')
    .replace(/\{\{endpoint\}\}/g, form.affected_endpoint || form.affected_url || '')
    .replace(/\{\{url\}\}/g, form.affected_url || '')
    .replace(/\{\{parameter\}\}/g, form.affected_parameter || '')
    .replace(/\{\{program\}\}/g, program?.name || '')
    .replace(/\{\{date\}\}/g, new Date().toLocaleDateString())
    .replace(/\{\{cwe_id\}\}/g, form.cwe_id || '')
    .replace(/\{\{cvss\}\}/g, form.cvss_score?.toString() || '')
}

const generateTitle = () => {
  if (activeTemplate.value?.title_template) {
    form.title = replaceVariables(activeTemplate.value.title_template)
  }
}

const fillFromTemplate = (field: 'description' | 'impact' | 'steps' | 'remediation') => {
  if (!activeTemplate.value) return
  
  const templateMap: Record<string, string> = {
    description: 'description_template',
    impact: 'impact_template',
    steps: 'steps_template',
    remediation: 'remediation_template',
  }
  
  const formMap: Record<string, keyof typeof form> = {
    description: 'description',
    impact: 'impact',
    steps: 'reproduction_steps',
    remediation: 'remediation',
  }
  
  const templateContent = activeTemplate.value[templateMap[field]]
  if (templateContent) {
    (form as any)[formMap[field]] = replaceVariables(templateContent)
  }
}

const submit = () => {
  if (!isValid.value) return
  
  // Parse reproduction steps to array
  const stepsArray = form.reproduction_steps
    .split('\n')
    .map(s => s.trim())
    .filter(s => s)
    .map(s => s.replace(/^\d+\.\s*/, ''))
  
  emit('submit', { 
    ...form,
    reproduction_steps: stepsArray.length > 0 ? stepsArray : null,
  })
}

const resetForm = () => {
  form.program_id = ''
  form.title = ''
  form.finding_type = 'xss'
  form.severity = 'medium'
  form.cvss_score = null
  form.cwe_id = ''
  form.affected_url = ''
  form.affected_endpoint = ''
  form.affected_parameter = ''
  form.description = ''
  form.impact = ''
  form.reproduction_steps = ''
  form.remediation = ''
  selectedTemplateId.value = ''
  activeTemplate.value = null
}

watch(() => props.visible, (val) => {
  if (val) {
    loadCustomTemplates()
    checkSessionTemplate()
  } else {
    resetForm()
  }
})

onMounted(() => {
  loadCustomTemplates()
})
</script>
