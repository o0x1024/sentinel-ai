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

// File system operations (custom ops)
// No import needed, ops are available via Deno.core.ops

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

// ============================================================
// Deno File System API (Custom Implementation via ops)
// ============================================================

/**
 * Read text file
 * @param {string} path - File path
 * @returns {Promise<string>} File content
 */
globalThis.Deno.readTextFile = async function(path) {
  return await Deno.core.ops.op_read_text_file(path)
}

/**
 * Write text file
 * @param {string} path - File path
 * @param {string} data - File content
 */
globalThis.Deno.writeTextFile = async function(path, data) {
  return await Deno.core.ops.op_write_text_file(path, data)
}

/**
 * Read binary file
 * @param {string} path - File path
 * @returns {Promise<Uint8Array>} File content
 */
globalThis.Deno.readFile = async function(path) {
  return await Deno.core.ops.op_read_file(path)
}

/**
 * Write binary file
 * @param {string} path - File path
 * @param {Uint8Array} data - File content
 */
globalThis.Deno.writeFile = async function(path, data) {
  const bytes = data instanceof Uint8Array ? Array.from(data) : data
  return await Deno.core.ops.op_write_file(path, bytes)
}

/**
 * Create directory
 * @param {string} path - Directory path
 * @param {object} options - Options { recursive: boolean }
 */
globalThis.Deno.mkdir = async function(path, options = {}) {
  const recursive = options.recursive || false
  return await Deno.core.ops.op_mkdir(path, recursive)
}

/**
 * Read directory contents (async iterator)
 * @param {string} path - Directory path
 * @returns {AsyncIterable<{name: string, isFile: boolean, isDirectory: boolean, isSymlink: boolean}>}
 */
globalThis.Deno.readDir = async function*(path) {
  const entries = await Deno.core.ops.op_read_dir(path)
  for (const entry of entries) {
    yield entry
  }
}

/**
 * Get file info
 * @param {string} path - File path
 * @returns {Promise<{size: number, isFile: boolean, isDirectory: boolean, isSymlink: boolean, mtime?: number}>}
 */
globalThis.Deno.stat = async function(path) {
  return await Deno.core.ops.op_stat(path)
}

/**
 * Copy file
 * @param {string} from - Source file path
 * @param {string} to - Destination file path
 */
globalThis.Deno.copyFile = async function(from, to) {
  return await Deno.core.ops.op_copy_file(from, to)
}

/**
 * Remove file or directory
 * @param {string} path - File or directory path
 * @param {object} options - Options { recursive: boolean }
 */
globalThis.Deno.remove = async function(path, options = {}) {
  const recursive = options.recursive || false
  return await Deno.core.ops.op_remove(path, recursive)
}

/**
 * Create temporary file
 * @param {object} options - Options { prefix?: string, suffix?: string }
 * @returns {Promise<string>} Temporary file path
 */
globalThis.Deno.makeTempFile = async function(options = {}) {
  const prefix = options.prefix || 'sentinel_'
  const suffix = options.suffix || '.tmp'
  return await Deno.core.ops.op_make_temp_file(prefix, suffix)
}

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

// ============================================================
// Node.js Compatibility Layer
// ============================================================
// This allows plugins to use Node.js-style APIs directly,
// reducing prompt token usage and LLM hallucinations.

// process object (Node.js global)
globalThis.process = {
  env: {},
  pid: Deno.pid,
  platform: Deno.build.os,
  arch: Deno.build.arch,
  version: 'v18.0.0',
  versions: {
    node: '18.0.0',
    v8: Deno.version.v8,
  },
  cwd: () => '/',
  exit: (code) => {
    throw new Error(`process.exit(${code}) called`)
  },
  nextTick: (callback, ...args) => {
    queueMicrotask(() => callback(...args))
  },
}

// require() function (simplified CommonJS module loader)
globalThis.require = function(moduleName) {
  // fs module (file system)
  if (moduleName === 'fs' || moduleName === 'node:fs') {
    return {
      readFile: (path, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        const encoding = typeof options === 'string' ? options : options?.encoding
        Deno.readFile(path).then(data => {
          if (encoding === 'utf8' || encoding === 'utf-8') {
            callback(null, new TextDecoder().decode(data))
          } else {
            callback(null, Buffer.from(data))
          }
        }).catch(err => callback(err))
      },
      
      writeFile: (path, data, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        const content = typeof data === 'string' ? new TextEncoder().encode(data) : data
        Deno.writeFile(path, content).then(() => callback(null)).catch(err => callback(err))
      },
      
      readFileSync: (path, options) => {
        throw new Error('Synchronous file operations not supported. Use fs.promises or async callbacks.')
      },
      
      writeFileSync: (path, data, options) => {
        throw new Error('Synchronous file operations not supported. Use fs.promises or async callbacks.')
      },
      
      mkdir: (path, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        Deno.mkdir(path, { recursive: options?.recursive || false })
          .then(() => callback(null))
          .catch(err => callback(err))
      },
      
      readdir: (path, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        Deno.core.ops.op_read_dir(path)
          .then(entries => callback(null, entries.map(e => e.name)))
          .catch(err => callback(err))
      },
      
      stat: (path, callback) => {
        Deno.stat(path)
          .then(info => callback(null, {
            isFile: () => info.isFile,
            isDirectory: () => info.isDirectory,
            isSymbolicLink: () => info.isSymlink,
            size: info.size,
            mtime: info.mtime ? new Date(info.mtime) : null,
          }))
          .catch(err => callback(err))
      },
      
      unlink: (path, callback) => {
        Deno.remove(path).then(() => callback(null)).catch(err => callback(err))
      },
      
      rmdir: (path, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        Deno.remove(path, { recursive: options?.recursive || false })
          .then(() => callback(null))
          .catch(err => callback(err))
      },
      
      copyFile: (src, dest, callback) => {
        Deno.copyFile(src, dest).then(() => callback(null)).catch(err => callback(err))
      },
      
      // fs.promises API (modern async/await style)
      promises: {
        readFile: async (path, options) => {
          const encoding = typeof options === 'string' ? options : options?.encoding
          const data = await Deno.readFile(path)
          if (encoding === 'utf8' || encoding === 'utf-8') {
            return new TextDecoder().decode(data)
          }
          return Buffer.from(data)
        },
        
        writeFile: async (path, data, options) => {
          const content = typeof data === 'string' ? new TextEncoder().encode(data) : data
          await Deno.writeFile(path, content)
        },
        
        mkdir: async (path, options) => {
          await Deno.mkdir(path, { recursive: options?.recursive || false })
        },
        
        readdir: async (path, options) => {
          const entries = await Deno.core.ops.op_read_dir(path)
          return entries.map(e => e.name)
        },
        
        stat: async (path) => {
          const info = await Deno.stat(path)
          return {
            isFile: () => info.isFile,
            isDirectory: () => info.isDirectory,
            isSymbolicLink: () => info.isSymlink,
            size: info.size,
            mtime: info.mtime ? new Date(info.mtime) : null,
          }
        },
        
        unlink: async (path) => {
          await Deno.remove(path)
        },
        
        rmdir: async (path, options) => {
          await Deno.remove(path, { recursive: options?.recursive || false })
        },
        
        copyFile: async (src, dest) => {
          await Deno.copyFile(src, dest)
        },
      },
    }
  }
  
  // path module
  if (moduleName === 'path' || moduleName === 'node:path') {
    return {
      join: (...paths) => paths.join('/').replace(/\/+/g, '/'),
      resolve: (...paths) => '/' + paths.join('/').replace(/^\/+/, '').replace(/\/+/g, '/'),
      dirname: (path) => path.substring(0, path.lastIndexOf('/')) || '/',
      basename: (path, ext) => {
        const base = path.substring(path.lastIndexOf('/') + 1)
        return ext && base.endsWith(ext) ? base.slice(0, -ext.length) : base
      },
      extname: (path) => {
        const base = path.substring(path.lastIndexOf('/') + 1)
        const dotIndex = base.lastIndexOf('.')
        return dotIndex > 0 ? base.substring(dotIndex) : ''
      },
      sep: '/',
      delimiter: ':',
    }
  }
  
  // crypto module
  if (moduleName === 'crypto' || moduleName === 'node:crypto') {
    return {
      randomBytes: (size) => {
        const bytes = new Uint8Array(size)
        crypto.getRandomValues(bytes)
        return Buffer.from(bytes)
      },
      
      createHash: (algorithm) => {
        let data = new Uint8Array(0)
        const hashObj = {
          update: function(chunk) {
            const newData = new Uint8Array(data.length + chunk.length)
            newData.set(data)
            newData.set(typeof chunk === 'string' ? new TextEncoder().encode(chunk) : chunk, data.length)
            data = newData
            return hashObj
          },
          digest: async function(encoding) {
            const algoMap = {
              'md5': 'MD5',
              'sha1': 'SHA-1',
              'sha256': 'SHA-256',
              'sha512': 'SHA-512',
            }
            const algo = algoMap[algorithm.toLowerCase()]
            if (!algo) throw new Error(`Unsupported hash algorithm: ${algorithm}`)
            
            // MD5 is not supported by Web Crypto API, throw error
            if (algorithm.toLowerCase() === 'md5') {
              throw new Error('MD5 is not supported. Use sha256 or sha512 instead.')
            }
            
            const hashBuffer = await crypto.subtle.digest(algo, data)
            const hashArray = new Uint8Array(hashBuffer)
            
            if (encoding === 'hex') {
              return Array.from(hashArray).map(b => b.toString(16).padStart(2, '0')).join('')
            } else if (encoding === 'base64') {
              return btoa(String.fromCharCode(...hashArray))
            }
            return Buffer.from(hashArray)
          },
        }
        return hashObj
      },
      
      randomUUID: () => crypto.randomUUID(),
    }
  }
  
  // http/https modules (simplified, use fetch instead)
  if (moduleName === 'http' || moduleName === 'https' || moduleName === 'node:http' || moduleName === 'node:https') {
    const httpModule = {
      request: (urlOrOptions, optionsOrCallback, callbackOrUndefined) => {
        let url, options, callback
        
        // Parse arguments (Node.js http.request has multiple signatures)
        if (typeof urlOrOptions === 'string') {
          url = urlOrOptions
          if (typeof optionsOrCallback === 'function') {
            callback = optionsOrCallback
            options = {}
          } else {
            options = optionsOrCallback || {}
            callback = callbackOrUndefined
          }
        } else {
          options = urlOrOptions || {}
          callback = optionsOrCallback
          // Construct URL from options
          const protocol = options.protocol || 'http:'
          const hostname = options.hostname || options.host || 'localhost'
          const port = options.port || (protocol === 'https:' ? 443 : 80)
          const path = options.path || '/'
          url = `${protocol}//${hostname}${port !== 80 && port !== 443 ? ':' + port : ''}${path}`
        }
        
        const eventHandlers = {
          error: [],
          timeout: [],
        }
        
        let bodyData = null
        let isEnded = false
        let timeoutId = null
        
        const req = {
          write: (data) => {
            if (isEnded) {
              throw new Error('Cannot write to request after end() is called')
            }
            if (bodyData === null) {
              bodyData = data
            } else {
              bodyData = bodyData + data
            }
            return true
          },
          
          end: async (data) => {
            if (isEnded) return
            isEnded = true
            
            if (data) {
              if (bodyData === null) {
                bodyData = data
              } else {
                bodyData = bodyData + data
              }
            }
            
            // Set up timeout
            if (options.timeout) {
              timeoutId = setTimeout(() => {
                eventHandlers.timeout.forEach(handler => handler())
              }, options.timeout)
            }
            
            try {
              const response = await fetch(url, {
                method: options.method || 'GET',
                headers: options.headers || {},
                body: bodyData,
                timeout: options.timeout,
              })
              
              // Clear timeout on success
              if (timeoutId) clearTimeout(timeoutId)
              
              let responseBody = ''
              const resEventHandlers = {
                data: [],
                end: [],
              }
              
              const res = {
                statusCode: response.status,
                statusMessage: response.statusText,
                headers: Object.fromEntries(response.headers.entries()),
                
                setEncoding: (encoding) => {
                  // Encoding is handled automatically
                },
                
                on: (event, handler) => {
                  if (event === 'data' || event === 'end') {
                    resEventHandlers[event].push(handler)
                  }
                  return res
                },
              }
              
              // Fetch response body
              responseBody = await response.text()
              
              // Call callback first to allow event listeners to be registered
              if (callback) callback(res)
              
              // Use nextTick/queueMicrotask to emit events after callback returns
              // This ensures event listeners are registered before events fire
              queueMicrotask(() => {
                // Emit 'data' event
                resEventHandlers.data.forEach(handler => handler(responseBody))
                
                // Emit 'end' event
                resEventHandlers.end.forEach(handler => handler())
              })
            } catch (err) {
              // Clear timeout on error
              if (timeoutId) clearTimeout(timeoutId)
              
              // Emit 'error' event
              eventHandlers.error.forEach(handler => handler(err))
            }
          },
          
          on: (event, handler) => {
            if (event === 'error' || event === 'timeout') {
              eventHandlers[event].push(handler)
            }
            return req
          },
          
          destroy: () => {
            if (timeoutId) clearTimeout(timeoutId)
            isEnded = true
          },
          
          setTimeout: (timeout, callback) => {
            options.timeout = timeout
            if (callback) {
              eventHandlers.timeout.push(callback)
            }
            return req
          },
        }
        
        return req
      },
      
      get: (url, options, callback) => {
        if (typeof options === 'function') {
          callback = options
          options = {}
        }
        const req = httpModule.request(url, { ...options, method: 'GET' }, callback)
        req.end()
        return req
      },
    }
    
    return httpModule
  }
  
  // util module
  if (moduleName === 'util' || moduleName === 'node:util') {
    return {
      promisify: globalThis.promisify,
      inspect: Deno.inspect,
    }
  }
  
  // os module
  if (moduleName === 'os' || moduleName === 'node:os') {
    return {
      platform: () => Deno.build.os,
      arch: () => Deno.build.arch,
      hostname: Deno.hostname,
      tmpdir: () => '/tmp',
      homedir: () => '/home',
      EOL: '\n',
    }
  }
  
  // url module
  if (moduleName === 'url' || moduleName === 'node:url') {
    return {
      URL: globalThis.URL,
      URLSearchParams: globalThis.URLSearchParams,
      parse: (urlString) => {
        const url = new URL(urlString)
        return {
          protocol: url.protocol,
          hostname: url.hostname,
          port: url.port,
          pathname: url.pathname,
          search: url.search,
          hash: url.hash,
          href: url.href,
        }
      },
    }
  }
  
  // querystring module
  if (moduleName === 'querystring' || moduleName === 'node:querystring') {
    return {
      parse: (str) => Object.fromEntries(new URLSearchParams(str)),
      stringify: (obj) => new URLSearchParams(obj).toString(),
    }
  }
  
  // buffer module
  if (moduleName === 'buffer' || moduleName === 'node:buffer') {
    return {
      Buffer: globalThis.Buffer,
    }
  }
  
  throw new Error(`Module not found: ${moduleName}. Supported modules: fs, path, crypto, http, https, util, os, url, querystring, buffer`)
}

// Buffer class (Node.js buffer API)
globalThis.Buffer = class Buffer extends Uint8Array {
  static from(data, encoding) {
    if (typeof data === 'string') {
      if (encoding === 'base64') {
        const binary = atob(data)
        const bytes = new Uint8Array(binary.length)
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i)
        }
        return new Buffer(bytes)
      } else if (encoding === 'hex') {
        const bytes = new Uint8Array(data.length / 2)
        for (let i = 0; i < data.length; i += 2) {
          bytes[i / 2] = parseInt(data.substr(i, 2), 16)
        }
        return new Buffer(bytes)
      } else {
        return new Buffer(new TextEncoder().encode(data))
      }
    } else if (data instanceof ArrayBuffer || data instanceof Uint8Array) {
      return new Buffer(data)
    } else if (Array.isArray(data)) {
      return new Buffer(new Uint8Array(data))
    }
    throw new Error('Unsupported data type for Buffer.from()')
  }
  
  static alloc(size, fill) {
    const buf = new Buffer(size)
    if (fill !== undefined) {
      buf.fill(fill)
    }
    return buf
  }
  
  static concat(list, totalLength) {
    if (totalLength === undefined) {
      totalLength = list.reduce((acc, buf) => acc + buf.length, 0)
    }
    const result = new Buffer(totalLength)
    let offset = 0
    for (const buf of list) {
      result.set(buf, offset)
      offset += buf.length
    }
    return result
  }
  
  toString(encoding) {
    if (encoding === 'base64') {
      return btoa(String.fromCharCode(...this))
    } else if (encoding === 'hex') {
      return Array.from(this).map(b => b.toString(16).padStart(2, '0')).join('')
    } else {
      return new TextDecoder().decode(this)
    }
  }
  
  toJSON() {
    return { type: 'Buffer', data: Array.from(this) }
  }
}

// __dirname and __filename (not available in ESM, but provide stubs)
globalThis.__dirname = '/plugin'
globalThis.__filename = '/plugin/index.js'

// module.exports and exports (CommonJS compatibility)
globalThis.module = { exports: {} }
globalThis.exports = globalThis.module.exports

// ============================================================
// Node.js Compatible Console API
// ============================================================
// Override deno_web console with 100% Node.js compatible implementation
// This ensures plugins can use console.log/error/warn/info/debug naturally

// Store original console from deno_web (if needed for debugging)
const _denoConsole = globalThis.console

// Format function to handle multiple arguments like Node.js
const formatArgs = (...args) => {
  return args.map(arg => {
    if (typeof arg === 'string') return arg
    if (arg === null) return 'null'
    if (arg === undefined) return 'undefined'
    if (typeof arg === 'function') return `[Function: ${arg.name || 'anonymous'}]`
    if (typeof arg === 'symbol') return arg.toString()
    if (typeof arg === 'bigint') return arg.toString() + 'n'
    if (arg instanceof Error) return arg.stack || arg.toString()
    if (arg instanceof Date) return arg.toISOString()
    if (arg instanceof RegExp) return arg.toString()
    if (typeof arg === 'object') {
      try {
        return JSON.stringify(arg, (key, value) => {
          // Handle circular references
          if (typeof value === 'function') return `[Function: ${value.name || 'anonymous'}]`
          if (typeof value === 'symbol') return value.toString()
          if (typeof value === 'bigint') return value.toString() + 'n'
          if (value instanceof Error) return value.toString()
          return value
        }, 2)
      } catch (e) {
        // Circular reference or other serialization error
        return '[Object (circular or non-serializable)]'
      }
    }
    return String(arg)
  }).join(' ')
}

// Node.js compatible console implementation
globalThis.console = {
  // Standard logging methods (Node.js compatible)
  log: (...args) => {
    const message = formatArgs(...args)
    Deno.core.ops.op_plugin_log('info', message)
  },
  
  info: (...args) => {
    const message = formatArgs(...args)
    Deno.core.ops.op_plugin_log('info', message)
  },
  
  warn: (...args) => {
    const message = formatArgs(...args)
    Deno.core.ops.op_plugin_log('warn', message)
  },
  
  error: (...args) => {
    const message = formatArgs(...args)
    Deno.core.ops.op_plugin_log('error', message)
  },
  
  debug: (...args) => {
    const message = formatArgs(...args)
    Deno.core.ops.op_plugin_log('debug', message)
  },
  
  // Timing methods (Node.js compatible)
  time: (label = 'default') => {
    if (!globalThis.console._timers) globalThis.console._timers = new Map()
    globalThis.console._timers.set(label, Date.now())
  },
  
  timeEnd: (label = 'default') => {
    if (!globalThis.console._timers) globalThis.console._timers = new Map()
    const start = globalThis.console._timers.get(label)
    if (start !== undefined) {
      const duration = Date.now() - start
      Deno.core.ops.op_plugin_log('info', `${label}: ${duration}ms`)
      globalThis.console._timers.delete(label)
    } else {
      Deno.core.ops.op_plugin_log('warn', `Timer '${label}' does not exist`)
    }
  },
  
  timeLog: (label = 'default', ...args) => {
    if (!globalThis.console._timers) globalThis.console._timers = new Map()
    const start = globalThis.console._timers.get(label)
    if (start !== undefined) {
      const duration = Date.now() - start
      const message = args.length > 0 ? `${label}: ${duration}ms ${formatArgs(...args)}` : `${label}: ${duration}ms`
      Deno.core.ops.op_plugin_log('info', message)
    } else {
      Deno.core.ops.op_plugin_log('warn', `Timer '${label}' does not exist`)
    }
  },
  
  // Assertion (Node.js compatible)
  assert: (condition, ...args) => {
    if (!condition) {
      const message = args.length > 0 ? formatArgs(...args) : 'Assertion failed'
      Deno.core.ops.op_plugin_log('error', `Assertion failed: ${message}`)
      throw new Error(`Assertion failed: ${message}`)
    }
  },
  
  // Counting (Node.js compatible)
  count: (label = 'default') => {
    if (!globalThis.console._counters) globalThis.console._counters = new Map()
    const current = (globalThis.console._counters.get(label) || 0) + 1
    globalThis.console._counters.set(label, current)
    Deno.core.ops.op_plugin_log('info', `${label}: ${current}`)
  },
  
  countReset: (label = 'default') => {
    if (!globalThis.console._counters) globalThis.console._counters = new Map()
    globalThis.console._counters.delete(label)
  },
  
  // Grouping (Node.js compatible - simplified)
  group: (...args) => {
    const message = args.length > 0 ? formatArgs(...args) : 'Group'
    Deno.core.ops.op_plugin_log('info', `[Group] ${message}`)
  },
  
  groupCollapsed: (...args) => {
    const message = args.length > 0 ? formatArgs(...args) : 'Group'
    Deno.core.ops.op_plugin_log('info', `[Group Collapsed] ${message}`)
  },
  
  groupEnd: () => {
    // No-op in our simplified implementation
  },
  
  // Table (Node.js compatible - simplified)
  table: (data, columns) => {
    try {
      const formatted = JSON.stringify(data, null, 2)
      Deno.core.ops.op_plugin_log('info', `Table:\n${formatted}`)
    } catch (e) {
      Deno.core.ops.op_plugin_log('info', `[Table data not serializable]`)
    }
  },
  
  // Trace (Node.js compatible)
  trace: (...args) => {
    const message = args.length > 0 ? formatArgs(...args) : 'Trace'
    const stack = new Error().stack || ''
    Deno.core.ops.op_plugin_log('debug', `Trace: ${message}\n${stack}`)
  },
  
  // Clear (no-op in backend)
  clear: () => {
    // No-op - can't clear terminal in plugin context
  },
  
  // Dir (Node.js compatible - similar to log with object inspection)
  dir: (obj, options = {}) => {
    const depth = options.depth || 2
    try {
      const formatted = JSON.stringify(obj, null, 2)
      Deno.core.ops.op_plugin_log('info', formatted)
    } catch (e) {
      Deno.core.ops.op_plugin_log('info', '[Object not serializable]')
    }
  },
  
  // DirXml (alias to dir)
  dirxml: (...args) => {
    globalThis.console.dir(...args)
  },
  
  // Internal storage for timers and counters
  _timers: new Map(),
  _counters: new Map(),
}


