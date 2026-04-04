<template>
  <div class="card bg-transparent shadow-none">
    <div class="card-body gap-4 px-0 py-0">
      <div class="grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-4">
        <SystemAgentListPanel
          :loading="loading"
          :items="profileListItems"
          :selected-id="selectedProfileId"
          @select="selectProfile"
        />

        <SystemAgentDetailLayout :has-selection="!!selectedProfile">
          <template #toolbar>
            <SystemAgentToolbarPanel :loading="loading" @seed="seedDefaults" @refresh="refreshAll" />
          </template>

          <template #empty>
            <SystemAgentEmptyStatePanel />
          </template>

          <template v-if="selectedProfile">
            <SystemAgentStatsCards
              v-model="statsWindow"
              :runs="runs"
              :recent-findings="recentFindings"
              :versions="versions"
              :total-findings-count="totalFindingsCount"
              :false-positive-findings-count="falsePositiveFindingsCount"
              :stats-mode="selectedProfileStatsMode"
            />

            <SystemAgentProfileHeaderPanel
              :name="selectedProfile.name"
              :description="selectedProfileDescription"
              :mode-badge="selectedProfileModeBadge"
              :passive-event-name="selectedProfilePassiveEventName"
              :enabled="selectedProfile.enabled"
              @update:enabled="selectedProfile.enabled = $event"
            />

            <SystemAgentPromptPatchPanel
              :name="selectedProfile.name"
              :prompt-patch="selectedProfile.promptPatch || ''"
              :placeholder="promptPatchPlaceholder"
              :guidance="promptPatchGuidance"
              @update:prompt-patch="selectedProfile.promptPatch = $event"
            />

            <SystemAgentBehaviorSourcePanel
              v-if="showBehaviorSourcePanel"
              :settings="behaviorSignalSettings"
            />

            <div class="grid grid-cols-1 gap-4">
              <SystemAgentToolBindingPanel v-model="toolBindingValue" />
              <SystemAgentSafetyPolicyPanel
                v-model="safetyPolicyValue"
                :safety-mode="selectedProfileSafetyMode"
              />
            </div>

            <SystemAgentAdvancedSettingsPanel
              :profile-id="selectedProfile.id"
              :base-prompt-id="selectedProfile.basePromptId"
              :cooldown-secs="selectedProfile.cooldownSecs"
              :max-concurrency="selectedProfile.maxConcurrency"
              @update:cooldown-secs="selectedProfile.cooldownSecs = $event"
              @update:max-concurrency="selectedProfile.maxConcurrency = $event"
            />

            <SystemAgentAutoSaveStatusBar
              :status-text="autoSaveStatusText"
              :status-class="autoSaveStatusClass"
            />

            <SystemAgentDebugPanel
              :profile="selectedProfile"
              :manual-input-text="manualInputText"
              :dispatch-payload-text="dispatchPayloadText"
              :running="running"
              :dispatching="dispatching"
              @update:manual-input-text="manualInputText = $event"
              @update:dispatch-payload-text="dispatchPayloadText = $event"
              @run="runSelectedProfile"
              @dispatch="dispatchSelectedProfileEvent"
            />

            <SystemAgentRunsPanel
              :runs="runs"
              :disabled="!selectedProfile.id"
              @refresh="loadRuns"
            />

            <SystemAgentVersionsPanel
              :versions="versions"
              :disabled="!selectedProfile.id"
              @refresh="loadVersions"
            />

            <SystemAgentRecentFindingsPanel
              v-if="showRecentFindingsPanel"
              :findings="recentFindings"
              :disabled="!selectedProfile.id"
              :window-start="statsWindowStart"
              @refresh="loadRecentFindings"
            />

            <SystemAgentSkillInsightsPanel
              v-if="showRecentFindingsPanel"
              :findings="recentFindings"
              :window-start="statsWindowStart"
            />
          </template>
        </SystemAgentDetailLayout>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import SystemAgentAdvancedSettingsPanel from './system-agent/SystemAgentAdvancedSettingsPanel.vue'
import SystemAgentAutoSaveStatusBar from './system-agent/SystemAgentAutoSaveStatusBar.vue'
import SystemAgentBehaviorSourcePanel from './system-agent/SystemAgentBehaviorSourcePanel.vue'
import SystemAgentDebugPanel from './system-agent/SystemAgentDebugPanel.vue'
import SystemAgentDetailLayout from './system-agent/SystemAgentDetailLayout.vue'
import SystemAgentEmptyStatePanel from './system-agent/SystemAgentEmptyStatePanel.vue'
import SystemAgentListPanel from './system-agent/SystemAgentListPanel.vue'
import SystemAgentProfileHeaderPanel from './system-agent/SystemAgentProfileHeaderPanel.vue'
import SystemAgentPromptPatchPanel from './system-agent/SystemAgentPromptPatchPanel.vue'
import SystemAgentRecentFindingsPanel from './system-agent/SystemAgentRecentFindingsPanel.vue'
import SystemAgentSkillInsightsPanel from './system-agent/SystemAgentSkillInsightsPanel.vue'
import SystemAgentToolbarPanel from './system-agent/SystemAgentToolbarPanel.vue'
import SystemAgentRunsPanel from './system-agent/SystemAgentRunsPanel.vue'
import SystemAgentSafetyPolicyPanel from './system-agent/SystemAgentSafetyPolicyPanel.vue'
import SystemAgentStatsCards from './system-agent/SystemAgentStatsCards.vue'
import SystemAgentVersionsPanel from './system-agent/SystemAgentVersionsPanel.vue'
import SystemAgentToolBindingPanel from './SystemAgentToolBindingPanel.vue'
import { useSystemAgentSettingsController } from './system-agent/useSystemAgentSettingsController'

const {
  loading,
  running,
  dispatching,
  runs,
  versions,
  recentFindings,
  totalFindingsCount,
  falsePositiveFindingsCount,
  statsWindow,
  selectedProfileId,
  selectedProfile,
  safetyPolicyValue,
  manualInputText,
  dispatchPayloadText,
  toolBindingValue,
  promptPatchPlaceholder,
  promptPatchGuidance,
  selectedProfileDescription,
  autoSaveStatusText,
  autoSaveStatusClass,
  behaviorSignalSettings,
  showBehaviorSourcePanel,
  showRecentFindingsPanel,
  selectedProfileModeBadge,
  selectedProfilePassiveEventName,
  selectedProfileStatsMode,
  selectedProfileSafetyMode,
  statsWindowStart,
  profileListItems,
  selectProfile,
  runSelectedProfile,
  dispatchSelectedProfileEvent,
  seedDefaults,
  refreshAll,
  loadRuns,
  loadVersions,
  loadRecentFindings,
} = useSystemAgentSettingsController()
</script>
