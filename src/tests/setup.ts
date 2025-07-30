import { vi } from 'vitest'
import { config } from '@vue/test-utils'

// Mock Tauri API
const mockInvoke = vi.fn()
const mockListen = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
}))

// Global test configuration
config.global.mocks = {
  $route: {
    path: '/',
    params: {},
    query: {},
    meta: {},
  },
  $router: {
    push: vi.fn(),
    replace: vi.fn(),
    go: vi.fn(),
    back: vi.fn(),
  },
}

// Setup DOM environment
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock Chart.js
vi.mock('chart.js', () => ({
  Chart: vi.fn().mockImplementation(() => ({
    destroy: vi.fn(),
    update: vi.fn(),
    resize: vi.fn(),
  })),
  registerables: [],
}))

// Global test utilities
declare global {
  // eslint-disable-next-line no-var
  var testUtils: {
    mockInvoke: typeof mockInvoke
    mockListen: typeof mockListen
  }
}

global.testUtils = {
  mockInvoke,
  mockListen,
} 