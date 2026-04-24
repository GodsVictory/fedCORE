# Audit Service Account Component

## Overview

This component creates a Kubernetes ServiceAccount with read-only access to all cluster objects for audit and monitoring purposes.

## Features

- **Read-Only Access**: ClusterRole with `get`, `list`, and `watch` permissions on all Kubernetes resources
- **Comprehensive Coverage**: Access to core resources, custom resources, and all API groups
- **AWS Integration**: Configured with EKS Pod Identity (IRSA) for secure AWS service access
- **Namespace Isolation**: Deployed in dedicated `audit-system` namespace

## Components Created

1. **Namespace**: `audit-system`
2. **ServiceAccount**: `audit-service-account` in `audit-system` namespace
3. **Secret**: `audit-service-account-token` - Long-lived token for the service account
4. **ClusterRole**: `audit-service-account-readonly` with read-only permissions
5. **ClusterRoleBinding**: Binds the ClusterRole to the ServiceAccount

## AWS Configuration

For AWS EKS clusters, the component automatically configures Pod Identity annotation:

```yaml
annotations:
  eks.amazonaws.com/role-arn: arn:aws:iam::<account-id>:role/<cluster-name>-audit-service-account
```

The IAM role should be created separately with appropriate AWS service permissions for audit operations.

## Permissions

The service account has read-only access to:

- **Core Resources**: Pods, Services, ConfigMaps, Secrets, Nodes, etc.
- **Workload Resources**: Deployments, StatefulSets, DaemonSets, Jobs, CronJobs
- **RBAC Resources**: Roles, RoleBindings, ClusterRoles, ClusterRoleBindings
- **Network Resources**: Ingresses, NetworkPolicies
- **Storage Resources**: PersistentVolumes, PersistentVolumeClaims, StorageClasses
- **Custom Resources**: All CRDs and custom resources
- **Cluster Metadata**: Events, Metrics, API Services

## Usage

### Retrieving the Service Account Token

The component automatically creates a long-lived token for the service account:

```bash
# Get the token
kubectl get secret audit-service-account-token -n audit-system -o jsonpath='{.data.token}' | base64 -d

# Get the CA certificate
kubectl get secret audit-service-account-token -n audit-system -o jsonpath='{.data.ca\.crt}' | base64 -d

# Use the token to authenticate
TOKEN=$(kubectl get secret audit-service-account-token -n audit-system -o jsonpath='{.data.token}' | base64 -d)
kubectl --token=$TOKEN get pods --all-namespaces
```

### Using the ServiceAccount in a Pod

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: audit-tool
  namespace: audit-system
spec:
  serviceAccountName: audit-service-account
  containers:
  - name: audit-container
    image: your-audit-tool:latest
    # Your container configuration
```

### Testing Access

```bash
# Test access to cluster resources
kubectl auth can-i get pods --as=system:serviceaccount:audit-system:audit-service-account
kubectl auth can-i list deployments --as=system:serviceaccount:audit-system:audit-service-account
kubectl auth can-i delete pods --as=system:serviceaccount:audit-system:audit-service-account  # Should return 'no'
```

## Security Considerations

1. **Read-Only**: This service account has NO write, update, or delete permissions
2. **Secrets Access**: The account can read secrets - ensure proper secret management practices
3. **IAM Role**: Configure the associated AWS IAM role with least-privilege permissions
4. **Audit Logging**: Monitor usage of this service account through cluster audit logs

## Cloud Support

- **AWS**: Full support with EKS Pod Identity (IRSA)
- **Azure**: Can be extended with Azure Workload Identity
- **On-Premises**: Works without cloud provider annotations

## Dependencies

None - this is a standalone component.

## Configuration

No additional configuration required. The component uses cluster-level data values:

- `data.values.cluster_name`: Cluster name for resource labeling
- `data.values.aws.account_id`: AWS account ID for IAM role ARN (AWS only)
