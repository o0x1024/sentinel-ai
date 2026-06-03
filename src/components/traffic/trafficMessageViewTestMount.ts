interface TrafficMessageViewTestGlobalOptions {
  httpMessageSurface?: unknown
  trafficMessageReader?: unknown
  trafficMessageViewTabs?: unknown
  codeDiffViewer?: unknown
  appDialog?: unknown
  stubs?: Record<string, unknown>
  components?: Record<string, unknown>
  mocks?: Record<string, unknown>
}

export function createTrafficMessageViewTestGlobal(
  options: TrafficMessageViewTestGlobalOptions = {},
) {
  const stubs: Record<string, unknown> = {
    ...(options.httpMessageSurface ? { HttpMessageSurface: options.httpMessageSurface } : {}),
    ...(options.trafficMessageReader ? { TrafficMessageReader: options.trafficMessageReader } : {}),
    ...(options.trafficMessageViewTabs ? { TrafficMessageViewTabs: options.trafficMessageViewTabs } : {}),
    ...(options.codeDiffViewer ? { CodeDiffViewer: options.codeDiffViewer } : {}),
    ...(options.stubs || {}),
  }

  const components: Record<string, unknown> = {
    ...(options.appDialog ? { AppDialog: options.appDialog } : {}),
    ...(options.components || {}),
  }

  return {
    mocks: {
      $t: (key: string, fallback?: string) => fallback ?? key,
      ...(options.mocks || {}),
    },
    stubs,
    components,
  }
}
