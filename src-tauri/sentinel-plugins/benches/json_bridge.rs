#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_plugins::plugin_context::PluginContext;

fn json_global_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_bridge/set_global");

    group.bench_function("small_json", |b| {
        let json = bench_helpers::make_small_json();
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(&sentinel_plugins::permissions::PluginPermissions::full_access())
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .set_json_global(black_box("__bench_input"), black_box(&json))
                .unwrap();
        });
    });

    group.bench_function("medium_json", |b| {
        let json = bench_helpers::make_medium_json();
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(&sentinel_plugins::permissions::PluginPermissions::full_access())
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .set_json_global(black_box("__bench_input"), black_box(&json))
                .unwrap();
        });
    });

    group.bench_function("large_json", |b| {
        let json = bench_helpers::make_large_json();
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(&sentinel_plugins::permissions::PluginPermissions::full_access())
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        b.iter(|| {
            runtime
                .set_json_global(black_box("__bench_input"), black_box(&json))
                .unwrap();
        });
    });

    group.finish();
}

fn json_serialization_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_bridge/serde_overhead");

    let txn = bench_helpers::make_large_transaction();

    group.bench_function("serialize_transaction", |b| {
        b.iter(|| {
            let json = serde_json::to_value(black_box(&txn)).unwrap();
            black_box(json);
        });
    });

    let json_val = serde_json::to_value(&txn).unwrap();
    let json_str = serde_json::to_string(&json_val).unwrap();

    group.bench_function("serialize_to_string", |b| {
        b.iter(|| {
            let s = serde_json::to_string(black_box(&json_val)).unwrap();
            black_box(s);
        });
    });

    group.bench_function("deserialize_from_string", |b| {
        b.iter(|| {
            let val: serde_json::Value = serde_json::from_str(black_box(&json_str)).unwrap();
            black_box(val);
        });
    });

    group.finish();
}

fn eval_and_retrieve(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_bridge/eval_retrieve");

    group.bench_function("eval_json_parse_in_js", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(&sentinel_plugins::permissions::PluginPermissions::full_access())
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        let json_str = serde_json::to_string(&bench_helpers::make_medium_json()).unwrap();
        let code = format!(
            r#"var __parsed = JSON.parse('{}');"#,
            json_str.replace('\\', "\\\\").replace('\'', "\\'")
        );

        b.iter(|| {
            runtime.eval(black_box(&code)).unwrap();
        });
    });

    group.bench_function("set_global_vs_json_parse", |b| {
        let mut runtime =
            sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
        runtime
            .register_host_functions(&sentinel_plugins::permissions::PluginPermissions::full_access())
            .unwrap();
        let ctx = PluginContext::new();
        sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
        runtime.eval_bootstrap(&ctx).unwrap();

        let json = bench_helpers::make_medium_json();

        b.iter(|| {
            runtime
                .set_json_global(black_box("__bench_global"), black_box(&json))
                .unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    json_global_set,
    json_serialization_overhead,
    eval_and_retrieve
);
criterion_main!(benches);
