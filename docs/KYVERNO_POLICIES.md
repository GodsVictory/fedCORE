# Kyverno Policies

## Overview

This document details all Kyverno admission control policies in the fedCORE platform. Kyverno provides validation and mutation of Kubernetes resources at admission time, enforcing security baselines, resource governance, and best practices.

The platform uses 20+ Kyverno policies organized into six categories:
1. **Container Security** - Pod Security Standards enforcement
2. **Network Security** - NetworkPolicy validation
3. **Supply Chain** - Image registry and tag restrictions
4. **Resource Management** - Quota, limit, and cost tracking
5. **Input Validation** - Tenant onboarding validation
6. **Best Practices** - Audit-mode recommendations

## Kyverno's Role in Security Architecture

**Purpose:** Validates and mutates resources at admission time

**Responsibilities:**
- ✅ Validate security policies (no privileged containers, etc.)
- ✅ Validate image restrictions (registry allowlist, no latest tags)
- ✅ Validate NetworkPolicies don't bypass isolation
- ✅ Validate quotas and limits exist before pod creation
- ✅ Mutate resources to add cost tracking labels
- ⚠️ Generate per-namespace ResourceQuotas (optional)

**Configuration Location:** [kyverno-policies/](../platform/components/kyverno-policies/base/)

**See Also:** [Security Overview - Capsule vs Kyverno Separation](SECURITY_OVERVIEW.md#architecture-capsule-vs-kyverno-separation-of-concerns)

## Container Security Policies

These policies enforce the Kubernetes Pod Security Standards (PSS) Restricted profile:

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Privileged Container Block** | Prevents privileged mode and privilege escalation via securityContext | **Enforced** - Deny | [tenant-security-baseline.yaml:14-58](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L14-L58) |
| **Non-Root Enforcement** | All containers must run as non-root users (runAsNonRoot: true) | **Enforced** - Deny | [tenant-security-baseline.yaml:60-95](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L60-L95) |
| **Capabilities Restriction** | Drops ALL capabilities, only allows safe ones (NET_BIND_SERVICE, CHOWN, DAC_OVERRIDE, SETGID, SETUID) | **Enforced** - Deny | [tenant-security-baseline.yaml:97-147](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L97-L147) |
| **Host Namespace Isolation** | Prevents sharing host network, PID, or IPC namespaces | **Enforced** - Deny | [tenant-security-baseline.yaml:183-231](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L183-L231) |
| **Host Port Blocking** | Prevents containers from binding to host ports | **Enforced** - Deny | [tenant-security-baseline.yaml:233-266](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L233-L266) |
| **Seccomp Profile Required** | Enforces RuntimeDefault or Localhost seccomp profiles | **Enforced** - Deny | [tenant-security-baseline.yaml:268-310](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L268-L310) |
| **Volume Type Restrictions** | Blocks hostPath and other dangerous volume types | **Enforced** - Deny | [tenant-security-baseline.yaml:149-181](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L149-L181) |
| **Sysctls Restriction** | Only allows safe kernel parameters, blocks unsafe sysctls | **Enforced** - Deny | [tenant-security-baseline.yaml:312-347](../platform/components/kyverno-policies/base/tenant-security-baseline.yaml#L312-L347) |

**Example Pod that Passes Security Policies:**

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: secure-app
  namespace: tenant-acme-prod
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    fsGroup: 1000
    seccompProfile:
      type: RuntimeDefault
  containers:
    - name: app
      image: nexus.fedcore.io/tenant-acme/app:v1.2.3
      securityContext:
        allowPrivilegeEscalation: false
        capabilities:
          drop:
            - ALL
        readOnlyRootFilesystem: true
      resources:
        requests:
          cpu: 100m
          memory: 128Mi
        limits:
          cpu: 500m
          memory: 512Mi
```

## Network Security Policies

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Cross-Tenant Prevention Validation** | Validates tenants cannot create NetworkPolicies that bypass isolation | **Enforced** - Kyverno deny | [tenant-network-policies.yaml](../platform/components/kyverno-policies/base/tenant-network-policies.yaml) |

**Note:** NetworkPolicy generation is handled by Capsule (see [Security Overview](SECURITY_OVERVIEW.md)). Kyverno validates that tenants cannot create NetworkPolicies that would bypass tenant isolation.

## Istio Service Mesh Policies

When Istio is enabled for a tenant, these policies enforce mTLS and prevent security bypasses:

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Strict mTLS Enforcement** | Tenant PeerAuthentication policies must use STRICT mode (no plaintext allowed) | **Enforced** - Deny | [istio-tenant-policies.yaml:14-49](../platform/components/kyverno-policies/base/istio-tenant-policies.yaml#L14-L49) |
| **Tenant AuthorizationPolicy Validation** | Prevents tenants from creating policies that allow cross-tenant traffic | **Enforced** - Deny | [istio-tenant-policies.yaml:51-95](../platform/components/kyverno-policies/base/istio-tenant-policies.yaml#L51-L95) |
| **Istio System Protection** | Blocks tenant modifications to istio-system namespace | **Enforced** - Deny | [istio-tenant-policies.yaml:97-134](../platform/components/kyverno-policies/base/istio-tenant-policies.yaml#L97-L134) |
| **DestinationRule TLS Enforcement** | Prevents tenants from disabling TLS via DestinationRule | **Enforced** - Deny | [istio-tenant-policies.yaml:220-247](../platform/components/kyverno-policies/base/istio-tenant-policies.yaml#L220-L247) |

**See Also:** [Runtime Security - Istio mTLS Architecture](RUNTIME_SECURITY.md#istio-mtls-architecture)

## Supply Chain Security Policies

These policies enforce image provenance and prevent supply chain attacks:

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Registry Restriction** | Only approved container registries allowed (configurable whitelist) | **Enforce (Prod)** / Audit (Dev/Staging) | [tenant-image-registry.yaml:14-71](../platform/components/kyverno-policies/base/tenant-image-registry.yaml#L14-L71) |
| **Latest Tag Prohibition** | Enforces semantic versioning, blocks "latest" tags | **Enforce (Prod)** / Audit (Dev/Staging) | [tenant-image-registry.yaml:73-105](../platform/components/kyverno-policies/base/tenant-image-registry.yaml#L73-L105) |
| **Image Signature Verification** | Optional Cosign/Sigstore integration for signed images | **Optional** - Configurable | [tenant-image-registry.yaml:107-167](../platform/components/kyverno-policies/base/tenant-image-registry.yaml#L107-L167) |

**Environment-Specific Configuration:**

Production:
```yaml
# Only nexus.fedcore.io allowed
# Latest tag blocked
disallow_latest_tag: true
enforce_image_registry: true
```

Development:
```yaml
# docker.io, ghcr.io also allowed
# Latest tag permitted for testing
disallow_latest_tag: false
enforce_image_registry: false
```

## Resource Management Policies

### Enforcement Policies

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Mandatory Resource Limits** | CPU and memory requests/limits required on all containers | **Enforce (Prod)** / Audit (Dev/Staging) | [tenant-resource-limits.yaml:16-70](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L16-L70) |
| **ResourceQuota Validation** | Blocks pod creation if namespace missing ResourceQuota | **Enforced** - Deny | [tenant-resource-limits.yaml:311-366](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L311-L366) |
| **LimitRange Validation** | Blocks pod creation if namespace missing LimitRange | **Enforced** - Deny | [tenant-resource-limits.yaml:368-422](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L368-L422) |
| **Expensive Resource Approval** | LoadBalancers and large PVCs require approval annotation | **Enforced** - Deny | [tenant-resource-limits.yaml:128-253](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L128-L253) |

### Mutation Policies

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Cost Tracking Labels** | Automatic tagging for chargeback and cost allocation | **Enforced** - Mutates | [tenant-resource-limits.yaml:255-309](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L255-L309) |
| **Per-Namespace ResourceQuota** | Optional per-namespace quotas within tenant | **Enforced** - Generates | [tenant-resource-limits.yaml:72-126](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L72-L126) |

### Audit Policies

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Namespace Quota Readiness Audit** | Reports namespaces missing ResourceQuota or LimitRange | **Audit** - Report only | [tenant-resource-limits.yaml:424-483](../platform/components/kyverno-policies/base/tenant-resource-limits.yaml#L424-L483) |

**Cost Tracking Labels Added Automatically:**

```yaml
metadata:
  labels:
    platform.fedcore.io/tenant: acme
    platform.fedcore.io/cluster: fedcore-prod-use1
    platform.fedcore.io/cost-center: engineering
    platform.fedcore.io/billing-contact: finance@acme-corp.com
```

## Input Validation Policies

These policies validate TenantOnboarding CRs before tenant creation:

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Name Format Validation** | DNS-compliant tenant naming (1-30 characters) | **Enforced** - Deny | [tenant-onboarding-validation.yaml:14-56](../platform/components/kyverno-policies/base/tenant-onboarding-validation.yaml#L14-L56) |
| **Quota Format Validation** | Proper Kubernetes resource format validation | **Enforced** - Deny | [tenant-onboarding-validation.yaml:58-141](../platform/components/kyverno-policies/base/tenant-onboarding-validation.yaml#L58-L141) |
| **Billing Information Validation** | Cost center and contact information required | **Enforced** - Deny | [tenant-onboarding-validation.yaml:143-193](../platform/components/kyverno-policies/base/tenant-onboarding-validation.yaml#L143-L193) |
| **Owner Requirements** | At least one owner required per tenant | **Enforced** - Deny | [tenant-onboarding-validation.yaml:195-226](../platform/components/kyverno-policies/base/tenant-onboarding-validation.yaml#L195-L226) |

## Best Practices Policies (Audit Mode)

These policies generate warnings and audit reports but do not block resources:

| Policy | Description | Enforcement | Policy File |
|--------|-------------|-------------|-------------|
| **Readiness/Liveness Probes** | Recommendations for health checks | **Audit** - Warn only | [tenant-best-practices.yaml:14-49](../platform/components/kyverno-policies/base/tenant-best-practices.yaml#L14-L49) |
| **Pod Disruption Budget** | Suggestions for high availability | **Audit** - Warn only | [tenant-best-practices.yaml:51-88](../platform/components/kyverno-policies/base/tenant-best-practices.yaml#L51-L88) |
| **HPA Recommendations** | Horizontal Pod Autoscaler guidance | **Audit** - Warn only | [tenant-best-practices.yaml:124-161](../platform/components/kyverno-policies/base/tenant-best-practices.yaml#L124-L161) |
| **Deprecated API Warnings** | Alerts on deprecated Kubernetes APIs | **Audit** - Warn only | [tenant-best-practices.yaml:163-195](../platform/components/kyverno-policies/base/tenant-best-practices.yaml#L163-L195) |

## Policy Reports

### Viewing Policy Violations

```bash
# View policy reports for a namespace
kubectl get policyreport -n tenant-acme-prod

# Detailed report
kubectl describe policyreport -n tenant-acme-prod

# View all cluster-wide policy reports
kubectl get clusterpolicyreport
```

### Understanding Enforcement Modes

**Enforce Mode:**
- Policy violations are **blocked** at admission time
- Resources that violate the policy are rejected
- User receives clear error message explaining the violation

**Audit Mode:**
- Policy violations are **logged** but not blocked
- Resources are admitted even if they violate the policy
- Violations appear in PolicyReports and Splunk logs
- Used for non-critical best practices

### Common Policy Violation Examples

**Example 1: Privileged Container Blocked**

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

tenant-security-baseline:
  check-privileged: 'Privileged containers are not allowed. Set securityContext.privileged to false or remove the field.'
```

**Example 2: Latest Tag Blocked (Production)**

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

disallow-latest-tag:
  validate-image-tag: 'Using latest tag is not allowed in production. Use semantic versioning (e.g., v1.2.3)'
```

**Example 3: Missing Resource Limits (Production)**

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

require-resource-limits:
  validate-resources: 'All containers must have CPU and memory requests and limits defined.'
```

## Modifying Kyverno Policies

### For Repository-Managed Policies

1. **Create a branch** from main
2. **Edit policy files** in `platform/components/kyverno-policies/base/` or `platform/components/kyverno-policies/overlays/`
3. **Test changes** in dev environment first
4. **Submit PR** for review by platform security team
5. **Deploy via GitOps** - Flux CD automatically applies approved changes

### Environment-Specific Overrides

To add environment-specific policy configuration:

```bash
# Add prod-specific overlay
vim platform/components/kyverno-policies/overlays/prod/registry-override.yaml

# Add dev-specific overlay
vim platform/components/kyverno-policies/overlays/dev/registry-override.yaml
```

## Monitoring and Alerting

### Kyverno Metrics

Kyverno exports Prometheus metrics for policy enforcement:

```bash
# View Kyverno metrics
kubectl port-forward -n kyverno svc/kyverno-metrics 8000:8000
curl http://localhost:8000/metrics | grep kyverno
```

**Key Metrics:**
- `kyverno_policy_results_total` - Total policy evaluation results (pass/fail)
- `kyverno_policy_execution_duration_seconds` - Policy execution latency
- `kyverno_admission_requests_total` - Total admission webhook requests

### Splunk Integration

All policy violations are sent to Splunk for centralized monitoring:

```
index=k8s_fedcore_all kyverno denied
| stats count by tenant_name, policy_name, namespace
| sort -count
```

## Related Documentation

- [Security Overview](SECURITY_OVERVIEW.md) - High-level security architecture
- [Runtime Security](RUNTIME_SECURITY.md) - Tetragon and network security
- [Security Audit & Alerting](SECURITY_AUDIT_ALERTING.md) - Compliance and monitoring
- [Tenant User Guide](TENANT_USER_GUIDE.md) - Working within policy constraints

---

## Navigation

[← Previous: Security Overview](SECURITY_OVERVIEW.md) | [Next: Runtime Security →](RUNTIME_SECURITY.md)

**Handbook Progress:** Page 22 of 35 | **Level 5:** Security & Compliance

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
