# Namespace RGD

KRO ResourceGraphDefinition for provisioning Kubernetes namespaces with optional service accounts and RBAC bindings.

## Overview

The Namespace RGD simplifies namespace creation with pre-configured access controls. It supports:
- Creating namespaces with custom labels and annotations
- Optional admin service account creation
- Binding external service accounts to standard Kubernetes roles (admin, edit, view)

## Use Cases

1. **CI/CD Namespace Creation**: Create namespaces with deployer service accounts
2. **Shared Service Namespaces**: Grant multiple teams different access levels
3. **Environment Provisioning**: Quickly provision dev/staging/prod namespaces
4. **Multi-Team Access**: Configure role-based access for different service accounts

## Schema

```yaml
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: <instance-name>
spec:
  # Required: Name of the namespace to create
  namespaceName: string

  # Optional: Metadata for the namespace (labels and annotations)
  metadata:
    environment: string     # e.g., dev, staging, prod
    team: string           # e.g., platform, app-team
    costCenter: string     # e.g., CC-12345
    description: string    # Description annotation
    owner: string          # Owner/contact annotation

  # Optional: Create a service account with admin access (default: false)
  createServiceAccount: boolean

  # Optional: Name of service account to create (default: "admin")
  # Only used if createServiceAccount is true
  serviceAccountName: string

  # Optional: List of role bindings for external service accounts
  roleBindings:
    - serviceAccount:
        name: string         # Service account name
        namespace: string    # Service account namespace
      role: string          # One of: admin, edit, view
```

## Resources Created

1. **Namespace**: The target namespace with custom labels/annotations
2. **ServiceAccount** (conditional): Created only if `createServiceAccount: true`
3. **RoleBinding** (conditional): Binds the created service account to admin role
4. **RoleBindings** (multiple): One per entry in `roleBindings` array

## Kubernetes Default Roles

The RGD uses Kubernetes built-in ClusterRoles:

- **admin**: Full access (read/write/delete all resources, manage RBAC)
- **edit**: Read/write access (cannot modify RBAC or ResourceQuotas)
- **view**: Read-only access (cannot see Secrets)

See [Kubernetes RBAC documentation](https://kubernetes.io/docs/reference/access-authn-authz/rbac/#user-facing-roles) for details.

## Examples

### Example 1: Simple Namespace

Create an empty namespace with custom metadata:

```yaml
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: my-app-dev
spec:
  namespaceName: my-app-dev
  metadata:
    environment: dev
    team: platform
    description: "Development namespace for my-app"
```

**Creates:**
- Namespace: `my-app-dev`

### Example 2: Namespace with Admin Service Account

Create a namespace with a CI/CD deployer service account:

```yaml
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: my-app-staging
spec:
  namespaceName: my-app-staging
  createServiceAccount: true
  serviceAccountName: deployer
```

**Creates:**
- Namespace: `my-app-staging`
- ServiceAccount: `deployer` in `my-app-staging`
- RoleBinding: `deployer` → `admin` role

### Example 3: Shared Namespace with Multi-Team Access

Create a namespace with different access levels for multiple teams:

```yaml
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: shared-services
spec:
  namespaceName: shared-services
  metadata:
    environment: prod
    team: platform
    description: "Shared services namespace with multi-team access"
  roleBindings:
    # Platform team: admin access
    - serviceAccount:
        name: platform-deployer
        namespace: platform-cicd
      role: admin

    # App team: edit access
    - serviceAccount:
        name: app-deployer
        namespace: app-cicd
      role: edit

    # Monitoring: view access
    - serviceAccount:
        name: prometheus
        namespace: monitoring
      role: view
```

**Creates:**
- Namespace: `shared-services`
- RoleBinding: `platform-deployer` → `admin` role
- RoleBinding: `app-deployer` → `edit` role
- RoleBinding: `prometheus` → `view` role

### Example 4: Complete Configuration

Combine service account creation with external role bindings:

```yaml
apiVersion: platform.fedcore.io/v1
kind: NamespaceProvisioning
metadata:
  name: my-app-prod
spec:
  namespaceName: my-app-prod
  metadata:
    environment: prod
    team: my-app
    costCenter: CC-12345
    description: "Production namespace for my-app"
    owner: "my-app-team@example.com"

  # Create CI/CD service account
  createServiceAccount: true
  serviceAccountName: cicd-deployer

  # Grant access to other service accounts
  roleBindings:
    - serviceAccount:
        name: sre-admin
        namespace: platform-admin
      role: admin
    - serviceAccount:
        name: prometheus
        namespace: monitoring
      role: view
```

**Creates:**
- Namespace: `my-app-prod`
- ServiceAccount: `cicd-deployer` in `my-app-prod`
- RoleBinding: `cicd-deployer` → `admin` role (in `my-app-prod`)
- RoleBinding: `sre-admin` → `admin` role (in `my-app-prod`)
- RoleBinding: `prometheus` → `view` role (in `my-app-prod`)

## Deployment

### Deploy the RGD to Cluster

```bash
kubectl apply -f platform/rgds/namespace/base/namespace-rgd.yaml
```

### Create a Namespace Instance

```bash
kubectl apply -f platform/rgds/namespace/examples/simple-namespace.yaml
```

### Verify Resources Created

```bash
# Check the NamespaceProvisioning instance
kubectl get namespaceprovisionings
kubectl describe namespaceprovisionings my-app-dev

# Check created resources
kubectl get namespace my-app-dev
kubectl get serviceaccount -n my-app-dev
kubectl get rolebindings -n my-app-dev
```

### Check Status

The RGD populates status fields when resources are created:

```bash
kubectl get namespaceprovisionings my-app-dev -o yaml
```

## Comparison with Tenant RGD

| Feature | Namespace RGD | Tenant RGD |
|---------|--------------|------------|
| **Scope** | Single namespace | Multiple namespaces (Capsule Tenant) |
| **Multi-tenancy** | No | Yes (Capsule isolation) |
| **Resource Quotas** | No | Yes (tenant-level + per-namespace) |
| **Network Policies** | No | Yes (default deny + allow rules) |
| **Service Account** | Optional, admin role | Always created with Pod Identity/Workload Identity |
| **Cloud IAM** | No | Yes (AWS IAM roles, Azure Managed Identity) |
| **Use Case** | Simple namespace + RBAC | Full tenant isolation with quotas |

**When to use Namespace RGD:**
- Simple namespace provisioning
- Internal platform namespaces
- Shared service namespaces
- When you don't need Capsule tenant isolation

**When to use Tenant RGD:**
- Multi-tenant environments
- Need resource quotas and isolation
- Need cloud IAM integration (AWS/Azure)
- Need network policies and service mesh

## Advanced Usage

### Using in CI/CD Pipelines

Create namespaces dynamically in CI/CD:

```yaml
# .github/workflows/deploy.yaml
- name: Create namespace
  run: |
    cat <<EOF | kubectl apply -f -
    apiVersion: platform.fedcore.io/v1
    kind: NamespaceProvisioning
    metadata:
      name: ${{ github.event.repository.name }}-${{ github.ref_name }}
    spec:
      namespaceName: ${{ github.event.repository.name }}-${{ github.ref_name }}
      createServiceAccount: true
      serviceAccountName: deployer
    EOF
```

### Using with GitOps

Store NamespaceProvisioning instances in Git for GitOps workflows:

```bash
# platform/clusters/{cluster}/namespaces/
├── kustomization.yaml
├── app1-dev.yaml
├── app1-staging.yaml
├── app1-prod.yaml
└── shared-services.yaml
```

Flux/ArgoCD automatically reconciles namespace changes.

## Troubleshooting

### RoleBinding fails: ServiceAccount not found

**Problem:** RoleBinding references a service account that doesn't exist yet.

**Solution:** Ensure the referenced service account exists before creating the NamespaceProvisioning:

```bash
# Check if service account exists
kubectl get serviceaccount platform-deployer -n platform-cicd

# Or create it first
kubectl create serviceaccount platform-deployer -n platform-cicd
```

### Multiple RoleBindings to same ServiceAccount

**Problem:** Can I bind the same service account to multiple roles?

**Answer:** Yes, but it's redundant. The most permissive role wins. If a service account has both `edit` and `admin` roles, `admin` permissions apply.

### Deleting a NamespaceProvisioning

When you delete a NamespaceProvisioning instance, KRO deletes all created resources:

```bash
kubectl delete namespaceprovisionings my-app-dev
```

This deletes:
- The namespace (and ALL resources inside it)
- Service accounts
- Role bindings

⚠️ **Warning:** Deleting the namespace deletes all workloads, configs, and data inside it!

## Security Considerations

1. **Service Account Permissions**: The `admin` role grants full access to the namespace. Use `edit` or `view` for less privileged access.

2. **Cross-Namespace Access**: Role bindings grant service accounts from other namespaces access to this namespace. Ensure you trust the source namespace.

3. **No Resource Quotas**: This RGD doesn't enforce resource quotas. For quota enforcement, use the Tenant RGD instead.

4. **No Network Policies**: This RGD doesn't create network policies. Add them manually or use the Tenant RGD for automatic network isolation.

## See Also

- [Tenant RGD](../tenant/README.md) - Full multi-tenant namespace provisioning
- [KRO Documentation](https://kro.run/docs/)
- [Kubernetes RBAC](https://kubernetes.io/docs/reference/access-authn-authz/rbac/)
