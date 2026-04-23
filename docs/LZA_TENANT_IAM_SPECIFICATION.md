# LZA Tenant Account IAM Specification

## Overview

This document specifies the **exact IAM resources** that AWS Landing Zone Accelerator (LZA) must create in each tenant AWS account for fedCORE platform integration.

**Total LZA Resources**: 1 (one role)

LZA's responsibility is minimal: create a single bootstrap role in each tenant account. fedCORE handles everything else -- including the permission boundary policy, the ACK provisioner role, and all application-level IAM resources -- using that bootstrap role.

**Last Updated**: 2026-04-03

---

## Prerequisites

Before implementing, the fedCORE team must provide:

| Variable | Example Value | Description |
|----------|---------------|-------------|
| `{CLUSTER_ACCOUNT_ID}` | `123456789012` | AWS account ID where EKS cluster runs |
| `{CLUSTER_NAME}` | `fedcore-prod-use1` | EKS cluster name |

**Note**: These values are **constant per cluster** and will be provided once by the fedCORE team.

---

## Resource 1: Bootstrap Role

### Metadata

| Property | Value |
|----------|-------|
| **Resource Type** | AWS IAM Role |
| **Role Name** | `FedCoreBootstrapRole` (FIXED) |
| **Path** | `/` |
| **Description** | Allows fedCORE to bootstrap IAM resources (policies and roles) in the tenant account |
| **Max Session Duration** | `3600` seconds (1 hour) |

### Purpose

This role is the **only** IAM resource LZA needs to create. fedCORE assumes this role during tenant onboarding to create all other required IAM resources in the tenant account, including:

- The `TenantMaxPermissions` permission boundary policy
- The `fedcore-ack-provisioner` role (used by ACK controllers for cross-account resource provisioning)
- Any future IAM resources needed by the platform

### Trust Policy (AssumeRolePolicyDocument)

**CRITICAL**: Replace placeholders with actual values provided by fedCORE team.

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::{CLUSTER_ACCOUNT_ID}:root"
      },
      "Action": "sts:AssumeRole",
      "Condition": {
        "StringLike": {
          "aws:PrincipalArn": "arn:aws:iam::{CLUSTER_ACCOUNT_ID}:role/{CLUSTER_NAME}-ack-*-controller"
        }
      }
    }
  ]
}
```

**Example with actual values**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::123456789012:root"
      },
      "Action": "sts:AssumeRole",
      "Condition": {
        "StringLike": {
          "aws:PrincipalArn": "arn:aws:iam::123456789012:role/fedcore-prod-use1-ack-*-controller"
        }
      }
    }
  ]
}
```

**Security note**: The trust policy uses IAM principal scoping via `StringLike` on `aws:PrincipalArn` to restrict access to only ACK controller roles from the designated cluster account. This prevents confused deputy attacks without requiring an external ID.

### Permission Policy

The bootstrap role needs permissions to create and manage IAM resources in the tenant account:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowIAMBootstrap",
      "Effect": "Allow",
      "Action": [
        "iam:CreateRole",
        "iam:GetRole",
        "iam:UpdateRole",
        "iam:TagRole",
        "iam:PutRolePolicy",
        "iam:GetRolePolicy",
        "iam:AttachRolePolicy",
        "iam:DetachRolePolicy",
        "iam:ListAttachedRolePolicies",
        "iam:ListRolePolicies",
        "iam:UpdateAssumeRolePolicy",
        "iam:CreatePolicy",
        "iam:GetPolicy",
        "iam:GetPolicyVersion",
        "iam:ListPolicyVersions",
        "iam:CreatePolicyVersion"
      ],
      "Resource": "*"
    }
  ]
}
```

### Tags (Optional but Recommended)

```json
{
  "Tags": [
    {
      "Key": "platform.fedcore.io/managed-by",
      "Value": "lza"
    },
    {
      "Key": "platform.fedcore.io/purpose",
      "Value": "bootstrap"
    }
  ]
}
```

### Validation

After creation, verify:

```bash
# 1. Check role exists
aws iam get-role \
  --role-name FedCoreBootstrapRole \
  --profile tenant-{TENANT_NAME}

# 2. Verify trust policy
aws iam get-role \
  --role-name FedCoreBootstrapRole \
  --profile tenant-{TENANT_NAME} \
  --query 'Role.AssumeRolePolicyDocument'
```

---

## IAMRoleSelector: Cross-Account Routing

Cross-account resource provisioning is handled by **IAMRoleSelector** custom resources rather than per-resource annotations (previously injected by Kyverno).

An `IAMRoleSelector` is a **cluster-scoped** CRD created once per tenant. It tells ACK controllers which IAM role to assume when provisioning resources in a given tenant account. This replaces the old model where Kyverno injected ACK annotations on every individual resource.

### Example IAMRoleSelector

```yaml
apiVersion: services.k8s.aws/v1alpha1
kind: IAMRoleSelector
metadata:
  name: acme
spec:
  targetAccountId: "987654321012"
  roleARN: "arn:aws:iam::987654321012:role/fedcore-ack-provisioner"
```

**Note**: IAMRoleSelector CRDs are created and managed by fedCORE during tenant onboarding. LZA does not need to create or manage these resources.

---

## What fedCORE Creates (For Your Reference)

After LZA provisions the `FedCoreBootstrapRole`, fedCORE handles everything else:

1. **Bootstrap Phase** (using `FedCoreBootstrapRole`)
   - `TenantMaxPermissions` policy -- permission boundary preventing privilege escalation in tenant accounts
   - `fedcore-ack-provisioner` role -- allows ACK controllers to provision AWS resources in the tenant account

2. **IAMRoleSelector CRDs** (cluster-scoped, one per tenant)
   - Routes ACK cross-account operations to the correct tenant account and role

3. **Cluster Account Roles** (for Pod Identity)
   - `{CLUSTER_NAME}-{TENANT_NAME}-deployer-{HASH}`
   - `{CLUSTER_NAME}-{APP_NAME}-{HASH}`

4. **Tenant Account Roles** (using ACK provisioner)
   - `{TENANT_NAME}-deployer-{HASH}` (zero AWS permissions)
   - `{APP_NAME}-{HASH}` (app-specific permissions, with TenantMaxPermissions boundary)

**LZA does NOT need to create any of these** -- they are managed via fedCORE's GitOps workflow.

---

## Coordination Process

### Step 1: fedCORE Team Provides Variables

fedCORE team sends:
```
CLUSTER_ACCOUNT_ID: 123456789012
CLUSTER_NAME: fedcore-prod-use1
```

### Step 2: LZA Team Creates Bootstrap Role

LZA team:
- Creates `FedCoreBootstrapRole` in each tenant account
- Configures the trust policy to allow ACK controller roles from the cluster account

### Step 3: fedCORE Onboards Tenant

fedCORE performs all remaining steps automatically:

1. Creates `IAMRoleSelector` CRD for the tenant (cluster-scoped)
2. Bootstraps IAM in the tenant account (creates `TenantMaxPermissions` policy and `fedcore-ack-provisioner` role via `FedCoreBootstrapRole`)
3. Provisions application-level IAM roles and AWS resources via ACK

fedCORE creates TenantOnboarding CR:
```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: test-tenant
spec:
  tenantName: test-tenant
  aws:
    accountId: "999888777666"
  owners: [...]
  quotas: [...]
```

### Step 4: End-to-End Verification

Both teams verify:
- Tenant onboards successfully
- fedCORE bootstraps IAM resources in tenant account
- ACK can create resources in tenant account via IAMRoleSelector routing
- Applications can access AWS resources
- Permission boundary prevents privilege escalation

---

## LZA Configuration Template

### For AWS Landing Zone Accelerator Config

```yaml
# Example LZA configuration (adapt to your LZA config format)
customizations:
  iamRoles:
    - name: FedCoreBootstrapRole
      assumeRolePolicyDocument: policies/fedcore-bootstrap-trust-policy.json
      policies:
        - inline:
            name: AllowIAMBootstrap
            policy: policies/fedcore-bootstrap-permissions.json
```

---

## Appendix: Complete Example

### Scenario
- Tenant: `acme`
- Tenant Account ID: `987654321012`
- Cluster Account ID: `123456789012`
- Cluster Name: `fedcore-prod-use1`

### LZA Creates

**Role ARN**: `arn:aws:iam::987654321012:role/FedCoreBootstrapRole`

**Role Trust Policy**:
```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {
      "AWS": "arn:aws:iam::123456789012:root"
    },
    "Action": "sts:AssumeRole",
    "Condition": {
      "StringLike": {
        "aws:PrincipalArn": "arn:aws:iam::123456789012:role/fedcore-prod-use1-ack-*-controller"
      }
    }
  }]
}
```

### fedCORE Bootstraps (Automatically via FedCoreBootstrapRole)

**In Tenant Account** (`987654321012`):
- `arn:aws:iam::987654321012:policy/TenantMaxPermissions` (permission boundary)
- `arn:aws:iam::987654321012:role/fedcore-ack-provisioner` (ACK provisioner role)

### fedCORE Creates (Automatically via ACK + IAMRoleSelector)

**IAMRoleSelector** (cluster-scoped):
```yaml
apiVersion: services.k8s.aws/v1alpha1
kind: IAMRoleSelector
metadata:
  name: acme
spec:
  targetAccountId: "987654321012"
  roleARN: "arn:aws:iam::987654321012:role/fedcore-ack-provisioner"
```

**Cluster Account**:
- `arn:aws:iam::123456789012:role/fedcore-prod-use1-acme-deployer-a7b3c9d2`
- `arn:aws:iam::123456789012:role/fedcore-prod-use1-photo-gallery-b4f7e1a3`

**Tenant Account**:
- `arn:aws:iam::987654321012:role/acme-deployer-a7b3c9d2` (with TenantMaxPermissions boundary)
- `arn:aws:iam::987654321012:role/photo-gallery-b4f7e1a3` (with TenantMaxPermissions boundary)

---

## Related Documentation

**IAM & Authentication:**
- [IAM Architecture](IAM_ARCHITECTURE.md) - Three-tier IAM role model
- [Pod Identity Full Guide](POD_IDENTITY_FULL.md) - EKS Pod Identity implementation details
- [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md) - Complete implementation guide

**For LZA Team:**
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Overall design and LZA integration
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) - Operational procedures
- [Glossary](GLOSSARY.md) - fedCORE and AWS terminology

**Troubleshooting:**
- [Troubleshooting Guide](TROUBLESHOOTING.md) - IAM and cross-account issues

---

## Navigation

[← Previous: Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) | [Next: Pod Identity →](POD_IDENTITY_FULL.md)

**Handbook Progress:** Page 31 of 35 | **Level 6:** IAM & Multi-Account Architecture

[Back to Handbook](HANDBOOK_INTRO.md) | [Glossary](GLOSSARY.md) | [Troubleshooting](TROUBLESHOOTING.md)

---

**Document Version**: 2.0
**Last Updated**: 2026-04-03
**Next Review**: After first production tenant onboarding
