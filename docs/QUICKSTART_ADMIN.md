# Quick Start: Platform Administrator

**Time to complete:** 5 minutes

## What You'll Do

Create your first tenant in 5 minutes - from YAML file to fully provisioned tenant with isolated namespaces, IAM roles, and cloud resources.

## Prerequisites

Before you begin:

1. **Access to GitHub repository** - Write access to the fedCORE platform repo
2. **AWS Account ID** - Obtain from Landing Zone Accelerator (LZA) team
3. **Tenant information** - Tenant name, owner email, and resource requirements
4. **kubectl access** - Configured to access the target cluster (optional, for verification)

## Step 1: Create Tenant File

Create a new file in your local repository: `platform/clusters/<cluster-name>/tenants/<tenant-name>-onboarding.yaml`

**Example:** For tenant "acme" on cluster "fedcore-prod-use1":

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme

  # AWS account ID from LZA
  aws:
    accountId: "123456789012"

  # Tenant owners (can manage namespaces and resources)
  owners:
    - kind: User
      name: admin@acme-corp.com
      apiGroup: rbac.authorization.k8s.io

  # Resource quotas across all tenant namespaces
  quotas:
    namespaces: 10        # Maximum number of namespaces
    cpu: "100"            # Total CPU cores
    memory: "200Gi"       # Total memory
    storage: "1Ti"        # Total persistent storage

  # Billing information for cost allocation
  billing:
    costCenter: "engineering"
    department: "platform-team"
    project: "acme-app"

  # Optional: Istio service mesh integration
  networking:
    enableIstio: false
```

**Key fields explained:**
- `tenantName`: Unique identifier (lowercase, alphanumeric, hyphens only)
- `aws.accountId`: AWS account provisioned by LZA for this tenant
- `owners`: List of users/groups with admin permissions for tenant namespaces
- `quotas`: Aggregate resource limits across all tenant namespaces
- `billing`: Tags applied to all AWS resources for cost tracking

## Step 2: Commit and Push

Commit your changes and push to the main branch:

```bash
# Stage the new tenant file
git add platform/clusters/<cluster-name>/tenants/<tenant-name>-onboarding.yaml

# Commit with descriptive message
git commit -m "Add tenant onboarding for acme"

# Push to trigger CI/CD
git push origin main
```

**What happens next:**
1. CI/CD pipeline validates the tenant manifest
2. Pipeline builds OCI artifact with tenant configuration
3. Pipeline pushes artifact to Nexus OCI registry
4. Flux detects new artifact and syncs to cluster
5. Kro processes TenantOnboarding CR and creates resources

**Timing:** Deployment typically completes in 2-3 minutes after push.

## Step 3: Verify Deployment

Once the pipeline completes, verify the tenant was created successfully:

```bash
# Check tenant exists
kubectl get tenant acme

# View tenant details
kubectl describe tenant acme

# List tenant namespaces (should show acme-cicd initially)
kubectl get namespaces -l capsule.clastix.io/tenant=acme

# Verify tenant IAM roles were created
kubectl get serviceaccount -n acme-cicd acme-deployer -o yaml
```

**Expected output:**

```yaml
# kubectl get tenant acme
NAME   STATE    NAMESPACE QUOTA   AGE
acme   Active   10                 2m

# kubectl get namespaces -l capsule.clastix.io/tenant=acme
NAME        STATUS   AGE
acme-cicd   Active   2m
```

## What You Created

Your tenant now has:

1. **Capsule Tenant** (`acme`)
   - Isolated multi-tenant boundary
   - Enforces namespace naming (`acme-*` pattern)
   - Aggregates resource quotas across namespaces
   - Provides tenant-scoped RBAC

2. **CI/CD Namespace** (`acme-cicd`)
   - Dedicated namespace for automation
   - ServiceAccount with kubectl permissions
   - IAM role for AWS access (if using Pod Identity)

3. **AWS IAM Roles** (in tenant AWS account)
   - **ACK Provisioner Role**: Used by cluster to provision AWS resources
   - **Tenant Deployer Role**: Used by CI/CD pipelines (kubectl only, zero AWS permissions)
   - **Permission Boundary**: Limits maximum permissions for all tenant roles

4. **Pod Identity Associations** (AWS EKS)
   - Maps Kubernetes ServiceAccounts to IAM roles
   - Enables pods to authenticate with AWS services

5. **Resource Quotas**
   - CPU, memory, storage, and namespace limits
   - Enforced at tenant level across all namespaces

## Next Steps

Now that your tenant is created, explore these guides:

- **[Tenant Admin Guide](TENANT_ADMIN_GUIDE.md)** - Detailed tenant management operations
- **[Tenant User Guide](TENANT_USER_GUIDE.md)** - How tenants use the platform
- **[Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)** - Understanding tenant isolation
- **[IAM Architecture](IAM_ARCHITECTURE.md)** - Three-tier IAM role design

## Troubleshooting

### Tenant stuck in "Pending" state

Check TenantOnboarding status:
```bash
kubectl describe tenantonboarding acme
```

Look for errors in the Kro controller logs:
```bash
kubectl logs -n kro-system -l app=kro-controller --tail=100
```

### AWS IAM role creation failed

Verify the AWS account ID is correct:
```bash
kubectl get tenantonboarding acme -o jsonpath='{.spec.aws.accountId}'
```

Check that LZA has provisioned the tenant account with required IAM roles. See [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md).

### Quota errors

If users report quota errors, check current usage:
```bash
kubectl describe tenant acme | grep -A 10 "Resource Quota"
```

To increase quotas, edit the TenantOnboarding manifest and push changes.

### Common Issues

| Issue | Solution |
|-------|----------|
| Invalid tenant name format | Use lowercase alphanumeric and hyphens only |
| Duplicate tenant name | Choose a unique tenant name |
| AWS account not found | Verify account ID with LZA team |
| Owner email not recognized | Verify user exists in identity provider |

## Additional Resources

- **[Glossary](GLOSSARY.md)** - Platform terminology
- **[Troubleshooting Guide](TROUBLESHOOTING.md)** - Comprehensive problem resolution
- **[FAQ](FAQ.md)** - Frequently asked questions
- **[Security Overview](SECURITY_OVERVIEW.md)** - Platform security model

---

## Navigation

[← Previous: Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) | [Next: Developer Quick Start →](QUICKSTART_DEVELOPER.md)

**Handbook Progress:** Page 5 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
