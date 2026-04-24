# Tenant Instances Component

Generates TenantOnboarding or NamespaceProvisioning CR instances from tenant configurations defined in cluster data values.

## Overview

This component eliminates the 2-step tenant onboarding process by automatically generating tenant/namespace CRs. Tenants are defined in the cluster configuration, validated by ytt schema, and deployed as an OCI artifact with proper Flux dependencies.

Supports two provisioning types:
- **`type: "tenant"`** - Full Capsule tenant with multi-namespace support, quotas, and isolation (TenantOnboarding CR)
- **`type: "namespace"`** - Simple namespace with ServiceAccount and RBAC (NamespaceProvisioning CR)

**Old way (2 steps):**
1. Bootstrap: Deploy tenant RGD component
2. Manual: Apply TenantOnboarding CRs from `config/` directory

**New way (1 step):**
1. Bootstrap: Deploy tenant RGD + tenant instances (with dependency)
   - Flux ensures RGD is deployed before instances
   - No manual step required

---

## How It Works

### Build Time

```bash
# Build all artifacts (includes tenant-instances per cluster)
fedcore build --all

# Or build for specific cluster
fedcore build -a platform/components/tenant-instances \
  -c platform/clusters/onprem-dc1-dev-app \
  > dist/tenant-instances-onprem-dc1-dev-app.yaml
```

The build process:
1. Reads `cluster.yaml` from cluster directory
2. Extracts `tenants` array from data values
3. Processes `tenant-instances.yaml` template with ytt
4. Generates TenantOnboarding CRs for each tenant
5. Packages as OCI artifact: `oci://ghcr.io/fedcore/tenant-instances-{cluster}:version`

### Deploy Time

```bash
# Bootstrap creates Flux resources
fedcore bootstrap -c platform/clusters/onprem-dc1-dev-app --deploy
```

Bootstrap generates:

```yaml
# 1. OCIRepository for tenant-instances
apiVersion: source.toolkit.fluxcd.io/v1
kind: OCIRepository
metadata:
  name: tenant-instances
  namespace: flux-system
spec:
  url: oci://ghcr.io/fedcore/tenant-instances-onprem-dc1-dev-app
  ref:
    semver: 1.0.0

# 2. Kustomization with dependency on tenant RGD
apiVersion: kustomize.toolkit.fluxcd.io/v2
kind: Kustomization
metadata:
  name: tenant-instances
  namespace: flux-system
spec:
  sourceRef:
    kind: OCIRepository
    name: tenant-instances
  dependsOn:
    - name: tenant  # Wait for RGD to deploy TenantOnboarding CRD
      namespace: flux-system
  wait: true
```

### Flux Reconciliation

1. Flux deploys `tenant` Kustomization (RGD component)
   - KRO registers TenantOnboarding CRD
2. Flux waits for `tenant` to become Ready
3. Flux deploys `tenant-instances` Kustomization
   - Applies TenantOnboarding CRs from OCI artifact
   - KRO reconciles and creates Capsule Tenant + IAM resources

---

## Configuration

### 1. Define Tenants in cluster.yaml

```yaml
#@data/values
---
cluster_name: "onprem-dc1-dev-app"
# ... other cluster config ...

# Tenants to onboard on this cluster
tenants:
  #! Full Capsule Tenant (type: tenant - default if omitted)
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
    default_cpu_request: "50m"
    default_memory_request: "64Mi"

  #! Full Capsule Tenant with Istio
  - name: acme
    type: "tenant"
    owners:
      - kind: User
        name: admin@acme.com
    namespace_quota: 10
    resources:
      cpu: "100"
      memory: "200Gi"
      storage: "1Ti"
      max_pvcs: 50
    cost_center: "ACME-ENG-001"
    billing_contact: "billing@acme.com"
    allow_loadbalancer: true
    #! Optional: Enable Istio service mesh
    istio:
      enabled: true
      strict_mtls: true

  #! Simple Namespace Provisioning (type: namespace)
  - name: simple-app
    type: "namespace"
    owners:
      - kind: User
        name: dev@example.com
        apiGroup: rbac.authorization.k8s.io
    cost_center: "development"
    description: "Simple namespace for standalone application"
    #! Optional: customize service account
    create_service_account: true  #! Default: true
    service_account_name: "deployer"  #! Default: "deployer"
```

### 2. Enable tenant-instances Component

```yaml
# In cluster.yaml components array
components:
  - name: tenant
    enabled: true
    version: "1.0.0"
    depends_on: [kro]

  - name: tenant-instances
    enabled: true
    version: "1.0.0"
    #! Configure dependencies based on what's enabled:
    #! - Only Capsule tenants: depends_on: [tenant]
    #! - Only simple namespaces: depends_on: [namespace]
    #! - Both types: depends_on: [tenant, namespace]
    depends_on: [tenant, namespace]
```

**Important:** Configure `depends_on` based on what tenant types you're actually using:

| Tenant Types in Use | Dependencies | Example |
|---------------------|--------------|---------|
| Only `type: "tenant"` | `[tenant]` | All tenants use Capsule |
| Only `type: "namespace"` | `[namespace]` | No Capsule, only simple namespaces |
| Both types | `[tenant, namespace]` | Mix of Capsule tenants and simple namespaces |

This ensures tenant-instances only waits for the RGDs it actually needs.

---

## Tenant Configuration Fields

See [schema.yaml:691-890](../../clusters/schema.yaml) for complete field definitions.

### Common Fields (Both Types)

**Required:**
- `name` - Unique tenant/namespace identifier (DNS-compliant)
- `owners` - Array of RBAC subjects with admin permissions

**Optional:**
- `type` - Provisioning type: `"tenant"` (default) or `"namespace"`
- `cost_center` - Cost center code for billing
- `billing_contact` - Contact email for billing (tenant type only)

### Type: "tenant" Fields (Capsule Tenant)

**Required:**
- `namespace_quota` - Maximum namespaces tenant can create
- `resources.cpu` - Total CPU quota across all namespaces
- `resources.memory` - Total memory quota
- `resources.storage` - Total storage quota
- `resources.max_pvcs` - Maximum PersistentVolumeClaims

**Optional:**
- `allow_loadbalancer` - Allow LoadBalancer-type Services (default: false)
- `default_cpu_limit` - Default CPU limit per container (default: 500m)
- `default_memory_limit` - Default memory limit per container (default: 512Mi)
- `default_cpu_request` - Default CPU request per container (default: 100m)
- `default_memory_request` - Default memory request per container (default: 128Mi)
- `istio.enabled` - Enable Istio sidecar injection (default: false)
- `istio.strict_mtls` - Enforce strict mTLS (default: true)

### Type: "namespace" Fields (Simple Namespace)

**Optional:**
- `description` - Description of namespace purpose
- `namespace_name` - Namespace name if different from tenant name
- `create_service_account` - Create deployer service account (default: true)
- `service_account_name` - Service account name (default: "deployer")
- `role_bindings` - Additional role bindings for other service accounts

---

## Alternative: Separate Tenant Files

For better organization, you can split tenants into separate files:

```bash
# Create tenant files
mkdir -p platform/clusters/onprem-dc1-dev-app/tenants

cat > platform/clusters/onprem-dc1-dev-app/tenants/test-tenant.yaml <<EOF
#@data/values
#@overlay/match-child-defaults missing_ok=True
---
tenants:
  - name: test-tenant
    # ... config ...
EOF

cat > platform/clusters/onprem-dc1-dev-app/tenants/acme.yaml <<EOF
#@data/values
#@overlay/match-child-defaults missing_ok=True
---
tenants:
  - name: acme
    # ... config ...
EOF
```

Then update build scripts to include tenant files:

```bash
# In build process, tenant files would be included via:
ytt -f platform/clusters/schema.yaml \
    -f platform/clusters/onprem-dc1-dev-app/cluster.yaml \
    -f platform/clusters/onprem-dc1-dev-app/tenants/*.yaml \
    -f platform/components/tenant-instances/base
```

ytt will automatically merge all tenant arrays together.

---

## Migration from Old Approach

If you have existing TenantOnboarding CRs in `config/` directory:

**Option 1: Move to cluster.yaml**

```bash
# Convert existing YAML to cluster data values
# (Manual process - extract fields from TenantOnboarding spec)
```

**Option 2: Keep both (transition period)**

- Leave `tenant-instances` disabled in components array
- Continue using manual `kubectl apply -f config/` approach
- Migrate tenants incrementally to cluster.yaml

**Option 3: Hybrid (not recommended)**

- Enable `tenant-instances` for some tenants (defined in cluster.yaml)
- Keep other tenants in `config/` directory (manual apply)
- This works but is confusing - pick one approach

---

## Benefits

1. ✅ **Single-step deployment** - No manual post-bootstrap step
2. ✅ **Schema validation** - ytt validates tenant configs at build time
3. ✅ **GitOps-friendly** - Tenant changes = commit to cluster.yaml
4. ✅ **Proper ordering** - Flux dependency ensures RGD deploys before instances
5. ✅ **Version control** - Tenant instances versioned as OCI artifacts
6. ✅ **Reusable** - Same artifact can be re-applied or rolled back

---

## Troubleshooting

### Tenant instances not deploying

```bash
# Check if tenant-instances component is enabled
yq '.components[] | select(.name == "tenant-instances")' \
  platform/clusters/onprem-dc1-dev-app/cluster.yaml

# Check Flux Kustomization status
kubectl get kustomization tenant-instances -n flux-system

# Check if tenant RGD deployed first
kubectl get kustomization tenant -n flux-system
```

### "no matches for kind TenantOnboarding" error

This means tenant-instances deployed before tenant RGD. Check dependency:

```bash
# Verify dependency exists in bootstrap output
fedcore bootstrap -c platform/clusters/onprem-dc1-dev-app \
  | grep -A10 "tenant-instances"

# Should show: dependsOn: [tenant]
```

### Tenant not created after deployment

```bash
# Check if TenantOnboarding CR was applied
kubectl get tenantonboarding

# Check KRO status
kubectl get tenant
kubectl describe tenantonboarding <name>
```

---

## Related Documentation

- [Tenant RGD](../../rgds/tenant/README.md)
- [Cluster Configuration Schema](../../clusters/schema.yaml)
- [Build Process](../../../scripts/README.md)

---

**Status:** ✅ Production ready
