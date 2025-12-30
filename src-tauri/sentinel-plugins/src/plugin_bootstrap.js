/* global Deno, sleep */
// Sentinel Deno runtime bootstrap (evaluated during JsRuntime::new via esm_entry_point)
//
// IMPORTANT:
// - Must NOT use top-level await (deno_core evaluates extension entry points synchronously).
// - Must import extension ESM modules so deno_core debug checks won't panic with
//   "Following modules were not evaluated".

// WebIDL first
import 'ext:deno_webidl/00_webidl.js'

// deno_web (keep in sync with deno_web's `esm = [...]` list)
import 'ext:deno_web/00_infra.js'
import 'ext:deno_web/01_dom_exception.js'
import 'ext:deno_web/01_mimesniff.js'
import 'ext:deno_web/02_event.js'
import 'ext:deno_web/02_structured_clone.js'
import 'ext:deno_web/02_timers.js'
import 'ext:deno_web/03_abort_signal.js'
import 'ext:deno_web/04_global_interfaces.js'
import 'ext:deno_web/06_streams.js'
import 'ext:deno_web/09_file.js'
import 'ext:deno_web/10_filereader.js'
import 'ext:deno_web/12_location.js'
import 'ext:deno_web/13_message_port.js'
import 'ext:deno_web/14_compression.js'
import 'ext:deno_web/15_performance.js'
import 'ext:deno_web/16_image_data.js'
import 'ext:deno_web/01_urlpattern.js'
import 'ext:deno_web/01_console.js'
import 'ext:deno_web/01_broadcast_channel.js'

// deno_web exports that are NOT automatically installed on globalThis in deno_core embeddings
import { atob, btoa } from 'ext:deno_web/05_base64.js'
import {
  TextDecoder,
  TextEncoder,
  TextDecoderStream,
  TextEncoderStream,
} from 'ext:deno_web/08_text_encoding.js'
import { URL, URLSearchParams } from 'ext:deno_web/00_url.js'
import {
  setTimeout,
  setInterval,
  clearTimeout,
  clearInterval,
} from 'ext:deno_web/02_timers.js'
import { Event, EventTarget, ErrorEvent, CloseEvent, MessageEvent, CustomEvent, ProgressEvent } from 'ext:deno_web/02_event.js'
import { AbortController, AbortSignal } from 'ext:deno_web/03_abort_signal.js'
import { ReadableStream, WritableStream, TransformStream, ByteLengthQueuingStrategy, CountQueuingStrategy } from 'ext:deno_web/06_streams.js'
import { Blob, File } from 'ext:deno_web/09_file.js'
import { DOMException } from 'ext:deno_web/01_dom_exception.js'
import { BroadcastChannel } from 'ext:deno_web/01_broadcast_channel.js'
import { CompressionStream, DecompressionStream } from 'ext:deno_web/14_compression.js'
import { performance, Performance, PerformanceEntry, PerformanceMark, PerformanceMeasure } from 'ext:deno_web/15_performance.js'

// deno_net
import * as net from 'ext:deno_net/01_net.js'
import * as tls from 'ext:deno_net/02_tls.js'

// deno_crypto (Web Crypto)
import { crypto, Crypto, SubtleCrypto } from 'ext:deno_crypto/00_crypto.js'

// Sentinel plugin API
globalThis.Sentinel = {
  emitFinding: (finding) => {
    Deno.core.ops.op_emit_finding(finding)
  },
  log: (level, message) => {
    Deno.core.ops.op_plugin_log(level, message)
  },
}

// Install Web APIs on globalThis (deno_web provides implementations but does not
// always wire them to globalThis outside of the full Deno runtime bootstrap).
// Text encoding
if (typeof globalThis.TextEncoder === 'undefined') globalThis.TextEncoder = TextEncoder
if (typeof globalThis.TextDecoder === 'undefined') globalThis.TextDecoder = TextDecoder
if (typeof globalThis.TextEncoderStream === 'undefined') globalThis.TextEncoderStream = TextEncoderStream
if (typeof globalThis.TextDecoderStream === 'undefined') globalThis.TextDecoderStream = TextDecoderStream

// URL APIs
if (typeof globalThis.URL === 'undefined') globalThis.URL = URL
if (typeof globalThis.URLSearchParams === 'undefined') globalThis.URLSearchParams = URLSearchParams

// Timer APIs (critical for plugins)
if (typeof globalThis.setTimeout === 'undefined') globalThis.setTimeout = setTimeout
if (typeof globalThis.setInterval === 'undefined') globalThis.setInterval = setInterval
if (typeof globalThis.clearTimeout === 'undefined') globalThis.clearTimeout = clearTimeout
if (typeof globalThis.clearInterval === 'undefined') globalThis.clearInterval = clearInterval
// Note: queueMicrotask is provided by V8 directly, not from deno_web

// Headers API - Custom implementation (deno_web doesn't export Headers directly)
if (typeof globalThis.Headers === 'undefined') {
  globalThis.Headers = class Headers {
    constructor(init) {
      this._headers = new Map();
      if (init) {
        if (init instanceof Headers) {
          init.forEach((value, key) => this.set(key, value));
        } else if (Array.isArray(init)) {
          init.forEach(([key, value]) => this.set(key, value));
        } else if (typeof init === 'object') {
          Object.entries(init).forEach(([key, value]) => this.set(key, value));
        }
      }
    }
    
    append(name, value) {
      const existing = this._headers.get(name.toLowerCase());
      if (existing) {
        this._headers.set(name.toLowerCase(), `${existing}, ${value}`);
      } else {
        this._headers.set(name.toLowerCase(), String(value));
      }
    }
    
    delete(name) {
      this._headers.delete(name.toLowerCase());
    }
    
    get(name) {
      return this._headers.get(name.toLowerCase()) || null;
    }
    
    has(name) {
      return this._headers.has(name.toLowerCase());
    }
    
    set(name, value) {
      this._headers.set(name.toLowerCase(), String(value));
    }
    
    forEach(callback, thisArg) {
      this._headers.forEach((value, key) => {
        callback.call(thisArg, value, key, this);
      });
    }
    
    keys() {
      return this._headers.keys();
    }
    
    values() {
      return this._headers.values();
    }
    
    entries() {
      return this._headers.entries();
    }
    
    [Symbol.iterator]() {
      return this._headers.entries();
    }
  };
}

// Base64
if (typeof globalThis.atob === 'undefined') globalThis.atob = atob
if (typeof globalThis.btoa === 'undefined') globalThis.btoa = btoa

// Crypto
if (typeof globalThis.crypto === 'undefined') globalThis.crypto = crypto
if (typeof globalThis.Crypto === 'undefined') globalThis.Crypto = Crypto
if (typeof globalThis.SubtleCrypto === 'undefined') globalThis.SubtleCrypto = SubtleCrypto

// Events
if (typeof globalThis.Event === 'undefined') globalThis.Event = Event
if (typeof globalThis.EventTarget === 'undefined') globalThis.EventTarget = EventTarget
if (typeof globalThis.ErrorEvent === 'undefined') globalThis.ErrorEvent = ErrorEvent
if (typeof globalThis.CloseEvent === 'undefined') globalThis.CloseEvent = CloseEvent
if (typeof globalThis.MessageEvent === 'undefined') globalThis.MessageEvent = MessageEvent
if (typeof globalThis.CustomEvent === 'undefined') globalThis.CustomEvent = CustomEvent
if (typeof globalThis.ProgressEvent === 'undefined') globalThis.ProgressEvent = ProgressEvent

// Abort
if (typeof globalThis.AbortController === 'undefined') globalThis.AbortController = AbortController
if (typeof globalThis.AbortSignal === 'undefined') globalThis.AbortSignal = AbortSignal

// Streams
if (typeof globalThis.ReadableStream === 'undefined') globalThis.ReadableStream = ReadableStream
if (typeof globalThis.WritableStream === 'undefined') globalThis.WritableStream = WritableStream
if (typeof globalThis.TransformStream === 'undefined') globalThis.TransformStream = TransformStream
if (typeof globalThis.ByteLengthQueuingStrategy === 'undefined') globalThis.ByteLengthQueuingStrategy = ByteLengthQueuingStrategy
if (typeof globalThis.CountQueuingStrategy === 'undefined') globalThis.CountQueuingStrategy = CountQueuingStrategy

// File APIs
if (typeof globalThis.Blob === 'undefined') globalThis.Blob = Blob
if (typeof globalThis.File === 'undefined') globalThis.File = File

// Other Web APIs
if (typeof globalThis.DOMException === 'undefined') globalThis.DOMException = DOMException
if (typeof globalThis.BroadcastChannel === 'undefined') globalThis.BroadcastChannel = BroadcastChannel
if (typeof globalThis.CompressionStream === 'undefined') globalThis.CompressionStream = CompressionStream
if (typeof globalThis.DecompressionStream === 'undefined') globalThis.DecompressionStream = DecompressionStream

// Performance
if (typeof globalThis.performance === 'undefined') globalThis.performance = performance
if (typeof globalThis.Performance === 'undefined') globalThis.Performance = Performance
if (typeof globalThis.PerformanceEntry === 'undefined') globalThis.PerformanceEntry = PerformanceEntry
if (typeof globalThis.PerformanceMark === 'undefined') globalThis.PerformanceMark = PerformanceMark
if (typeof globalThis.PerformanceMeasure === 'undefined') globalThis.PerformanceMeasure = PerformanceMeasure

// Minimal Deno namespace (required by many Deno-land libraries like deno_mongo)
globalThis.Deno = globalThis.Deno || {}

// Deno.build (platform info - required by deno_mongo and other libs)
globalThis.Deno.build = {
  target: 'unknown-unknown-unknown',
  arch: 'x86_64', // Will be replaced by Rust at runtime
  os: 'linux',    // Will be replaced by Rust at runtime
  vendor: 'unknown',
  env: undefined,
}

// Deno.version (version info)
globalThis.Deno.version = {
  deno: '1.0.0',
  v8: '12.0.0',
  typescript: '5.0.0',
}

// Deno.env (environment variables - stub for security)
globalThis.Deno.env = {
  get: (key) => undefined,
  set: (key, value) => {},
  delete: (key) => {},
  has: (key) => false,
  toObject: () => ({}),
}

// Deno.args (command line arguments - empty for plugins)
globalThis.Deno.args = []

// Deno.pid (process ID - stub)
globalThis.Deno.pid = 0

// Deno.ppid (parent process ID - stub)
globalThis.Deno.ppid = 0

// Deno.noColor (disable color output - default false)
globalThis.Deno.noColor = false

// Deno.isatty (check if stdout/stderr is a TTY - stub)
globalThis.Deno.isatty = (rid) => false

// Deno.hostname (get hostname - stub)
globalThis.Deno.hostname = () => 'localhost'

// Deno.osRelease (OS release version - stub)
globalThis.Deno.osRelease = () => '0.0.0'

// Deno.osUptime (OS uptime in seconds - stub)
globalThis.Deno.osUptime = () => 0

// Deno.loadavg (load average - stub)
globalThis.Deno.loadavg = () => [0, 0, 0]

// Deno.networkInterfaces (network interfaces - stub)
globalThis.Deno.networkInterfaces = () => []

// Deno.systemMemoryInfo (memory info - stub)
globalThis.Deno.systemMemoryInfo = () => ({ total: 0, free: 0, available: 0, buffers: 0, cached: 0, swapTotal: 0, swapFree: 0 })

// Deno.uid (user ID - stub, returns null for security)
globalThis.Deno.uid = () => null

// Deno.gid (group ID - stub, returns null for security)
globalThis.Deno.gid = () => null

// Deno.permissions (permission API - all granted for plugin context)
globalThis.Deno.permissions = {
  query: async (desc) => ({ state: 'granted' }),
  revoke: async (desc) => ({ state: 'granted' }),
  request: async (desc) => ({ state: 'granted' }),
}

// Utility functions for security testing
globalThis.Deno.inspect = (value, options) => {
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

// setTimeout/setInterval/clearTimeout/clearInterval (already provided by deno_web timers)
// These are critical for rate limiting, retries, timeouts in security testing

// Deno networking APIs
globalThis.Deno.connect = net.connect
globalThis.Deno.listen = net.listen
globalThis.Deno.resolveDns = net.resolveDns
globalThis.Deno.connectTls = tls.connectTls
globalThis.Deno.listenTls = tls.listenTls
globalThis.Deno.startTls = tls.startTls

// Deno.core (already available, but ensure it's exposed)
globalThis.Deno.core = globalThis.Deno.core || Deno.core

// Fetch polyfill (custom op based) with enhanced features for security testing
globalThis.fetch = async function (input, init = {}) {
  const url = typeof input === 'string' ? input : input.url
  const method = init.method || (input.method || 'GET')
  const headers = {}

  if (init.headers) {
    if (init.headers instanceof Headers) {
      init.headers.forEach((v, k) => (headers[k] = v))
    } else if (Array.isArray(init.headers)) {
      init.headers.forEach(([k, v]) => (headers[k] = v))
    } else {
      Object.assign(headers, init.headers)
    }
  }

  const body = init.body || null
  const timeout = init.timeout || 30000
  const result = await Deno.core.ops.op_fetch(url, { method, headers, body, timeout })

  if (!result.success) {
    throw new Error(result.error || 'Fetch failed')
  }

  return {
    ok: result.ok,
    status: result.status,
    statusText: result.ok ? 'OK' : 'Error',
    headers: new Headers(Object.entries(result.headers)),
    text: async () => result.body,
    json: async () => JSON.parse(result.body),
    arrayBuffer: async () => new TextEncoder().encode(result.body).buffer,
    blob: async () => new Blob([result.body]),
    formData: async () => {
      throw new Error('FormData parsing not implemented')
    },
    clone: function() {
      return {
        ok: this.ok,
        status: this.status,
        statusText: this.statusText,
        headers: new Headers(this.headers),
        text: this.text,
        json: this.json,
        arrayBuffer: this.arrayBuffer,
        blob: this.blob,
      }
    },
  }
}

// Request constructor polyfill (for compatibility with fetch-based libraries)
if (typeof globalThis.Request === 'undefined') {
  globalThis.Request = class Request {
    constructor(input, init = {}) {
      this.url = typeof input === 'string' ? input : input.url
      this.method = init.method || 'GET'
      this.headers = new Headers(init.headers || {})
      this.body = init.body || null
      this.mode = init.mode || 'cors'
      this.credentials = init.credentials || 'same-origin'
      this.cache = init.cache || 'default'
      this.redirect = init.redirect || 'follow'
      this.referrer = init.referrer || ''
      this.integrity = init.integrity || ''
    }
    
    clone() {
      return new Request(this.url, {
        method: this.method,
        headers: this.headers,
        body: this.body,
        mode: this.mode,
        credentials: this.credentials,
        cache: this.cache,
        redirect: this.redirect,
        referrer: this.referrer,
        integrity: this.integrity,
      })
    }
  }
}

// Response constructor polyfill (for compatibility with fetch-based libraries)
if (typeof globalThis.Response === 'undefined') {
  globalThis.Response = class Response {
    constructor(body, init = {}) {
      this.body = body
      this.status = init.status || 200
      this.statusText = init.statusText || 'OK'
      this.headers = new Headers(init.headers || {})
      this.ok = this.status >= 200 && this.status < 300
      this.redirected = false
      this.type = 'default'
      this.url = ''
    }
    
    async text() {
      return String(this.body || '')
    }
    
    async json() {
      return JSON.parse(String(this.body || '{}'))
    }
    
    async arrayBuffer() {
      return new TextEncoder().encode(String(this.body || '')).buffer
    }
    
    async blob() {
      return new Blob([String(this.body || '')])
    }
    
    clone() {
      return new Response(this.body, {
        status: this.status,
        statusText: this.statusText,
        headers: this.headers,
      })
    }
    
    static json(data, init = {}) {
      return new Response(JSON.stringify(data), {
        ...init,
        headers: {
          'content-type': 'application/json',
          ...(init.headers || {}),
        },
      })
    }
    
    static redirect(url, status = 302) {
      return new Response(null, {
        status,
        headers: { location: url },
      })
    }
    
    static error() {
      const r = new Response(null, { status: 0, statusText: '' })
      r.type = 'error'
      return r
    }
  }
}

// Headers polyfill enhancements (if needed)
if (typeof globalThis.Headers !== 'undefined' && !globalThis.Headers.prototype.getSetCookie) {
  globalThis.Headers.prototype.getSetCookie = function() {
    const cookies = []
    this.forEach((value, key) => {
      if (key.toLowerCase() === 'set-cookie') {
        cookies.push(value)
      }
    })
    return cookies
  }
}

// Utility: sleep/delay function (critical for rate limiting in security testing)
globalThis.sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms))
globalThis.delay = globalThis.sleep

// Utility: promisify callback-based functions
globalThis.promisify = (fn) => {
  return (...args) => {
    return new Promise((resolve, reject) => {
      fn(...args, (err, result) => {
        if (err) reject(err)
        else resolve(result)
      })
    })
  }
}

// Utility: retry with exponential backoff (useful for fuzzing/brute force with rate limits)
globalThis.retry = async (fn, options = {}) => {
  const {
    maxAttempts = 3,
    initialDelay = 1000,
    maxDelay = 30000,
    backoffFactor = 2,
    onRetry = null,
  } = options
  
  let lastError
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn()
    } catch (error) {
      lastError = error
      if (attempt === maxAttempts) break
      
      const delayMs = Math.min(initialDelay * Math.pow(backoffFactor, attempt - 1), maxDelay)
      if (onRetry) onRetry(error, attempt, delayMs)
      await sleep(delayMs)
    }
  }
  throw lastError
}

// Utility: timeout wrapper (critical for preventing hung requests in security testing)
globalThis.timeout = (promise, ms, timeoutError) => {
  return Promise.race([
    promise,
    new Promise((_, reject) =>
      setTimeout(() => reject(timeoutError || new Error(`Timeout after ${ms}ms`)), ms)
    ),
  ])
}

// Utility: chunk array (useful for batch processing in fuzzing/scanning)
globalThis.chunk = (array, size) => {
  const chunks = []
  for (let i = 0; i < array.length; i += size) {
    chunks.push(array.slice(i, i + size))
  }
  return chunks
}

// Utility: parallel execution with concurrency limit (critical for rate-limited scanning)
globalThis.parallelLimit = async (tasks, limit) => {
  const results = []
  const executing = []
  
  for (const task of tasks) {
    const p = Promise.resolve().then(() => task())
    results.push(p)
    
    if (limit <= tasks.length) {
      const e = p.then(() => executing.splice(executing.indexOf(e), 1))
      executing.push(e)
      if (executing.length >= limit) {
        await Promise.race(executing)
      }
    }
  }
  
  return Promise.all(results)
}

// Security Testing Utilities
globalThis.SecurityUtils = {
  // URL encoding/decoding
  urlEncode: (str) => encodeURIComponent(str),
  urlDecode: (str) => decodeURIComponent(str),
  
  // HTML entity encoding/decoding
  htmlEncode: (str) => {
    const div = { innerHTML: '' }
    const text = String(str)
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;')
  },
  
  htmlDecode: (str) => {
    return String(str)
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&quot;/g, '"')
      .replace(/&#x27;/g, "'")
      .replace(/&#39;/g, "'")
  },
  
  // Hex encoding/decoding
  hexEncode: (str) => {
    const bytes = new TextEncoder().encode(str)
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
  },
  
  hexDecode: (hex) => {
    const bytes = new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)))
    return new TextDecoder().decode(bytes)
  },
  
  // Unicode escape
  unicodeEscape: (str) => {
    return String(str).split('').map(char => {
      const code = char.charCodeAt(0)
      return code > 127 ? '\\u' + code.toString(16).padStart(4, '0') : char
    }).join('')
  },
  
  // Generate random string (useful for CSRF tokens, session IDs, etc.)
  randomString: (length = 16, charset = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789') => {
    const randomValues = new Uint8Array(length)
    crypto.getRandomValues(randomValues)
    return Array.from(randomValues).map(x => charset[x % charset.length]).join('')
  },
  
  // Generate random bytes
  randomBytes: (length) => {
    const bytes = new Uint8Array(length)
    crypto.getRandomValues(bytes)
    return bytes
  },
  
  // MD5 hash (note: use crypto.subtle.digest for SHA-256/SHA-512)
  // For MD5, users should use deno.land/std/crypto or similar
  
  // Parse cookies
  parseCookies: (cookieHeader) => {
    const cookies = {}
    if (!cookieHeader) return cookies
    
    cookieHeader.split(';').forEach(cookie => {
      const [name, ...rest] = cookie.split('=')
      if (name) {
        cookies[name.trim()] = rest.join('=').trim()
      }
    })
    return cookies
  },
  
  // Build cookie header
  buildCookieHeader: (cookies) => {
    return Object.entries(cookies)
      .map(([name, value]) => `${name}=${value}`)
      .join('; ')
  },
  
  // Parse query string
  parseQuery: (queryString) => {
    const params = new URLSearchParams(queryString)
    const result = {}
    for (const [key, value] of params) {
      result[key] = value
    }
    return result
  },
  
  // Build query string
  buildQuery: (params) => {
    return new URLSearchParams(params).toString()
  },
  
  // Extract URLs from text
  extractUrls: (text) => {
    const urlRegex = /(https?:\/\/[^\s<>"{}|\\^`[\]]+)/gi
    return (text.match(urlRegex) || [])
  },
  
  // Extract emails from text
  extractEmails: (text) => {
    const emailRegex = /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g
    return (text.match(emailRegex) || [])
  },
  
  // Extract IPs from text
  extractIPs: (text) => {
    const ipRegex = /\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b/g
    return (text.match(ipRegex) || [])
  },
  

  
}


