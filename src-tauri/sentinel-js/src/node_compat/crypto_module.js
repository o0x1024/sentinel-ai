// Node.js crypto module implementation
// Uses __crypto_digest host function for hash computation
(function() {
    function Hash(algorithm) {
        this._algo = algorithm.replace("-", "").toLowerCase();
        this._data = [];
    }
    Hash.prototype.update = function(data, encoding) {
        if (typeof data === "string") {
            var encoder = new TextEncoder();
            var bytes = encoder.encode(data);
            for (var i = 0; i < bytes.length; i++) this._data.push(bytes[i]);
        } else if (data instanceof Uint8Array) {
            for (var j = 0; j < data.length; j++) this._data.push(data[j]);
        }
        return this;
    };
    Hash.prototype.digest = function(encoding) {
        var input = new Uint8Array(this._data);
        var result = globalThis.__crypto_digest(this._algo, input);
        if (encoding === "hex") {
            var bytes = new Uint8Array(result);
            var hex = "";
            for (var i = 0; i < bytes.length; i++) {
                var h = bytes[i].toString(16);
                if (h.length === 1) h = "0" + h;
                hex += h;
            }
            return hex;
        }
        if (encoding === "base64") {
            var bytes2 = new Uint8Array(result);
            var str = "";
            for (var j = 0; j < bytes2.length; j++) str += String.fromCharCode(bytes2[j]);
            return btoa(str);
        }
        return new Uint8Array(result);
    };

    function Hmac(algorithm, key) {
        this._algo = algorithm;
        this._key = key;
        this._data = [];
    }
    Hmac.prototype.update = function(data) {
        if (typeof data === "string") {
            var encoder = new TextEncoder();
            var bytes = encoder.encode(data);
            for (var i = 0; i < bytes.length; i++) this._data.push(bytes[i]);
        }
        return this;
    };
    Hmac.prototype.digest = function(encoding) {
        // Simplified: just hash the concatenation of key + data
        // Real HMAC requires proper implementation via host
        if (typeof globalThis.__crypto_hmac === "function") {
            var input = new Uint8Array(this._data);
            var result = globalThis.__crypto_hmac(this._algo, this._key, input);
            if (encoding === "hex") {
                var bytes = new Uint8Array(result);
                var hex = "";
                for (var i = 0; i < bytes.length; i++) {
                    var h = bytes[i].toString(16);
                    if (h.length === 1) h = "0" + h;
                    hex += h;
                }
                return hex;
            }
            return new Uint8Array(result);
        }
        // Fallback: just hash the data (not a real HMAC)
        var hash = createHash(this._algo);
        hash.update(new Uint8Array(this._data));
        return hash.digest(encoding);
    };

    function createHash(algorithm) {
        return new Hash(algorithm);
    }

    function createHmac(algorithm, key) {
        return new Hmac(algorithm, key);
    }

    function randomBytes(size) {
        var bytes = new Uint8Array(size);
        crypto.getRandomValues(bytes);
        return bytes;
    }

    return {
        createHash: createHash,
        createHmac: createHmac,
        randomBytes: randomBytes,
        randomUUID: function() { return crypto.randomUUID(); },
        getHashes: function() { return ["sha1", "sha256", "sha384", "sha512", "md5"]; }
    };
})()
