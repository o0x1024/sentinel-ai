#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_plugins::types::PluginMainCategory;

fn plugin_load_variants(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("plugin_loading");
    group.sample_size(30);

    group.bench_function("load_simple_js", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata = bench_helpers::make_metadata("bench-simple", PluginMainCategory::Traffic);
        b.to_async(&rt).iter(|| async {
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(black_box(code), metadata.clone())
                .await
                .unwrap();
            black_box(engine);
        });
    });

    group.bench_function("load_medium_js", |b| {
        let code = bench_helpers::medium_js_plugin();
        let metadata = bench_helpers::make_metadata("bench-medium", PluginMainCategory::Traffic);
        b.to_async(&rt).iter(|| async {
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(black_box(code), metadata.clone())
                .await
                .unwrap();
            black_box(engine);
        });
    });

    group.bench_function("load_typescript_plugin", |b| {
        let code = bench_helpers::typescript_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-typescript", PluginMainCategory::Traffic);
        b.to_async(&rt).iter(|| async {
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(black_box(code), metadata.clone())
                .await
                .unwrap();
            black_box(engine);
        });
    });

    group.bench_function("load_agent_plugin", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata = bench_helpers::make_metadata("bench-agent", PluginMainCategory::Bounty);
        b.to_async(&rt).iter(|| async {
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(black_box(code), metadata.clone())
                .await
                .unwrap();
            black_box(engine);
        });
    });

    group.bench_function("load_heavy_computation", |b| {
        let code = bench_helpers::heavy_computation_plugin();
        let metadata = bench_helpers::make_metadata("bench-heavy", PluginMainCategory::Traffic);
        b.to_async(&rt).iter(|| async {
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(black_box(code), metadata.clone())
                .await
                .unwrap();
            black_box(engine);
        });
    });

    group.finish();
}

fn plugin_reload_skip(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("plugin_reload");
    group.sample_size(50);

    group.bench_function("reload_same_code_skip", |b| {
        let code = bench_helpers::medium_js_plugin();
        let metadata = bench_helpers::make_metadata("bench-reload", PluginMainCategory::Traffic);
        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata.clone())
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    engine
                        .load_plugin_with_metadata(&code, metadata.clone())
                        .await
                        .unwrap();
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("reload_changed_code", |b| {
        let metadata =
            bench_helpers::make_metadata("bench-reload-changed", PluginMainCategory::Traffic);
        b.to_async(&rt).iter_custom(|iters| {
            let metadata = metadata.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                let start = std::time::Instant::now();
                for i in 0..iters {
                    let code = format!(
                        "var VERSION = {};\nfunction scan_transaction(t) {{ Sentinel.emit({{ vuln_type: 'test', title: 'v{}', description: 'd', severity: 'info', confidence: 'high' }}); }}",
                        i, i
                    );
                    engine
                        .load_plugin_with_metadata(&code, metadata.clone())
                        .await
                        .unwrap();
                }
                start.elapsed()
            }
        });
    });

    group.finish();
}

criterion_group!(benches, plugin_load_variants, plugin_reload_skip);
criterion_main!(benches);
