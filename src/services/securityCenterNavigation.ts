const DEFAULT_SECURITY_CENTER_LOCATION = '/security-center/workbench'
const SECURITY_CENTER_LOCATION_KEY = 'security-center:last-location:v1'

const isSecurityCenterLocation = (value: unknown): value is string =>
  typeof value === 'string' && value.startsWith('/security-center')

export const rememberSecurityCenterLocation = (fullPath: string) => {
  if (!isSecurityCenterLocation(fullPath)) return
  try {
    window.localStorage.setItem(SECURITY_CENTER_LOCATION_KEY, fullPath)
  } catch (error) {
    console.warn('Failed to persist security center location', error)
  }
}

export const resolveLastSecurityCenterLocation = () => {
  try {
    const saved = window.localStorage.getItem(SECURITY_CENTER_LOCATION_KEY)
    return isSecurityCenterLocation(saved) ? saved : DEFAULT_SECURITY_CENTER_LOCATION
  } catch (error) {
    console.warn('Failed to load security center location', error)
    return DEFAULT_SECURITY_CENTER_LOCATION
  }
}
