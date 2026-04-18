export const immersiveDrillTrafficTabs = [
  'control',
  'proxyhistory',
  'repeater',
  'comparer',
  'intruder',
  'proxyconfig',
] as const

export const immersiveDrillSecurityTabs = ['workbench', 'vulnerabilities'] as const

export function isImmersiveDrillTrafficTab(tab: string) {
  return immersiveDrillTrafficTabs.includes(
    tab as (typeof immersiveDrillTrafficTabs)[number],
  )
}

export function isImmersiveDrillSecurityTab(tab: string) {
  return immersiveDrillSecurityTabs.includes(
    tab as (typeof immersiveDrillSecurityTabs)[number],
  )
}
