<template>
  <Transition name="drawer">
    <div v-if="showWorkflowListPanel" class="absolute top-0 left-0 bottom-0 w-[300px] bg-base-100 shadow-xl border-r border-base-300 z-20 flex flex-col">
      <div class="p-3 flex items-center justify-between border-b border-base-300 flex-shrink-0">
        <h2 class="text-sm font-semibold">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.title') }}</h2>
        <button class="btn btn-xs btn-ghost" @click="onClose">✕</button>
      </div>

      <div class="tabs tabs-boxed mx-3 mt-3 flex-shrink-0">
        <a class="tab tab-xs flex-1" :class="{ 'tab-active': workflowListTab === 'workflows' }" @click="onSwitchTab('workflows')">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.myWorkflows') }}</a>
        <a class="tab tab-xs flex-1" :class="{ 'tab-active': workflowListTab === 'templates' }" @click="onSwitchTab('templates')">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.templates') }}</a>
      </div>

      <div class="px-3 pt-3 flex-shrink-0">
        <div class="relative">
          <input :value="workflowListSearch" class="input input-bordered input-xs w-full pr-7" :placeholder="t('trafficAnalysis.workflowStudio.workflowListPanel.searchPlaceholder')" @input="$emit('update:workflowListSearch', ($event.target as HTMLInputElement).value)" />
          <button v-if="workflowListSearch" class="btn btn-xs btn-ghost absolute right-0.5 top-1/2 -translate-y-1/2 h-5 w-5 min-h-0 p-0" @click="$emit('update:workflowListSearch', '')">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto p-3 space-y-2">
        <template v-if="workflowListTab === 'workflows'">
          <div v-if="filteredWorkflowList.length === 0" class="text-center text-base-content/60 py-6">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
            <p class="text-xs">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.emptyWorkflows') }}</p>
          </div>
          <div v-for="workflow in filteredWorkflowList" :key="workflow.id" class="card bg-base-200 hover:bg-base-300 cursor-pointer transition-colors" :class="{ 'ring-2 ring-primary': workflow.id === workflowId }" @click="onLoadWorkflow(workflow.id)">
            <div class="card-body p-2">
              <div class="flex items-start justify-between gap-1">
                <div class="flex-1 min-w-0">
                  <h4 class="font-semibold text-xs truncate">{{ workflow.name }}</h4>
                  <p v-if="workflow.description" class="text-xs text-base-content/70 mt-0.5 line-clamp-1">{{ workflow.description }}</p>
                  <div class="flex flex-wrap gap-1 mt-1 text-xs text-base-content/60">
                    <span class="badge badge-xs badge-ghost">{{ workflow.version }}</span>
                    <span v-if="workflow.is_tool" class="badge badge-xs badge-secondary">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.aiTool') }}</span>
                  </div>
                </div>
                <div class="flex gap-0.5">
                  <button class="btn btn-xs btn-ghost h-6 w-6 min-h-0 p-0" :title="t('trafficAnalysis.workflowStudio.header.editMetadataTooltip')" @click.stop="onEditWorkflowMetadata(workflow.id)"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5M18.586 3.586a2 2 0 112.828 2.828L12 15.828l-4 1 1-4 9.586-9.242z" /></svg></button>
                  <button class="btn btn-xs btn-ghost h-6 w-6 min-h-0 p-0" :title="t('trafficAnalysis.workflowStudio.workflowListPanel.duplicate')" @click.stop="onCloneWorkflow(workflow.id)"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg></button>
                  <button class="btn btn-xs btn-ghost text-error h-6 w-6 min-h-0 p-0" :title="t('trafficAnalysis.workflowStudio.workflowListPanel.delete')" @click.stop="onDeleteWorkflow(workflow.id)"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg></button>
                </div>
              </div>
            </div>
          </div>
        </template>

        <template v-else>
          <div v-if="filteredTemplateList.length === 0" class="text-center text-base-content/60 py-6">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 mx-auto mb-2 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>
            <p class="text-xs">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.emptyTemplates') }}</p>
          </div>
          <div v-for="templateItem in filteredTemplateList" :key="templateItem.id" class="card bg-base-200 hover:bg-base-300 cursor-pointer transition-colors">
            <div class="card-body p-2">
              <div class="flex items-start justify-between gap-1">
                <div class="flex-1 min-w-0">
                  <h4 class="font-semibold text-xs truncate flex items-center gap-1">
                    {{ templateItem.name }}
                    <span class="badge badge-primary badge-xs">{{ t('trafficAnalysis.workflowStudio.workflowListPanel.templateBadge') }}</span>
                  </h4>
                  <p v-if="templateItem.description" class="text-xs text-base-content/70 mt-0.5 line-clamp-1">{{ templateItem.description }}</p>
                  <div class="flex flex-wrap gap-1 mt-1 text-xs text-base-content/60">
                    <span>{{ t('trafficAnalysis.workflowStudio.workflowListPanel.nodeCount', { count: templateItem.node_count || 0 }) }}</span>
                  </div>
                </div>
                <div class="flex gap-0.5">
                  <button class="btn btn-xs btn-primary h-6 w-6 min-h-0 p-0" :title="t('trafficAnalysis.workflowStudio.workflowListPanel.useTemplate')" @click.stop="onUseTemplate(templateItem.id)"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg></button>
                  <button class="btn btn-xs btn-ghost text-error h-6 w-6 min-h-0 p-0" :title="t('trafficAnalysis.workflowStudio.workflowListPanel.deleteTemplate')" @click.stop="onDeleteTemplate(templateItem.id)"><svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg></button>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <div class="p-3 border-t border-base-300 flex-shrink-0">
        <button v-if="workflowListTab === 'workflows'" class="btn btn-xs btn-primary w-full" @click="onCreateNewWorkflow">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
          {{ t('trafficAnalysis.workflowStudio.workflowListPanel.newWorkflow') }}
        </button>
        <button v-if="workflowListTab === 'templates'" class="btn btn-xs btn-primary w-full" :disabled="!workflowName.trim()" @click="onSaveCurrentAsTemplate">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" /></svg>
          {{ t('trafficAnalysis.workflowStudio.workflowListPanel.saveAsTemplate') }}
        </button>
      </div>
    </div>
  </Transition>

  <Transition name="fade">
    <div v-if="showWorkflowListPanel" class="absolute inset-0 bg-black/20 z-10" @click="onClose"></div>
  </Transition>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  showWorkflowListPanel: boolean
  workflowListTab: 'workflows' | 'templates'
  workflowListSearch: string
  workflowId: string
  workflowName: string
  filteredWorkflowList: any[]
  filteredTemplateList: any[]
  onClose: () => void
  onSwitchTab: (tab: 'workflows' | 'templates') => void
  onLoadWorkflow: (id: string) => void
  onEditWorkflowMetadata: (id: string) => void
  onCloneWorkflow: (id: string) => void
  onDeleteWorkflow: (id: string) => void
  onUseTemplate: (id: string) => void
  onDeleteTemplate: (id: string) => void
  onCreateNewWorkflow: () => void
  onSaveCurrentAsTemplate: () => void
}>()

defineEmits<{
  'update:workflowListSearch': [value: string]
}>()
</script>
