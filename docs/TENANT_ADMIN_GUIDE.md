# Tenant Admin Guide

**Multi-Tenant Platform Administration**

This guide is for platform administrators who need to create and manage tenants on the fedCORE Platform. It covers the architecture, tenant onboarding procedures, and multi-cluster deployment strategies.

## Architecture Overview

### Three-Layer Security Model

1. **Capsule** - Tenant boundaries and resource aggregation
   - Defines tenant ownership and namespace quotas
   - Aggregates resource quotas across all tenant namespaces
   - Enforces naming conventions (tenant prefix)

2. **Kyverno (Enforce Mode)** - Critical security policies
   - Image registry restrictions
   - Security baselines (no privileged containers, must run as non-root)
   - Network isolation
   - Resource limits enforcement

3. **Kyverno (Audit Mode)** - Best practices guidance
   - Readiness/liveness probes
   - PodDisruptionBudgets
   - Standard labels
   - HPA recommendations

---

## Creating a New Tenant

There are two approaches for tenant onboarding, both GitOps-based:

### Approach 1: ytt + Capsule (Basic Isolation)

**Use for:** Simple tenants that only need namespace isolation, no CI/CD automation

**Creates:** Capsule Tenant, network policies, resource quotas

### Approach 2: KRO + TenantOnboarding (Full Automation)

**Use for:** Tenants needing CI/CD automation, ServiceAccounts, and cloud IAM roles

**Creates:** Everything from Approach 1, plus:
- CI/CD namespace with ServiceAccount
- AWS IAM Role (Pod Identity) or Azure Managed Identity
- RBAC for automated deployments
- Configuration secrets

See [Tenant Onboarding with KRO](../platform/rgds/tenant/README.md) for complete documentation.

**Both approaches can coexist!** You can use ytt for basic Capsule setup and add KRO for automation.

---

## Approach 1: ytt + Capsule

### For Platform Administrators

Tenants are declared as code in cluster configuration files and deployed via GitOps.

**Step 1: Create tenant file**

Create a new file in the cluster's `tenants/` directory:

```bash
cat > platform/clusters/fedcore-prod-use1/tenants/acme.yaml <<'EOF'
#@data/values
---
#! Tenant: acme
#! Owner: Acme Corp Engineering Team

tenants:
  acme:
    owners:
      - kind: User
        name: john@acme-corp.com
        apiGroup: rbac.authorization.k8s.io
      - kind: Group
        name: acme-admins
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 10
    resources:
      cpu: "100"
      memory: "200Gi"
      storage: "1Ti"
      max_pvcs: 50
    cost_center: "engineering"
    billing_contact: "finance@acme-corp.com"
    allow_loadbalancer: true
EOF
```

**Step 2: Commit and push**

```bash
git add platform/clusters/fedcore-prod-use1/tenants/acme.yaml
git commit -m "Add acme tenant to fedcore-prod-use1"
git push origin main
```

**Step 3: CI/CD automatically deploys**

The GitHub Actions workflow will:
1. Discover the cluster with updated tenants
2. Build the infrastructure artifact (includes tenant definitions)
3. Deploy to the cluster via Flux

This creates:
- Capsule Tenant with namespace quota and resource limits
- Network policies for isolation
- Documentation ConfigMap
- RBAC for tenant admins

### Verify Tenant Creation

```bash
# Check Capsule tenant
kubectl get tenants.capsule.clastix.io acme

# View tenant details
kubectl describe tenant acme

# Check tenant documentation
kubectl get configmap acme-tenant-info -n capsule-system -o yaml
```

### Multi-Cluster Tenants

To deploy a tenant across multiple clusters, create the same tenant file in each cluster's `tenants/` directory:

```bash
# Copy tenant to AWS production cluster
cp platform/clusters/fedcore-prod-use1/tenants/acme.yaml platform/clusters/fedcore-prod-azeus/tenants/

# Or create directly
cat > platform/clusters/fedcore-prod-azeus/tenants/acme.yaml <<'EOF'
#@data/values
---
tenants:
  acme:
    owners: [...]
    namespace_quota: 10
    # ... same config
EOF

# Commit once, deploys everywhere
git add platform/clusters/*/tenants/acme.yaml
git commit -m "Deploy acme tenant to all prod clusters"
git push
```

## Approach 2: KRO + TenantOnboarding (Full Automation)

For tenants that need CI/CD automation, ServiceAccounts, and cloud IAM integration, use the KRO-based approach:

```bash
# 1. Copy template
cp platform/rgds/tenant/base/example-tenant.yaml \
   platform/clusters/fedcore-prod-use1/tenants/acme-onboarding.yaml

# 2. Edit with tenant details (quotas, cloud permissions, billing)

# 3. Commit and push
git add platform/clusters/fedcore-prod-use1/tenants/acme-onboarding.yaml
git commit -m "Onboard tenant: acme with CI/CD automation"
git push
```

**What you get:**
- ✅ Capsule Tenant (same as ytt approach)
- ✅ CI/CD namespace (`<tenant>-cicd`)
- ✅ ServiceAccount with Pod Identity/Workload Identity
- ✅ AWS IAM Role or Azure Managed Identity
- ✅ RBAC for automated deployments
- ✅ Pre-configured secrets and documentation

See complete documentation: [Tenant Onboarding with KRO](../platform/rgds/tenant/README.md)

---

## Multi-Account Tenants (AWS)

When using AWS multi-account architecture, each tenant receives a dedicated AWS account:

### TenantOnboarding with AWS Account

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme

  # Tenant's dedicated AWS account from LZA
  aws:
    accountId: "987654321012"

  owners:
    - kind: User
      name: john@acme-corp.com
      apiGroup: rbac.authorization.k8s.io

  quotas:
    namespaces: 10
    cpu: "100"
    memory: "200Gi"
    storage: "1Ti"

  billing:
    costCenter: "CC12345"
    contact: "finance@acme-corp.com"
```

**What gets created automatically:**

**In Tenant AWS Account:**
- Permission boundary policy (`TenantMaxPermissions`)
- ACK provisioner role (`fedcore-ack-provisioner`)
- Tenant deployer role with actual permissions

**In Cluster AWS Account:**
- Cluster deployer role (Pod Identity)
- Pod Identity Association

**In Kubernetes:**
- Capsule Tenant with account ID annotation
- CI/CD namespace (`acme-cicd`)
- ServiceAccount with Pod Identity annotation
- RBAC for tenant owners and CI/CD

**See:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) for complete details

---

## Tenant Configuration Options

### Quotas and Limits

```yaml
tenants:
  acme:
    namespace_quota: 10  # Maximum namespaces
    resources:
      cpu: "100"         # Total CPU cores across all namespaces
      memory: "200Gi"    # Total memory across all namespaces
      storage: "1Ti"     # Total PVC storage
      max_pvcs: 50       # Maximum number of PVCs
```

### Networking Options

```yaml
tenants:
  acme:
    allow_loadbalancer: true  # Allow LoadBalancer services
    allow_internet_egress: true  # Allow egress to internet
```

### Istio Service Mesh

```yaml
tenants:
  acme:
    settings:
      istio:
        enabled: true      # Enable Istio sidecar injection
        strictMTLS: true   # Enforce STRICT mTLS mode
```

### Default Resource Limits

```yaml
tenants:
  acme:
    settings:
      defaultCPULimit: "500m"
      defaultMemoryLimit: "512Mi"
```

---

## Managing Existing Tenants

### Updating Tenant Quotas

Edit the tenant file and commit:

```bash
vim platform/clusters/fedcore-prod-use1/tenants/acme.yaml

# Change:
#   cpu: "100"
# To:
#   cpu: "200"

git add platform/clusters/fedcore-prod-use1/tenants/acme.yaml
git commit -m "Increase acme tenant CPU quota to 200 cores"
git push origin main
```

### Adding Tenant Owners

```yaml
tenants:
  acme:
    owners:
      - kind: User
        name: john@acme-corp.com
        apiGroup: rbac.authorization.k8s.io
      - kind: User
        name: jane@acme-corp.com  # NEW
        apiGroup: rbac.authorization.k8s.io
      - kind: Group
        name: acme-admins
        apiGroup: rbac.authorization.k8s.io
```

### Offboarding a Tenant

Remove the tenant file from git:

```bash
git rm platform/clusters/fedcore-prod-use1/tenants/acme.yaml
git commit -m "Offboard tenant: acme"
git push origin main
```

**IMPORTANT:** This will delete:
- Capsule Tenant
- All tenant namespaces
- RBAC bindings
- CI/CD resources (if using KRO approach)
- AWS IAM resources (if multi-account)

**Backup first!** Export critical data before offboarding.

---

## Monitoring Tenants

### Tenant Status

```bash
# List all tenants
kubectl get tenants

# View tenant details
kubectl describe tenant acme

# Check tenant resource usage
kubectl get tenant acme -o jsonpath='{.status.namespaces}'
```

### Resource Quota Usage

```bash
# View aggregate quota
kubectl get tenant acme -o yaml | grep -A 10 resourceQuotas

# Check per-namespace usage
kubectl get resourcequota -A | grep acme
```

### Policy Violations

```bash
# View policy reports for tenant
kubectl get policyreport -A | grep acme

# Check specific violations
kubectl describe policyreport -n tenant-acme-frontend
```

---

## Troubleshooting

### Tenant Not Created

**Check CI/CD pipeline:**
```bash
# View GitHub Actions workflow logs
# Navigate to: https://github.com/<org>/<repo>/actions

# Check Flux reconciliation
flux get ocirepositories -n flux-system
flux get kustomizations -n flux-system
```

**Check Capsule operator:**
```bash
kubectl logs -n capsule-system -l app.kubernetes.io/name=capsule
```

### Namespace Creation Fails

**Common issues:**
- Namespace quota exceeded
- Invalid namespace name (must match `<tenant>-*` pattern)
- RBAC issues (user not a tenant owner)

**Debug:**
```bash
# Check quota
kubectl get tenant acme -o jsonpath='{.spec.namespaceOptions.quota}'
kubectl get tenant acme -o jsonpath='{.status.namespaces}' | jq length

# Check user permissions
kubectl auth can-i create namespaces --as=user@example.com
```

### Multi-Account Issues

**ACK cannot create resources in tenant account:**
```bash
# Check ACK logs
kubectl logs -n ack-system -l k8s-app=ack-iam-controller --tail=100

# Verify account ID annotation
kubectl get tenant acme -o jsonpath='{.metadata.annotations.platform\.fedcore\.io/aws-account-id}'
```

**See:** [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) for troubleshooting

---

## Best Practices

### 1. Tenant Naming

- Use lowercase alphanumeric characters
- Keep names short (1-30 characters)
- Use meaningful, recognizable names
- Avoid reserved names (platform, system, admin, etc.)

### 2. Quota Planning

- Start conservative, increase as needed
- Monitor actual usage before increasing
- Plan for sidecar overhead (Istio adds ~50Mi memory per pod)
- Consider CI/CD resource needs

### 3. Multi-Cluster Strategy

- Use same tenant name across all clusters
- Adjust quotas per cluster based on workload
- Test in dev/staging before deploying to prod
- Use consistent owner configuration

### 4. GitOps Hygiene

- Always commit tenant changes to git
- Never use `kubectl apply` directly
- Write clear commit messages
- Review changes in PR before merge

### 5. Security

- Regularly review tenant owners
- Audit policy violation reports
- Monitor resource usage trends
- Review permission boundaries quarterly

---

## Related Documentation

- [Tenant User Guide](TENANT_USER_GUIDE.md) - For tenant owners
- [Tenant Advanced Topics](TENANT_ADVANCED_TOPICS.md) - Cross-namespace communication, service mesh
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - AWS multi-account design
- [Security Overview](SECURITY_OVERVIEW.md) - Security model and policies
- [Tenant Onboarding RGD](../platform/rgds/tenant/README.md) - KRO-based onboarding

---

## Navigation

[← Previous: Environment Setup](ENVIRONMENT_SETUP.md) | [Next: Tenant User Guide →](TENANT_USER_GUIDE.md)

**Handbook Progress:** Page 13 of 35 | **Level 3:** Tenant Management

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
