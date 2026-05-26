export function readText(path) {
    return __sentinel_read_text_file(path);
}

export function writeText(path, content) {
    return __sentinel_write_text_file(path, content);
}

export function read(path) {
    return __sentinel_read_file(path);
}

export function write(path, data) {
    return __sentinel_write_file(path, data);
}

export function mkdir(path, options) {
    var recursive = false;
    if (options && options.recursive) {
        recursive = true;
    }
    return __sentinel_mkdir(path, recursive);
}

export function readDir(path) {
    return __sentinel_read_dir(path);
}

export function stat(path) {
    return __sentinel_stat(path);
}

export function copyFile(src, dst) {
    return __sentinel_copy_file(src, dst);
}

export function remove(path, options) {
    var recursive = false;
    if (options && options.recursive) {
        recursive = true;
    }
    return __sentinel_remove(path, recursive);
}

export function makeTempFile(options) {
    var prefix = "sentinel_";
    var suffix = ".tmp";
    if (options) {
        if (options.prefix) prefix = options.prefix;
        if (options.suffix) suffix = options.suffix;
    }
    return __sentinel_make_temp_file(prefix, suffix);
}
