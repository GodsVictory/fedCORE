# kro - Kubernetes Resource Orchestrator

This directory contains the kro operator installation and RBAC configuration for the cluster.

## What is kro?

kro is a Kubernetes operator that enables creation of custom resource abstractions (ResourceGraphDefinitions or RGDs) that orchestrate multiple underlying Kubernetes resources. It's comparable to Helm or Crossplane in that it allows platform teams to define reusable patterns, but uses native Kubernetes CRDs and CEL expressions instead of templating.

**Official Documentation**: https://kro.run

## Architecture

```
┌─────────────────────────────────────────────┐
│  ResourceGraphDefinition (RGD)              │
│  Defines: Custom Resource Type + Resources │
└──────────────┬──────────────────────────────┘
               │ Creates
               ▼
┌─────────────────────────────────────────────┐
│  CustomResourceDefinition (CRD)             │
│  + Dynamic Controller                       │
└──────────────┬──────────────────────────────┘
               │ Watches
               ▼
┌─────────────────────────────────────────────┐
│  Custom Resource Instance                   │
│  (e.g., NamespaceProvisioning)              │
└──────────────┬──────────────────────────────┘
               │ Reconciles to
               ▼
┌─────────────────────────────────────────────┐
│  Kubernetes Resources                       │
│  (Namespaces, ServiceAccounts, RBAC, etc.) │
└─────────────────────────────────────────────┘
```

## Components

### Base Installation
- **install.yaml**: Core kro operator (v0.8.5)
  - Namespace: `kro-system`
  - ServiceAccount: `kro`
  - Base ClusterRole with aggregation
  - Deployment with security hardening

### RBAC Configuration

#### core-resources-rbac.yaml
Grants kro access to common Kubernetes resources that RGDs typically manage:
- Core: namespaces, serviceaccounts, secrets, services
- Workloads: deployments, statefulsets, daemonsets, jobs
- Networking: ingresses, networkpolicies
- Autoscaling: horizontalpodautoscalers
- RBAC: rolebindings, clusterrolebindings, roles, clusterroles

#### platform-fedcore-rbac.yaml
Grants kro wildcard access to all custom resources in the `platform.fedcore.io` API group:
```yaml
- apiGroups: ["platform.fedcore.io"]
  resources: ["*"]
  verbs: ["create", "delete", "get", "list", "patch", "update", "watch"]
```

#### default-clusterroles-rbac.yaml
Grants kro permission to bind the standard Kubernetes ClusterRoles in RoleBindings:
```yaml
- apiGroups: ["rbac.authorization.k8s.io"]
  resources: ["clusterroles"]
  resourceNames: ["admin", "edit", "view"]
  verbs: ["bind"]
```

This is required for the NamespaceProvisioning RGD to create RoleBindings that grant these roles.

## Security Model

### ⚠️ Critical: kro Operates with Near Cluster-Admin Permissions

The kro service account has broad permissions comparable to cluster-admin. This is **by design** - kro needs these permissions to create arbitrary resources on behalf of RGDs.

### Threat Model

#### 🔴 CRITICAL: ResourceGraphDefinition Creation = Cluster-Admin Access

**Who can create RGDs:**
- ✅ GitOps controllers (FluxCD) - trusted, audited, version-controlled
- ✅ Platform administrators with cluster-admin
- ❌ **NEVER regular users or application service accounts**

**Why RGD creation is dangerous:**

Creating an RGD allows arbitrary resource creation with kro's full permissions:

```yaml
# Malicious RGD example - DO NOT CREATE
apiVersion: kro.run/v1alpha1
kind: ResourceGraphDefinition
metadata:
  name: privilege-escalation
spec:
  schema:
    apiVersion: v1alpha1
    group: evil.io
    kind: SecretStealer
  resources:
    - id: steal-all-secrets
      template:
        # Can read any secret in any namespace
        # Can create cluster-admin ClusterRoleBindings
        # Can modify CustomResourceDefinitions
        # Essentially full cluster compromise
```

**RGDs are cluster-scoped** - all kro instances see all RGDs, making multi-tenant isolation difficult.

#### 🟢 LOW SECURITY RISK: Custom Resource Instance Creation

Creating instances of custom resources defined by RGDs is **safe** from a security perspective because:
1. The resource schema is pre-defined and validated by OpenAPI
2. Actions are fully constrained by the RGD template (users can't escape it)
3. Resources created, their types, and locations are controlled by the RGD, not the user
4. Users only provide data for pre-defined fields

**Example: NamespaceProvisioning**
- ✅ **Safe to delegate to users** with proper policies
- Users create namespaces and only get admin access to those new namespaces
- Cannot escalate beyond namespace-level permissions
- Cannot access existing namespaces they don't own
- Cannot inject arbitrary resources or modify the RGD template

**The real risks are operational, not security-related:**
- Resource exhaustion (quota management needed)
- Namespace naming conflicts (policy enforcement needed)
- Cost/billing implications (budget limits needed)
- Management overhead (lifecycle policies needed)

**Security boundaries:**
```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: NamespaceProvisioning
spec:
  namespaceName: my-team-dev  # ← Creates NEW namespace
  roleBindings:
    - serviceAccount:
        name: my-sa
        namespace: my-existing-ns
      role: admin
```

Result:
- ✅ RoleBinding created **in** `my-team-dev` (the new namespace)
- ❌ Cannot grant access to `my-existing-ns` or other namespaces
- ✅ Appropriate for self-service namespace provisioning

### RBAC Privilege Escalation Prevention

Kubernetes prevents privilege escalation by default. The kro service account can only create RoleBindings that grant permissions if:
1. kro already has those permissions, OR
2. kro has explicit `bind` permission for those ClusterRoles

We grant `bind` permission for `admin`, `edit`, and `view` only (scoped with `resourceNames`), which allows RGDs to create RoleBindings for these standard roles without requiring kro to have every single permission in the `admin` role.

### Summary: Risk Levels

| Action | Who | Security Risk | Why |
|--------|-----|---------------|-----|
| **Create RGD** | Admins/GitOps only | 🔴 **CRITICAL** | Equivalent to cluster-admin - can create any resource type with kro's full permissions |
| **Create custom resource instance** | ✅ Users OK (with policies) | 🟢 **LOW** | Constrained by RGD schema - cannot escape template or escalate privileges |
| **Grant kro more RBAC** | Admins only | 🔴 **CRITICAL** | Increases blast radius of compromised RGDs |
| **Bind admin/edit/view roles** | kro (automated) | 🟢 **LOW** | Scoped to new namespaces only, cannot affect existing resources |

**Bottom line**: Keep RGD creation tightly controlled (GitOps only), but you can safely allow users to create instances of well-designed custom resources.

## Recommendations

### ✅ DO

1. **Keep RGD creation restricted to GitOps and platform admins**
   ```bash
   # Verify only trusted principals can create RGDs
   kubectl auth can-i create resourcegraphdefinitions.kro.run --as=system:serviceaccount:default:random-user
   # Should return "no"
   ```

2. **Allow users to create custom resource instances with policy enforcement**
   - Use Kyverno or OPA to validate custom resource specs
   - Enforce naming conventions
   - Prevent reserved namespace names
   - Apply resource quotas

3. **Audit and monitor kro's actions**
   - Enable audit logging for RGD creation/modification
   - Alert on unexpected RoleBinding creation
   - Monitor namespace creation patterns

4. **Review RGDs before deployment**
   - Treat RGDs like code - require peer review
   - Test in dev/staging before production
   - Document the resources each RGD creates

5. **Use labels for resource attribution**
   ```yaml
   labels:
     platform.fedcore.io/managed-by: kro
     platform.fedcore.io/rgd-name: namespace
   ```

### ❌ DON'T

1. **Never grant users permission to create RGDs**
   ```yaml
   # ❌ NEVER DO THIS
   apiVersion: rbac.authorization.k8s.io/v1
   kind: ClusterRole
   metadata:
     name: user-rgd-creator  # ← Equivalent to cluster-admin!
   rules:
   - apiGroups: ["kro.run"]
     resources: ["resourcegraphdefinitions"]
     verbs: ["create"]
   ```

2. **Don't run multiple kro instances without proper isolation**
   - RGDs are cluster-scoped - all kro instances see all RGDs
   - Multi-tenant kro requires label-based filtering (not currently supported)

3. **Don't skip policy enforcement for custom resources**
   - Even safe-looking custom resources can have unintended consequences
   - Always validate user-provided specs

4. **Don't grant kro more permissions than necessary**
   - Review and prune unused permissions periodically
   - Use `resourceNames` restrictions where possible

## Incident Response

### If an unauthorized RGD is detected:

1. **Immediately delete the RGD**
   ```bash
   kubectl delete resourcegraphdefinition <malicious-rgd>
   ```

2. **Check for created resources**
   ```bash
   kubectl get all -A -l kro.run/resource-graph-definition-name=<malicious-rgd>
   ```

3. **Audit who created it**
   ```bash
   kubectl get resourcegraphdefinition <malicious-rgd> -o yaml | grep -A5 'annotations:'
   ```

4. **Review RBAC**
   ```bash
   kubectl auth can-i create resourcegraphdefinitions.kro.run --as=<suspected-user>
   ```

5. **Check kro logs**
   ```bash
   kubectl logs -n kro-system deploy/kro --tail=1000 | grep <malicious-rgd>
   ```

## Comparison to Other Tools

| Tool | Permissions Model | Multi-Tenancy | Risk Level |
|------|-------------------|---------------|------------|
| **kro** | Cluster-admin (needs access to all resource types) | Limited (cluster-scoped RGDs) | High if RGD creation is open |
| **Helm** | Per-release (uses user's kubeconfig) | Good (namespace-scoped releases) | Low (user brings own auth) |
| **Crossplane** | Cluster-admin (manages cloud resources) | Good (namespace-scoped claims) | High if Compositions are user-created |
| **FluxCD** | Cluster-admin (GitOps reconciliation) | Limited (trust git repos) | High if repo write access is open |

kro is similar to FluxCD in trust model: both need broad permissions, both should be limited to trusted automation.

## Contributing RGDs

When creating new RGDs:

1. **Principle of least privilege**: Only create resources that are necessary
2. **Validation**: Use CEL expressions to validate user inputs
3. **Documentation**: Document what resources are created and why
4. **Labels**: Apply consistent labels for tracking
5. **Examples**: Provide example instances showing safe usage
6. **Security review**: Get platform team approval before deploying

## References

- [kro Official Documentation](https://kro.run)
- [Kubernetes RBAC Documentation](https://kubernetes.io/docs/reference/access-authn-authz/rbac/)
- [CEL Expression Language](https://kubernetes.io/docs/reference/using-api/cel/)
- [Platform RGDs](../../rgds/README.md)

## Troubleshooting

### kro controller has permission errors

Check if RBAC ClusterRoles are aggregating properly:
```bash
kubectl get clusterrole kro:controller -o jsonpath='{.rules}' | jq
```

Restart kro after RBAC changes:
```bash
kubectl rollout restart deployment/kro -n kro-system
```

### Custom resource instances stuck in "Reconciling"

Check kro logs:
```bash
kubectl logs -n kro-system deploy/kro --tail=100
```

Check RGD status:
```bash
kubectl describe resourcegraphdefinition <name>
```

### Permission denied when creating RoleBindings

Ensure the `default-clusterroles-rbac.yaml` is applied and kro has been restarted.
