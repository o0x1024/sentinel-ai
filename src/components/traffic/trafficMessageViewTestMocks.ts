import { ref } from 'vue'
import { vi } from 'vitest'

export function createVueI18nMock(locale = 'zh-CN') {
  return {
    useI18n: () => ({
      t: (key: string, fallback?: string) => fallback ?? key,
      locale: ref(locale),
    }),
  }
}

export function createTrafficPaneCompactModeMock() {
  return {
    useTrafficPaneCompactMode: () => ({
      panelRef: ref(null),
      isCompact: ref(false),
    }),
  }
}

export function createImmersiveDrillModeMock() {
  return {
    immersiveDrillModeEnabled: ref(false),
  }
}

export function createDialogToastMock() {
  return {
    dialog: {
      toast: {
        success: vi.fn(),
        error: vi.fn(),
        warning: vi.fn(),
        info: vi.fn(),
      },
    },
  }
}

export function createDialogConfirmAndToastMock() {
  return {
    dialog: {
      confirm: vi.fn().mockResolvedValue(true),
      toast: {
        success: vi.fn(),
        error: vi.fn(),
        warning: vi.fn(),
        info: vi.fn(),
      },
    },
  }
}
