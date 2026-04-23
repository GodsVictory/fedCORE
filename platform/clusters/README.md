# Cluster Configuration Reference

This document describes all available configuration options for cluster definitions.

## Structure

Each cluster directory contains:
- `cluster.yaml` - Main cluster configuration (required)
- `overlays/` - Cluster-specific infrastructure customizations (optional)

### Infrastructure Overlays (Waterfall/Cascade Pattern)

Infrastructure customizations follow a **waterfall pattern** with increasing specificity:

```
1. Base Templates       → All clusters (platform/components/{component}/base/)
2. Component Overlays   → Selected by the cluster's `overlays` array ({component}/overlays/{id}/)
3. Cluster Overlays     → Specific cluster (platform/clusters/{cluster}/overlays/)
```

Which component overlays are applied is controlled by the `overlays` array in `cluster.yaml`. For example, `overlays: ["aws", "prod"]` will apply `{component}/overlays/aws/` then `{component}/overlays/prod/` for each component. Order matters — later entries have **higher precedence**. Cluster overlays always apply last.

#### When to Use Each Layer

**Use environment overlays** for policies that apply to all clusters in an environment:
- Production: Strict resource limits, no :latest tags, require probes
- Development: Relaxed policies, allow experimentation

**Use cloud overlays** for provider-specific configurations:
- AWS: Additional ACK controllers (RDS, DynamoDB)
- Azure: ASO resource configurations
- OnPrem: Custom networking or storage

**Use cluster overlays** only for truly unique, one-off customizations:
- GPU-specific policies for ML clusters
- Special hardware configurations
- Unique integrations

**Example:** All production clusters automatically get strict policies via `platform/components/overlays/prod/`:
- `require-resource-limits.yaml` - Enforces CPU/memory limits
- `disallow-latest-tag.yaml` - Blocks :latest image tags

No need to duplicate these in individual cluster directories!

For details on the overlay system, see [Infrastructure Overlays Documentation](../components/OVERLAY-SYSTEM.md).

## Data Values Reference

### Physical Facts (Required)

```yaml
cluster_name: "fedcore-prod-use1"  # Must match directory name
cloud: "aws"                         # aws, azure, onprem
region: "us-east-1"                  # Cloud region or datacenter
ingress_domain: "prod.us-east-1.fedcore.io"
```

### Cloud-Specific Configuration (Optional)

**AWS:**
```yaml
aws:
  account_id: "123456789012"
```

**Note:** Pod Identity is used for AWS authentication. No OIDC provider configuration needed.

**Azure:**
```yaml
azure:
  subscription_id: "00000000-0000-0000-0000-000000000000"
  resource_group: "fedcore-prod-eastus-rg"
  tenant_id: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
  oidc_issuer: "https://eastus.oic.prod-aks.azure.com/..."
  region: "eastus"
```

**On-Premises:**
```yaml
datacenter: "lab-dc"
```

### Backup Configuration

```yaml
backup:
  retention_days: 7
  schedule: "0 2 * * *"  # Cron format
```

### Monitoring & Observability

```yaml
monitoring:
  enabled: true  # Enables platform component ServiceMonitors
```

### Istio Gateway

```yaml
istio_gateway:
  name: "istio-ingressgateway"
  namespace: "istio-system"
  # See schema.yaml for additional configuration options
```

### Tenant Policies (Multi-Tenancy)

```yaml
tenant_policies:
  # Image registry enforcement
  enforce_image_registry: true
  allowed_registries:
    - "nexus.fedcore.io/tenant-"
    - "nexus.fedcore.io/platform/"
  disallow_latest_tag: true
  require_image_signatures: false
  image_signing_public_key: ""  # Optional Cosign public key

  # Resource management
  require_resource_limits: true
  default_namespace_cpu_quota: "20"
  default_namespace_memory_quota: "40Gi"

  # Network isolation
  allow_internet_egress: true

  # Security baseline
  require_seccomp: true
```

### Tenants (Defined in cluster.yaml)

Tenants are defined in the cluster configuration and automatically deployed as part of bootstrap. This eliminates the manual 2-step onboarding process.

**In cluster.yaml:**

```yaml
#@data/values
---
cluster_name: "onprem-dc1-dev-app"
# ... other config ...

# Tenants to onboard on this cluster
tenants:
  #! Full Capsule Tenant (type: tenant - default)
  - name: test-tenant
    type: "tenant"  #! Optional - defaults to "tenant" if omitted
    owners:
      - kind: User
        name: dev@fedcore.io
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 5
    resources:
      cpu: "20"
      memory: "40Gi"
      storage: "200Gi"
      max_pvcs: 20
    cost_center: "development"
    billing_contact: "dev-team@fedcore.io"
    allow_loadbalancer: false
    default_cpu_limit: "200m"
    default_memory_limit: "256Mi"

  #! Full Capsule Tenant with Istio
  - name: acme
    type: "tenant"
    owners:
      - kind: User
        name: john@acme-corp.com
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 10
    resources:
      cpu: "100"
      memory: "200Gi"
      storage: "1Ti"
      max_pvcs: 50
    cost_center: "engineering"
    billing_contact: "finance@acme-corp.com"
    allow_loadbalancer: true
    #! Optional: Enable Istio service mesh
    istio:
      enabled: true
      strict_mtls: true

  #! Simple Namespace (type: namespace)
  - name: simple-app
    type: "namespace"
    owners:
      - kind: User
        name: dev@example.com
        apiGroup: rbac.authorization.k8s.io
    cost_center: "development"
    description: "Standalone namespace for simple application"
```

**Enable tenant-instances component:**

```yaml
components:
  - name: tenant
  - name: tenant-instances
```

Components are enabled by listing them. `depends_on` is resolved automatically from each component's `overlay.yaml` file (e.g., `tenant-instances/overlay.yaml` sets `depends_on: [namespace]`). You can override `depends_on` in cluster.yaml if needed.

**What gets automatically created (via TenantOnboarding CRs):**
- ✅ Capsule Tenant (namespace isolation + quotas)
- ✅ CI/CD namespace (`<tenant>-cicd`)
- ✅ ServiceAccount with Pod Identity (AWS) or Workload Identity (Azure) annotations
- ✅ Two-tier IAM Roles (AWS cluster + tenant) or Managed Identity (Azure)
- ✅ RBAC for automated deployments
- ✅ Pod Identity Association (AWS only)
- ✅ Istio mTLS and authorization policies (if enabled)

**Benefits:**
- ✅ Single-step deployment - No manual post-bootstrap step
- ✅ Schema validation - ytt validates tenant configs at build time
- ✅ GitOps-friendly - Tenant changes = commit to cluster.yaml
- ✅ Proper ordering - Flux dependency ensures RGD deploys before instances

**Legacy approach (still supported):**

You can still define tenants as standalone TenantOnboarding CRs and apply them manually:

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme
  # ... same spec as above ...
```

Then manually apply after bootstrap: `kubectl apply -f <file>`

See [Tenant Onboarding Documentation](../rgds/tenant/README.md) and [Tenant Instances Component](../components/tenant-instances/README.md) for complete guides.

## Complete Example

Example cluster directory names by environment:
- `aws-prod-use1-app` - AWS production cluster (us-east-1)
- `azure-prod-eus-app` - Azure production cluster (eastus)
- `onprem-dc1-dev-app` - On-premises development cluster

## Validation

Test your cluster configuration locally:

```bash
# Validate YAML syntax
fedcore validate

# Build a specific component artifact
fedcore build --artifact platform/components/kro --cluster platform/clusters/fedcore-prod-use1 > /tmp/test.yaml

# Generate bootstrap configuration
fedcore bootstrap --cluster platform/clusters/fedcore-prod-use1 > /tmp/test.yaml

# Generate admin-prep manifest (CRDs, namespace, RBAC) for non-admin clusters
fedcore bootstrap --cluster platform/clusters/fedcore-prod-use1 --admin-prep -r registry.example.com

# Inspect merged data values
ytt -f platform/clusters/fedcore-prod-use1/ --data-values-inspect
```

**Build process with overlay cascade:**

The build script applies overlays based on the cluster's `overlays` array:

```yaml
# In cluster.yaml:
overlays:
  - aws
  - prod
```

```bash
# Build a component — overlays are applied in order:
fedcore build --artifact platform/components/kro --cluster platform/clusters/fedcore-prod-use1

# Effective ytt load order:
#   1. schema.yaml + cluster.yaml        (base data values)
#   2. platform/components/kro/base/      (base manifests)
#   3. platform/components/kro/overlays/aws/   (from overlays[0])
#   4. platform/components/kro/overlays/prod/  (from overlays[1])
#   5. platform/clusters/fedcore-prod-use1/overlays/  (cluster overlays, if present)
```

Each overlay layer can override previous layers, with cluster overlays having the highest precedence.

**Result:** Setting `overlays: ["aws", "prod"]` on any cluster automatically applies both cloud and environment overlays without duplicating them in each cluster directory.

## Adding New Fields

If you need to add new configuration fields:

1. Update [schema.yaml](schema.yaml) with the new field definition
2. Add the field to your cluster.yaml or tenant file
3. Use them in your templates with `#@ data.values.your_field`
4. Document them in this README
5. Consider if they should be cluster-level or tenant-level

ytt will automatically merge and validate all data values from:
1. Schema validation (platform/clusters/schema.yaml)
2. Cluster configuration (cluster.yaml)
3. Component overlays (each component's `overlay.yaml`, if present)

The schema provides validation and type checking for all configuration fields.
