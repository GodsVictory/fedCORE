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
#! overlay-phase: pre-render
---
#@overlay/match by=overlay.subset({"name": "tetragon"})
---
helm:
  values:
    tetragon:
      extraEnv:
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
#! overlay-phase: post-render
---
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: aws-iam-credential-access
spec:
  # ... policy definition
```

## Declaring Overlay Phase

Add a metadata comment at the top of your overlay file:

```yaml
#! overlay-phase: pre-render
#! OR
#! overlay-phase: post-render
```

If no metadata is present, the overlay defaults to `post-render` (backward compatible).

## Overlay Directory Structure

```
platform/components/<component>/overlays/
├── aws/
│   ├── values-overlay.yaml       # overlay-phase: pre-render
│   └── aws-resources.yaml        # overlay-phase: post-render
├── azure/
│   └── values-overlay.yaml
├── prod/
│   └── ha-overlay.yaml           # overlay-phase: pre-render
└── dev/
    └── dev-overlay.yaml

platform/clusters/<cluster>/overlays/
└── karpenter-tolerations.yaml        # overlay-phase: post-render (cluster overlays are always post-render)
```

## Build Flow

```bash
# 1. Collect overlays by phase
for each cloud/environment overlay:
  if "overlay-phase: pre-render" → PRE_RENDER_OVERLAYS[]
  else → POST_RENDER_OVERLAYS[]

# 2. Apply pre-render overlays to component.yaml
ytt -f schema.yaml -f cluster.yaml -f component.yaml -f ${PRE_RENDER_OVERLAYS[@]}
  → component-with-overlays.yaml

# 3. Extract helm.values from merged component.yaml
extract_helm_values(component-with-overlays.yaml)
  → values.yaml

# 4. Render Helm chart with merged values
helm template --values values.yaml
  → helm-rendered.yaml

# 5. Combine with base/ resources
cat helm-rendered.yaml base/*.yaml
  → combined.yaml

# 6. Apply post-render overlays
ytt -f schema.yaml -f cluster.yaml -f combined.yaml -f ${POST_RENDER_OVERLAYS[@]} -f cluster/overlays/
  → final.yaml
```

## Examples

### Example 1: Tetragon AWS Configuration

**Pre-render** (`platform/components/tetragon/overlays/aws/values-overlay.yaml`):
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
      #@overlay/match missing_ok=True
      extraEnv:
        #@overlay/append
        - name: CLOUD_PROVIDER
          value: "aws"
        #@overlay/append
        - name: AWS_REGION
          value: #@ data.values.region
```

**Post-render** (`platform/components/tetragon/overlays/aws/aws-policies.yaml`):
```yaml
#@ load("@ytt:data", "data")

#! overlay-phase: post-render
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

**Pre-render** (`platform/components/kyverno/overlays/prod/ha-overlay.yaml`):
```yaml
#@ load("@ytt:overlay", "overlay")

#! overlay-phase: pre-render
---
#@overlay/match by=overlay.subset({"name": "kyverno"})
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

#! overlay-phase: post-render
#! Note: Cluster overlays are always post-render

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

## When to Use Pre-render vs Post-render

### Use Pre-render when:
- ✅ The Helm chart exposes the value you want to modify
- ✅ You're adding environment variables, resource limits, replicas, etc.
- ✅ You want the chart's templating logic to apply (conditionals, loops)
- ✅ You want cleaner, more maintainable overlays

### Use Post-render when:
- ✅ Adding resources not in the Helm chart (CRDs, policies, etc.)
- ✅ The Helm chart doesn't expose the field you need to modify
- ✅ Applying universal patches (cluster tolerations, labels)
- ✅ Patching rendered resources in ways not supported by chart values

## Best Practices

1. **Prefer pre-render** - Modify Helm values when possible, it's cleaner
2. **Separate files** - One file per overlay phase for clarity
3. **Name files clearly** - `values-overlay.yaml` (pre-render), `aws-resources.yaml` (post-render)
4. **Document phase** - Always include `#! overlay-phase:` comment
5. **Test both phases** - Verify overlays work: `fedcore build -a <component> -c <cluster>`

## Migration from Old System

If you have overlays that patch rendered manifests (post-render), consider if they can be converted to pre-render overlays:

**Before (post-render):**
```yaml
#@overlay/match by=overlay.subset({"kind": "DaemonSet", "metadata": {"name": "tetragon"}})
---
spec:
  template:
    spec:
      containers:
      - name: tetragon
        env:
        - name: CLOUD_PROVIDER
          value: "aws"
```

**After (pre-render):**
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
```

The pre-render version is simpler, doesn't require matching container names, and leverages the chart's built-in support.

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

- ✅ Setting `depends_on` for a component (most common use case)
- ✅ Injecting default data values that are component-specific
- ✅ Any cluster data value override that should travel with the component

### When NOT to Use

- ❌ Modifying Helm values (use pre-render overlays instead)
- ❌ Adding Kubernetes resources (use post-render overlays or base/ manifests)
- ❌ Cluster-specific customizations (use cluster overlays in `platform/clusters/{cluster}/overlays/`)
