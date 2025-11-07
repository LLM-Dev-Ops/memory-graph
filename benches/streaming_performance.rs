//! Performance benchmarks for stream-based queries
//!
//! These benchmarks compare streaming vs batch query performance to demonstrate
//! the memory efficiency and throughput characteristics of streaming queries.
//!
//! Run with: cargo bench streaming_performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::stream::StreamExt;
use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    storage::{AsyncSledBackend, AsyncStorageBackend},
    types::{Config, ConversationSession, Node, NodeType, PromptNode},
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

/// Helper to create a graph with a session containing N prompts
async fn create_test_graph(num_prompts: usize) -> (AsyncMemoryGraph, ConversationSession) {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = AsyncMemoryGraph::open(config).await.unwrap();

    let session = graph.create_session().await.unwrap();

    for i in 0..num_prompts {
        graph
            .add_prompt(session.id, format!("Test prompt {}", i), None)
            .await
            .unwrap();
    }

    (graph, session)
}

/// Benchmark batch query performance (loads all results into memory)
fn bench_batch_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_queries");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        create_test_graph(size).await
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        let results = graph
                            .query()
                            .session(session.id)
                            .node_type(NodeType::Prompt)
                            .execute()
                            .await
                            .unwrap();
                        black_box(results.len());
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark streaming query performance (memory-efficient iteration)
fn bench_streaming_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_queries");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [10, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        create_test_graph(size).await
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        let query = graph
                            .query()
                            .session(session.id)
                            .node_type(NodeType::Prompt);
                        let mut stream = query.execute_stream();

                        let mut count = 0;
                        while let Some(result) = stream.next().await {
                            result.unwrap();
                            count += 1;
                        }
                        black_box(count);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark count operations (should be O(1) for unfiltered queries)
fn bench_count_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_operations");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        create_test_graph(size).await
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        // Efficient count (no filtering)
                        let count = graph
                            .query()
                            .session(session.id)
                            .count()
                            .await
                            .unwrap();
                        black_box(count);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark filtered count operations (requires streaming)
fn bench_filtered_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtered_count");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    runtime.block_on(async {
                        create_test_graph(size).await
                    })
                },
                |(graph, session)| {
                    runtime.block_on(async {
                        // Filtered count (requires iteration)
                        let count = graph
                            .query()
                            .session(session.id)
                            .node_type(NodeType::Prompt)
                            .count()
                            .await
                            .unwrap();
                        black_box(count);
                    })
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark pagination with streaming
fn bench_pagination(c: &mut Criterion) {
    let mut group = c.benchmark_group("pagination");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Test pagination on 1000 node dataset
    group.bench_function("page_10_of_1000", |b| {
        b.iter_batched(
            || {
                runtime.block_on(async {
                    create_test_graph(1000).await
                })
            },
            |(graph, session)| {
                runtime.block_on(async {
                    // Get page 10 (items 90-99)
                    let results = graph
                        .query()
                        .session(session.id)
                        .node_type(NodeType::Prompt)
                        .offset(90)
                        .limit(10)
                        .execute()
                        .await
                        .unwrap();
                    black_box(results.len());
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("page_10_stream_of_1000", |b| {
        b.iter_batched(
            || {
                runtime.block_on(async {
                    create_test_graph(1000).await
                })
            },
            |(graph, session)| {
                runtime.block_on(async {
                    // Get page 10 via streaming
                    let query = graph
                        .query()
                        .session(session.id)
                        .node_type(NodeType::Prompt)
                        .offset(90)
                        .limit(10);
                    let mut stream = query.execute_stream();

                    let mut count = 0;
                    while let Some(result) = stream.next().await {
                        result.unwrap();
                        count += 1;
                    }
                    black_box(count);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark storage-level streaming vs batch loading
fn bench_storage_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_streaming");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for size in [100, 500, 1000].iter() {
        // Batch loading
        group.bench_with_input(
            BenchmarkId::new("batch", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn AsyncStorageBackend>;

                            let session = ConversationSession::new();
                            backend.store_node(&Node::Session(session.clone())).await.unwrap();

                            for i in 0..size {
                                let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
                                backend.store_node(&Node::Prompt(prompt)).await.unwrap();
                            }

                            (backend, session.id)
                        })
                    },
                    |(backend, session_id)| {
                        runtime.block_on(async {
                            let nodes = backend.get_session_nodes(&session_id).await.unwrap();
                            black_box(nodes.len());
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Streaming
        group.bench_with_input(
            BenchmarkId::new("stream", size),
            size,
            |b, &size| {
                b.iter_batched(
                    || {
                        runtime.block_on(async {
                            let dir = tempdir().unwrap();
                            let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn AsyncStorageBackend>;

                            let session = ConversationSession::new();
                            backend.store_node(&Node::Session(session.clone())).await.unwrap();

                            for i in 0..size {
                                let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
                                backend.store_node(&Node::Prompt(prompt)).await.unwrap();
                            }

                            (backend, session.id)
                        })
                    },
                    |(backend, session_id)| {
                        runtime.block_on(async {
                            let mut stream = backend.get_session_nodes_stream(&session_id);
                            let mut count = 0;
                            while let Some(result) = stream.next().await {
                                result.unwrap();
                                count += 1;
                            }
                            black_box(count);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark early termination in streaming (limit=10 from 1000)
fn bench_early_termination(c: &mut Criterion) {
    let mut group = c.benchmark_group("early_termination");
    group.measurement_time(Duration::from_secs(10));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Batch: loads all 1000, then takes 10
    group.bench_function("batch_take_10_from_1000", |b| {
        b.iter_batched(
            || {
                runtime.block_on(async {
                    create_test_graph(1000).await
                })
            },
            |(graph, session)| {
                runtime.block_on(async {
                    let results = graph
                        .query()
                        .session(session.id)
                        .node_type(NodeType::Prompt)
                        .limit(10)
                        .execute()
                        .await
                        .unwrap();
                    black_box(results.len());
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Streaming: stops after 10 items
    group.bench_function("stream_take_10_from_1000", |b| {
        b.iter_batched(
            || {
                runtime.block_on(async {
                    create_test_graph(1000).await
                })
            },
            |(graph, session)| {
                runtime.block_on(async {
                    let query = graph
                        .query()
                        .session(session.id)
                        .node_type(NodeType::Prompt)
                        .limit(10);
                    let mut stream = query.execute_stream();

                    let mut count = 0;
                    while let Some(result) = stream.next().await {
                        result.unwrap();
                        count += 1;
                    }
                    black_box(count);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_queries,
    bench_streaming_queries,
    bench_count_operations,
    bench_filtered_count,
    bench_pagination,
    bench_storage_streaming,
    bench_early_termination
);
criterion_main!(benches);
