<template>
  <div class="rounded-lg border border-dashed border-base-300 bg-base-200/40 p-6 mb-4">
    <div class="flex items-start justify-between gap-4 flex-wrap">
      <div class="space-y-2">
        <div class="font-medium">
          {{ t('dictionary.emptyRuleTitle', '当前字典还没有规则') }}
        </div>
        <div class="text-sm text-base-content/70">
          {{ t(`dictionary.${hintKey}`, '你可以先新增一条 starter 规则，或者导入现有 JSON / 规则文件。') }}
        </div>
      </div>

      <div class="flex gap-2 flex-wrap">
        <button
          v-if="showNmapImport"
          class="btn btn-secondary btn-sm"
          @click="emit('import-nmap')"
        >
          <i class="fas fa-file-import mr-2"></i>
          {{ t('dictionary.importNmapServiceProbes', '导入 Nmap Service Probes') }}
        </button>
        <button class="btn btn-primary btn-sm" @click="emit('create-starter-rule')">
          <i class="fas fa-plus mr-2"></i>
          {{ t('dictionary.createStarterRule', '新增 Starter 规则') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  getSubtypeEmptyStateHintKey,
  subtypeSupportsNmapImport,
} from '@/components/Dictionary/dictionarySubtypeConfig'

const props = defineProps<{
  subtype: string | null
  supportsNmapImport: boolean
}>()

const emit = defineEmits<{
  (e: 'import-nmap'): void
  (e: 'create-starter-rule'): void
}>()

const { t } = useI18n()
const hintKey = computed(() => getSubtypeEmptyStateHintKey(props.subtype))
const showNmapImport = computed(() =>
  props.supportsNmapImport && subtypeSupportsNmapImport(props.subtype)
)
</script>
