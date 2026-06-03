<template>
  <div class="p-8 h-full flex items-center justify-center">
    <div class="max-w-2xl w-full text-center space-y-6">
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
        {{
          t(
            'bugBountyPro.description',
            'Program 管理、资产发现、Finding 运营、Submission 跟踪、监控调度与知识库等功能仅在 Sentinel AI Pro 中提供。Community 版不包含此模块。'
          )
        }}
      </p>

      <ul class="text-left text-sm text-base-content/40 space-y-2 max-w-md mx-auto">
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature1', 'Program 与 Scope 管理') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature2', '资产面发现与拓扑分析') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature3', 'Finding / Submission 全流程') }}</li>
        <li><i class="fas fa-check mr-2 opacity-50"></i>{{ t('bugBountyPro.feature4', '监控任务与 Workflow 编排') }}</li>
      </ul>

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

const { t } = useI18n()
</script>
