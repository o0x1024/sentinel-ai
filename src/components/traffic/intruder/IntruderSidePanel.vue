<template>
  <section class="flex h-full min-h-0 rounded-r-lg border border-l-0 border-base-300 bg-base-100">
    <div class="min-w-0 flex-1 overflow-hidden">
      <div v-if="activeTab === 'payloads'" class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.payloads') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <label class="form-control">
            <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.payloadPosition') }}</span>
            <select v-model="selectedPayloadSetId" class="select select-bordered select-sm">
              <option v-for="payloadSet in payloadSets" :key="payloadSet.id" :value="payloadSet.id">
                {{ payloadSet.name }}
              </option>
            </select>
          </label>

          <label class="form-control">
            <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.payloadType') }}</span>
            <select
              :value="activePayloadSet?.payloadType || 'simpleList'"
              class="select select-bordered select-sm"
              @change="handlePayloadTypeChange(($event.target as HTMLSelectElement).value as IntruderPayloadSet['payloadType'])"
            >
              <option value="simpleList">{{ $t('trafficAnalysis.intruder.labels.simpleList') }}</option>
              <option value="extensionGenerated">{{ $t('trafficAnalysis.intruder.labels.extensionGenerated') }}</option>
              <option value="numbers">{{ $t('trafficAnalysis.intruder.labels.numbers') }}</option>
              <option value="dates">{{ $t('trafficAnalysis.intruder.labels.dates') }}</option>
              <option value="runtimeFile">{{ $t('trafficAnalysis.intruder.labels.runtimeFile') }}</option>
              <option value="characterList">{{ $t('trafficAnalysis.intruder.labels.characterList') }}</option>
              <option value="nullPayloads">{{ $t('trafficAnalysis.intruder.labels.nullPayloads') }}</option>
              <option value="characterSubstitution">{{ $t('trafficAnalysis.intruder.labels.characterSubstitution') }}</option>
              <option value="usernameGenerator">{{ $t('trafficAnalysis.intruder.labels.usernameGenerator') }}</option>
            </select>
          </label>

          <div class="grid grid-cols-2 gap-3 text-xs text-base-content/70">
            <div>
              <div>{{ $t('trafficAnalysis.intruder.labels.payloadCount') }}</div>
              <div class="mt-1 font-semibold text-base-content">{{ activePayloadCount }}</div>
            </div>
            <div>
              <div>{{ $t('trafficAnalysis.intruder.labels.requestCount') }}</div>
              <div class="mt-1 font-semibold text-base-content">{{ estimatedRequests }}</div>
            </div>
          </div>

          <div class="rounded-lg border border-base-300">
            <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.payloadConfiguration') }}
            </div>

            <div v-if="activePayloadSet?.payloadType === 'simpleList'" class="grid grid-cols-[6rem_1fr] gap-3 p-3">
              <div class="space-y-2">
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="pastePayloads">
                  {{ $t('trafficAnalysis.intruder.actions.paste') }}
                </button>
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="deduplicatePayloads">
                  {{ $t('trafficAnalysis.intruder.actions.deduplicate') }}
                </button>
                <button class="btn btn-sm btn-ghost w-full justify-start" type="button" @click="clearPayloads">
                  {{ $t('trafficAnalysis.intruder.actions.clear') }}
                </button>
              </div>

              <textarea
                :value="activePayloadSet?.payloadsText || ''"
                class="h-64 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.payloads')"
                @input="updateActivePayloadSet({ payloadsText: ($event.target as HTMLTextAreaElement).value })"
              ></textarea>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'extensionGenerated'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.extensionPlugin') }}</span>
                <select
                  :value="activePayloadSet.pluginId"
                  class="select select-bordered select-sm"
                  @change="handlePluginChange(($event.target as HTMLSelectElement).value)"
                >
                  <option value="">{{ $t('trafficAnalysis.intruder.labels.pleaseSelect') }}</option>
                  <option v-for="plugin in availablePayloadPlugins" :key="plugin.id" :value="plugin.id">
                    {{ plugin.name }}
                  </option>
                </select>
              </label>

              <div
                v-if="selectedPayloadPlugin?.description"
                class="rounded-lg border border-base-300 bg-base-200 px-3 py-2 text-xs text-base-content/70"
              >
                {{ selectedPayloadPlugin.description }}
              </div>

              <div v-if="activePayloadSet.pluginId && !selectedPayloadPlugin" class="text-xs text-warning">
                {{ $t('trafficAnalysis.intruder.messages.selectedPluginUnavailable') }}
              </div>

              <div class="flex flex-wrap gap-2">
                <button
                  class="btn btn-sm btn-ghost"
                  type="button"
                  :disabled="!activePayloadSet.pluginId"
                  @click="openPluginConfigDialog"
                >
                  {{ $t('trafficAnalysis.intruder.actions.configurePlugin') }}
                </button>
                <button
                  class="btn btn-sm btn-primary"
                  type="button"
                  :disabled="!activePayloadSet.pluginId || pluginPreviewLoading"
                  @click="generatePluginPayloadPreview"
                >
                  <span v-if="pluginPreviewLoading" class="loading loading-spinner loading-xs"></span>
                  {{ $t('trafficAnalysis.intruder.actions.generatePreview') }}
                </button>
              </div>

              <div class="grid grid-cols-2 gap-3 text-xs text-base-content/70">
                <div>
                  <div>{{ $t('trafficAnalysis.intruder.labels.cachedPayloads') }}</div>
                  <div class="mt-1 font-semibold text-base-content">{{ activePayloadCount }}</div>
                </div>
                <div>
                  <div>{{ $t('trafficAnalysis.intruder.labels.pluginConfigured') }}</div>
                  <div class="mt-1 font-semibold text-base-content">
                    {{ hasPluginConfig ? $t('trafficAnalysis.intruder.labels.yes') : $t('trafficAnalysis.intruder.labels.no') }}
                  </div>
                </div>
              </div>

              <textarea
                :value="activePayloadSet.payloadsText"
                class="h-56 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                :placeholder="$t('trafficAnalysis.intruder.placeholders.generatedPayloads')"
                readonly
              ></textarea>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'runtimeFile'" class="grid gap-3 p-4">
              <div class="flex items-center gap-2">
                <button class="btn btn-sm btn-ghost" type="button" @click="loadPayloadFile">
                  {{ $t('trafficAnalysis.intruder.actions.loadFile') }}
                </button>
                <input
                  :value="activePayloadSet.filePath"
                  type="text"
                  readonly
                  class="input input-bordered input-sm flex-1"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.payloadFile')"
                />
              </div>
              <textarea
                :value="activePayloadSet?.payloadsText || ''"
                class="h-56 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                readonly
              ></textarea>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'numbers'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberFrom') }}</span>
                <input
                  :value="activePayloadSet.numberFrom"
                  type="number"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberFrom: Number(($event.target as HTMLInputElement).value) || 0 })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberTo') }}</span>
                <input
                  :value="activePayloadSet.numberTo"
                  type="number"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberTo: Number(($event.target as HTMLInputElement).value) || 0 })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberStep') }}</span>
                <input
                  :value="activePayloadSet.numberStep"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberStep: Math.max(1, Number(($event.target as HTMLInputElement).value) || 1) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.numberPadWidth') }}</span>
                <input
                  :value="activePayloadSet.numberPadWidth"
                  type="number"
                  min="0"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ numberPadWidth: Math.max(0, Number(($event.target as HTMLInputElement).value) || 0) })"
                />
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'dates'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateFrom') }}</span>
                <input
                  :value="activePayloadSet.dateFrom"
                  type="date"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateFrom: ($event.target as HTMLInputElement).value })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateTo') }}</span>
                <input
                  :value="activePayloadSet.dateTo"
                  type="date"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateTo: ($event.target as HTMLInputElement).value })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateStepDays') }}</span>
                <input
                  :value="activePayloadSet.dateStepDays"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ dateStepDays: Math.max(1, Number(($event.target as HTMLInputElement).value) || 1) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.dateFormat') }}</span>
                <select
                  :value="activePayloadSet.dateFormat"
                  class="select select-bordered select-sm"
                  @change="updateActivePayloadSet({ dateFormat: ($event.target as HTMLSelectElement).value as IntruderPayloadSet['dateFormat'] })"
                >
                  <option value="yyyy-MM-dd">yyyy-MM-dd</option>
                  <option value="yyyyMMdd">yyyyMMdd</option>
                  <option value="MM/dd/yyyy">MM/dd/yyyy</option>
                </select>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'characterList'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.characters') }}</span>
                <textarea
                  :value="activePayloadSet.characterList"
                  class="h-40 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.characterList')"
                  @input="updateActivePayloadSet({ characterList: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'nullPayloads'" class="grid gap-3 p-4 md:grid-cols-2">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.nullCount') }}</span>
                <input
                  :value="activePayloadSet.nullCount"
                  type="number"
                  min="0"
                  max="100000"
                  class="input input-bordered input-sm"
                  @input="updateActivePayloadSet({ nullCount: Math.max(0, Number(($event.target as HTMLInputElement).value) || 0) })"
                />
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.nullValue') }}</span>
                <input
                  :value="activePayloadSet.nullValue"
                  type="text"
                  class="input input-bordered input-sm"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.nullValue')"
                  @input="updateActivePayloadSet({ nullValue: ($event.target as HTMLInputElement).value })"
                />
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'characterSubstitution'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.sourcePayloads') }}</span>
                <textarea
                  :value="activePayloadSet.substitutionSource"
                  class="h-32 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.substitutionSource')"
                  @input="updateActivePayloadSet({ substitutionSource: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.substitutionRules') }}</span>
                <textarea
                  :value="activePayloadSet.substitutionRules"
                  class="h-28 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.substitutionRules')"
                  @input="updateActivePayloadSet({ substitutionRules: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>

            <div v-else-if="activePayloadSet?.payloadType === 'usernameGenerator'" class="grid gap-3 p-4">
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.firstNames') }}</span>
                <textarea
                  :value="activePayloadSet.usernameFirstNames"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameFirstNames')"
                  @input="updateActivePayloadSet({ usernameFirstNames: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.lastNames') }}</span>
                <textarea
                  :value="activePayloadSet.usernameLastNames"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameLastNames')"
                  @input="updateActivePayloadSet({ usernameLastNames: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.usernameFormats') }}</span>
                <textarea
                  :value="activePayloadSet.usernameFormats"
                  class="h-24 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none transition focus:border-primary"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.usernameFormats')"
                  @input="updateActivePayloadSet({ usernameFormats: ($event.target as HTMLTextAreaElement).value })"
                ></textarea>
              </label>
            </div>
          </div>

          <IntruderPayloadProcessingPanel
            :rules="payloadProcessingRules"
            @update:rules="$emit('update:payloadProcessingRules', $event)"
          />

          <IntruderPluginProcessorPanel
            :title="$t('trafficAnalysis.intruder.labels.pluginPayloadProcessing')"
            :description="$t('trafficAnalysis.intruder.help.pluginPayloadProcessingHint')"
            category="payload_processor"
            :processors="payloadProcessorPlugins"
            :empty-text="$t('trafficAnalysis.intruder.empty.noPayloadProcessorPlugins')"
            @update:processors="$emit('update:payloadProcessorPlugins', $event)"
          />

          <div class="rounded-lg border border-base-300">
            <div class="border-b border-base-300 bg-base-200 px-4 py-2 text-xs font-semibold uppercase tracking-wide text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.payloadEncoding') }}
            </div>
            <div class="space-y-3 p-4">
              <p class="text-sm text-base-content/70">
                {{ $t('trafficAnalysis.intruder.help.payloadEncodingHint') }}
              </p>
              <label class="flex items-center gap-2">
                <input
                  :checked="activePayloadSet?.urlEncode || false"
                  type="checkbox"
                  class="checkbox checkbox-sm"
                  @change="updateActivePayloadSet({ urlEncode: ($event.target as HTMLInputElement).checked })"
                />
                <span>{{ $t('trafficAnalysis.intruder.labels.urlEncodePayloads') }}</span>
              </label>

              <label class="form-control">
                <span class="label-text text-xs">{{ $t('trafficAnalysis.intruder.labels.urlEncodeCharacters') }}</span>
                <input
                  :value="activePayloadSet?.urlEncodeCharacters || ''"
                  type="text"
                  class="input input-bordered input-sm font-mono"
                  :placeholder="$t('trafficAnalysis.intruder.placeholders.urlEncodeCharacters')"
                  :disabled="!(activePayloadSet?.urlEncode || false)"
                  @input="updateActivePayloadSet({ urlEncodeCharacters: ($event.target as HTMLInputElement).value })"
                />
              </label>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="activeTab === 'resourcePool'" class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.resourcePool') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <IntruderResourcePoolPanel
            :pools="resourcePoolPresets"
            :selected-pool-id="selectedResourcePoolId"
            :attack-options="{
              concurrency: attackOptions.concurrency,
              delayMs: attackOptions.delayMs,
              randomDelayMs: attackOptions.randomDelayMs,
              delayIncrementMs: attackOptions.delayIncrementMs,
              autoThrottleEnabled: attackOptions.autoThrottleEnabled,
              autoThrottleStatusCodes: attackOptions.autoThrottleStatusCodes,
            }"
            @select:pool="$emit('selectResourcePoolPreset', $event)"
            @update:attack-options="$emit('update:attackOptions', { ...attackOptions, ...$event })"
            @upsert:pool="$emit('upsertResourcePool', $event)"
            @delete:pool="$emit('deleteResourcePool', $event)"
          />
        </div>
      </div>

      <div v-else class="flex h-full min-h-0 flex-col">
        <div class="border-b border-base-300 px-4 py-3">
          <h3 class="text-sm font-semibold">{{ $t('trafficAnalysis.intruder.sections.settings') }}</h3>
        </div>

        <div class="space-y-4 overflow-auto p-4 text-sm">
          <IntruderPluginProcessorPanel
            :title="$t('trafficAnalysis.intruder.labels.requestProcessing')"
            :description="$t('trafficAnalysis.intruder.help.requestProcessingHint')"
            category="request_processor"
            :processors="requestProcessorPlugins"
            :empty-text="$t('trafficAnalysis.intruder.empty.noRequestProcessorPlugins')"
            @update:processors="$emit('update:requestProcessorPlugins', $event)"
          />

          <div class="flex justify-end">
            <button class="btn btn-sm btn-primary" type="button" :disabled="requestProcessingPreviewLoading" @click="openRequestPreviewDialog">
              <span v-if="requestProcessingPreviewLoading" class="loading loading-spinner loading-xs"></span>
              {{ $t('trafficAnalysis.intruder.actions.previewProcessedRequest') }}
            </button>
          </div>

          <IntruderRequestHeadersPanel
            :attack-options="{
              updateContentLength: attackOptions.updateContentLength,
              setConnectionClose: attackOptions.setConnectionClose,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderErrorHandlingPanel
            :attack-options="{
              retryCount: attackOptions.retryCount,
              retryPauseMs: attackOptions.retryPauseMs,
              timeoutSecs: attackOptions.timeoutSecs,
              maxRequests: attackOptions.maxRequests,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderRedirectHandlingPanel
            :attack-options="{
              followRedirects: attackOptions.followRedirects,
              maxRedirects: attackOptions.maxRedirects,
              processCookiesInRedirects: attackOptions.processCookiesInRedirects,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderAttackResultsSettingsPanel
            :attack-options="{
              storeRequests: attackOptions.storeRequests,
              storeResponses: attackOptions.storeResponses,
              makeUnmodifiedBaseline: attackOptions.makeUnmodifiedBaseline,
              denialOfServiceMode: attackOptions.denialOfServiceMode,
              storeFullPayloads: attackOptions.storeFullPayloads,
            }"
            @update:options="updateAttackOptions"
          />

          <IntruderGrepMatchPanel
            :rules="grepMatchRules"
            @update:rules="$emit('update:grepMatchRules', $event)"
          />

          <IntruderGrepExtractPanel
            :rules="grepExtractRules"
            @update:rules="$emit('update:grepExtractRules', $event)"
          />

          <IntruderGrepPayloadPanel
            :settings="grepPayloadSettings"
            @update:settings="$emit('update:grepPayloadSettings', $event)"
          />

          <IntruderAutoPausePanel
            :enabled="attackOptions.autoPauseEnabled"
            :mode="attackOptions.autoPauseMode"
            :expressions="attackOptions.autoPauseExpressions"
            @update:enabled="updateOption('autoPauseEnabled', $event)"
            @update:mode="updateOption('autoPauseMode', $event)"
            @update:expressions="updateAutoPauseExpressions"
          />
        </div>
      </div>
    </div>

    <div class="flex w-12 flex-col border-l border-base-300 bg-base-200">
      <button
        class="flex-1 border-b border-base-300 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'payloads' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'payloads')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.payloads') }}</span>
      </button>
      <button
        class="flex-1 border-b border-base-300 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'resourcePool' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'resourcePool')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.resourcePool') }}</span>
      </button>
      <button
        class="flex-1 px-1 text-xs font-medium tracking-wide transition"
        :class="activeTab === 'settings' ? 'bg-base-100 text-primary' : 'text-base-content/70 hover:bg-base-300'"
        type="button"
        @click="$emit('update:activeTab', 'settings')"
      >
        <span class="side-label">{{ $t('trafficAnalysis.intruder.sections.settings') }}</span>
      </button>
    </div>
  </section>

  <IntruderPluginConfigDialog
    :open="pluginConfigDialogOpen"
    :title="pluginConfigDialogTitle"
    :plugin-id="activePayloadSet?.pluginId || ''"
    :preset-name="activePayloadSet?.pluginPresetName || ''"
    :schema="activePluginSchema"
    :model-value="activePayloadSet?.pluginConfig || '{}'"
    @update:model-value="updateActivePayloadSet({ pluginConfig: $event })"
    @update:preset-name="updateActivePayloadSet({ pluginPresetName: $event })"
    @close="pluginConfigDialogOpen = false"
  />

  <dialog :open="requestPreviewDialogOpen" class="modal" @click.self="requestPreviewDialogOpen = false">
    <div class="modal-box max-w-4xl">
      <div class="mb-4 flex items-center justify-between">
        <div>
          <h3 class="text-base font-semibold">{{ $t('trafficAnalysis.intruder.labels.requestProcessingPreview') }}</h3>
          <p v-if="requestProcessingPreviewPayloadSummary" class="mt-1 text-xs text-base-content/60">
            {{ requestProcessingPreviewPayloadSummary }}
          </p>
          <div
            v-if="requestPreviewPayloadSourceLines.length"
            class="mt-2 space-y-1 text-xs text-base-content/60"
          >
            <div class="font-medium text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.payloadSources') }}
            </div>
            <div
              v-for="line in requestPreviewPayloadSourceLines"
              :key="line"
            >
              {{ line }}
            </div>
          </div>
        </div>
        <button class="btn btn-ghost btn-xs" type="button" @click="requestPreviewDialogOpen = false">✕</button>
      </div>

      <div v-if="requestProcessingPreviewLoading" class="flex items-center justify-center py-10">
        <span class="loading loading-spinner loading-lg"></span>
      </div>
      <div v-else-if="requestProcessingPreviewError" class="rounded-lg border border-error/30 bg-error/10 px-4 py-3 text-sm text-error">
        {{ requestProcessingPreviewError }}
      </div>
      <div v-else class="space-y-4">
        <div class="space-y-3">
          <div class="text-xs font-medium uppercase tracking-wide text-base-content/60">
            {{ $t('trafficAnalysis.intruder.labels.requestProcessingDiff') }}
          </div>

          <div class="grid gap-3 sm:grid-cols-3">
            <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2">
              <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.requestLine') }}
              </div>
              <div class="mt-1 text-sm font-medium" :class="requestPreviewDiff.requestLineChanged ? 'text-warning' : 'text-base-content/70'">
                {{ formatDiffStatus(requestPreviewDiff.requestLineChanged) }}
              </div>
            </div>

            <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2">
              <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.headerChanges') }}
              </div>
              <div class="mt-1 text-sm font-medium">
                {{ requestPreviewDiff.headerChanges.length }}
              </div>
            </div>

            <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2">
              <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.body') }}
              </div>
              <div class="mt-1 text-sm font-medium" :class="requestPreviewDiff.bodyChanged ? 'text-warning' : 'text-base-content/70'">
                {{ formatDiffStatus(requestPreviewDiff.bodyChanged) }}
              </div>
            </div>

            <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2">
              <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.queryParameters') }}
              </div>
              <div class="mt-1 text-sm font-medium">
                {{ requestPreviewDiff.queryParameterDiff.changes.length }}
              </div>
            </div>

            <div class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2">
              <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.formParameters') }}
              </div>
              <div class="mt-1 text-sm font-medium">
                {{ requestPreviewDiff.formParameterDiff.changes.length }}
              </div>
            </div>
          </div>

          <div class="space-y-3 rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.requestLine') }}</div>
              <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(requestPreviewDiff.requestLineChanged ? 'changed' : 'same')">
                {{ formatDiffStatus(requestPreviewDiff.requestLineChanged) }}
              </div>
            </div>

            <div class="grid gap-3 lg:grid-cols-2">
              <label class="form-control gap-2">
                <span class="label-text text-xs font-medium text-base-content/70">
                  {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
                </span>
                <textarea
                  :value="requestPreviewDiff.requestLineBefore"
                  class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                  readonly
                ></textarea>
              </label>
              <label class="form-control gap-2">
                <span class="label-text text-xs font-medium text-base-content/70">
                  {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
                </span>
                <textarea
                  :value="requestPreviewDiff.requestLineAfter"
                  class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                  readonly
                ></textarea>
              </label>
            </div>
          </div>

          <div class="space-y-3 rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.headerChanges') }}</div>
              <div class="text-xs text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.unchangedHeaders') }}: {{ requestPreviewDiff.unchangedHeaderCount }}
              </div>
            </div>

            <div
              v-if="!requestPreviewDiff.headerChanges.length"
              class="rounded-lg border border-dashed border-base-300 bg-base-200/50 px-4 py-3 text-sm text-base-content/60"
            >
              {{ $t('trafficAnalysis.intruder.empty.noHeaderChanges') }}
            </div>

            <div
              v-for="change in requestPreviewDiff.headerChanges"
              :key="change.id"
              class="space-y-2 rounded-lg border border-base-300 bg-base-200/30 p-3"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-medium">{{ change.label }}</div>
                <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(change.kind)">
                  {{ formatChangeKind(change.kind) }}
                </div>
              </div>

              <div class="grid gap-3 lg:grid-cols-2">
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
                  </span>
                  <textarea
                    :value="change.before"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
                  </span>
                  <textarea
                    :value="change.after"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
              </div>
            </div>
          </div>

          <div class="space-y-3 rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.queryParameters') }}</div>
              <div class="text-xs text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.unchangedParameters') }}: {{ requestPreviewDiff.queryParameterDiff.unchangedCount }}
              </div>
            </div>

            <div
              v-if="!requestPreviewDiff.queryParameterDiff.changes.length"
              class="rounded-lg border border-dashed border-base-300 bg-base-200/50 px-4 py-3 text-sm text-base-content/60"
            >
              {{ $t('trafficAnalysis.intruder.empty.noQueryParameterChanges') }}
            </div>

            <div
              v-for="change in requestPreviewDiff.queryParameterDiff.changes"
              :key="`query-${change.id}`"
              class="space-y-2 rounded-lg border border-base-300 bg-base-200/30 p-3"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-medium">{{ change.label }}</div>
                <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(change.kind)">
                  {{ formatChangeKind(change.kind) }}
                </div>
              </div>

              <div class="grid gap-3 lg:grid-cols-2">
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
                  </span>
                  <textarea
                    :value="change.before"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
                  </span>
                  <textarea
                    :value="change.after"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
              </div>
            </div>
          </div>

          <div class="space-y-3 rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.formParameters') }}</div>
              <div class="text-xs text-base-content/60">
                {{ $t('trafficAnalysis.intruder.labels.unchangedParameters') }}: {{ requestPreviewDiff.formParameterDiff.unchangedCount }}
              </div>
            </div>

            <div
              v-if="!requestPreviewDiff.formParameterDiff.changes.length"
              class="rounded-lg border border-dashed border-base-300 bg-base-200/50 px-4 py-3 text-sm text-base-content/60"
            >
              {{ $t('trafficAnalysis.intruder.empty.noFormParameterChanges') }}
            </div>

            <div
              v-for="change in requestPreviewDiff.formParameterDiff.changes"
              :key="`form-${change.id}`"
              class="space-y-2 rounded-lg border border-base-300 bg-base-200/30 p-3"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-medium">{{ change.label }}</div>
                <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(change.kind)">
                  {{ formatChangeKind(change.kind) }}
                </div>
              </div>

              <div class="grid gap-3 lg:grid-cols-2">
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
                  </span>
                  <textarea
                    :value="change.before"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
                <label class="form-control gap-2">
                  <span class="label-text text-xs font-medium text-base-content/70">
                    {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
                  </span>
                  <textarea
                    :value="change.after"
                    class="h-20 w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
                    readonly
                  ></textarea>
                </label>
              </div>
            </div>
          </div>

          <div class="space-y-3 rounded-lg border border-base-300 bg-base-100 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ $t('trafficAnalysis.intruder.labels.body') }}</div>
              <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(requestPreviewDiff.bodyChanged ? 'changed' : 'same')">
                {{ formatDiffStatus(requestPreviewDiff.bodyChanged) }}
              </div>
            </div>

            <div v-if="!requestPreviewDiff.bodyChanged" class="rounded-lg border border-dashed border-base-300 bg-base-200/50 px-4 py-3 text-sm text-base-content/60">
              {{ $t('trafficAnalysis.intruder.empty.noBodyChanges') }}
            </div>

            <div v-else class="grid gap-3 lg:grid-cols-2">
              <label class="form-control gap-2">
                <span class="label-text text-xs font-medium text-base-content/70">
                  {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
                </span>
                <textarea
                  :value="requestPreviewDiff.bodyBefore"
                  class="h-28 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                  readonly
                ></textarea>
              </label>
              <label class="form-control gap-2">
                <span class="label-text text-xs font-medium text-base-content/70">
                  {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
                </span>
                <textarea
                  :value="requestPreviewDiff.bodyAfter"
                  class="h-28 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                  readonly
                ></textarea>
              </label>
            </div>
          </div>
        </div>

        <div class="grid gap-4 lg:grid-cols-2">
          <label class="form-control gap-2">
            <span class="label-text text-xs font-medium text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.requestProcessingOriginal') }}
            </span>
            <textarea
              :value="requestProcessingPreviewOriginal || ''"
              class="h-[18rem] w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
              readonly
            ></textarea>
          </label>
          <label class="form-control gap-2">
            <span class="label-text text-xs font-medium text-base-content/70">
              {{ $t('trafficAnalysis.intruder.labels.requestProcessingFinal') }}
            </span>
            <textarea
              :value="requestProcessingPreviewFinal || ''"
              class="h-[18rem] w-full resize-none rounded-lg border border-base-300 bg-base-100 p-3 font-mono text-xs leading-6 outline-none"
              readonly
            ></textarea>
          </label>
        </div>

        <div class="space-y-3">
          <div class="text-xs font-medium uppercase tracking-wide text-base-content/60">
            {{ $t('trafficAnalysis.intruder.labels.requestProcessingTrace') }}
          </div>

          <div
            v-if="!requestProcessingPreviewTraces.length"
            class="rounded-lg border border-dashed border-base-300 bg-base-200/50 px-4 py-3 text-sm text-base-content/60"
          >
            {{ $t('trafficAnalysis.intruder.empty.noRequestProcessorTrace') }}
          </div>

          <div
            v-for="(trace, index) in requestProcessingPreviewTraces"
            :key="`${trace.pluginId}-${index}`"
            class="space-y-2 rounded-lg border border-base-300 bg-base-100 p-3"
          >
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ trace.pluginId }}</div>
              <div class="badge badge-outline badge-sm">
                {{ $t('trafficAnalysis.intruder.labels.processorOutput') }}
              </div>
            </div>

            <div class="space-y-3 rounded-lg border border-base-300 bg-base-200/20 p-3">
              <div class="flex items-center justify-between gap-3">
                <div class="text-xs font-medium uppercase tracking-wide text-base-content/60">
                  {{ $t('trafficAnalysis.intruder.labels.processorDiff') }}
                </div>
                <div class="badge badge-outline badge-sm" :class="getDiffBadgeClass(hasTraceDiffChanges(getTraceDiff(index)) ? 'changed' : 'same')">
                  {{ formatDiffStatus(hasTraceDiffChanges(getTraceDiff(index))) }}
                </div>
              </div>

              <div
                v-if="!hasTraceDiffChanges(getTraceDiff(index))"
                class="rounded-lg border border-dashed border-base-300 bg-base-100/70 px-4 py-3 text-sm text-base-content/60"
              >
                {{ $t('trafficAnalysis.intruder.empty.noProcessorDiffChanges') }}
              </div>

              <div v-else class="space-y-3">
                <div class="grid gap-3 sm:grid-cols-4">
                  <div class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2">
                    <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.labels.headerChanges') }}
                    </div>
                    <div class="mt-1 text-sm font-medium">
                      {{ getTraceDiff(index)?.headerChanges.length ?? 0 }}
                    </div>
                  </div>
                  <div class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2">
                    <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.labels.queryParameters') }}
                    </div>
                    <div class="mt-1 text-sm font-medium">
                      {{ getTraceDiff(index)?.queryParameterDiff.changes.length ?? 0 }}
                    </div>
                  </div>
                  <div class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2">
                    <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.labels.formParameters') }}
                    </div>
                    <div class="mt-1 text-sm font-medium">
                      {{ getTraceDiff(index)?.formParameterDiff.changes.length ?? 0 }}
                    </div>
                  </div>
                  <div class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2">
                    <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                      {{ $t('trafficAnalysis.intruder.labels.body') }}
                    </div>
                    <div class="mt-1 text-sm font-medium" :class="getTraceDiff(index)?.bodyChanged ? 'text-warning' : 'text-base-content/70'">
                      {{ formatDiffStatus(Boolean(getTraceDiff(index)?.bodyChanged)) }}
                    </div>
                  </div>
                </div>

                <div v-if="getTraceDiff(index)?.queryParameterDiff.changes.length" class="space-y-2">
                  <div class="text-xs font-medium uppercase tracking-wide text-base-content/60">
                    {{ $t('trafficAnalysis.intruder.labels.queryParameters') }}
                  </div>
                  <div class="flex flex-wrap gap-2">
                    <div
                      v-for="change in getTraceDiff(index)?.queryParameterDiff.changes ?? []"
                      :key="`trace-query-${index}-${change.id}`"
                      class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2 text-xs"
                    >
                      <div class="font-medium">{{ change.label }}</div>
                      <div class="mt-1 font-mono text-[11px] text-base-content/70">
                        {{ displayChangeValue(change.before) }} -> {{ displayChangeValue(change.after) }}
                      </div>
                    </div>
                  </div>
                </div>

                <div v-if="getTraceDiff(index)?.formParameterDiff.changes.length" class="space-y-2">
                  <div class="text-xs font-medium uppercase tracking-wide text-base-content/60">
                    {{ $t('trafficAnalysis.intruder.labels.formParameters') }}
                  </div>
                  <div class="flex flex-wrap gap-2">
                    <div
                      v-for="change in getTraceDiff(index)?.formParameterDiff.changes ?? []"
                      :key="`trace-form-${index}-${change.id}`"
                      class="rounded-lg border border-base-300 bg-base-100/70 px-3 py-2 text-xs"
                    >
                      <div class="font-medium">{{ change.label }}</div>
                      <div class="mt-1 font-mono text-[11px] text-base-content/70">
                        {{ displayChangeValue(change.before) }} -> {{ displayChangeValue(change.after) }}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="extractTraceFields(trace).length" class="grid gap-3 sm:grid-cols-2">
              <div
                v-for="field in extractTraceFields(trace)"
                :key="`${trace.pluginId}-${field.key}`"
                class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-2"
              >
                <div class="text-[11px] font-medium uppercase tracking-wide text-base-content/60">
                  {{ field.label }}
                </div>
                <div class="mt-1 break-all font-mono text-xs leading-6 text-base-content">
                  {{ field.value }}
                </div>
              </div>
            </div>

            <label class="form-control gap-2">
              <span class="label-text text-xs font-medium text-base-content/70">
                {{ $t('trafficAnalysis.intruder.labels.requestAfterProcessor') }}
              </span>
              <textarea
                :value="trace.requestText"
                class="h-32 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                readonly
              ></textarea>
            </label>

            <label v-if="shouldShowTraceOutput(trace)" class="form-control gap-2">
              <span class="label-text text-xs font-medium text-base-content/70">
                {{ $t('trafficAnalysis.intruder.labels.rawOutput') }}
              </span>
              <textarea
                :value="formatTraceOutput(trace)"
                class="h-28 w-full resize-none rounded-lg border border-base-300 bg-base-200/40 p-3 font-mono text-xs leading-6 outline-none"
                readonly
              ></textarea>
            </label>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button
          class="btn btn-ghost btn-sm"
          type="button"
          :disabled="requestProcessingPreviewLoading || !hasRequestPreviewContent"
          @click="copyRequestPreviewReport"
        >
          {{ $t('trafficAnalysis.intruder.actions.copyDebugReport') }}
        </button>
        <button
          class="btn btn-ghost btn-sm"
          type="button"
          :disabled="requestProcessingPreviewLoading || !hasRequestPreviewContent"
          @click="exportRequestPreviewReport"
        >
          {{ $t('trafficAnalysis.intruder.actions.exportDebugReport') }}
        </button>
        <button class="btn btn-outline btn-sm" type="button" @click="requestPreviewDialogOpen = false">
          {{ $t('trafficAnalysis.intruder.actions.closePreview') }}
        </button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { dialog } from '@/composables/useDialog'
import IntruderAttackResultsSettingsPanel from './IntruderAttackResultsSettingsPanel.vue'
import IntruderAutoPausePanel from './IntruderAutoPausePanel.vue'
import IntruderErrorHandlingPanel from './IntruderErrorHandlingPanel.vue'
import IntruderGrepExtractPanel from './IntruderGrepExtractPanel.vue'
import IntruderGrepMatchPanel from './IntruderGrepMatchPanel.vue'
import IntruderGrepPayloadPanel from './IntruderGrepPayloadPanel.vue'
import IntruderPayloadProcessingPanel from './IntruderPayloadProcessingPanel.vue'
import IntruderPluginProcessorPanel from './IntruderPluginProcessorPanel.vue'
import IntruderPluginConfigDialog from './IntruderPluginConfigDialog.vue'
import IntruderRedirectHandlingPanel from './IntruderRedirectHandlingPanel.vue'
import IntruderRequestHeadersPanel from './IntruderRequestHeadersPanel.vue'
import IntruderResourcePoolPanel from './IntruderResourcePoolPanel.vue'
import { expandPayloadSet, parsePayloadLines } from './payloads'
import {
  generateIntruderPluginPayloads,
  getIntruderPluginInputSchema,
  listIntruderPayloadGeneratorPlugins,
  type IntruderRequestProcessorTrace,
  type IntruderPluginSummary,
} from './plugins'
import {
  buildRequestPreviewDiff,
  buildRequestPreviewTraceDiffs,
  type RequestPreviewChangeKind,
  type RequestPreviewDiff,
} from './requestPreview'
import { buildRequestPreviewDebugReport } from './requestPreviewReport'
import { extractIntruderPayloadSourceSummaryEntries } from './intruderPayloadSourceSummary'
import { getIntruderDictionaryTypeTranslationKey } from './intruderDictionaries'
import type {
  IntruderAttackOptions,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPluginProcessorBinding,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderPosition,
  IntruderResourcePool,
  IntruderTarget,
} from './types'

const { t } = useI18n()

const props = defineProps<{
  activeTab: 'payloads' | 'resourcePool' | 'settings'
  payloadSets: IntruderPayloadSet[]
  payloadProcessingRules: IntruderPayloadProcessingRule[]
  payloadProcessorPlugins: IntruderPluginProcessorBinding[]
  requestProcessorPlugins: IntruderPluginProcessorBinding[]
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  grepPayloadSettings: IntruderGrepPayloadSettings
  resourcePoolPresets: IntruderResourcePool[]
  selectedResourcePoolId: string
  attackOptions: IntruderAttackOptions
  estimatedRequests: number
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  maxRequests: number
  requestProcessingPreviewLoading: boolean
  requestProcessingPreviewOriginal: string
  requestProcessingPreviewFinal: string
  requestProcessingPreviewPayloadSummary: string
  requestProcessingPreviewTraces: IntruderRequestProcessorTrace[]
  requestProcessingPreviewError: string
}>()

const emit = defineEmits<{
  (e: 'update:activeTab', value: 'payloads' | 'resourcePool' | 'settings'): void
  (e: 'update:attackOptions', value: IntruderAttackOptions): void
  (e: 'updatePayloadSet', id: string, patch: Partial<IntruderPayloadSet>): void
  (e: 'update:payloadProcessingRules', value: IntruderPayloadProcessingRule[]): void
  (e: 'update:payloadProcessorPlugins', value: IntruderPluginProcessorBinding[]): void
  (e: 'update:requestProcessorPlugins', value: IntruderPluginProcessorBinding[]): void
  (e: 'update:grepMatchRules', value: IntruderGrepMatchRule[]): void
  (e: 'update:grepExtractRules', value: IntruderGrepExtractRule[]): void
  (e: 'update:grepPayloadSettings', value: IntruderGrepPayloadSettings): void
  (e: 'selectResourcePoolPreset', presetId: string): void
  (e: 'upsertResourcePool', value: {
    id?: string
    name: string
    concurrencyEnabled: boolean
    concurrency: number
    delayEnabled: boolean
    delayMs: number
    randomDelayEnabled: boolean
    randomDelayMs: number
    delayIncrementEnabled: boolean
    delayIncrementMs: number
    autoThrottleEnabled: boolean
    autoThrottleStatusCodes: number[]
  }): void
  (e: 'deleteResourcePool', resourcePoolId: string): void
  (e: 'previewRequestProcessing'): void
}>()

const requestPreviewPayloadSourceEntries = computed(() =>
  extractIntruderPayloadSourceSummaryEntries(props.payloadSets),
)
const requestPreviewPayloadSourceLines = computed(() =>
  requestPreviewPayloadSourceEntries.value.map((entry) => {
    const sourceLabels = entry.sources.map((source) => {
      if (source.type === 'default_dictionary') {
        return t('trafficAnalysis.intruder.labels.defaultDictionaryLabel', {
          type: t(`dictionary.types.${getIntruderDictionaryTypeTranslationKey(source.dictType || '')}`, source.dictType || ''),
        })
      }

      return source.dictionaryName
        ? `${source.dictionaryName} (${source.dictionaryId || ''})`
        : source.dictionaryId || ''
    })
    return `${entry.payloadSetName}: ${sourceLabels.join(', ')}`
  }),
)

const selectedPayloadSetId = ref(props.payloadSets[0]?.id ?? '')
const availablePayloadPlugins = ref<IntruderPluginSummary[]>([])
const pluginPreviewLoading = ref(false)
const pluginConfigDialogOpen = ref(false)
const requestPreviewDialogOpen = ref(false)
const pluginSchemaCache = ref<Record<string, Record<string, any> | null>>({})
let pluginChangedUnlisten: UnlistenFn | null = null

watch(
  () => props.payloadSets,
  (payloadSets) => {
    if (!payloadSets.some((payloadSet) => payloadSet.id === selectedPayloadSetId.value)) {
      selectedPayloadSetId.value = payloadSets[0]?.id ?? ''
    }
  },
  { deep: true },
)

const activePayloadSet = computed(() => props.payloadSets.find((payloadSet) => payloadSet.id === selectedPayloadSetId.value) ?? props.payloadSets[0] ?? null)
const activePayloadCount = computed(() => (activePayloadSet.value ? expandPayloadSet(activePayloadSet.value).length : 0))
const selectedPayloadPlugin = computed(() => availablePayloadPlugins.value.find((plugin) => plugin.id === activePayloadSet.value?.pluginId) ?? null)
const activePluginSchema = computed(() => {
  const pluginId = activePayloadSet.value?.pluginId
  return pluginId ? pluginSchemaCache.value[pluginId] ?? null : null
})
const pluginConfigDialogTitle = computed(() => selectedPayloadPlugin.value?.name || 'Intruder Plugin')
const hasPluginConfig = computed(() => {
  const raw = activePayloadSet.value?.pluginConfig?.trim()
  return Boolean(raw && raw !== '{}' && raw !== '')
})
const requestPreviewDiff = computed(() => buildRequestPreviewDiff(
  props.requestProcessingPreviewOriginal,
  props.requestProcessingPreviewFinal,
))
const requestProcessingTraceDiffs = computed(() => buildRequestPreviewTraceDiffs(
  props.requestProcessingPreviewOriginal,
  props.requestProcessingPreviewTraces.map((trace) => trace.requestText),
))
const hasRequestPreviewContent = computed(() => Boolean(
  props.requestProcessingPreviewOriginal
  || props.requestProcessingPreviewFinal
  || props.requestProcessingPreviewTraces.length,
))

async function refreshAvailablePayloadPlugins() {
  try {
    availablePayloadPlugins.value = await listIntruderPayloadGeneratorPlugins()
  } catch (error) {
    console.error('Failed to load Intruder payload plugins', error)
  }
}

onMounted(async () => {
  await refreshAvailablePayloadPlugins()
  pluginChangedUnlisten = await listen('plugin:changed', async () => {
    await refreshAvailablePayloadPlugins()
  })
})

onUnmounted(() => {
  if (pluginChangedUnlisten) {
    pluginChangedUnlisten()
    pluginChangedUnlisten = null
  }
})

function updateOption<K extends keyof IntruderAttackOptions>(key: K, value: IntruderAttackOptions[K]) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    [key]: value,
  })
}

function updateAttackOptions(patch: Partial<IntruderAttackOptions>) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    ...patch,
  })
}

function updateAutoPauseExpressions(expressions: string[]) {
  emit('update:attackOptions', {
    ...props.attackOptions,
    autoPauseExpressions: expressions,
    autoPauseExpression: expressions[0] ?? '',
  })
}

function updateActivePayloadSet(patch: Partial<IntruderPayloadSet>) {
  if (!activePayloadSet.value) return
  emit('updatePayloadSet', activePayloadSet.value.id, patch)
}

function handlePayloadTypeChange(payloadType: IntruderPayloadSet['payloadType']) {
  const patch: Partial<IntruderPayloadSet> = { payloadType }
  if (payloadType === 'extensionGenerated') {
    patch.filePath = ''
  }
  updateActivePayloadSet(patch)
}

function handlePluginChange(pluginId: string) {
  updateActivePayloadSet({
    pluginId,
    pluginPresetName: '',
    pluginConfig: '{}',
    payloadsText: '',
  })

  if (pluginId) {
    void ensurePluginSchema(pluginId)
  }
}

async function ensurePluginSchema(pluginId: string) {
  if (pluginSchemaCache.value[pluginId] !== undefined) return
  try {
    pluginSchemaCache.value[pluginId] = await getIntruderPluginInputSchema(pluginId)
  } catch (error) {
    console.error(`Failed to load schema for plugin ${pluginId}`, error)
    pluginSchemaCache.value[pluginId] = null
  }
}

function openPluginConfigDialog() {
  if (!activePayloadSet.value?.pluginId) return
  void ensurePluginSchema(activePayloadSet.value.pluginId)
  pluginConfigDialogOpen.value = true
}

async function generatePluginPayloadPreview() {
  if (!activePayloadSet.value) return

  pluginPreviewLoading.value = true
  try {
    const payloads = await generateIntruderPluginPayloads({
      requestText: props.requestText,
      target: props.target,
      positions: props.positions,
      payloadSet: activePayloadSet.value,
      maxRequests: props.maxRequests,
    })

    updateActivePayloadSet({
      payloadsText: payloads.join('\n'),
    })
  } catch (error) {
    console.error('Failed to generate Intruder payload preview', error)
    dialog.toast.error(error instanceof Error ? error.message : 'Failed to generate payload preview')
  } finally {
    pluginPreviewLoading.value = false
  }
}

function openRequestPreviewDialog() {
  requestPreviewDialogOpen.value = true
  emit('previewRequestProcessing')
}

function formatDiffStatus(changed: boolean): string {
  return changed
    ? t('trafficAnalysis.intruder.labels.changed')
    : t('trafficAnalysis.intruder.labels.unchanged')
}

function formatChangeKind(kind: RequestPreviewChangeKind | 'same'): string {
  if (kind === 'same') {
    return t('trafficAnalysis.intruder.labels.unchanged')
  }

  return t(`trafficAnalysis.intruder.labels.${kind}`)
}

function getDiffBadgeClass(kind: RequestPreviewChangeKind | 'same'): string {
  switch (kind) {
    case 'added':
      return 'badge-success'
    case 'removed':
      return 'badge-error'
    case 'changed':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

function getTraceDiff(index: number): RequestPreviewDiff | null {
  return requestProcessingTraceDiffs.value[index]?.diff ?? null
}

function hasTraceDiffChanges(diff: RequestPreviewDiff | null): boolean {
  if (!diff) return false

  return diff.requestLineChanged
    || diff.headerChanges.length > 0
    || diff.queryParameterDiff.changes.length > 0
    || diff.formParameterDiff.changes.length > 0
    || diff.bodyChanged
}

function displayChangeValue(value: string): string {
  return value || '(empty)'
}

function buildCurrentRequestPreviewReport(): string {
  return buildRequestPreviewDebugReport({
    payloadSummary: props.requestProcessingPreviewPayloadSummary,
    payloadSources: requestPreviewPayloadSourceEntries.value,
    originalRequest: props.requestProcessingPreviewOriginal,
    finalRequest: props.requestProcessingPreviewFinal,
    traces: props.requestProcessingPreviewTraces,
  })
}

async function copyRequestPreviewReport() {
  try {
    await navigator.clipboard.writeText(buildCurrentRequestPreviewReport())
    dialog.toast.success(t('trafficAnalysis.intruder.messages.debugReportCopied'))
  } catch (error) {
    console.error('Failed to copy Intruder debug report', error)
    dialog.toast.error(t('trafficAnalysis.intruder.messages.debugReportCopyFailed'))
  }
}

async function exportRequestPreviewReport() {
  try {
    const selected = await save({
      defaultPath: 'intruder-request-preview-report.txt',
      filters: [{ name: 'Text', extensions: ['txt', 'log'] }],
    })
    if (!selected || Array.isArray(selected)) return

    await writeTextFile(selected, buildCurrentRequestPreviewReport())
    dialog.toast.success(t('trafficAnalysis.intruder.messages.debugReportExported'))
  } catch (error) {
    console.error('Failed to export Intruder debug report', error)
    dialog.toast.error(t('trafficAnalysis.intruder.messages.debugReportExportFailed'))
  }
}

function formatTraceOutput(trace: IntruderRequestProcessorTrace): string {
  if (trace.output == null) {
    return trace.requestText
  }

  if (typeof trace.output === 'string') {
    return trace.output
  }

  try {
    return JSON.stringify(trace.output, null, 2)
  } catch {
    return String(trace.output)
  }
}

function shouldShowTraceOutput(trace: IntruderRequestProcessorTrace): boolean {
  return trace.output != null
}

function extractTraceFields(trace: IntruderRequestProcessorTrace): Array<{
  key: string
  label: string
  value: string
}> {
  if (!trace.output || typeof trace.output !== 'object' || Array.isArray(trace.output)) {
    return []
  }

  const record = trace.output as Record<string, unknown>
  const preferredFields: Array<{ key: string; label: string }> = [
    { key: 'canonical', label: 'Canonical' },
    { key: 'signature', label: 'Signature' },
    { key: 'timestamp', label: 'Timestamp' },
    { key: 'ts', label: 'Timestamp' },
    { key: 'nonce', label: 'Nonce' },
    { key: 'algorithm', label: 'Algorithm' },
    { key: 'location', label: 'Location' },
  ]

  const extracted: Array<{ key: string; label: string; value: string }> = []
  const seenKeys = new Set<string>()

  for (const field of preferredFields) {
    const value = record[field.key]
    if (value == null || typeof value === 'object') continue
    extracted.push({
      key: field.key,
      label: field.label,
      value: String(value),
    })
    seenKeys.add(field.key)
  }

  for (const [key, value] of Object.entries(record)) {
    if (seenKeys.has(key)) continue
    if (value == null || typeof value === 'object') continue
    extracted.push({
      key,
      label: formatTraceFieldLabel(key),
      value: String(value),
    })
    if (extracted.length >= 6) {
      break
    }
  }

  return extracted
}

function formatTraceFieldLabel(key: string): string {
  return key
    .replace(/[_-]+/g, ' ')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, (char) => char.toUpperCase())
}

async function pastePayloads() {
  try {
    const text = await navigator.clipboard.readText()
    updateActivePayloadSet({ payloadsText: text })
  } catch {
    dialog.toast.error('Clipboard read failed')
  }
}

async function loadPayloadFile() {
  if (!activePayloadSet.value) return

  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Text', extensions: ['txt', 'lst', 'csv', 'log'] }],
    })
    if (!selected || Array.isArray(selected)) return

    const content = await readTextFile(selected)
    updateActivePayloadSet({
      filePath: selected,
      payloadsText: content,
    })
  } catch (error) {
    console.error('Failed to load payload file', error)
    dialog.toast.error('Failed to load payload file')
  }
}

function clearPayloads() {
  updateActivePayloadSet({ payloadsText: '' })
}

function deduplicatePayloads() {
  if (!activePayloadSet.value || activePayloadSet.value.payloadType !== 'simpleList') return
  const deduped = Array.from(new Set(parsePayloadLines(activePayloadSet.value.payloadsText)))
  updateActivePayloadSet({ payloadsText: deduped.join('\n') })
}

watch(
  () => activePayloadSet.value?.pluginId,
  (pluginId) => {
    if (pluginId) {
      void ensurePluginSchema(pluginId)
    }
  },
  { immediate: true },
)
</script>

<style scoped>
.side-label {
  display: inline-block;
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  letter-spacing: 0.06em;
}
</style>
