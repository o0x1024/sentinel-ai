// Sentinel Plugin Runtime Polyfills
// 提供插件运行时所需的标准 Web API polyfill
// 这些 polyfill 仅在 deno_core 环境中使用，因为 deno_core 不包含完整的 Web API

(function () {
    "use strict";

    // ============================================================
    // TextDecoder / TextEncoder
    // ============================================================
    if (typeof TextDecoder === "undefined") {
        class SimpleTextDecoder {
            constructor(label = "utf-8", options) {
                this.encoding = label.toLowerCase();
            }
            decode(input) {
                if (input == null) {
                    return "";
                }
                if (input instanceof Uint8Array) {
                    let s = "";
                    for (let i = 0; i < input.length; i++) {
                        s += String.fromCharCode(input[i]);
                    }
                    try {
                        return decodeURIComponent(escape(s));
                    } catch (_) {
                        return s;
                    }
                }
                return String(input);
            }
        }
        globalThis.TextDecoder = SimpleTextDecoder;
    }

    if (typeof TextEncoder === "undefined") {
        class SimpleTextEncoder {
            constructor() { }
            encode(input) {
                input = input == null ? "" : String(input);
                const utf8 = unescape(encodeURIComponent(input));
                const arr = new Uint8Array(utf8.length);
                for (let i = 0; i < utf8.length; i++) {
                    arr[i] = utf8.charCodeAt(i);
                }
                return arr;
            }
        }
        globalThis.TextEncoder = SimpleTextEncoder;
    }

    // ============================================================
    // URLSearchParams
    // ============================================================
    if (typeof URLSearchParams === "undefined") {
        class SimpleURLSearchParams {
            constructor(init) {
                this._params = [];
                if (!init) return;
                let query = typeof init === "string" ? init : "";
                if (query.startsWith("?")) {
                    query = query.substring(1);
                }
                if (query.length === 0) return;
                const pairs = query.split("&");
                for (const pair of pairs) {
                    if (!pair) continue;
                    const [k, v = ""] = pair.split("=");
                    const key = decodeURIComponent(k.replace(/\+/g, " "));
                    const value = decodeURIComponent(v.replace(/\+/g, " "));
                    this._params.push([key, value]);
                }
            }
            append(name, value) {
                this._params.push([String(name), String(value)]);
            }
            delete(name) {
                name = String(name);
                this._params = this._params.filter(([k]) => k !== name);
            }
            get(name) {
                name = String(name);
                for (const [k, v] of this._params) {
                    if (k === name) return v;
                }
                return null;
            }
            getAll(name) {
                name = String(name);
                const res = [];
                for (const [k, v] of this._params) {
                    if (k === name) res.push(v);
                }
                return res;
            }
            has(name) {
                name = String(name);
                for (const [k] of this._params) {
                    if (k === name) return true;
                }
                return false;
            }
            set(name, value) {
                name = String(name);
                value = String(value);
                let found = false;
                this._params = this._params.filter(([k, v]) => {
                    if (k === name) {
                        if (!found) {
                            found = true;
                            return true;
                        }
                        return false;
                    }
                    return true;
                });
                if (found) {
                    for (let i = 0; i < this._params.length; i++) {
                        if (this._params[i][0] === name) {
                            this._params[i][1] = value;
                            break;
                        }
                    }
                } else {
                    this._params.push([name, value]);
                }
            }
            sort() {
                this._params.sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
            }
            toString() {
                return this._params
                    .map(([k, v]) => encodeURIComponent(k) + "=" + encodeURIComponent(v))
                    .join("&");
            }
            forEach(callback, thisArg) {
                for (const [k, v] of this._params) {
                    callback.call(thisArg, v, k, this);
                }
            }
            entries() {
                return this._params[Symbol.iterator]();
            }
            keys() {
                return this._params.map(([k]) => k)[Symbol.iterator]();
            }
            values() {
                return this._params.map(([, v]) => v)[Symbol.iterator]();
            }
            [Symbol.iterator]() {
                return this._params[Symbol.iterator]();
            }
            get size() {
                return this._params.length;
            }
        }
        globalThis.URLSearchParams = SimpleURLSearchParams;
    }

    // ============================================================
    // URL
    // ============================================================
    if (typeof URL === "undefined") {
        class SimpleURL {
            constructor(input, base) {
                let url = String(input);
                if (base) {
                    if (!/^[a-zA-Z][a-zA-Z0-9+\-.]*:/.test(url)) {
                        const b = String(base);
                        if (b.endsWith("/") && !url.startsWith("/")) {
                            url = b + url;
                        } else {
                            url = b.replace(/\/+$/, "") + "/" + url.replace(/^\/+/, "");
                        }
                    }
                }
                this._originalHref = url;
                const m = url.match(/^(https?:)(\/\/([^\/?#]*))?([^?#]*)(\?[^#]*)?(#.*)?$/i);
                this.protocol = m && m[1] ? m[1].toLowerCase() : "";
                this.host = m && m[3] ? m[3] : "";
                const hostParts = this.host.split(":");
                this.hostname = hostParts[0] || "";
                this.port = hostParts[1] || "";
                this.pathname = m && m[4] ? (m[4] || "/") : "/";
                this.hash = m && m[6] ? m[6] : "";
                this.origin = this.protocol && this.host ? this.protocol + "//" + this.host : "";
                const searchWithoutQ = (m && m[5]) ? (m[5].startsWith("?") ? m[5].substring(1) : m[5]) : "";
                this.searchParams = new globalThis.URLSearchParams(searchWithoutQ);
            }
            get search() {
                const s = this.searchParams.toString();
                return s ? "?" + s : "";
            }
            get href() {
                return this.origin + this.pathname + this.search + this.hash;
            }
            toString() {
                return this.href;
            }
        }
        globalThis.URL = SimpleURL;
    }

    // ============================================================
    // setTimeout / setInterval
    // ============================================================
    if (typeof setTimeout === "undefined") {
        const _timers = new Map();
        let _timerId = 0;

        globalThis.setTimeout = function (callback, delay = 0, ...args) {
            const id = ++_timerId;
            const startTime = Date.now();
            _timers.set(id, { callback, delay, args, startTime, cleared: false });

            (async () => {
                const timer = _timers.get(id);
                if (timer && !timer.cleared) {
                    const elapsed = Date.now() - timer.startTime;
                    if (elapsed < timer.delay) {
                        await new Promise(r => queueMicrotask(r));
                    }
                    if (!timer.cleared) {
                        timer.callback(...timer.args);
                    }
                    _timers.delete(id);
                }
            })();
            return id;
        };

        globalThis.clearTimeout = function (id) {
            const timer = _timers.get(id);
            if (timer) {
                timer.cleared = true;
                _timers.delete(id);
            }
        };

        globalThis.setInterval = function (callback, delay = 0, ...args) {
            return globalThis.setTimeout(callback, delay, ...args);
        };

        globalThis.clearInterval = globalThis.clearTimeout;
    }

    // ============================================================
    // AbortController / AbortSignal
    // ============================================================
    if (typeof AbortController === "undefined") {
        class SimpleAbortSignal {
            constructor() {
                this.aborted = false;
                this.reason = undefined;
                this._listeners = [];
            }
            addEventListener(type, listener) {
                if (type === 'abort') this._listeners.push(listener);
            }
            removeEventListener(type, listener) {
                if (type === 'abort') {
                    this._listeners = this._listeners.filter(l => l !== listener);
                }
            }
            throwIfAborted() {
                if (this.aborted) throw this.reason;
            }
        }

        class SimpleAbortController {
            constructor() {
                this.signal = new SimpleAbortSignal();
            }
            abort(reason) {
                if (this.signal.aborted) return;
                this.signal.aborted = true;
                this.signal.reason = reason || new Error('AbortError');
                for (const listener of this.signal._listeners) {
                    try { listener({ type: 'abort', target: this.signal }); } catch (_) { }
                }
            }
        }

        globalThis.AbortController = SimpleAbortController;
        globalThis.AbortSignal = SimpleAbortSignal;
    }

    // ============================================================
    // Fetch API (使用 Deno.core.ops.op_fetch)
    // ============================================================
    if (typeof fetch === "undefined") {
        class SimpleHeaders {
            constructor(init) {
                this._map = {};
                if (init && typeof init === 'object') {
                    if (typeof init.forEach === 'function') {
                        init.forEach((value, key) => { this._map[key] = value; });
                    } else {
                        Object.assign(this._map, init);
                    }
                }
            }
            get(name) {
                const target = String(name).toLowerCase();
                for (const [k, v] of Object.entries(this._map)) {
                    if (String(k).toLowerCase() === target) return String(v);
                }
                return null;
            }
            set(name, value) {
                this._map[String(name)] = String(value);
            }
            has(name) {
                const target = String(name).toLowerCase();
                for (const k of Object.keys(this._map)) {
                    if (String(k).toLowerCase() === target) return true;
                }
                return false;
            }
            delete(name) {
                const target = String(name).toLowerCase();
                for (const k of Object.keys(this._map)) {
                    if (String(k).toLowerCase() === target) {
                        delete this._map[k];
                        return;
                    }
                }
            }
            forEach(callback, thisArg) {
                for (const [k, v] of Object.entries(this._map)) {
                    callback.call(thisArg, String(v), k, this);
                }
            }
            entries() { return Object.entries(this._map)[Symbol.iterator](); }
            keys() { return Object.keys(this._map)[Symbol.iterator](); }
            values() { return Object.values(this._map)[Symbol.iterator](); }
            [Symbol.iterator]() { return Object.entries(this._map)[Symbol.iterator](); }
        }

        globalThis.Headers = SimpleHeaders;

        globalThis.fetch = async function (url, options = {}) {
            let headersObj = {};
            const h = options.headers;
            if (h instanceof SimpleHeaders) {
                h.forEach((value, key) => { headersObj[key] = value; });
            } else if (h && typeof h.forEach === "function") {
                h.forEach((value, key) => { headersObj[String(key)] = String(value); });
            } else if (h && typeof h === "object") {
                headersObj = { ...h };
            }

            const fetchOptions = {
                method: options.method || "GET",
                headers: headersObj,
                body: options.body || null,
                timeout: options.timeout || 30000,
            };

            const response = await Deno.core.ops.op_fetch(String(url), fetchOptions);

            if (!response.success || response.error) {
                throw new Error(`Fetch failed: ${response.error || 'Unknown error'}`);
            }

            return {
                ok: response.ok,
                status: response.status,
                statusText: response.statusText || "",
                headers: new SimpleHeaders(response.headers || {}),
                url: String(url),
                redirected: false,
                type: "basic",
                text: async () => response.body,
                json: async () => JSON.parse(response.body),
                arrayBuffer: async () => new TextEncoder().encode(response.body).buffer,
                blob: async () => new Blob([response.body]),
                clone: function () { return this; },
                body: response.body,
            };
        };
    }

    // ============================================================
    // console (基础实现)
    // ============================================================
    if (typeof console === "undefined" || typeof console.log !== "function") {
        const _console = {
            log: (...args) => { Deno.core.ops.op_plugin_log("info", args.map(String).join(" ")); },
            info: (...args) => { Deno.core.ops.op_plugin_log("info", args.map(String).join(" ")); },
            warn: (...args) => { Deno.core.ops.op_plugin_log("warn", args.map(String).join(" ")); },
            error: (...args) => { Deno.core.ops.op_plugin_log("error", args.map(String).join(" ")); },
            debug: (...args) => { Deno.core.ops.op_plugin_log("debug", args.map(String).join(" ")); },
            trace: (...args) => { Deno.core.ops.op_plugin_log("trace", args.map(String).join(" ")); },
        };
        globalThis.console = _console;
    }

    // ============================================================
    // atob / btoa (Base64)
    // ============================================================
    if (typeof atob === "undefined") {
        const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";

        globalThis.btoa = function (data) {
            const str = String(data);
            let output = "";
            for (let i = 0; i < str.length; i += 3) {
                const a = str.charCodeAt(i);
                const b = str.charCodeAt(i + 1);
                const c = str.charCodeAt(i + 2);
                const enc1 = a >> 2;
                const enc2 = ((a & 3) << 4) | (b >> 4);
                const enc3 = isNaN(b) ? 64 : ((b & 15) << 2) | (c >> 6);
                const enc4 = isNaN(c) ? 64 : c & 63;
                output += chars[enc1] + chars[enc2] + chars[enc3] + chars[enc4];
            }
            return output;
        };

        globalThis.atob = function (data) {
            const str = String(data).replace(/=+$/, "");
            let output = "";
            for (let i = 0; i < str.length; i += 4) {
                const enc1 = chars.indexOf(str[i]);
                const enc2 = chars.indexOf(str[i + 1]);
                const enc3 = chars.indexOf(str[i + 2]);
                const enc4 = chars.indexOf(str[i + 3]);
                const a = (enc1 << 2) | (enc2 >> 4);
                const b = ((enc2 & 15) << 4) | (enc3 >> 2);
                const c = ((enc3 & 3) << 6) | enc4;
                output += String.fromCharCode(a);
                if (enc3 !== 64) output += String.fromCharCode(b);
                if (enc4 !== 64) output += String.fromCharCode(c);
            }
            return output;
        };
    }

    // ============================================================
    // Sentinel Plugin API
    // ============================================================
    globalThis.Sentinel = {
        emitFinding: function (finding) {
            Deno.core.ops.op_emit_finding(finding);
        },
        log: function (level, message) {
            Deno.core.ops.op_plugin_log(level, message);
        }
    };

})();
