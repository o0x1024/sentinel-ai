import { listen } from '@tauri-apps/api/event'

export function useWorkflowEvents() {
  const unsubs: Array<() => void> = []

  const on_run_start = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:run-start', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const on_step_start = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:step-start', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const on_step_complete = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:step-complete', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const on_progress = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:progress', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const on_run_complete = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:run-complete', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const on_run_stop = async (handler: (payload: any) => void) => {
    const u = await listen('workflow:run-stop', (e: any) => handler(e?.payload))
    unsubs.push(u)
    return u
  }

  const unsubscribe_all = () => {
    unsubs.forEach(u => { try { u() } catch (_e) { void 0 } })
    unsubs.length = 0
  }

  return {
    on_run_start,
    on_step_start,
    on_step_complete,
    on_progress,
    on_run_complete,
    on_run_stop,
    unsubscribe_all,
  }
}
