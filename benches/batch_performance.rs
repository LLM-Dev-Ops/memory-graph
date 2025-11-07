//! Performance benchmarks for concurrent batch operations
//!
//! These benchmarks measure the throughput and performance characteristics
//! of batch operations compared to sequential operations.
//!
//! Run with: cargo bench batch_performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    types::{Config, TokenUsage},
};
use std::time::Duration;
use tempfile::tempdir;

/// Benchmark batch prompt creation vs sequential
fn bench_batch_prompts(c: &mut Criterion) {
    let mut group = c.benchmark_group("prompt_creation");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 50, 100, 200].iter() {
        // Sequential operation
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            let graph = AsyncMemoryGraph::open(config).await.unwrap();
                            let session = graph.create_session().await.unwrap();
                            (graph, session)
                        })
                    },
                    |(graph, session)| {
                        runtime.block_on(async {
                            for i in 0..size {
                                graph
                                    .add_prompt(session.id, format!("Prompt {}", i), None)
                                    .await
                                    .unwrap();
                            }
                            black_box(size);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let graph = AsyncMemoryGraph::open(config).await.unwrap();
                        let session = graph.create_session().await.unwrap();
                        (graph, session)
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        let prompts: Vec<_> = (0..size)
                            .map(|i| (session.id, format!("Prompt {}", i)))
                            .collect();
                        graph.add_prompts_batch(prompts).await.unwrap();
                        black_box(size);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch response creation
fn bench_batch_responses(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_creation");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 50, 100, 200].iter() {
        // Sequential operation
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            let graph = AsyncMemoryGraph::open(config).await.unwrap();
                            let session = graph.create_session().await.unwrap();

                            // Create prompts first
                            let mut prompt_ids = vec![];
                            for i in 0..size {
                                let id = graph
                                    .add_prompt(session.id, format!("Prompt {}", i), None)
                                    .await
                                    .unwrap();
                                prompt_ids.push(id);
                            }

                            (graph, prompt_ids)
                        })
                    },
                    |(graph, prompt_ids)| {
                        runtime.block_on(async {
                            for (i, prompt_id) in prompt_ids.iter().enumerate() {
                                graph
                                    .add_response(
                                        *prompt_id,
                                        format!("Response {}", i),
                                        TokenUsage::new(10, 20),
                                        None,
                                    )
                                    .await
                                    .unwrap();
                            }
                            black_box(prompt_ids.len());
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let graph = AsyncMemoryGraph::open(config).await.unwrap();
                        let session = graph.create_session().await.unwrap();

                        // Create prompts first
                        let mut prompt_ids = vec![];
                        for i in 0..size {
                            let id = graph
                                .add_prompt(session.id, format!("Prompt {}", i), None)
                                .await
                                .unwrap();
                            prompt_ids.push(id);
                        }

                        (graph, prompt_ids)
                    })
                },
                |(graph, prompt_ids)| {
                    runtime.block_on(async {
                        let responses: Vec<_> = prompt_ids
                            .iter()
                            .enumerate()
                            .map(|(i, &id)| (id, format!("Response {}", i), TokenUsage::new(10, 20)))
                            .collect();
                        graph.add_responses_batch(responses).await.unwrap();
                        black_box(prompt_ids.len());
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch session creation
fn bench_batch_sessions(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_creation");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [5, 10, 25, 50].iter() {
        // Sequential operation
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            AsyncMemoryGraph::open(config).await.unwrap()
                        })
                    },
                    |graph| {
                        runtime.block_on(async {
                            for _ in 0..size {
                                graph.create_session().await.unwrap();
                            }
                            black_box(size);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        AsyncMemoryGraph::open(config).await.unwrap()
                    })
                },
                |graph| {
                    runtime.block_on(async {
                        graph.create_sessions_batch(size).await.unwrap();
                        black_box(size);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch node retrieval
fn bench_batch_get_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_retrieval");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 50, 100, 200].iter() {
        // Sequential operation
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            let graph = AsyncMemoryGraph::open(config).await.unwrap();
                            let session = graph.create_session().await.unwrap();

                            let mut ids = vec![];
                            for i in 0..size {
                                let id = graph
                                    .add_prompt(session.id, format!("Prompt {}", i), None)
                                    .await
                                    .unwrap();
                                ids.push(id);
                            }

                            (graph, ids)
                        })
                    },
                    |(graph, ids)| {
                        runtime.block_on(async {
                            for id in &ids {
                                graph.get_node(id).await.unwrap();
                            }
                            black_box(ids.len());
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let graph = AsyncMemoryGraph::open(config).await.unwrap();
                        let session = graph.create_session().await.unwrap();

                        let mut ids = vec![];
                        for i in 0..size {
                            let id = graph
                                .add_prompt(session.id, format!("Prompt {}", i), None)
                                .await
                                .unwrap();
                            ids.push(id);
                        }

                        (graph, ids)
                    })
                },
                |(graph, ids)| {
                    runtime.block_on(async {
                        graph.get_nodes_batch(ids).await.unwrap();
                        black_box(size);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch node deletion (only batch operation since no single delete API)
fn bench_batch_delete_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("node_deletion");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 50, 100, 200].iter() {
        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let graph = AsyncMemoryGraph::open(config).await.unwrap();
                        let session = graph.create_session().await.unwrap();

                        let mut ids = vec![];
                        for i in 0..size {
                            let id = graph
                                .add_prompt(session.id, format!("Prompt {}", i), None)
                                .await
                                .unwrap();
                            ids.push(id);
                        }

                        (graph, ids)
                    })
                },
                |(graph, ids)| {
                    runtime.block_on(async {
                        graph.delete_nodes_batch(ids).await.unwrap();
                        black_box(size);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark batch conversation creation
fn bench_batch_conversations(c: &mut Criterion) {
    let mut group = c.benchmark_group("conversation_creation");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 25, 50, 100].iter() {
        // Sequential operation
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            let graph = AsyncMemoryGraph::open(config).await.unwrap();
                            let session = graph.create_session().await.unwrap();
                            (graph, session)
                        })
                    },
                    |(graph, session)| {
                        runtime.block_on(async {
                            for i in 0..size {
                                let prompt_id = graph
                                    .add_prompt(session.id, format!("Prompt {}", i), None)
                                    .await
                                    .unwrap();
                                graph
                                    .add_response(
                                        prompt_id,
                                        format!("Response {}", i),
                                        TokenUsage::new(10, 20),
                                        None,
                                    )
                                    .await
                                    .unwrap();
                            }
                            black_box(size);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Batch operation
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        let dir = tempdir().unwrap();
                        let config = Config::new(dir.path());
                        let graph = AsyncMemoryGraph::open(config).await.unwrap();
                        let session = graph.create_session().await.unwrap();
                        (graph, session)
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        let conversations: Vec<_> = (0..size)
                            .map(|i| {
                                (
                                    (session.id, format!("Prompt {}", i)),
                                    Some((format!("Response {}", i), TokenUsage::new(10, 20))),
                                )
                            })
                            .collect();
                        graph.add_conversations_batch(conversations).await.unwrap();
                        black_box(size);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark scalability with large batches
fn bench_large_batches(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_batches");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("prompts", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let config = Config::new(dir.path());
                            let graph = AsyncMemoryGraph::open(config).await.unwrap();
                            let session = graph.create_session().await.unwrap();
                            (graph, session)
                        })
                    },
                    |(graph, session)| {
                        runtime.block_on(async {
                            let prompts: Vec<_> = (0..size)
                                .map(|i| (session.id, format!("Prompt {}", i)))
                                .collect();
                            graph.add_prompts_batch(prompts).await.unwrap();
                            black_box(size);
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
    bench_batch_prompts,
    bench_batch_responses,
    bench_batch_sessions,
    bench_batch_get_nodes,
    bench_batch_delete_nodes,
    bench_batch_conversations,
    bench_large_batches
);
criterion_main!(benches);
