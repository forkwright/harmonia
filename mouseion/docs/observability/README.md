# Observability Guide

Mouseion includes comprehensive observability features powered by OpenTelemetry, enabling metrics collection, distributed tracing, and integration with popular observability platforms.

## Table of Contents

- [Overview](#overview)
- [Configuration](#configuration)
- [Metrics](#metrics)
- [Distributed Tracing](#distributed-tracing)
- [Prometheus Integration](#prometheus-integration)
- [Jaeger Integration](#jaeger-integration)
- [Grafana Dashboards](#grafana-dashboards)
- [Best Practices](#best-practices)

## Overview

Mouseion's observability stack includes:

- **Metrics**: Counters, histograms, and gauges for API performance, media operations, database queries, and more
- **Distributed Tracing**: End-to-end request tracing across service boundaries
- **Exporters**: Support for Prometheus, OTLP (Jaeger, Grafana Tempo), and console output

## Configuration

Configure telemetry in `appsettings.json`:

```json
{
  "Telemetry": {
    "Enabled": true,
    "EnablePrometheus": true,
    "EnableConsoleExporter": false,
    "OtlpEndpoint": "http://localhost:4317",
    "ServiceInstanceId": "mouseion-prod-1",
    "ResourceAttributes": {
      "deployment.environment": "production",
      "service.namespace": "media-stack"
    }
  }
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `Enabled` | bool | `true` | Enable/disable all telemetry collection |
| `EnablePrometheus` | bool | `true` | Expose `/metrics` endpoint for Prometheus scraping |
| `EnableConsoleExporter` | bool | `false` | Output telemetry to console (debugging) |
| `OtlpEndpoint` | string | `""` | OTLP endpoint for Jaeger/Tempo (e.g., `http://localhost:4317`) |
| `ServiceInstanceId` | string | hostname | Unique identifier for this service instance |
| `ResourceAttributes` | object | `{}` | Additional resource attributes for all telemetry |

### Environment Variables

Override configuration with environment variables:

```bash
TELEMETRY__ENABLED=true
TELEMETRY__ENABLEPROMETHEUS=true
TELEMETRY__OTLPENDPOINT=http://jaeger:4317
```

## Metrics

### Available Metrics

#### API Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_api_requests_total` | Counter | Total API requests (labels: endpoint, method, status_code) |
| `mouseion_api_errors_total` | Counter | Total API errors (labels: endpoint, error_type, exception_type) |
| `mouseion_api_request_duration_milliseconds` | Histogram | API request latency |

#### Media Operations

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_media_imports_total` | Counter | Media imports (labels: media_type, status) |
| `mouseion_media_downloads_total` | Counter | Media downloads (labels: media_type, status, client) |
| `mouseion_transcodes_total` | Counter | Transcode operations (labels: media_type, format, status) |
| `mouseion_import_duration_milliseconds` | Histogram | Import operation duration |
| `mouseion_import_file_size_bytes` | Histogram | Size of imported files |

#### Metadata Providers

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_metadata_requests_total` | Counter | Metadata API requests (labels: provider, operation, success) |
| `mouseion_metadata_errors_total` | Counter | Metadata API errors |
| `mouseion_metadata_request_duration_milliseconds` | Histogram | Metadata API latency |

#### Download Clients

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_download_client_requests_total` | Counter | Download client API calls (labels: client, operation, success) |
| `mouseion_download_client_response_time_milliseconds` | Histogram | Download client response time |

#### Database

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_database_queries_total` | Counter | Database queries (labels: operation, table) |
| `mouseion_database_query_duration_milliseconds` | Histogram | Query execution time |
| `mouseion_database_connections_total` | Counter | Database connections opened |

#### Cache

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_cache_hits_total` | Counter | Cache hits (labels: cache) |
| `mouseion_cache_misses_total` | Counter | Cache misses (labels: cache) |
| `mouseion_cache_evictions_total` | Counter | Cache evictions (labels: cache, reason) |

#### Background Jobs

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_jobs_total` | Counter | Jobs executed (labels: job_name, job_type, success) |
| `mouseion_job_failures_total` | Counter | Failed jobs |
| `mouseion_job_duration_milliseconds` | Histogram | Job execution time |
| `mouseion_active_jobs` | Gauge | Currently running jobs |

#### Library

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_library_items` | Gauge | Library item counts (labels: type) |

#### Indexers

| Metric | Type | Description |
|--------|------|-------------|
| `mouseion_indexer_searches_total` | Counter | Indexer searches (labels: indexer, search_type, success) |
| `mouseion_indexer_search_duration_milliseconds` | Histogram | Search duration |
| `mouseion_indexer_results_total` | Counter | Results returned |

## Distributed Tracing

### Trace Propagation

Mouseion automatically propagates trace context across:

- Incoming HTTP requests
- Outgoing HTTP calls to metadata providers
- Database operations
- Background job execution
- SignalR connections

### Span Attributes

All spans include:

- `mouseion.endpoint`: Normalized API endpoint
- `mouseion.api_version`: API version (v3)
- `http.method`: HTTP method
- `http.status_code`: Response status code
- `db.system`: Database type (sqlite/postgresql)
- `db.operation`: SQL operation type
- `job.name`: Background job name
- `job.type`: Job trigger type (scheduled/manual)

### Custom Instrumentation

Add custom spans in your code:

```csharp
using Mouseion.Common.Instrumentation;

// Simple span
using var activity = MouseionMetrics.StartActivity("custom_operation");

// Database span
using var dbActivity = MouseionMetrics.StartDatabaseActivity("SELECT", "Movies");

// HTTP client span
using var httpActivity = MouseionMetrics.StartHttpActivity("GET", url, "tmdb");

// Job span
using var jobActivity = MouseionMetrics.StartJobActivity("CustomJob", "manual");
```

Record metrics:

```csharp
// API metrics
MouseionMetrics.RecordApiRequest("/api/v3/movies", "GET", 200, durationMs);

// Media operations
MouseionMetrics.RecordMediaImport("movie", "success", durationMs, fileSizeBytes);
MouseionMetrics.RecordMediaDownload("music", "completed", "qbittorrent");

// Metadata providers
MouseionMetrics.RecordMetadataRequest("tmdb", "search", true, durationMs);

// Database
MouseionMetrics.RecordDatabaseQuery("SELECT", "Movies", durationMs, rowCount);

// Cache
MouseionMetrics.RecordCacheHit("metadata", key);
MouseionMetrics.RecordCacheMiss("metadata", key);

// Jobs
MouseionMetrics.RecordJobStarted("DiskScan", "scheduled");
MouseionMetrics.RecordJobCompleted("DiskScan", "scheduled", true, durationMs);
```

## Prometheus Integration

### Scrape Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mouseion'
    scrape_interval: 15s
    static_configs:
      - targets: ['mouseion:7878']
    metrics_path: /metrics
```

### Useful PromQL Queries

**API Request Rate:**
```promql
rate(mouseion_api_requests_total[5m])
```

**API Latency P95:**
```promql
histogram_quantile(0.95, rate(mouseion_api_request_duration_milliseconds_bucket[5m]))
```

**Error Rate:**
```promql
rate(mouseion_api_errors_total[5m]) / rate(mouseion_api_requests_total[5m])
```

**Metadata Provider Success Rate:**
```promql
rate(mouseion_metadata_requests_total{success="true"}[5m]) / rate(mouseion_metadata_requests_total[5m])
```

**Active Jobs:**
```promql
mouseion_active_jobs
```

**Cache Hit Rate:**
```promql
rate(mouseion_cache_hits_total[5m]) / (rate(mouseion_cache_hits_total[5m]) + rate(mouseion_cache_misses_total[5m]))
```

## Jaeger Integration

### Docker Compose Example

```yaml
version: '3.8'
services:
  mouseion:
    image: mouseion:latest
    environment:
      - TELEMETRY__OTLPENDPOINT=http://jaeger:4317
    depends_on:
      - jaeger

  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC
    environment:
      - COLLECTOR_OTLP_ENABLED=true
```

Access Jaeger UI at http://localhost:16686

## Grafana Dashboards

### Example Dashboard JSON

Save as `mouseion-dashboard.json` and import into Grafana:

```json
{
  "dashboard": {
    "title": "Mouseion Overview",
    "panels": [
      {
        "title": "API Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(mouseion_api_requests_total[5m])) by (endpoint)",
            "legendFormat": "{{endpoint}}"
          }
        ]
      },
      {
        "title": "API Latency (P95)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(mouseion_api_request_duration_milliseconds_bucket[5m])) by (le, endpoint))",
            "legendFormat": "{{endpoint}}"
          }
        ]
      },
      {
        "title": "Library Items",
        "type": "stat",
        "targets": [
          {
            "expr": "mouseion_library_items",
            "legendFormat": "{{type}}"
          }
        ]
      },
      {
        "title": "Metadata Provider Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(mouseion_metadata_request_duration_milliseconds_bucket[5m])) by (le, provider))",
            "legendFormat": "{{provider}}"
          }
        ]
      },
      {
        "title": "Active Background Jobs",
        "type": "gauge",
        "targets": [
          {
            "expr": "mouseion_active_jobs"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "type": "gauge",
        "targets": [
          {
            "expr": "sum(rate(mouseion_cache_hits_total[5m])) / (sum(rate(mouseion_cache_hits_total[5m])) + sum(rate(mouseion_cache_misses_total[5m])))"
          }
        ]
      }
    ]
  }
}
```

## Best Practices

### 1. Use Consistent Labels

Keep metric cardinality manageable by using normalized endpoints:
- ✅ `/api/v3/movies/{id}`
- ❌ `/api/v3/movies/12345`

### 2. Set Appropriate Histogram Buckets

The default histogram buckets work for most use cases. For specific needs, configure custom buckets.

### 3. Monitor Error Rates

Set up alerts for:
- API error rate > 1%
- Metadata provider failure rate > 10%
- Job failure rate > 5%

### 4. Use Trace Sampling in Production

For high-traffic deployments, configure trace sampling to reduce overhead:

```json
{
  "Telemetry": {
    "TraceSamplingRatio": 0.1
  }
}
```

### 5. Correlate Logs and Traces

Include trace IDs in log output for easy correlation:

```csharp
Log.Information("Processing request {TraceId}", Activity.Current?.TraceId);
```

## Troubleshooting

### Metrics Not Appearing

1. Verify `Telemetry:Enabled` is `true`
2. Check `/metrics` endpoint is accessible
3. Verify Prometheus scrape configuration

### Traces Not Appearing in Jaeger

1. Verify `OtlpEndpoint` is correctly configured
2. Check network connectivity to OTLP collector
3. Ensure Jaeger OTLP receiver is enabled

### High Cardinality Warning

If you see cardinality warnings:
1. Check for unbounded label values
2. Ensure endpoints are normalized (no dynamic IDs)
3. Review custom metric implementations
