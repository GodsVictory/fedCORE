# Multi-Account Implementation

## Overview

This document provides technical implementation details for the fedCORE multi-account architecture. It covers IAM role setup, ACK cross-account configuration, RGD patterns, security considerations, and best practices.

**Prerequisites**: Understanding of [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

## Implementation Components

### 1. Updated TenantOnboarding Schema

```yaml
# platform/rgds/tenant/base/tenant-rgd.yaml
spec:
  schema:
    apiVersion: v1
    kind: TenantOnboarding
    spec:
      tenantName: string

      # NEW: AWS account information
      aws:
        # Tenant's dedicated AWS account ID
        accountId: string

        # Optional: If LZA provides a specific region for this tenant
        region: string | default="${cluster.region}"

      owners: [...]
      quotas: {...}
      billing: {...}
```

### 2. Cross-Account Role Setup in Tenant Accounts

Each tenant account needs these IAM resources:

#### A. ACK Provisioner Role

**Purpose**: Allow ACK controllers in cluster account to provision resources in tenant account

```yaml
# Created via ACK IAM controller (cross-account)
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Role
metadata:
  name: acme-ack-provisioner
  namespace: acme-cicd
spec:
  name: fedcore-ack-provisioner
  description: "Allows ACK controllers from cluster account to provision resources"

  # Trust cluster account's ACK controller role
  assumeRolePolicyDocument: |
    {
      "Version": "2012-10-17",
      "Statement": [{
        "Effect": "Allow",
        "Principal": {
          "AWS": "arn:aws:iam::123456789012:role/ack-controller-role"
        },
        "Action": "sts:AssumeRole",
        "Condition": {
          "StringEquals": {
            "sts:ExternalId": "fedcore-ack-acme"
          }
        }
      }]
    }

  # Permission boundary to prevent privilege escalation
  permissionsBoundary: arn:aws:iam::987654321012:policy/TenantMaxPermissions

  # Policies for creating resources
  policies:
    - policyName: AllowResourceProvisioning
      policyDocument: |
        {
          "Version": "2012-10-17",
          "Statement": [
            {
              "Effect": "Allow",
              "Action": [
                "s3:*",
                "rds:*",
                "dynamodb:*",
                "elasticache:*",
                "ec2:*",
                "iam:CreateRole",
                "iam:PutRolePolicy",
                "iam:AttachRolePolicy",
                "iam:TagRole"
              ],
              "Resource": "*"
            }
          ]
        }
```

#### B. Cluster Account Workload Roles (Pod Identity)

**Purpose**: Enable pods to get AWS credentials via Pod Identity

**Note**: These roles are created in the CLUSTER account, not tenant accounts. Pod Identity provides credentials for these roles, which then assume roles in tenant accounts.

```yaml
# Created via ACK IAM controller (in cluster account)
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Role
metadata:
  name: acme-deployer-cluster
  namespace: acme-cicd
spec:
  name: fedcore-prod-use1-acme-deployer-xyz123
  description: "Cluster role for acme workloads (Pod Identity)"

  # Pod Identity trust policy
  assumeRolePolicyDocument: |
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

  # Permission to assume tenant account role
  policies:
    - policyName: AssumeTenantRole
      policyDocument: |
        {
          "Version": "2012-10-17",
          "Statement": [{
            "Effect": "Allow",
            "Action": "sts:AssumeRole",
            "Resource": "arn:aws:iam::987654321012:role/acme-deployer-xyz123"
          }]
        }
```

#### C. Permission Boundary Policy

**Purpose**: Prevent tenant workload roles from escalating privileges

```yaml
# Created via ACK IAM controller (cross-account)
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Policy
metadata:
  name: acme-permission-boundary
  namespace: acme-cicd
spec:
  name: TenantMaxPermissions
  description: "Permission boundary for all tenant-created IAM roles"
  policyDocument: |
    {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Action": [
            "s3:*",
            "rds:*",
            "dynamodb:*",
            "elasticache:*",
            "sqs:*",
            "sns:*",
            "secretsmanager:GetSecretValue",
            "kms:Decrypt"
          ],
          "Resource": "*"
        },
        {
          "Effect": "Deny",
          "Action": [
            "iam:CreateUser",
            "iam:CreateAccessKey",
            "iam:DeleteUserPolicy",
            "iam:PutUserPolicy",
            "iam:AttachUserPolicy",
            "iam:CreatePolicyVersion",
            "iam:DeletePolicy",
            "organizations:*",
            "account:*"
          ],
          "Resource": "*"
        }
      ]
    }
```

#### D. Tenant Account Workload Role (Actual Permissions)

**Purpose**: Allow cluster account roles to assume and access tenant AWS resources

```yaml
# Created via ACK IAM controller (cross-account in tenant account)
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Role
metadata:
  name: acme-deployer-tenant
  namespace: acme-cicd
spec:
  name: acme-deployer-xyz123
  description: "Tenant role for acme workloads with actual permissions"

  # Permission boundary applied
  permissionsBoundary: arn:aws:iam::987654321012:policy/TenantMaxPermissions

  # Trust the cluster account role (Pod Identity)
  assumeRolePolicyDocument: |
    {
      "Version": "2012-10-17",
      "Statement": [{
        "Effect": "Allow",
        "Principal": {
          "AWS": "arn:aws:iam::123456789012:role/fedcore-prod-use1-acme-deployer-xyz123"
        },
        "Action": "sts:AssumeRole",
        "Condition": {
          "StringEquals": {
            "sts:ExternalId": "fedcore-tenant-acme"
          }
        }
      }]
    }
```

### 3. ACK Cross-Account Configuration via IAMRoleSelector

Cross-account routing is handled by **IAMRoleSelector** CRDs (cluster-scoped), not per-resource annotations. CARM is disabled; the IAMRoleSelector feature gate is enabled on all ACK controllers.

#### How It Works

Each tenant gets two IAMRoleSelector CRDs that match ACK resources by label:

1. **Bootstrap selector** (`<tenant>-bootstrap`) - matches resources labeled for bootstrap, routes to `FedCoreBootstrapRole`
2. **Provisioner selector** (`<tenant>`) - matches resources labeled for ongoing provisioning, routes to `fedcore-ack-provisioner`

Kyverno policy `ack-tenant-metadata` automatically adds `platform.fedcore.io/ack-target` and `platform.fedcore.io/resource-type` labels to ACK resources created in tenant namespaces. IAMRoleSelector matches on these labels to route resources to the correct tenant account and role.

#### IAMRoleSelector CRD Examples

**Bootstrap selector** (used during initial account setup):

```yaml
apiVersion: services.k8s.aws/v1alpha1
kind: IAMRoleSelector
metadata:
  name: acme-bootstrap
spec:
  selector:
    matchLabels:
      platform.fedcore.io/ack-target: acme
      platform.fedcore.io/resource-type: bootstrap
  roleARN: arn:aws:iam::987654321012:role/FedCoreBootstrapRole
```

**Provisioner selector** (used for ongoing resource provisioning):

```yaml
apiVersion: services.k8s.aws/v1alpha1
kind: IAMRoleSelector
metadata:
  name: acme
spec:
  selector:
    matchLabels:
      platform.fedcore.io/ack-target: acme
      platform.fedcore.io/resource-type: provisioner
  roleARN: arn:aws:iam::987654321012:role/fedcore-ack-provisioner
```

#### Cluster-Account Resources

Resources that should be created in the cluster account (not cross-account) opt out by adding:

```yaml
metadata:
  annotations:
    platform.fedcore.io/cluster-account: "true"
```

#### ACK Controller IAM Role

ACK controllers still need permission to assume roles in tenant accounts:

```yaml
# In cluster account (123456789012)
# Applied to ACK controller IAM role
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "sts:AssumeRole",
      "Resource": "arn:aws:iam::*:role/fedcore-ack-provisioner"
    }
  ]
}
```

#### Example: Creating a Bucket (No Annotations Needed)

With IAMRoleSelector, ACK resources no longer need `services.k8s.aws/*` annotations. Kyverno adds the routing labels automatically:

```yaml
apiVersion: s3.services.k8s.aws/v1alpha1
kind: Bucket
metadata:
  name: acme-app-data
  namespace: acme-frontend
  # Kyverno automatically adds:
  #   platform.fedcore.io/ack-target: acme
  #   platform.fedcore.io/resource-type: provisioner
spec:
  name: acme-app-data-prod-use1
```

### 4. RGD Cross-Account Resource Pattern

**Key Insight**: RGDs no longer need to inject `services.k8s.aws/*` annotations. Kyverno policy `ack-tenant-metadata` automatically adds `platform.fedcore.io/ack-target` and `platform.fedcore.io/resource-type` labels to ACK resources in tenant namespaces. IAMRoleSelector CRDs then handle routing to the correct account and role.

```yaml
# In WebApp RGD (or any RGD creating AWS resources)
# No cross-account annotations needed - IAMRoleSelector handles routing
spec:
  resources:
    - id: appBucket
      template:
        apiVersion: s3.services.k8s.aws/v1alpha1
        kind: Bucket
        metadata:
          name: ${schema.spec.appName}-data
          namespace: ${schema.spec.namespace}
          # Kyverno automatically adds ack-target and resource-type labels
          # IAMRoleSelector matches those labels to route to correct tenant account
        spec:
          name: ${schema.spec.appName}-data-${cluster.name}
```

## Prerequisites for Multi-Account Setup

### In Cluster Account (123456789012)

```yaml
# ACK controller IAM role must have:
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "sts:AssumeRole",
      "Resource": "arn:aws:iam::*:role/fedcore-ack-provisioner"
    }
  ]
}
```

**ACK Controller Configuration:**
- CARM is **disabled** on all ACK controllers
- IAMRoleSelector feature gate is **enabled** on all ACK controllers
- Kyverno policy `ack-tenant-metadata` is deployed (adds `platform.fedcore.io/ack-target` and `platform.fedcore.io/resource-type` labels)

### In Each Tenant Account (via TenantOnboarding RGD)

1. `TenantMaxPermissions` permission boundary policy
2. `fedcore-ack-provisioner` role with trust to cluster account ACK role (principal scoping in trust policy for confused deputy prevention)
3. Tenant workload roles (trusted by cluster roles)

### In Cluster (via TenantOnboarding RGD)

1. IAMRoleSelector CRDs: `<tenant>-bootstrap` and `<tenant>` (provisioner)
2. Cluster workload roles (Pod Identity)
3. Pod Identity Associations (ServiceAccount -> IAM role links)

## Security Considerations

### 1. FedCoreBootstrapRole is Powerful (LZA-Specific)

This role typically has **AdministratorAccess** in tenant accounts. Mitigations:

- **Time-bound**: ACK only assumes it during bootstrap (2-3 minutes)
- **Principal scoping**: IAM trust policy restricts which principals can assume the role (confused deputy prevention)
- **Audit trail**: CloudTrail logs all assume-role calls
- **Least privilege progression**: After bootstrap, ACK uses `fedcore-ack-provisioner` (less privileged)
- **Permission boundaries**: All tenant workload roles have `TenantMaxPermissions` boundary

**Verify security resources after onboarding:**
```bash
# Permission boundary should exist
aws iam get-policy \
  --policy-arn arn:aws:iam::987654321012:policy/TenantMaxPermissions \
  --profile tenant-acme

# ACK provisioner role trust policy should scope to cluster account principal
aws iam get-role \
  --role-name fedcore-ack-provisioner \
  --profile tenant-acme \
  --query 'Role.AssumeRolePolicyDocument'
# Should show principal scoped to cluster account ACK controller role

# Verify IAMRoleSelectors exist
kubectl get iamroleselector acme
kubectl get iamroleselector acme-bootstrap
```

### 2. Permission Boundaries

- Applied to ALL roles created in tenant accounts
- Prevents privilege escalation attacks
- Denies IAM user creation (enforce Pod Identity only)
- Denies access to AWS Organizations and account settings

### 3. IAMRoleSelector and Confused Deputy Prevention

- IAMRoleSelector does **not** support external-id
- Confused deputy prevention for ACK cross-account routing relies on **IAM trust policy principal scoping** (the trust policy on tenant roles restricts which cluster-account principals can assume them)
- Pod Identity workload role chaining still uses external-id for its own assume-role calls

### 4. Pod Identity Trust Conditions

- Service principal validation (pods.eks.amazonaws.com)
- Source account and cluster ARN verification
- Cluster roles use external IDs when assuming tenant roles (workload role chaining)
- Subject validation (system:serviceaccount:namespace:sa-name)

### 5. ACK Controller Isolation

- ACK controllers run in cluster account
- Cannot directly access tenant resources
- Must explicitly assume role per operation
- Audit trail via CloudTrail in tenant account

## Best Practices

### 1. Single CR per Tenant

Everything is in one TenantOnboarding CR - no separate bootstrap step needed. This simplifies operations and ensures consistency.

### 2. Always Use GitOps (Never Direct kubectl)

**DO THIS:**
```bash
# Commit tenant CR to git
git add platform/clusters/fedcore-prod-use1/tenants/acme-onboarding.yaml
git commit -m "Onboard tenant: acme"
git push origin main
# CI/CD pipeline builds and deploys automatically
```

**DON'T DO THIS:**
```bash
# ❌ Never apply directly - bypasses CI/CD and version control
kubectl apply -f tenants/acme-onboarding.yaml
```

**Store all tenant CRs in version control:**
```
platform/clusters/fedcore-prod-use1/tenants/
├── acme-onboarding.yaml
└── platform-team-onboarding.yaml
```

### 3. Align Naming Conventions

```
LZA Account Name: acme-production
Tenant Name: acme
K8s Namespace Prefix: acme-*
AWS Resource Tags: tenant=acme
```

### 4. Use Descriptive Commit Messages

```bash
# Good commit messages
git commit -m "Onboard tenant acme with AWS account 987654321012"
git commit -m "Update acme tenant quota: cpu from 50 to 100 cores"
git commit -m "Offboard tenant foo - contract ended"

# Bad commit messages
git commit -m "update tenant"
git commit -m "fix"
```

### 5. Monitor GitOps Pipeline

Regularly check:
```bash
# GitHub Actions for build status
# https://github.com/<org>/<repo>/actions

# Flux for sync status
flux get ocirepositories -n flux-system
flux get kustomizations -n flux-system

# Kro for RGD processing
kubectl get resourcegraphdefinitions
kubectl get tenantonboardings
```

### 6. Cost Allocation

Resources are automatically tagged for cost tracking:
```yaml
tags:
  platform.fedcore.io/cluster: fedcore-prod-use1
  platform.fedcore.io/tenant: acme
  platform.fedcore.io/cost-center: CC12345
```

Review cost allocation regularly using AWS Cost Explorer filtered by these tags.

## Advanced Implementation Patterns

### RGD Pattern: Multiple AWS Resources

With IAMRoleSelector, RGDs no longer need to inject cross-account annotations. Kyverno adds the routing labels, and IAMRoleSelector handles the rest:

```yaml
# Create multiple AWS resources - no cross-account annotations needed
spec:
  resources:
    # S3 Bucket
    - id: bucket
      template:
        apiVersion: s3.services.k8s.aws/v1alpha1
        kind: Bucket
        metadata:
          name: ${schema.spec.appName}-data
          namespace: ${schema.spec.namespace}
          # Kyverno adds ack-target + resource-type labels automatically
          # IAMRoleSelector routes to correct tenant account

    # RDS Database
    - id: database
      template:
        apiVersion: rds.services.k8s.aws/v1alpha1
        kind: DBInstance
        metadata:
          name: ${schema.spec.appName}-db
          namespace: ${schema.spec.namespace}

    # DynamoDB Table
    - id: table
      template:
        apiVersion: dynamodb.services.k8s.aws/v1alpha1
        kind: Table
        metadata:
          name: ${schema.spec.appName}-table
          namespace: ${schema.spec.namespace}
```

### Workload Pattern: Application Using Tenant Resources

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: webapp
  namespace: acme-frontend
spec:
  template:
    spec:
      serviceAccountName: acme-deployer  # Has Pod Identity annotation
      containers:
        - name: app
          image: nexus.fedcore.io/tenant-acme/webapp:v1.0.0
          env:
            # Application uses AWS SDK with Pod Identity
            - name: S3_BUCKET
              value: acme-app-data-prod-use1
            - name: AWS_REGION
              value: us-east-1
          # AWS credentials automatically provided via Pod Identity
```

**Authentication Flow:**
1. Pod requests AWS credentials via Pod Identity Agent
2. Pod Identity Agent provides cluster account role credentials (15 min)
3. Application SDK uses cluster role to assume tenant account role
4. Application accesses S3 bucket in tenant account

### CI/CD Pattern: Deploying to Tenant Namespaces

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ci-cd-job
  namespace: acme-cicd
spec:
  serviceAccountName: acme-deployer
  containers:
    - name: deploy
      image: bitnami/kubectl:latest
      command:
        - kubectl
        - apply
        - -k
        - ./manifests
      env:
        - name: AWS_REGION
          value: us-east-1
```

## Testing and Validation

### Verify Cross-Account Access

```bash
# From a pod with Pod Identity
kubectl run -it --rm test \
  --image=amazon/aws-cli \
  --serviceaccount=acme-deployer \
  --namespace=acme-cicd \
  -- aws sts get-caller-identity

# Should show cluster account role
# Then assume tenant role:
kubectl run -it --rm test \
  --image=amazon/aws-cli \
  --serviceaccount=acme-deployer \
  --namespace=acme-cicd \
  -- aws sts assume-role \
    --role-arn arn:aws:iam::987654321012:role/acme-deployer-xyz123 \
    --role-session-name test \
    --external-id fedcore-tenant-acme
```

### Verify ACK Cross-Account Provisioning

```bash
# Verify IAMRoleSelectors are in place for the tenant
kubectl get iamroleselector acme
kubectl get iamroleselector acme-bootstrap

# Create a test S3 bucket (no cross-account annotations needed)
kubectl apply -f - <<EOF
apiVersion: s3.services.k8s.aws/v1alpha1
kind: Bucket
metadata:
  name: test-bucket
  namespace: acme-frontend
  # Kyverno will add ack-target and resource-type labels
  # IAMRoleSelector will route to tenant account automatically
spec:
  name: test-bucket-${RANDOM}
EOF

# Verify Kyverno added routing labels
kubectl get bucket test-bucket -n acme-frontend -o yaml | grep -A2 'platform.fedcore.io'

# Check bucket created in tenant account
aws s3 ls --profile tenant-acme

# Delete test bucket
kubectl delete bucket test-bucket -n acme-frontend
```

### Verify Permission Boundaries

```bash
# Try to create a user (should be denied by permission boundary)
aws iam create-user --user-name test-user --profile tenant-acme
# Expected: Access Denied

# Try to modify permission boundary (should be denied)
aws iam delete-role-permissions-boundary \
  --role-name acme-deployer-xyz123 \
  --profile tenant-acme
# Expected: Access Denied
```

## Related Documentation

- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - High-level design and principles
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) - Operational procedures
- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Creating tenants
- [Security Overview](SECURITY_OVERVIEW.md) - Security architecture

---

## Navigation

[← Previous: Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) | [Next: Multi-Account Operations →](MULTI_ACCOUNT_OPERATIONS.md)

**Handbook Progress:** Page 29 of 35 | **Level 6:** IAM & Multi-Account Architecture

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
