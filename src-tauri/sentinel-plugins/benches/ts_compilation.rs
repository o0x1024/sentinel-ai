#[allow(dead_code)]
mod bench_helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn ts_strip_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ts_strip");

    let small_ts = r#"
interface Foo { bar: string; }
type Name = string;
const x: number = 42;
function greet(name: string): string { return "hello " + name; }
"#;

    let medium_ts = bench_helpers::typescript_plugin();

    let large_ts = {
        let mut s = String::with_capacity(20_000);
        s.push_str("declare const Sentinel: { emit: (f: any) => void; resolve: (r: any) => void };\n");
        for i in 0..50 {
            s.push_str(&format!(
                r#"
interface Config{i} {{
    name: string;
    value: number;
    enabled: boolean;
    metadata?: Record<string, unknown>;
}}

function check{i}(input: Config{i}): {{ success: boolean; error?: string }} {{
    if (!input.name || input.name.length === 0) {{
        return {{ success: false, error: "name required" }};
    }}
    if (input.value < 0 || input.value > 1000) {{
        return {{ success: false, error: "value out of range" }};
    }}
    return {{ success: true }};
}}
"#,
                i = i
            ));
        }
        s.push_str(
            r#"
function scan_transaction(transaction: any) {
    var results = [];
    for (var i = 0; i < 50; i++) {
        results.push({ index: i, status: "ok" });
    }
    Sentinel.resolve({ results: results });
}
"#,
        );
        s
    };

    group.bench_function("small", |b| {
        b.iter(|| {
            let result = sentinel_plugins::ts_strip::strip_typescript(black_box(small_ts));
            black_box(result.unwrap());
        });
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let result = sentinel_plugins::ts_strip::strip_typescript(black_box(medium_ts));
            black_box(result.unwrap());
        });
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            let result = sentinel_plugins::ts_strip::strip_typescript(black_box(&large_ts));
            black_box(result.unwrap());
        });
    });

    group.bench_function("strip_for_script_medium", |b| {
        b.iter(|| {
            let result =
                sentinel_plugins::ts_strip::strip_typescript_for_script(black_box(medium_ts));
            black_box(result.unwrap());
        });
    });

    group.bench_function("strip_for_script_large", |b| {
        b.iter(|| {
            let result =
                sentinel_plugins::ts_strip::strip_typescript_for_script(black_box(&large_ts));
            black_box(result.unwrap());
        });
    });

    group.finish();
}

fn compile_cache_perf(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_cache");

    let plugin_source = bench_helpers::typescript_plugin();

    sentinel_plugins::compile_cache::clear_all();

    group.bench_function("cold_miss", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for i in 0..iters {
                sentinel_plugins::compile_cache::clear_all();
                let result = sentinel_plugins::compile_cache::compile_cached(
                    &format!("bench-plugin-{i}"),
                    plugin_source,
                    false,
                );
                black_box(result.unwrap());
            }
            start.elapsed()
        });
    });

    sentinel_plugins::compile_cache::clear_all();
    sentinel_plugins::compile_cache::compile_cached("bench-cached", plugin_source, false).unwrap();

    group.bench_function("warm_hit", |b| {
        b.iter(|| {
            let result = sentinel_plugins::compile_cache::compile_cached(
                black_box("bench-cached"),
                black_box(plugin_source),
                false,
            );
            black_box(result.unwrap());
        });
    });

    group.bench_function("hash_plugin_source", |b| {
        b.iter(|| {
            let hash =
                sentinel_plugins::compile_cache::hash_plugin_source(black_box(plugin_source));
            black_box(hash);
        });
    });

    group.finish();
}

criterion_group!(benches, ts_strip_sizes, compile_cache_perf);
criterion_main!(benches);
