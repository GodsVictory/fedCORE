# Why the CI/CD Deployer Role Has Zero AWS Permissions

## The Critical Insight

**CI/CD pipelines run `kubectl apply`, not AWS API calls.**

Therefore, the deployer role needs **Kubernetes RBAC**, not AWS IAM permissions.

---

## The Flow Explained

### What Actually Happens in CI/CD

```bash
# GitHub Actions Workflow
name: Deploy Application
on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Configure kubectl
        uses: azure/k8s-set-context@v3
        with:
          kubeconfig: ${{ secrets.KUBECONFIG }}

      - name: Deploy
        run: kubectl apply -f manifests/

      # No AWS CLI commands! ←  KEY INSIGHT
```

**At no point does GitHub Actions call AWS APIs.**

---

## The Three Actors

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. GitHub Actions Runner                                       │
│    • Runs: kubectl apply -f webapp.yaml                        │
│    • Authenticates to: Kubernetes API server                   │
│    • Uses: ServiceAccount token (Kubernetes credential)        │
│    • Does NOT call: AWS APIs                                   │
└─────────────────────────────────────────────────────────────────┘
                               │
                               │ Creates Kubernetes resource
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. ACK Controllers (Running in Cluster)                        │
│    • Watches: Kubernetes resources (S3 Bucket, DynamoDB Table) │
│    • Authenticates to: AWS via Pod Identity                    │
│    • Uses: fedcore-ack-provisioner role                        │
│    • Calls: AWS APIs (CreateBucket, CreateTable)               │
└─────────────────────────────────────────────────────────────────┘
                               │
                               │ Creates AWS resource
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. Application Pod (Running in Cluster)                        │
│    • Runs: Application code                                    │
│    • Authenticates to: AWS via Pod Identity                    │
│    • Uses: App-specific role (photo-gallery-b4f7e1a3)          │
│    • Calls: AWS APIs (s3:PutObject, s3:GetObject)              │
└─────────────────────────────────────────────────────────────────┘
```

**Notice**: GitHub Actions never touches AWS. It only creates Kubernetes resources.

---

## Validation: Kubernetes Status vs AWS APIs

### ❌ Wrong: CI/CD Calls AWS APIs

```bash
# DON'T DO THIS
- name: Deploy and Validate
  run: |
    kubectl apply -f manifests/

    # Wait for ACK to create bucket
    sleep 30

    # Call AWS API to validate (REQUIRES AWS PERMISSIONS)
    aws s3api head-bucket --bucket my-app-data
```

**Problems**:
- Requires AWS IAM permissions on deployer role
- Adds unnecessary complexity
- Creates a security risk (broader permissions than needed)

### ✅ Right: CI/CD Checks Kubernetes Status

```bash
# DO THIS INSTEAD
- name: Deploy and Validate
  run: |
    kubectl apply -f manifests/

    # Wait for ACK to sync resource
    kubectl wait --for=condition=ACK.ResourceSynced bucket/my-app-data --timeout=5m

    # Check Kubernetes status (NO AWS PERMISSIONS NEEDED)
    BUCKET_NAME=$(kubectl get bucket my-app-data -o jsonpath='{.status.bucketName}')
    echo "Bucket created: $BUCKET_NAME"
```

**Benefits**:
- No AWS permissions needed
- Faster (no need to call AWS API)
- More reliable (Kubernetes is source of truth)
- Better security (zero trust AWS credentials)

---

## When Would CI/CD Need AWS Permissions?

### Scenario 1: Custom Migration Jobs (Rare)

```yaml
# This is NOT typical CI/CD - it's a custom operation
apiVersion: batch/v1
kind: Job
metadata:
  name: migrate-s3-data
  namespace: acme-cicd
spec:
  template:
    spec:
      serviceAccountName: acme-deployer  # ← Uses deployer role
      containers:
        - name: migrate
          image: amazon/aws-cli:latest
          command:
            - aws
            - s3
            - sync
            - s3://old-bucket/
            - s3://new-bucket/
```

**This is a Job running IN the cluster**, not GitHub Actions. If you need this, create a **separate ServiceAccount** with specific S3 permissions.

### Scenario 2: Smoke Tests (Also Rare)

```yaml
# Testing that app can access AWS
apiVersion: v1
kind: Pod
metadata:
  name: smoke-test
  namespace: acme-cicd
spec:
  serviceAccountName: acme-deployer
  containers:
    - name: test
      command: ["aws", "s3", "ls", "my-app-bucket"]
```

**Better approach**: Use the app's ServiceAccount for smoke tests, not the deployer SA.

---

## The Role Definition

```yaml
# platform/rgds/tenant/overlays/aws/overlay.yaml
- id: tenantDeployerRole
  spec:
    name: acme-deployer-a7b3c9d2
    description: "CI/CD role - Kubernetes operations only (no AWS permissions)"
    policies:
      - policyName: MinimalIdentityAccess
        policyDocument: |
          {
            "Statement": [{
              "Effect": "Allow",
              "Action": ["sts:GetCallerIdentity"],
              "Resource": "*"
            }]
          }
```

**Only `sts:GetCallerIdentity`** for debugging (to see which role you have). That's it!

---

## What About CloudWatch Logs?

You might think: "CI/CD needs to write logs to CloudWatch!"

**No, it doesn't.** CI/CD logs go to:
- GitHub Actions log output (free, included)
- Or Kubernetes logs (`kubectl logs`)
- Or application logs (via app pods)

CloudWatch Logs are for **application runtime logs**, not CI/CD logs.

---

## The Architecture Principle

```
┌────────────────────────────────────────────────────────────┐
│ SEPARATION OF CONCERNS                                     │
├────────────────────────────────────────────────────────────┤
│ CI/CD Layer      → Kubernetes Operations (RBAC)           │
│ Control Plane    → Infrastructure Creation (ACK)          │
│ Data Plane       → Application Runtime (App-specific IAM) │
└────────────────────────────────────────────────────────────┘
```

Each layer has credentials for **only what it needs**:
- CI/CD: Kubernetes ServiceAccount token
- ACK: `fedcore-ack-provisioner` IAM role
- Apps: App-specific IAM roles

No overlap. No shared credentials. Perfect isolation.

---

## Security Benefits

| Benefit | Impact |
|---------|--------|
| **Reduced Attack Surface** | Compromised CI/CD can't access AWS |
| **Zero Trust** | CI/CD has no AWS credentials to leak |
| **Least Privilege** | Each component has exactly what it needs |
| **Audit Clarity** | CloudTrail never shows deployer role (it never calls AWS!) |
| **Simplicity** | One less credential to manage |

---

## How to Extend (If You Really Need AWS Access)

If your team has a legitimate need for CI/CD to access AWS, create an **overlay**:

```yaml
# platform/clusters/fedcore-prod-use1/tenants/acme-cicd-aws-access.yaml
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Role
metadata:
  name: acme-cicd-aws-role
  namespace: acme-cicd
spec:
  name: acme-cicd-custom-aws-access
  policies:
    - policyName: SpecificS3Access
      policyDocument: |
        {
          "Statement": [{
            "Effect": "Allow",
            "Action": ["s3:ListBucket", "s3:GetObject"],
            "Resource": [
              "arn:aws:s3:::acme-migration-bucket",
              "arn:aws:s3:::acme-migration-bucket/*"
            ]
          }]
        }
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: acme-migration-sa
  namespace: acme-cicd
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::...:role/acme-cicd-custom-aws-access
```

Then use this **dedicated ServiceAccount** in your Job, not the deployer SA.

---

## Summary

**The deployer role has zero AWS permissions because CI/CD doesn't need them.**

- ✅ CI/CD deploys via `kubectl apply` (Kubernetes RBAC)
- ✅ ACK creates AWS resources (using ACK provisioner role)
- ✅ Apps access AWS (using app-specific roles)
- ✅ Validation happens via Kubernetes status (no AWS calls)

**Result**: Simpler, more secure, and follows the principle of least privilege.

---

**Last Updated**: 2026-02-18
**Applies to**: fedCORE v2.0+

---

## Navigation

[← Previous: Development Guide](DEVELOPMENT.md) | [Next: Troubleshooting →](TROUBLESHOOTING.md)

**Handbook Progress:** Page 20 of 35 | **Level 4:** Deployment & Development

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
**Related**: [IAM_ARCHITECTURE.md](./IAM_ARCHITECTURE.md)
