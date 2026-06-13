<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AppDialog from '@/components/AppDialog.vue'
import { useAssistantProfiles } from '@/components/Agent/assistantProfiles'
import {
  activateMission,
  createMission,
  updateMissionFields,
  type CreateMissionRequest,
  type Mission,
} from '@/api/missions'

const props = defineProps<{
  open: boolean
  mission?: Mission | null
}>()

const emit = defineEmits<{
  close: []
  saved: [mission: Mission]
}>()

const { t } = useI18n()
const { profileOptions, loadAssistantProfiles } = useAssistantProfiles()

const dialogRef = ref<{ showModal: () => void; close: () => void } | null>(null)
const saving = ref(false)
const error = ref('')

const form = ref({
  title: 'Bot 运行观察者',
  objective: '监控 Bot 全局执行状况并生成 AI 汇报',
  cronExpr: '0 * * * *',
  timezone: 'Asia/Shanghai',
  assistantProfileId: '',
  deliveryTarget: 'app_notification' as 'app_notification' | 'bot' | 'webhook',
  webhookUrl: '',
  weixinAccountId: '',
  weixinPeerType: 'dm',
  weixinPeerId: '',
  failureThreshold: 3,
  failureWindowMinutes: 30,
  failureCooldownMinutes: 60,
})

const isEditing = computed(() => Boolean(props.mission?.id))

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      dialogRef.value?.close()
      return
    }
    await loadAssistantProfiles()
    resetForm()
    dialogRef.value?.showModal()
  },
)

function resetForm() {
  error.value = ''
  if (props.mission) {
    form.value.title = props.mission.title
    form.value.objective = props.mission.objective
    form.value.assistantProfileId = props.mission.assistant_profile_id || ''
    try {
      const trigger = props.mission.trigger_json ? JSON.parse(props.mission.trigger_json) : null
      form.value.cronExpr = trigger?.cron_expr || trigger?.expr || '0 * * * *'
      form.value.timezone = trigger?.timezone || 'Asia/Shanghai'
    } catch {
      /* keep defaults */
    }
    try {
      const delivery = props.mission.delivery_policy_json
        ? JSON.parse(props.mission.delivery_policy_json)
        : null
      form.value.deliveryTarget = delivery?.primary?.kind || 'app_notification'
      if (delivery?.primary?.kind === 'webhook') {
        form.value.webhookUrl = delivery.primary.refData?.url || ''
      }
      if (delivery?.primary?.kind === 'bot') {
        form.value.weixinAccountId = delivery.primary.refData?.account_id || ''
        form.value.weixinPeerType = delivery.primary.refData?.peer_type || 'dm'
        form.value.weixinPeerId = delivery.primary.refData?.peer_id || ''
      }
    } catch {
      /* keep defaults */
    }
    return
  }

  form.value = {
    title: 'Bot 运行观察者',
    objective: '监控 Bot 全局执行状况并生成 AI 汇报',
    cronExpr: '0 * * * *',
    timezone: 'Asia/Shanghai',
    assistantProfileId: '',
    deliveryTarget: 'app_notification',
    webhookUrl: '',
    weixinAccountId: '',
    weixinPeerType: 'dm',
    weixinPeerId: '',
    failureThreshold: 3,
    failureWindowMinutes: 30,
    failureCooldownMinutes: 60,
  }
}

function buildDeliveryPrimary() {
  if (form.value.deliveryTarget === 'webhook') {
    return { kind: 'webhook', refData: { url: form.value.webhookUrl.trim() } }
  }
  if (form.value.deliveryTarget === 'bot') {
    return {
      kind: 'bot',
      refData: {
        transport: 'weixin',
        account_id: form.value.weixinAccountId.trim(),
        peer_type: form.value.weixinPeerType.trim(),
        peer_id: form.value.weixinPeerId.trim(),
      },
    }
  }
  return { kind: 'app_notification', refData: {} }
}

function buildPayloadJson() {
  const triggerJson = JSON.stringify({
    kind: 'cron',
    cron_expr: form.value.cronExpr.trim(),
    timezone: form.value.timezone.trim() || 'UTC',
  })
  const deliveryPolicyJson = JSON.stringify({
    onSuccess: 'summary',
    onChange: 'none',
    onFailure: 'immediate',
    primary: buildDeliveryPrimary(),
  })
  const missionSpecJson = JSON.stringify({
    kind: 'observer_mission',
    event_triggers: {
      on_failure: {
        enabled: true,
        threshold: form.value.failureThreshold,
        window_minutes: form.value.failureWindowMinutes,
        cooldown_minutes: form.value.failureCooldownMinutes,
      },
      on_account_offline: {
        enabled: true,
        offline_minutes: 30,
        cooldown_minutes: 120,
      },
    },
  })
  const budgetJson = JSON.stringify({ max_runs_per_day: 48, timeout_seconds: 300 })
  return { triggerJson, deliveryPolicyJson, missionSpecJson, budgetJson }
}

async function handleSave() {
  if (!form.value.title.trim() || !form.value.objective.trim()) {
    error.value = t('botConsole.observer.configRequired')
    return
  }

  saving.value = true
  error.value = ''
  try {
    const payload = buildPayloadJson()
    let mission: Mission
    if (props.mission) {
      mission = await updateMissionFields({
        id: props.mission.id,
        title: form.value.title.trim(),
        objective: form.value.objective.trim(),
        triggerJson: payload.triggerJson,
        missionSpecJson: payload.missionSpecJson,
        deliveryPolicyJson: payload.deliveryPolicyJson,
        assistantProfileId: form.value.assistantProfileId || null,
        budgetJson: payload.budgetJson,
      })
    } else {
      const request: CreateMissionRequest = {
        title: form.value.title.trim(),
        objective: form.value.objective.trim(),
        ownerKind: 'observer',
        ownerRef: 'global',
        triggerJson: payload.triggerJson,
        missionSpecJson: payload.missionSpecJson,
        deliveryPolicyJson: payload.deliveryPolicyJson,
        assistantProfileId: form.value.assistantProfileId || undefined,
        budgetJson: payload.budgetJson,
        missedRunPolicy: 'skip',
      }
      mission = await createMission(request)
      mission = await activateMission(mission.id)
    }
    emit('saved', mission)
    dialogRef.value?.close()
    emit('close')
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}

function handleClose() {
  dialogRef.value?.close()
  emit('close')
}
</script>

<template>
  <AppDialog ref="dialogRef" class="modal" @click.self="handleClose">
    <div class="modal-box max-w-2xl">
      <h3 class="text-lg font-semibold">
        {{ isEditing ? t('botConsole.observer.editConfig') : t('botConsole.observer.create') }}
      </h3>
      <p class="mt-1 text-sm text-base-content/70">{{ t('botConsole.observer.configDescription') }}</p>

      <div v-if="error" class="alert alert-error mt-4">
        <span>{{ error }}</span>
      </div>

      <div class="mt-4 space-y-4">
        <label class="form-control">
          <span class="label-text">{{ t('botConsole.observer.fields.title') }}</span>
          <input v-model="form.title" class="input input-bordered input-sm" />
        </label>
        <label class="form-control">
          <span class="label-text">{{ t('botConsole.observer.fields.objective') }}</span>
          <textarea v-model="form.objective" class="textarea textarea-bordered textarea-sm" rows="2" />
        </label>
        <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
          <label class="form-control">
            <span class="label-text">{{ t('botConsole.observer.fields.cron') }}</span>
            <input v-model="form.cronExpr" class="input input-bordered input-sm font-mono" />
          </label>
          <label class="form-control">
            <span class="label-text">{{ t('botConsole.observer.fields.timezone') }}</span>
            <input v-model="form.timezone" class="input input-bordered input-sm" />
          </label>
        </div>
        <label class="form-control">
          <span class="label-text">{{ t('botConsole.observer.fields.assistantProfile') }}</span>
          <select v-model="form.assistantProfileId" class="select select-bordered select-sm">
            <option value="">{{ t('botConsole.observer.fields.defaultProfile') }}</option>
            <option v-for="profile in profileOptions" :key="profile.id" :value="profile.id">
              {{ profile.label }}
            </option>
          </select>
        </label>
        <label class="form-control">
          <span class="label-text">{{ t('botConsole.observer.fields.delivery') }}</span>
          <select v-model="form.deliveryTarget" class="select select-bordered select-sm">
            <option value="app_notification">{{ t('botConsole.observer.deliveryTargets.app') }}</option>
            <option value="bot">{{ t('botConsole.observer.deliveryTargets.bot') }}</option>
            <option value="webhook">{{ t('botConsole.observer.deliveryTargets.webhook') }}</option>
          </select>
        </label>
        <label v-if="form.deliveryTarget === 'webhook'" class="form-control">
          <span class="label-text">Webhook URL</span>
          <input v-model="form.webhookUrl" class="input input-bordered input-sm font-mono" />
        </label>
        <div v-if="form.deliveryTarget === 'bot'" class="grid grid-cols-1 gap-3 md:grid-cols-3">
          <label class="form-control">
            <span class="label-text">{{ t('botConsole.observer.fields.weixinAccount') }}</span>
            <input v-model="form.weixinAccountId" class="input input-bordered input-sm" />
          </label>
          <label class="form-control">
            <span class="label-text">{{ t('botConsole.observer.fields.peerType') }}</span>
            <select v-model="form.weixinPeerType" class="select select-bordered select-sm">
              <option value="dm">{{ t('botConsole.dm') }}</option>
              <option value="group">{{ t('botConsole.group') }}</option>
            </select>
          </label>
          <label class="form-control">
            <span class="label-text">{{ t('botConsole.observer.fields.peerId') }}</span>
            <input v-model="form.weixinPeerId" class="input input-bordered input-sm" />
          </label>
        </div>
        <div class="rounded-lg border border-base-300 p-3">
          <div class="text-sm font-semibold">{{ t('botConsole.observer.fields.eventTriggers') }}</div>
          <div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-3">
            <label class="form-control">
              <span class="label-text">{{ t('botConsole.observer.fields.failureThreshold') }}</span>
              <input v-model.number="form.failureThreshold" type="number" min="1" class="input input-bordered input-sm" />
            </label>
            <label class="form-control">
              <span class="label-text">{{ t('botConsole.observer.fields.failureWindow') }}</span>
              <input v-model.number="form.failureWindowMinutes" type="number" min="1" class="input input-bordered input-sm" />
            </label>
            <label class="form-control">
              <span class="label-text">{{ t('botConsole.observer.fields.failureCooldown') }}</span>
              <input v-model.number="form.failureCooldownMinutes" type="number" min="1" class="input input-bordered input-sm" />
            </label>
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-sm btn-ghost" @click="handleClose">{{ t('botConsole.accounts.close') }}</button>
        <button class="btn btn-sm btn-primary" :disabled="saving" @click="handleSave">
          <span v-if="saving" class="loading loading-spinner loading-xs"></span>
          {{ t('botConsole.observer.save') }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click="handleClose">{{ t('botConsole.accounts.close') }}</button>
    </form>
  </AppDialog>
</template>
