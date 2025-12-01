# LLM Memory Graph - Canonical Benchmark Suite

This directory contains the output from the canonical benchmark suite for llm-memory-graph.

## Directory Structure

```
benchmarks/
├── output/
│   ├── raw/
│   │   ├── results_TIMESTAMP.json    # Raw benchmark data in JSON format
│   │   └── results_TIMESTAMP.csv     # Raw benchmark data in CSV format
│   └── summary.md                     # Human-readable markdown report
└── README.md                          # This file
```

## Running Benchmarks

To run the complete benchmark suite:

```bash
cargo run --bin bench_runner
```

This will:
1. Execute all registered benchmark targets
2. Collect results in the canonical format
3. Write results to `output/raw/results_TIMESTAMP.json` and `.csv`
4. Generate a summary report at `output/summary.md`

## Benchmark Results Format

### JSON Format

Results are stored as an array of `BenchmarkResult` objects:

```json
[
  {
    "target_id": "batch_prompts_sequential_10",
    "metrics": {
      "mean_ns": 1234567.89,
      "std_dev_ns": 12345.67,
      "throughput": 8000.0,
      "operations": 10,
      "category": "batch_operations"
    },
    "timestamp": "2025-12-01T23:40:00.000Z"
  }
]
```

### CSV Format

Results are also available in CSV format for easy import into spreadsheets:

```csv
target_id,timestamp,mean_ns,std_dev_ns,median_ns,throughput,metrics_json
batch_prompts_sequential_10,2025-12-01T23:40:00.000Z,1234567.89,12345.67,,,{"mean_ns":1234567.89,...}
```

## Benchmark Categories

The benchmark suite is organized into categories:

- **batch_operations**: Sequential vs batch operation performance
- **cache_performance**: Cache hit and miss performance
- **pool_performance**: Connection pooling overhead

## Adding New Benchmarks

To add a new benchmark:

1. Create a struct implementing the `BenchTarget` trait in `src/benchmarks/targets.rs`
2. Register the benchmark in the `all_targets()` function in `src/benchmarks/adapters.rs`
3. Run the suite to verify it's included

## Integration with Cross-Repository Analysis

These results follow the canonical benchmark format shared across multiple repositories:
- llm-memory-graph (this repository)
- llm-context-cache
- llm-prompt-lineage

This enables aggregation and comparison of benchmark results across the entire ecosystem.

## Output Files

### results_TIMESTAMP.json

Raw benchmark data in JSON format. This is the primary data source for automated analysis and aggregation.

### results_TIMESTAMP.csv

Benchmark data in CSV format for manual analysis in spreadsheet applications.

### summary.md

Human-readable markdown report with:
- Overview table of all benchmarks
- Detailed metrics for each benchmark
- Timestamp information

## Notes

- Benchmarks run sequentially by default to avoid resource contention
- Each benchmark gets a clean temporary environment
- Results include both timing metrics and custom benchmark-specific data
- All timestamps are in UTC
