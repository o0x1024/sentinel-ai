<template>
  <section class="rounded-lg border border-base-300 bg-base-100 overflow-hidden">
    <div class="border-b border-base-300 px-4 py-3">
      <div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <div class="text-sm font-semibold">{{ t('botConsole.accounts.title') }}</div>
          <div class="mt-1 text-xs text-base-content/70">{{ t('botConsole.accounts.description') }}</div>
        </div>
        <div class="text-xs text-base-content/60">
          {{ t('botConsole.accounts.currentTransport') }}:
          <span class="font-medium">{{ transportLabel(setupTransport) }}</span>
        </div>
      </div>
    </div>

    <div class="border-b border-base-300 px-4 py-3">
      <div class="tabs tabs-boxed inline-flex bg-base-200/60">
        <button
          v-for="transport in setupTransports"
          :key="transport"
          class="tab gap-2"
          :class="{ 'tab-active': setupTransport === transport }"
          @click="selectSetupTransport(transport)"
        >
          <span>{{ transportLabel(transport) }}</span>
          <span class="badge badge-xs">{{ accountCountForTransport(transport) }}</span>
        </button>
      </div>
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-[18rem_minmax(0,1fr)]">
      <div class="border-b border-base-300 xl:border-b-0 xl:border-r xl:border-base-300">
        <div class="border-b border-base-300 px-4 py-3">
          <button
            class="btn btn-sm w-full justify-between"
            :class="selectedAccountId ? 'btn-ghost' : 'btn-primary'"
            :disabled="loading"
            @click="emitAccountSelection('', setupTransport)"
          >
            <span>{{ t('botConsole.accounts.allAccounts') }}</span>
            <span class="badge badge-sm">{{ filteredAccounts.length }}</span>
          </button>
        </div>

        <div class="max-h-[24rem] overflow-y-auto">
          <div v-if="loading" class="p-4 text-sm text-base-content/60">
            {{ t('botConsole.accounts.loading') }}
          </div>
          <div v-else-if="filteredAccounts.length === 0" class="p-4 text-sm text-base-content/60">
            {{ t('botConsole.accounts.empty') }}
          </div>
          <button
            v-for="account in filteredAccounts"
            :key="account.id"
            class="w-full border-b border-base-200 px-4 py-3 text-left hover:bg-base-200/60"
            :class="selectedAccountId === account.account_id ? 'bg-primary/10' : ''"
            @click="emitAccountSelection(account.account_id, account.transport)"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium">
                  {{ account.display_name || account.account_id }}
                </div>
                <div class="mt-1 truncate text-xs text-base-content/60">
                  {{ account.account_id }}
                </div>
                <div class="mt-2 flex flex-wrap items-center gap-2 text-xs text-base-content/60">
                  <span class="badge badge-outline badge-xs">
                    {{ transportLabel(account.transport) }}
                  </span>
                  <span class="badge badge-xs" :class="statusBadgeClass(account.status)">
                    {{ statusLabel(account.status) }}
                  </span>
                </div>
              </div>
              <div class="text-right text-xs text-base-content/60">
                <div>{{ t('botConsole.accounts.lastSeen') }}</div>
                <div>{{ formatTimestamp(account.last_seen_at) }}</div>
              </div>
            </div>
          </button>
        </div>
      </div>

        <div class="p-4">
        <BotWeixinSetupPanel
          v-if="setupTransport === 'weixin'"
          @updated="handleTransportSetupUpdated"
        />
        <div
          v-else
          class="rounded-lg border border-dashed border-base-300 px-4 py-8 text-sm text-base-content/60"
        >
          {{ t('botConsole.accounts.noSetup', { transport: transportLabel(setupTransport) }) }}
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { BotAccount } from '@/api/botConsole'
import BotWeixinSetupPanel from '@/components/Bot/BotWeixinSetupPanel.vue'
import { DEFAULT_BOT_TRANSPORT, SUPPORTED_BOT_TRANSPORTS } from '@/components/Bot/botTransportCatalog'

const props = defineProps<{
  accounts: BotAccount[]
  selectedTransport: string
  selectedAccountId: string
  loading: boolean
}>()

const emit = defineEmits<{
  accountChange: [payload: { accountId: string; transport: string }]
  updated: [payload: { accountId: string; transport: string }]
}>()

const { t } = useI18n()

const setupTransport = ref(DEFAULT_BOT_TRANSPORT)

const setupTransports = computed(() => {
  const values = new Set<string>(SUPPORTED_BOT_TRANSPORTS)
  for (const account of props.accounts) {
    const transport = String(account.transport || '').trim()
    if (transport) values.add(transport)
  }
  return Array.from(values)
})

const filteredAccounts = computed(() => {
  return props.accounts.filter((account) => account.transport === setupTransport.value)
})

const accountFingerprint = computed(() =>
  props.accounts.map((account) => `${account.transport}:${account.account_id}`).join('|'),
)

watch([
  () => props.selectedTransport,
  () => props.selectedAccountId,
  accountFingerprint,
], () => {
  const preferredTransport = derivePreferredTransport()
  if (props.selectedTransport.trim()) {
    setupTransport.value = preferredTransport
    return
  }
  if (props.selectedAccountId.trim()) {
    setupTransport.value = preferredTransport
    return
  }
  if (!setupTransport.value || !setupTransports.value.includes(setupTransport.value)) {
    setupTransport.value = preferredTransport
  }
}, { immediate: true })

function derivePreferredTransport(): string {
  const selectedTransport = props.selectedTransport.trim()
  if (selectedTransport) return selectedTransport

  const selectedAccount = props.accounts.find((account) => account.account_id === props.selectedAccountId)
  const accountTransport = String(selectedAccount?.transport || '').trim()
  if (accountTransport) return accountTransport

  const discoveredTransport = String(props.accounts[0]?.transport || '').trim()
  return discoveredTransport || DEFAULT_BOT_TRANSPORT
}

function accountCountForTransport(transport: string): number {
  return props.accounts.filter((account) => account.transport === transport).length
}

function selectSetupTransport(transport: string) {
  setupTransport.value = transport
}

function emitAccountSelection(accountId: string, transport: string) {
  emit('accountChange', {
    accountId,
    transport,
  })
}

function normalizeTransport(transport?: string | null): string {
  return String(transport || '').trim().toLowerCase()
}

function transportLabel(transport?: string | null): string {
  const normalized = normalizeTransport(transport)
  if (normalized === 'weixin') return t('botConsole.accounts.transports.weixin')
  if (normalized === 'feishu') return t('botConsole.accounts.transports.feishu')
  if (normalized === 'discord') return t('botConsole.accounts.transports.discord')
  return transport || '—'
}

function formatTimestamp(value?: string | null): string {
  const normalized = String(value || '').trim()
  if (!normalized) return '—'
  const date = new Date(normalized)
  if (Number.isNaN(date.getTime())) return normalized
  return date.toLocaleString()
}

function statusBadgeClass(status?: string | null): string {
  const normalized = String(status || '').trim().toLowerCase()
  if (normalized === 'running') return 'badge-success'
  if (normalized === 'disabled' || normalized === 'stopped') return 'badge-ghost'
  if (normalized === 'failed' || normalized === 'error') return 'badge-error'
  return 'badge-outline'
}

function statusLabel(status?: string | null): string {
  const normalized = String(status || '').trim().toLowerCase()
  if (normalized === 'running') return t('botConsole.status.running')
  if (normalized === 'failed' || normalized === 'error') return t('botConsole.status.failed')
  if (normalized === 'disabled' || normalized === 'stopped') return t('botConsole.status.disabled')
  return status || t('botConsole.status.unknown')
}

function handleTransportSetupUpdated(payload: { accountId: string }) {
  emit('updated', {
    accountId: payload.accountId,
    transport: setupTransport.value,
  })
}
</script>
