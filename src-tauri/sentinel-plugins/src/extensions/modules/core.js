export function log(level, message) {
    __sentinel_log(level, message);
}

export function emitFinding(finding) {
    __sentinel_emit_finding(finding);
}

export function resolve(value) {
    __sentinel_return(value);
}
