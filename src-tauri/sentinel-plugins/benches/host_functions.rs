#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_plugins::plugin_context::PluginContext;

fn host_function_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("host_functions");

    group.bench_function("sentinel_log", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"__sentinel_log("debug", "benchmark message");"#,
                ))
                .unwrap();
        });
    });

    group.bench_function("sentinel_return", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"__sentinel_return({ success: true, data: { count: 42 } });"#,
                ))
                .unwrap();
            ctx.take_last_result();
        });
    });

    group.bench_function("sentinel_emit_finding", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        ctx.set_plugin_id(Some("bench-plugin".to_string()));
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"__sentinel_emit_finding({ vuln_type: "test", title: "bench", description: "d", severity: "info", confidence: "high" });"#,
                ))
                .unwrap();
            ctx.take_findings();
        });
    });

    group.bench_function("sentinel_return_large_payload", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        runtime
            .eval(
                r#"
            var __large_result = { success: true, data: { items: [] } };
            for (var i = 0; i < 100; i++) {
                __large_result.data.items.push({
                    id: i,
                    name: "item_" + i,
                    value: Math.random(),
                    tags: ["tag1", "tag2", "tag3"]
                });
            }
        "#,
            )
            .unwrap();

        b.iter(|| {
            runtime
                .eval(black_box("__sentinel_return(__large_result);"))
                .unwrap();
            ctx.take_last_result();
        });
    });

    group.finish();
}

fn js_eval_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("js_eval");

    group.bench_function("eval_empty", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime.eval(black_box("")).unwrap();
        });
    });

    group.bench_function("eval_simple_expression", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime.eval(black_box("var x = 1 + 2 * 3;")).unwrap();
        });
    });

    group.bench_function("eval_string_ops", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"var s = "hello world".split(" ").map(function(w) { return w.toUpperCase(); }).join("-");"#,
                ))
                .unwrap();
        });
    });

    group.bench_function("eval_regex_match", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"var m = "SELECT * FROM users WHERE id = 1".match(/SELECT\s+.*?\s+FROM/gi);"#,
                ))
                .unwrap();
        });
    });

    group.bench_function("eval_loop_1000", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    "var __sum = 0; for (var __i = 0; __i < 1000; __i++) { __sum += __i; }",
                ))
                .unwrap();
        });
    });

    group.bench_function("eval_object_creation", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"var __arr = []; for (var __j = 0; __j < 100; __j++) { __arr.push({ id: __j, name: "item" + __j }); }"#,
                ))
                .unwrap();
        });
    });

    group.bench_function("eval_json_stringify_parse", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        runtime
            .eval(r#"var __obj = { a: 1, b: "hello", c: [1,2,3], d: { nested: true } };"#)
            .unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    "var __rt = JSON.parse(JSON.stringify(__obj));",
                ))
                .unwrap();
        });
    });

    group.bench_function("eval_promise_resolve", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(
                &sentinel_plugins::permissions::PluginPermissions::full_access(),
            )
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .eval(black_box(
                    r#"(async function() { var r = await Promise.resolve(42); })()"#,
                ))
                .unwrap();
            runtime.execute_pending_jobs().unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    host_function_overhead,
    js_eval_overhead
);
criterion_main!(benches);
