# Splunk OpenTelemetry Collector

Modern OpenTelemetry-based log and metrics collection for Kubernetes, replacing the deprecated `splunk-connect-for-kubernetes`.

## Overview

The Splunk OTel Collector provides:
- **Log Collection**: Container logs from all pods using the Filelog receiver
- **Metrics Collection**: Host metrics, kubelet stats, and cluster metrics
- **Tenant Awareness**: Automatic extraction of Capsule tenant labels
- **Multi-destination**: Simultaneous export to Splunk HEC and AppDynamics (when enabled)

## Features

### Log Collection
- Collects logs from all containers via filelog receiver
- Parses CRI-O/containerd log formats automatically
- Extracts Kubernetes metadata (namespace, pod, container, node)
- Captures Capsule tenant labels for multi-tenancy
- Indexes key fields for fast Splunk searches

### Metrics Collection (Optional)
- Host metrics (CPU, memory, disk, network)
- Kubelet stats (pod/container resource usage)
- Kubernetes cluster state
- Controlled by `monitoring.enabled` cluster configuration

### AppDynamics Integration (Optional)
- Automatically enabled when `appdynamics.ip_ranges` is configured
- Sends logs and metrics to AppDynamics OTLP endpoint
- Uses environment variables from Secret for credentials
- Dual-export: Data goes to both Splunk and AppDynamics

## Configuration

### Basic Setup

1. **Deploy the component** (add to cluster components list):
```yaml
components:
  - name: splunk-otel-collector
    enabled: true
    version: "0.119.0"
```

2. **Configure Splunk HEC endpoint** (in cloud-specific overlay):
```yaml
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.all
---
helm:
  values:
    splunkPlatform:
      endpoint: "https://your-splunk-hec.example.com:8088/services/collector"
      token: "your-hec-token-here"
      metricsEndpoint: "https://your-splunk-hec.example.com:8088/services/collector"
      metricsToken: "your-metrics-hec-token"
      index: "main"
      metricsIndex: "metrics"
```

### AppDynamics Integration

When `appdynamics.ip_ranges` is configured in your cluster config (indicating AppDynamics is in use), the collector automatically enables AppDynamics export.

1. **Create AppDynamics credentials Secret**:
```bash
kubectl create secret generic appdynamics-credentials \
  --from-literal=endpoint=https://your-tenant.saas.appdynamics.com/v1/logs \
  --from-literal=oauth_token=your-oauth-token \
  --from-literal=api_key=your-api-key \
  -n splunk-system
```

2. **Configure cluster with AppDynamics IP ranges**:
```yaml
appdynamics:
  ip_ranges:
    - "34.192.0.0/16"  # Your AppDynamics SaaS IP ranges
    - "52.20.0.0/16"
```

The collector will automatically:
- Add AppDynamics OTLP exporter to the pipeline
- Load credentials from the Secret
- Send logs and metrics to both Splunk and AppDynamics

### Indexed Fields

The following fields are extracted and available for Splunk searches:
- `tenant_name` - Capsule tenant (from `capsule.clastix.io/tenant` label)
- `namespace` - Kubernetes namespace
- `pod_name` - Pod name
- `container_name` - Container name
- `node_name` - Node name
- `cluster_name` - Cluster identifier
- `app_name` - Application name (from `app.kubernetes.io/name` label)

Example Splunk search:
```spl
index=main tenant_name="acme" namespace="acme-prod" | stats count by pod_name
```

## Architecture

### Agent DaemonSet
- Runs on every node as a DaemonSet
- Collects logs from `/var/log/pods/`
- Collects node-level metrics
- Processes and enriches data with Kubernetes metadata
- Exports directly to Splunk HEC (and optionally AppDynamics)

### Processing Pipeline

```
Logs: filelog → k8sattributes → resource → attributes → batch → exporters
Metrics: hostmetrics → k8sattributes → resource → batch → exporters
```

Key processors:
- **k8sattributes**: Enriches with pod metadata, labels, annotations
- **resource**: Adds cluster name, extracts tenant labels
- **attributes**: Renames K8s attributes to match Splunk conventions
- **batch**: Batches data for efficiency
- **memory_limiter**: Prevents OOM

## Resource Usage

Agent DaemonSet per node:
- **CPU**: 100m request, 500m limit
- **Memory**: 200Mi request, 500Mi limit

## Comparison with splunk-connect

| Feature | splunk-connect (deprecated) | splunk-otel-collector (new) |
|---------|----------------------------|----------------------------|
| Log collection | Fluent Bit | OpenTelemetry Filelog |
| Metrics | Splunk Metrics Collector | OTel Host/Kubelet metrics |
| Standards | Proprietary | OpenTelemetry (CNCF) |
| Multi-destination | No | Yes (Splunk + AppDynamics) |
| Active development | No (deprecated) | Yes |
| Configuration | Helm values | OTel Collector config |

## Troubleshooting

### Check collector status
```bash
kubectl get pods -n splunk-system
kubectl logs -n splunk-system -l app.kubernetes.io/name=splunk-otel-collector
```

### Verify configuration
```bash
kubectl get cm -n splunk-system splunk-otel-collector-otel-agent -o yaml
```

### Test connectivity
```bash
# From collector pod
kubectl exec -n splunk-system <pod-name> -- curl -v https://your-splunk-hec.example.com:8088
```

### Common issues

**No logs appearing in Splunk:**
- Check HEC endpoint and token in overlay
- Verify HEC token is enabled in Splunk
- Check collector logs for export errors

**AppDynamics not receiving data:**
- Verify `appdynamics-credentials` Secret exists in `splunk-system` namespace
- Check Secret contains valid endpoint, oauth_token, and api_key
- Verify `appdynamics.ip_ranges` is configured in cluster config

**High memory usage:**
- Adjust `memory_limiter` processor limits
- Reduce batch size in config

## References

- [Splunk OTel Collector Chart](https://github.com/signalfx/splunk-otel-collector-chart)
- [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/)
- [Splunk HEC Documentation](https://docs.splunk.com/Documentation/Splunk/latest/Data/UsetheHTTPEventCollector)
