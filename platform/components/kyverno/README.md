# Kyverno Admission Controller

Policy engine for Kubernetes resource validation, mutation, and generation.

## Overview

This component deploys **Kyverno**, a Kubernetes-native policy engine that enforces security, compliance, and operational standards through admission control. Kyverno validates, mutates, and generates Kubernetes resources without requiring any external dependencies.

**Note:** This component installs the Kyverno operator itself. Policy definitions are deployed separately via the [kyverno-policies component](../kyverno-policies/README.md).

## What Kyverno Provides

- **Admission Control**: Validate and mutate resources at creation time
- **Background Scanning**: Continuously evaluate existing resources against policies
- **Policy Reports**: Generate reports on policy violations across the cluster
- **Policy Exceptions**: Allow selective exemptions from policies
- **Webhook-based**: No external dependencies or policy storage required

## Architecture

```
┌─────────────────────────────────────────────────────┐
│ Kyverno Architecture                                │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌────────────────────┐   ┌──────────────────────┐ │
│  │ Admission          │   │ Background           │ │
│  │ Controller         │   │ Controller           │ │
│  │ (2 replicas)       │   │ (1 replica)          │ │
│  │                    │   │                      │ │
│  │ • Validates pods   │   │ • Policy reports     │ │
│  │ • Mutates resources│   │ • Background scan    │ │
│  │ • Generates configs│   │ • Existing resources │ │
│  └────────────────────┘   └──────────────────────┘ │
│                                                     │
│  ┌────────────────────┐   ┌──────────────────────┐ │
│  │ Reports            │   │ Cleanup              │ │
│  │ Controller         │   │ Controller           │ │
│  │ (1 replica)        │   │ (1 replica)          │ │
│  │                    │   │                      │ │
│  │ • PolicyReport CRDs│   │ • Webhooks cleanup   │ │
│  │ • Violation tracking│   │ • Resource cleanup  │ │
│  └────────────────────┘   └──────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

Kyverno components work together to:
1. **Admission Controller**: Intercepts resource creation/updates via webhooks
2. **Background Controller**: Scans existing resources and generates policy reports
3. **Reports Controller**: Creates and maintains PolicyReport CRDs
4. **Cleanup Controller**: Manages webhook configurations and resource lifecycle

## Deployment

This component is deployed automatically as part of the infrastructure stack.

**Build:**
```bash
# Build this component for a specific cluster
fedcore build platform/components/kyverno platform/clusters/fedcore-prod-use1 > kyverno-fedcore-prod-use1.yaml
```

**Prerequisites:**
- None (Kyverno is a foundational component)

**Deployment Order:**
1. Install Kyverno operator (this component) first
2. Then deploy kyverno-policies component with policy definitions

## Configuration

The Kyverno operator is configured via Helm values in [base/kyverno.yaml](base/kyverno.yaml:1):

### Admission Controller

```yaml
admissionController:
  replicas: 2  # High availability
  resources:
    limits:
      cpu: 500m
      memory: 512Mi
    requests:
      cpu: 200m
      memory: 256Mi
```

**Why 2 replicas?** Ensures admission control availability during rolling updates.

### Background Controller

```yaml
backgroundController:
  enabled: true
  replicas: 1
```

Continuously scans existing resources and generates policy reports.

### Reports Controller

```yaml
reportsController:
  enabled: true
```

Creates `PolicyReport` and `ClusterPolicyReport` CRDs for violation tracking.

### Feature Flags

```yaml
features:
  policyExceptions:
    enabled: true
    namespace: kyverno
  backgroundScan:
    enabled: true
    interval: 1h
  reports:
    chunkSize: 1000
```

## How It Works

### Admission Control Flow

```
1. User submits resource (kubectl apply)
   ↓
2. API server sends AdmissionReview to Kyverno webhook
   ↓
3. Kyverno evaluates resource against all ClusterPolicies
   ↓
4. Kyverno returns response:
   - Allow (possibly with mutations)
   - Deny (with violation message)
   ↓
5. API server creates/rejects resource based on response
```

### Background Scanning

Every hour (configurable), the background controller:
1. Lists all resources in the cluster
2. Evaluates them against policies with `background: true`
3. Generates PolicyReport CRDs with results
4. Updates violation metrics for monitoring

## Webhook Configuration

Kyverno registers admission webhooks that intercept resource operations. The webhook is configured to skip system namespaces:

```yaml
config:
  webhooks:
    - namespaceSelector:
        matchExpressions:
          - key: kubernetes.io/metadata.name
            operator: NotIn
            values:
              - kube-system
              - kube-public
              - kube-node-lease
              - flux-system
```

**Why skip these namespaces?** Prevents policy enforcement on infrastructure components that may not comply with tenant policies.

## Monitoring

If monitoring is enabled in your cluster configuration, Kyverno exposes Prometheus metrics:

```yaml
serviceMonitor:
  enabled: true
  labels:
    prometheus: kube-prometheus
```

**Key metrics:**
- `kyverno_policy_results_total`: Total policy evaluations by result (pass/fail)
- `kyverno_policy_execution_duration_seconds`: Policy evaluation latency
- `kyverno_admission_requests_total`: Admission webhook requests
- `kyverno_policy_changes_total`: Policy changes over time

## Troubleshooting

### Admission Webhook Failures

**Symptom:** `kubectl apply` hangs or times out

**Cause:** Kyverno webhook is unavailable

**Solution:**
1. Check Kyverno pods:
   ```bash
   kubectl get pods -n kyverno
   ```

2. Review admission controller logs:
   ```bash
   kubectl logs -n kyverno -l app.kubernetes.io/component=admission-controller
   ```

3. Check webhook configuration:
   ```bash
   kubectl get validatingwebhookconfigurations kyverno-resource-validating-webhook-cfg
   kubectl get mutatingwebhookconfigurations kyverno-resource-mutating-webhook-cfg
   ```

4. If webhook is stuck, temporarily remove it:
   ```bash
   kubectl delete validatingwebhookconfiguration kyverno-resource-validating-webhook-cfg
   ```
   Then restart Kyverno to recreate it.

### Policy Reports Not Generated

**Symptom:** `kubectl get policyreport` returns no results

**Cause:** Background controller is disabled or not running

**Solution:**
1. Verify background controller is enabled:
   ```bash
   kubectl get deployment -n kyverno kyverno-background-controller
   ```

2. Check background controller logs:
   ```bash
   kubectl logs -n kyverno -l app.kubernetes.io/component=background-controller
   ```

3. Force policy re-evaluation:
   ```bash
   kubectl annotate clusterpolicy <policy-name> force-resync=true --overwrite
   ```

### High Memory Usage

**Symptom:** Kyverno pods consuming excessive memory

**Cause:** Large number of resources or complex policies

**Solution:**
1. Increase memory limits in Helm values:
   ```yaml
   admissionController:
     resources:
       limits:
         memory: 1Gi  # Increase from 512Mi
   ```

2. Reduce background scan frequency:
   ```yaml
   features:
     backgroundScan:
       interval: 2h  # Increase from 1h
   ```

3. Reduce report chunk size:
   ```yaml
   features:
     reports:
       chunkSize: 500  # Decrease from 1000
   ```

### CRD Installation Failures

**Symptom:** Kyverno Helm release fails with CRD errors

**Cause:** CRDs exist from previous installation

**Solution:**
```bash
# Remove old CRDs (this will delete existing policies!)
kubectl delete crds $(kubectl get crds | grep kyverno | awk '{print $1}')

# Reinstall Kyverno
flux reconcile helmrelease kyverno -n kyverno
```

## Customization

### Updating Kyverno Version

Edit [base/kyverno.yaml](base/kyverno.yaml:1) and update the chart version:

```yaml
spec:
  chart:
    spec:
      version: "3.2.6"  # Update this version
```

Then rebuild the artifact:
```bash
fedcore build platform/components/kyverno platform/clusters/fedcore-prod-use1 > kyverno-fedcore-prod-use1.yaml
```

### Adjusting Resource Limits

For larger clusters, increase resource allocations:

```yaml
admissionController:
  replicas: 3  # More replicas for high traffic
  resources:
    limits:
      cpu: 1000m
      memory: 1Gi
```

### Enabling Additional Features

Kyverno supports additional features that can be enabled:

```yaml
features:
  # Generate resources based on policy rules
  generateValidatingAdmissionPolicy:
    enabled: true

  # Integrate with external data sources
  configMapCaching:
    enabled: true
```

## Security Considerations

### Webhook Failure Mode

Kyverno webhooks are configured to **fail-closed** by default:
- If Kyverno is unavailable, resource creation is **blocked**
- This prevents bypassing policies during outages
- Ensure high availability with 2+ admission controller replicas

### RBAC Permissions

Kyverno requires cluster-admin permissions to:
- Register admission webhooks
- Read all resources for background scanning
- Create PolicyReport CRDs

**Limit who can modify Kyverno:**
```bash
# Only platform admins should have these permissions
kubectl auth can-i delete deployment kyverno -n kyverno --as=tenant-user
# Should return "no"
```

### Policy Bypass Prevention

Prevent users from bypassing policies:
1. Restrict access to Kyverno namespace via RBAC
2. Use Kyverno policies to validate other Kyverno policies
3. Monitor webhook configuration changes via audit logs

## Related Documentation

- [Kyverno Policies Component](../kyverno-policies/README.md) - Policy definitions
- [Security Policies Documentation](../../docs/SECURITY_POLICIES.md) - Platform security architecture
- [Kyverno Official Documentation](https://kyverno.io)
- [Kubernetes Admission Control](https://kubernetes.io/docs/reference/access-authn-authz/admission-controllers/)

## Component Dependencies

**This component must be deployed before:**
- [kyverno-policies](../kyverno-policies/README.md) - Requires Kyverno CRDs

**This component depends on:**
- None (foundational component)

---

**Status:** ✅ Production ready
