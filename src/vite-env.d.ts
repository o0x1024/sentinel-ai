/// <reference types="vite/client" />

import 'vue-router'

declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    standalone?: boolean
    requiresLicense?: boolean
  }
}

interface Window {
  __VUE_LAST_ERROR__?: {
    message: string
    componentName: string
    info: string
    stack?: string
    timestamp: number
  }
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
