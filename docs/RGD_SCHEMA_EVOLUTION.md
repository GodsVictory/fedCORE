# RGD Schema Evolution

**How to safely change RGD schemas without breaking existing instances**

When you build Resource Graph Definitions (RGDs) for your platform, schemas will evolve over time. This guide shows you how to migrate schemas safely without disrupting teams using your RGDs.

---

## Quick Decision

- **Adding optional fields?** → Just deploy it. No migration needed.
- **Renaming fields or small changes?** → Use backward-compatible approach
- **Removing fields or type changes?** → Use versioned approach

---

## Approach 1: Backward-Compatible (Preferred for Minor Changes)

Keep old fields as optional, support both patterns with CEL.

**Example: Supporting both `spec.namespaceName` and `metadata.name`**

```yaml
apiVersion: kro.run/v1alpha1
kind: ResourceGraphDefinition
metadata:
  name: namespace.platform.fedcore.io
spec:
  schema:
    apiVersion: v1alpha1
    kind: NamespaceProvisioning
    spec:
      # DEPRECATED: Use metadata.name instead (remove in v1)
      namespaceName: string | optional
      createServiceAccount: boolean | default=true

  resources:
    - id: targetNamespace
      template:
        metadata:
          # Support both old and new patterns
          name: ${has(schema.spec.namespaceName) && schema.spec.namespaceName != "" ? schema.spec.namespaceName : schema.metadata.name}
```

**Both patterns work:**

```yaml
# Old pattern (deprecated but works)
metadata:
  name: my-app
spec:
  namespaceName: my-app-prod

# New pattern (recommended)
metadata:
  name: my-app-prod
spec:
  # namespaceName omitted
```

**Cleanup (after 6-12 months):**
1. Verify no instances use deprecated fields
2. Remove optional fields from schema
3. Remove CEL conditionals
4. Deploy with `force: true`

---

## Approach 2: Side-by-Side Versions (For Breaking Changes)

Deploy two RGDs simultaneously with different versions.

**Deploy both versions:**

```yaml
# platform/rgds/namespace-v1alpha1/base/rgd.yaml
metadata:
  name: namespace-v1alpha1.platform.fedcore.io
  annotations:
    deprecated: "true"
spec:
  schema:
    apiVersion: v1alpha1
    kind: NamespaceProvisioning
    spec:
      namespaceName: string  # Old required field

# platform/rgds/namespace/base/rgd.yaml
metadata:
  name: namespace.platform.fedcore.io
spec:
  schema:
    apiVersion: v1
    kind: NamespaceProvisioning
    # Uses metadata.name, namespaceName removed
```

**Enable both in cluster config:**

```yaml
components:
  - name: namespace-v1alpha1
    enabled: true
  - name: namespace
    enabled: true
    force: true
```

**Migration script:**

```bash
#!/bin/bash
# Migrate v1alpha1 → v1

kubectl get namespaceprovisionings.platform.fedcore.io/v1alpha1 -A -o json | \
  jq -c '.items[]' | while read -r instance; do
    NAME=$(echo "$instance" | jq -r '.metadata.name')
    NS=$(echo "$instance" | jq -r '.metadata.namespace')
    NS_NAME=$(echo "$instance" | jq -r '.spec.namespaceName')

    # Create v1 instance
    kubectl apply -f - <<EOF
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: $NS_NAME
  namespace: $NS
spec:
  createServiceAccount: $(echo "$instance" | jq -r '.spec.createServiceAccount')
EOF

    # Delete v1alpha1 instance
    kubectl delete namespaceprovisionings.platform.fedcore.io/v1alpha1/$NAME -n $NS
done
```

**Cleanup (after 6-12 months):**

```yaml
components:
  - name: namespace-v1alpha1
    enabled: false  # Remove deprecated version
```

---

## Troubleshooting

**RGD stuck in "Inactive" state:**
```bash
kubectl delete rgd namespace.platform.fedcore.io
flux reconcile kustomization namespace --with-source
```

**CRD won't update (breaking changes detected):**
```yaml
# Add force flag to component
components:
  - name: namespace
    force: true  # Allows CRD deletion/recreation
```

**Check which instances use deprecated fields:**
```bash
# For side-by-side approach
kubectl get namespaceprovisionings.platform.fedcore.io/v1alpha1 -A

# For backward-compatible approach (add labels to track usage)
kubectl get namespaceprovisionings -A -l 'uses-deprecated-fields=true'
```

---

## Timeline Recommendations

| Change Type | Approach | Notice Period |
|-------------|----------|---------------|
| Add optional field | None needed | 0 months |
| Rename field | Backward-compatible | 6-12 months |
| Remove required field | Side-by-side versions | 6-12 months |
| Type change | Side-by-side versions | 6-12 months |
| Major redesign | Side-by-side versions | 12+ months |

---

## Key Points

- ✅ Always test in dev/staging first
- ✅ Provide migration scripts, not just documentation
- ✅ Give teams 6+ months notice for breaking changes
- ✅ Back up instances before CRD changes: `kubectl get <resource> -A -o yaml > backup.yaml`
- ✅ Use `force: true` carefully - it deletes and recreates RGDs/CRDs
- ⚠️ Deleting CRDs deletes all instances (finalizers clean up managed resources)

---

## Navigation

[← Previous: Development](DEVELOPMENT.md) | [Next: CI/CD Role Zero Permissions →](CICD_ROLE_ZERO_PERMISSIONS.md)

**Handbook Progress:** Page 18 of 35 | **Level 4:** Deployment & Development

[📖 Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
