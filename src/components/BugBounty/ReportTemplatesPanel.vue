<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex justify-between items-center">
      <div>
        <h3 class="text-lg font-bold">{{ t('bugBounty.templates.title') }}</h3>
        <p class="text-sm text-base-content/60">{{ t('bugBounty.templates.description') }}</p>
      </div>
      <button class="btn btn-primary btn-sm" @click="createTemplate">
        <i class="fas fa-plus mr-2"></i>
        {{ t('bugBounty.templates.create') }}
      </button>
    </div>

    <!-- Built-in Templates -->
    <div class="card bg-base-100 shadow-md border border-base-200">
      <div class="card-body">
        <h4 class="font-medium flex items-center gap-2 mb-4">
          <i class="fas fa-star text-warning"></i>
          {{ t('bugBounty.templates.builtIn') }}
        </h4>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div 
            v-for="template in builtInTemplates" 
            :key="template.id"
            class="card bg-base-200 hover:bg-base-300 transition-colors cursor-pointer"
            @click="previewTemplate(template)"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div>
                  <h5 class="font-medium">{{ template.name }}</h5>
                  <p class="text-xs text-base-content/60 mt-1">{{ template.description }}</p>
                </div>
                <span class="badge badge-ghost badge-sm">{{ template.category }}</span>
              </div>
              <div class="flex gap-2 mt-3">
                <button class="btn btn-xs btn-primary" @click.stop="useTemplate(template)">
                  <i class="fas fa-copy mr-1"></i>
                  {{ t('bugBounty.templates.use') }}
                </button>
                <button class="btn btn-xs btn-ghost" @click.stop="previewTemplate(template)">
                  <i class="fas fa-eye mr-1"></i>
                  {{ t('common.view') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Custom Templates -->
    <div class="card bg-base-100 shadow-md border border-base-200">
      <div class="card-body">
        <h4 class="font-medium flex items-center gap-2 mb-4">
          <i class="fas fa-user text-primary"></i>
          {{ t('bugBounty.templates.custom') }}
        </h4>
        
        <div v-if="customTemplates.length === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-file-alt text-4xl mb-3 opacity-30"></i>
          <p>{{ t('bugBounty.templates.noCustom') }}</p>
          <button class="btn btn-sm btn-primary mt-4" @click="createTemplate">
            {{ t('bugBounty.templates.createFirst') }}
          </button>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div 
            v-for="template in customTemplates" 
            :key="template.id"
            class="card bg-base-200 hover:bg-base-300 transition-colors"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div>
                  <h5 class="font-medium">{{ template.name }}</h5>
                  <p class="text-xs text-base-content/60 mt-1">{{ template.description || t('bugBounty.templates.noDescription') }}</p>
                </div>
                <span class="badge badge-ghost badge-sm">{{ template.category }}</span>
              </div>
              <div class="flex gap-2 mt-3">
                <button class="btn btn-xs btn-primary" @click="useTemplate(template)">
                  <i class="fas fa-copy mr-1"></i>
                  {{ t('bugBounty.templates.use') }}
                </button>
                <button class="btn btn-xs btn-ghost" @click="editTemplate(template)">
                  <i class="fas fa-edit mr-1"></i>
                  {{ t('common.edit') }}
                </button>
                <button class="btn btn-xs btn-ghost text-error" @click="deleteTemplate(template)">
                  <i class="fas fa-trash"></i>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Template Editor Modal -->
    <div v-if="showEditorModal" class="modal modal-open">
      <div class="modal-box max-w-4xl h-[80vh] flex flex-col">
        <h3 class="font-bold text-lg mb-4">
          {{ editingTemplate ? t('bugBounty.templates.editTemplate') : t('bugBounty.templates.createTemplate') }}
        </h3>
        
        <div class="flex-1 overflow-auto space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('bugBounty.templates.templateName') }} *</span></label>
              <input v-model="templateForm.name" type="text" class="input input-bordered" :placeholder="t('bugBounty.templates.namePlaceholder')" />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text">{{ t('bugBounty.templates.category') }}</span></label>
              <select v-model="templateForm.category" class="select select-bordered">
                <option value="general">{{ t('bugBounty.templates.categories.general') }}</option>
                <option value="web">{{ t('bugBounty.templates.categories.web') }}</option>
                <option value="api">{{ t('bugBounty.templates.categories.api') }}</option>
                <option value="mobile">{{ t('bugBounty.templates.categories.mobile') }}</option>
                <option value="infrastructure">{{ t('bugBounty.templates.categories.infrastructure') }}</option>
              </select>
            </div>
          </div>

          <div class="form-control">
            <label class="label"><span class="label-text">{{ t('bugBounty.templates.templateDescription') }}</span></label>
            <input v-model="templateForm.description" type="text" class="input input-bordered" :placeholder="t('bugBounty.templates.descriptionPlaceholder')" />
          </div>

          <div class="divider">{{ t('bugBounty.templates.sections') }}</div>

          <!-- Title Template -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">{{ t('bugBounty.templates.titleTemplate') }}</span>
              <span class="label-text-alt text-base-content/60">{{ t('bugBounty.templates.variablesHint') }}</span>
            </label>
            <input v-model="templateForm.title_template" type="text" class="input input-bordered font-mono text-sm" placeholder="[{{severity}}] {{vuln_type}} in {{endpoint}}" />
          </div>

          <!-- Description Template -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.templates.descriptionTemplate') }}</span></label>
            <textarea v-model="templateForm.description_template" class="textarea textarea-bordered font-mono text-sm h-32" :placeholder="descriptionPlaceholder"></textarea>
          </div>

          <!-- Impact Template -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.templates.impactTemplate') }}</span></label>
            <textarea v-model="templateForm.impact_template" class="textarea textarea-bordered font-mono text-sm h-24" :placeholder="impactPlaceholder"></textarea>
          </div>

          <!-- Steps Template -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.templates.stepsTemplate') }}</span></label>
            <textarea v-model="templateForm.steps_template" class="textarea textarea-bordered font-mono text-sm h-32" :placeholder="stepsPlaceholder"></textarea>
          </div>

          <!-- Remediation Template -->
          <div class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.templates.remediationTemplate') }}</span></label>
            <textarea v-model="templateForm.remediation_template" class="textarea textarea-bordered font-mono text-sm h-24" :placeholder="remediationPlaceholder"></textarea>
          </div>

          <!-- Variables Reference -->
          <div class="collapse collapse-arrow bg-base-200">
            <input type="checkbox" />
            <div class="collapse-title text-sm font-medium">
              <i class="fas fa-code mr-2"></i>
              {{ t('bugBounty.templates.availableVariables') }}
            </div>
            <div class="collapse-content">
              <div class="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm">
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}vuln_type{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}severity{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}endpoint{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}url{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}parameter{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}program{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}date{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}cwe_id{{ '}' }}{{ '}' }}</code>
                <code class="bg-base-300 px-2 py-1 rounded">{{ '{' }}{{ '{' }}cvss{{ '}' }}{{ '}' }}</code>
              </div>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-ghost" @click="closeEditor">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary" @click="saveTemplate" :disabled="!templateForm.name || saving">
            <span v-if="saving" class="loading loading-spinner loading-sm mr-2"></span>
            {{ t('common.save') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Template Preview Modal -->
    <div v-if="showPreviewModal" class="modal modal-open">
      <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg mb-4">{{ previewingTemplate?.name }}</h3>
        
        <div class="space-y-4">
          <div v-if="previewingTemplate?.title_template" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.table.title') }}</span></label>
            <div class="bg-base-200 rounded-lg p-3 font-mono text-sm">{{ previewingTemplate.title_template }}</div>
          </div>
          
          <div v-if="previewingTemplate?.description_template" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.form.description') }}</span></label>
            <div class="bg-base-200 rounded-lg p-3 font-mono text-sm whitespace-pre-wrap">{{ previewingTemplate.description_template }}</div>
          </div>
          
          <div v-if="previewingTemplate?.impact_template" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.impact') }}</span></label>
            <div class="bg-base-200 rounded-lg p-3 font-mono text-sm whitespace-pre-wrap">{{ previewingTemplate.impact_template }}</div>
          </div>
          
          <div v-if="previewingTemplate?.steps_template" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.reproductionSteps') }}</span></label>
            <div class="bg-base-200 rounded-lg p-3 font-mono text-sm whitespace-pre-wrap">{{ previewingTemplate.steps_template }}</div>
          </div>
          
          <div v-if="previewingTemplate?.remediation_template" class="form-control">
            <label class="label"><span class="label-text font-medium">{{ t('bugBounty.findingDetail.remediation') }}</span></label>
            <div class="bg-base-200 rounded-lg p-3 font-mono text-sm whitespace-pre-wrap">{{ previewingTemplate.remediation_template }}</div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-primary" @click="useTemplate(previewingTemplate!)">
            <i class="fas fa-copy mr-2"></i>
            {{ t('bugBounty.templates.use') }}
          </button>
          <button class="btn" @click="showPreviewModal = false">{{ t('common.close') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useToast } from '../../composables/useToast'

const { t } = useI18n()
const toast = useToast()

const emit = defineEmits<{
  (e: 'use-template', template: any): void
}>()

// State
const showEditorModal = ref(false)
const showPreviewModal = ref(false)
const saving = ref(false)
const editingTemplate = ref<any>(null)
const previewingTemplate = ref<any>(null)
const customTemplates = ref<any[]>([])

const templateForm = reactive({
  name: '',
  description: '',
  category: 'general',
  title_template: '',
  description_template: '',
  impact_template: '',
  steps_template: '',
  remediation_template: '',
})

// Placeholders
const descriptionPlaceholder = `## Summary
A {{severity}} {{vuln_type}} vulnerability was discovered in {{endpoint}}.

## Technical Details
The vulnerability exists because...

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}`

const impactPlaceholder = `An attacker could exploit this vulnerability to:
- Unauthorized access to sensitive data
- Execute arbitrary code
- Bypass authentication controls`

const stepsPlaceholder = `1. Navigate to {{url}}
2. Enter payload in the {{parameter}} field
3. Submit the request
4. Observe the vulnerable behavior`

const remediationPlaceholder = `To fix this vulnerability:
1. Implement proper input validation
2. Use parameterized queries
3. Apply the principle of least privilege`

// Built-in templates
const builtInTemplates = [
  {
    id: 'xss-reflected',
    name: 'Reflected XSS',
    description: 'Template for reflected cross-site scripting vulnerabilities',
    category: 'web',
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
- Execute administrative operations
- In some cases, execute OS commands`,
    steps_template: `1. Navigate to {{url}}
2. Insert a single quote (') in the {{parameter}} parameter
3. Observe the SQL error in the response
4. Use the following payload to confirm:
   \`' OR '1'='1\`
5. Use sqlmap for automated exploitation`,
    remediation_template: `To fix this vulnerability:
1. Use parameterized queries (prepared statements)
2. Implement input validation and whitelisting
3. Apply the principle of least privilege to database accounts
4. Use stored procedures where appropriate
5. Enable SQL query logging for monitoring`,
  },
  {
    id: 'idor',
    name: 'IDOR',
    description: 'Template for Insecure Direct Object Reference vulnerabilities',
    category: 'api',
    title_template: '[{{severity}}] IDOR in {{endpoint}}',
    description_template: `## Summary
An Insecure Direct Object Reference (IDOR) vulnerability was discovered in {{endpoint}}.

## Technical Details
The application exposes internal object references without proper authorization checks, allowing unauthorized access to resources.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-639`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access other users' private data
- Modify or delete other users' resources
- Escalate privileges horizontally
- Enumerate sensitive information`,
    steps_template: `1. Login as User A and navigate to {{url}}
2. Note the resource ID in the {{parameter}} parameter
3. Change the ID to another user's resource ID
4. Observe that you can access User B's data without authorization`,
    remediation_template: `To fix this vulnerability:
1. Implement proper authorization checks for all object access
2. Use indirect reference maps instead of direct IDs
3. Validate that the current user owns the requested resource
4. Implement access control lists (ACLs)
5. Log and monitor unauthorized access attempts`,
  },
  {
    id: 'ssrf',
    name: 'SSRF',
    description: 'Template for Server-Side Request Forgery vulnerabilities',
    category: 'web',
    title_template: '[{{severity}}] SSRF in {{endpoint}}',
    description_template: `## Summary
A Server-Side Request Forgery (SSRF) vulnerability was discovered in {{endpoint}}.

## Technical Details
The application makes HTTP requests to user-controlled URLs without proper validation, allowing an attacker to make requests to internal services.

## Affected Component
- URL: {{url}}
- Parameter: {{parameter}}
- CWE: CWE-918`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access internal services and APIs
- Scan internal network infrastructure
- Read sensitive files via file:// protocol
- Access cloud metadata endpoints (AWS, GCP, Azure)
- Bypass firewalls and access controls`,
    steps_template: `1. Navigate to {{url}}
2. Set the {{parameter}} parameter to: http://127.0.0.1:80
3. Observe the server making a request to localhost
4. Try accessing cloud metadata: http://169.254.169.254/latest/meta-data/`,
    remediation_template: `To fix this vulnerability:
1. Implement URL validation and whitelist allowed hosts
2. Block requests to private IP ranges and localhost
3. Disable unnecessary URL schemes (file://, gopher://)
4. Use a proxy for outbound requests
5. Implement network segmentation`,
  },
  {
    id: 'auth-bypass',
    name: 'Authentication Bypass',
    description: 'Template for authentication bypass vulnerabilities',
    category: 'web',
    title_template: '[{{severity}}] Authentication Bypass in {{endpoint}}',
    description_template: `## Summary
An authentication bypass vulnerability was discovered in {{endpoint}}.

## Technical Details
The application fails to properly verify user authentication, allowing unauthorized access to protected resources.

## Affected Component
- URL: {{url}}
- CWE: CWE-287`,
    impact_template: `An attacker could exploit this vulnerability to:
- Access protected resources without authentication
- Impersonate other users
- Gain administrative access
- Access sensitive user data`,
    steps_template: `1. Attempt to access {{url}} without authentication
2. Observe that the resource is accessible
3. Alternatively, manipulate the authentication token/cookie
4. Verify access to protected functionality`,
    remediation_template: `To fix this vulnerability:
1. Implement proper authentication checks on all protected endpoints
2. Use a centralized authentication middleware
3. Validate session tokens on every request
4. Implement proper session management
5. Use secure authentication libraries`,
  },
  {
    id: 'info-disclosure',
    name: 'Information Disclosure',
    description: 'Template for information disclosure vulnerabilities',
    category: 'general',
    title_template: '[{{severity}}] Information Disclosure in {{endpoint}}',
    description_template: `## Summary
An information disclosure vulnerability was discovered in {{endpoint}}.

## Technical Details
The application exposes sensitive information that could aid an attacker in further attacks.

## Affected Component
- URL: {{url}}
- CWE: CWE-200`,
    impact_template: `An attacker could use this information to:
- Gather intelligence for further attacks
- Identify software versions with known vulnerabilities
- Discover internal system architecture
- Access sensitive configuration details`,
    steps_template: `1. Navigate to {{url}}
2. Observe the sensitive information in the response
3. Document the exposed data`,
    remediation_template: `To fix this vulnerability:
1. Remove sensitive information from responses
2. Implement proper error handling (no stack traces)
3. Configure proper access controls
4. Disable directory listing
5. Remove unnecessary files and endpoints`,
  },
]

// Methods
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

const saveCustomTemplates = () => {
  localStorage.setItem('bounty-report-templates', JSON.stringify(customTemplates.value))
}

const createTemplate = () => {
  editingTemplate.value = null
  resetForm()
  showEditorModal.value = true
}

const editTemplate = (template: any) => {
  editingTemplate.value = template
  Object.assign(templateForm, template)
  showEditorModal.value = true
}

const previewTemplate = (template: any) => {
  previewingTemplate.value = template
  showPreviewModal.value = true
}

const useTemplate = (template: any) => {
  emit('use-template', template)
  showPreviewModal.value = false
  toast.success(t('bugBounty.templates.templateCopied'))
}

const saveTemplate = () => {
  if (!templateForm.name) return
  
  saving.value = true
  
  try {
    const template = {
      id: editingTemplate.value?.id || `custom-${Date.now()}`,
      ...templateForm,
    }
    
    if (editingTemplate.value) {
      const index = customTemplates.value.findIndex(t => t.id === editingTemplate.value.id)
      if (index !== -1) {
        customTemplates.value[index] = template
      }
    } else {
      customTemplates.value.push(template)
    }
    
    saveCustomTemplates()
    toast.success(t('bugBounty.templates.saved'))
    closeEditor()
  } catch (error) {
    console.error('Failed to save template:', error)
    toast.error(t('bugBounty.errors.createFailed'))
  } finally {
    saving.value = false
  }
}

const deleteTemplate = (template: any) => {
  
  customTemplates.value = customTemplates.value.filter(t => t.id !== template.id)
  saveCustomTemplates()
  toast.success(t('bugBounty.templates.deleted'))
}

const closeEditor = () => {
  showEditorModal.value = false
  editingTemplate.value = null
  resetForm()
}

const resetForm = () => {
  templateForm.name = ''
  templateForm.description = ''
  templateForm.category = 'general'
  templateForm.title_template = ''
  templateForm.description_template = ''
  templateForm.impact_template = ''
  templateForm.steps_template = ''
  templateForm.remediation_template = ''
}

onMounted(() => {
  loadCustomTemplates()
})
</script>
