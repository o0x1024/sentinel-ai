<template>
  <div class="card bg-base-100 shadow-xl mb-4">
    <div class="card-body py-3">
      <div class="flex justify-between items-center">
        <h3 class="card-title text-lg">{{ title }}</h3>
        <div class="flex gap-2">
          <button class="btn btn-sm btn-outline" @click="onNewWorkflow" :title="newWorkflowTooltip">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
            {{ newWorkflowLabel }}
          </button>
          <button class="btn btn-sm btn-outline btn-secondary" @click="onOpenAiGenerateModal" :title="aiGenerateTooltip"><i class="fas fa-magic mr-1"></i>{{ aiGenerateLabel }}</button>
          <div class="join">
            <button class="btn btn-sm join-item" @click="onZoomOut"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" /></svg></button>
            <button class="btn btn-sm join-item" @click="onResetZoom">{{ zoomLabel }}</button>
            <button class="btn btn-sm join-item" @click="onZoomIn"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v3m0 0v3m0-3h3m-3 0H7" /></svg></button>
          </div>
          <button class="btn btn-sm btn-outline" @click="onToggleFullscreen"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" /></svg></button>
          <button class="btn btn-sm btn-outline" @click="onFitToView" :title="fitToViewTooltip"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" /></svg></button>
          <button class="btn btn-sm btn-outline" @click="onResetCanvasView" :title="resetViewTooltip"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg></button>
          <button class="btn btn-sm" :class="showMinimap ? 'btn-primary' : 'btn-outline'" @click="onToggleMinimap" :title="minimapTooltip"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" /></svg></button>
          <button class="btn btn-sm btn-outline" @click="onArrangeNodes" :title="arrangeNodesTooltip">{{ arrangeNodesLabel }}</button>
          <div class="join">
            <button class="btn btn-sm join-item" @click="onUndo" :disabled="!canUndo" :title="undoTooltip"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg></button>
            <button class="btn btn-sm join-item" @click="onRedo" :disabled="!canRedo" :title="redoTooltip"><svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" /></svg></button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title: string
  newWorkflowLabel: string
  newWorkflowTooltip: string
  aiGenerateLabel: string
  aiGenerateTooltip: string
  zoomLabel: string
  fitToViewTooltip: string
  resetViewTooltip: string
  minimapTooltip: string
  arrangeNodesLabel: string
  arrangeNodesTooltip: string
  undoTooltip: string
  redoTooltip: string
  canUndo: boolean
  canRedo: boolean
  showMinimap: boolean
  onNewWorkflow: () => void
  onOpenAiGenerateModal: () => void
  onZoomOut: () => void
  onResetZoom: () => void
  onZoomIn: () => void
  onToggleFullscreen: () => void
  onFitToView: () => void
  onResetCanvasView: () => void
  onToggleMinimap: () => void
  onArrangeNodes: () => void
  onUndo: () => void
  onRedo: () => void
}>()
</script>
