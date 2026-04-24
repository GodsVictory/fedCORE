# Tenant Onboarding with KRO

Automated tenant onboarding with CI/CD infrastructure, ServiceAccounts, and Cloud IAM.

## Overview

This KRO ResourceGraphDefinition creates everything needed for tenant onboarding:

- ✅ Capsule Tenant (namespace isolation + quotas)
- ✅ CI/CD namespace (`<tenant>-cicd`)
- ✅ ServiceAccount with Pod Identity/Workload Identity annotations
- ✅ Two-tier AWS IAM Roles (cluster + tenant) or Azure Managed Identity (based on cluster)
- ✅ Pod Identity Association (AWS) or Federated Identity (Azure)
- ✅ RBAC for automated deployments

**Time savings:** Manual onboarding (30-60 min) → This approach (5 min)

---

## Architecture

This RGD is built **per-cluster** (Tier 2) with full access to cluster-specific configuration like AWS account IDs and cluster regions.

```
Structure:
  platform/rgds/tenant/
    ├── base/tenant-rgd.yaml        (cloud-agnostic common resources)
    └── overlays/
        ├── aws/overlay.yaml        (Pod Identity + Two-tier IAM Roles via ACK)
        └── azure/overlay.yaml      (Workload Identity + Managed Identity via ASO)

Build Process:
  fedcore build --artifact platform/rgds/tenant --cluster platform/clusters/fedcore-prod-use1
      ↓ ytt processes base + cloud-specific overlay
  RGD artifact: tenant-fedcore-prod-use1.yaml
      ↓ packaged as OCI artifact
  oci://ghcr.io/fedcore/tenant-fedcore-prod-use1:1.0.0
      ↓ deployed to cluster via Flux OCIRepository
  KRO reconciles TenantOnboarding CRs
```

**AWS clusters** get: Common resources + Pod Identity annotation + Two-tier IAM Roles via ACK (with Permission Boundary)
**Azure clusters** get: Common resources + Workload Identity annotations + Managed Identity via ASO

**Key difference from Tier 1:** RGDs are now built per-cluster (not per-cloud), allowing cluster-specific customization while maintaining the benefits of OCI packaging and version management.

**Security:** AWS IAM roles are automatically created with permission boundaries to prevent privilege escalation. The boundary ARN is auto-derived as `arn:aws:iam::{account_id}:policy/{cluster_name}-TenantMaxPermissions` and references the policy created by `platform/components/cloud-permissions`.

---

## Deployment

### 1. Build the RGD Artifact

Build the RGD for your specific cluster:

```bash
fedcore build --artifact platform/rgds/tenant --cluster platform/clusters/fedcore-prod-use1 \
  > build/tenant-fedcore-prod-use1.yaml
```

The build script:
- Loads cluster schema for validation
- Loads cluster config for cluster-specific values (AWS account ID, region, etc.)
- Processes ytt templates with full cluster context
- Outputs a ready-to-deploy RGD

### 2. Package as OCI Artifact

```bash
# Package and push to registry (in your CI/CD)
oras push ghcr.io/fedcore/tenant-fedcore-prod-use1:1.0.0 \
  build/tenant-fedcore-prod-use1.yaml:application/vnd.cncf.kro.rgd.v1+yaml
```

### 3. Enable in Cluster Config

Add to your `cluster.yaml`:

```yaml
rgds:
- name: tenant
  enabled: true
  version: "1.0.0"
```

The infrastructure build will generate Flux OCIRepository and Kustomization resources that reference the cluster-specific artifact.

### Prerequisites

Ensure your `cluster.yaml` has cloud settings (these are used during RGD build):

**AWS clusters:**
```yaml
cloud: aws
region: "us-east-1"
aws:
  account_id: "123456789012"
  # Note: Permission boundary ARN is auto-derived as:
  # arn:aws:iam::{account_id}:policy/{cluster_name}-TenantMaxPermissions
  # Pod Identity is used - no OIDC provider configuration needed
```

**Azure clusters:**
```yaml
cloud: azure
azure:
  region: "eastus"
  resource_group: "my-rg"
  tenant_id: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
  oidc_issuer: "https://eastus.oic.prod-aks.azure.com/..."
```

---

## Usage

### Create a Tenant

Create a TenantOnboarding CR in your cluster's tenants directory:

```bash
# Create tenant file
cat > platform/clusters/fedcore-prod-use1/config/acme-onboarding.yaml <<EOF
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme
  owners:
    - kind: User
      name: admin@acme.com
  quotas:
    namespaces: 10
    cpu: "100"
    memory: "200Gi"
    storage: "1Ti"
    maxPVCs: 50
  billing:
    costCenter: "ACME-ENG-001"
    contact: "billing@acme.com"
  settings:
    allowLoadBalancer: false
    allowInternetEgress: true
    #! Istio service mesh (optional)
    istio:
      enabled: true           # Enable automatic sidecar injection
      strictMTLS: true        # Enforce STRICT mTLS (no plaintext allowed)
EOF

# Commit and push (GitOps)
git add platform/clusters/fedcore-prod-use1/config/acme-onboarding.yaml
git commit -m "Onboard tenant: acme"
git push
```

### Monitor

```bash
kubectl get tenantonboarding acme -w
kubectl get tenant acme
kubectl get namespace acme-cicd
kubectl get serviceaccount -n acme-cicd acme-deployer
```

---

## What Gets Created

For each TenantOnboarding CR, KRO creates:

### Common Resources (All Clouds)
1. **Capsule Tenant** - Multi-tenant namespace isolation with quotas
   - Automatic Istio injection label added to all tenant namespaces if enabled
2. **CI/CD Namespace** - `<tenant>-cicd` for deployment automation
   - Includes Istio injection label matching tenant settings
3. **ServiceAccount** - `<tenant>-deployer` for CI/CD with cloud IAM annotations
4. **ClusterRole** - Full cluster access for tenant resources
   - Includes permissions for Istio resources (AuthorizationPolicy, PeerAuthentication, etc.)
5. **ClusterRoleBinding** - Binds ServiceAccount to ClusterRole

### Istio Resources (When `settings.istio.enabled: true`)
6. **PeerAuthentication** - Tenant-level mTLS policy
   - Mode: STRICT (default) or PERMISSIVE based on `settings.istio.strictMTLS`
   - Applies to all tenant workloads
7. **AuthorizationPolicy** - Tenant isolation policy
   - Allows traffic only from same-tenant namespaces
   - Allows traffic from istio-system (ingress gateway, telemetry)
   - Denies cross-tenant communication at Layer 7

### AWS-Specific Resources
8. **Cluster IAM Role** (via ACK) - Pod Identity role in cluster account
   - Trust: `pods.eks.amazonaws.com` service principal
9. **Tenant IAM Role** (via ACK) - Actual permissions role in tenant account
   - Trust: Cluster IAM role with external ID
   - **Security:** Permission boundary applied to prevent privilege escalation
     - Prevents IAM policy modifications
     - Prevents assuming roles outside tenant scope
     - Limits access to tenant-specific resources only
10. **Pod Identity Association** - Links ServiceAccount to cluster IAM role
    - Annotation added to ServiceAccount: `eks.amazonaws.com/role-arn`

### Azure-Specific Resources
8. **UserAssignedIdentity** (via ASO) - Managed Identity in resource group
9. **FederatedIdentityCredential** (via ASO) - Workload Identity federation
   - Annotations added to ServiceAccount:
     - `azure.workload.identity/client-id`
     - `azure.workload.identity/tenant-id`

---

## Related Documentation

- [Cluster Configuration Reference](../../../clusters/README.md)

---

**Status:** ✅ Production ready
