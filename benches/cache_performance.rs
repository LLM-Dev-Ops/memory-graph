//! Performance benchmarks for the caching layer
//!
//! Run with: cargo bench cache_performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    types::{Config, TokenUsage},
};
use std::time::Duration;
use tempfile::tempdir;

/// Benchmark cache hit performance (should be < 1ms)
fn bench_cache_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let (graph, _session, prompt_ids) = runtime.block_on(async {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await.unwrap();

        let session = graph.create_session().await.unwrap();

        // Pre-populate cache with 100 prompts
        let mut ids = Vec::new();
        for i in 0..100 {
            let id = graph
                .add_prompt(session.id, format!("Test prompt {}", i), None)
                .await
                .unwrap();
            ids.push(id);
        }

        (graph, session, ids)
    });

    group.bench_function("single_node_cache_hit", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let node = graph.get_node(&prompt_ids[0]).await.unwrap();
                black_box(node);
            })
        });
    });

    group.bench_function("100_sequential_cache_hits", |b| {
        b.iter(|| {
            runtime.block_on(async {
                for id in &prompt_ids {
                    let node = graph.get_node(id).await.unwrap();
                    black_box(node);
                }
            })
        });
    });

    group.finish();
}

/// Benchmark cache miss and load performance (should be < 10ms)
fn bench_cache_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_miss");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("single_node_cache_miss", |b| {
        b.iter_batched(
            || {
                // Setup: Create new graph and prompt for each iteration
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                let graph = runtime.block_on(async {
                    let g = AsyncMemoryGraph::open(config).await.unwrap();
                    let session = g.create_session().await.unwrap();
                    let id = g
                        .add_prompt(session.id, "Test prompt".to_string(), None)
                        .await
                        .unwrap();
                    (g, id)
                });
                graph
            },
            |(graph, id)| {
                runtime.block_on(async {
                    // Benchmark: Clear cache and read (cache miss)
                    graph.get_node(&id).await.unwrap();
                    black_box(id);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark write performance with cache population
fn bench_write_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_operations");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("add_prompt", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                let (graph, session) = runtime.block_on(async {
                    let g = AsyncMemoryGraph::open(config).await.unwrap();
                    let s = g.create_session().await.unwrap();
                    (g, s)
                });
                (graph, session)
            },
            |(graph, session)| {
                runtime.block_on(async {
                    graph
                        .add_prompt(session.id, "Benchmark prompt".to_string(), None)
                        .await
                        .unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("add_response", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                let (graph, prompt_id) = runtime.block_on(async {
                    let g = AsyncMemoryGraph::open(config).await.unwrap();
                    let s = g.create_session().await.unwrap();
                    let p = g
                        .add_prompt(s.id, "Test".to_string(), None)
                        .await
                        .unwrap();
                    (g, p)
                });
                (graph, prompt_id)
            },
            |(graph, prompt_id)| {
                runtime.block_on(async {
                    let usage = TokenUsage::new(10, 50);
                    graph
                        .add_response(prompt_id, "Benchmark response".to_string(), usage, None)
                        .await
                        .unwrap();
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(15));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for concurrency in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                b.iter_batched(
                    || {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let (graph, session) = runtime.block_on(async {
                            let g = AsyncMemoryGraph::open(config).await.unwrap();
                            let s = g.create_session().await.unwrap();
                            (std::sync::Arc::new(g), s)
                        });
                        (graph, session)
                    },
                    |(graph, session)| {
                        runtime.block_on(async {
                            let mut handles = Vec::new();
                            for i in 0..concurrency {
                                let g = std::sync::Arc::clone(&graph);
                                let s = session.id;
                                let handle = tokio::spawn(async move {
                                    g.add_prompt(s, format!("Concurrent prompt {}", i), None)
                                        .await
                                });
                                handles.push(handle);
                            }

                            for handle in handles {
                                handle.await.unwrap().unwrap();
                            }
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark batch operations
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let (graph, session) = runtime.block_on(async {
                            let g = AsyncMemoryGraph::open(config).await.unwrap();
                            let s = g.create_session().await.unwrap();
                            (g, s)
                        });
                        (graph, session, batch_size)
                    },
                    |(graph, session, batch_size)| {
                        runtime.block_on(async {
                            let prompts: Vec<_> = (0..batch_size)
                                .map(|i| (session.id, format!("Batch prompt {}", i)))
                                .collect();

                            graph.add_prompts_batch(prompts).await.unwrap();
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_hit,
    bench_cache_miss,
    bench_write_operations,
    bench_concurrent_operations,
    bench_batch_operations
);
criterion_main!(benches);
