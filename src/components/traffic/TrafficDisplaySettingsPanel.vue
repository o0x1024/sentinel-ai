<template>
  <div class="grid gap-4 lg:grid-cols-2">
    <section class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h2 class="card-title mb-2 text-base">
              <i class="fas fa-font mr-2"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.httpMessageDisplayTitle') }}
            </h2>
            <p class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.proxyConfiguration.httpMessageDisplayDesc') }}
            </p>
          </div>
          <span class="badge badge-info badge-outline">
            {{ $t('trafficAnalysis.proxyConfiguration.userSettingBadge') }}
          </span>
        </div>

        <div class="mt-5 space-y-4">
          <div class="grid gap-3 md:grid-cols-[5rem_minmax(0,1fr)] md:items-start">
            <span class="pt-2 text-sm font-medium">{{ $t('trafficAnalysis.proxyConfiguration.font') }}</span>
            <div class="space-y-3">
              <div class="rounded-lg border border-base-300 bg-base-200/60 px-4 py-3 font-mono text-sm">
                {{ selectedFontSummary }}
              </div>
              <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_7rem]">
                <select v-model="settings.fontFamily" class="select select-bordered w-full">
                  <option v-for="option in fontOptions" :key="option.value" :value="option.value">
                    {{ option.label }}
                  </option>
                </select>
                <input
                  v-model.number="settings.fontSize"
                  type="number"
                  min="10"
                  max="20"
                  class="input input-bordered w-full"
                />
              </div>
            </div>
          </div>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.highlightRequestSyntax" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.highlightRequestSyntax') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.highlightResponseSyntax" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.highlightResponseSyntax') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.prettyPrintByDefault" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.prettyPrintByDefault') }}</span>
          </label>
        </div>
      </div>
    </section>

    <section class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h2 class="card-title mb-2 text-base">
              <i class="fas fa-share-square mr-2"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.sendTargetsTitle') }}
            </h2>
            <p class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.proxyConfiguration.sendTargetsDesc') }}
            </p>
          </div>
          <span class="badge badge-info badge-outline">
            {{ $t('trafficAnalysis.proxyConfiguration.userSettingBadge') }}
          </span>
        </div>

        <div class="mt-5 space-y-3">
          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.showSendToRepeater" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.showSendToRepeater') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.showSendToComparer" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.showSendToComparer') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.showSendToIntruder" type="checkbox" class="checkbox checkbox-primary checkbox-sm" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.showSendToIntruder') }}</span>
          </label>
        </div>
      </div>
    </section>

    <section class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h2 class="card-title mb-2 text-base">
              <i class="fas fa-language mr-2"></i>
              {{ $t('trafficAnalysis.proxyConfiguration.characterSetsTitle') }}
            </h2>
            <p class="text-sm text-base-content/70">
              {{ $t('trafficAnalysis.proxyConfiguration.characterSetsDesc') }}
            </p>
          </div>
          <span class="badge badge-info badge-outline">
            {{ $t('trafficAnalysis.proxyConfiguration.userSettingBadge') }}
          </span>
        </div>

        <div class="mt-5 space-y-3">
          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.charsetMode" type="radio" class="radio radio-primary radio-sm" value="auto" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.recognizeCharsetAutomatically') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.charsetMode" type="radio" class="radio radio-primary radio-sm" value="platformDefault" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.usePlatformDefaultCharset') }}</span>
          </label>

          <label class="label cursor-pointer justify-start gap-3 py-1">
            <input v-model="settings.charsetMode" type="radio" class="radio radio-primary radio-sm" value="rawBytes" />
            <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.displayAsRawBytes') }}</span>
          </label>

          <div class="grid gap-3 md:grid-cols-[auto_minmax(0,1fr)] md:items-center">
            <label class="label cursor-pointer justify-start gap-3 py-1">
              <input v-model="settings.charsetMode" type="radio" class="radio radio-primary radio-sm" value="specific" />
              <span class="label-text">{{ $t('trafficAnalysis.proxyConfiguration.useSpecificCharset') }}</span>
            </label>
            <select
              v-model="settings.specificCharset"
              class="select select-bordered w-full"
              :disabled="settings.charsetMode !== 'specific'"
            >
              <option v-for="option in charsetOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useTrafficDisplaySettings } from './trafficDisplaySettings'

const { settings, fontOptions, charsetOptions } = useTrafficDisplaySettings()

const selectedFontSummary = computed(() => {
  const font = fontOptions.find((option) => option.value === settings.value.fontFamily)
  return `${font?.label || 'Monospaced'} ${settings.value.fontSize}px`
})
</script>
