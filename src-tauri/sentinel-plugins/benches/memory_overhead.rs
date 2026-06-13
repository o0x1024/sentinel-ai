#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use sentinel_plugins::permissions::PluginPermissions;
use sentinel_plugins::plugin_context::PluginContext;
use sentinel_plugins::types::PluginMainCategory;
use sysinfo::{Pid, System};

fn current_process_memory_kb() -> u64 {
    let mut sys = System::new();
    let pid = Pid::from(std::process::id() as usize);
    sys.refresh_process(pid);
    sys.process(pid).map(|p| p.memory() / 1024).unwrap_or(0)
}

/// Benchmarks runtime creation cost (time), while also reporting memory footprint.
fn runtime_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");
    group.sample_size(10);

    group.bench_function("sentinel_js_runtime_create_10", |b| {
        b.iter(|| {
            let mut runtimes = Vec::with_capacity(10);
            for _ in 0..10 {
                let mut runtime =
                    sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
                runtime
                    .register_host_functions(&PluginPermissions::full_access())
                    .unwrap();
                let ctx = PluginContext::new();
                sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
                runtime.eval_bootstrap(&ctx).unwrap();
                runtimes.push(runtime);
            }
            black_box(runtimes);
        });
    });

    group.finish();

    // Report memory footprint (informational, not benchmarked by criterion)
    eprintln!("\n=== Memory Footprint Report ===");
    {
        let before = current_process_memory_kb();
        let mut runtimes = Vec::with_capacity(20);
        for _ in 0..20 {
            let mut runtime =
                sentinel_plugins::sentinel_js_runtime::SentinelJsRuntime::new().unwrap();
            runtime
                .register_host_functions(&PluginPermissions::full_access())
                .unwrap();
            let ctx = PluginContext::new();
            sentinel_plugins::sentinel_js_runtime::set_plugin_ctx(&ctx);
            runtime.eval_bootstrap(&ctx).unwrap();
            runtimes.push(runtime);
        }
        let after = current_process_memory_kb();
        eprintln!(
            "SentinelJsRuntime: 20 runtimes = +{}KB ({:.0}KB/runtime)",
            after.saturating_sub(before),
            after.saturating_sub(before) as f64 / 20.0
        );
        drop(runtimes);
    }
    eprintln!("=================================\n");
}

fn plugin_engine_memory(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("memory_overhead/plugin_engine");
    group.sample_size(10);

    let counts = [1u64, 5, 10];
    for &count in &counts {
        group.bench_with_input(
            BenchmarkId::new("create_and_load_engines", count),
            &count,
            |b, &count| {
                let code = bench_helpers::medium_js_plugin();
                b.to_async(&rt).iter(|| async {
                    let mut engines = Vec::with_capacity(count as usize);
                    for i in 0..count {
                        let metadata = bench_helpers::make_metadata(
                            &format!("bench-mem-{}", i),
                            PluginMainCategory::Traffic,
                        );
                        let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
                        engine
                            .load_plugin_with_metadata(code, metadata)
                            .await
                            .unwrap();
                        engines.push(engine);
                    }
                    black_box(engines);
                });
            },
        );
    }

    group.finish();

    // Report memory per engine (informational)
    eprintln!("\n=== PluginEngine Memory Report ===");
    rt.block_on(async {
        let code = bench_helpers::medium_js_plugin();
        let before = current_process_memory_kb();
        let mut engines = Vec::with_capacity(10);
        for i in 0..10 {
            let metadata =
                bench_helpers::make_metadata(&format!("bench-report-{}", i), PluginMainCategory::Traffic);
            let mut engine = sentinel_plugins::PluginEngine::new().unwrap();
            engine
                .load_plugin_with_metadata(code, metadata)
                .await
                .unwrap();
            engines.push(engine);
        }
        let after = current_process_memory_kb();
        eprintln!(
            "PluginEngine (loaded): 10 engines = +{}KB ({:.0}KB/engine)",
            after.saturating_sub(before),
            after.saturating_sub(before) as f64 / 10.0
        );
        drop(engines);
    });
    eprintln!("===================================\n");
}

fn compile_cache_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead/compile_cache");
    group.sample_size(10);

    group.bench_function("populate_50_entries", |b| {
        b.iter(|| {
            sentinel_plugins::compile_cache::clear_all();
            for i in 0..50 {
                let code = format!(
                    "var VERSION_{} = {};\nfunction scan_transaction(t) {{ return []; }}\n",
                    i, i
                );
                sentinel_plugins::compile_cache::compile_cached(
                    &format!("bench-cache-{}", i),
                    &code,
                    false,
                )
                .unwrap();
            }
            let stats = sentinel_plugins::compile_cache::stats();
            black_box(stats);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    runtime_memory_overhead,
    plugin_engine_memory,
    compile_cache_memory
);
criterion_main!(benches);
