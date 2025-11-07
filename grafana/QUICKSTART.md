# Quick Start: Monitoring Stack Deployment

This guide will help you quickly deploy the complete monitoring stack for LLM Memory Graph.

## Prerequisites

- Docker and Docker Compose installed
- LLM Memory Graph application running
- Basic familiarity with Prometheus and Grafana

## Option 1: Docker Compose (Recommended)

### 1. Create Docker Compose Configuration

Create `docker-compose.monitoring.yml`:

```yaml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    container_name: llm-memory-graph-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/usr/share/prometheus/console_libraries'
      - '--web.console.templates=/usr/share/prometheus/consoles'
    restart: unless-stopped
    networks:
      - monitoring

  grafana:
    image: grafana/grafana:latest
    container_name: llm-memory-graph-grafana
    ports:
      - "3000:3000"
    volumes:
      - ./grafana:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources.yml:/etc/grafana/provisioning/datasources/datasources.yml
      - grafana-data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_SERVER_DOMAIN=localhost
      - GF_SMTP_ENABLED=false
    restart: unless-stopped
    depends_on:
      - prometheus
    networks:
      - monitoring

volumes:
  prometheus-data:
  grafana-data:

networks:
  monitoring:
    driver: bridge
```

### 2. Create Prometheus Configuration

Create `prometheus/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: 'llm-memory-graph'
    environment: 'production'

scrape_configs:
  - job_name: 'llm-memory-graph'
    static_configs:
      - targets: ['host.docker.internal:9090']  # Adjust to your app's metrics port
        labels:
          app: 'llm-memory-graph'
          instance: 'main'

    # Optional: Add custom metrics paths if needed
    metrics_path: '/metrics'

    # Optional: Add basic auth if you've secured your metrics endpoint
    # basic_auth:
    #   username: 'prometheus'
    #   password: 'your-password'

  # Add more jobs for different instances/environments
  # - job_name: 'llm-memory-graph-staging'
  #   static_configs:
  #     - targets: ['staging-host:9090']
```

### 3. Create Grafana Data Source Configuration

Create `grafana/datasources.yml`:

```yaml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: false
    jsonData:
      httpMethod: POST
      timeInterval: 15s
```

### 4. Deploy the Stack

```bash
# Start the monitoring stack
docker-compose -f docker-compose.monitoring.yml up -d

# Check logs
docker-compose -f docker-compose.monitoring.yml logs -f

# Stop the stack
docker-compose -f docker-compose.monitoring.yml down
```

### 5. Access Services

- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090

### 6. Import Dashboards

1. Login to Grafana (http://localhost:3000)
2. Go to Dashboards → Import
3. Upload `memory-graph-overview.json`
4. Select "Prometheus" as the data source
5. Click "Import"

## Option 2: Kubernetes Deployment

### 1. Create Namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: llm-memory-graph-monitoring
```

### 2. Deploy Prometheus

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: llm-memory-graph-monitoring
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
    scrape_configs:
      - job_name: 'llm-memory-graph'
        kubernetes_sd_configs:
          - role: pod
            namespaces:
              names:
                - llm-memory-graph
        relabel_configs:
          - source_labels: [__meta_kubernetes_pod_label_app]
            action: keep
            regex: llm-memory-graph
          - source_labels: [__meta_kubernetes_pod_name]
            target_label: instance
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: prometheus
  namespace: llm-memory-graph-monitoring
spec:
  replicas: 1
  selector:
    matchLabels:
      app: prometheus
  template:
    metadata:
      labels:
        app: prometheus
    spec:
      containers:
      - name: prometheus
        image: prom/prometheus:latest
        ports:
        - containerPort: 9090
        volumeMounts:
        - name: config
          mountPath: /etc/prometheus
        - name: data
          mountPath: /prometheus
      volumes:
      - name: config
        configMap:
          name: prometheus-config
      - name: data
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: prometheus
  namespace: llm-memory-graph-monitoring
spec:
  selector:
    app: prometheus
  ports:
  - port: 9090
    targetPort: 9090
```

### 3. Deploy Grafana

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: grafana
  namespace: llm-memory-graph-monitoring
spec:
  replicas: 1
  selector:
    matchLabels:
      app: grafana
  template:
    metadata:
      labels:
        app: grafana
    spec:
      containers:
      - name: grafana
        image: grafana/grafana:latest
        ports:
        - containerPort: 3000
        env:
        - name: GF_SECURITY_ADMIN_PASSWORD
          valueFrom:
            secretKeyRef:
              name: grafana-admin
              key: password
        volumeMounts:
        - name: dashboards
          mountPath: /etc/grafana/provisioning/dashboards
        - name: datasources
          mountPath: /etc/grafana/provisioning/datasources
      volumes:
      - name: dashboards
        configMap:
          name: grafana-dashboards
      - name: datasources
        configMap:
          name: grafana-datasources
---
apiVersion: v1
kind: Service
metadata:
  name: grafana
  namespace: llm-memory-graph-monitoring
spec:
  type: LoadBalancer
  selector:
    app: grafana
  ports:
  - port: 80
    targetPort: 3000
```

### 4. Apply Kubernetes Manifests

```bash
kubectl apply -f monitoring-namespace.yaml
kubectl apply -f prometheus-deployment.yaml
kubectl apply -f grafana-deployment.yaml

# Check status
kubectl get pods -n llm-memory-graph-monitoring

# Get Grafana URL
kubectl get svc -n llm-memory-graph-monitoring grafana
```

## Option 3: Manual Installation

### 1. Install Prometheus

```bash
# Download Prometheus
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
tar xvfz prometheus-2.45.0.linux-amd64.tar.gz
cd prometheus-2.45.0.linux-amd64

# Create configuration
cat > prometheus.yml <<EOF
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'llm-memory-graph'
    static_configs:
      - targets: ['localhost:9090']
EOF

# Start Prometheus
./prometheus --config.file=prometheus.yml
```

### 2. Install Grafana

```bash
# On Ubuntu/Debian
sudo apt-get install -y adduser libfontconfig1
wget https://dl.grafana.com/enterprise/release/grafana-enterprise_10.0.0_amd64.deb
sudo dpkg -i grafana-enterprise_10.0.0_amd64.deb
sudo systemctl start grafana-server
sudo systemctl enable grafana-server

# On macOS
brew install grafana
brew services start grafana

# Access Grafana at http://localhost:3000
```

## Exposing Metrics from Your Application

### Rust Application Setup

```rust
use llm_memory_graph::observatory::prometheus::PrometheusMetrics;
use prometheus::{Encoder, Registry, TextEncoder};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Prometheus registry
    let registry = Registry::new();

    // Create metrics
    let metrics = Arc::new(PrometheusMetrics::new(&registry)?);

    // Create your memory graph with metrics
    let graph = AsyncMemoryGraph::new(config).await?;

    // Setup metrics endpoint
    let metrics_route = warp::path("metrics")
        .map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = vec![];
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });

    // Start server
    warp::serve(metrics_route)
        .run(([0, 0, 0, 0], 9090))
        .await;

    Ok(())
}
```

### Alternative: Using Actix-Web

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use prometheus::{Encoder, TextEncoder};

async fn metrics_handler(registry: web::Data<Registry>) -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(buffer)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let registry = Registry::new();
    let metrics = PrometheusMetrics::new(&registry).unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(registry.clone()))
            .route("/metrics", web::get().to(metrics_handler))
    })
    .bind(("0.0.0.0", 9090))?
    .run()
    .await
}
```

## Verification Steps

### 1. Verify Metrics Endpoint

```bash
curl http://localhost:9090/metrics

# Should see output like:
# memory_graph_nodes_created_total 12345
# memory_graph_write_latency_seconds_bucket{le="0.001"} 100
# ...
```

### 2. Verify Prometheus Scraping

1. Open Prometheus UI: http://localhost:9090
2. Go to Status → Targets
3. Verify "llm-memory-graph" job is UP
4. Test a query: `memory_graph_nodes_created_total`

### 3. Verify Grafana Dashboard

1. Open Grafana: http://localhost:3000
2. Login (admin/admin)
3. Go to Dashboards
4. Open "LLM Memory Graph - Overview"
5. Verify panels show data

## Troubleshooting

### Metrics Not Showing in Prometheus

1. Check if metrics endpoint is accessible:
   ```bash
   curl http://localhost:9090/metrics
   ```

2. Verify Prometheus configuration:
   ```bash
   docker exec llm-memory-graph-prometheus cat /etc/prometheus/prometheus.yml
   ```

3. Check Prometheus logs:
   ```bash
   docker logs llm-memory-graph-prometheus
   ```

### Grafana Shows No Data

1. Verify Prometheus data source:
   - Settings → Data Sources → Prometheus
   - Click "Test" button
   - Should show "Data source is working"

2. Check query in Prometheus first:
   - Open Prometheus UI
   - Run the same query used in Grafana panel
   - Verify data exists

3. Check time range:
   - Ensure dashboard time range includes when app was running
   - Try "Last 5 minutes" for recent data

### Docker Containers Won't Start

1. Check port conflicts:
   ```bash
   lsof -i :9090  # Prometheus
   lsof -i :3000  # Grafana
   ```

2. Check Docker logs:
   ```bash
   docker-compose -f docker-compose.monitoring.yml logs
   ```

3. Verify Docker network:
   ```bash
   docker network ls
   docker network inspect llm-memory-graph_monitoring
   ```

## Production Recommendations

1. **Security**:
   - Enable authentication on Prometheus
   - Use HTTPS/TLS for Grafana
   - Restrict network access
   - Use secrets management for passwords

2. **Persistence**:
   - Configure Prometheus retention (default: 15 days)
   - Use persistent volumes for both Prometheus and Grafana
   - Regular backups of Grafana dashboards

3. **High Availability**:
   - Run multiple Prometheus instances
   - Use Prometheus federation or Thanos
   - Deploy Grafana with database backend (PostgreSQL)

4. **Alerting**:
   - Configure Prometheus Alertmanager
   - Set up alert rules (see README.md)
   - Configure notification channels (Slack, PagerDuty, etc.)

5. **Performance**:
   - Adjust scrape intervals based on load
   - Use recording rules for expensive queries
   - Configure appropriate retention periods

## Next Steps

1. Import additional dashboards (Operations, Performance, Streaming)
2. Configure alerting rules
3. Set up notification channels
4. Customize dashboards for your specific needs
5. Enable authentication and HTTPS for production

## Support

For issues or questions:
- GitHub Issues: https://github.com/globalbusinessadvisors/llm-memory-graph/issues
- Documentation: See main README.md
- Dashboards: See grafana/README.md
