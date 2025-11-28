<template>
  <div class="page-content-padded safe-top space-y-6">
    <div class="flex justify-between items-center">
      <h2 class="text-2xl font-bold">{{ t('notifications.title', '通知管理') }}</h2>
    </div>

    <div class="space-y-4">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex items-center justify-between mb-2">
            <button class="btn btn-primary" @click="open_new_rule_modal"><i class="fas fa-plus mr-2"></i>新建通知</button>
            <div class="text-sm text-base-content/60">共 {{ rules.length }} 条</div>
          </div>
          <div class="overflow-x-auto">
            <table class="table w-full">
              <thead>
                <tr>
                  <th>通知类型</th>
                  <th>通知状态</th>
                  <th>通知配置</th>
                  <th>更新时间</th>
                  <th>操作</th>
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
                      <span class="badge badge-ghost">{{ channel_name_map[rule.channel] }}</span>
                      <span class="text-xs">{{ rule.endpoint_name }}</span>
                    </div>
                  </td>
                  <td>{{ rule.updated_at }}</td>
                  <td>
                    <button class="btn btn-link btn-xs" @click="open_edit_rule_modal(rule)">编辑</button>
                    <button class="btn btn-link btn-xs text-error" @click="delete_rule(rule)">删除</button>
                  </td>
                </tr>
                <tr v-if="rules.length === 0">
                  <td colspan="5" class="text-center text-base-content/50">暂无规则</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="flex items-center justify-end gap-2 mt-2">
            <span class="text-sm">每页</span>
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
              <h3 class="card-title">{{ editing_rule_id ? '编辑通知' : '新建通知' }}</h3>
              <button class="btn btn-ghost btn-sm" @click="close_rule_modal"><i class="fas fa-times"></i></button>
            </div>
            <div class="space-y-4">
              <div class="space-y-3">
                <div>
                  <label class="label"><span class="label-text">通知类型</span></label>
                  <input class="input input-bordered w-full" v-model.trim="rule_form.type_name" placeholder="请输入通知类型" />
                </div>
                <div>
                  <label class="label"><span class="label-text">说明</span></label>
                  <input class="input input-bordered w-full" v-model.trim="rule_form.description" placeholder="可选，补充说明" />
                </div>
                <div class="tabs tabs-boxed rounded-lg">
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'feishu' }" @click="channel_tab = 'feishu'">飞书</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'dingtalk' }" @click="channel_tab = 'dingtalk'">钉钉</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'wecom' }" @click="channel_tab = 'wecom'">企业微信</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'webhook' }" @click="channel_tab = 'webhook'">Webhook</a>
                  <a class="tab" :class="{ 'tab-active': channel_tab === 'email' }" @click="channel_tab = 'email'">邮件</a>
                </div>
                  <div class="space-y-3 mt-3" :key="channel_tab">
                    <div v-if="channel_tab === 'feishu' || channel_tab === 'dingtalk' || channel_tab === 'wecom' || channel_tab === 'webhook'">
                      <label class="label"><span class="label-text">WebHookURL</span></label>
                      <input class="input input-bordered w-full" v-model.trim="rule_form.config.webhook_url" />
                    </div>
                    <div v-if="channel_tab === 'feishu' || channel_tab === 'dingtalk' || channel_tab === 'wecom'">
                      <label class="label"><span class="label-text">Secret</span></label>
                      <input class="input input-bordered w-full" type="password" v-model.trim="rule_secret_input" />
                    </div>
                    <div v-if="channel_tab === 'email'" class="space-y-3">
                      <div class="grid grid-cols-6 gap-3 items-end">
                        <div class="col-span-5">
                          <label class="label"><span class="label-text">SMTP 服务器 *</span></label>
                          <input class="input input-bordered w-full" placeholder="主机名、域名或 IP 地址" v-model.trim="rule_form.config.smtp_host" />
                        </div>
                        <div>
                          <label class="label"><span class="label-text">端口</span></label>
                          <input class="input input-bordered w-full" type="number" placeholder="25" v-model.number="rule_form.config.smtp_port" />
                        </div>
                      </div>
                      <div class="flex items-center gap-6">
                        <div class="text-sm">传输加密方式</div>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="TLS" v-model="rule_form.config.transport_encryption" />
                          <span>TLS</span>
                        </label>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="SSL" v-model="rule_form.config.transport_encryption" />
                          <span>SSL</span>
                        </label>
                        <label class="flex items-center gap-2">
                          <input type="radio" class="radio radio-primary" value="NONE" v-model="rule_form.config.transport_encryption" />
                          <span>未加密（明文传输）</span>
                        </label>
                      </div>
                      <div>
                        <label class="label"><span class="label-text">发件邮箱账号</span></label>
                        <input class="input input-bordered w-full" placeholder="为空时不使用账号" v-model.trim="rule_form.config.email_username" />
                      </div>
                      <div class="relative">
                        <label class="label"><span class="label-text">发件邮箱密码</span></label>
                        <input :type="email_password_visible ? 'text' : 'password'" class="input input-bordered w-full pr-10" placeholder="为空时使用空密码" v-model.trim="rule_form.config.email_password" />
                        <button type="button" class="btn btn-ghost btn-xs absolute right-2 top-9" @click="email_password_visible = !email_password_visible">
                          <i :class="email_password_visible ? 'fas fa-eye-slash' : 'fas fa-eye'"></i>
                        </button>
                      </div>
                      <div>
                        <label class="label"><span class="label-text">发件邮箱地址</span></label>
                        <input class="input input-bordered w-full" placeholder="为空时使用默认地址" v-model.trim="rule_form.config.email_from" />
                      </div>
                      <div>
                        <label class="label"><span class="label-text">收件人地址</span></label>
                        <input class="input input-bordered w-full" placeholder="多个地址用逗号分隔" v-model.trim="rule_form.config.email_to" />
                      </div>
                    </div>
                    <div>
                      <label class="label"><span class="label-text">用户备注</span></label>
                      <input class="input input-bordered w-full" v-model.trim="rule_form.config.remarks" />
                    </div>
                    <div class="text-sm">
                      <span>1. 确保上方必填信息完整</span>
                      <a class="link link-primary ml-2" @click="test_rule_connection">测试连接状态</a>
                    </div>
                  </div>
              </div>
              <div class="flex justify-end gap-2">
                <button class="btn" @click="close_rule_modal">取消</button>
                <button class="btn btn-primary" @click="confirm_rule_modal">确定</button>
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
  const ok = await dialog.confirm({ title: '删除通知', message: '确定删除该通知吗？', variant: 'warning' })
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
    if (!rule_form.config.webhook_url) return dialog.toast.error('请填写WebHookURL')
    try {
      const res = await fetch(String(rule_form.config.webhook_url), { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ title: 'Sentinel AI', content: 'Test' }) })
      if (res.ok) dialog.toast.success('连接正常')
      else dialog.toast.error(String(res.status))
    } catch (e: any) {
      dialog.toast.error(e?.message || '连接失败')
    }
    return
  }
  try {
    await send_test(channel_tab.value, { preview: rule_form.config })
    dialog.toast.success('已触发测试')
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
