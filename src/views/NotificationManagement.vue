<template>
  <div class="page-content-padded safe-top space-y-6">
    <div class="flex justify-between items-center">
      <h2 class="text-2xl font-bold">{{ t('notifications.title', '通知管理') }}</h2>
    </div>

    <div class="space-y-4">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex items-center justify-between mb-2">
            <button class="btn btn-primary" @click="open_new_rule_modal"><i class="fas fa-plus mr-2"></i>{{ t('notifications.newNotification') }}</button>
            <div class="text-sm text-base-content/60">{{ t('notifications.totalCount', { count: rules.length }) }}</div>
          </div>
          <div class="overflow-x-auto">
            <table class="table w-full">
              <thead>
                <tr>
                  <th>{{ t('notifications.table.notificationType') }}</th>
                  <th>{{ t('notifications.table.notificationStatus') }}</th>
                  <th>{{ t('notifications.table.notificationConfig') }}</th>
                  <th>{{ t('notifications.table.updateTime') }}</th>
                  <th>{{ t('notifications.table.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="rule in paged_rules" :key="rule.id">
                  <td>
                    <div class="font-medium">{{ rule.type_name }}</div>
                    <div class="text-xs text-base-content/60">{{ rule.description }}</div>
                  </td>
                  <td>
                    <input type="checkbox" class="toggle toggle-primary" v-model="rule.enabled" @change="update_rule_status(rule)" />
                  </td>
                  <td>
                    <div class="flex items-center gap-2">
                      <span class="badge badge-ghost">{{ t(`notifications.channels.${rule.channel}`) }}</span>
                      <span class="text-xs">{{ rule.endpoint_name }}</span>
                    </div>
                  </td>
                  <td>{{ rule.updated_at }}</td>
                  <td>
                    <button class="btn btn-link btn-xs" @click="open_edit_rule_modal(rule)">{{ t('notifications.edit') }}</button>
                    <button class="btn btn-link btn-xs text-error" @click="delete_rule(rule)">{{ t('notifications.delete') }}</button>
                  </td>
                </tr>
                <tr v-if="rules.length === 0">
                  <td colspan="5" class="text-center text-base-content/50">{{ t('notifications.messages.noRules') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="flex items-center justify-end gap-2 mt-2">
            <span class="text-sm">{{ t('notifications.perPage') }}</span>
            <select v-model.number="page_size" class="select select-bordered select-xs w-20">
              <option :value="10">10</option>
              <option :value="20">20</option>
              <option :value="50">50</option>
            </select>
          </div>
        </div>
      </div>

      <div v-if="new_rule_modal_open" class="fixed inset-0 z-[1000] bg-black/40 flex items-center justify-center">
        <div class="card bg-base-100 w-full max-w-2xl">
          <div class="card-body">
            <div class="flex items-center justify-between mb-2">
              <h3 class="card-title">{{ editing_rule_id ? t('notifications.editNotification') : t('notifications.newNotification') }}</h3>
              <button class="btn btn-ghost btn-sm" @click="close_rule_modal"><i class="fas fa-times"></i></button>
            </div>
            <div class="space-y-4">
              <div class="space-y-3">
                <div>
                  <label class="label"><span class="label-text">{{ t('notifications.form.notificationType') }}</span></label>
                  <input class="input input-bordered w-full" v-model.trim="rule_form.type_name" :placeholder="t('notifications.form.typeNamePlaceholder')" />
                </div>
                <div>
                  <label class="label"><span class="label-text">{{ t('notifications.form.description') }}</span></label>
                  <input class="input input-bordered w-full" v-model.trim="rule_form.description" :placeholder="t('notifications.form.descriptionPlaceholder')" />
                </div>
                <div class="tabs tabs-boxed rounded-lg">
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'feishu' }" @click="channel_tab = 'feishu'">{{ t('notifications.channels.feishu') }}</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'dingtalk' }" @click="channel_tab = 'dingtalk'">{{ t('notifications.channels.dingtalk') }}</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'wecom' }" @click="channel_tab = 'wecom'">{{ t('notifications.channels.wecom') }}</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'webhook' }" @click="channel_tab = 'webhook'">{{ t('notifications.channels.webhook') }}</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'email' }" @click="channel_tab = 'email'">{{ t('notifications.channels.email') }}</a>
                </div>
                  <div class="space-y-3 mt-3" :key="channel_tab">
                    <div v-if="channel_tab === 'feishu' || channel_tab === 'dingtalk' || channel_tab === 'wecom' || channel_tab === 'webhook'">
                      <label class="label"><span class="label-text">{{ t('notifications.form.webhookUrl') }}</span></label>
                      <input class="input input-bordered w-full" v-model.trim="rule_form.config.webhook_url" />
                    </div>
                    <div v-if="channel_tab === 'feishu' || channel_tab === 'dingtalk' || channel_tab === 'wecom'">
                      <label class="label"><span class="label-text">{{ t('notifications.form.secret') }}</span></label>
                      <input class="input input-bordered w-full" type="password" v-model.trim="rule_secret_input" />
                    </div>
                    <div v-if="channel_tab === 'email'" class="space-y-3">
                      <div class="grid grid-cols-6 gap-3 items-end">
                        <div class="col-span-5">
                          <label class="label"><span class="label-text">{{ t('notifications.email.smtpServer') }} *</span></label>
                          <input class="input input-bordered w-full" :placeholder="t('notifications.email.smtpServerPlaceholder')" v-model.trim="rule_form.config.smtp_host" />
                        </div>
                        <div>
                          <label class="label"><span class="label-text">{{ t('notifications.email.port') }}</span></label>
                          <input class="input input-bordered w-full" type="number" :placeholder="t('notifications.email.portPlaceholder')" v-model.number="rule_form.config.smtp_port" />
                        </div>
                      </div>
                      <div class="flex items-center gap-6">
                        <div class="text-sm">{{ t('notifications.email.transportEncryption') }}</div>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="TLS" v-model="rule_form.config.transport_encryption" />
                          <span>{{ t('notifications.email.tls') }}</span>
                        </label>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="SSL" v-model="rule_form.config.transport_encryption" />
                          <span>{{ t('notifications.email.ssl') }}</span>
                        </label>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="NONE" v-model="rule_form.config.transport_encryption" />
                          <span>{{ t('notifications.email.none') }}</span>
                        </label>
                      </div>
                      <div>
                        <label class="label"><span class="label-text">{{ t('notifications.email.senderAccount') }}</span></label>
                        <input class="input input-bordered w-full" :placeholder="t('notifications.email.senderAccountPlaceholder')" v-model.trim="rule_form.config.email_username" />
                      </div>
                      <div class="relative">
                        <label class="label"><span class="label-text">{{ t('notifications.email.senderPassword') }}</span></label>
                        <input :type="email_password_visible ? 'text' : 'password'" class="input input-bordered w-full pr-10" :placeholder="t('notifications.email.senderPasswordPlaceholder')" v-model.trim="rule_form.config.email_password" />
                        <button type="button" class="btn btn-ghost btn-xs absolute right-2 top-9" @click="email_password_visible = !email_password_visible">
                          <i :class="email_password_visible ? 'fas fa-eye-slash' : 'fas fa-eye'"></i>
                        </button>
                      </div>
                      <div>
                        <label class="label"><span class="label-text">{{ t('notifications.email.senderAddress') }}</span></label>
                        <input class="input input-bordered w-full" :placeholder="t('notifications.email.senderAddressPlaceholder')" v-model.trim="rule_form.config.email_from" />
                      </div>
                      <div>
                        <label class="label"><span class="label-text">{{ t('notifications.email.recipientAddress') }}</span></label>
                        <input class="input input-bordered w-full" :placeholder="t('notifications.email.recipientAddressPlaceholder')" v-model.trim="rule_form.config.email_to" />
                      </div>
                    </div>
                    <div>
                      <label class="label"><span class="label-text">{{ t('notifications.form.remarks') }}</span></label>
                      <input class="input input-bordered w-full" v-model.trim="rule_form.config.remarks" />
                    </div>
                    <div class="text-sm">
                      <span>{{ t('notifications.tips.ensureRequiredInfo') }}</span>
                      <a class="link link-primary ml-2" @click="test_rule_connection">{{ t('notifications.testConnection') }}</a>
                    </div>
                  </div>
              </div>
              <div class="flex justify-end gap-2">
                <button class="btn" @click="close_rule_modal">{{ t('notifications.cancel') }}</button>
                <button class="btn btn-primary" @click="confirm_rule_modal">{{ t('notifications.confirm') }}</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import { v4 as uuid_v4 } from 'uuid'

const { t } = useI18n()


const channel_name_map: Record<string, string> = { feishu: '飞书', dingtalk: '钉钉', wecom: '企业微信', webhook: 'Webhook', email: '邮件' }
const page_size = ref(10)

type RuleConfig = { webhook_url?: string, remarks?: string, email_to?: string, email_from?: string, smtp_host?: string, smtp_port?: number, transport_encryption?: string, email_username?: string, email_password?: string }
type NotificationRule = { id: string, type_name: string, description: string, enabled: boolean, channel: string, endpoint_name: string, config: RuleConfig, updated_at: string }
const rules = ref<NotificationRule[]>([])
const rule_secrets = reactive<Record<string, string>>({})
const new_rule_modal_open = ref(false)
const editing_rule_id = ref<string | null>(null)
const channel_tab = ref('feishu')
const rule_form = reactive<NotificationRule>({ id: '', type_name: '', description: '', enabled: true, channel: 'feishu', endpoint_name: '', config: { transport_encryption: 'TLS', smtp_port: 25 }, updated_at: '' })
const rule_secret_input = ref('')
const email_password_visible = ref(false)

const paged_rules = computed(() => rules.value.slice(0, page_size.value))

const load_rules = () => {
  try {
    const raw = localStorage.getItem('sentinel-notification-rules')
    if (raw) rules.value = JSON.parse(raw)
  } catch (e) {
    console.warn('load_rules_failed')
  }
}
const save_rules = () => {
  localStorage.setItem('sentinel-notification-rules', JSON.stringify(rules.value))
}
const open_new_rule_modal = () => {
  editing_rule_id.value = null
  rule_form.id = uuid_v4()
  rule_form.enabled = true
  channel_tab.value = 'feishu'
  rule_form.channel = 'feishu'
  rule_form.endpoint_name = ''
  rule_form.type_name = ''
  rule_form.description = ''
  rule_form.config = { transport_encryption: 'TLS', smtp_port: 25 }
  rule_secret_input.value = ''
  new_rule_modal_open.value = true
}
const open_edit_rule_modal = (r: NotificationRule) => {
  editing_rule_id.value = r.id
  rule_form.id = r.id
  rule_form.type_name = r.type_name
  rule_form.description = r.description
  rule_form.enabled = r.enabled
  channel_tab.value = r.channel
  rule_form.channel = r.channel
  rule_form.endpoint_name = r.endpoint_name
  rule_form.config = { ...r.config }
  rule_secret_input.value = rule_secrets[r.id] || ''
  new_rule_modal_open.value = true
}
const close_rule_modal = () => { new_rule_modal_open.value = false }
const confirm_rule_modal = () => {
  rule_form.channel = channel_tab.value
  rule_form.updated_at = new Date().toISOString().slice(0, 19).replace('T', ' ')
  if (rule_secret_input.value) rule_secrets[rule_form.id] = rule_secret_input.value
  if (editing_rule_id.value) {
    const idx = rules.value.findIndex(x => x.id === rule_form.id)
    if (idx >= 0) rules.value[idx] = { ...rule_form }
  } else {
    rules.value.unshift({ ...rule_form })
  }
  save_rules()
  new_rule_modal_open.value = false
}
const delete_rule = async (r: NotificationRule) => {
  const ok = await dialog.confirm({ title: t('notifications.messages.deleteTitle'), message: t('notifications.messages.deleteConfirm'), variant: 'warning' })
  if (!ok) return
  rules.value = rules.value.filter(x => x.id !== r.id)
  delete rule_secrets[r.id]
  save_rules()
}
const update_rule_status = (r: NotificationRule) => {
  r.updated_at = new Date().toISOString().slice(0, 19).replace('T', ' ')
  save_rules()
}
const test_rule_connection = async () => {
  if (channel_tab.value === 'webhook') {
    if (!rule_form.config.webhook_url) return dialog.toast.error(t('notifications.messages.webhookUrlRequired'))
    try {
      const res = await fetch(String(rule_form.config.webhook_url), { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ title: 'Sentinel AI', content: 'Test' }) })
      if (res.ok) dialog.toast.success(t('notifications.messages.connectionNormal'))
      else dialog.toast.error(String(res.status))
    } catch (e: any) {
      dialog.toast.error(e?.message || t('notifications.messages.connectionFailed'))
    }
    return
  }
  try {
    await send_test(channel_tab.value, { preview: rule_form.config })
    dialog.toast.success(t('notifications.messages.testTriggered'))
  } catch (e) {
    console.warn('test_rule_connection_failed')
  }
}
const send_test = async (channel: string, extra: Record<string, any> = {}) => {
  const payload = {
    channel,
    config: extra.preview || {},
    message: { title: 'Sentinel AI', content: 'This is a test.' }
  }
  try {
    await invoke('send_notification', { payload } as any)
    dialog.toast.success(t('notifications.testSent', '测试已发送'))
  } catch {
    dialog.toast.info(t('notifications.testSimulated', '发送失败'))
  }
}

onMounted(() => {
  load_rules()
})
</script>

<style scoped>
.page-content-padded { padding: 16px; }
.safe-top { padding-top: 12px; }
.fade-slide-enter-active { transition: all .15s ease; }
.fade-slide-leave-active { transition: all .12s ease; }
.fade-slide-enter-from { opacity: 0; transform: translateY(6px); }
.fade-slide-leave-to { opacity: 0; transform: translateY(-6px); }
</style>
