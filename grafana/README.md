# Grafana Dashboards for LLM Memory Graph

This directory contains production-ready Grafana dashboard specifications for monitoring the LLM Memory Graph system.

## Dashboards

### 1. Overview Dashboard (`memory-graph-overview.json`)

**Purpose**: High-level operational overview and health monitoring

**Panels**:
- **Total Nodes Created** (Gauge) - Cumulative count with thresholds
- **Total Edges Created** (Gauge) - Cumulative count with thresholds
- **Active Sessions Over Time** (Time Series) - Live session tracking
- **Creation Rate** (Time Series) - Nodes and edges created per second
- **Prompts Submitted** (Stat) - Total prompt submissions
- **Responses Generated** (Stat) - Total responses
- **P95 Latency** (Time Series) - 95th percentile for write, read, and query operations
- **Graph Size** (Time Series) - Total nodes and edges over time

**Refresh Rate**: 10 seconds
**Time Range**: Last 1 hour (adjustable)

**Use Cases**:
- Real-time monitoring of system health
- Capacity planning
- Performance SLA tracking
- Incident detection

### 2. Operations Metrics Dashboard (`memory-graph-operations.json`)

**Purpose**: Detailed operation-level metrics and throughput analysis

**Panels**:
- **Operation Rates** - All 8 operation types (nodes, edges, prompts, responses, tools, handoffs, templates, queries)
- **Tool Invocations** - Success rate and performance
- **Agent Handoffs** - Transfer frequency and patterns
- **Template Usage** - Instantiation counts and trends
- **Query Execution** - Query types and performance
- **Batch Operation Sizes** - Distribution histogram
- **Operation Success Rates** - Error tracking per operation type

**Refresh Rate**: 10 seconds
**Time Range**: Last 6 hours

**Use Cases**:
- Operation-specific performance analysis
- Bottleneck identification
- Usage pattern analysis
- Optimization target identification

### 3. Performance Dashboard (`memory-graph-performance.json`)

**Purpose**: Deep performance analysis and SLA monitoring

**Panels**:
- **Latency Heatmaps** - Distribution visualization
- **P50/P95/P99 Latencies** - Multi-percentile tracking
- **Throughput Metrics** - Requests per second by operation
- **Cache Performance** - Hit rates and efficiency
- **Buffer Utilization** - Event buffer size tracking
- **Error Rates** - Failure percentages
- **Resource Utilization** - Cache size in bytes

**Refresh Rate**: 5 seconds
**Time Range**: Last 3 hours

**Use Cases**:
- Performance tuning
- SLA compliance verification
- Capacity planning
- Performance regression detection

### 4. Event Streaming Dashboard (`memory-graph-streaming.json`)

**Purpose**: Real-time event streaming and Observatory monitoring

**Panels**:
- **Event Emission Rate** - Events published per second
- **Event Types Distribution** - Breakdown by event type
- **Kafka Producer Stats** - Batching and throughput
- **Event Buffer Status** - Queue depth and health
- **Emission Success Rate** - Event delivery reliability
- **Batch Size Distribution** - Kafka batch efficiency
- **Event Lag** - Processing delay tracking

**Refresh Rate**: 5 seconds
**Time Range**: Last 30 minutes

**Use Cases**:
- Event streaming health monitoring
- Kafka integration verification
- Event delivery guarantee tracking
- Stream processing optimization

## Installation

### Prerequisites
- Grafana 9.0 or later
- Prometheus data source configured
- Prometheus scraping LLM Memory Graph metrics on the configured endpoint

### Import Instructions

1. **Access Grafana**:
   ```
   http://your-grafana-instance:3000
   ```

2. **Import Dashboard**:
   - Navigate to Dashboards → Import
   - Click "Upload JSON file"
   - Select a dashboard file from this directory
   - Configure the Prometheus data source
   - Click "Import"

3. **Configure Data Source**:
   - Ensure the `DS_PROMETHEUS` variable points to your Prometheus instance
   - Update the dashboard settings if using a different data source name

### Automated Deployment

Using Grafana Provisioning:

```yaml
# /etc/grafana/provisioning/dashboards/llm-memory-graph.yaml
apiVersion: 1

providers:
  - name: 'LLM Memory Graph'
    orgId: 1
    folder: 'LLM Systems'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /path/to/llm-memory-graph/grafana
```

## Metrics Reference

### Counters
- `memory_graph_nodes_created_total` - Total nodes created
- `memory_graph_edges_created_total` - Total edges created
- `memory_graph_prompts_submitted_total` - Total prompts submitted
- `memory_graph_responses_generated_total` - Total responses generated
- `memory_graph_tools_invoked_total` - Total tools invoked
- `memory_graph_agent_handoffs_total` - Total agent handoffs
- `memory_graph_template_instantiations_total` - Total template instantiations
- `memory_graph_queries_executed_total` - Total queries executed

### Histograms
- `memory_graph_write_latency_seconds` - Write operation latency
- `memory_graph_read_latency_seconds` - Read operation latency
- `memory_graph_query_duration_seconds` - Query execution duration
- `memory_graph_tool_duration_seconds` - Tool execution duration
- `memory_graph_batch_size` - Batch operation size distribution

### Gauges
- `memory_graph_active_sessions` - Current active sessions
- `memory_graph_total_nodes` - Total nodes in graph
- `memory_graph_total_edges` - Total edges in graph
- `memory_graph_cache_size_bytes` - Cache size in bytes
- `memory_graph_buffer_size` - Current event buffer size

## Alert Rules

Recommended alert rules for Prometheus Alertmanager:

```yaml
groups:
  - name: llm_memory_graph
    rules:
      # High write latency
      - alert: HighWriteLatency
        expr: histogram_quantile(0.95, rate(memory_graph_write_latency_seconds_bucket[5m])) > 0.050
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P95 write latency detected"
          description: "P95 write latency is {{ $value }}s (threshold: 50ms)"

      # High error rate
      - alert: HighErrorRate
        expr: rate(memory_graph_errors_total[5m]) > 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors/sec"

      # Low cache hit rate
      - alert: LowCacheHitRate
        expr: rate(memory_graph_cache_hits_total[5m]) / rate(memory_graph_cache_requests_total[5m]) < 0.80
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit rate"
          description: "Cache hit rate is {{ $value | humanizePercentage }}"

      # High event buffer size
      - alert: HighEventBufferSize
        expr: memory_graph_buffer_size > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Event buffer is filling up"
          description: "Buffer size is {{ $value }} events"
```

## Customization

### Modifying Panels

1. Import the dashboard
2. Click the panel title → Edit
3. Modify queries, visualization, or settings
4. Save the dashboard
5. Export the modified JSON to update this repository

### Adding New Panels

1. Click "Add panel" in the dashboard
2. Configure the query using Prometheus metrics
3. Set visualization options
4. Position the panel in the grid
5. Save and export

### Variables

Available template variables:
- `$DS_PROMETHEUS` - Prometheus data source
- Custom variables can be added for filtering by:
  - Session ID
  - Node type
  - Operation type
  - Time ranges

## Best Practices

1. **Refresh Rates**:
   - Overview: 10s (low overhead)
   - Operations: 10s (balanced)
   - Performance: 5s (detailed monitoring)
   - Streaming: 5s (real-time tracking)

2. **Time Ranges**:
   - Use appropriate time windows for your use case
   - Enable auto-refresh for live monitoring
   - Adjust based on data retention policies

3. **Threshold Configuration**:
   - Adjust threshold values based on your SLAs
   - Update alert rules to match dashboard thresholds
   - Document threshold changes

4. **Performance**:
   - Limit the number of time series queries
   - Use rate() and increase() functions appropriately
   - Consider downsampling for long time ranges

## Troubleshooting

### No Data Displayed

1. Verify Prometheus is scraping the metrics endpoint
2. Check that metrics are being emitted by the application
3. Verify data source configuration in Grafana
4. Check time range selection

### Slow Dashboard Load

1. Reduce the time range
2. Increase the step interval
3. Optimize PromQL queries
4. Check Prometheus performance

### Missing Panels

1. Verify all metrics are being exported
2. Check Prometheus scrape configuration
3. Ensure proper metric naming
4. Verify no query syntax errors

## Version History

- **v1.0.0** (Current) - Initial dashboard release
  - Overview dashboard with 8 core panels
  - Comprehensive metrics coverage
  - Enterprise-grade visualization
  - Production-ready configuration

## Contributing

When contributing dashboard improvements:

1. Test thoroughly with real data
2. Follow Grafana best practices
3. Document panel purposes
4. Update this README
5. Export clean JSON (no personal data)

## License

These dashboards are part of the LLM Memory Graph project and follow the project's license (MIT OR Apache-2.0).

## Support

For issues or questions:
- GitHub Issues: https://github.com/globalbusinessadvisors/llm-memory-graph/issues
- Documentation: See project README
- Metrics: See `/src/observatory/prometheus.rs` for metric definitions
