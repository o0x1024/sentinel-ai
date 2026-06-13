#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentinel_plugins::engine_pool::{EnginePool, EnginePoolConfig};
use sentinel_plugins::types::PluginMainCategory;

fn pool_dispatch_benches(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("engine_pool");
    group.sample_size(20);

    group.bench_function("scan_cold_start", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-pool-cold", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let pool = EnginePool::new(EnginePoolConfig {
                    worker_count: 2,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                });
                let start = std::time::Instant::now();
                for i in 0..iters {
                    let plugin_id = format!("bench-pool-cold-{}", i);
                    let findings = pool
                        .execute_scan(&plugin_id, &metadata, &code, &txn, None)
                        .await
                        .unwrap();
                    black_box(&findings);
                }
                pool.shutdown().await;
                start.elapsed()
            }
        });
    });

    group.bench_function("scan_warm_cached", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-pool-warm", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let pool = EnginePool::new(EnginePoolConfig {
                    worker_count: 2,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                });
                pool.execute_scan("bench-pool-warm", &metadata, &code, &txn, None)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let findings = pool
                        .execute_scan("bench-pool-warm", &metadata, &code, &txn, None)
                        .await
                        .unwrap();
                    black_box(&findings);
                }
                pool.shutdown().await;
                start.elapsed()
            }
        });
    });

    group.bench_function("agent_cached", |b| {
        let code = bench_helpers::agent_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-pool-agent", PluginMainCategory::Bounty);
        let input = serde_json::json!({
            "targets": ["https://example.com"],
            "timeout": 5000
        });

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let input = input.clone();
            async move {
                let pool = EnginePool::new(EnginePoolConfig {
                    worker_count: 2,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                });
                pool.execute_agent(
                    "bench-pool-agent",
                    &metadata,
                    &code,
                    input.clone(),
                    None,
                    None,
                )
                .await
                .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let result = pool
                        .execute_agent(
                            "bench-pool-agent",
                            &metadata,
                            &code,
                            input.clone(),
                            None,
                            None,
                        )
                        .await
                        .unwrap();
                    black_box(&result);
                }
                pool.shutdown().await;
                start.elapsed()
            }
        });
    });

    group.bench_function("pool_evict_and_recreate", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-pool-evict", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter_custom(|iters| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let pool = EnginePool::new(EnginePoolConfig {
                    worker_count: 2,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                });
                pool.execute_scan("bench-pool-evict", &metadata, &code, &txn, None)
                    .await
                    .unwrap();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    pool.evict("bench-pool-evict").await;
                    let findings = pool
                        .execute_scan("bench-pool-evict", &metadata, &code, &txn, None)
                        .await
                        .unwrap();
                    black_box(&findings);
                }
                pool.shutdown().await;
                start.elapsed()
            }
        });
    });

    group.finish();
}

fn pool_concurrent_benches(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("engine_pool_concurrent");
    group.sample_size(10);

    group.bench_function("4_concurrent_scans_same_plugin", |b| {
        let code = bench_helpers::simple_js_plugin();
        let metadata =
            bench_helpers::make_metadata("bench-pool-conc", PluginMainCategory::Traffic);
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter(|| {
            let code = code.to_string();
            let metadata = metadata.clone();
            let txn = txn.clone();
            async move {
                let pool = std::sync::Arc::new(EnginePool::new(EnginePoolConfig {
                    worker_count: 4,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                }));
                let mut handles = Vec::new();
                for _ in 0..4 {
                    let pool = pool.clone();
                    let code = code.clone();
                    let metadata = metadata.clone();
                    let txn = txn.clone();
                    handles.push(tokio::spawn(async move {
                        pool.execute_scan("bench-pool-conc", &metadata, &code, &txn, None)
                            .await
                            .unwrap()
                    }));
                }
                for handle in handles {
                    let findings = handle.await.unwrap();
                    black_box(findings);
                }
                pool.shutdown().await;
            }
        });
    });

    group.bench_function("4_concurrent_scans_different_plugins", |b| {
        let code = bench_helpers::simple_js_plugin();
        let txn = bench_helpers::make_simple_transaction();

        b.to_async(&rt).iter(|| {
            let code = code.to_string();
            let txn = txn.clone();
            async move {
                let pool = std::sync::Arc::new(EnginePool::new(EnginePoolConfig {
                    worker_count: 4,
                    max_engines_per_worker: 4,
                    max_executions_per_engine: 10_000,
                    idle_evict_secs: 300,
                }));
                let mut handles = Vec::new();
                for i in 0..4 {
                    let pool = pool.clone();
                    let code = code.clone();
                    let txn = txn.clone();
                    let metadata = bench_helpers::make_metadata(
                        &format!("bench-pool-diff-{}", i),
                        PluginMainCategory::Traffic,
                    );
                    handles.push(tokio::spawn(async move {
                        let plugin_id = format!("bench-pool-diff-{}", i);
                        pool.execute_scan(&plugin_id, &metadata, &code, &txn, None)
                            .await
                            .unwrap()
                    }));
                }
                for handle in handles {
                    let findings = handle.await.unwrap();
                    black_box(findings);
                }
                pool.shutdown().await;
            }
        });
    });

    group.finish();
}

fn pool_stats(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("engine_pool_stats");

    group.bench_function("get_stats", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let pool = EnginePool::new(EnginePoolConfig {
                worker_count: 2,
                max_engines_per_worker: 4,
                max_executions_per_engine: 10_000,
                idle_evict_secs: 300,
            });
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let stats = pool.stats().await;
                black_box(stats);
            }
            pool.shutdown().await;
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    pool_dispatch_benches,
    pool_concurrent_benches,
    pool_stats
);
criterion_main!(benches);
