// Node.js path module implementation
(function() {
    var sep = "/";

    function normalize(path) {
        var parts = path.split(/[\/\\]+/);
        var result = [];
        for (var i = 0; i < parts.length; i++) {
            if (parts[i] === "..") { result.pop(); }
            else if (parts[i] !== "." && parts[i] !== "") { result.push(parts[i]); }
        }
        var normalized = result.join("/");
        if (path.charAt(0) === "/") normalized = "/" + normalized;
        return normalized || ".";
    }

    function join() {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            if (arguments[i]) parts.push(arguments[i]);
        }
        return normalize(parts.join("/"));
    }

    function resolve() {
        var resolved = "";
        for (var i = arguments.length - 1; i >= 0; i--) {
            resolved = arguments[i] + "/" + resolved;
            if (arguments[i].charAt(0) === "/") break;
        }
        return normalize(resolved);
    }

    function basename(path, ext) {
        var base = path.split(/[\/\\]/).pop() || "";
        if (ext && base.endsWith(ext)) {
            base = base.substring(0, base.length - ext.length);
        }
        return base;
    }

    function dirname(path) {
        var parts = path.split(/[\/\\]/);
        parts.pop();
        return parts.join("/") || "/";
    }

    function extname(path) {
        var base = basename(path);
        var dot = base.lastIndexOf(".");
        return dot > 0 ? base.substring(dot) : "";
    }

    function isAbsolute(path) {
        return path.charAt(0) === "/";
    }

    function relative(from, to) {
        var fromParts = resolve(from).split("/").filter(Boolean);
        var toParts = resolve(to).split("/").filter(Boolean);
        var common = 0;
        while (common < fromParts.length && common < toParts.length && fromParts[common] === toParts[common]) {
            common++;
        }
        var ups = [];
        for (var i = common; i < fromParts.length; i++) ups.push("..");
        return ups.concat(toParts.slice(common)).join("/") || ".";
    }

    function parse(path) {
        var dir = dirname(path);
        var base = basename(path);
        var ext = extname(path);
        var name = ext ? base.substring(0, base.length - ext.length) : base;
        return { root: path.charAt(0) === "/" ? "/" : "", dir: dir, base: base, ext: ext, name: name };
    }

    function format(obj) {
        var dir = obj.dir || obj.root || "";
        var base = obj.base || ((obj.name || "") + (obj.ext || ""));
        if (dir && dir !== obj.root) return dir + "/" + base;
        return dir + base;
    }

    return {
        sep: sep,
        delimiter: ":",
        join: join,
        resolve: resolve,
        normalize: normalize,
        basename: basename,
        dirname: dirname,
        extname: extname,
        isAbsolute: isAbsolute,
        relative: relative,
        parse: parse,
        format: format,
        posix: null,
        win32: null
    };
})()
