#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_plugins::types::PluginMainCategory;

fn scan_transaction_benches(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("scan_transaction");
    group.sample_size(30);

    group.bench_function("simple_plugin_small_txn", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-scan-simple", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = engine.scan_transaction(&txn).await.unwrap();
                    black_box(&findings);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("medium_plugin_small_txn", |b| {
        let code = bench_helpers::medium_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-scan-medium", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = engine.scan_transaction(&txn).await.unwrap();
                    black_box(&findings);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("medium_plugin_large_txn", |b| {
        let code = bench_helpers::medium_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-scan-medium-lg", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_large_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = engine.scan_transaction(&txn).await.unwrap();
                    black_box(&findings);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("heavy_computation_large_txn", |b| {
        let code = bench_helpers::heavy_computation_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-scan-heavy", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_large_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = engine.scan_transaction(&txn).await.unwrap();
                    black_box(&findings);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("typescript_plugin_small_txn", |b| {
        let code = bench_helpers::typescript_plugin();
        let metadata = bench_helpers::make_metadata("bench-scan-ts", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = engine.scan_transaction(&txn).await.unwrap();
                    black_box(&findings);
                }
                start.elapsed()
            }
        });
    });

    group.finish();
}

fn execute_agent_benches(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("execute_agent");
    group.sample_size(30);

    group.bench_function("agent_small_input", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-agent-small", PluginMainCategory::Bounty);
        let input = serde_json::json!({
            "targets": ["https://example.com"],
            "timeout": 5000
        });

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let input = input.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let result = engine.execute_agent(&input).await.unwrap();
                    black_box(&result);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("agent_many_targets", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-agent-many", PluginMainCategory::Bounty);
        let targets: Vec<String> = (0..100)
            .map(|i| format!("https://target{}.example.com", i))
            .collect();
        let input = serde_json::json!({
            "targets": targets,
            "timeout": 10000
        });

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let input = input.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let result = engine.execute_agent(&input).await.unwrap();
                    black_box(&result);
                }
                start.elapsed()
            }
        });
    });

    group.bench_function("get_input_schema", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-schema", PluginMainCategory::Bounty);

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let schema = engine.get_input_schema().await.unwrap();
                    black_box(&schema);
                }
                start.elapsed()
            }
        });
    });

    group.finish();
}

fn end_to_end_benches(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("end_to_end");
    group.sample_size(20);

    group.bench_function("create_load_scan_simple", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-e2e-simple", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter(|| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let findings = engine.scan_transaction(&txn).await.unwrap();
                black_box(findings);
            }
        });
    });

    group.bench_function("create_load_scan_medium", |b| {
        let code = bench_helpers::medium_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-e2e-medium", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_large_transaction();

        b.to_async(&rt).iter(|| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let findings = engine.scan_transaction(&txn).await.unwrap();
                black_box(findings);
            }
        });
    });

    group.bench_function("create_load_agent_call", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-e2e-agent", PluginMainCategory::Bounty);
        let input = serde_json::json!({
            "targets": ["https://example.com"],
            "timeout": 5000
        });

        b.to_async(&rt).iter(|| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let input = input.clone();
            async move {
                let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                engine
                    .load_plugin_with_metadata(&code, metadata)
                    .await
                    .unwrap();
                let result = engine.execute_agent(&input).await.unwrap();
                black_box(result);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    scan_transaction_benches,
    execute_agent_benches,
    end_to_end_benches
);
criterion_main!(benches);
