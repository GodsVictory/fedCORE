# Helm Charts OCI Registry Guide

Complete guide for mirroring Helm charts to your Nexus OCI registry for air-gapped deployments.

**Default OCI Registry:** `oci://registry.example.com/fedcore/helm-charts`

---

## Quick Start

### 1. Download Charts

```bash
fedcore helm-manage --dir ~/helm-charts
```

### 2. Push to Nexus

```bash
export NEXUS_OCI_URL="oci://registry.example.com/fedcore/helm-charts"
export NEXUS_USER="your-username"
export NEXUS_PASS="your-password"

fedcore helm-manage --dir ~/helm-charts --push
```

### 3. Enable in Cluster

```yaml
# platform/clusters/your-cluster/cluster.yaml
helm_repositories:
  use_mirror: true
  oci_registry: "nexus.example.com"
  chart_repo: "fedcore/helm-charts"
```

Deploy:

```bash
git add platform/clusters/your-cluster/values.yaml
git commit -m "Enable Helm OCI registry"
git push
```

---

## Required Charts (11 total)

| Chart | Version | Original Repository |
|-------|---------|---------------------|
| tetragon | 1.1.2 | https://helm.cilium.io/ |
| kyverno | 3.2.6 | https://kyverno.github.io/kyverno/ |
| base | 1.22.0 | https://istio-release.storage.googleapis.com/charts |
| istiod | 1.22.0 | https://istio-release.storage.googleapis.com/charts |
| ingress-nginx | 4.10.0 | https://kubernetes.github.io/ingress-nginx |
| splunk-connect-for-kubernetes | 1.4.12 | https://splunk.github.io/splunk-connect-for-kubernetes/ |
| capsule | 0.7.1 | https://projectcapsule.github.io/charts |
| kro | 0.1.0 | oci://ghcr.io/kro-run/charts |
| azure-service-operator* | 2.0.0 | https://raw.githubusercontent.com/Azure/azure-service-operator/main/helm-charts |
| s3-chart* | 1.0.0 | oci://public.ecr.aws/aws-controllers-k8s |
| iam-chart* | 1.0.0 | oci://public.ecr.aws/aws-controllers-k8s |

*Cloud-specific charts (ACK for AWS, ASO for Azure)

---

## Nexus Setup

### Create OCI Registry

1. Log into Nexus: `https://registry.example.com`
2. Settings → Repositories → Create repository
3. Select "docker (hosted)"
4. Configure:
   - Name: `fedcore/helm-charts`
   - Enable Docker V2 API
   - Storage: Select blob store (1+ GB recommended)
5. Save

### Configure Authentication

Create Kubernetes secrets in each namespace:

```bash
# For component namespaces
for ns in kube-system kyverno istio-system ingress-nginx splunk-system capsule-system flux-system; do
  kubectl create secret docker-registry nexus-registry-creds \
    --docker-server=registry.example.com \
    --docker-username=your-username \
    --docker-password=your-password \
    --namespace=$ns
done
```

---

## Downloading Charts

### Automated Download

```bash
fedcore helm-manage --dir ~/helm-charts
```

### Manual Download

```bash
# Add repositories
helm repo add cilium https://helm.cilium.io/
helm repo add kyverno https://kyverno.github.io/kyverno/
helm repo add istio https://istio-release.storage.googleapis.com/charts
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm repo add splunk https://splunk.github.io/splunk-connect-for-kubernetes/
helm repo add capsule https://projectcapsule.github.io/charts
helm repo add aso https://raw.githubusercontent.com/Azure/azure-service-operator/main/helm-charts
helm repo update

# Download charts
helm pull cilium/tetragon --version 1.1.2
helm pull kyverno/kyverno --version 3.2.6
helm pull istio/base --version 1.22.0
helm pull istio/istiod --version 1.22.0
helm pull ingress-nginx/ingress-nginx --version 4.10.0
helm pull splunk/splunk-connect-for-kubernetes --version 1.4.12
helm pull capsule/capsule --version 0.7.1
helm pull aso/azure-service-operator --version 2.0.0

# Download OCI charts
helm pull oci://ghcr.io/kro-run/charts/kro --version 0.1.0
helm pull oci://public.ecr.aws/aws-controllers-k8s/s3-chart --version 1.0.0
helm pull oci://public.ecr.aws/aws-controllers-k8s/iam-chart --version 1.0.0
```

---

## Pushing Charts to Nexus

### Automated Push

```bash
export NEXUS_OCI_URL="oci://registry.example.com/fedcore/helm-charts"
export NEXUS_USER="your-username"
export NEXUS_PASS="your-password"

fedcore helm-manage --dir ~/helm-charts --push
```

### Manual Push

```bash
# Login
helm registry login registry.example.com \
  --username your-username \
  --password your-password

# Push charts
helm push tetragon-1.1.2.tgz oci://registry.example.com/fedcore/helm-charts
helm push kyverno-3.2.6.tgz oci://registry.example.com/fedcore/helm-charts
helm push base-1.22.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push istiod-1.22.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push ingress-nginx-4.10.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push splunk-connect-for-kubernetes-1.4.12.tgz oci://registry.example.com/fedcore/helm-charts
helm push capsule-0.7.1.tgz oci://registry.example.com/fedcore/helm-charts
helm push kro-0.1.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push azure-service-operator-2.0.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push s3-chart-1.0.0.tgz oci://registry.example.com/fedcore/helm-charts
helm push iam-chart-1.0.0.tgz oci://registry.example.com/fedcore/helm-charts

# Logout
helm registry logout registry.example.com
```

---

## Cluster Configuration

### Standard Configuration

```yaml
# platform/clusters/your-cluster/cluster.yaml
helm_repositories:
  use_mirror: true
  oci_registry: "nexus.example.com"
  chart_repo: "fedcore/helm-charts"
  # Charts pulled from: oci://nexus.example.com/fedcore/helm-charts/{chart}:{version}
  # Images pulled from: nexus.example.com/{image-path}
```

### Custom Registry

```yaml
helm_repositories:
  use_mirror: true
  oci_registry: "custom-registry.example.com"
  chart_repo: "my-org/helm-charts"
```

### Use Upstream (Testing)

```yaml
helm_repositories:
  use_mirror: false
  # Charts pulled directly from upstream sourceRepo in each component.yaml
  # Images use chart defaults (public registries)
```

---

## Verification

### Verify Charts in Nexus

```bash
# List all charts
curl -u "$NEXUS_USER:$NEXUS_PASS" \
  https://registry.example.com/v2/_catalog

# List chart versions
curl -u "$NEXUS_USER:$NEXUS_PASS" \
  https://registry.example.com/v2/kyverno/tags/list

# Pull a chart to test
helm pull oci://registry.example.com/fedcore/helm-charts/kyverno --version 3.2.6
```

### Verify Flux CD

```bash
# Check HelmRepository status
flux get sources helm --all-namespaces

# Check HelmRelease status
flux get helmreleases --all-namespaces

# Force reconciliation
flux reconcile source helm helm-charts -n kyverno
flux reconcile helmrelease kyverno -n kyverno
```

### Verify Deployments

```bash
# Check all HelmReleases
kubectl get helmreleases -A

# Check component pods
kubectl get pods -n kyverno
kubectl get pods -n istio-system
kubectl get pods -n ingress-nginx
```

---

## Troubleshooting

### Chart Not Found

```bash
# 1. Verify chart exists
curl -u "$NEXUS_USER:$NEXUS_PASS" \
  https://registry.example.com/v2/kyverno/tags/list

# 2. Try pulling manually
helm pull oci://registry.example.com/fedcore/helm-charts/kyverno --version 3.2.6

# 3. Check HelmRepository URL
kubectl get helmrepository helm-charts -n kyverno -o yaml | grep url

# 4. Force Flux resync
flux reconcile source helm helm-charts -n kyverno
```

### Authentication Errors

```bash
# Test login
helm registry login registry.example.com \
  --username your-username \
  --password your-password

# Create/update secrets
kubectl delete secret nexus-registry-creds -n kyverno
kubectl create secret docker-registry nexus-registry-creds \
  --docker-server=registry.example.com \
  --docker-username=your-username \
  --docker-password=your-password \
  --namespace=kyverno
```

### Connectivity Issues

```bash
# Test from cluster
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl -v https://registry.example.com/v2/

# Check DNS
kubectl run -it --rm debug --image=busybox --restart=Never -- \
  nslookup registry.example.com
```

---

## Updating Charts

When new chart versions are available:

```bash
# 1. Download new version
helm pull cilium/tetragon --version 1.2.0

# 2. Push to registry
helm push tetragon-1.2.0.tgz oci://registry.example.com/fedcore/helm-charts

# 3. Update component version
# Edit platform/components/tetragon/base/tetragon.yaml
#   version: "1.2.0"

# 4. Commit and deploy
git add platform/components/tetragon/base/tetragon.yaml
git commit -m "Update tetragon to 1.2.0"
git push
```

---

## Architecture Details

### OCI Registry Layout

```
oci://registry.example.com/fedcore/helm-charts/
├── tetragon:1.1.2
├── kyverno:3.2.6
├── base:1.22.0
├── istiod:1.22.0
├── ingress-nginx:4.10.0
├── splunk-connect-for-kubernetes:1.4.12
├── capsule:0.7.1
├── kro:0.1.0
├── azure-service-operator:2.0.0
├── s3-chart:1.0.0
└── iam-chart:1.0.0
```

### Flux CD Integration

Each component defines an OCI HelmRepository:

```yaml
apiVersion: source.toolkit.fluxcd.io/v1
kind: HelmRepository
metadata:
  name: helm-charts
  namespace: kyverno
spec:
  interval: 24h
  type: oci
  url: oci://registry.example.com/fedcore/helm-charts
```

HelmRelease references the chart by name:

```yaml
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: kyverno
  namespace: kyverno
spec:
  chart:
    spec:
      chart: kyverno
      version: "3.2.6"
      sourceRef:
        kind: HelmRepository
        name: helm-charts
```

### Component Files

All components use OCI-only configuration:

- [platform/components/tetragon/base/tetragon.yaml](../platform/components/tetragon/base/tetragon.yaml)
- [platform/components/kyverno/base/kyverno.yaml](../platform/components/kyverno/base/kyverno.yaml)
- [platform/components/istio/base/istio.yaml](../platform/components/istio/base/istio.yaml)
- [platform/components/ingress-nginx/base/ingress-nginx.yaml](../platform/components/ingress-nginx/base/ingress-nginx.yaml)
- [platform/components/splunk-connect/base/splunk-connect.yaml](../platform/components/splunk-connect/base/splunk-connect.yaml)
- [platform/components/capsule/base/capsule.yaml](../platform/components/capsule/base/capsule.yaml)
- [platform/components/kro/base/kro-operator.yaml](../platform/components/kro/base/kro-operator.yaml)

---

## Security Considerations

1. **Authentication** - Always use authentication for OCI registry access
2. **TLS/HTTPS** - Use HTTPS for all registry connections
3. **RBAC** - Restrict push/pull permissions in Nexus
4. **Secrets Management** - Use Kubernetes secrets for credentials
5. **Network Policies** - Implement network policies to restrict access

---

## Storage Requirements

- **Total Charts:** 11
- **Compressed Size:** ~150-200 MB
- **Recommended Storage:** 1+ GB (for multiple versions + metadata)

---

## References

- [Flux CD OCI Repositories](https://fluxcd.io/docs/components/source/helmrepositories/#oci-repository)
- [Helm OCI Support](https://helm.sh/docs/topics/registries/)
- [Nexus Repository Manager](https://help.sonatype.com/repomanager3)
- [ytt Templating](https://carvel.dev/ytt/)

---

## Files Reference

- **Schema:** [platform/clusters/schema.yaml](../platform/clusters/schema.yaml)
- **Download Command:** `fedcore helm-manage --dir <path>`
- **Push Command:** `fedcore helm-manage --dir <path> --push`

---

## Navigation

[← Previous: Ingress Management](INGRESS_MANAGEMENT.md) | [Next: Tenant AppDynamics →](TENANT_APPDYNAMICS.md)

**Handbook Progress:** Page 34 of 35 | **Level 7:** Advanced Features

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
