// WHATWG URL and URLSearchParams polyfill for QuickJS-NG
// Minimal spec-compliant implementation for plugin runtime use.

(function(globalThis) {
"use strict";

// --- URLSearchParams ---
function URLSearchParams(init) {
    this._params = [];
    if (typeof init === "string") {
        var s = init.charAt(0) === "?" ? init.substring(1) : init;
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
        if (Array.isArray(init)) {
            for (var j = 0; j < init.length; j++) {
                this._params.push([String(init[j][0]), String(init[j][1])]);
            }
        } else {
            var keys = Object.keys(init);
            for (var k = 0; k < keys.length; k++) {
                this._params.push([keys[k], String(init[keys[k]])]);
            }
        }
    }
}
URLSearchParams.prototype.get = function(name) {
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) return this._params[i][1];
    }
    return null;
};
URLSearchParams.prototype.getAll = function(name) {
    var result = [];
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) result.push(this._params[i][1]);
    }
    return result;
};
URLSearchParams.prototype.has = function(name) { return this.get(name) !== null; };
URLSearchParams.prototype.set = function(name, value) {
    var found = false;
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) {
            if (!found) { this._params[i][1] = String(value); found = true; }
            else { this._params.splice(i, 1); i--; }
        }
    }
    if (!found) this._params.push([name, String(value)]);
};
URLSearchParams.prototype.append = function(name, value) {
    this._params.push([name, String(value)]);
};
URLSearchParams.prototype["delete"] = function(name) {
    for (var i = 0; i < this._params.length; i++) {
        if (this._params[i][0] === name) { this._params.splice(i, 1); i--; }
    }
};
URLSearchParams.prototype.toString = function() {
    return this._params.map(function(p) {
        return encodeURIComponent(p[0]) + "=" + encodeURIComponent(p[1]);
    }).join("&");
};
URLSearchParams.prototype.forEach = function(callback, thisArg) {
    for (var i = 0; i < this._params.length; i++) {
        callback.call(thisArg, this._params[i][1], this._params[i][0], this);
    }
};
URLSearchParams.prototype.entries = function() { return this._params.slice(); };
URLSearchParams.prototype.keys = function() { return this._params.map(function(p) { return p[0]; }); };
URLSearchParams.prototype.values = function() { return this._params.map(function(p) { return p[1]; }); };
URLSearchParams.prototype[Symbol.iterator] = function() {
    var idx = 0, params = this._params;
    return { next: function() {
        if (idx >= params.length) return { done: true, value: undefined };
        return { done: false, value: params[idx++] };
    }};
};

// --- URL ---
function URL(url, base) {
    var full = String(url);
    if (base) {
        var bstr = String(base instanceof URL ? base.href : base);
        if (full.indexOf("://") === -1 && full.charAt(0) !== "/") {
            if (bstr.charAt(bstr.length - 1) !== "/") bstr += "/";
            full = bstr + full;
        } else if (full.charAt(0) === "/") {
            var m = bstr.match(/^(https?:\/\/[^\/]+)/);
            full = (m ? m[1] : "") + full;
        }
    }
    this.href = full;

    var proto = "", rest = full;
    var protoIdx = full.indexOf("://");
    if (protoIdx !== -1) {
        proto = full.substring(0, protoIdx + 1);
        rest = full.substring(protoIdx + 3);
    }
    this.protocol = proto;

    var hashIdx = rest.indexOf("#");
    this.hash = "";
    if (hashIdx !== -1) { this.hash = rest.substring(hashIdx); rest = rest.substring(0, hashIdx); }

    var qIdx = rest.indexOf("?");
    this.search = "";
    if (qIdx !== -1) { this.search = rest.substring(qIdx); rest = rest.substring(0, qIdx); }
    this.searchParams = new URLSearchParams(this.search);

    var slashIdx = rest.indexOf("/");
    if (slashIdx === -1) { this.host = rest; this.pathname = "/"; }
    else { this.host = rest.substring(0, slashIdx); this.pathname = rest.substring(slashIdx); }

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
}
URL.prototype.toString = function() { return this.href; };
URL.prototype.toJSON = function() { return this.href; };

if (typeof globalThis.URL === "undefined") globalThis.URL = URL;
if (typeof globalThis.URLSearchParams === "undefined") globalThis.URLSearchParams = URLSearchParams;

})(globalThis);
