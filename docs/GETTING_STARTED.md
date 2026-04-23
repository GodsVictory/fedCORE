# Getting Started

Practical guide for onboarding your first tenant to the fedCORE platform.

## Overview

**What is fedCORE?**
A multi-cloud internal developer platform providing self-service infrastructure across AWS, Azure, and on-premises. Each tenant gets isolated namespaces and a dedicated AWS account.

**Reading Time:** 10 minutes
**Hands-on Time:** 5 minutes (automated deployment)

**Prerequisites:**
- AWS account provisioned via Landing Zone Accelerator (LZA)
- AWS account ID from LZA team
- Tenant details (name, owners, resource requirements)

## Quick Start: Onboard a Tenant

**For a 5-minute walkthrough, see [Admin Quick Start](QUICKSTART_ADMIN.md).**

### Create TenantOnboarding Manifest

```yaml
# platform/clusters/<cluster-name>/tenants/acme-onboarding.yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme
  aws:
    accountId: "123456789012"  # From LZA
  owners:
    - kind: User
      name: admin@acme-corp.com
      apiGroup: rbac.authorization.k8s.io
  quotas:
    namespaces: 10
    cpu: "100"
    memory: "200Gi"
    storage: "1Ti"
  billing:
    costCenter: "engineering"
    contact: "finance@acme-corp.com"
```

### Deploy via GitOps

```bash
git add platform/clusters/*/tenants/acme-onboarding.yaml
git commit -m "Onboard tenant: acme"
git push origin main
```

**What happens automatically:**
1. CI/CD validates configuration
2. Builds infrastructure artifact
3. Flux deploys to cluster
4. Creates: Capsule tenant, IAM roles, namespaces, policies, quotas

### Verify Deployment

```bash
kubectl get tenants acme
kubectl get ns -l capsule.clastix.io/tenant=acme
```

## What Tenants Receive

✅ Isolated namespaces (`<tenant>-*` pattern)
✅ Dedicated AWS account with IAM roles
✅ CI/CD ServiceAccount for deployments
✅ Resource quotas (CPU, memory, storage, namespaces)
✅ Self-service namespace creation
✅ Network isolation via Capsule + Kyverno policies

## Next Steps

**For Platform Admins:**
- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Complete tenant management
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Account isolation design
- [Security Overview](SECURITY_OVERVIEW.md) - Security model

**For Tenant Users:**
- [Tenant User Guide](TENANT_USER_GUIDE.md) - Self-service operations
- [Developer Quick Start](QUICKSTART_DEVELOPER.md) - Deploy your first app

**Need Help?**
- [Troubleshooting Guide](TROUBLESHOOTING.md)
- [Glossary](GLOSSARY.md) - Terminology reference

---

## Navigation

[← Previous: FAQ](FAQ.md) | [Next: Cluster Structure →](CLUSTER_STRUCTURE.md)

**Handbook Progress:** Page 10 of 35 | **Level 2:** Platform Setup & Structure

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
