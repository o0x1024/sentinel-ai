#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use sentinel_plugins::permissions::PluginPermissions;
use sentinel_plugins::plugin_context::PluginContext;

fn runtime_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("runtime_lifecycle");

    group.bench_function("sentinel_js_runtime_create", |b| {
        b.iter(|| {
            let runtime = sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
            black_box(runtime);
        });
    });

    group.bench_function("sentinel_js_runtime_create_and_register_full_perms", |b| {
        b.iter(|| {
            let mut runtime =
                sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
            runtime
                .register_host_functions(&PluginPermissions::full_access())
                .unwrap();
            black_box(runtime);
        });
    });

    group.bench_function(
        "sentinel_js_runtime_create_and_register_sandboxed",
        |b| {
            b.iter(|| {
                let mut runtime =
                    sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
                runtime
                    .register_host_functions(&PluginPermissions::sandboxed())
                    .unwrap();
                black_box(runtime);
            });
        },
    );

    group.bench_function("sentinel_js_runtime_full_bootstrap", |b| {
        b.iter(|| {
            let mut runtime =
                sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
            runtime
                .register_host_functions(&PluginPermissions::full_access())
                .unwrap();
            let ctx = PluginContext::new();
            sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
            runtime.eval_bootstrap(&ctx).unwrap();
            black_box(runtime);
        });
    });

    group.bench_function("plugin_engine_new", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = sentinel_plugins::PluginEngine::new().unwrap();
            black_box(engine);
        });
    });

    group.finish();
}

fn permission_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("permission_registration");

    let variants: Vec<(&str, PluginPermissions)> = vec![
        ("no_perms", PluginPermissions {
            network: sentinel_plugins::permissions::NetworkPermission::None,
            filesystem: sentinel_plugins::permissions::FsPermission::None,
            dictionary: false,
            monitor_events: false,
            ast_parse: false,
            tls_inspect: false,
        }),
        ("fetch_only", PluginPermissions::sandboxed()),
        ("full_access", PluginPermissions::full_access()),
    ];

    for (name, perms) in &variants {
        group.bench_with_input(BenchmarkId::new("sentinel_js", name), perms, |b, perms| {
            b.iter(|| {
                let mut runtime =
                    sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
                runtime.register_host_functions(perms).unwrap();
                black_box(runtime);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, runtime_creation, permission_variants);
criterion_main!(benches);
