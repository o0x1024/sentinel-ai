// Sentinel plugin runtime bootstrap (QuickJS)
//
// Plain script evaluated in the QuickJS global scope before plugin code runs.
// Host functions (__sentinel_*) are registered by the Rust plugin runtime and
// are synchronous.

// ============================================================
// Sentinel plugin API
// ============================================================

globalThis.Sentinel = {
    log: function(level, message) {
        __sentinel_log(level, message);
    },
    emitFinding: function(finding) {
        __sentinel_emit_finding(finding);
    },
    "return": function(value) {
        __sentinel_return(value);
    },
    resolve: function(value) {
        __sentinel_return(value);
    },
    TLS: {
        getCertificate: function(hostname, options) {
            var port = 443;
            if (options && options.port) {
                port = options.port;
            }
            return __sentinel_get_tls_certificate(hostname, port, 3000);
        }
    },
    Network: {
        scanPorts: function(request) {
            return __sentinel_scan_ports(request);
        },
        probeServices: function(request) {
            return __sentinel_probe_services(request);
        },
        getServiceProbeCapabilities: function() {
            return __sentinel_get_service_probe_capabilities();
        }
    },
    Monitor: {
        reportProgress: function(request) {
            return __sentinel_report_monitor_progress(request);
        }
    },
    Runtime: {
        getSettings: function() {
            return __sentinel_get_plugin_runtime_settings();
        }
    },
    AST: {
        parse: function(code, filename) {
            return __sentinel_parse_js(code, filename);
        }
    },
    Dictionary: {
        get: function(idOrName) {
            return __sentinel_get_dictionary(idOrName);
        },
        getDefaultId: function(dictType) {
            return __sentinel_get_default_dictionary_id(dictType);
        },
        getWords: function(idOrName, limit) {
            return __sentinel_get_dictionary_words(idOrName, limit);
        },
        getEntries: function(idOrName, limit) {
            return __sentinel_get_dictionary_entries(idOrName, limit);
        },
        list: function(filter) {
            var dictType = null;
            var category = null;
            if (filter) {
                if (filter.dictType) {
                    dictType = filter.dictType;
                }
                if (filter.category) {
                    category = filter.category;
                }
            }
            return __sentinel_list_dictionaries(dictType, category);
        }
    }
};

// ============================================================
// Minimal Deno namespace (compatibility stubs)
// ============================================================

globalThis.Deno = {
    readTextFile: function(path) {
        return __sentinel_read_text_file(path);
    },
    writeTextFile: function(path, content) {
        return __sentinel_write_text_file(path, content);
    },
    readFile: function(path) {
        return __sentinel_read_file(path);
    },
    writeFile: function(path, data) {
        return __sentinel_write_file(path, data);
    },
    mkdir: function(path, options) {
        var recursive = false;
        if (options && options.recursive) {
            recursive = true;
        }
        return __sentinel_mkdir(path, recursive);
    },
    readDir: function(path) {
        return __sentinel_read_dir(path);
    },
    stat: function(path) {
        return __sentinel_stat(path);
    },
    copyFile: function(src, dst) {
        return __sentinel_copy_file(src, dst);
    },
    remove: function(path, options) {
        var recursive = false;
        if (options && options.recursive) {
            recursive = true;
        }
        return __sentinel_remove(path, recursive);
    },
    makeTempFile: function(options) {
        var prefix = "sentinel_";
        var suffix = ".tmp";
        if (options) {
            if (options.prefix) {
                prefix = options.prefix;
            }
            if (options.suffix) {
                suffix = options.suffix;
            }
        }
        return __sentinel_make_temp_file(prefix, suffix);
    },
    build: {
        os: "unknown",
        arch: "unknown"
    },
    version: {
        deno: "1.0.0"
    },
    env: {
        get: function(key) {
            return undefined;
        },
        set: function(key, value) {
        },
        has: function(key) {
            return false;
        },
        toObject: function() {
            return {};
        }
    },
    args: [],
    pid: 0,
    noColor: false,
    inspect: function(value) {
        if (typeof value === "string") {
            return value;
        }
        try {
            return JSON.stringify(value, null, 2);
        } catch (e) {
            return String(value);
        }
    }
};

// ============================================================
// fetch polyfill (simplified)
// ============================================================

globalThis.fetch = function(url, init) {
    if (!init) {
        init = {};
    }
    var method = "GET";
    if (init.method) {
        method = init.method;
    }
    var headers = {};
    if (init.headers) {
        if (typeof init.headers === "object") {
            var keys = Object.keys(init.headers);
            for (var hi = 0; hi < keys.length; hi++) {
                headers[keys[hi]] = init.headers[keys[hi]];
            }
        }
    }
    var body = null;
    if (init.body) {
        if (typeof init.body === "string") {
            body = { kind: "text", text: init.body };
        }
    }
    var redirect = "follow";
    if (init.redirect) {
        redirect = init.redirect;
    }
    var result = __sentinel_fetch(url, {
        method: method,
        headers: headers,
        body: body,
        redirect: redirect,
        max_redirects: init.maxRedirects,
        max_body_bytes: init.maxBodyBytes,
        request_id: null,
        active_probe: null
    });
    if (!result.success) {
        throw new Error(result.error || "Fetch failed");
    }
    var rawHeaders = result.headers || {};
    var headerObj = {
        _raw: rawHeaders,
        get: function(name) {
            var lower = name.toLowerCase();
            var keys = Object.keys(rawHeaders);
            for (var hi = 0; hi < keys.length; hi++) {
                if (keys[hi].toLowerCase() === lower) {
                    return rawHeaders[keys[hi]];
                }
            }
            return null;
        },
        has: function(name) {
            return this.get(name) !== null;
        },
        entries: function() {
            var keys = Object.keys(rawHeaders);
            var result = [];
            for (var hi = 0; hi < keys.length; hi++) {
                result.push([keys[hi].toLowerCase(), rawHeaders[keys[hi]]]);
            }
            return result;
        },
        forEach: function(callback) {
            var keys = Object.keys(rawHeaders);
            for (var hi = 0; hi < keys.length; hi++) {
                callback(rawHeaders[keys[hi]], keys[hi].toLowerCase(), this);
            }
        }
    };
    return {
        ok: result.ok,
        status: result.status,
        statusText: result.ok ? "OK" : "Error",
        headers: headerObj,
        url: result.final_url || url,
        redirected: result.redirected || false,
        text: function() {
            return result.body;
        },
        json: function() {
            return JSON.parse(result.body);
        }
    };
};

// ============================================================
// Console override
// ============================================================

function formatArgs() {
    var parts = [];
    for (var i = 0; i < arguments.length; i++) {
        var arg = arguments[i];
        if (typeof arg === "string") {
            parts.push(arg);
        } else if (arg === null) {
            parts.push("null");
        } else if (arg === undefined) {
            parts.push("undefined");
        } else {
            try {
                parts.push(JSON.stringify(arg));
            } catch (e) {
                parts.push(String(arg));
            }
        }
    }
    return parts.join(" ");
}

globalThis.console = {
    log: function() {
        __sentinel_log("info", formatArgs.apply(null, arguments));
    },
    info: function() {
        __sentinel_log("info", formatArgs.apply(null, arguments));
    },
    warn: function() {
        __sentinel_log("warn", formatArgs.apply(null, arguments));
    },
    error: function() {
        __sentinel_log("error", formatArgs.apply(null, arguments));
    },
    debug: function() {
        __sentinel_log("debug", formatArgs.apply(null, arguments));
    }
};

// ============================================================
// performance polyfill (Date.now-based)
// ============================================================

globalThis.performance = {
    now: function() {
        return Date.now();
    },
    timeOrigin: Date.now()
};

// ============================================================
// Utility functions
// ============================================================

globalThis.sleep = function(ms) {
    return undefined;
};

// ============================================================
// URL / URLSearchParams polyfill
// ============================================================

globalThis.URLSearchParams = function URLSearchParams(init) {
    this._params = [];
    if (typeof init === "string") {
        var s = init;
        if (s.charAt(0) === "?") { s = s.substring(1); }
        var pairs = s.split("&");
        for (var i = 0; i < pairs.length; i++) {
            if (!pairs[i]) continue;
            var eq = pairs[i].indexOf("=");
            if (eq === -1) {
                this._params.push([decodeURIComponent(pairs[i]), ""]);
            } else {
                this._params.push([
                    decodeURIComponent(pairs[i].substring(0, eq)),
                    decodeURIComponent(pairs[i].substring(eq + 1))
                ]);
            }
        }
    } else if (init && typeof init === "object") {
        var keys = Object.keys(init);
        for (var k = 0; k < keys.length; k++) {
            this._params.push([keys[k], String(init[keys[k]])]);
        }
    }
};
globalThis.URLSearchParams.prototype.get = function(name) {
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) return this._params[i][1];
    }
    return null;
};
globalThis.URLSearchParams.prototype.getAll = function(name) {
    var result = [];
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) result.push(this._params[i][1]);
    }
    return result;
};
globalThis.URLSearchParams.prototype.has = function(name) {
    return this.get(name) !== null;
};
globalThis.URLSearchParams.prototype.set = function(name, value) {
    var found = false;
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) {
            if (!found) { this._params[i][1] = String(value); found = true; }
            else { this._params.splice(i, 1); i--; }
        }
    }
    if (!found) this._params.push([name, String(value)]);
};
globalThis.URLSearchParams.prototype.append = function(name, value) {
    this._params.push([name, String(value)]);
};
globalThis.URLSearchParams.prototype["delete"] = function(name) {
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) { this._params.splice(i, 1); i--; }
    }
};
globalThis.URLSearchParams.prototype.toString = function() {
    var parts = [];
    for (var i = 0; i < this._params.length; i++) {
        parts.push(encodeURIComponent(this._params[i][0]) + "=" + encodeURIComponent(this._params[i][1]));
    }
    return parts.join("&");
};
globalThis.URLSearchParams.prototype.forEach = function(callback) {
    for (var i = 0; i < this._params.length; i++) {
        callback(this._params[i][1], this._params[i][0], this);
    }
};
globalThis.URLSearchParams.prototype.entries = function() {
    return this._params.slice();
};
globalThis.URLSearchParams.prototype.keys = function() {
    var result = [];
    for (var i = 0; i < this._params.length; i++) result.push(this._params[i][0]);
    return result;
};
globalThis.URLSearchParams.prototype.values = function() {
    var result = [];
    for (var i = 0; i < this._params.length; i++) result.push(this._params[i][1]);
    return result;
};

globalThis.URL = function URL(url, base) {
    var full = url;
    if (base) {
        if (url.indexOf("://") === -1 && url.charAt(0) !== "/") {
            var bstr = String(base);
            if (bstr.charAt(bstr.length - 1) !== "/") bstr += "/";
            full = bstr + url;
        } else if (url.charAt(0) === "/") {
            var m = String(base).match(/^(https?:\/\/[^\/]+)/);
            full = (m ? m[1] : "") + url;
        }
    }
    this.href = full;

    var proto = "";
    var rest = full;
    var protoIdx = full.indexOf("://");
    if (protoIdx !== -1) {
        proto = full.substring(0, protoIdx + 1);
        rest = full.substring(protoIdx + 3);
    }
    this.protocol = proto;

    var hashIdx = rest.indexOf("#");
    this.hash = "";
    if (hashIdx !== -1) {
        this.hash = rest.substring(hashIdx);
        rest = rest.substring(0, hashIdx);
    }

    var qIdx = rest.indexOf("?");
    this.search = "";
    if (qIdx !== -1) {
        this.search = rest.substring(qIdx);
        rest = rest.substring(0, qIdx);
    }
    this.searchParams = new URLSearchParams(this.search);

    var slashIdx = rest.indexOf("/");
    if (slashIdx === -1) {
        this.host = rest;
        this.pathname = "/";
    } else {
        this.host = rest.substring(0, slashIdx);
        this.pathname = rest.substring(slashIdx);
    }

    var colonIdx = this.host.indexOf(":");
    if (colonIdx !== -1) {
        this.hostname = this.host.substring(0, colonIdx);
        this.port = this.host.substring(colonIdx + 1);
    } else {
        this.hostname = this.host;
        this.port = "";
    }

    this.origin = this.protocol + "//" + this.host;
    this.username = "";
    this.password = "";
};
globalThis.URL.prototype.toString = function() {
    return this.href;
};
globalThis.URL.prototype.toJSON = function() {
    return this.href;
};

// ============================================================
// Node.js require() stub (minimal)
// ============================================================

globalThis.require = function(moduleName) {
    if (moduleName === "path" || moduleName === "node:path") {
        return {
            join: function() {
                var result = "";
                for (var pi = 0; pi < arguments.length; pi++) {
                    if (result) {
                        result = result + "/";
                    }
                    result = result + arguments[pi];
                }
                return result;
            },
            basename: function(path) {
                var parts = path.split("/");
                return parts[parts.length - 1];
            },
            dirname: function(path) {
                var parts = path.split("/");
                parts.pop();
                var joined = parts.join("/");
                if (joined) {
                    return joined;
                }
                return "/";
            },
            extname: function(path) {
                var base = path.split("/").pop();
                var dot = base.lastIndexOf(".");
                if (dot > 0) {
                    return base.substring(dot);
                }
                return "";
            },
            sep: "/"
        };
    }
    if (moduleName === "url" || moduleName === "node:url") {
        return { URL: URL, URLSearchParams: URLSearchParams };
    }
    throw new Error("Module not found: " + moduleName);
};

// ============================================================
// SecurityUtils (simplified)
// ============================================================

// ============================================================
// TextEncoder / TextDecoder polyfill (UTF-8 only)
// ============================================================
if (typeof globalThis.TextEncoder === 'undefined') {
    globalThis.TextEncoder = function TextEncoder() {};
    globalThis.TextEncoder.prototype.encode = function(str) {
        var s = String(str);
        var bytes = [];
        for (var ti = 0; ti < s.length; ti++) {
            var code = s.charCodeAt(ti);
            if (code < 0x80) {
                bytes.push(code);
            } else if (code < 0x800) {
                bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
            } else if (code >= 0xd800 && code <= 0xdbff && ti + 1 < s.length) {
                var lo = s.charCodeAt(ti + 1);
                if (lo >= 0xdc00 && lo <= 0xdfff) {
                    code = ((code - 0xd800) << 10) + (lo - 0xdc00) + 0x10000;
                    ti++;
                    bytes.push(0xf0 | (code >> 18), 0x80 | ((code >> 12) & 0x3f),
                               0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
                    continue;
                }
                bytes.push(0xef, 0xbf, 0xbd);
            } else if (code >= 0xdc00 && code <= 0xdfff) {
                bytes.push(0xef, 0xbf, 0xbd);
            } else {
                bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
            }
        }
        return new Uint8Array(bytes);
    };
}
if (typeof globalThis.TextDecoder === 'undefined') {
    globalThis.TextDecoder = function TextDecoder() {};
    globalThis.TextDecoder.prototype.decode = function(buf) {
        var bytes = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
        var result = "";
        for (var di = 0; di < bytes.length; ) {
            var b = bytes[di];
            var code;
            if (b < 0x80) { code = b; di++; }
            else if ((b & 0xe0) === 0xc0) { code = ((b & 0x1f) << 6) | (bytes[di+1] & 0x3f); di += 2; }
            else if ((b & 0xf0) === 0xe0) { code = ((b & 0x0f) << 12) | ((bytes[di+1] & 0x3f) << 6) | (bytes[di+2] & 0x3f); di += 3; }
            else { code = ((b & 0x07) << 18) | ((bytes[di+1] & 0x3f) << 12) | ((bytes[di+2] & 0x3f) << 6) | (bytes[di+3] & 0x3f); di += 4; }
            if (code > 0xffff) {
                code -= 0x10000;
                result += String.fromCharCode(0xd800 + (code >> 10), 0xdc00 + (code & 0x3ff));
            } else {
                result += String.fromCharCode(code);
            }
        }
        return result;
    };
}

globalThis.SecurityUtils = {
    urlEncode: function(str) {
        return encodeURIComponent(str);
    },
    urlDecode: function(str) {
        return decodeURIComponent(str);
    },
    htmlEncode: function(str) {
        var s = String(str);
        var result = "";
        for (var ei = 0; ei < s.length; ei++) {
            var ch = s.charAt(ei);
            if (ch === "&") {
                result = result + "&amp;";
            } else if (ch === "<") {
                result = result + "&lt;";
            } else if (ch === ">") {
                result = result + "&gt;";
            } else if (ch === "\"") {
                result = result + "&quot;";
            } else {
                result = result + ch;
            }
        }
        return result;
    }
};
