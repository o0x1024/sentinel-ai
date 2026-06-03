import { DefineComponent } from 'vue'
import { Composer } from 'vue-i18n'

declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $t: (key: string, ...args: any[]) => string
    $tc: (key: string, choice?: number, ...args: any[]) => string
    $te: (key: string) => boolean
    $d: (value: number | Date, ...args: any[]) => string
    $n: (value: number, ...args: any[]) => string
  }
}

declare module 'vue' {
  interface ComponentCustomProperties {
    $t: (key: string, ...args: any[]) => string
    $tc: (key: string, choice?: number, ...args: any[]) => string
    $te: (key: string) => boolean
    $d: (value: number | Date, ...args: any[]) => string
    $n: (value: number, ...args: any[]) => string
  }
}

export {}