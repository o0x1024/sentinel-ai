const INTRUDER_DICTIONARY_PICKER_TYPE_STORAGE_KEY = 'trafficAnalysis.intruder.dictionaryPicker.lastType'

export function loadIntruderDictionaryPickerType(): string {
  try {
    return localStorage.getItem(INTRUDER_DICTIONARY_PICKER_TYPE_STORAGE_KEY)?.trim() || ''
  } catch {
    return ''
  }
}

export function saveIntruderDictionaryPickerType(dictType: string) {
  try {
    const normalizedType = dictType.trim()
    if (!normalizedType) {
      localStorage.removeItem(INTRUDER_DICTIONARY_PICKER_TYPE_STORAGE_KEY)
      return
    }
    localStorage.setItem(INTRUDER_DICTIONARY_PICKER_TYPE_STORAGE_KEY, normalizedType)
  } catch {
    // Ignore localStorage failures.
  }
}
