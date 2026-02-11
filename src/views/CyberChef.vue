<template>
  <div class="h-full flex flex-col bg-base-100 overflow-hidden text-sm select-none">
    <!-- Main Content: Three-column layout -->
    <div class="flex-1 flex overflow-hidden">
      <!-- Left Column: Operations Library (CyberChef style) -->
      <div class="w-64 border-r border-base-300 flex flex-col bg-base-200/50">
        <div class="p-2 border-b border-base-300 bg-base-300/30">
          <div class="font-bold text-xs uppercase opacity-70 mb-2 px-1 flex justify-between">
            <span>{{ t('cyberchef.operations', '操作') }}</span>
          </div>
          <div class="relative">
            <input 
              v-model="searchQuery" 
              type="text" 
              :placeholder="t('cyberchef.searchPlaceholder', '搜索操作...')" 
              class="input input-xs input-bordered w-full pl-7 py-3"
            />
            <i class="fas fa-search absolute left-2.5 top-2 text-base-content/40 text-[10px]"></i>
          </div>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar">
          <div v-for="(category, name) in categorizedOperations" :key="name" class="collapse collapse-arrow rounded-none border-b border-base-300/50">
            <input type="checkbox" checked /> 
            <div class="collapse-title text-[11px] font-bold uppercase py-2 min-h-0 flex items-center bg-base-300/20">
              {{ name }}
            </div>
            <div class="collapse-content p-0">
              <draggable 
                class="list-group p-0 min-h-[10px]"
                :list="category"
                :group="{ name: 'operations', pull: 'clone', put: false }"
                :clone="cloneOperation"
                :sort="false"
                item-key="id"
                @start="drag = true"
                @end="drag = false"
              >
                <template #item="{ element }">
                  <div 
                    class="op-item px-3 py-1.5 cursor-pointer hover:bg-primary/10 hover:text-primary transition-colors text-xs border-b border-base-300/30 last:border-0 select-none flex items-center justify-between"
                    @dblclick="addToRecipe(element)"
                  >
                    <span>{{ element.name }}</span>
                    <i class="fas fa-grip-vertical opacity-0 hover:opacity-50 text-[10px] cursor-grab active:cursor-grabbing"></i>
                  </div>
                </template>
              </draggable>
            </div>
          </div>
        </div>
      </div>

      <!-- Center Column: Recipe (The mixing bowl) -->
      <div class="w-80 border-r border-base-300 flex flex-col bg-base-100">
        <div class="p-2 border-b border-base-300 bg-base-300/30 flex justify-between items-center shrink-0">
          <div class="flex items-center gap-2">
            <span class="font-bold text-xs uppercase opacity-70">{{ t('cyberchef.recipe', '配方') }}</span>
            <span v-if="recipe.length > 0" class="badge badge-primary badge-xs scale-90">{{ recipe.length }}</span>
          </div>
          <div class="flex gap-1">
            <button class="btn btn-ghost btn-xs px-1" :title="t('cyberchef.clearRecipe', '清空')" @click="clearRecipe">
              <i class="fas fa-trash-alt text-[10px]"></i>
            </button>
          </div>
        </div>

        <draggable
          class="flex-1 overflow-y-auto p-2 space-y-2 bg-base-300/10 custom-scrollbar min-h-[50%]"
          v-model="recipe"
          group="operations"
          item-key="instanceId"
          handle=".handle"
          ghost-class="ghost-item"
          :animation="200"
        >
          <template #item="{ element, index }">
            <div class="recipe-card group bg-base-100 border border-base-300 shadow-sm rounded-sm">
              <div class="bg-primary/5 px-2 py-1.5 border-b border-base-300 flex justify-between items-center handle cursor-grab active:cursor-grabbing">
                <span class="text-[11px] font-bold text-primary flex items-center gap-2">
                  <i class="fas fa-grip-vertical opacity-30 text-[9px]"></i>
                  {{ element.name }}
                </span>
                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                   <button class="btn btn-ghost btn-xs h-5 min-h-0 w-5 p-0" @click="toggleOperation(element)">
                    <i :class="element.disabled ? 'fas fa-eye-slash' : 'fas fa-eye'" class="text-[10px]"></i>
                  </button>
                  <button class="btn btn-ghost btn-xs h-5 min-h-0 w-5 p-0 text-error" @click.stop="removeFromRecipe(index)">
                    <i class="fas fa-times text-[10px]"></i>
                  </button>
                </div>
              </div>
              
              <div v-if="!element.disabled && element.options" class="p-2 space-y-2 bg-base-100">
                <div v-for="opt in element.options" :key="opt.name" class="flex items-center gap-2">
                  <span class="text-[10px] uppercase opacity-60 w-16 truncate shrink-0">{{ opt.label }}</span>
                  <input 
                    v-if="opt.type === 'text'" 
                    v-model="element.values[opt.name]" 
                    type="text" 
                    class="input input-xs input-bordered flex-1 h-6 text-[11px]"
                    @input="autoBake"
                  />
                  <select 
                    v-else-if="opt.type === 'select'" 
                    v-model="element.values[opt.name]" 
                    class="select select-xs select-bordered flex-1 h-6 min-h-0 text-[11px] py-0"
                    @change="autoBake"
                  >
                    <option v-for="o in opt.choices" :key="o" :value="o">{{ o }}</option>
                  </select>
                </div>
              </div>
              <div v-else-if="element.disabled" class="p-2 py-1 text-center bg-base-200/50">
                <span class="text-[10px] opacity-40 italic">{{ t('cyberchef.operationDisabled', '已禁用') }}</span>
              </div>
            </div>
          </template>
          
          <template #footer>
            <div v-if="recipe.length === 0" class="flex flex-col items-center justify-center py-12 text-base-content/20 italic text-center pointer-events-none select-none">
              <i class="fas fa-mortar-pestle text-3xl mb-3 opacity-30"></i>
              <p class="text-xs">{{ t('cyberchef.emptyRecipe', '从左侧拖动操作到此处') }}</p>
              <p class="text-[10px] mt-1 opacity-70">({{ t('cyberchef.doubleClickHint', '或双击添加') }})</p>
            </div>
          </template>
        </draggable>

        <div class="p-2 border-t border-base-300 bg-base-200">
          <button class="btn btn-primary btn-sm w-full gap-2 shadow-lg" :loading="baking" @click="bake">
            <i class="fas fa-play text-[10px]"></i> {{ t('cyberchef.bake', '执行 BAKE!') }}
          </button>
          <div class="mt-2 flex items-center gap-2 px-1">
            <input type="checkbox" v-model="isAutoBake" class="checkbox checkbox-xs checkbox-primary" />
            <span class="text-[10px] uppercase font-bold opacity-60">{{ t('cyberchef.autoBake', '自动执行') }}</span>
          </div>
        </div>
      </div>

      <!-- Right Column: Input/Output -->
      <div class="flex-1 flex flex-col min-w-0">
        <!-- Input Area -->
        <div class="flex-1 flex flex-col min-h-0 border-b border-base-300 bg-base-200/20">
          <div class="p-2 border-b border-base-300 bg-base-300/30 flex justify-between items-center shrink-0">
            <span class="font-bold text-xs uppercase opacity-70">{{ t('cyberchef.input', '输入') }}</span>
            <div class="flex gap-2 items-center">
              <span class="text-[10px] opacity-40 font-mono">{{ t('cyberchef.chars', '长度') }}: {{ input.length }}</span>
              <button class="btn btn-ghost btn-xs h-6 min-h-0 px-2 text-[10px]" @click="input = ''">
                {{ t('cyberchef.clear', '清空') }}
              </button>
            </div>
          </div>
          <div class="flex-1 relative">
            <textarea 
              v-model="input" 
              class="editor-area w-full h-full resize-none font-mono focus:outline-none p-3 bg-transparent"
              :placeholder="t('cyberchef.inputPlaceholder', '在此输入待处理数据...')"
              spellcheck="false"
            ></textarea>
          </div>
        </div>

        <!-- Output Area -->
        <div class="flex-1 flex flex-col min-h-0 bg-base-200/10">
          <div class="p-2 border-b border-base-300 bg-base-300/30 flex justify-between items-center shrink-0">
            <span class="font-bold text-xs uppercase opacity-70">{{ t('cyberchef.output', '输出') }}</span>
            <div class="flex gap-2 items-center">
              <span class="text-[10px] opacity-40 font-mono">{{ t('cyberchef.chars', '长度') }}: {{ output.length }}</span>
              <button class="btn btn-ghost btn-xs h-6 min-h-0 px-2 text-[10px]" @click="copyOutput">
                <i class="fas fa-copy mr-1"></i> {{ t('cyberchef.copy', '复制') }}
              </button>
            </div>
          </div>
          <div class="flex-1 relative">
             <div v-if="error" class="absolute inset-0 z-10 p-4 pointer-events-none">
              <div class="alert alert-error rounded-sm shadow-md py-2 px-3 text-xs w-fit pointer-events-auto">
                <i class="fas fa-exclamation-triangle"></i>
                <span>{{ error }}</span>
              </div>
            </div>
            <textarea 
              readonly 
              v-model="output" 
              class="editor-area w-full h-full resize-none font-mono focus:outline-none p-3 bg-transparent text-primary/90"
              :placeholder="t('cyberchef.outputPlaceholder', '结果将在此实时呈现...')"
              spellcheck="false"
            ></textarea>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import { useI18n } from 'vue-i18n'
import draggable from 'vuedraggable'

const { t } = useI18n()

// --- UI State ---
const input = ref('')
const output = ref('')
const error = ref('')
const searchQuery = ref('')
const recipe = ref<any[]>([])
const isAutoBake = ref(true)
const baking = ref(false)
const drag = ref(false)

// --- Operations Definitions ---
interface OperationOption {
  name: string
  label: string
  type: 'text' | 'select'
  choices?: string[]
}

interface Operation {
  id: string
  name: string
  category: string
  description: string
  options?: OperationOption[]
  process: (input: string, values: Record<string, any>) => string | Promise<string>
  disabled?: boolean
  instanceId?: string
  values?: any
}

const operations: Operation[] = [
  // --- Data Formats ---
  {
    id: 'base64-encode',
    name: 'To Base64',
    category: 'Data Formats',
    description: 'Encodes data in Base64 format.',
    process: (input) => btoa(input)
  },
  {
    id: 'base64-decode',
    name: 'From Base64',
    category: 'Data Formats',
    description: 'Decodes data from Base64 format.',
    process: (input) => {
      try {
        return atob(input.replace(/\s/g, ''))
      } catch (e) {
        throw new Error('Invalid Base64 input')
      }
    }
  },
  {
    id: 'url-encode',
    name: 'URL Encode',
    category: 'Data Formats',
    description: 'Encodes characters for use in URLs.',
    process: (input) => encodeURIComponent(input)
  },
  {
    id: 'url-decode',
    name: 'URL Decode',
    category: 'Data Formats',
    description: 'Decodes URL-encoded data.',
    process: (input) => decodeURIComponent(input)
  },
  {
    id: 'hex-encode',
    name: 'To Hex',
    category: 'Data Formats',
    description: 'Converts data to hexadecimal strings.',
    options: [
      { name: 'delimiter', label: 'Delimiter', type: 'select', choices: ['None', 'Space', 'Comma', '0x', '\\x'] }
    ],
    process: (input, values) => {
      const hex = Array.from(input).map(c => c.charCodeAt(0).toString(16).padStart(2, '0'))
      const delimiter = values.delimiter || 'None'
      if (delimiter === 'Space') return hex.join(' ')
      if (delimiter === 'Comma') return hex.join(',')
      if (delimiter === '0x') return hex.map(h => '0x' + h).join(' ')
      if (delimiter === '\\x') return hex.map(h => '\\x' + h).join('')
      return hex.join('')
    }
  },
  {
    id: 'hex-decode',
    name: 'From Hex',
    category: 'Data Formats',
    description: 'Converts hexadecimal strings back to data.',
    process: (input) => {
      const clean = input.replace(/[^0-9a-fA-F]/g, '')
      if (clean.length % 2 !== 0) throw new Error('Invalid Hex length')
      let res = ''
      for (let i = 0; i < clean.length; i += 2) {
        res += String.fromCharCode(parseInt(clean.substr(i, 2), 16))
      }
      return res
    }
  },
  {
    id: 'to-binary',
    name: 'To Binary',
    category: 'Data Formats',
    description: 'Converts data to binary strings.',
    process: (input) => {
      return Array.from(input).map(c => c.charCodeAt(0).toString(2).padStart(8, '0')).join(' ')
    }
  },
  {
    id: 'from-binary',
    name: 'From Binary',
    category: 'Data Formats',
    description: 'Converts binary strings back to data.',
    process: (input) => {
      const clean = input.replace(/[^01]/g, '')
      if (clean.length % 8 !== 0) throw new Error('Invalid Binary length')
      let res = ''
      for (let i = 0; i < clean.length; i += 8) {
        res += String.fromCharCode(parseInt(clean.substr(i, 8), 2))
      }
      return res
    }
  },
  {
    id: 'to-decimal',
    name: 'To Decimal',
    category: 'Data Formats',
    description: 'Converts data to decimal strings.',
    process: (input) => {
      return Array.from(input).map(c => c.charCodeAt(0).toString(10)).join(' ')
    }
  },
  {
    id: 'from-decimal',
    name: 'From Decimal',
    category: 'Data Formats',
    description: 'Converts decimal strings back to data.',
    process: (input) => {
      const parts = input.trim().split(/\s+/)
      return parts.map(p => String.fromCharCode(parseInt(p, 10))).join('')
    }
  },
  // --- Hashing ---
  {
    id: 'sha256',
    name: 'SHA-256',
    category: 'Hashing',
    description: 'Generates a SHA-256 hash of the input.',
    process: async (input) => {
      if (!input) return ''
      const msgUint8 = new TextEncoder().encode(input)
      const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8)
      const hashArray = Array.from(new Uint8Array(hashBuffer))
      return hashArray.map(b => b.toString(16).padStart(2, '0')).join('')
    }
  },
  {
    id: 'sha1',
    name: 'SHA-1',
    category: 'Hashing',
    description: 'Generates a SHA-1 hash of the input.',
    process: async (input) => {
      if (!input) return ''
      const msgUint8 = new TextEncoder().encode(input)
      const hashBuffer = await crypto.subtle.digest('SHA-1', msgUint8)
      const hashArray = Array.from(new Uint8Array(hashBuffer))
      return hashArray.map(b => b.toString(16).padStart(2, '0')).join('')
    }
  },
  // --- Utils ---
  {
    id: 'json-beautify',
    name: 'JSON Beautify',
    category: 'Utils',
    description: 'Formats JSON with indentation.',
    process: (input) => {
      if (!input) return ''
      try {
        return JSON.stringify(JSON.parse(input), null, 4)
      } catch (e) {
        throw new Error('Invalid JSON input')
      }
    }
  },
  {
    id: 'json-minify',
    name: 'JSON Minify',
    category: 'Utils',
    description: 'Removes whitespace from JSON.',
    process: (input) => {
      if (!input) return ''
      try {
        return JSON.stringify(JSON.parse(input))
      } catch (e) {
        throw new Error('Invalid JSON input')
      }
    }
  },
  {
    id: 'reverse',
    name: 'Reverse',
    category: 'Utils',
    description: 'Reverses the input string.',
    process: (input) => input.split('').reverse().join('')
  },
  {
    id: 'to-uppercase',
    name: 'To Uppercase',
    category: 'Utils',
    description: 'Converts input to uppercase.',
    process: (input) => input.toUpperCase()
  },
  {
    id: 'to-lowercase',
    name: 'To Lowercase',
    category: 'Utils',
    description: 'Converts input to lowercase.',
    process: (input) => input.toLowerCase()
  }
]

const categorizedOperations = computed(() => {
  const result: Record<string, Operation[]> = {}
  const filtered = operations.filter(op => 
    op.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
    op.category.toLowerCase().includes(searchQuery.value.toLowerCase())
  )
  
  filtered.forEach(op => {
    if (!result[op.category]) result[op.category] = []
    result[op.category].push(op)
  })
  return result
})

// --- Logic ---

const cloneOperation = (op: Operation) => {
  const defaultValues: Record<string, any> = {}
  op.options?.forEach(opt => {
    if (opt.type === 'select' && opt.choices) {
      defaultValues[opt.name] = opt.choices[0]
    } else {
      defaultValues[opt.name] = ''
    }
  })

  return {
    ...op,
    instanceId: uuidv4(),
    values: defaultValues,
    disabled: false
  }
}

const addToRecipe = (op: Operation) => {
  const item = cloneOperation(op)
  recipe.value.push(item)
}

const toggleOperation = (element: any) => {
  element.disabled = !element.disabled
  autoBake()
}

const removeFromRecipe = (index: number) => {
  recipe.value.splice(index, 1)
  autoBake()
}

const clearRecipe = () => {
  recipe.value = []
  output.value = ''
  error.value = ''
}

const bake = async () => {
  if (baking.value) return
  baking.value = true
  error.value = ''
  
  let currentData = input.value
  
  try {
    for (const op of recipe.value) {
      if (op.disabled) continue
      currentData = await op.process(currentData, op.values)
    }
    output.value = currentData
  } catch (err: any) {
    error.value = err.message
    output.value = ''
  } finally {
    baking.value = false
  }
}

const autoBake = () => {
  if (isAutoBake.value) {
    bake()
  }
}

const copyOutput = () => {
  navigator.clipboard.writeText(output.value)
}

watch(input, () => autoBake())
watch(recipe, () => autoBake(), { deep: true })

</script>

<style scoped>
.editor-area {
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 0.85rem;
  line-height: 1.6;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--fallback-bc,oklch(var(--bc)/0.1));
  border-radius: 10px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: var(--fallback-bc,oklch(var(--bc)/0.2));
}

.ghost-item {
  opacity: 0.5;
  background: var(--fallback-p,oklch(var(--p)/0.1)) !important;
  border: 1px dashed var(--fallback-p,oklch(var(--p)/1)) !important;
}

.op-item:active {
  cursor: grabbing;
}

.recipe-card {
  transition: transform 0.1s ease;
}

.recipe-card:active {
  transform: scale(0.98);
}

.collapse-title::after {
  font-size: 10px;
  right: 0.5rem;
}

/* Animations for draggable */
.list-enter-active,
.list-leave-active {
  transition: all 0.5s ease;
}
.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateX(30px);
}
</style>
