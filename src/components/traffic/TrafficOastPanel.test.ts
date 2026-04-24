import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import TrafficOastPanel from './TrafficOastPanel.vue'
import type { TrafficOastRecord } from './proxyConfigurationTypes'

const dialogMocks = vi.hoisted(() => ({
  dialogConfirmMock: vi.fn().mockResolvedValue(true),
  toastSuccessMock: vi.fn(),
  toastErrorMock: vi.fn(),
  toastWarningMock: vi.fn(),
  toastInfoMock: vi.fn(),
}))

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('./trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('@/composables/useDialog', () => ({
  dialog: {
    confirm: dialogMocks.dialogConfirmMock,
    toast: {
      success: dialogMocks.toastSuccessMock,
      error: dialogMocks.toastErrorMock,
      warning: dialogMocks.toastWarningMock,
      info: dialogMocks.toastInfoMock,
    },
  },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  writeTextFile: vi.fn(),
}))

function createEvent(overrides: Partial<TrafficOastRecord['events'][number]> = {}) {
  return {
    time: '2026-04-23T10:00:00.000Z',
    host: 'token-1.oast.test',
    method: 'GET',
    url: 'https://token-1.oast.test/a',
    path: '/a',
    query: {},
    userAgent: 'Vitest UA',
    referer: '',
    ip: '127.0.0.1',
    ray: '',
    colo: '',
    country: 'CN',
    asn: null,
    ...overrides,
  }
}

function createRecord(events = [createEvent()]): TrafficOastRecord {
  return {
    token: 'token-1',
    fqdn: 'token-1.oast.test',
    httpUrl: 'http://token-1.oast.test/',
    httpsUrl: 'https://token-1.oast.test/',
    createdAt: '2026-04-23T09:00:00.000Z',
    label: 'Token 1',
    sourceTool: 'traffic-oast-panel',
    sourceRequestId: null,
    hitCount: events.length,
    lastHitAt: events.at(-1)?.time ?? null,
    lastSyncAt: '2026-04-23T10:00:00.000Z',
    events,
  }
}

function createCommandResponse<T>(data: T) {
  return { success: true, data }
}

function mountPanel() {
  return mount(TrafficOastPanel, {
    global: {
      mocks: {
        $t: (key: string) => key,
      },
      stubs: {
        teleport: true,
      },
    },
  })
}

describe('TrafficOastPanel', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    dialogMocks.dialogConfirmMock.mockClear()
    dialogMocks.toastSuccessMock.mockClear()
    dialogMocks.toastErrorMock.mockClear()
    dialogMocks.toastWarningMock.mockClear()
    dialogMocks.toastInfoMock.mockClear()
  })

  it('hides a single event without confirmation', async () => {
    const initialRecord = createRecord()
    const updatedRecord = createRecord([])

    global.testUtils.mockInvoke
      .mockResolvedValueOnce(createCommandResponse({ enabled: true, serverBaseUrl: 'https://oast.test', apiKey: '', pollIntervalSecs: 15, requestTimeoutSecs: 10 }))
      .mockResolvedValueOnce(createCommandResponse([initialRecord]))
      .mockResolvedValueOnce(createCommandResponse({
        token: initialRecord.token,
        hiddenCount: 1,
        visibleEventCount: 0,
        record: updatedRecord,
      }))

    const wrapper = mountPanel()
    await flushPromises()

    const deleteButton = wrapper.findAll('button').find(button => button.text().includes('trafficAnalysis.oast.deleteEvent'))
    expect(deleteButton).toBeTruthy()

    await deleteButton!.trigger('click')
    await flushPromises()

    expect(dialogMocks.dialogConfirmMock).not.toHaveBeenCalled()
    expect(global.testUtils.mockInvoke).toHaveBeenLastCalledWith('hide_traffic_oast_events_command', {
      payload: {
        token: 'token-1',
        eventKeys: [
          {
            time: '2026-04-23T10:00:00.000Z',
            method: 'GET',
            url: 'https://token-1.oast.test/a',
            ip: '127.0.0.1',
          },
        ],
      },
    })
    expect(wrapper.text()).toContain('trafficAnalysis.oast.noEvents')
  })

  it('hides all events for a record in one action', async () => {
    const initialRecord = createRecord([
      createEvent(),
      createEvent({
        time: '2026-04-23T10:05:00.000Z',
        method: 'POST',
        url: 'https://token-1.oast.test/b',
        path: '/b',
        ip: '127.0.0.2',
      }),
    ])
    const updatedRecord = createRecord([])

    global.testUtils.mockInvoke
      .mockResolvedValueOnce(createCommandResponse({ enabled: true, serverBaseUrl: 'https://oast.test', apiKey: '', pollIntervalSecs: 15, requestTimeoutSecs: 10 }))
      .mockResolvedValueOnce(createCommandResponse([initialRecord]))
      .mockResolvedValueOnce(createCommandResponse({
        token: initialRecord.token,
        hiddenCount: 2,
        visibleEventCount: 0,
        record: updatedRecord,
      }))

    const wrapper = mountPanel()
    await flushPromises()

    const deleteAllButton = wrapper.findAll('button').find(button => button.text().includes('trafficAnalysis.oast.deleteAllEvents'))
    expect(deleteAllButton).toBeTruthy()

    await deleteAllButton!.trigger('click')
    await flushPromises()

    expect(dialogMocks.dialogConfirmMock).not.toHaveBeenCalled()
    expect(global.testUtils.mockInvoke).toHaveBeenLastCalledWith('hide_traffic_oast_events_command', {
      payload: {
        token: 'token-1',
        eventKeys: [
          {
            time: '2026-04-23T10:00:00.000Z',
            method: 'GET',
            url: 'https://token-1.oast.test/a',
            ip: '127.0.0.1',
          },
          {
            time: '2026-04-23T10:05:00.000Z',
            method: 'POST',
            url: 'https://token-1.oast.test/b',
            ip: '127.0.0.2',
          },
        ],
      },
    })
    expect(wrapper.text()).toContain('trafficAnalysis.oast.noEvents')
  })
})
