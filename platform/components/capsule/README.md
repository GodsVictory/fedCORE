# Capsule Multi-Tenancy

Multi-tenant namespace isolation and resource quotas using Project Capsule.

## Overview

This component deploys **Capsule**, a Kubernetes operator that provides lightweight multi-tenancy through namespace grouping and resource quotas. Capsule enables platform teams to give tenant owners self-service capabilities while enforcing resource boundaries and isolation.

## What Capsule Provides

- **Tenant-scoped namespace management**: Group namespaces under tenant ownership
- **Resource quota enforcement**: Limit CPU, memory, storage, and PVC usage per tenant
- **Network policy isolation**: Automatic network isolation between tenants
- **RBAC integration**: Tenant owners can create/manage their own namespaces
- **Namespace naming conventions**: Force tenant-specific namespace prefixes

## Architecture

```
┌─────────────────────────────────────────┐
│ Capsule Tenant (CRD)                    │
├─────────────────────────────────────────┤
│ spec:                                   │
│   owners: [User, Group]                 │
│   namespaceQuota: 10                    │
│   limitRanges:                          │
│     - default: 500m CPU, 512Mi memory   │
│   resourceQuotas:                       │
│     - limits.cpu: "100"                 │
│     - limits.memory: "200Gi"            │
│   networkPolicies: [...]                │
└─────────────────────────────────────────┘
         │
         ├── acme-app (Namespace)
         ├── acme-cicd (Namespace)
         └── acme-staging (Namespace)
```

When a tenant is onboarded via the `TenantOnboarding` RGD, KRO creates a Capsule Tenant resource that defines:
- Which namespaces belong to the tenant
- Resource quotas and limits for all tenant namespaces
- Network policies for isolation

## Deployment

This component is deployed automatically as part of the infrastructure stack.

**Build:**
```bash
# Build this component for a specific cluster
fedcore build platform/components/capsule platform/clusters/fedcore-prod-use1 > capsule-fedcore-prod-use1.yaml
```

**Prerequisites:**
- None (Capsule is a foundational component)

## Configuration

The Capsule operator is configured via Helm values in [base/capsule.yaml](base/capsule.yaml:1):

### Resource Limits

```yaml
manager:
  resources:
    limits:
      cpu: 200m
      memory: 256Mi
    requests:
      cpu: 100m
      memory: 128Mi
```

### Tenant Options

```yaml
options:
  # Force tenant owners to use specific namespace naming
  forceTenantPrefix: true

  # Enable tenant resource quota enforcement
  enableTenantResourceQuota: true
```

### Webhook Timeouts

```yaml
mutatingWebhooksTimeoutSeconds: 30
validatingWebhooksTimeoutSeconds: 30
```

## How Tenants Use Capsule

Tenants don't interact with Capsule directly. Instead:

1. Platform team creates a `TenantOnboarding` resource
2. KRO creates a Capsule `Tenant` with quotas and owners
3. Tenant owners can now create namespaces within their quota
4. Capsule automatically enforces resource limits and network policies

Example tenant namespace creation:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: acme-newapp
  labels:
    capsule.clastix.io/tenant: acme
```

Capsule automatically:
- Associates the namespace with the `acme` tenant
- Applies resource quotas and limit ranges
- Configures network policies for isolation
- Grants tenant owners RBAC permissions

## Security Features

### Namespace Isolation

Capsule automatically creates network policies that:
- Deny cross-tenant pod-to-pod traffic
- Allow intra-tenant communication
- Permit ingress from external sources (controlled by ingress controllers)

### Resource Quotas

Capsule enforces hard limits at the tenant level:
- CPU and memory quotas prevent resource exhaustion
- PVC quotas prevent storage abuse
- Namespace quotas prevent namespace sprawl

### RBAC Enforcement

Capsule integrates with Kubernetes RBAC:
- Tenant owners get admin access to their namespaces only
- Platform team retains cluster-admin access
- Tenant users cannot escalate privileges

## Monitoring

If monitoring is enabled in your cluster configuration, Capsule exposes Prometheus metrics:

```yaml
serviceMonitor:
  enabled: true
  labels:
    prometheus: kube-prometheus
```

**Key metrics:**
- `capsule_tenant_namespaces`: Number of namespaces per tenant
- `capsule_tenant_quota_usage`: Resource quota usage per tenant

## Troubleshooting

### Tenant Cannot Create Namespace

**Symptom:** `kubectl create namespace` fails with quota exceeded error

**Solution:**
1. Check tenant quota:
   ```bash
   kubectl get tenant <tenant-name> -o yaml
   ```

2. Verify current namespace count:
   ```bash
   kubectl get namespaces -l capsule.clastix.io/tenant=<tenant-name>
   ```

3. Increase quota if necessary (edit `TenantOnboarding` resource)

### Resource Quota Conflicts

**Symptom:** Pods fail to schedule due to quota violations

**Cause:** Capsule enforces both namespace-level and tenant-level quotas

**Solution:**
1. Check tenant-level quotas:
   ```bash
   kubectl get tenant <tenant-name> -o jsonpath='{.spec.resourceQuotas}'
   ```

2. Check namespace-level quotas:
   ```bash
   kubectl get resourcequota -n <namespace>
   ```

3. Adjust tenant quotas in `TenantOnboarding` spec

### Webhook Timeout Errors

**Symptom:** Namespace creation hangs or times out

**Cause:** Capsule webhooks are slow or unavailable

**Solution:**
1. Check Capsule pod status:
   ```bash
   kubectl get pods -n capsule-system
   ```

2. Review webhook logs:
   ```bash
   kubectl logs -n capsule-system -l app.kubernetes.io/name=capsule
   ```

3. Increase webhook timeouts in Helm values if needed

## Customization

### Adjusting Global Defaults

To change default resource limits or namespace quotas, edit [base/capsule.yaml](base/capsule.yaml:1) and rebuild:

```yaml
options:
  forceTenantPrefix: true
  enableTenantResourceQuota: true
  # Add new options here
```

Then rebuild the artifact:
```bash
fedcore build platform/components/capsule platform/clusters/fedcore-prod-use1 > capsule-fedcore-prod-use1.yaml
```

### Per-Tenant Customization

Tenant-specific quotas are defined in the `TenantOnboarding` resource:

```yaml
apiVersion: platform.fedcore.io/v1
kind: TenantOnboarding
spec:
  tenantName: acme
  quotas:
    namespaces: 10
    cpu: "100"
    memory: "200Gi"
    storage: "1Ti"
    maxPVCs: 100
```

## Related Documentation

- [Tenant Onboarding RGD](../../rgds/tenant/README.md)
- [Capsule Official Documentation](https://capsule.clastix.io)
- [Kubernetes Network Policies](https://kubernetes.io/docs/concepts/services-networking/network-policies/)
- [Kubernetes Resource Quotas](https://kubernetes.io/docs/concepts/policy/resource-quotas/)

---

**Status:** ✅ Production ready
