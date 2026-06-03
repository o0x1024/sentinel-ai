<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

type AdminUser = {
  admin_id: string
  role: string
  active: boolean
}

type LicenseCard = {
  id: number
  batch_id: number | null
  username: string | null
  activation_key: string | null
  tier: string
  feature_ids: string[]
  status: string
  device_limit: number
  device_count: number
  expires_at: number | null
  first_activated_at: number | null
  last_activated_at: number | null
  created_at: number
}

type LicenseDevice = {
  id: number
  username: string
  machine_id: string
  device_name: string | null
  revoked: boolean
  first_seen_at: number
  last_seen_at: number
}

type CreatedCard = {
  id: number
  username: string
  activation_key: string
  tier: string
  device_limit: number
  expires_at: number | null
}

const API_KEY_STORAGE = 'sentinel.license-admin.api-key'

const apiKey = ref('')
const admin = ref<AdminUser | null>(null)
const authError = ref('')
const loading = ref(false)
const activeView = ref<'cards' | 'devices' | 'audit'>('cards')
const cards = ref<LicenseCard[]>([])
const selectedCard = ref<LicenseCard | null>(null)
const devices = ref<LicenseDevice[]>([])
const createdCards = ref<CreatedCard[]>([])
const deleteError = ref('')
const deletingCardId = ref<number | null>(null)
const selectedCardIds = ref<number[]>([])
const bulkDeleting = ref(false)
const detailLoading = ref(false)
const detailCard = ref<LicenseCard | null>(null)
const detailDevices = ref<LicenseDevice[]>([])
const form = ref({
  name: '',
  count: 10,
  tier: 'pro',
  deviceLimit: 1,
  ttlDays: 365,
})

const isAuthed = computed(() => Boolean(admin.value))
const visibleCards = computed(() => cards.value)
const allVisibleSelected = computed(() =>
  visibleCards.value.length > 0 && visibleCards.value.every(card => selectedCardIds.value.includes(card.id)),
)

onMounted(() => {
  apiKey.value = localStorage.getItem(API_KEY_STORAGE) || ''
  if (apiKey.value) {
    void login()
  }
})

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${apiKey.value}`,
      ...(init.headers || {}),
    },
  })
  if (!response.ok) {
    const payload = await response.json().catch(() => null)
    throw new Error(payload?.error || payload?.message || `HTTP ${response.status}`)
  }
  return response.json()
}

async function login() {
  loading.value = true
  authError.value = ''
  try {
    admin.value = await request<AdminUser>('/api/admin/session')
    localStorage.setItem(API_KEY_STORAGE, apiKey.value)
    await loadCards()
  } catch (error) {
    admin.value = null
    authError.value = String(error)
  } finally {
    loading.value = false
  }
}

function logout() {
  admin.value = null
  apiKey.value = ''
  localStorage.removeItem(API_KEY_STORAGE)
}

async function loadCards() {
  const payload = await request<{ cards: LicenseCard[] }>('/api/admin/cards')
  cards.value = payload.cards
  selectedCardIds.value = selectedCardIds.value.filter(id => cards.value.some(card => card.id === id))
  if (selectedCard.value) {
    selectedCard.value = cards.value.find(card => card.id === selectedCard.value?.id) || null
  }
}

async function createCards() {
  loading.value = true
  createdCards.value = []
  try {
    const ttlSeconds = Math.max(1, form.value.ttlDays) * 24 * 60 * 60
    const payload = await request<{ cards: CreatedCard[] }>('/api/admin/card-batches', {
      method: 'POST',
      body: JSON.stringify({
        name: form.value.name || null,
        count: form.value.count,
        tier: form.value.tier,
        device_limit: form.value.deviceLimit,
        ttl_seconds: ttlSeconds,
      }),
    })
    createdCards.value = payload.cards
    await loadCards()
  } finally {
    loading.value = false
  }
}

async function openDevices(card: LicenseCard) {
  selectedCard.value = card
  activeView.value = 'devices'
  const payload = await request<{ devices: LicenseDevice[] }>(`/api/admin/cards/${card.id}/devices`)
  devices.value = payload.devices
}

async function openCardDetail(card: LicenseCard) {
  detailLoading.value = true
  deleteError.value = ''
  try {
    const payload = await request<{ card: LicenseCard; devices: LicenseDevice[] }>(`/api/admin/cards/${card.id}`)
    detailCard.value = payload.card
    detailDevices.value = payload.devices
  } catch (error) {
    deleteError.value = String(error)
  } finally {
    detailLoading.value = false
  }
}

function closeCardDetail() {
  detailCard.value = null
  detailDevices.value = []
}

function toggleCardSelection(cardId: number, checked: boolean) {
  const next = new Set(selectedCardIds.value)
  if (checked) {
    next.add(cardId)
  } else {
    next.delete(cardId)
  }
  selectedCardIds.value = Array.from(next)
}

function toggleAllVisibleCards(checked: boolean) {
  selectedCardIds.value = checked ? visibleCards.value.map(card => card.id) : []
}

async function bulkDeleteCards() {
  if (selectedCardIds.value.length === 0) {
    return
  }
  if (!window.confirm(`确认删除选中的 ${selectedCardIds.value.length} 张卡密吗？`)) {
    return
  }

  bulkDeleting.value = true
  deleteError.value = ''
  try {
    const payload = await request<{ deleted_ids: number[]; cards: LicenseCard[] }>('/api/admin/cards/bulk-delete', {
      method: 'POST',
      body: JSON.stringify({ card_ids: selectedCardIds.value }),
    })
    cards.value = payload.cards
    selectedCardIds.value = []
    if (selectedCard.value && payload.deleted_ids.includes(selectedCard.value.id)) {
      selectedCard.value = null
      devices.value = []
      activeView.value = 'cards'
    }
    if (detailCard.value && payload.deleted_ids.includes(detailCard.value.id)) {
      closeCardDetail()
    }
  } catch (error) {
    deleteError.value = String(error)
  } finally {
    bulkDeleting.value = false
  }
}

async function deleteCard(card: LicenseCard) {
  const label = card.username ? `用户 ${card.username}` : `卡密 #${card.id}`
  if (!window.confirm(`确认删除 ${label} 吗？删除后该卡密和关联设备将不能继续激活或续期。`)) {
    return
  }

  deletingCardId.value = card.id
  deleteError.value = ''
  try {
    const payload = await request<{ cards: LicenseCard[] }>(`/api/admin/cards/${card.id}`, {
      method: 'DELETE',
    })
    cards.value = payload.cards
    selectedCardIds.value = selectedCardIds.value.filter(id => id !== card.id)
    if (selectedCard.value?.id === card.id) {
      selectedCard.value = null
      devices.value = []
      activeView.value = 'cards'
    }
    if (detailCard.value?.id === card.id) {
      closeCardDetail()
    }
  } catch (error) {
    deleteError.value = String(error)
  } finally {
    deletingCardId.value = null
  }
}

function formatTime(value: number | null) {
  if (!value) return '-'
  return new Date(value * 1000).toLocaleString()
}

function formatFeatures(features: string[]) {
  return features.length ? features.join(', ') : '-'
}

function readChecked(event: Event) {
  return event.target instanceof HTMLInputElement && event.target.checked
}
</script>

<template>
  <main v-if="!isAuthed" class="login-shell">
    <section class="login-panel">
      <div class="brand-mark">S</div>
      <h1>Sentinel License Admin</h1>
      <p>输入管理员密钥进入卡密管理后台。</p>
      <label>
        管理员 API Key
        <input v-model="apiKey" type="password" placeholder="Bearer key" @keyup.enter="login" />
      </label>
      <button :disabled="loading || !apiKey.trim()" @click="login">
        {{ loading ? '登录中...' : '登录' }}
      </button>
      <div v-if="authError" class="error">{{ authError }}</div>
    </section>
  </main>

  <main v-else class="app-shell">
    <nav class="navbar">
      <div>
        <strong>Sentinel License</strong>
        <span>卡密授权平台</span>
      </div>
      <div class="nav-user">
        <span>{{ admin?.admin_id }} / {{ admin?.role }}</span>
        <button @click="logout">退出</button>
      </div>
    </nav>

    <aside class="sidebar">
      <button :class="{ active: activeView === 'cards' }" @click="activeView = 'cards'">卡密管理</button>
      <button :class="{ active: activeView === 'devices' }" :disabled="!selectedCard" @click="activeView = 'devices'">设备绑定</button>
      <button :class="{ active: activeView === 'audit' }" disabled>审计日志</button>
    </aside>

    <section class="content">
      <template v-if="activeView === 'cards'">
        <header class="content-header">
          <div>
            <h2>卡密管理</h2>
            <p>创建卡密后，系统自动生成 8 位用户名；把用户名和卡密交给用户，客户端激活时会自动绑定设备。</p>
          </div>
          <div class="header-actions">
            <button
              class="danger"
              :disabled="selectedCardIds.length === 0 || bulkDeleting"
              @click="bulkDeleteCards"
            >
              {{ bulkDeleting ? '批量删除中' : `批量删除 (${selectedCardIds.length})` }}
            </button>
            <button @click="loadCards">刷新</button>
          </div>
        </header>

        <form class="create-panel" @submit.prevent="createCards">
          <label>批次名称<input v-model="form.name" placeholder="2026-Q2 Pro" /></label>
          <label>数量<input v-model.number="form.count" type="number" min="1" max="500" /></label>
          <label>等级<input v-model="form.tier" /></label>
          <label>设备数<input v-model.number="form.deviceLimit" type="number" min="1" max="128" /></label>
          <label>有效天数<input v-model.number="form.ttlDays" type="number" min="1" /></label>
          <button :disabled="loading">{{ loading ? '生成中...' : '生成卡密' }}</button>
        </form>

        <section v-if="createdCards.length" class="created-panel">
          <h3>本次生成的卡密</h3>
          <textarea readonly :value="createdCards.map(card => `${card.username},${card.activation_key}`).join('\n')" />
        </section>

        <div v-if="deleteError" class="error-banner">{{ deleteError }}</div>

        <table>
          <thead>
            <tr>
              <th class="select-cell">
                <input
                  type="checkbox"
                  :checked="allVisibleSelected"
                  @change="toggleAllVisibleCards(readChecked($event))"
                />
              </th>
              <th>ID</th>
              <th>用户</th>
              <th>卡密</th>
              <th>状态</th>
              <th>等级</th>
              <th>设备</th>
              <th>过期时间</th>
              <th>最近激活</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="card in visibleCards" :key="card.id">
              <td class="select-cell">
                <input
                  type="checkbox"
                  :checked="selectedCardIds.includes(card.id)"
                  @change="toggleCardSelection(card.id, readChecked($event))"
                />
              </td>
              <td>#{{ card.id }}</td>
              <td>{{ card.username || '-' }}</td>
              <td class="mono compact">{{ card.activation_key || '-' }}</td>
              <td><span class="status" :data-status="card.status">{{ card.status }}</span></td>
              <td>{{ card.tier }}</td>
              <td>{{ card.device_count }} / {{ card.device_limit }}</td>
              <td>{{ formatTime(card.expires_at) }}</td>
              <td>{{ formatTime(card.last_activated_at) }}</td>
              <td class="row-actions">
                <button @click="openCardDetail(card)">详情</button>
                <button @click="openDevices(card)">设备</button>
                <button
                  class="danger"
                  :disabled="deletingCardId === card.id"
                  @click="deleteCard(card)"
                >
                  {{ deletingCardId === card.id ? '删除中' : '删除' }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>

        <section v-if="detailCard" class="detail-panel">
          <div class="detail-header">
            <div>
              <h3>卡密详情 #{{ detailCard.id }}</h3>
              <p>用户和卡密信息可直接发给用户用于客户端激活。</p>
            </div>
            <button @click="closeCardDetail">关闭</button>
          </div>
          <div v-if="detailLoading" class="muted">加载中...</div>
          <div v-else class="detail-grid">
            <label>用户名<input readonly :value="detailCard.username || '-'" /></label>
            <label>卡密<input readonly :value="detailCard.activation_key || '-'" /></label>
            <label>等级<input readonly :value="detailCard.tier" /></label>
            <label>功能<input readonly :value="formatFeatures(detailCard.feature_ids)" /></label>
            <label>设备数<input readonly :value="`${detailCard.device_count} / ${detailCard.device_limit}`" /></label>
            <label>过期时间<input readonly :value="formatTime(detailCard.expires_at)" /></label>
          </div>

          <h4>绑定设备</h4>
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>用户</th>
                <th>设备</th>
                <th>状态</th>
                <th>首次绑定</th>
                <th>最近使用</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="device in detailDevices" :key="device.id">
                <td>#{{ device.id }}</td>
                <td>{{ device.username }}</td>
                <td class="mono">{{ device.machine_id }}</td>
                <td>{{ device.revoked ? '已吊销' : '正常' }}</td>
                <td>{{ formatTime(device.first_seen_at) }}</td>
                <td>{{ formatTime(device.last_seen_at) }}</td>
              </tr>
              <tr v-if="detailDevices.length === 0">
                <td colspan="6" class="muted">暂无绑定设备</td>
              </tr>
            </tbody>
          </table>
        </section>
      </template>

      <template v-else-if="activeView === 'devices'">
        <header class="content-header">
          <div>
            <h2>设备绑定</h2>
            <p>卡密 #{{ selectedCard?.id }} 当前绑定的设备。</p>
          </div>
          <button @click="activeView = 'cards'">返回卡密</button>
        </header>

        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>用户</th>
              <th>设备</th>
              <th>状态</th>
              <th>首次绑定</th>
              <th>最近使用</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="device in devices" :key="device.id">
              <td>#{{ device.id }}</td>
              <td>{{ device.username }}</td>
              <td class="mono">{{ device.machine_id }}</td>
              <td>{{ device.revoked ? '已吊销' : '正常' }}</td>
              <td>{{ formatTime(device.first_seen_at) }}</td>
              <td>{{ formatTime(device.last_seen_at) }}</td>
            </tr>
          </tbody>
        </table>
      </template>
    </section>
  </main>
</template>
