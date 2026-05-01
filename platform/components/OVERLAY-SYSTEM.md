# Component Overlay System

This document explains the two-phase overlay system for component builds.

## Overview

The build system supports two types of overlays:

1. **Pre-render overlays** - Applied to `component.yaml` to modify Helm values BEFORE rendering
2. **Post-render overlays** - Applied to rendered manifests AFTER Helm template

## Why Two Phases?

### Pre-render (Modify Helm Values)
Some Helm charts expose values for configuration (e.g., `extraEnv`, `resources`, `replicas`). It's cleaner to modify these values before rendering rather than patching the rendered manifests.

**Example:** Adding AWS environment variables to Tetragon
```yaml
#@ load("@ytt:data", "data")
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.subset({"helm": {}}), expects="0+"
---
helm:
  values:
    tetragon:
      extraEnv:
        #@overlay/append
        - name: CLOUD_PROVIDER
          value: "aws"
```

### Post-render (Add/Patch Resources)
Some resources cannot be configured via Helm values:
- Additional CRDs or resources not in the chart
- Patches to rendered manifests (though pre-render is preferred)
- Universal patches like cluster-wide tolerations

**Example:** Adding AWS-specific TracingPolicies
```yaml
---
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: aws-iam-credential-access
spec:
  # ... policy definition
```

## Overlay Directory Structure

Overlay phase is determined by directory placement — **pre-render/** or **post-render/** subdirectories:

```
platform/components/<component>/overlays/
├── aws/
│   ├── pre-render/
│   │   └── service-annotations.yaml     # Modifies helm values before rendering
│   └── post-render/
│       └── aws-policies.yaml            # Patches rendered manifests
├── azure/
│   ├── pre-render/
│   │   └── env-config.yaml
│   └── post-render/
│       └── azure-policies.yaml
├── prod/
│   ├── pre-render/
│   │   └── ha-config.yaml               # Scale up replicas for prod
│   └── post-render/
│       └── security-policies.yaml
└── dev/
    └── post-render/
        └── dev-overlay.yaml

platform/clusters/<cluster>/overlays/     # Cluster-level overlays (always post-render)
└── karpenter-tolerations.yaml
```

## Build Flow

```bash
# 1. Resolve cluster.yaml (envsubst for ${VAR} placeholders)
# 2. Resolve component matrix (ytt matrix template)

# 3. Pre-render: merge component.yaml with cluster values + overlays
ytt -f schema.yaml \
    -f cluster.yaml \
    -f component.yaml \
    -f platform/build/prerender/ \
    --data-values-file component-values.yaml \
    -f overlays/{cloud}/pre-render/ \
    -f overlays/{env}/pre-render/
  → component-merged.yaml
# platform/build/prerender/ handles:
#   - chart mirror URL injection (resolvedChartRef)
#   - deep merge of cluster-level helm.values overrides

# 4. Render Helm chart with merged values
helm template {release_name} {chart} --values values.yaml
  → helm-rendered.yaml

# 5. Post-render + base manifests in one ytt call
ytt --ignore-unknown-comments \
    -f schema.yaml \
    -f cluster.yaml \
    -f helm-rendered.yaml \
    --data-values-file component-values.yaml \
    -f base/ \
    -f overlays/{cloud}/post-render/ \
    -f overlays/{env}/post-render/ \
    -f cluster/overlays/
  → final.yaml
```

## Examples

### Example 1: Tetragon AWS Configuration

**Pre-render** (`platform/components/tetragon/overlays/aws/pre-render/env-config.yaml`):
```yaml
#@ load("@ytt:data", "data")
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.subset({"helm": {}}), expects="0+"
---
helm:
  values:
    tetragon:
      #@overlay/match missing_ok=True
      extraEnv:
        #@overlay/append
        - name: CLOUD_PROVIDER
          value: "aws"
        #@overlay/append
        - name: AWS_REGION
          value: #@ data.values.region
```

**Post-render** (`platform/components/tetragon/overlays/aws/post-render/aws-policies.yaml`):
```yaml
#@ load("@ytt:data", "data")
---
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: aws-iam-credential-access
  namespace: kube-system
spec:
  kprobes:
  - call: "security_file_open"
    # ... policy definition
```

### Example 2: High Availability Overlay

**Pre-render** (`platform/components/kyverno/overlays/prod/pre-render/ha-overlay.yaml`):
```yaml
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.subset({"helm": {}}), expects="0+"
---
helm:
  values:
    admissionController:
      replicas: 3
    backgroundController:
      replicas: 2
```

### Example 3: Cluster-wide Tolerations

**Post-render** (`platform/clusters/*/overlays/karpenter-tolerations.yaml`):
```yaml
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.subset({"kind": "Deployment"}), expects="0+"
---
spec:
  template:
    spec:
      tolerations:
        - key: workload-type
          operator: Equal
          value: platform
          effect: NoSchedule
```

## Component Helm Overrides (Preferred for Value Customization)

Before reaching for overlays, consider using the `helm:` key on the component entry in `cluster.yaml`. The build pipeline deep-merges these overrides into the component's helm config during pre-render (via the ytt overlay in `platform/build/prerender/`). This is the simplest way to customize helm values per cluster or per instance.

```yaml
# cluster.yaml
components:
  - name: kong-ingress
    id: kong-dev
    helm:
      values:
        gateway:
          replicaCount: 1
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
        controller:
          replicaCount: 1
```

The merge is recursive — only the keys you specify are overridden, everything else keeps the component.yaml defaults. This works for any helm value the chart exposes, and supports multiple instances of the same component with different configurations.

### Use component `helm:` overrides when:
- Adjusting resource sizing, replica counts, or feature flags per cluster
- Running multiple instances of the same component with different configs
- The override is specific to a single cluster or instance

### Use pre-render overlays when:
- The override applies to all clusters with a given cloud/environment
- The overlay needs complex ytt logic (conditionals, loops, data.values references)
- You need to append to arrays (overlays support `#@overlay/append`, deep merge replaces arrays)

## When to Use Pre-render vs Post-render Overlays

### Use Pre-render when:
- The Helm chart exposes the value you want to modify
- The change applies to all clusters with a given cloud or environment
- You want the chart's templating logic to apply (conditionals, loops)

### Use Post-render when:
- Adding resources not in the Helm chart (CRDs, policies, etc.)
- The Helm chart doesn't expose the field you need to modify
- Applying universal patches (cluster tolerations, labels)
- Patching rendered resources in ways not supported by chart values

## Best Practices

1. **Prefer component `helm:` overrides** - For per-cluster/per-instance value customization
2. **Use pre-render overlays** - For cloud/environment-wide changes that need ytt logic
3. **Separate files by phase** - Place in `pre-render/` or `post-render/` subdirectory
4. **Name files clearly** - `service-annotations.yaml` (pre-render), `aws-policies.yaml` (post-render)
5. **Test both phases** - Verify overlays work: `fedcore build -a <component> -c <cluster>`

## Bootstrap Overlays (overlay.yaml)

In addition to the two-phase build overlays above, components can include a **bootstrap overlay** — a file named `overlay.yaml` at the component root. These overlays are not used during `fedcore build`; they are collected and applied during `fedcore bootstrap` to inject component-specific data into the cluster's data values before bootstrap rendering.

### Purpose

Bootstrap overlays allow components to declare their own metadata (such as `depends_on`) without requiring users to manually specify it in `cluster.yaml`. The CLI automatically discovers `overlay.yaml` files from all listed components and includes them as ytt data value overlays.

### How It Works

```bash
# During fedcore bootstrap:
# 1. Read cluster.yaml components list
# 2. For each component, check if {component_path}/overlay.yaml exists
# 3. Include all found overlay.yaml files as -f arguments to ytt
# 4. ytt merges them into the cluster data values before rendering bootstrap manifests
```

### Example: Declaring depends_on

`platform/components/tenant-instances/overlay.yaml`:
```yaml
#@data/values
---
#@overlay/match missing_ok=True
components:
#@overlay/match by=lambda idx,old,new: old["name"] == "tenant-instances"
- depends_on:
  - namespace
```

This ensures `tenant-instances` always depends on `namespace` without users having to remember to set it in every cluster.yaml.

### When to Use

- Setting `depends_on` for a component (most common use case)
- Injecting default data values that are component-specific
- Any cluster data value override that should travel with the component

### When NOT to Use

- Modifying Helm values (use pre-render overlays instead)
- Adding Kubernetes resources (use post-render overlays or base/ manifests)
- Cluster-specific customizations (use cluster overlays in `platform/clusters/{cluster}/overlays/`)
