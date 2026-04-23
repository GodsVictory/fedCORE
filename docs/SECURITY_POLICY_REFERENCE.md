# Security Policy Reference

## Overview

This document provides a quick reference table for all security policies in the fedCORE platform. Use this for fast lookups when you need to understand policy enforcement, exemptions, or troubleshooting.

For detailed policy documentation, see:
- [Kyverno Policies](KYVERNO_POLICIES.md) - Admission control details
- [Runtime Security](RUNTIME_SECURITY.md) - Runtime monitoring
- [Security Overview](SECURITY_OVERVIEW.md) - Architecture overview

## Quick Lookup Table

### Container Security Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Privileged Container Block | Container Security | Enforce | All | Prevents privileged mode and privilege escalation via securityContext | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Non-Root Enforcement | Container Security | Enforce | All | All containers must run as non-root users (runAsNonRoot: true) | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Capabilities Restriction | Container Security | Enforce | All | Drops ALL capabilities, only allows safe ones (NET_BIND_SERVICE, CHOWN, DAC_OVERRIDE, SETGID, SETUID) | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Host Namespace Isolation | Container Security | Enforce | All | Prevents sharing host network, PID, or IPC namespaces | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Host Port Blocking | Container Security | Enforce | All | Prevents containers from binding to host ports | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Seccomp Profile Required | Container Security | Enforce | All | Enforces RuntimeDefault or Localhost seccomp profiles | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Volume Type Restrictions | Container Security | Enforce | All | Blocks hostPath and other dangerous volume types | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |
| Sysctls Restriction | Container Security | Enforce | All | Only allows safe kernel parameters, blocks unsafe sysctls | [KYVERNO_POLICIES.md#container-security-policies](KYVERNO_POLICIES.md#container-security-policies) |

### Network Security Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Default Deny All Ingress | Network Security | Enforce | All | NetworkPolicy blocks all incoming traffic by default | [RUNTIME_SECURITY.md#network-security](RUNTIME_SECURITY.md#network-security) |
| Same-Tenant Communication | Network Security | Enforce | All | NetworkPolicy allows pod-to-pod communication within same tenant | [RUNTIME_SECURITY.md#network-security](RUNTIME_SECURITY.md#network-security) |
| DNS Access Control | Network Security | Enforce | All | Permits egress to CoreDNS for name resolution | [RUNTIME_SECURITY.md#network-security](RUNTIME_SECURITY.md#network-security) |
| Internet Egress Control | Network Security | Enforce | All | Configurable external access based on tenant requirements | [RUNTIME_SECURITY.md#network-security](RUNTIME_SECURITY.md#network-security) |
| Cross-Tenant Prevention | Network Security | Enforce | All | Validates tenants cannot create NetworkPolicies that bypass isolation | [KYVERNO_POLICIES.md#network-security-policies](KYVERNO_POLICIES.md#network-security-policies) |

### Istio Service Mesh Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Strict mTLS Enforcement | Network Security | Enforce | Prod/Staging | Tenant PeerAuthentication policies must use STRICT mode (no plaintext allowed) | [KYVERNO_POLICIES.md#istio-service-mesh-policies](KYVERNO_POLICIES.md#istio-service-mesh-policies) |
| Tenant AuthorizationPolicy Validation | Network Security | Enforce | All (when Istio enabled) | Prevents tenants from creating policies that allow cross-tenant traffic | [KYVERNO_POLICIES.md#istio-service-mesh-policies](KYVERNO_POLICIES.md#istio-service-mesh-policies) |
| Istio System Protection | Network Security | Enforce | All (when Istio enabled) | Blocks tenant modifications to istio-system namespace | [KYVERNO_POLICIES.md#istio-service-mesh-policies](KYVERNO_POLICIES.md#istio-service-mesh-policies) |
| DestinationRule TLS Enforcement | Network Security | Enforce | All (when Istio enabled) | Prevents tenants from disabling TLS via DestinationRule | [KYVERNO_POLICIES.md#istio-service-mesh-policies](KYVERNO_POLICIES.md#istio-service-mesh-policies) |

### Supply Chain Security

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Registry Restriction | Supply Chain | Enforce (Prod) / Audit (Dev/Staging) | Varies | Only approved container registries allowed (configurable whitelist) | [KYVERNO_POLICIES.md#supply-chain-security-policies](KYVERNO_POLICIES.md#supply-chain-security-policies) |
| Latest Tag Prohibition | Supply Chain | Enforce (Prod) / Audit (Dev/Staging) | Varies | Enforces semantic versioning, blocks "latest" tags | [KYVERNO_POLICIES.md#supply-chain-security-policies](KYVERNO_POLICIES.md#supply-chain-security-policies) |
| Image Signature Verification | Supply Chain | Optional | All | Optional Cosign/Sigstore integration for signed images | [KYVERNO_POLICIES.md#supply-chain-security-policies](KYVERNO_POLICIES.md#supply-chain-security-policies) |

### Resource Management Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Mandatory Resource Limits | Resource Management | Enforce (Prod) / Audit (Dev/Staging) | Varies | CPU and memory requests/limits required on all containers | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| ResourceQuota Validation | Resource Management | Enforce | All | Blocks pod creation if namespace missing ResourceQuota | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| LimitRange Validation | Resource Management | Enforce | All | Blocks pod creation if namespace missing LimitRange | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| Expensive Resource Approval | Resource Management | Enforce | All | LoadBalancers and large PVCs require approval annotation | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| Cost Tracking Labels | Resource Management | Enforce (Mutates) | All | Automatic tagging for chargeback and cost allocation | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| Per-Namespace ResourceQuota | Resource Management | Optional (Generates) | All | Optional per-namespace quotas within tenant | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |
| Namespace Quota Readiness Audit | Resource Management | Audit | All | Reports namespaces missing ResourceQuota or LimitRange | [KYVERNO_POLICIES.md#resource-management-policies](KYVERNO_POLICIES.md#resource-management-policies) |

### Input Validation Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Name Format Validation | Input Validation | Enforce | All | DNS-compliant tenant naming (1-30 characters) | [KYVERNO_POLICIES.md#input-validation-policies](KYVERNO_POLICIES.md#input-validation-policies) |
| Quota Format Validation | Input Validation | Enforce | All | Proper Kubernetes resource format validation | [KYVERNO_POLICIES.md#input-validation-policies](KYVERNO_POLICIES.md#input-validation-policies) |
| Billing Information Validation | Input Validation | Enforce | All | Cost center and contact information required | [KYVERNO_POLICIES.md#input-validation-policies](KYVERNO_POLICIES.md#input-validation-policies) |
| Owner Requirements | Input Validation | Enforce | All | At least one owner required per tenant | [KYVERNO_POLICIES.md#input-validation-policies](KYVERNO_POLICIES.md#input-validation-policies) |

### Best Practices (Audit Mode)

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Readiness/Liveness Probes | Best Practices | Audit | All | Recommendations for health checks | [KYVERNO_POLICIES.md#best-practices-policies-audit-mode](KYVERNO_POLICIES.md#best-practices-policies-audit-mode) |
| Pod Disruption Budget | Best Practices | Audit | All | Suggestions for high availability | [KYVERNO_POLICIES.md#best-practices-policies-audit-mode](KYVERNO_POLICIES.md#best-practices-policies-audit-mode) |
| HPA Recommendations | Best Practices | Audit | All | Horizontal Pod Autoscaler guidance | [KYVERNO_POLICIES.md#best-practices-policies-audit-mode](KYVERNO_POLICIES.md#best-practices-policies-audit-mode) |
| Deprecated API Warnings | Best Practices | Audit | All | Alerts on deprecated Kubernetes APIs | [KYVERNO_POLICIES.md#best-practices-policies-audit-mode](KYVERNO_POLICIES.md#best-practices-policies-audit-mode) |

### Runtime Security Policies

| Policy Name | Category | Enforcement Level | Environment | What It Does | Link to Full Policy |
|-------------|----------|-------------------|-------------|--------------|---------------------|
| Tenant Boundary Violation Detection | Runtime Security | Audit + Alert (HIGH) | All | Monitors unauthorized ServiceAccount token access and cross-tenant namespace attempts | [RUNTIME_SECURITY.md#tetragon-security-policies](RUNTIME_SECURITY.md#tetragon-security-policies) |
| Privilege Escalation Detection | Runtime Security | Audit + Alert (HIGH) | All | Tracks capability changes (CAP_SYS_ADMIN, CAP_SYS_MODULE, etc.) | [RUNTIME_SECURITY.md#tetragon-security-policies](RUNTIME_SECURITY.md#tetragon-security-policies) |
| Suspicious Process Execution | Runtime Security | Audit + Alert (MEDIUM) | All | Detects shells and network tools (nc, wget, curl, ssh) in tenant namespaces | [RUNTIME_SECURITY.md#tetragon-security-policies](RUNTIME_SECURITY.md#tetragon-security-policies) |
| Cryptocurrency Mining Prevention | Runtime Security | Enforce (SIGKILL) + Alert (CRITICAL) | All | Detects mining binaries and automatically kills processes | [RUNTIME_SECURITY.md#tetragon-security-policies](RUNTIME_SECURITY.md#tetragon-security-policies) |
| Container Escape Detection | Runtime Security | Audit + Alert (CRITICAL) | All | Monitors kernel file access attempts (/proc/sys/kernel/, /sys/kernel/, /dev/kmem, /dev/mem) | [RUNTIME_SECURITY.md#tetragon-security-policies](RUNTIME_SECURITY.md#tetragon-security-policies) |

## Environment-Specific Enforcement

Security policies are configured differently across environments to balance security with developer velocity:

| Environment | Latest Tag | Resource Limits | Image Registry | Policy Enforcement |
|-------------|------------|-----------------|----------------|-------------------|
| **Development** | Allowed (Audit mode) | Recommended (Audit mode) | Recommended (Audit mode) | Relaxed - policies log warnings but don't block |
| **Staging** | Discouraged (Audit mode) | Recommended (Audit mode) | Recommended (Audit mode) | Moderate - most policies audit, critical ones enforce |
| **Production** | Blocked (Enforce mode) | Required (Enforce mode) | Required (Enforce mode) | Strict - all policies enforce |

See [Security Overview - Environment-Specific Configuration](SECURITY_OVERVIEW.md#environment-specific-configuration) for details.

## Requesting Policy Exemptions

### When Exemptions Are Allowed

Policy exemptions are **rarely granted** and only for exceptional circumstances:

- **Approved Use Cases:**
  - Legacy applications with documented migration plan
  - Temporary exemptions for testing with expiration date
  - Vendor software with documented security justification
  - Platform infrastructure components (admin approval required)

- **Never Approved:**
  - Privileged containers in tenant namespaces
  - Cross-tenant access bypasses
  - Production use of "latest" image tags
  - Disabling mTLS in production Istio mesh

### How to Request an Exemption

1. **Contact Platform Team:**
   - Open a GitHub issue in the platform repository with the label "policy-exemption"

2. **Information Required:**
   - Tenant name and namespace
   - Policy name and specific rule being violated
   - Business justification for exemption
   - Security risk assessment
   - Compensating controls if any
   - Duration of exemption (temporary vs permanent)
   - Migration plan (if temporary)

3. **Approval Process:**
   - Platform Security Team reviews request
   - Security approval required for production exemptions
   - Exemption documented in compliance records
   - Regular review of active exemptions

4. **Implementation:**
   - Exemptions implemented via policy annotations
   - Audit trail maintained in Splunk
   - Quarterly review of exemption status

**Example Exemption Annotation:**
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: legacy-app
  namespace: tenant-acme-prod
  annotations:
    # Exemption approved by Security Team - Ticket SEC-12345
    policies.kyverno.io/exclude: "require-resource-limits,disallow-latest-tag"
spec:
  containers:
    - name: app
      image: legacy-app:latest  # Exempted - migration planned for Q3 2026
```

## Quick Commands

### Check Policy Status

```bash
# List all Kyverno cluster policies
kubectl get clusterpolicy

# View policy details
kubectl describe clusterpolicy <policy-name>

# Check policy status and violations
kubectl get clusterpolicy -o wide
```

### View Policy Violations

```bash
# View policy reports for all namespaces
kubectl get policyreport -A

# View detailed report for specific namespace
kubectl describe policyreport -n tenant-acme-prod

# View cluster-wide policy reports
kubectl get clusterpolicyreport

# Show failed policy results only
kubectl get policyreport -A -o json | jq '.items[] | select(.summary.fail > 0)'
```

### Check Tenant Security Configuration

```bash
# View tenant configuration
kubectl get tenant acme -o yaml

# Check NetworkPolicies for tenant namespace
kubectl get networkpolicies -n tenant-acme-prod

# View ResourceQuota and LimitRange
kubectl get resourcequota,limitrange -n tenant-acme-prod

# Check if Istio is enabled
kubectl get namespace tenant-acme-prod -o jsonpath='{.metadata.labels.istio-injection}'
```

### Monitor Runtime Security

```bash
# Check Tetragon DaemonSet status
kubectl get daemonset -n kube-system tetragon

# View Tetragon TracingPolicies
kubectl get tracingpolicy -n kube-system

# View recent Tetragon security events
kubectl logs -n kube-system daemonset/tetragon --tail=50

# View Tetragon metrics
kubectl port-forward -n kube-system svc/tetragon-metrics 9090:9090
curl http://localhost:9090/metrics | grep tetragon
```

### Troubleshoot Policy Issues

```bash
# Check why a resource was denied
kubectl apply -f deployment.yaml
# Read error message carefully - includes policy name and violation

# View admission webhook configuration
kubectl get validatingwebhookconfigurations | grep kyverno
kubectl get mutatingwebhookconfigurations | grep kyverno

# Check Kyverno logs for issues
kubectl logs -n kyverno -l app.kubernetes.io/name=kyverno

# Verify policy is active and not in audit mode
kubectl get clusterpolicy <policy-name> -o jsonpath='{.spec.validationFailureAction}'
# Should return: Enforce (blocking) or Audit (logging only)
```

### Splunk Queries for Policy Analysis

```bash
# View all policy violations for a tenant
index=k8s_fedcore_all kyverno denied tenant_name="acme"
| stats count by policy_name, namespace
| sort -count

# Find most violated policies
index=k8s_fedcore_all kyverno denied
| stats count by policy_name
| sort -count

# Track policy violations over time
index=k8s_fedcore_all kyverno denied
| timechart span=1h count by policy_name

# Security events by severity
index=k8s_fedcore_security
| stats count by severity, policy_name
| sort -severity
```

## Understanding Enforcement Modes

### Enforce Mode
- Policy violations are **blocked** at admission time
- Resources that violate the policy are rejected
- User receives clear error message explaining the violation
- Used for critical security policies

**Example Error:**
```
Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

tenant-security-baseline:
  check-privileged: 'Privileged containers are not allowed. Set securityContext.privileged to false or remove the field.'
```

### Audit Mode
- Policy violations are **logged** but not blocked
- Resources are admitted even if they violate the policy
- Violations appear in PolicyReports and Splunk logs
- Used for non-critical best practices and new policy rollouts

**Example Audit Entry:**
```json
{
  "policy": "readiness-probe-recommended",
  "rule": "check-readiness-probe",
  "resource": "Deployment/tenant-acme-prod/webapp",
  "result": "fail",
  "message": "Readiness probe is recommended for production workloads",
  "severity": "medium"
}
```

## Common Policy Violation Examples

### Privileged Container Blocked

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

tenant-security-baseline:
  check-privileged: 'Privileged containers are not allowed. Set securityContext.privileged to false or remove the field.'
```

**Fix:** Remove `privileged: true` from securityContext or set it to `false`.

### Latest Tag Blocked (Production)

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

disallow-latest-tag:
  validate-image-tag: 'Using latest tag is not allowed in production. Use semantic versioning (e.g., v1.2.3)'
```

**Fix:** Use a specific version tag like `v1.2.3` instead of `latest`.

### Missing Resource Limits (Production)

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

require-resource-limits:
  validate-resources: 'All containers must have CPU and memory requests and limits defined.'
```

**Fix:** Add resource requests and limits to all containers:
```yaml
resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 512Mi
```

### Unallowed Container Registry

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/tenant-acme-prod/webapp for resource violation:

tenant-image-registry:
  validate-registry: 'Image must be from approved registry: nexus.fedcore.io'
```

**Fix:** Push image to approved registry or request registry to be added to allowlist.

## Related Documentation

- [Kyverno Policies](KYVERNO_POLICIES.md) - Complete policy documentation
- [Runtime Security](RUNTIME_SECURITY.md) - Tetragon and network security
- [Security Overview](SECURITY_OVERVIEW.md) - Security architecture
- [Security Audit & Alerting](SECURITY_AUDIT_ALERTING.md) - Monitoring and compliance
- [Tenant User Guide](TENANT_USER_GUIDE.md) - Deploying secure workloads
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues and solutions

---

## Navigation

[← Previous: Security Audit & Alerting](SECURITY_AUDIT_ALERTING.md) | [Next: Runtime Security & Logging →](RUNTIME_SECURITY_AND_LOGGING.md)

**Handbook Progress:** Page 25 of 35 | **Level 5:** Security & Compliance

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
