// Web-standard fetch, Headers, Request, Response implementation.
// Delegates actual network I/O to __host_fetch (registered by the Rust plugin engine).

(function(globalThis) {
"use strict";

// --- Headers ---
function Headers(init) {
    this._headers = {};
    if (init) {
        if (init instanceof Headers) {
            var entries = init.entries();
            for (var i = 0; i < entries.length; i++) {
                this._headers[entries[i][0]] = entries[i][1];
            }
        } else if (Array.isArray(init)) {
            for (var j = 0; j < init.length; j++) {
                this._headers[init[j][0].toLowerCase()] = init[j][1];
            }
        } else if (typeof init === "object") {
            var keys = Object.keys(init);
            for (var k = 0; k < keys.length; k++) {
                this._headers[keys[k].toLowerCase()] = String(init[keys[k]]);
            }
        }
    }
}
Headers.prototype.get = function(name) {
    return this._headers[name.toLowerCase()] || null;
};
Headers.prototype.set = function(name, value) {
    this._headers[name.toLowerCase()] = String(value);
};
Headers.prototype.has = function(name) {
    return name.toLowerCase() in this._headers;
};
Headers.prototype.delete = function(name) {
    delete this._headers[name.toLowerCase()];
};
Headers.prototype.append = function(name, value) {
    var key = name.toLowerCase();
    if (this._headers[key]) {
        this._headers[key] += ", " + value;
    } else {
        this._headers[key] = String(value);
    }
};
Headers.prototype.forEach = function(callback, thisArg) {
    var keys = Object.keys(this._headers);
    for (var i = 0; i < keys.length; i++) {
        callback.call(thisArg, this._headers[keys[i]], keys[i], this);
    }
};
Headers.prototype.entries = function() {
    var keys = Object.keys(this._headers);
    var result = [];
    for (var i = 0; i < keys.length; i++) {
        result.push([keys[i], this._headers[keys[i]]]);
    }
    return result;
};
Headers.prototype.keys = function() { return Object.keys(this._headers); };
Headers.prototype.values = function() { return Object.values(this._headers); };
Headers.prototype[Symbol.iterator] = function() {
    var entries = this.entries(), idx = 0;
    return { next: function() {
        if (idx >= entries.length) return { done: true };
        return { done: false, value: entries[idx++] };
    }};
};

// --- Response ---
function Response(body, init) {
    init = init || {};
    this._body = body || "";
    this.status = init.status || 200;
    this.statusText = init.statusText || "OK";
    this.ok = this.status >= 200 && this.status < 300;
    this.headers = new Headers(init.headers);
    this.url = init.url || "";
    this.redirected = init.redirected || false;
    this.type = "basic";
    this._bodyUsed = false;
}
Response.prototype.text = function() {
    this._bodyUsed = true;
    return Promise.resolve(typeof this._body === "string" ? this._body : String(this._body));
};
Response.prototype.json = function() {
    return this.text().then(function(t) { return JSON.parse(t); });
};
Response.prototype.arrayBuffer = function() {
    this._bodyUsed = true;
    var str = typeof this._body === "string" ? this._body : String(this._body);
    var encoder = new TextEncoder();
    var bytes = encoder.encode(str);
    return Promise.resolve(bytes.buffer);
};
Response.prototype.clone = function() {
    return new Response(this._body, {
        status: this.status,
        statusText: this.statusText,
        headers: this.headers,
        url: this.url,
        redirected: this.redirected
    });
};
Object.defineProperty(Response.prototype, "body", {
    get: function() { return null; } // ReadableStream not fully supported
});
Object.defineProperty(Response.prototype, "bodyUsed", {
    get: function() { return this._bodyUsed; }
});

// --- Request ---
function Request(input, init) {
    init = init || {};
    if (typeof input === "string") {
        this.url = input;
    } else if (input instanceof Request) {
        this.url = input.url;
        init.method = init.method || input.method;
        init.headers = init.headers || input.headers;
        init.body = init.body || input._body;
    }
    this.method = (init.method || "GET").toUpperCase();
    this.headers = new Headers(init.headers);
    this._body = init.body || null;
    this.redirect = init.redirect || "follow";
}
Request.prototype.text = function() {
    return Promise.resolve(this._body || "");
};
Request.prototype.json = function() {
    return this.text().then(function(t) { return JSON.parse(t); });
};
Request.prototype.clone = function() {
    return new Request(this.url, {
        method: this.method,
        headers: this.headers,
        body: this._body,
        redirect: this.redirect
    });
};

// --- fetch batching infrastructure ---
// Queues fetch requests within a single synchronous JS turn and flushes them
// concurrently via __sentinel_fetch_batch on the next microtask tick.  This
// makes Promise.all(urls.map(u => fetch(u))) truly concurrent without any
// plugin-side changes.

var __fetchQueue = [];
var __fetchFlushScheduled = false;

function __settleHostResult(hostResult, url, resolve, reject) {
    if (!hostResult || !hostResult.success) {
        reject(new TypeError((hostResult && hostResult.error) || "Network request failed"));
        return;
    }
    resolve(new Response(hostResult.body, {
        status: hostResult.status,
        statusText: hostResult.ok ? "OK" : "Error",
        headers: hostResult.headers,
        url: hostResult.final_url || url,
        redirected: hostResult.redirected || false
    }));
}

function __flushFetchQueue() {
    var batch = __fetchQueue;
    __fetchQueue = [];
    __fetchFlushScheduled = false;
    if (batch.length === 0) return;

    if (typeof globalThis.__sentinel_fetch_batch === "function" && batch.length > 1) {
        var requests = [];
        for (var bi = 0; bi < batch.length; bi++) {
            requests.push(batch[bi].request);
        }
        try {
            var results = globalThis.__sentinel_fetch_batch(requests);
            for (var ri = 0; ri < batch.length; ri++) {
                __settleHostResult(results[ri], batch[ri].request.url, batch[ri].resolve, batch[ri].reject);
            }
        } catch (e) {
            for (var ei = 0; ei < batch.length; ei++) {
                batch[ei].reject(new TypeError("fetch batch failed: " + (e.message || e)));
            }
        }
    } else {
        for (var si = 0; si < batch.length; si++) {
            var entry = batch[si];
            try {
                var hostResult = globalThis.__host_fetch(entry.request.url, entry.request.options);
                __settleHostResult(hostResult, entry.request.url, entry.resolve, entry.reject);
            } catch (e) {
                entry.reject(new TypeError("fetch failed: " + (e.message || e)));
            }
        }
    }
}

// --- fetch ---
function fetch(input, init) {
    init = init || {};
    var url, method, hdrs, body, redirect;

    if (input instanceof Request) {
        url = input.url;
        method = init.method || input.method;
        hdrs = init.headers ? new Headers(init.headers) : input.headers;
        body = init.body || input._body;
        redirect = init.redirect || input.redirect;
    } else {
        url = String(input);
        method = (init.method || "GET").toUpperCase();
        hdrs = new Headers(init.headers);
        body = init.body || null;
        redirect = init.redirect || "follow";
    }

    var headerObj = {};
    hdrs.forEach(function(value, name) { headerObj[name] = value; });

    var bodyPayload = null;
    if (typeof body === "string") {
        bodyPayload = { kind: "text", text: body };
    }

    var requestOptions = {
        method: method,
        headers: headerObj,
        body: bodyPayload,
        redirect: redirect,
        max_redirects: init.maxRedirects,
        max_body_bytes: init.maxBodyBytes
    };

    return new Promise(function(resolve, reject) {
        __fetchQueue.push({
            request: { url: url, options: requestOptions },
            resolve: resolve,
            reject: reject
        });
        if (!__fetchFlushScheduled) {
            __fetchFlushScheduled = true;
            Promise.resolve().then(__flushFetchQueue);
        }
    });
}

globalThis.Headers = Headers;
globalThis.Request = Request;
globalThis.Response = Response;
globalThis.fetch = fetch;

})(globalThis);
