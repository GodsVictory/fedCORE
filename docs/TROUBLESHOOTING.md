# Troubleshooting Guide

Comprehensive troubleshooting reference for the fedCORE Platform. Use the symptom index below to quickly find solutions.

---

## Quick Symptom Index

| Symptom | Section | Page |
|---------|---------|------|
| Pod has no AWS credentials | [Pod Identity Issues](#pod-identity-issues) | Below |
| AccessDenied when assuming role | [Cross-Account Access](#cross-account-access-issues) | Below |
| TenantOnboarding CR stuck | [Tenant Onboarding](#tenant-onboarding-issues) | Below |
| Resources created in wrong account | [Multi-Account Issues](#multi-account-issues) | Below |
| Flux not syncing OCI artifacts | [Deployment Issues](#deployment-and-flux-issues) | Below |
| Policy violation blocking deployment | [Kyverno Policy Issues](#kyverno-policy-issues) | Below |
| Build failures in CI/CD | [CI/CD Issues](#cicd-and-build-issues) | Below |
| Namespace creation denied | [Tenant RBAC Issues](#tenant-rbac-issues) | Below |

---

## Pod Identity Issues

### Issue: Pod Not Getting AWS Credentials

**Symptoms:**
- No `AWS_CONTAINER_*` environment variables in pod
- Application logs show "unable to locate credentials"
- AWS SDK errors: `NoCredentialProviders`

**Diagnostic Steps:**

```bash
# 1. Check Pod Identity Agent is running
kubectl get daemonset -n kube-system eks-pod-identity-agent

# Expected: DaemonSet shows DESIRED = CURRENT = READY

# 2. Check Pod Identity Association exists for the cluster
aws eks list-pod-identity-associations --cluster-name <cluster-name>

# 3. Check specific ServiceAccount has role-arn annotation
kubectl get sa -n <namespace> <service-account-name> -o yaml | grep role-arn

# Expected: eks.amazonaws.com/role-arn: arn:aws:iam::ACCOUNT:role/ROLE_NAME

# 4. Check pod environment variables
kubectl exec -n <namespace> <pod-name> -- env | grep AWS_CONTAINER

# Expected:
# AWS_CONTAINER_CREDENTIALS_FULL_URI=...
# AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE=...
```

**Common Fixes:**

1. **Pod Identity Agent not running:**
   ```bash
   # Restart the agent
   kubectl rollout restart daemonset -n kube-system eks-pod-identity-agent

   # Wait for rollout
   kubectl rollout status daemonset -n kube-system eks-pod-identity-agent
   ```

2. **ServiceAccount missing annotation:**
   ```bash
   # Add annotation manually (or via TenantOnboarding CR)
   kubectl annotate serviceaccount -n <namespace> <sa-name> \
     eks.amazonaws.com/role-arn=arn:aws:iam::<account-id>:role/<role-name>
   ```

3. **Pod not using the correct ServiceAccount:**
   ```bash
   # Check pod's serviceAccountName
   kubectl get pod -n <namespace> <pod-name> -o jsonpath='{.spec.serviceAccountName}'

   # Update deployment to use correct ServiceAccount
   kubectl patch deployment -n <namespace> <deployment-name> \
     -p '{"spec":{"template":{"spec":{"serviceAccountName":"<correct-sa>"}}}}'
   ```

4. **Restart pod to pick up new credentials:**
   ```bash
   kubectl rollout restart deployment -n <namespace> <deployment-name>
   ```

**Related Documentation:**
- [Pod Identity Full Guide](POD_IDENTITY_FULL.md)
- [IAM Architecture](IAM_ARCHITECTURE.md)

---

## Cross-Account Access Issues

### Issue: AccessDenied When Assuming Role Across Accounts

**Symptoms:**
```
AccessDenied: User: arn:aws:sts::123456789012:assumed-role/SOURCE_ROLE/...
is not authorized to perform: sts:AssumeRole on resource:
arn:aws:iam::987654321012:role/TARGET_ROLE
```

**Diagnostic Steps:**

```bash
# 1. Verify source role has sts:AssumeRole permission
aws iam get-role-policy \
  --role-name <source-role-name> \
  --policy-name <policy-name>

# Look for:
# {
#   "Effect": "Allow",
#   "Action": "sts:AssumeRole",
#   "Resource": "arn:aws:iam::*:role/<target-role-pattern>"
# }

# 2. Verify target role trusts source role
aws iam get-role \
  --role-name <target-role-name> \
  --profile <tenant-profile> \
  --query 'Role.AssumeRolePolicyDocument'

# Look for:
# {
#   "Effect": "Allow",
#   "Principal": {
#     "AWS": "arn:aws:iam::<source-account>:role/<source-role>"
#   },
#   "Action": "sts:AssumeRole",
#   "Condition": {
#     "StringEquals": {
#       "sts:ExternalId": "<external-id>"
#     }
#   }
# }

# 3. Test assume role manually
aws sts assume-role \
  --role-arn arn:aws:iam::<target-account>:role/<target-role> \
  --role-session-name test \
  --external-id <external-id>
```

**Common Fixes:**

1. **Source role missing sts:AssumeRole permission:**
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [{
       "Effect": "Allow",
       "Action": "sts:AssumeRole",
       "Resource": "arn:aws:iam::*:role/fedcore-*"
     }]
   }
   ```

2. **Target role trust policy missing source role:**
   - Update trust policy in target account to include source role ARN
   - Ensure external ID matches (e.g., `fedcore-ack-<tenant-name>`)

3. **External ID mismatch:**
   ```bash
   # Check external ID in use
   kubectl get roles.iam.services.k8s.aws -n <namespace> <role-name> -o yaml | grep externalID

   # Compare with target role trust policy
   aws iam get-role --role-name <target-role> --query 'Role.AssumeRolePolicyDocument.Statement[0].Condition.StringEquals."sts:ExternalId"'
   ```

**Related Documentation:**
- [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)
- [IAM Architecture](IAM_ARCHITECTURE.md)

---

## Tenant Onboarding Issues

### Issue: TenantOnboarding CR Stuck or Not Creating Resources

**Symptoms:**
- `kubectl get tenantonboarding <name>` shows no progress
- Expected resources (Capsule Tenant, namespaces, IAM roles) not created
- CR status shows error or no status

**Diagnostic Steps:**

```bash
# 1. Check if CR was applied to cluster
kubectl get tenantonboarding <tenant-name>

# 2. Describe CR for detailed status
kubectl describe tenantonboarding <tenant-name>

# Look for:
# - Status.Conditions
# - Events showing errors

# 3. Check Kro operator logs (processes TenantOnboarding RGD)
kubectl logs -n kro-system -l app=kro-controller --tail=100 --follow

# 4. Check ACK IAM controller logs (creates IAM roles)
kubectl logs -n ack-system -l k8s-app=ack-iam-controller --tail=100 --follow

# 5. Check Flux reconciliation (ensures artifact is deployed)
flux get ocirepositories -n flux-system
flux get kustomizations -n flux-system

# 6. Check GitHub Actions workflow status
# Navigate to: https://github.com/<org>/<repo>/actions
```

**Common Issues:**

1. **Build failure - check CI/CD:**
   ```bash
   # Check GitHub Actions logs for:
   # - yamllint errors
   # - ytt template errors
   # - Validation failures
   ```

2. **Flux not syncing from Nexus:**
   ```bash
   # Check OCI repository authentication
   kubectl get secret -n flux-system nexus-credentials

   # Check Flux can reach Nexus
   kubectl logs -n flux-system -l app=source-controller
   ```

3. **FedCoreBootstrapRole doesn't exist in tenant account:**
   - Verify LZA created the tenant account
   - Verify LZA provisioned the bootstrap role
   - See [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md)

4. **ACK can't assume FedCoreBootstrapRole:**
   ```bash
   # Check ACK controller role has AssumeRole permission
   aws iam get-role-policy \
     --role-name <ack-controller-role> \
     --policy-name AssumeFedCoreBootstrapRole

   # Test assume role
   aws sts assume-role \
     --role-arn arn:aws:iam::<tenant-account>:role/FedCoreBootstrapRole \
     --role-session-name test \
     --external-id fedcore-bootstrap
   ```

5. **Pod Identity not working:**
   - See [Pod Identity Issues](#pod-identity-issues) above

**Related Documentation:**
- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md)
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md)

---

## Multi-Account Issues

### Issue: Resources Created in Wrong AWS Account

**Symptoms:**
- S3 bucket appears in cluster account instead of tenant account
- RDS database created in wrong account
- IAM role created in cluster account

**Diagnostic Steps:**

```bash
# 1. Check ACK CRD has account-id annotation
kubectl get <resource-type>.ack.services.k8s.aws -n <namespace> <resource-name> -o yaml | grep -A3 annotations

# Expected:
# annotations:
#   services.k8s.aws/account-id: "987654321012"
#   services.k8s.aws/role-arn: arn:aws:iam::987654321012:role/fedcore-ack-provisioner
#   services.k8s.aws/external-id: fedcore-ack-<tenant-name>

# 2. Check Capsule Tenant has aws-account-id annotation
kubectl get tenant <tenant-name> -o jsonpath='{.metadata.annotations.platform\.fedcore\.io/aws-account-id}'

# 3. Check TenantOnboarding CR has correct account ID
kubectl get tenantonboarding <tenant-name> -o yaml | grep accountId
```

**Common Fixes:**

1. **Missing annotations on ACK resource:**
   ```yaml
   apiVersion: rds.services.k8s.aws/v1alpha1
   kind: DBInstance
   metadata:
     name: mydb
     annotations:
       services.k8s.aws/account-id: "987654321012"
       services.k8s.aws/role-arn: arn:aws:iam::987654321012:role/fedcore-ack-provisioner
       services.k8s.aws/external-id: fedcore-ack-acme
   ```

2. **TenantOnboarding CR missing AWS account ID:**
   ```yaml
   apiVersion: platform.fedcore.io/v1alpha1
   kind: TenantOnboarding
   metadata:
     name: acme
   spec:
     tenantName: acme
     aws:
       accountId: "987654321012"  # <- Must be specified
   ```

**Related Documentation:**
- [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md)

---

## Deployment and Flux Issues

### Issue: Flux Not Syncing OCI Artifacts

**Symptoms:**
- `flux get ocirepositories` shows "reconciliation failed"
- Changes pushed to git not appearing in cluster
- `flux get kustomizations` shows stale version

**Diagnostic Steps:**

```bash
# 1. Check OCI repository status
flux get ocirepositories -n flux-system

# 2. Check detailed error
flux logs -n flux-system --level=error

# 3. Check Nexus credentials secret exists
kubectl get secret -n flux-system nexus-credentials

# 4. Test Nexus connectivity from cluster
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl -v https://<nexus-host>/v2/

# 5. Check if artifact exists in Nexus
flux pull artifact oci://<nexus-host>/<repo>:<tag> --output /tmp/test
```

**Common Fixes:**

1. **Authentication failure:**
   ```bash
   # Verify Nexus credentials are correct
   kubectl get secret -n flux-system nexus-credentials -o yaml

   # Re-create secret if needed (get credentials from Vault)
   kubectl delete secret -n flux-system nexus-credentials
   kubectl create secret docker-registry nexus-credentials \
     --namespace flux-system \
     --docker-server=<nexus-host> \
     --docker-username=<username> \
     --docker-password=<password>
   ```

2. **Artifact not found:**
   ```bash
   # Check if build succeeded in GitHub Actions
   # Navigate to: https://github.com/<org>/<repo>/actions

   # Verify artifact was pushed
   # Check build-infra job logs for "flux push artifact" step
   ```

3. **Network policy blocking Flux:**
   ```bash
   # Check if Flux pods can reach Nexus
   kubectl exec -n flux-system <flux-pod> -- nslookup <nexus-host>

   # May need to add NetworkPolicy egress rule for Nexus
   ```

4. **Stale OCI repository cache:**
   ```bash
   # Force reconciliation
   flux reconcile ocirepository -n flux-system <repo-name>

   # Suspend and resume if needed
   flux suspend ocirepository -n flux-system <repo-name>
   flux resume ocirepository -n flux-system <repo-name>
   ```

**Related Documentation:**
- [Deployment Guide](DEPLOYMENT.md)
- [Environment Setup](ENVIRONMENT_SETUP.md)

### Issue: Flux Bootstrap Failures

**Symptoms:**
- `flux install` fails during GitHub Actions workflow
- Error: "context deadline exceeded"
- Error: "unable to install CRDs"

**Common Fixes:**

1. **"flux-system namespace already exists":**
   - This is normal - Flux is already installed
   - Workflow skips bootstrap and continues
   - No action needed

2. **"context deadline exceeded":**
   ```bash
   # Increase timeout
   flux install --timeout=10m

   # Check network connectivity to install source
   kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
     curl -v https://github.com/fluxcd/flux2/releases
   ```

3. **"unable to install CRDs":**
   ```bash
   # Check cluster has sufficient resources
   kubectl top nodes
   kubectl describe nodes

   # Verify network policy allows Flux to reach API server
   kubectl get networkpolicies -A
   ```

**Related Documentation:**
- [Deployment Guide](DEPLOYMENT.md)

---

## Kyverno Policy Issues

### Issue: Policy Violation Blocking Resource Creation

**Symptoms:**
- `kubectl apply` fails with "admission webhook denied the request"
- Error message references Kyverno policy
- Resource not created in cluster

**Common Policy Violations:**

#### 1. Disallowed Image Registry

**Error:**
```
admission webhook "validate.kyverno.svc" denied the request:
policy restrict-image-registries:
  image: docker.io/nginx:latest not from approved registry
```

**Fix:**
```yaml
# Use approved registry (e.g., Nexus mirror)
spec:
  containers:
  - name: app
    image: nexus.fedcore.io/nginx:latest  # ← Use approved registry
```

**Related:** [Kyverno Policies](KYVERNO_POLICIES.md), [Security Overview](SECURITY_OVERVIEW.md)

#### 2. Missing Resource Limits

**Error:**
```
admission webhook "validate.kyverno.svc" denied the request:
policy require-resource-limits:
  containers must have resource limits
```

**Fix:**
```yaml
spec:
  containers:
  - name: app
    resources:
      requests:
        cpu: "100m"
        memory: "128Mi"
      limits:
        cpu: "500m"
        memory: "512Mi"
```

#### 3. Running as Root

**Error:**
```
admission webhook "validate.kyverno.svc" denied the request:
policy restrict-privilege-escalation:
  containers must not run as root
```

**Fix:**
```yaml
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
  containers:
  - name: app
    securityContext:
      allowPrivilegeEscalation: false
      capabilities:
        drop: ["ALL"]
```

#### 4. Privileged Container

**Error:**
```
admission webhook "validate.kyverno.svc" denied the request:
policy restrict-privileged:
  privileged containers are not allowed
```

**Fix:**
```yaml
spec:
  containers:
  - name: app
    securityContext:
      privileged: false  # ← Must be false
```

**Requesting Exemption:**

For legitimate use cases requiring policy exemptions:

1. **Audit mode policies** - Violations logged but not blocked
2. **Enforce mode policies** - Must request exemption from platform team

```yaml
# Example: Exemption annotation (if approved)
metadata:
  annotations:
    policies.kyverno.io/scored: "false"
```

**Related Documentation:**
- [Kyverno Policies](KYVERNO_POLICIES.md)
- [Security Policy Reference](SECURITY_POLICY_REFERENCE.md)

---

## Tenant RBAC Issues

### Issue: Namespace Creation Denied

**Symptoms:**
- Tenant owner cannot create namespace
- Error: "forbidden: User cannot create resource "namespaces""
- kubectl auth check fails

**Diagnostic Steps:**

```bash
# 1. Check if user is a tenant owner
kubectl get tenant <tenant-name> -o yaml | grep -A5 owners

# 2. Check user's RBAC permissions
kubectl auth can-i create namespaces --as=<user-email>

# 3. Check Capsule TenantOwner role
kubectl get clusterrole capsule-namespace-provisioner -o yaml

# 4. Verify namespace follows naming convention
# Must be: <tenant-name>-<suffix>
```

**Common Fixes:**

1. **User not listed as tenant owner:**
   ```yaml
   # Update TenantOnboarding CR or Capsule Tenant
   spec:
     owners:
       - kind: User
         name: user@example.com
         apiGroup: rbac.authorization.k8s.io
   ```

2. **Namespace doesn't follow naming convention:**
   ```bash
   # Correct: acme-dev, acme-prod, acme-staging
   # Wrong: dev-acme, my-namespace

   kubectl create namespace acme-dev  # ← Follows pattern
   ```

3. **Tenant quota exceeded:**
   ```bash
   # Check current namespace count
   kubectl get namespaces -l capsule.clastix.io/tenant=<tenant-name> --no-headers | wc -l

   # Check tenant quota
   kubectl get tenant <tenant-name> -o jsonpath='{.spec.namespaceOptions.quota}'
   ```

**Related Documentation:**
- [Tenant User Guide](TENANT_USER_GUIDE.md)
- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md)

---

## CI/CD and Build Issues

### Issue: GitHub Actions Build Failures

**Symptoms:**
- Workflow fails at validation or build step
- yamllint errors
- ytt template errors

**Common Errors:**

#### 1. YAML Lint Failures

**Error:**
```
yamllint: line too long (120 > 100 characters)
```

**Fix:**
```yaml
# Break long lines
- name: my-very-long-resource-name-
    that-exceeds-the-limit
  value: some-value
```

#### 2. ytt Template Errors

**Error:**
```
ytt: undefined variable: cloud_region
```

**Fix:**
```yaml
# Ensure data values are defined in cluster.yaml
#@data/values
---
cloud_region: us-east-1
```

#### 3. Flux Push Artifact Failures

**Error:**
```
authentication required
```

**Fix:**
- Verify Vault credentials are correct
- Check Nexus OCI registry is accessible from GitHub runners
- Verify Nexus credentials have push permissions

```bash
# Test Nexus connectivity
curl -u username:password https://<nexus-host>/v2/
```

**Related Documentation:**
- [Development Guide](DEVELOPMENT.md)
- [Deployment Guide](DEPLOYMENT.md)

---

## Getting More Help

### Check Logs

**Kro Operator (RGD processing):**
```bash
kubectl logs -n kro-system -l app=kro-controller --tail=100 --follow
```

**ACK Controllers (AWS resource provisioning):**
```bash
# IAM controller
kubectl logs -n ack-system -l k8s-app=ack-iam-controller --tail=100 --follow

# RDS controller
kubectl logs -n ack-system -l k8s-app=ack-rds-controller --tail=100 --follow

# S3 controller
kubectl logs -n ack-system -l k8s-app=ack-s3-controller --tail=100 --follow
```

**Flux (GitOps deployment):**
```bash
# Source controller (OCI sync)
kubectl logs -n flux-system -l app=source-controller --tail=100 --follow

# Kustomize controller (manifest application)
kubectl logs -n flux-system -l app=kustomize-controller --tail=100 --follow
```

**Kyverno (Policy enforcement):**
```bash
kubectl logs -n kyverno -l app.kubernetes.io/name=kyverno --tail=100 --follow
```

**Capsule (Multi-tenancy):**
```bash
kubectl logs -n capsule-system -l app.kubernetes.io/name=capsule --tail=100 --follow
```

### Useful Commands

```bash
# Describe resource for events
kubectl describe <resource-type> <name> -n <namespace>

# Check all resources in namespace
kubectl get all -n <namespace>

# Check resource quotas
kubectl get resourcequota -n <namespace>

# Check network policies
kubectl get networkpolicies -n <namespace>

# Check Kyverno policy reports
kubectl get policyreport -n <namespace>

# Check admission webhooks
kubectl get validatingwebhookconfigurations
kubectl get mutatingwebhookconfigurations
```

### Platform Support

- **GitHub Issues:** File issues in the platform repository
- **GitHub Discussions:** For questions and general discussions
- **Documentation:** [Handbook Intro](HANDBOOK_INTRO.md)

### Related Documentation

- **Pod Identity:** [Pod Identity Full Guide](POD_IDENTITY_FULL.md)
- **Multi-Account:** [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md)
- **Security:** [Security Overview](SECURITY_OVERVIEW.md)
- **Deployment:** [Deployment Guide](DEPLOYMENT.md)
- **Tenant Management:** [Tenant User Guide](TENANT_USER_GUIDE.md)

---

## Navigation

[← Previous: CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md) | [Next: Security Overview →](SECURITY_OVERVIEW.md)

**Handbook Progress:** Page 20 of 35 | **Level 4:** Deployment & Development

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md)
