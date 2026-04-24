# Component Sources Bootstrap

GitOps bootstrap configuration for deploying infrastructure components from OCI artifacts.

## Overview

This bootstrap component is the **entry point** for cluster initialization. It generates Flux `OCIRepository` and `Kustomization` resources that deploy all infrastructure components to a cluster. This is the foundation of the platform's GitOps approach.

## What This Component Does

When a cluster is bootstrapped, this component:

1. **Creates OCIRepository resources** pointing to versioned component artifacts
2. **Creates Kustomization resources** that tell Flux how to deploy each component
3. **Manages component dependencies** to ensure correct deployment order
4. **Handles component versioning** through semantic version constraints

## How Bootstrap Works

```
┌──────────────────────────────────────────────────┐
│ 1. Build Infrastructure Artifacts               │
│                                                  │
│    For each component, run:                      │
│    fedcore build                                 │
│      --artifact platform/components/<component>  │
│      --cluster fedcore-prod-use1                 │
│                                                  │
│    Processes each component:                     │
│    - cloud-permissions → OCI artifact            │
│    - capsule → OCI artifact                      │
│    - kro → OCI artifact                          │
│    - kyverno-policies → OCI artifact             │
└────────────────┬─────────────────────────────────┘
                 │
                 ↓
┌──────────────────────────────────────────────────┐
│ 2. Generate Bootstrap Configuration              │
│                                                  │
│    fedcore bootstrap                             │
│      --cluster fedcore-prod-use1                 │
│                                                  │
│    Outputs:                                      │
│    - OCIRepository resources for each component  │
│    - Kustomization resources with dependencies   │
└────────────────┬─────────────────────────────────┘
                 │
                 ↓
┌──────────────────────────────────────────────────┐
│ 3. Apply to Cluster                              │
│                                                  │
│    kubectl apply -f bootstrap.yaml               │
│                                                  │
│    Flux reconciles:                              │
│    - Pulls OCI artifacts                         │
│    - Deploys components in dependency order      │
│    - Waits for readiness before next component   │
└──────────────────────────────────────────────────┘
```

## Generated Resources

For each enabled component in `cluster.yaml`, this bootstrap creates:

### OCIRepository

Points to the component's versioned OCI artifact:

```yaml
apiVersion: source.toolkit.fluxcd.io/v1
kind: OCIRepository
metadata:
  name: capsule
  namespace: flux-system
spec:
  interval: 5m
  url: oci://ghcr.io/fedcore/capsule-fedcore-prod-use1
  ref:
    semver: "1.0.0"
```

### Kustomization

Tells Flux how to deploy the component:

```yaml
apiVersion: kustomize.toolkit.fluxcd.io/v2
kind: Kustomization
metadata:
  name: capsule
  namespace: flux-system
spec:
  interval: 10m
  sourceRef:
    kind: OCIRepository
    name: capsule
  path: ./
  prune: true
  wait: true
```

### Dependency Management

Components can depend on others using `dependsOn`:

```yaml
apiVersion: kustomize.toolkit.fluxcd.io/v2
kind: Kustomization
metadata:
  name: cloud-permissions
spec:
  dependsOn:
    - name: ack-iam-controller  # Must deploy IAM controller first
      namespace: flux-system
  # ... rest of spec
```

## Cluster Configuration

Components are enabled in `cluster.yaml`:

```yaml
# cluster.yaml
cluster_name: fedcore-prod-use1
cloud: aws
region: us-east-1

components:
  - name: capsule
    enabled: true
    version: "1.0.0"

  - name: ack-iam-controller
    enabled: true
    version: "1.2.0"

  - name: cloud-permissions
    enabled: true
    version: "1.0.0"
    depends_on:
      - ack-iam-controller  # Requires ACK IAM controller

  - name: kro
    enabled: true
    version: "0.1.0"

  - name: kyverno-policies
    enabled: true
    version: "1.1.0"

rgds:
  - name: tenant
    enabled: true
    version: "1.0.0"
```

## Bootstrap Process

### 1. Build Component Artifacts

Build and push each component as an OCI artifact:

```bash
# Build all components for a cluster
fedcore build --all --cluster fedcore-prod-use1

# Or build and push all components to OCI registry
fedcore build --all --cluster fedcore-prod-use1 --push
```

### 2. Generate Bootstrap Configuration

Generate Flux resources to deploy components:

```bash
fedcore bootstrap --cluster fedcore-prod-use1 > bootstrap.yaml
```

This processes [component-sources.yaml](component-sources.yaml:1) with cluster configuration to generate OCIRepository and Kustomization resources for each enabled component.

### 3. Apply Bootstrap to Cluster

```bash
# Ensure Flux is installed
flux check

# Apply bootstrap configuration
kubectl apply -f bootstrap.yaml

# Watch components deploy
watch kubectl get kustomizations -n flux-system
```

## Dependency Order

Components are deployed in the order determined by `depends_on` relationships:

```
1. capsule (no dependencies)
2. ack-iam-controller (no dependencies)
3. cloud-permissions (depends on ack-iam-controller)
4. kro (no dependencies)
5. kyverno-policies (no dependencies)
```

Within each level, components deploy in parallel. Flux waits for readiness before proceeding.

## Component Versioning

### Semantic Versioning

Components use semantic versioning:

```yaml
ref:
  semver: "1.0.0"      # Exact version
  semver: "~1.0"       # Latest 1.0.x
  semver: "^1.0"       # Latest 1.x (compatible)
```

### Version Updates

To update a component version:

1. Build new version:
   ```bash
   fedcore build --artifact platform/components/capsule --cluster fedcore-prod-use1 --push
   ```

2. Update `cluster.yaml`:
   ```yaml
   components:
     - name: capsule
       version: "1.1.0"  # Updated
   ```

3. Regenerate bootstrap:
   ```bash
   fedcore bootstrap --cluster fedcore-prod-use1 > bootstrap.yaml
   kubectl apply -f bootstrap.yaml
   ```

Flux automatically pulls the new version and reconciles.

## How It Works

The [component-sources.yaml](component-sources.yaml:1) template:

1. **Loops through enabled components** from `cluster.yaml`
2. **Skips itself** (avoids circular dependency)
3. **Generates OCIRepository** pointing to component artifact
4. **Generates Kustomization** with deployment configuration
5. **Adds dependencies** if specified

```yaml
#@ for component in data.values.components:
#@   if component.enabled and component.name != "component-sources":
---
apiVersion: source.toolkit.fluxcd.io/v1
kind: OCIRepository
# ... references component artifact
---
apiVersion: kustomize.toolkit.fluxcd.io/v2
kind: Kustomization
# ... deploys component
#@ if hasattr(component, "depends_on"):
  dependsOn:
    # ... component dependencies
#@ end
#@   end
#@ end
```

## Benefits of This Approach

### Versioned Artifacts
- Components are immutable OCI artifacts
- Each cluster can run different component versions
- Easy rollback by changing version numbers

### Declarative Dependencies
- Dependencies are explicit in configuration
- Flux enforces deployment order
- No manual sequencing required

### GitOps-Native
- All configuration is version controlled
- Changes go through code review
- Audit trail for all updates

### Cloud-Agnostic Bootstrap
- Same bootstrap mechanism for all clouds
- Cloud-specific logic is in component overlays
- Bootstrap itself doesn't need cloud knowledge

## Troubleshooting

### Component Fails to Deploy

**Symptom:** Kustomization shows `Ready: False`

**Check:**
1. Kustomization status:
   ```bash
   kubectl describe kustomization <component-name> -n flux-system
   ```

2. Common issues:
   - OCI artifact not found (check URL and version)
   - Dependency not ready (check `dependsOn`)
   - Invalid manifests in artifact

### OCI Artifact Not Found

**Symptom:** `failed to pull artifact: not found`

**Solution:**
1. Verify artifact exists:
   ```bash
   oras discover ghcr.io/fedcore/capsule-fedcore-prod-use1:1.0.0
   ```

2. Check registry authentication:
   ```bash
   flux reconcile source oci <component-name> -n flux-system
   ```

3. Verify version exists in registry

### Dependency Deadlock

**Symptom:** Multiple components stuck waiting for each other

**Cause:** Circular dependencies in `depends_on`

**Solution:**
1. Review dependency graph:
   ```bash
   kubectl get kustomizations -n flux-system -o yaml | grep -A 5 dependsOn
   ```

2. Remove circular dependencies in `cluster.yaml`

3. Regenerate bootstrap configuration

### Bootstrap Configuration Out of Sync

**Symptom:** Components enabled in cluster but not deploying

**Solution:**
1. Regenerate bootstrap:
   ```bash
   fedcore bootstrap --cluster fedcore-prod-use1 > bootstrap.yaml
   ```

2. Compare with applied configuration:
   ```bash
   kubectl get kustomizations -n flux-system
   ```

3. Re-apply if different:
   ```bash
   kubectl apply -f bootstrap.yaml
   ```

## Adding New Components

To add a new infrastructure component:

1. **Create component directory:**
   ```bash
   mkdir -p platform/components/my-component/base
   ```

2. **Add component manifest:**
   ```yaml
   # platform/components/my-component/base/my-component.yaml
   apiVersion: v1
   kind: Namespace
   metadata:
     name: my-component-system
   ```

3. **Build and push artifact:**
   ```bash
   fedcore build --artifact platform/components/my-component --cluster fedcore-prod-use1 --push
   ```

4. **Enable in cluster.yaml:**
   ```yaml
   components:
     - name: my-component
       enabled: true
       version: "1.0.0"
       depends_on:
         - capsule  # If depends on other components
   ```

5. **Regenerate bootstrap:**
   ```bash
   fedcore bootstrap --cluster fedcore-prod-use1 > bootstrap.yaml
   kubectl apply -f bootstrap.yaml
   ```

## Security Considerations

### OCI Registry Access

Bootstrap requires read access to OCI registry:
- Use Flux image pull secrets for private registries
- Implement image scanning in CI/CD
- Use signed artifacts with cosign for verification

### Component Isolation

Each component should:
- Run in dedicated namespace
- Use least-privilege service accounts
- Have resource quotas and limits
- Be isolated by network policies

### Version Pinning

For production:
- Pin exact versions (`1.0.0` not `~1.0`)
- Test version updates in dev/staging first
- Use GitOps for version changes (no manual kubectl)

## Related Documentation

- [Infrastructure Components Overview](../../components/README.md)
- [Cluster Configuration Reference](../../clusters/README.md)
- [Flux OCIRepository Documentation](https://fluxcd.io/flux/components/source/ocirepositories/)
- [Flux Kustomization Documentation](https://fluxcd.io/flux/components/kustomize/kustomizations/)

---

**Status:** ✅ Production ready
