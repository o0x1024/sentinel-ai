//! setTimeout / setInterval / clearTimeout / clearInterval implementation.

use rquickjs::Ctx;
use crate::error::{JsError, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Install timer functions via JS that work with QuickJS's Promise/microtask queue.
    // Since our runtime is synchronous, setTimeout callbacks fire during the next
    // event loop pump (via Promise.resolve().then(...)).
    let timer_code = r#"
(function(globalThis) {
    "use strict";
    var __nextTimerId = 1;
    var __timers = {};

    globalThis.setTimeout = function(callback, delay) {
        var id = __nextTimerId++;
        var args = Array.prototype.slice.call(arguments, 2);
        __timers[id] = true;
        Promise.resolve().then(function() {
            if (__timers[id]) {
                delete __timers[id];
                callback.apply(null, args);
            }
        });
        return id;
    };

    globalThis.setInterval = function(callback, delay) {
        // In plugin runtime, setInterval fires once (plugins shouldn't rely on repeated intervals)
        return globalThis.setTimeout(callback, delay);
    };

    globalThis.clearTimeout = function(id) {
        delete __timers[id];
    };

    globalThis.clearInterval = function(id) {
        delete __timers[id];
    };

    globalThis.queueMicrotask = function(callback) {
        Promise.resolve().then(callback);
    };
})(globalThis);
"#;

    ctx.eval::<(), _>(timer_code)
        .map_err(|e| JsError::Internal(format!("timers install failed: {e}")))?;

    Ok(())
}
