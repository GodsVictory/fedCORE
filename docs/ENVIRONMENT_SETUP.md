# GitHub Environment Setup

This document explains how to set up GitHub Environments for cluster deployments.

## Overview

The workflow uses GitHub Environments to store cluster-specific secrets. Each infrastructure artifact gets its own environment named after the matrix `target_name`.

## Required Environments

Based on the current cluster configuration, create the following GitHub Environments:

1. **platform-infra-aws-us-east-1-prod**
2. **platform-infra-azure-eastus-prod**
3. **platform-infra-onprem-lab-dc-dev**

## Creating Environments

### In GitHub UI

1. Navigate to your repository
2. Go to **Settings** → **Environments**
3. Click **New environment**
4. Name it exactly as the `target_name` from the matrix
5. Click **Configure environment**
6. Add the required secrets (see below)

### Via GitHub CLI

```bash
# Create environment
gh api repos/OWNER/REPO/environments/platform-infra-aws-us-east-1-prod -X PUT

# Add secrets (repeat for each secret)
gh secret set AWS_ACCESS_KEY_ID \
  --env platform-infra-aws-us-east-1-prod \
  --body "your-access-key-id"
```

## Required Secrets Per Environment

### AWS Clusters

For environments like `platform-infra-aws-us-east-1-prod`:

| Secret Name | Description | Example Value |
|-------------|-------------|---------------|
| `AWS_ACCESS_KEY_ID` | AWS access key for deployment | `AKIA...` |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key | `wJalr...` |
| `SPLUNK_HEC_HOST` | Splunk HEC endpoint for AWS region | `splunk-hec-aws.example.com` |
| `SPLUNK_HEC_TOKEN` | Splunk HEC authentication token | `00000000-0000-0000-0000-000000000000` |

**Note**: The `AWS_REGION` is automatically set from the matrix.

**Obtaining Splunk HEC Credentials:**
1. Contact your Splunk administrator
2. Request HEC token for the cluster (specify cluster name and region)
3. Token should have write access to indexes: `k8s_fedcore_*` or tenant-specific indexes
4. HEC endpoint varies by region (ask Splunk admin for correct endpoint)

### Azure Clusters

For environments like `platform-infra-azure-eastus-prod`:

| Secret Name | Description | Example Value |
|-------------|-------------|---------------|
| `AZURE_CLIENT_ID` | Service principal client ID | `00000000-0000-...` |
| `AZURE_CLIENT_SECRET` | Service principal secret | `abc123...` |
| `AZURE_TENANT_ID` | Azure AD tenant ID | `00000000-0000-...` |
| `AZURE_SUBSCRIPTION_ID` | Azure subscription ID | `00000000-0000-...` |
| `SPLUNK_HEC_HOST` | Splunk HEC endpoint for Azure region | `splunk-hec-azure.example.com` |
| `SPLUNK_HEC_TOKEN` | Splunk HEC authentication token | `11111111-1111-1111-1111-111111111111` |

### On-Premises Clusters

For environments like `platform-infra-onprem-lab-dc-dev`:

| Secret Name | Description | Example Value |
|-------------|-------------|---------------|
| `KUBECONFIG` | Base64-encoded kubeconfig | `YXBpVmVy...` |
| `SPLUNK_HEC_HOST` | Splunk HEC endpoint (internal) | `splunk-hec.internal.example.com` |
| `SPLUNK_HEC_TOKEN` | Splunk HEC authentication token | `22222222-2222-2222-2222-222222222222` |

**Generating KUBECONFIG secret**:
```bash
# Base64 encode your kubeconfig
cat ~/.kube/config | base64 -w 0 > kubeconfig.b64

# Add as secret
gh secret set KUBECONFIG \
  --env platform-infra-onprem-lab-dc-dev \
  --body-file kubeconfig.b64
```

## Environment Protection Rules (Optional)

For production environments, consider enabling:

1. **Required reviewers**: Require manual approval before deployment
2. **Wait timer**: Add a delay before deployment starts
3. **Deployment branches**: Limit to `main`/`master` only

### Example: Protecting Production

```yaml
# In GitHub UI: Settings → Environments → platform-infra-aws-us-east-1-prod

Required reviewers: @platform-team
Wait timer: 5 minutes
Deployment branches: Selected branches → main, master
```

## AWS Credentials Setup

### Option 1: IAM User (Simple)

1. Create IAM user for GitHub Actions
2. Attach policies: `AmazonEKSClusterPolicy`, `AmazonEKSWorkerNodePolicy`
3. Create access key
4. Add to GitHub environment secrets

### Option 2: Federated Identity (Recommended)

See [DEPLOYMENT.md](DEPLOYMENT.md) for Pod Identity setup details.

## Azure Credentials Setup

### Create Service Principal

```bash
# Create service principal
az ad sp create-for-rbac \
  --name github-actions-deploy \
  --role "Azure Kubernetes Service Cluster User Role" \
  --scopes /subscriptions/SUBSCRIPTION_ID/resourceGroups/RG_NAME

# Output will include:
# - appId (use as AZURE_CLIENT_ID)
# - password (use as AZURE_CLIENT_SECRET)
# - tenant (use as AZURE_TENANT_ID)
```

## On-Prem Kubeconfig Setup

### Create Service Account

```bash
# Create service account
kubectl create serviceaccount github-actions -n kube-system

# Create cluster role binding
kubectl create clusterrolebinding github-actions \
  --clusterrole=cluster-admin \
  --serviceaccount=kube-system:github-actions

# Get token (Kubernetes 1.24+)
kubectl create token github-actions -n kube-system --duration=87600h > /tmp/token

# Create kubeconfig
kubectl config view --minify --flatten > /tmp/kubeconfig
# Manually edit to use the service account token

# Base64 encode
cat /tmp/kubeconfig | base64 -w 0
```

## Verifying Environment Setup

After creating environments and secrets, verify:

```bash
# List environments
gh api repos/OWNER/REPO/environments | jq '.environments[].name'

# Check secrets in environment (names only, not values)
gh api repos/OWNER/REPO/environments/platform-infra-aws-us-east-1-prod/secrets | jq '.secrets[].name'
```

## Adding New Clusters

When adding a new cluster:

1. Create cluster directory: `platform/clusters/<cloud>/<region>/<env>/`
2. Add `cluster.yaml` with configuration
3. Matrix discovery will automatically find it
4. **Create matching GitHub Environment** with name: `platform-infra-<cloud>-<region>-<env>`
5. Add cloud-specific secrets to the environment
6. Commit and push to trigger workflow

## Troubleshooting

### Error: "Environment not found"

- Verify environment name exactly matches `matrix.target_name`
- Check in Settings → Environments

### Error: "Secret not found"

- Verify secret name matches expected name (case-sensitive)
- Check secret is added to the environment (not repository-level)

### Error: "Unauthorized" during kubectl commands

- For AWS: Verify IAM user has EKS permissions and cluster access configured
- For Azure: Verify service principal has AKS access
- For on-prem: Verify kubeconfig is valid and base64-encoded correctly

### Deployment Stuck "Waiting for approval"

- Environment has required reviewers enabled
- Someone with permission must approve the deployment

## Security Best Practices

1. **Use separate credentials per cluster** - Don't share AWS/Azure credentials across environments
2. **Rotate credentials regularly** - Especially on-prem kubeconfig tokens and Splunk HEC tokens (quarterly recommended)
3. **Enable environment protection for prod** - Require manual approval
4. **Audit secret access** - Monitor who accesses deployment secrets
5. **Use least privilege** - Grant minimal required permissions to service accounts
6. **Separate Splunk HEC tokens per cluster** - Don't reuse tokens across clusters for audit trail
7. **Never commit secrets to git** - Always use GitHub Environment secrets or Vault

## Splunk HEC Token Management

### Token Creation

Work with your Splunk administrator to create HEC tokens for each cluster:

1. **Request Format:**
   ```
   Cluster: fedcore-prod-use1
   Purpose: Kubernetes log ingestion
   Indexes: k8s_fedcore_* (or tenant-specific if using index-per-tenant strategy)
   Source Types: kube:container:logs, kube:metrics, kube:objects, tetragon:security
   ```

2. **Token Permissions:**
   - Write access to target indexes
   - No admin permissions needed
   - Consider read-only tokens for metrics collection if supported

3. **Token Rotation:**
   - Rotate quarterly or per organization policy
   - Update GitHub Environment secret
   - Redeploy infrastructure artifact (Flux will pick up new secret)

### Testing HEC Connectivity

After adding secrets, verify HEC connectivity:

```bash
# Test HEC endpoint is reachable (run from GitHub Actions runner or cluster node)
curl -k https://splunk-hec-aws.example.com:8088/services/collector/health

# Expected response:
# {"text":"HEC is healthy","code":200}

# Test authentication (replace with actual token)
curl -k https://splunk-hec-aws.example.com:8088/services/collector/event \
  -H "Authorization: Splunk 00000000-0000-0000-0000-000000000000" \
  -d '{"event":"test","sourcetype":"manual"}'

# Expected response:
# {"text":"Success","code":0}
```

### Troubleshooting HEC Issues

**"Connection refused" or timeout:**
- Verify HEC endpoint URL (check with Splunk admin)
- Check firewall rules allow outbound HTTPS from cluster nodes
- For on-prem, verify internal DNS resolves HEC endpoint

**"Invalid token" error:**
- Verify token is correct (no extra spaces)
- Verify token has not been disabled in Splunk
- Check token has write access to target indexes

**Logs not appearing in Splunk:**
- Check Splunk Connect DaemonSet logs: `kubectl logs -n splunk-system -l app=fluent-bit`
- Verify Fluent Bit is sending to correct HEC endpoint

---

## Navigation

[← Previous: Cluster Structure](CLUSTER_STRUCTURE.md) | [Next: Tenant Admin Guide →](TENANT_ADMIN_GUIDE.md)

**Handbook Progress:** Page 12 of 35 | **Level 2:** Platform Setup & Structure

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
- Check Splunk for rejected events (Splunk admin assistance needed)
