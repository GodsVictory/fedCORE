# Architecture & Methodology FAQ

**Why We Do Things the Way We Do**

This FAQ explains the architectural decisions and methodologies in the fedCORE platform, with emphasis on recent changes to the overlay and component system.

---

## Table of Contents

1. [Component Architecture](#component-architecture)
2. [Pre-Render vs Runtime Helm](#pre-render-vs-runtime-helm)
3. [Overlay System](#overlay-system)
4. [Component Structure](#component-structure)
5. [Build & Deployment](#build--deployment)
6. [Multi-Chart Component Splitting](#multi-chart-component-splitting)
7. [OCI Artifacts](#oci-artifacts)

---

## Component Architecture

### Why use `component.yaml` as a single source of truth?

**Decision:** All Helm-based components define their configuration in a single `component.yaml` file instead of separate HelmRepository and HelmRelease resources.

**Reasons:**

1. **Single Source of Truth** - All component metadata (chart name, version, repository, values) in one file
2. **Pre-Render Support** - Enables pre-rendering Helm charts at build time with overlays applied
3. **Consistency** - Same structure across all components makes maintenance easier
4. **Version Control** - Chart versions are explicit and version-controlled, not discovered at runtime
5. **Overlay Compatibility** - ytt overlays can modify Helm values before rendering

**Before (Runtime Helm):**
```yaml
# Multiple files, runtime discovery
apiVersion: source.toolkit.fluxcd.io/v1
kind: HelmRepository
---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
# Chart version discovered at runtime
```

**After (component.yaml):**
```yaml
#@ load("@ytt:data", "data")
---
name: tetragon
type: helm
helm:
  chart: tetragon
  version: "1.6.0"
  repo:
    type: oci
    url: oci://nexus.example.com/helm-charts
  values:
    # Helm values here
```

**Trade-offs:**
- ❌ More opinionated structure
- ❌ Requires custom build tooling
- ✅ Better control over versions
- ✅ Overlay system works seamlessly
- ✅ Pre-rendering enables cluster-wide customizations

---

### Why split multi-chart components into separate components?

**Decision:** Split `istio` into `istio` and `istio-gateway`.

**Reasons:**

1. **Independent Versioning** - Each component can be upgraded independently
   - Istio control plane: `v1.24.1`
   - Istio gateway: `v1.24.1`

2. **Selective Deployment** - Deploy only the components needed for a cluster
   - All clusters: Deploy `istio` control plane
   - Edge clusters: Deploy `istio-gateway` for ingress
   - Internal clusters: May skip gateway

3. **Cleaner Overlays** - Each component has its own overlay directory
   - Before: Complex overlays in one directory for multiple charts
   - After: Simple overlays per component

4. **Better Helm Support** - Each component uses a single Helm chart
   - Before: Custom logic to render multiple charts in one component
   - After: Standard Helm template process per component

**Example - Before:**
```
istio/
├── base/
│   └── istio.yaml        # Multiple HelmReleases (istiod + gateway)
└── overlays/
    └── cloud/aws/        # Both control plane and gateway configs mixed
```

**Example - After:**
```
ack-s3-controller/
├── base/
│   └── namespace.yaml
├── component.yaml        # Single chart, single version
└── overlays/             # S3-specific overlays only

ack-iam-controller/
├── base/
│   └── namespace.yaml
├── component.yaml        # Single chart, single version
└── overlays/             # IAM-specific overlays only
```

**Trade-offs:**
- ❌ More component directories
- ❌ More OCI artifacts to build
- ✅ Independent versioning
- ✅ Selective deployment
- ✅ Cleaner separation of concerns

---

## Pre-Render vs Runtime Helm

### Why pre-render Helm charts instead of using Flux HelmRelease at runtime?

**Decision:** Pre-render all Helm charts at build time and deploy as plain Kubernetes manifests, instead of using Flux HelmRelease resources that render charts at runtime.

**Key Problem Solved:** Cluster-wide overlays (tolerations, node selectors, labels) couldn't be applied to Helm charts when using runtime HelmRelease.

**Before (Runtime Helm with Flux):**
```yaml
# Flux renders chart at runtime in the cluster
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: kyverno
spec:
  chart:
    spec:
      chart: kyverno
      version: "3.2.6"
  values:
    replicas: 3
```

**Problem:** Cluster-specific overlays couldn't modify the rendered manifests because Flux rendered them at runtime.

**After (Pre-Render at Build Time):**
```bash
# Build script pre-renders Helm chart with all overlays applied
1. Apply pre-render overlays to component.yaml → modified values
2. helm template --values modified-values.yaml → rendered manifests
3. Apply post-render overlays to rendered manifests → final manifests
4. Push final manifests as OCI artifact
5. Flux deploys plain Kubernetes manifests (not HelmRelease)
```

**Benefits:**

1. **Universal Cluster Overlays** - Apply tolerations, node selectors to ALL components
   ```yaml
   # platform/clusters/my-cluster/overlays/karpenter-tolerations.yaml
   #! overlay-phase: post-render
   #@overlay/match by=overlay.subset({"kind": "Deployment"}), expects="0+"
   ---
   spec:
     template:
       spec:
         tolerations:
           - key: workload-type
             value: platform
   ```
   This overlay now applies to kyverno, istio, ingress-nginx, etc. - any Deployment in any component.

2. **Predictable Deployments** - Same manifests every time (no runtime variability)
3. **Audit Trail** - Exact manifests are version-controlled in OCI registry
4. **Faster Deployments** - No Helm rendering in cluster, just apply manifests
5. **Air-Gapped Friendly** - No need for Helm repository access at runtime

**Trade-offs:**
- ❌ Longer build times (pre-rendering takes time)
- ❌ Larger OCI artifacts (full manifests vs chart reference)
- ❌ Must rebuild to change values (can't just update HelmRelease)
- ✅ Cluster overlays work on ALL components
- ✅ Complete control over rendered output
- ✅ Deterministic deployments

**When Runtime Helm Makes Sense:**
- Development environments where quick iteration is needed
- Components that need dynamic values (e.g., from cluster secrets)
- External components not managed by platform team

**Why We Chose Pre-Render:**
Our primary use case is platform components that need cluster-wide customizations (tolerations, node affinity, labels). Pre-rendering enables this while maintaining GitOps principles and deterministic deployments.

---

## Overlay System

### Why use a two-phase overlay system (pre-render and post-render)?

**Decision:** Split overlays into two phases:
- **Pre-render** - Modify Helm values BEFORE rendering
- **Post-render** - Modify rendered manifests AFTER Helm template

**Reasons:**

1. **Leverage Helm Chart Logic** - Pre-render overlays use the chart's built-in templating
   ```yaml
   #! overlay-phase: pre-render
   ---
   helm:
     values:
       tetragon:
         extraEnv:           # Chart knows how to add env vars
           - name: CLOUD_PROVIDER
             value: "aws"
   ```
   The chart's templates will place this in the correct location with proper formatting.

2. **Add Resources Not in Chart** - Post-render overlays add resources the chart doesn't provide
   ```yaml
   #! overlay-phase: post-render
   ---
   apiVersion: cilium.io/v1alpha1
   kind: TracingPolicy       # Not in Helm chart
   metadata:
     name: aws-iam-monitoring
   ```

3. **Cleaner Overlays** - Pre-render overlays are simpler and more maintainable
   - Don't need to match container names, deployment names, etc.
   - Just specify the Helm value path
   - Chart handles the rest

4. **Universal Patches** - Post-render overlays can patch ALL resources of a type
   ```yaml
   #! overlay-phase: post-render
   #@overlay/match by=overlay.subset({"kind": "Deployment"}), expects="0+"
   ---
   spec:
     template:
       spec:
         tolerations: [...]  # Applies to ALL Deployments
   ```

**Example: Tetragon AWS Configuration**

**Pre-render** (modify Helm values):
```yaml
#! overlay-phase: pre-render
#@overlay/match by=overlay.subset({"name": "tetragon"})
---
helm:
  values:
    tetragon:
      extraEnv:
        - name: CLOUD_PROVIDER
          value: "aws"
        - name: AWS_REGION
          value: "us-east-1"
```

**Post-render** (add AWS-specific TracingPolicies):
```yaml
#! overlay-phase: post-render
---
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: aws-iam-credential-access
spec:
  kprobes:
    - call: "security_file_open"
      # Detect access to AWS credential files
```

**Build Flow:**
```bash
1. Collect pre-render overlays → apply to component.yaml
2. Extract helm.values from merged component.yaml
3. helm template --values values.yaml → rendered manifests
4. Collect post-render overlays → apply to rendered manifests
5. Final manifests → OCI artifact
```

**When to Use Each Phase:**

| Use Case | Phase | Reason |
|----------|-------|--------|
| Modify Helm values | Pre-render | Leverage chart's templating logic |
| Add env vars, resources, replicas | Pre-render | Chart exposes these as values |
| Add CRDs, policies, resources | Post-render | Not in the Helm chart |
| Universal patches (tolerations) | Post-render | Apply to all resources of a type |
| Chart doesn't expose the field | Post-render | No Helm value available |

**Trade-offs:**
- ❌ More complex build process (two phases)
- ❌ Developers must understand which phase to use
- ✅ Cleaner, more maintainable overlays
- ✅ Leverage Helm chart capabilities
- ✅ Support universal patches

---

### Why use ytt for overlays instead of Kustomize or Helm values files?

**Decision:** Use ytt (YAML Templating Tool) for all overlays instead of Kustomize or Helm values files.

**Comparison:**

| Feature | ytt | Kustomize | Helm Values |
|---------|-----|-----------|-------------|
| Strategic merge | ✅ Yes | ✅ Yes | ❌ No |
| Conditional logic | ✅ Yes | ❌ No | ✅ Yes (in chart) |
| Functions/loops | ✅ Yes | ❌ No | ✅ Yes (in chart) |
| Data values | ✅ Yes | ❌ No | ✅ Yes |
| Overlay matching | ✅ Powerful | ⚠️ Limited | ❌ N/A |
| Pre-render support | ✅ Yes | ⚠️ Limited | ✅ Yes |
| Post-render support | ✅ Yes | ✅ Yes | ❌ No |

**Reasons for ytt:**

1. **Unified Tooling** - Same tool for pre-render and post-render overlays
2. **Powerful Matching** - Flexible overlay matching with predicates
   ```yaml
   #@overlay/match by=overlay.subset({"kind": "Deployment", "metadata": {"name": "kyverno"}}), missing_ok=True
   ```
3. **Data Values** - Access cluster config in overlays
   ```yaml
   #@ load("@ytt:data", "data")
   #@ data.values.cloud           # "aws"
   #@ data.values.environment     # "prod"
   #@ data.values.cluster_name    # "fedcore-prod-use1"
   ```
4. **Conditional Logic** - Apply overlays conditionally
   ```yaml
   #@ if data.values.cloud == "aws":
   # AWS-specific configuration
   #@ end
   ```
5. **Functions** - Reusable logic
   ```yaml
   #@ def commonLabels():
   app.kubernetes.io/managed-by: fedcore
   platform.fedcore.io/cluster: #@ data.values.cluster_name
   #@ end
   ```

**Example - Overlay with Data Values:**
```yaml
#@ load("@ytt:data", "data")
#@ load("@ytt:overlay", "overlay")

#! overlay-phase: pre-render
---
#@overlay/match by=overlay.subset({"name": "tetragon"})
---
helm:
  values:
    tetragon:
      extraEnv:
        - name: CLOUD_PROVIDER
          value: #@ data.values.cloud         # Dynamic from cluster.yaml
        - name: ENVIRONMENT
          value: #@ data.values.environment   # Dynamic from cluster.yaml
```

**Trade-offs:**
- ❌ Learning curve (ytt syntax)
- ❌ Smaller community than Kustomize
- ✅ More powerful than Kustomize
- ✅ Unified pre/post-render workflow
- ✅ Access to cluster metadata

---

### Why organize overlays by cloud and environment?

**Decision:** Overlay directories follow this structure:
```
component/
├── base/
└── overlays/
    ├── aws/
    ├── azure/
    ├── onprem/
    ├── prod/
    ├── dev/
    └── staging/
```

**Reasons:**

1. **Cloud-Specific Resources** - AWS, Azure, and on-prem have different resources
   - AWS: ACK controllers (S3, IAM, RDS)
   - Azure: Azure Service Operator (Storage, KeyVault, SQL)
   - On-prem: StatefulSets, NFS storage

2. **Environment-Specific Policies** - Production needs stricter controls
   - Prod: Enforce resource limits, require image tags, strict mTLS
   - Dev: Relaxed policies for faster iteration

3. **Automatic Application** - Build script applies overlays based on cluster config
   ```yaml
   # platform/clusters/fedcore-prod-use1/cluster.yaml
   cloud: aws
   environment: prod
   # Build script automatically applies:
   #   component/overlays/aws/*
   #   component/overlays/prod/*
   ```

4. **Separation of Concerns** - Cloud and environment are orthogonal
   - You can have AWS dev and AWS prod (same cloud, different environment)
   - You can have AWS prod and Azure prod (different clouds, same environment)
   - Overlays don't duplicate logic

**Example - Istio Overlays:**

```
istio/
├── base/
│   ├── istio-base.yaml
│   └── namespace.yaml
├── component.yaml
└── overlays/
    ├── aws/
    │   └── istiod-config.yaml        # AWS-specific: EKS Pod Identity
    ├── azure/
    │   └── istiod-config.yaml        # Azure-specific: AKS Workload Identity
    ├── onprem/
    │   └── istiod-config.yaml        # On-prem: No cloud identity
    ├── prod/
    │   ├── istiod-ha-config.yaml     # Prod: 3 replicas, HA
    │   └── security-policies.yaml    # Prod: STRICT mTLS
    └── dev/
        └── istiod-config.yaml        # Dev: 1 replica, PERMISSIVE mTLS
```

**Overlay Application Order:**
```
1. Base templates
   ↓
2. Cloud overlays (component/overlays/{cloud}/)
   ↓
3. Environment overlays (component/overlays/{env}/)
   ↓
4. Cluster overlays (platform/clusters/{cluster}/overlays/)
```

Each layer has higher precedence, allowing fine-grained control.

**Trade-offs:**
- ❌ More directories to manage
- ❌ Overlays can get complex with many layers
- ✅ Clear separation of concerns
- ✅ Reusable across clusters
- ✅ Automatic application based on cluster metadata

---

## Component Structure

### Why include `base/` resources alongside Helm charts?

**Decision:** Components can have both Helm charts (defined in `component.yaml`) and plain Kubernetes manifests (in `base/` directory).

**Structure:**
```
component/
├── base/
│   ├── namespace.yaml          # Plain manifest
│   ├── rbac.yaml               # Plain manifest
│   └── custom-resource.yaml    # Plain manifest
├── component.yaml              # Helm chart definition
└── overlays/
```

**Build Process:**
```bash
1. Pre-render overlays → component.yaml (Helm values modified)
2. helm template → rendered-chart.yaml
3. Combine: rendered-chart.yaml + base/*.yaml → combined.yaml
4. Post-render overlays → combined.yaml
5. Output: final.yaml
```

**Reasons:**

1. **Resources Not in Chart** - Add resources the Helm chart doesn't provide
   - Namespace (many charts don't create the namespace)
   - Additional RBAC rules
   - Custom CRDs
   - Policies

2. **Cluster-Specific Resources** - Add resources specific to your platform
   - NetworkPolicies for fedCORE multi-tenancy
   - PodDisruptionBudgets for HA
   - ServiceMonitors for Prometheus

3. **Simple Resources** - No need for Helm templating for static resources
   ```yaml
   # base/namespace.yaml - no templating needed
   apiVersion: v1
   kind: Namespace
   metadata:
     name: kyverno
   ```

4. **Consistent Structure** - All components follow same pattern
   - Even if Helm chart is 90% of the component
   - base/ provides predictable location for additional resources

**Example - Tetragon Component:**

```
tetragon/
├── base/
│   ├── namespace.yaml          # Create kube-system namespace (if needed)
│   └── tracing-policies.yaml  # Custom TracingPolicy CRDs
├── component.yaml              # Helm chart (tetragon DaemonSet, operator)
└── overlays/
    ├── aws/
    │   ├── env-config.yaml          # Pre-render: AWS env vars
    │   └── aws-policies.yaml        # Post-render: AWS TracingPolicies
    └── azure/
        ├── env-config.yaml          # Pre-render: Azure env vars
        └── azure-policies.yaml      # Post-render: Azure TracingPolicies
```

**Build Output:**
```
1. helm template (tetragon chart) → tetragon-daemonset.yaml, tetragon-operator.yaml
2. base/tracing-policies.yaml → base-policies.yaml
3. Combined → tetragon-complete.yaml
4. Post-render overlays (aws-policies.yaml) → tetragon-final.yaml
```

**Trade-offs:**
- ❌ Mixing Helm and plain manifests can be confusing
- ❌ Must understand which resources come from Helm vs base/
- ✅ Flexibility to add any resource
- ✅ Consistent structure across components
- ✅ Simple resources don't need Helm

---

## Build & Deployment

### Why build OCI artifacts instead of pointing Flux directly at the Git repository?

**Decision:** Build components into OCI artifacts (container images containing YAML manifests), push to Nexus, and have Flux pull from OCI registry.

**Before (Git-based Flux):**
```yaml
apiVersion: source.toolkit.fluxcd.io/v1
kind: GitRepository
metadata:
  name: platform
spec:
  url: https://github.com/org/platform
---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: kyverno
spec:
  sourceRef:
    kind: GitRepository
    name: platform
  path: ./platform/components/kyverno
```

**After (OCI-based Flux):**
```bash
# Build creates OCI artifact
fedcore build --artifact platform/components/kyverno --cluster platform/clusters/fedcore-prod-use1
# Output: ghcr.io/org/platform/components/kyverno:fedcore-prod-use1-abc123
```

```yaml
apiVersion: source.toolkit.fluxcd.io/v1beta2
kind: OCIRepository
metadata:
  name: kyverno
spec:
  url: oci://ghcr.io/org/platform/components/kyverno
  ref:
    tag: fedcore-prod-use1-abc123
---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: kyverno
spec:
  sourceRef:
    kind: OCIRepository
    name: kyverno
```

**Reasons:**

1. **Pre-Built Artifacts** - Overlays applied at build time, not runtime
   - Build output is deterministic
   - Same manifests for every deployment
   - No ytt/Helm needed in cluster

2. **Air-Gapped Support** - OCI artifacts can be mirrored to private registries
   - No GitHub access needed from clusters
   - Nexus hosts all artifacts internally
   - Compliance with air-gapped requirements

3. **Immutable Deployments** - OCI artifacts are immutable
   - Git commits can be rewritten/force-pushed (bad practice, but possible)
   - OCI tags are content-addressable (digest-based)
   - Audit trail: exact artifact deployed is known

4. **Cluster-Specific Builds** - Each cluster gets its own artifact
   - AWS prod cluster: `kyverno:fedcore-prod-use1-abc123`
   - Azure dev cluster: `kyverno:azure-dev-westus-abc123`
   - Different overlays applied per cluster
   - No runtime conditional logic needed

5. **Faster Flux Reconciliation** - Flux just applies manifests
   - No kustomize build at runtime
   - No ytt processing at runtime
   - No Helm rendering at runtime
   - Just `kubectl apply -f`

**Two-Tier Architecture:**

**Tier 1 (Infrastructure Components):**
```
platform/components/kyverno → OCI artifact per cluster
  oci://nexus/fedcore/components/kyverno:fedcore-prod-use1-v1.2.3
  oci://nexus/fedcore/components/kyverno:azure-dev-westus-v1.2.3
```

**Tier 2 (RGDs - Platform APIs):**
```
platform/rgds/webapps → OCI artifact (same for all clusters)
  oci://nexus/fedcore/rgds/webapps:v1.0.0
```

RGDs are the same across clusters (they're API definitions), so one artifact per version.

**Trade-offs:**
- ❌ Longer deployment time (must build artifacts first)
- ❌ More infrastructure (OCI registry required)
- ❌ Larger storage requirements (artifacts per cluster)
- ✅ Deterministic deployments
- ✅ Air-gapped support
- ✅ Faster Flux reconciliation
- ✅ Complete control over output

---

### Why use GitHub Actions matrix strategy for builds?

**Decision:** Build all cluster-component combinations in parallel using GitHub Actions matrix strategy.

**Workflow:**
```yaml
strategy:
  matrix:
    cluster:
      - fedcore-prod-use1
      - fedcore-dev-use1
      - azure-prod-westus
    component:
      - platform/components/kyverno
      - platform/components/istio
      - platform/components/capsule
```

**Reasons:**

1. **Parallelization** - Build 50+ artifacts in parallel instead of sequentially
   - Sequential: 50 artifacts × 2 minutes = 100 minutes
   - Parallel: ~5-10 minutes total (GitHub limits: 20 concurrent jobs on free tier)

2. **Fail Fast** - If one combination fails, others continue
   - Know immediately which cluster-component combo is broken
   - Don't wait 100 minutes to discover last artifact failed

3. **Independent Artifacts** - Each combination is independent
   - AWS prod kyverno doesn't depend on Azure dev istio
   - Perfect for parallelization

4. **Scalability** - Adding clusters/components scales well
   - Add new cluster: builds for all components automatically created
   - Add new component: builds for all clusters automatically created

**Example - Platform with 5 Clusters, 10 Components:**
- Total artifacts: 5 × 10 = 50
- Sequential build time: 50 × 2 min = 100 minutes
- Parallel build time: ~10 minutes (GitHub Actions limit)

**Trade-offs:**
- ❌ More complex GitHub Actions workflow
- ❌ Higher GitHub Actions minutes usage (but faster total time)
- ❌ Harder to debug (50 jobs running concurrently)
- ✅ Dramatically faster CI/CD pipeline
- ✅ Scales with platform growth
- ✅ Fail fast on errors

---

## Multi-Chart Component Splitting

**Trade-offs:**
- ❌ More component directories (3 instead of 1)
- ❌ More OCI artifacts to build
- ❌ Breaking change requiring cluster config updates
- ✅ Independent versioning
- ✅ Clearer separation
- ✅ Simpler maintenance

---

### Why split `istio` into `istio` and `istio-gateway`?

**Context:** The original `istio` component deployed both the control plane (istiod) and the ingress gateway.

**Decision:** Split into:
- `istio` - Control plane (istiod)
- `istio-gateway` - Ingress gateway

**Reasons:**

1. **Different Lifecycles** - Control plane and gateway have different upgrade considerations
   - Control plane: Upgrade carefully (affects all services)
   - Gateway: Upgrade more frequently (only affects ingress)

2. **Separate Scaling** - Gateway and control plane scale independently
   - Control plane: 2-3 replicas (HA)
   - Gateway: 3-10 replicas (handle traffic)

3. **Cloud-Specific Gateway Configs** - Gateway needs heavy cloud customization
   - AWS: NLB annotations, Pod Identity
   - Azure: Azure LB annotations, Workload Identity
   - On-prem: MetalLB or NodePort
   - Control plane: Mostly the same across clouds

4. **Optional Gateway** - Some clusters might not need ingress gateway
   - Internal clusters: Control plane only
   - Edge clusters: Control plane + gateway

5. **Helm Charts Separation** - Istio project ships separate charts
   - `istio/base` - CRDs
   - `istio/istiod` - Control plane
   - `istio/gateway` - Gateway
   - Aligning with upstream chart structure

**Component Structure:**

**istio:**
```
istio/
├── base/
│   ├── istio-base.yaml      # CRDs (from base chart)
│   ├── namespace.yaml
│   └── policies.yaml        # Platform policies
├── component.yaml           # istiod chart
└── overlays/
    ├── aws/
    │   └── istiod-config.yaml    # EKS Pod Identity
    ├── azure/
    │   └── istiod-config.yaml    # AKS Workload Identity
    └── prod/
        └── istiod-ha-config.yaml  # 3 replicas, HA
```

**istio-gateway:**
```
istio-gateway/
├── base/
│   └── empty.yaml           # Placeholder
├── component.yaml           # gateway chart
└── overlays/
    ├── aws/
    │   └── service-annotations.yaml  # NLB annotations
    ├── azure/
    │   └── service-annotations.yaml  # Azure LB annotations
    └── onprem/
        └── service-config.yaml       # MetalLB/NodePort
```

**Trade-offs:**
- ❌ Two components instead of one
- ❌ Breaking change requiring cluster config updates
- ✅ Independent scaling
- ✅ Clearer separation of concerns
- ✅ Aligns with upstream Helm chart structure

---

## OCI Artifacts

### Why use OCI artifacts for both components and RGDs?

**Decision:** Store all platform artifacts (components and RGDs) in OCI registries, not Git repositories.

**Two Tiers:**

**Tier 1 - Infrastructure Components (cluster-specific):**
```
oci://nexus/fedcore/components/kyverno:fedcore-prod-use1-abc123
oci://nexus/fedcore/components/istio:azure-dev-westus-abc123
```

**Tier 2 - RGDs (version-tagged, same for all clusters):**
```
oci://nexus/fedcore/rgds/webapps:v1.0.0
oci://nexus/fedcore/rgds/dynamodb:v1.2.0
```

**Reasons:**

1. **Uniform Distribution** - Same mechanism for all artifacts
   - Flux pulls components from OCI
   - Flux pulls RGDs from OCI
   - No special cases

2. **Immutability** - OCI artifacts are content-addressable
   - Digest-based verification: `kyverno@sha256:abc123...`
   - Can't be tampered with after push
   - Audit trail: know exact artifact deployed

3. **Air-Gapped Support** - Mirror entire registry
   ```bash
   # Mirror production registry to air-gapped environment
   for artifact in $(list-artifacts); do
     crane copy $artifact air-gapped-nexus/$artifact
   done
   ```

4. **Version Control** - RGDs use semantic versioning
   ```
   webapps:v1.0.0 → Initial release
   webapps:v1.1.0 → Add PostgreSQL support
   webapps:v2.0.0 → Breaking change (new schema)
   ```
   Tenants can pin to specific versions:
   ```yaml
   apiVersion: source.toolkit.fluxcd.io/v1beta2
   kind: OCIRepository
   spec:
     url: oci://nexus/fedcore/rgds/webapps
     ref:
       tag: v1.1.0  # Pin to specific version
   ```

5. **Efficient Storage** - OCI registries deduplicate layers
   - Common base layers shared across artifacts
   - Only deltas stored for each version

**Comparison to Git:**

| Aspect | OCI Artifacts | Git Repository |
|--------|---------------|----------------|
| Immutability | ✅ Content-addressable | ⚠️ Tags can move |
| Air-gapped | ✅ Easy mirroring | ❌ Complex (Git mirror) |
| Versioning | ✅ Semantic tags | ✅ Git tags |
| Storage efficiency | ✅ Layer deduplication | ❌ Full history |
| Distribution | ✅ OCI registries | ⚠️ Git clones |
| Flux support | ✅ OCIRepository | ✅ GitRepository |

**Trade-offs:**
- ❌ Requires OCI registry infrastructure
- ❌ Build step needed (can't point Flux at Git)
- ❌ Less familiar to developers (OCI vs Git)
- ✅ Immutable, auditable artifacts
- ✅ Air-gapped friendly
- ✅ Efficient storage and distribution

---

### Why version components per-cluster instead of semantic versioning?

**Decision:** Infrastructure components use cluster-specific tags (e.g., `fedcore-prod-use1-abc123`) instead of semantic versions (e.g., `v1.0.0`).

**Tier 1 Components (cluster-specific):**
```
oci://nexus/fedcore/components/kyverno:fedcore-prod-use1-abc123
oci://nexus/fedcore/components/kyverno:azure-dev-westus-abc123
```

**Tier 2 RGDs (semantic versioning):**
```
oci://nexus/fedcore/rgds/webapps:v1.0.0
oci://nexus/fedcore/rgds/dynamodb:v1.2.0
```

**Reasons:**

1. **Different Artifacts Per Cluster** - Each cluster's build is unique
   - AWS prod: AWS-specific overlays applied
   - Azure dev: Azure-specific overlays applied
   - Same component, different rendered manifests
   - Can't share a version tag

2. **No Semantic Meaning** - Component versions are build artifacts
   - Not published APIs (like RGDs)
   - No breaking changes to worry about
   - No consumers depending on specific versions

3. **Git Commit Linkage** - Cluster tag includes Git commit
   - `fedcore-prod-use1-abc123` → cluster + Git SHA
   - Easy to trace artifact back to source code
   - Audit trail: "What code built this artifact?"

4. **Flux Tracks Git** - Clusters automatically get new builds
   ```yaml
   apiVersion: source.toolkit.fluxcd.io/v1beta2
   kind: OCIRepository
   spec:
     url: oci://nexus/fedcore/components/kyverno
     ref:
       tag: fedcore-prod-use1-${GIT_SHA}  # Updated by CI
   ```
   When Git SHA changes, Flux pulls new artifact.

**Contrast with RGDs:**

RGDs are **platform APIs** consumed by tenants:
- Tenants depend on specific RGD versions
- Breaking changes need major version bump
- Semantic versioning communicates compatibility
- Tenants can pin to stable versions

**Example:**
```yaml
# Tenant pins to stable RGD version
apiVersion: source.toolkit.fluxcd.io/v1beta2
kind: OCIRepository
spec:
  url: oci://nexus/fedcore/rgds/webapps
  ref:
    semver: ">=1.0.0 <2.0.0"  # Semantic versioning range
```

**Trade-offs:**
- ❌ Can't easily rollback to "previous version" (must use Git SHA)
- ❌ Not human-friendly tags
- ✅ Accurate representation (different artifacts per cluster)
- ✅ Git SHA traceability
- ✅ Simple CI/CD (no version bump logic)

---

## Summary

### Core Principles

1. **Pre-Render Everything** - Build-time rendering enables cluster-wide overlays
2. **Single Source of Truth** - component.yaml defines all component metadata
3. **Two-Phase Overlays** - Pre-render (Helm values) + Post-render (manifests)
4. **OCI Distribution** - Immutable, air-gapped friendly artifact distribution
5. **Separation of Concerns** - Split components by chart, cloud, and environment

### Key Benefits

- ✅ **Universal cluster overlays** (tolerations, node selectors) apply to all components
- ✅ **Deterministic deployments** (same manifests every time)
- ✅ **Independent versioning** (each controller/component upgrades separately)
- ✅ **Air-gapped support** (OCI artifacts mirror easily)
- ✅ **Clear separation** (cloud vs environment vs cluster overlays)

### Trade-Offs Accepted

- ❌ **Longer build times** (pre-rendering vs runtime)
- ❌ **More artifacts** (per-cluster components)
- ❌ **Learning curve** (ytt, two-phase overlays)
- ❌ **Breaking changes** (component splits required config updates)

---

## Related Documentation

- [Overlay System Reference](../platform/components/OVERLAY-SYSTEM.md) - Two-phase overlay details
- [Component README](../platform/components/README.md) - Component structure
- [Build Process Flow](BUILD_PROCESS_FLOW.md) - Build process reference
- [Helm Charts Guide](HELM_CHARTS.md) - Helm OCI registry setup
- [Deployment Guide](DEPLOYMENT.md) - CI/CD pipeline
- [FAQ](FAQ.md) - General platform questions

---

## Navigation

[← Previous: FAQ](FAQ.md) | [Next: Getting Started →](GETTING_STARTED.md)

**Handbook Progress:** Supplementary | **Level 8:** Architecture Deep Dives

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
