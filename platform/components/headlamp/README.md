# Headlamp - Kubernetes Web UI

Modern, lightweight web-based Kubernetes dashboard for cluster management and observability.

## Overview

Headlamp is an easy-to-use and extensible Kubernetes web UI that provides:
- Real-time cluster resource visualization
- Pod logs and shell access
- Resource editing and management
- Multi-cluster support
- Plugin architecture for customization
- OIDC authentication support

Headlamp is a modern alternative to the traditional Kubernetes Dashboard, with better UX and extensibility.

## Features

### Core Capabilities

- **Resource Management**: View, edit, and delete Kubernetes resources
- **Real-time Updates**: Live updates of cluster state
- **Pod Logs**: Stream and search container logs
- **Shell Access**: Execute commands in containers
- **Resource Metrics**: CPU and memory usage (requires metrics-server)
- **Multi-Namespace**: Browse across all namespaces
- **Search & Filter**: Quickly find resources
- **Dark Mode**: Built-in dark theme support

### Platform Integration

- **RBAC**: Read-only cluster access by default (configurable)
- **OIDC Authentication**: Optional SSO integration (AWS Cognito, Azure AD, Keycloak)
- **Cloud-Aware**: Understands cloud-native resources (ACK, ASO, Capsule, Kyverno, Istio)
- **Secure**: Runs with restricted security context

## Installation

Headlamp is deployed as a platform component:

```yaml
# In platform/clusters/{cluster-name}/cluster.yaml
components:
- name: headlamp
  enabled: true
  version: "1.0.0"
```

## Directory Structure

```
platform/components/headlamp/
├── base/
│   └── headlamp.yaml              # Base Headlamp installation
├── overlays/
│   ├── aws/
│   │   └── overlay.yaml           # AWS ingress (ALB/NLB/NGINX)
│   ├── azure/
│   │   └── overlay.yaml           # Azure ingress (AGIC/NGINX)
│   └── onprem/
│       └── overlay.yaml           # On-prem (NodePort/MetalLB)
└── README.md
```

## Configuration

### Base Configuration

**File**: [base/headlamp.yaml](base/headlamp.yaml)

- 2 replicas for high availability
- ClusterIP service (exposed via ingress)
- Read-only RBAC by default
- Restricted security context
- Pod anti-affinity across zones
- No authentication by default (protected via ingress)

### Cloud-Specific Overlays

**AWS** ([overlays/aws/overlay.yaml](overlays/aws/overlay.yaml)):
- Ingress with NGINX or ALB
- Hostname: `headlamp.{cluster-name}.{domain}`
- TLS termination
- Optional: AWS Cognito OIDC

**Azure** ([overlays/azure/overlay.yaml](overlays/azure/overlay.yaml)):
- Ingress with NGINX or AGIC (Application Gateway)
- Hostname: `headlamp.{cluster-name}.{domain}`
- TLS termination
- Optional: Azure AD (Entra ID) OIDC

**On-Prem** ([overlays/onprem/overlay.yaml](overlays/onprem/overlay.yaml)):
- NodePort or MetalLB
- Ingress with NGINX
- Optional: Keycloak or local OIDC

## Access

### Default Access

By default, Headlamp is accessible via ingress at:
```
https://headlamp.{cluster-name}.{domain}/
```

**Example URLs**:
- AWS Production: `https://headlamp.fedcore-prod-use1.fedcore.io/`
- Azure Staging: `https://headlamp.fedcore-staging-weu.fedcore.io/`
- On-Prem Dev: `https://headlamp.fedcore-dev-onprem.fedcore.local/`

### Port Forward (Development/Testing)

```bash
# Port forward to local machine
kubectl port-forward -n headlamp svc/headlamp 8080:80

# Access at http://localhost:8080
```

### Authentication

#### No Authentication (Default)
Headlamp runs without authentication by default. Access control should be handled at the ingress level:
- Network policies
- VPN/Bastion access
- IP allowlisting
- OAuth2 Proxy in front of ingress

#### OIDC Authentication (Recommended for Production)

Enable OIDC via cluster-specific overlay:

```yaml
# In platform/clusters/{cluster-name}/overlays/headlamp-oidc.yaml
#@ load("@ytt:data", "data")
#@ load("@ytt:overlay", "overlay")

#@overlay/match by=overlay.subset({"kind": "HelmRelease", "metadata": {"name": "headlamp"}})
---
spec:
  values:
    config:
      oidc:
        enabled: true
        clientID: "headlamp-client"
        clientSecret: "your-client-secret"  #! Store in sealed secret
        issuerURL: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_XXXXX"
        scopes: "openid profile email groups"
```

**OIDC Providers**:
- **AWS**: Cognito User Pools
- **Azure**: Azure AD (Entra ID)
- **On-Prem**: Keycloak, Dex, Okta

## RBAC Configuration

### Default: Read-Only Access

Headlamp uses a read-only ClusterRole by default:
- **Allowed**: `get`, `list`, `watch` on all resources
- **Denied**: `create`, `update`, `patch`, `delete`

This is suitable for most users who need visibility but not control.

### Custom RBAC: Write Access

To enable resource editing, create a more permissive ClusterRole:

```yaml
# In platform/clusters/{cluster-name}/overlays/headlamp-admin-rbac.yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: headlamp-admin
rules:
  - apiGroups: ["*"]
    resources: ["*"]
    verbs: ["*"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: headlamp-admin
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: headlamp-admin
subjects:
  - kind: ServiceAccount
    name: headlamp
    namespace: headlamp
```

**⚠️ Warning**: Admin access allows full cluster control. Use with caution and pair with strong authentication.

### Namespace-Scoped Access

For multi-tenant clusters, you can scope Headlamp to specific namespaces:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: headlamp-namespace-viewer
  namespace: acme-production
rules:
  - apiGroups: ["*"]
    resources: ["*"]
    verbs: ["get", "list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: headlamp-namespace-viewer
  namespace: acme-production
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: headlamp-namespace-viewer
subjects:
  - kind: ServiceAccount
    name: headlamp
    namespace: headlamp
```

## Platform Integration

### Capsule Multi-Tenancy

Headlamp automatically displays Capsule tenant resources:
- Tenant CRs
- Namespace quotas
- Tenant owner information

### Kyverno Policies

View Kyverno policies and violations:
- ClusterPolicy and Policy resources
- PolicyReport and ClusterPolicyReport
- Admission Review events

### Istio Service Mesh

Visualize Istio resources:
- VirtualServices
- Gateways
- DestinationRules
- AuthorizationPolicies
- PeerAuthentication

### Cloud Controllers

Monitor cloud-native resources:
- **AWS**: ACK resources (S3Bucket, IAMRole, DynamoDB)
- **Azure**: ASO resources (StorageAccount, KeyVault)

### Flux GitOps

Track Flux reconciliation:
- HelmRelease status
- Kustomization status
- GitRepository sync status
- Source status

## Use Cases

### 1. Cluster Health Monitoring

- View node status and resource usage
- Check pod health and restarts
- Monitor deployment rollout status
- Investigate pod logs for errors

### 2. Troubleshooting

- Stream real-time logs from containers
- Execute shell commands in pods
- Inspect resource configurations
- Check events for error messages

### 3. Resource Discovery

- Search for resources across namespaces
- View resource relationships (pods → deployments → replicasets)
- Inspect ConfigMaps and Secrets
- Browse CRDs and custom resources

### 4. Capacity Planning

- View resource requests/limits
- Check node capacity and allocatable resources
- Monitor PVC usage
- Identify resource bottlenecks

### 5. Security Auditing

- Review RBAC roles and bindings
- Check pod security contexts
- Inspect network policies
- View service account permissions

## Comparison with Kubernetes Dashboard

| Feature | Headlamp | Kubernetes Dashboard |
|---------|----------|---------------------|
| **UI/UX** | Modern, intuitive | Dated, complex |
| **Performance** | Fast, lightweight | Slower with large clusters |
| **Plugins** | ✅ Extensible | ❌ Limited |
| **Multi-Cluster** | ✅ Built-in | ❌ Requires workarounds |
| **OIDC** | ✅ Native | ✅ Via kubectl proxy |
| **Dark Mode** | ✅ Yes | ❌ No |
| **CRD Support** | ✅ Excellent | ⚠️ Limited |
| **Active Development** | ✅ Yes | ⚠️ Maintenance mode |

**Recommendation**: Use Headlamp for new deployments. Kubernetes Dashboard is no longer actively developed.

## Security Considerations

### 1. Network Access Control

**Best Practices**:
- Deploy Headlamp behind VPN or bastion host
- Use IP allowlisting on ingress
- Enable OIDC authentication for production
- Consider OAuth2 Proxy for additional auth layer

### 2. RBAC Permissions

**Default (Read-Only)**:
- ✅ Safe for most users
- ✅ Suitable for monitoring and troubleshooting
- ❌ Cannot modify resources

**Admin Access**:
- ⚠️ Use with caution
- ⚠️ Equivalent to cluster-admin
- ⚠️ Requires strong authentication

### 3. TLS/HTTPS

**Requirements**:
- Always use HTTPS in production
- Provision valid TLS certificates
- Enable forced HTTPS redirect

### 4. Audit Logging

Monitor Headlamp access:
```bash
# View audit logs for Headlamp service account
kubectl get events --all-namespaces --field-selector involvedObject.kind=ServiceAccount,involvedObject.name=headlamp
```

## Monitoring and Troubleshooting

### Health Check

```bash
# Check Headlamp pods
kubectl get pods -n headlamp

# Expected output:
# NAME                        READY   STATUS    RESTARTS   AGE
# headlamp-5d9c4f8b7d-abc12   1/1     Running   0          10m
# headlamp-5d9c4f8b7d-def34   1/1     Running   0          10m
```

### View Logs

```bash
# Stream Headlamp logs
kubectl logs -n headlamp -l app.kubernetes.io/name=headlamp -f

# Check for errors
kubectl logs -n headlamp -l app.kubernetes.io/name=headlamp --tail=100 | grep -i error
```

### Test Ingress

```bash
# Get ingress hostname
kubectl get ingress -n headlamp headlamp -o jsonpath='{.spec.rules[0].host}'

# Test connectivity
curl -k https://headlamp.fedcore-prod-use1.fedcore.io/ -I

# Expected: HTTP 200 OK
```

### Common Issues

#### Issue: 403 Forbidden

**Cause**: RBAC permissions too restrictive

**Solution**: Check ClusterRoleBinding:
```bash
kubectl get clusterrolebinding headlamp-readonly -o yaml
```

#### Issue: 404 Not Found

**Cause**: Ingress misconfiguration

**Solution**: Check ingress:
```bash
kubectl describe ingress -n headlamp headlamp
```

#### Issue: Slow Performance

**Cause**: Large cluster with many resources

**Solution**:
- Increase resource limits
- Enable metrics caching
- Use namespace filtering

## Customization

### Custom Branding

Add custom logo and colors via ConfigMap:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: headlamp-config
  namespace: headlamp
data:
  branding.json: |
    {
      "logo": "https://your-company.com/logo.png",
      "logoSmall": "https://your-company.com/logo-small.png",
      "primaryColor": "#0066cc",
      "appName": "FedCore Platform Dashboard"
    }
```

Mount this ConfigMap in the Headlamp deployment.

### Custom Plugins

Headlamp supports plugins for custom functionality:
- Custom resource views
- Additional actions
- Custom dashboards

See [Headlamp Plugin Documentation](https://headlamp.dev/docs/latest/development/plugins/) for details.

## Alternatives

### K9s
- **Type**: Terminal-based UI
- **Pros**: Lightweight, fast, keyboard-driven
- **Cons**: Not web-based, less accessible to non-technical users

### Lens
- **Type**: Desktop application
- **Pros**: Feature-rich, multi-cluster
- **Cons**: Requires installation, commercial license for teams

### Octant
- **Type**: Web UI
- **Pros**: Plugin architecture
- **Cons**: No longer actively maintained (archived)

**Recommendation**: Headlamp is the best web-based option for modern Kubernetes clusters.

## Related Documentation

- [Official Headlamp Documentation](https://headlamp.dev/docs/)
- [RBAC Guide](../../docs/RBAC_GUIDE.md)
- [Ingress Management](../../docs/INGRESS_MANAGEMENT.md)
- [OIDC Authentication Setup](../../docs/OIDC_AUTH.md)
- [Capsule Multi-Tenancy](../capsule/README.md)

## Best Practices

1. **Always use HTTPS** - Never expose Headlamp over plain HTTP
2. **Enable OIDC in production** - Don't rely on network-only access control
3. **Start with read-only RBAC** - Only grant write access when necessary
4. **Monitor access logs** - Track who accesses the dashboard
5. **Keep updated** - Regularly update Headlamp to latest stable version
6. **Use OAuth2 Proxy** - Add additional auth layer for sensitive clusters
7. **Limit network exposure** - Deploy behind VPN or private network
8. **Test RBAC changes** - Verify permissions before deploying to production

---

**Status:** ✅ Production ready
**Maintainer:** Platform Team
**Support:** [GitHub Issues](https://github.com/headlamp-k8s/headlamp/issues)
