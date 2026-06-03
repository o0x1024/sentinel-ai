export function setLocalStorageItem(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value)
  } catch {
    // Local UI preference persistence must not interrupt the active workflow.
  }
}

export function setSessionStorageItem(key: string, value: string) {
  try {
    window.sessionStorage.setItem(key, value)
  } catch {
    // Local UI preference persistence must not interrupt the active workflow.
  }
}
