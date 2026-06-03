import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SecurityEvidenceRawExchangePanel from './SecurityEvidenceRawExchangePanel.vue'

const dialogMock = vi.hoisted(() => ({
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}))

vi.mock('@/composables/useDialog', () => ({
  dialog: {
    toast: {
      success: dialogMock.toastSuccess,
      error: dialogMock.toastError,
    },
  },
}))

describe('SecurityEvidenceRawExchangePanel', () => {
  const writeText = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })
  })

  it('copies the raw request content from the request block', async () => {
    const wrapper = mount(SecurityEvidenceRawExchangePanel, {
      props: {
        stateKeyPrefix: 'test:evidence',
        fallbackUrl: 'https://example.test/root',
        exchange: {
          requestMethod: 'POST',
          requestUrl: '/login?debug=1',
          requestHeaders: 'Content-Type: application/json',
          requestBody: '{"user":"alice"}',
          responseStatus: 200,
          responseHeaders: 'Content-Type: application/json',
          responseBody: '{"ok":true}',
        },
      },
    })

    await wrapper.get('button[aria-label="复制请求"]').trigger('click')

    expect(writeText).toHaveBeenCalledWith(
      [
        'POST /login?debug=1 HTTP/1.1',
        'Host: example.test',
        'Content-Type: application/json',
        '',
        '{"user":"alice"}',
      ].join('\r\n'),
    )
    expect(dialogMock.toastSuccess).toHaveBeenCalledWith('请求已复制')
  })

  it('copies the raw response content from the response block', async () => {
    const wrapper = mount(SecurityEvidenceRawExchangePanel, {
      props: {
        stateKeyPrefix: 'test:evidence',
        exchange: {
          requestMethod: 'GET',
          requestUrl: 'https://example.test/status',
          responseStatus: 404,
          responseHeaders: 'X-Test: missing',
          responseBody: 'not found',
        },
      },
    })

    await wrapper.get('button[aria-label="复制响应"]').trigger('click')

    expect(writeText).toHaveBeenCalledWith(
      ['HTTP/1.1 404 Not Found', 'X-Test: missing', '', 'not found'].join('\r\n'),
    )
    expect(dialogMock.toastSuccess).toHaveBeenCalledWith('响应已复制')
  })
})
