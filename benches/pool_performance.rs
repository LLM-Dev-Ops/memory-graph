//! Performance benchmarks for the connection pooling layer
//!
//! Run with: cargo bench pool_performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use llm_memory_graph::{
    storage::{AsyncSledBackend, AsyncStorageBackend, PoolConfig, PooledAsyncBackend},
    types::{Config, ConversationSession, Node, SessionId},
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

/// Helper to create a test node
fn create_test_node(_session_id: SessionId, _index: usize) -> Node {
    Node::Session(ConversationSession::new())
}

/// Benchmark pool overhead by comparing pooled vs non-pooled backends
fn bench_pool_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_overhead");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Benchmark non-pooled backend
    group.bench_function("non_pooled_single_write", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                let backend = runtime.block_on(async {
                    AsyncSledBackend::open(&config.path).await.unwrap()
                });
                (backend, create_test_node(SessionId::new(), 0))
            },
            |(backend, node)| {
                runtime.block_on(async {
                    backend.store_node(&node).await.unwrap();
                    black_box(node);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark pooled backend with default config
    group.bench_function("pooled_single_write", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                let backend = runtime.block_on(async {
                    PooledAsyncBackend::open(&config.path, PoolConfig::default()).await.unwrap()
                });
                (backend, create_test_node(SessionId::new(), 0))
            },
            |(backend, node)| {
                runtime.block_on(async {
                    backend.store_node(&node).await.unwrap();
                    black_box(node);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark non-pooled sequential reads
    group.bench_function("non_pooled_sequential_reads", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let backend = AsyncSledBackend::open(&config.path).await.unwrap();
                    let session_id = SessionId::new();
                    let mut node_ids = Vec::new();

                    // Pre-populate with 100 nodes
                    for i in 0..100 {
                        let node = create_test_node(session_id, i);
                        let id = node.id();
                        backend.store_node(&node).await.unwrap();
                        node_ids.push(id);
                    }

                    (backend, node_ids)
                })
            },
            |(backend, node_ids)| {
                runtime.block_on(async {
                    for id in &node_ids {
                        let node = backend.get_node(id).await.unwrap();
                        black_box(node);
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark pooled sequential reads
    group.bench_function("pooled_sequential_reads", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let backend = PooledAsyncBackend::open(&config.path, PoolConfig::default()).await.unwrap();
                    let session_id = SessionId::new();
                    let mut node_ids = Vec::new();

                    // Pre-populate with 100 nodes
                    for i in 0..100 {
                        let node = create_test_node(session_id, i);
                        let id = node.id();
                        backend.store_node(&node).await.unwrap();
                        node_ids.push(id);
                    }

                    (backend, node_ids)
                })
            },
            |(backend, node_ids)| {
                runtime.block_on(async {
                    for id in &node_ids {
                        let node = backend.get_node(id).await.unwrap();
                        black_box(node);
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark concurrent operations with different pool sizes
fn bench_pool_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_sizes");
    group.measurement_time(Duration::from_secs(15));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for pool_size in [10, 50, 100, 500].iter() {
        for concurrency in [10, 50, 100].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("pool_{}", pool_size), concurrency),
                &(pool_size, concurrency),
                |b, &(pool_size, concurrency)| {
                    b.iter_batched(
                        || {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            runtime.block_on(async {
                                let pool_config = PoolConfig {
                                    max_concurrent: *pool_size,
                                    acquire_timeout_ms: 5000,
                                    enable_metrics: true,
                                };
                                let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                                let session_id = SessionId::new();
                                (Arc::new(pooled), session_id)
                            })
                        },
                        |(backend, session_id)| {
                            runtime.block_on(async {
                                let mut handles = Vec::new();
                                for i in 0..*concurrency {
                                    let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                                    let s = session_id;
                                    let handle = tokio::spawn(async move {
                                        let node = create_test_node(s, i);
                                        b.store_node(&node).await
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
    }

    group.finish();
}

/// Benchmark backpressure behavior when pool is saturated
fn bench_backpressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("backpressure");
    group.measurement_time(Duration::from_secs(15));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Test with small pool and high concurrency
    group.bench_function("small_pool_high_concurrency", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let pool_config = PoolConfig {
                        max_concurrent: 10,  // Small pool
                        acquire_timeout_ms: 10000,  // Generous timeout
                        enable_metrics: true,
                    };
                    let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                    let session_id = SessionId::new();
                    (Arc::new(pooled), session_id)
                })
            },
            |(backend, session_id)| {
                runtime.block_on(async {
                    let mut handles = Vec::new();
                    // Spawn 100 concurrent operations with pool size of 10
                    for i in 0..100 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let s = session_id;
                        let handle = tokio::spawn(async move {
                            let node = create_test_node(s, i);
                            b.store_node(&node).await
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
    });

    // Test with balanced pool
    group.bench_function("balanced_pool", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let pool_config = PoolConfig {
                        max_concurrent: 100,  // Balanced pool
                        acquire_timeout_ms: 5000,
                        enable_metrics: true,
                    };
                    let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                    let session_id = SessionId::new();
                    (Arc::new(pooled), session_id)
                })
            },
            |(backend, session_id)| {
                runtime.block_on(async {
                    let mut handles = Vec::new();
                    for i in 0..100 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let s = session_id;
                        let handle = tokio::spawn(async move {
                            let node = create_test_node(s, i);
                            b.store_node(&node).await
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
    });

    group.finish();
}

/// Benchmark metrics collection overhead
fn bench_metrics_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_overhead");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Benchmark with metrics enabled
    group.bench_function("metrics_enabled", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let pool_config = PoolConfig {
                        max_concurrent: 100,
                        acquire_timeout_ms: 5000,
                        enable_metrics: true,
                    };
                    let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                    let session_id = SessionId::new();
                    (Arc::new(pooled), session_id)
                })
            },
            |(backend, session_id)| {
                runtime.block_on(async {
                    let mut handles = Vec::new();
                    for i in 0..50 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let s = session_id;
                        let handle = tokio::spawn(async move {
                            let node = create_test_node(s, i);
                            b.store_node(&node).await
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap().unwrap();
                    }

                    // Get metrics
                    let _metrics = backend.metrics();
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark with metrics disabled
    group.bench_function("metrics_disabled", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let pool_config = PoolConfig {
                        max_concurrent: 100,
                        acquire_timeout_ms: 5000,
                        enable_metrics: false,
                    };
                    let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                    let session_id = SessionId::new();
                    (Arc::new(pooled), session_id)
                })
            },
            |(backend, session_id)| {
                runtime.block_on(async {
                    let mut handles = Vec::new();
                    for i in 0..50 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let s = session_id;
                        let handle = tokio::spawn(async move {
                            let node = create_test_node(s, i);
                            b.store_node(&node).await
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
    });

    group.finish();
}

/// Benchmark batch operations with pooling
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.measurement_time(Duration::from_secs(15));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for batch_size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        runtime.block_on(async {
                            let pool_config = PoolConfig {
                                max_concurrent: 100,
                                acquire_timeout_ms: 10000,
                                enable_metrics: true,
                            };
                            let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                            let session_id = SessionId::new();

                            // Create batch of nodes
                            let nodes: Vec<_> = (0..batch_size)
                                .map(|i| create_test_node(session_id, i))
                                .collect();

                            (Arc::new(pooled), nodes)
                        })
                    },
                    |(backend, nodes)| {
                        runtime.block_on(async {
                            backend.store_nodes_batch(&nodes).await.unwrap();
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark mixed read/write workloads
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.measurement_time(Duration::from_secs(15));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("70_read_30_write", |b| {
        b.iter_batched(
            || {
                let dir = tempdir().unwrap();
                let config = Config::new(dir.path());
                runtime.block_on(async {
                    let pool_config = PoolConfig {
                        max_concurrent: 100,
                        acquire_timeout_ms: 5000,
                        enable_metrics: true,
                    };
                    let pooled = PooledAsyncBackend::open(&config.path, pool_config).await.unwrap();
                    let session_id = SessionId::new();

                    // Pre-populate with 100 nodes
                    let mut node_ids = Vec::new();
                    for i in 0..100 {
                        let node = create_test_node(session_id, i);
                        let id = node.id();
                        pooled.store_node(&node).await.unwrap();
                        node_ids.push(id);
                    }

                    (Arc::new(pooled), session_id, node_ids)
                })
            },
            |(backend, session_id, node_ids)| {
                runtime.block_on(async {
                    let mut read_handles = Vec::new();
                    let mut write_handles = Vec::new();

                    // 70% reads
                    for i in 0..70 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let id = node_ids[i % node_ids.len()];
                        let handle = tokio::spawn(async move {
                            b.get_node(&id).await
                        });
                        read_handles.push(handle);
                    }

                    // 30% writes
                    for i in 0..30 {
                        let b: Arc<PooledAsyncBackend> = Arc::clone(&backend);
                        let s = session_id;
                        let handle = tokio::spawn(async move {
                            let node = create_test_node(s, i + 1000);
                            b.store_node(&node).await
                        });
                        write_handles.push(handle);
                    }

                    // Await all reads
                    for handle in read_handles {
                        handle.await.unwrap().unwrap();
                    }

                    // Await all writes
                    for handle in write_handles {
                        handle.await.unwrap().unwrap();
                    }
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pool_overhead,
    bench_pool_sizes,
    bench_backpressure,
    bench_metrics_overhead,
    bench_batch_operations,
    bench_mixed_workload
);
criterion_main!(benches);
