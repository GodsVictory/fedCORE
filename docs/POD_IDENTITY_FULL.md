# EKS Pod Identity - Full Implementation Guide

## Overview

This platform uses **EKS Pod Identity** for all AWS IAM authentication - ACK controllers, tenant workloads, and application pods. **No OIDC providers are used anywhere**.

**Architecture:** Two-Tier Role System
- **Cluster Account Roles**: Pod Identity provides credentials
- **Tenant Account Roles**: Cluster roles assume these for actual permissions

**Benefits:**
- ✅ No OIDC provider setup in any account
- ✅ Simpler trust policies (IAM role principals only)
- ✅ 15-minute credential rotation everywhere
- ✅ Consistent authentication pattern across platform
- ✅ More secure (specific role principals, not OIDC URLs)

---

## Architecture

### Two-Tier Role System

```
┌─────────────────────────────────────────────────────────────┐
│  Cluster Account (123456789012)                             │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ EKS Pod Identity Agent (DaemonSet)                   │  │
│  │ - Validates pods                                     │  │
│  │ - Provides credentials for cluster account roles     │  │
│  └───────────────────┬──────────────────────────────────┘  │
│                      │                                      │
│                      │ Injects credentials                  │
│                      ▼                                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Pods                                                 │  │
│  │ - ACK IAM Controller                                 │  │
│  │ - ACK S3 Controller                                  │  │
│  │ - acme-deployer (tenant CI/CD)                       │  │
│  │ - myapp (tenant application)                         │  │
│  └───────────────────┬──────────────────────────────────┘  │
│                      │                                      │
│                      │ Use credentials                      │
│                      ▼                                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Cluster Account IAM Roles (Pod Identity)            │  │
│  │ - fedcore-prod-use1-ack-iam-controller              │  │
│  │ - fedcore-prod-use1-acme-deployer-abc123            │  │
│  │ - fedcore-prod-use1-myapp-def456                     │  │
│  │                                                       │  │
│  │ Trust: pods.eks.amazonaws.com                       │  │
│  │ Permissions: sts:AssumeRole (cross-account)         │  │
│  └───────────────────┬──────────────────────────────────┘  │
│                      │                                      │
└──────────────────────┼──────────────────────────────────────┘
                       │
                       │ sts:AssumeRole + ExternalId
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  Tenant Account (987654321012)                              │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Tenant Account IAM Roles                             │  │
│  │ - fedcore-ack-provisioner                            │  │
│  │ - acme-deployer-abc123                               │  │
│  │ - myapp-def456                                        │  │
│  │                                                       │  │
│  │ Trust: Cluster account role ARNs                    │  │
│  │ Permissions: Actual AWS service access (S3, RDS, etc)│  │
│  │ Boundary: TenantMaxPermissions                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ AWS Resources                                        │  │
│  │ - S3 Buckets                                         │  │
│  │ - DynamoDB Tables                                    │  │
│  │ - RDS Databases                                      │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  NO OIDC PROVIDER ✅                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Authentication Flow

**Example: App Pod Accessing S3 in Tenant Account**

```
1. Pod starts → Pod Identity Agent intercepts

2. Agent validates pod identity:
   - Namespace: acme-app
   - ServiceAccount: myapp
   - Role annotation: arn:aws:iam::123456789012:role/fedcore-prod-use1-myapp-def456

3. Agent provides STS credentials for cluster account role:
   AWS_CONTAINER_CREDENTIALS_FULL_URI → cluster role credentials

4. App code calls AWS SDK (boto3.client('s3'))

5. SDK credential chain:
   a. Detects Pod Identity credentials
   b. Loads cluster account role: fedcore-prod-use1-myapp-def456
   c. Finds AWS_ROLE_ARN env var: arn:aws:iam::987654321012:role/myapp-def456
   d. Auto-calls sts:AssumeRole with external ID

6. SDK has tenant account credentials → S3 access works
```

**Key Point:** Developers don't need to manually assume roles - AWS SDK does it automatically using environment variables.

---

## Implementation

### 1. Install Pod Identity Agent

```bash
# Install Pod Identity Agent as EKS add-on
aws eks create-addon \
  --cluster-name fedcore-prod-use1 \
  --addon-name eks-pod-identity-agent \
  --addon-version v1.2.0-eksbuild.1 \
  --resolve-conflicts OVERWRITE

# Wait for installation (2-3 minutes)
aws eks describe-addon \
  --cluster-name fedcore-prod-use1 \
  --addon-name eks-pod-identity-agent \
  --query 'addon.status'

# Verify DaemonSet
kubectl get daemonset eks-pod-identity-agent -n kube-system
```

### 2. Update ACK Controller IAM Roles

```bash
# Update trust policy for ACK IAM controller
CLUSTER_NAME="fedcore-prod-use1"
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
REGION="us-east-1"

cat > /tmp/ack-iam-trust.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"Service": "pods.eks.amazonaws.com"},
    "Action": ["sts:AssumeRole", "sts:TagSession"],
    "Condition": {
      "StringEquals": {"aws:SourceAccount": "${ACCOUNT_ID}"},
      "ArnEquals": {"aws:SourceArn": "arn:aws:eks:${REGION}:${ACCOUNT_ID}:cluster/${CLUSTER_NAME}"}
    }
  }]
}
EOF

aws iam update-assume-role-policy \
  --role-name ${CLUSTER_NAME}-ack-iam-controller \
  --policy-document file:///tmp/ack-iam-trust.json

# Repeat for other ACK controllers (S3, DynamoDB, etc.)
```

### 3. Create Pod Identity Associations

```bash
# Associate ACK IAM Controller ServiceAccount with IAM Role
aws eks create-pod-identity-association \
  --cluster-name fedcore-prod-use1 \
  --namespace ack-system \
  --service-account ack-iam-controller \
  --role-arn arn:aws:iam::${ACCOUNT_ID}:role/fedcore-prod-use1-ack-iam-controller

# Associate S3 Controller
aws eks create-pod-identity-association \
  --cluster-name fedcore-prod-use1 \
  --namespace ack-system \
  --service-account ack-s3-controller \
  --role-arn arn:aws:iam::${ACCOUNT_ID}:role/fedcore-prod-use1-ack-s3-controller

# Verify associations
aws eks list-pod-identity-associations --cluster-name fedcore-prod-use1
```

### 4. Deploy Updated RGDs

The updated tenant and webapp RGDs automatically create two-tier role architecture:

```bash
# Commit and push updated RGDs
git add platform/rgds/tenant/overlays/aws/overlay.yaml
git add platform/rgds/webapps/overlays/aws/overlay.yaml
git commit -m "refactor: full Pod Identity implementation (no OIDC)"
git push

# GitOps will deploy automatically
flux reconcile kustomization tenant-rgd --with-source
flux reconcile kustomization webapp-rgd --with-source
```

### 5. Restart ACK Controllers

```bash
# Restart to pick up Pod Identity
kubectl rollout restart deployment -n ack-system ack-iam-controller
kubectl rollout restart deployment -n ack-system ack-s3-controller

# Verify Pod Identity credentials
kubectl exec -n ack-system deploy/ack-iam-controller -- env | grep AWS_CONTAINER

# Expected:
# AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE=/var/run/secrets/pods.eks.amazonaws.com/serviceaccount/eks-pod-identity-token
# AWS_CONTAINER_CREDENTIALS_FULL_URI=http://169.254.170.23/v1/credentials
```

### 6. Create Test Tenant

```bash
cat <<EOF | kubectl apply -f -
apiVersion: platform.fedcore.io/v1
kind: TenantOnboarding
metadata:
  name: test-tenant
spec:
  tenantName: test-tenant
  aws:
    accountId: "999888777666"
    region: us-east-1
  owners:
    - kind: User
      name: admin@example.com
  quotas:
    namespaces: 5
    cpu: "10"
    memory: "20Gi"
    storage: "100Gi"
    maxPVCs: 50
  billing:
    costCenter: "TEST123"
    contact: "test@example.com"
EOF

# Watch resources created
kubectl get roles.iam.services.k8s.aws -n test-tenant-cicd -w

# Expected:
# - Permission boundary policy
# - ACK provisioner role
# - Cluster deployer role (cluster account)
# - Tenant deployer role (tenant account)
# NO OIDC provider! ✅
```

### 7. Test Workload Pod Identity

```bash
# Create test webapp
cat <<EOF | kubectl apply -f -
apiVersion: platform.fedcore.io/v1
kind: WebApp
metadata:
  name: testapp
  namespace: test-tenant-app
spec:
  appName: testapp
  image: nginx:latest
  replicas: 1
EOF

# Verify roles created
kubectl get roles.iam.services.k8s.aws -n test-tenant-app

# Check pod environment
POD=$(kubectl get pod -n test-tenant-app -l app=testapp -o name | head -1)
kubectl exec -n test-tenant-app $POD -- env | grep AWS

# Expected:
# AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE=...
# AWS_CONTAINER_CREDENTIALS_FULL_URI=...
# AWS_ROLE_ARN=arn:aws:iam::999888777666:role/testapp-...
# AWS_STS_EXTERNAL_ID=fedcore-app-test-tenant-testapp

# Test S3 access
kubectl exec -n test-tenant-app $POD -- aws s3 ls

# SDK will automatically:
# 1. Use Pod Identity for cluster role
# 2. Assume tenant role using AWS_ROLE_ARN
# 3. Access S3 in tenant account
```

---

## Role Naming Convention

### Cluster Account Roles

Pattern: `{cluster}-{tenant}-{resource}-{hash}`

Examples:
- `fedcore-prod-use1-ack-iam-controller`
- `fedcore-prod-use1-acme-deployer-abc12345`
- `fedcore-prod-use1-myapp-def67890`

### Tenant Account Roles

Pattern: `{tenant}-{resource}-{hash}` or `fedcore-ack-provisioner`

Examples:
- `fedcore-ack-provisioner` (for ACK controllers)
- `acme-deployer-abc12345` (for tenant CI/CD)
- `myapp-def67890` (for application pods)

---

## Trust Policy Patterns

### Cluster Account Role (Pod Identity)

Used by: All pods in cluster account

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"Service": "pods.eks.amazonaws.com"},
    "Action": ["sts:AssumeRole", "sts:TagSession"],
    "Condition": {
      "StringEquals": {"aws:SourceAccount": "123456789012"},
      "ArnEquals": {"aws:SourceArn": "arn:aws:eks:us-east-1:123456789012:cluster/fedcore-prod-use1"}
    }
  }]
}
```

**Trust:** EKS Pod Identity service principal
**Validated by:** Pod Identity Agent (pod namespace + ServiceAccount)

### Tenant Account Role (Cross-Account AssumeRole)

Used by: Resources in tenant accounts

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"AWS": "arn:aws:iam::123456789012:role/fedcore-prod-use1-myapp-def67890"},
    "Action": "sts:AssumeRole",
    "Condition": {
      "StringEquals": {"sts:ExternalId": "fedcore-app-acme-myapp"}
    }
  }]
}
```

**Trust:** Specific cluster account role
**Protection:** External ID (confused deputy prevention)

---

## AWS SDK Configuration

The RGDs automatically inject environment variables for AWS SDK role chaining:

```yaml
env:
  - name: AWS_REGION
    value: us-east-1
  - name: AWS_ROLE_ARN  # Tenant role to assume
    value: arn:aws:iam::987654321012:role/myapp-def67890
  - name: AWS_ROLE_SESSION_NAME
    value: myapp-session
  - name: AWS_STS_EXTERNAL_ID  # For role assumption
    value: fedcore-app-acme-myapp
```

**SDK Behavior:**
1. Detects Pod Identity credentials (cluster role)
2. Reads `AWS_ROLE_ARN` environment variable
3. Automatically calls `sts:AssumeRole` with external ID
4. Uses tenant account credentials for all AWS API calls

**No code changes required** - SDK handles role chaining automatically.

---

## Security Features

### Pod Identity Advantages

1. **15-Minute Credentials**
   - Pod Identity rotates every 15 minutes (vs IRSA 1 hour)
   - Shorter exposure window

2. **No OIDC Token Files**
   - IRSA projected tokens to pod filesystem
   - Pod Identity uses webhook (no filesystem exposure)

3. **Agent Validation**
   - Pod Identity Agent validates pod before providing credentials
   - Checks namespace, ServiceAccount, and cluster membership

4. **Specific Trust Policies**
   - Tenant roles trust specific cluster role ARNs
   - More restrictive than OIDC provider URLs
   - Easier to audit

### Permission Boundaries

All tenant account roles have permission boundary:

```json
"permissionsBoundary": "arn:aws:iam::{tenant-account}:policy/TenantMaxPermissions"
```

**Enforces:**
- ✅ Tenant can use AWS services (S3, RDS, DynamoDB, etc.)
- ❌ Tenant cannot modify IAM policies/roles
- ❌ Tenant cannot access Organizations
- ❌ Tenant cannot disable security logging

### External IDs

All cross-account assumptions use external IDs:

- ACK provisioner: `fedcore-ack-{tenant}`
- Tenant deployer: `fedcore-tenant-{tenant}`
- Application: `fedcore-app-{tenant}-{app}`

**Prevents confused deputy attacks**

---

## Verification

### Manual Verification Steps

**1. Check Pod Identity Agent:**
```bash
kubectl get daemonset -n kube-system eks-pod-identity-agent
# Should show READY = DESIRED
```

**2. Check Pod Credentials:**
```bash
POD=$(kubectl get pod -n ack-system -l k8s-app=ack-iam-controller -o name | head -1)
kubectl exec -n ack-system $POD -- env | grep AWS_CONTAINER

# Expected: Two AWS_CONTAINER_* environment variables
```

**3. Test Cross-Account Access:**
```bash
kubectl exec -n ack-system $POD -- \
  aws sts assume-role \
    --role-arn arn:aws:iam::{tenant-account}:role/fedcore-ack-provisioner \
    --role-session-name test \
    --external-id fedcore-ack-{tenant}

# Should return credentials (no access denied)
```

**4. Verify No OIDC Providers:**
```bash
kubectl get openidconnectproviders.iam.services.k8s.aws --all-namespaces

# Should return "No resources found"
```

---

## Troubleshooting

### Issue: Pod Not Getting Credentials

**Symptoms:** No `AWS_CONTAINER_*` environment variables

**Check:**
```bash
# 1. Pod Identity Agent running?
kubectl get daemonset -n kube-system eks-pod-identity-agent

# 2. Pod Identity Association exists?
aws eks list-pod-identity-associations --cluster-name fedcore-prod-use1

# 3. ServiceAccount has role-arn annotation?
kubectl get sa -n {namespace} {sa-name} -o yaml | grep role-arn
```

**Fix:**
```bash
# Restart Pod Identity Agent
kubectl rollout restart daemonset -n kube-system eks-pod-identity-agent

# Restart pod
kubectl rollout restart deployment -n {namespace} {deployment}
```

### Issue: Cross-Account AssumeRole Fails

**Symptoms:** Access Denied when accessing tenant resources

**Check:**
```bash
# 1. Cluster role has sts:AssumeRole permission?
aws iam get-role-policy \
  --role-name {cluster-role} \
  --policy-name AssumeTenantRole

# 2. Tenant role trusts cluster role?
aws iam get-role \
  --role-name {tenant-role} \
  --profile tenant-profile \
  --query 'Role.AssumeRolePolicyDocument'

# 3. External ID matches?
# Check tenant role trust policy and AWS_STS_EXTERNAL_ID env var
```

**Fix:**
Update tenant role trust policy to include cluster role ARN.

### Issue: OIDC Provider Still Exists

**Symptoms:** Old OIDC provider resources in tenant accounts

**Fix:**
```bash
# Delete OIDC provider (if no longer needed)
kubectl delete openidconnectproviders.iam.services.k8s.aws -n {namespace} {name}

# ACK will delete from AWS
```

---

## Migration from IRSA (If Applicable)

If migrating existing tenants from IRSA to Pod Identity:

1. **Install Pod Identity Agent** (no impact on running workloads)
2. **Create Pod Identity Associations** (coexist with IRSA)
3. **Update ACK controller trust policies** (add Pod Identity, keep IRSA)
4. **Restart ACK controllers** (switch to Pod Identity)
5. **Deploy updated RGDs** (new tenants use Pod Identity)
6. **Update existing tenant roles** (optional - keep IRSA for existing tenants)

**No downtime required** - Pod Identity and IRSA can coexist.

---

## Cost Impact

**Pod Identity adds ~$2/month per 100 pods**

| Item | IRSA | Pod Identity |
|------|------|--------------|
| STS calls/pod/day | 24 | 96 |
| STS calls/100 pods/month | 72,000 | 288,000 |
| Cost (@ $0.01/1000 calls) | $0.72 | $2.88 |

**Justification:** Improved security and performance worth the minimal cost increase.

---

## References

- [EKS Pod Identity Documentation](https://docs.aws.amazon.com/eks/latest/userguide/pod-identities.html)
- [ACK Runtime Documentation](https://aws-controllers-k8s.github.io/community/)
- [AWS SDK Credential Chain](https://docs.aws.amazon.com/sdkref/latest/guide/standardized-credentials.html)

---

## Summary

✅ **Full Pod Identity implementation complete**
- No OIDC providers in any account
- Two-tier role architecture (cluster + tenant)
- Consistent authentication across platform
- 15-minute credential rotation everywhere
- Simpler trust policies (IAM role principals)
- AWS SDK auto-assumes tenant roles

**Tenant onboarding creates:**
- 1 permission boundary policy (tenant account)
- 1 ACK provisioner role (tenant account)
- 2 deployer roles (cluster + tenant)
- **0 OIDC providers** ✅

**Application deployment creates:**
- 2 app roles (cluster + tenant)
- 1 S3 bucket (tenant account)
- 1 S3 access policy (tenant account)
- **0 OIDC providers** ✅

---

## Related Documentation

**IAM & Authentication:**
- [IAM Architecture](IAM_ARCHITECTURE.md) - Three-tier IAM role model overview
- [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md) - Exact IAM resources LZA creates
- [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md) - Why deployers don't need AWS permissions

**Multi-Account:**
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Account isolation design
- [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md) - Technical setup with Pod Identity
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) - Onboarding and troubleshooting procedures

**Troubleshooting:**
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Pod Identity issues and cross-account access problems
- [Glossary](GLOSSARY.md) - Pod Identity and IAM terminology

---

## Navigation

[← Previous: LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md) | [Next: Ingress Management →](INGRESS_MANAGEMENT.md)

**Handbook Progress:** Page 32 of 35 | **Level 6:** IAM & Multi-Account Architecture

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
