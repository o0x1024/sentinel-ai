<template>
  <div class="p-8 h-full flex items-center justify-center overflow-y-auto">
    <div class="max-w-2xl w-full text-center space-y-6 py-4">
      <div class="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-base-200 opacity-60">
        <i class="fas fa-trophy text-4xl text-base-content/40"></i>
      </div>

      <div class="space-y-2">
        <div class="flex items-center justify-center gap-2">
          <h1 class="text-3xl font-bold text-base-content/50">
            {{ t('sidebar.bugBounty', '漏洞赏金') }}
          </h1>
          <span class="badge badge-ghost opacity-60">PRO</span>
        </div>
        <p class="text-base-content/50 text-lg">
          {{ t('bugBountyPro.subtitle', '漏洞赏金工作台') }}
        </p>
      </div>

      <p class="text-base-content/40 leading-relaxed">
        {{ t('bugBountyPro.description') }}
      </p>

      <ul class="text-left text-sm text-base-content/40 space-y-2 max-w-md mx-auto">
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature1') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature2') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature3') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature4') }}</li>
      </ul>

      <div class="rounded-2xl border border-base-300 bg-base-200/40 p-6 space-y-4 max-w-sm mx-auto">
        <p class="text-sm text-base-content/50">
          {{ t('bugBountyPro.contactHint') }}
        </p>
        <img
          :src="wechatQrPath"
          :alt="t('bugBountyPro.wechatQrAlt')"
          class="w-44 h-44 mx-auto rounded-xl border border-base-300 bg-base-100 object-contain"
        />
        <p class="text-xs text-base-content/40">
          {{ t('bugBountyPro.contactNote') }}
        </p>
      </div>

      <div class="flex items-center justify-center gap-3 pt-2">
        <a
          href="https://github.com/o0x1024/sentinel-ai-pro"
          target="_blank"
          rel="noopener noreferrer"
          class="btn btn-primary btn-sm"
        >
          {{ t('bugBountyPro.learnPro', '了解 Pro 版') }}
        </a>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { COMMUNITY_WECHAT_QR_PATH } from '@/views/help-center/helpCenterBugBountyProContent'

const { t } = useI18n()
const wechatQrPath = COMMUNITY_WECHAT_QR_PATH
</script>
