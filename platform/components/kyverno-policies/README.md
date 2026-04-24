# Kyverno Policies

Policy engine for tenant security, validation, and best practices enforcement.

## Overview

This component contains Kyverno ClusterPolicy resources that enforce security, compliance, and operational standards across the platform. Policies are organized into base policies (apply to all clusters) and overlays for cloud-specific or environment-specific rules.

**Note:** This component contains only the policy definitions. The Kyverno operator itself is deployed separately via the [kyverno component](../kyverno/README.md).

## What These Policies Do

Kyverno policies provide three types of enforcement:

1. **Validation**: Reject resources that don't meet requirements
2. **Mutation**: Automatically modify resources to add missing fields
3. **Generation**: Automatically create related resources

These policies focus on tenant security and operational best practices.

## Policy Organization

```
kyverno-policies/
├── base/                                          # Policies for all clusters
│   ├── tenant-onboarding-validation.yaml          # Validate tenant onboarding inputs
│   ├── tenant-security-baseline.yaml              # Pod security standards
│   ├── tenant-resource-limits.yaml                # Require resource limits
│   ├── tenant-image-registry.yaml                 # Restrict image sources
│   ├── tenant-network-policies.yaml               # Network isolation
│   ├── istio-tenant-policies.yaml                 # Istio service mesh security
│   └── tenant-best-practices.yaml                 # General best practices
└── overlays/
    ├── aws/
    │   └── ack-cross-account.yaml                 # Inject ACK cross-account annotations
    └── prod/
        └── kyverno-policies/
                ├── disallow-latest-tag.yaml       # Require specific image versions
                └── require-resource-limits.yaml   # Stricter resource enforcement
```

## Base Policies

### 1. Tenant Onboarding Validation

**File:** [base/tenant-onboarding-validation.yaml](base/tenant-onboarding-validation.yaml:1)

Validates `TenantOnboarding` custom resources to ensure:
- Tenant names are DNS-compliant (lowercase alphanumeric with hyphens, 1-30 chars)
- Resource quotas are within acceptable ranges
- Billing information is properly formatted (cost center, contact email)
- At least one owner is specified
- Default resource limits are in correct format

**Example violations:**
```yaml
# Invalid: Tenant name too long
spec:
  tenantName: this-tenant-name-is-way-too-long-for-iam

# Invalid: Quota out of range
spec:
  quotas:
    namespaces: 200  # Max is 100

# Invalid: Missing owner
spec:
  owners: []  # At least one required
```

### 2. Security Baseline

**File:** [base/tenant-security-baseline.yaml](base/tenant-security-baseline.yaml:1)

Enforces Pod Security Standards for tenant workloads:
- Disallow privileged containers
- Require running as non-root
- Restrict host path mounts
- Limit capabilities
- Enforce seccomp and AppArmor profiles

**Why this matters:** Prevents tenant containers from breaking out of their sandbox or accessing host resources.

### 3. Resource Limits

**File:** [base/tenant-resource-limits.yaml](base/tenant-resource-limits.yaml:1)

Requires all tenant containers to specify:
- CPU limits and requests
- Memory limits and requests

**Why this matters:** Prevents resource exhaustion and ensures fair resource sharing.

**Example:**
```yaml
# Valid: Resource limits specified
containers:
  - name: app
    resources:
      limits:
        cpu: "1"
        memory: 512Mi
      requests:
        cpu: 500m
        memory: 256Mi

# Invalid: Missing limits
containers:
  - name: app
    resources: {}  # Policy will reject this
```

### 4. Image Registry Restrictions

**File:** [base/tenant-image-registry.yaml](base/tenant-image-registry.yaml:1)

Restricts container images to approved registries:
- Internal registry (e.g., `registry.fedcore.io`)
- Approved public registries (e.g., `ghcr.io`, `docker.io`)

**Why this matters:** Prevents pulling images from untrusted sources that could contain malware.

### 5. Network Policies

**File:** [base/tenant-network-policies.yaml](base/tenant-network-policies.yaml:1)

Automatically generates NetworkPolicy resources for tenant namespaces to:
- Deny all ingress by default
- Allow egress to DNS and API server
- Require explicit allow rules for application traffic

**Why this matters:** Provides network isolation between tenants and prevents lateral movement.

### 6. Istio Service Mesh Policies

**File:** [base/istio-tenant-policies.yaml](base/istio-tenant-policies.yaml:1)

Enforces Istio service mesh security and multi-tenant isolation:

**Policies Included:**
1. **Require Strict mTLS**: Forces STRICT mTLS mode for all tenant PeerAuthentication policies
2. **Validate Authorization Policies**: Prevents cross-tenant AuthorizationPolicy rules
3. **Prevent Istio System Modifications**: Blocks tenant access to istio-system namespace
4. **Validate Sidecar Resources**: Ensures Envoy sidecar resource limits are reasonable
5. **Auto-Label Pods**: Automatically adds tenant labels to Istio-injected pods for observability
6. **Require Injection Label**: Encourages explicit opt-in/opt-out via istio-injection label
7. **Validate DestinationRule TLS**: Prevents disabling TLS via DestinationRule
8. **Validate ServiceEntry**: Monitors external service definitions for security compliance

**Why this matters:** Enforces service-to-service mTLS for compliance requirements and prevents tenants from bypassing mesh-wide security policies.

**Example violations:**
```yaml
# Invalid: PERMISSIVE mTLS not allowed in tenant namespaces
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: acme-frontend
spec:
  mtls:
    mode: PERMISSIVE  # Policy will reject - must be STRICT

# Invalid: AuthorizationPolicy allowing cross-tenant traffic
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: allow-all-tenants
  namespace: acme-frontend
spec:
  action: ALLOW
  rules:
  - from:
    - source:
        namespaces: ["*"]  # Policy will reject - must restrict to same tenant
```

**Integration:** Works alongside NetworkPolicies (Layer 3/4) to provide defense-in-depth with Layer 7 identity-based authorization.

See [Istio Component Documentation](../istio/README.md) for complete service mesh architecture details.

### 7. Best Practices

**File:** [base/tenant-best-practices.yaml](base/tenant-best-practices.yaml:1)

Enforces operational best practices:
- Require labels (app, component, version)
- Disallow default service accounts
- Require health probes (liveness, readiness)
- Validate resource naming conventions

## Cloud-Specific Policies (AWS)

### ACK Cross-Account Annotations

**File:** [overlays/aws/ack-cross-account.yaml](overlays/aws/ack-cross-account.yaml:1)

Automatically injects cross-account annotations on ACK resources:
- Looks up tenant account ID from TenantOnboarding resource
- Adds `services.k8s.aws/account-id` annotation
- Adds `services.k8s.aws/role-arn` annotation pointing to provisioner role

**Why this matters:** Enables ACK to provision resources in tenant AWS accounts without manual annotation.

**Example mutation:**
```yaml
# Before mutation
apiVersion: s3.services.k8s.aws/v1alpha1
kind: Bucket
metadata:
  name: my-bucket
  namespace: acme-app

# After mutation (automatic)
apiVersion: s3.services.k8s.aws/v1alpha1
kind: Bucket
metadata:
  name: my-bucket
  namespace: acme-app
  annotations:
    services.k8s.aws/account-id: "987654321098"
    services.k8s.aws/role-arn: "arn:aws:iam::987654321098:role/fedcore-ack-provisioner"
```

## Environment-Specific Policies (Production)

### Disallow Latest Tag

**File:** [overlays/prod/kyverno-policies/disallow-latest-tag.yaml](overlays/prod/kyverno-policies/disallow-latest-tag.yaml:1)

Rejects container images using the `:latest` tag in production:
- Requires specific version tags (e.g., `v1.2.3`, `sha256:abc123`)
- Prevents accidental deployment of untested images

**Why production only:** Development environments benefit from rapid iteration with latest tags.

### Strict Resource Limits

**File:** [overlays/prod/kyverno-policies/require-resource-limits.yaml](overlays/prod/kyverno-policies/require-resource-limits.yaml:1)

Enforces stricter resource limit requirements in production:
- Requires both limits AND requests (not just limits)
- Enforces minimum and maximum ratios
- Ensures requests are reasonable percentages of limits

**Why production only:** Production demands predictable resource consumption for capacity planning.

## Deployment

This component is deployed automatically as part of the infrastructure stack.

**Build:**
```bash
# Build this component for a specific cluster
# Cloud and environment overlays are selected based on cluster.yaml
fedcore build platform/components/kyverno-policies platform/clusters/fedcore-prod-use1 > kyverno-policies-fedcore-prod-use1.yaml
```

**Prerequisites:**
- Kyverno operator must be installed first (see [kyverno component](../kyverno/README.md))
- This component automatically depends on the kyverno component

## Policy Modes

Kyverno policies can run in two modes:

### Audit Mode
```yaml
spec:
  validationFailureAction: Audit
```
- Logs policy violations
- Does not block resources
- Useful for testing new policies

### Enforce Mode
```yaml
spec:
  validationFailureAction: Enforce
```
- Actively blocks non-compliant resources
- Required for production security
- All policies in this component use Enforce mode

## Monitoring Policy Violations

Kyverno tracks policy violations and reports them:

### View Policy Reports

```bash
# Cluster-wide violations
kubectl get clusterpolicyreport -A

# Namespace-specific violations
kubectl get policyreport -n acme-app
```

### Example Report

```yaml
apiVersion: wgpolicyk8s.io/v1alpha2
kind: PolicyReport
metadata:
  name: acme-app
  namespace: acme-app
results:
  - policy: require-resource-limits
    rule: validate-cpu-limits
    result: fail
    resources:
      - apiVersion: v1
        kind: Pod
        name: webapp-xyz
    message: "CPU limits not specified"
```

### Prometheus Metrics

Kyverno exposes metrics for monitoring:

- `kyverno_policy_results_total`: Total policy evaluations by result (pass/fail)
- `kyverno_policy_execution_duration_seconds`: Policy evaluation latency
- `kyverno_admission_requests_total`: Admission webhook requests

## Troubleshooting

### Policy Blocking Valid Resources

**Symptom:** `kubectl apply` fails with Kyverno policy violation

**Solution:**
1. Review the error message:
   ```bash
   kubectl apply -f my-resource.yaml
   # Error from server: admission webhook "validate.kyverno.svc" denied the request:
   # Resource validation failed. Tenant name must be DNS-compliant...
   ```

2. Check policy details:
   ```bash
   kubectl describe clusterpolicy validate-tenant-name
   ```

3. Options:
   - Fix the resource to comply with policy
   - If the policy is incorrect, update it and redeploy
   - Temporarily switch policy to Audit mode for testing

### Cross-Account Annotations Not Injected

**Symptom:** ACK resources fail to create in tenant accounts

**Check:**
1. Verify TenantOnboarding resource exists:
   ```bash
   kubectl get tenantonboarding -A
   ```

2. Check if policy is active:
   ```bash
   kubectl get clusterpolicy inject-ack-cross-account
   ```

3. Review resource annotations:
   ```bash
   kubectl get bucket my-bucket -o yaml | grep services.k8s.aws
   ```

4. Check Kyverno logs:
   ```bash
   kubectl logs -n kyverno -l app.kubernetes.io/name=kyverno
   ```

### Policy Reports Not Generated

**Symptom:** `kubectl get policyreport` returns no results

**Cause:** Background scanning may be disabled or running slowly

**Solution:**
1. Check Kyverno background controller:
   ```bash
   kubectl get pods -n kyverno -l app.kubernetes.io/component=background-controller
   ```

2. Verify background scanning is enabled in Kyverno config

3. Manually trigger policy evaluation:
   ```bash
   kubectl annotate clusterpolicy <policy-name> force-resync=true
   ```

## Customization

### Adding New Policies

To add a new base policy (all clusters):

1. Create policy file in `base/`:
```yaml
# base/require-pod-anti-affinity.yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-pod-anti-affinity
spec:
  validationFailureAction: Enforce
  rules:
    - name: check-pod-anti-affinity
      match:
        any:
          - resources:
              kinds: [Deployment]
      validate:
        message: "Deployments must specify pod anti-affinity"
        pattern:
          spec:
            template:
              spec:
                affinity:
                  podAntiAffinity: "?*"
```

2. Rebuild the artifact:
```bash
fedcore build platform/components/kyverno-policies platform/clusters/fedcore-prod-use1 > kyverno-policies-fedcore-prod-use1.yaml
```

### Adding Cloud-Specific Policies

Create policy in `overlays/{cloud}/`:

```yaml
# overlays/azure/aso-cross-account.yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: inject-aso-resource-group
spec:
  rules:
    - name: inject-resource-group
      match:
        any:
          - resources:
              kinds: [StorageAccount, KeyVault]
      mutate:
        patchStrategicMerge:
          metadata:
            annotations:
              serviceoperator.azure.com/resource-group: "tenant-{{ request.namespace }}"
```

### Adding Environment-Specific Policies

Create policy in `overlays/{env}/kyverno-policies/`:

```yaml
# overlays/prod/kyverno-policies/require-pod-disruption-budget.yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-pdb
spec:
  validationFailureAction: Enforce
  rules:
    - name: check-pdb-exists
      match:
        any:
          - resources:
              kinds: [Deployment]
              selector:
                matchLabels:
                  environment: production
      validate:
        message: "Production deployments must have a PodDisruptionBudget"
        # Implementation details...
```

## Policy Testing

Before deploying new policies to production:

1. **Test in Audit mode:**
```yaml
spec:
  validationFailureAction: Audit  # Start with Audit
```

2. **Review policy reports:**
```bash
kubectl get clusterpolicyreport -A
```

3. **Analyze impact:**
- How many resources would be blocked?
- Are there legitimate use cases being denied?

4. **Switch to Enforce mode:**
```yaml
spec:
  validationFailureAction: Enforce  # After validation
```

## Security Considerations

### Policy Bypass Prevention

Kyverno admission webhooks must be properly configured:
- Fail-closed mode: Block resources if webhook is unavailable
- RBAC: Restrict who can modify ClusterPolicy resources
- Audit logs: Monitor policy changes

### Exception Handling

For legitimate exceptions, use Kyverno policy exceptions:

```yaml
apiVersion: kyverno.io/v2beta1
kind: PolicyException
metadata:
  name: allow-privileged-monitoring
spec:
  exceptions:
    - policyName: disallow-privileged-containers
      ruleNames:
        - validate-privileged
  match:
    any:
      - resources:
          kinds: [Pod]
          namespaces: [monitoring]
          names: [node-exporter-*]
```

## Related Documentation

- [Kyverno Official Documentation](https://kyverno.io)
- [Tenant Onboarding RGD](../../rgds/tenant/README.md)
- [Cloud Permissions Component](../cloud-permissions/README.md)
- [Pod Security Standards](https://kubernetes.io/docs/concepts/security/pod-security-standards/)

---

**Status:** ✅ Production ready
