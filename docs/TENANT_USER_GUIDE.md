# Tenant User Guide

**Working Within Your Tenant**

This guide is for tenant owners and developers who need to create namespaces, deploy applications, and work within tenant boundaries on the fedCORE Platform.

---

## Tenant Self-Service: Creating Namespaces

### For Tenant Owners

Once you're a tenant owner, you can create namespaces directly with kubectl:

```bash
# Create a namespace (must follow pattern: <tenant-name>-*)
kubectl create namespace acme-frontend

# Verify you have admin access
kubectl auth can-i '*' '*' --namespace acme-frontend
# Should return: yes
```

### Namespace Naming Rules

Namespaces **must** follow the pattern: `<tenant-name>-*`

**Examples for tenant "acme":**
- ✅ `acme-frontend`
- ✅ `acme-api`
- ✅ `acme-staging`
- ❌ `frontend` (missing tenant prefix)
- ❌ `other-tenant-app` (wrong tenant prefix)

### What Happens Automatically

When you create a namespace, Kyverno automatically generates:

1. **ResourceQuota** - Per-namespace limits (within tenant aggregate quota)
2. **LimitRange** - Default resource requests/limits for containers
3. **NetworkPolicies**:
   - Default deny all ingress
   - Allow ingress from same tenant namespaces
   - Allow DNS egress
   - Allow internet egress (if enabled)

---

## Working Within Tenant Namespaces

### Deploying Workloads

Example deployment that passes all policies:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: webapp
  namespace: acme-frontend
  labels:
    app.kubernetes.io/name: webapp
    app.kubernetes.io/version: "1.2.3"
spec:
  replicas: 3
  selector:
    matchLabels:
      app: webapp
  template:
    metadata:
      labels:
        app: webapp
    spec:
      # Run as non-root (required)
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
        seccompProfile:
          type: RuntimeDefault

      containers:
        - name: webapp
          # Use approved registry (required)
          image: nexus.fedcore.io/tenant-acme/webapp:1.2.3

          ports:
            - containerPort: 8080

          # Resource requests/limits (required)
          resources:
            requests:
              cpu: "100m"
              memory: "128Mi"
            limits:
              cpu: "500m"
              memory: "512Mi"

          # Security context (required)
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop:
                - ALL
            readOnlyRootFilesystem: true

          # Health checks (recommended)
          livenessProbe:
            httpGet:
              path: /healthz
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10

          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5

          # Writable ephemeral storage
          volumeMounts:
            - name: tmp
              mountPath: /tmp

      volumes:
        - name: tmp
          emptyDir: {}
```

### Policy Violations

If your deployment violates a policy, you'll get a clear error:

```bash
kubectl apply -f deployment.yaml

Error from server: admission webhook "validate.kyverno.svc" denied the request:

policy Deployment/acme-frontend/webapp for resource violation:

restrict-tenant-image-registries:
  validate-image-registry: 'Images must be from approved registries. Allowed
    registries: - nexus.fedcore.io/tenant-'
```

---

## Network Policies

### Default Network Isolation

Every tenant namespace has these default policies:

1. **Deny All Ingress** - No traffic allowed in by default
2. **Allow Same Tenant** - Pods can talk to other pods in the same tenant
3. **Allow DNS** - DNS resolution always works
4. **Allow Internet Egress** - (if enabled for tenant)

### Allowing External Traffic

To expose a service to external traffic, create an Ingress:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: webapp-ingress
  namespace: acme-frontend
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - app.acme-corp.com
      secretName: webapp-tls
  rules:
    - host: app.acme-corp.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: webapp
                port:
                  number: 80
```

### Custom Network Policies

You can add more specific network policies, but they **cannot**:
- Allow access to other tenants' namespaces
- Allow access to system namespaces (kube-system, etc.)
- Bypass default deny-all rules

Example - Allow traffic from ingress controller:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-ingress-controller
  namespace: acme-frontend
spec:
  podSelector:
    matchLabels:
      app: webapp
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: ingress-nginx
```

---

## Resource Quotas

### Tenant-Level Quotas

Your tenant has aggregate quotas across **all namespaces**:

```bash
# View tenant quota
kubectl get tenant acme -o yaml | grep -A 10 resourceQuotas
```

Example output:
```yaml
resourceQuotas:
  items:
    - hard:
        limits.cpu: "100"
        limits.memory: "200Gi"
        requests.cpu: "100"
        requests.memory: "200Gi"
        requests.storage: "1Ti"
        persistentvolumeclaims: "50"
```

### Per-Namespace Quotas

Each namespace also has individual quotas (auto-generated):

```bash
# View namespace quota
kubectl get resourcequota -n acme-frontend
kubectl describe resourcequota default-resource-quota -n acme-frontend
```

### Viewing Current Usage

```bash
# See how much of your quota is used
kubectl get resourcequota -n acme-frontend
kubectl get resourcequota -n acme-api
kubectl get resourcequota -n acme-staging

# Total usage across all tenant namespaces
kubectl get tenant acme -o jsonpath='{.status.namespaces}'
```

---

## Security Policies

### What's Blocked (Hard Enforcement)

These will **reject** your deployment:

- ❌ Privileged containers
- ❌ Running as root (UID 0)
- ❌ Host namespaces (hostNetwork, hostPID, hostIPC)
- ❌ Host path volumes
- ❌ Host ports
- ❌ Unsafe capabilities (must drop ALL)
- ❌ Images from unapproved registries
- ❌ Missing resource requests/limits (in production)

### What's Recommended (Audit Only)

These generate warnings but don't block:

- ⚠️ Missing readiness probes
- ⚠️ Missing liveness probes
- ⚠️ Not using QoS Guaranteed (requests != limits)
- ⚠️ Missing standard labels
- ⚠️ No PodDisruptionBudget for HA workloads
- ⚠️ No HorizontalPodAutoscaler

### Checking Policy Reports

Kyverno generates reports for audit policies:

```bash
# View policy reports for your namespace
kubectl get policyreport -n acme-frontend

# Detailed report
kubectl describe policyreport -n acme-frontend
```

---

## Image Registries

### Approved Registries

**Production clusters:**
- `nexus.fedcore.io/tenant-<your-tenant>/`
- `nexus.fedcore.io/platform/`

**Lab/Dev clusters:**
- All production registries, plus:
- `docker.io/`
- `ghcr.io/`

### Pushing Images to Nexus

```bash
# Login to Nexus
docker login nexus.fedcore.io

# Tag your image
docker tag myapp:latest nexus.fedcore.io/tenant-acme/myapp:1.2.3

# Push
docker push nexus.fedcore.io/tenant-acme/myapp:1.2.3
```

### Image Tag Requirements

**Production:**
- ❌ `latest` tag is blocked
- ✅ Use semantic versions: `v1.2.3`, `1.2.3`

**Lab/Dev:**
- ✅ `latest` tag allowed for testing

---

## Storage

### Creating PersistentVolumeClaims

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: webapp-data
  namespace: acme-frontend
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: standard  # Use your cluster's storage class
  resources:
    requests:
      storage: 10Gi  # Max 50Gi without approval
```

### Large PVC Approval

PVCs larger than 50Gi require explicit approval:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: large-storage
  namespace: acme-frontend
  annotations:
    fedcore.io/large-pvc-approved: "true"  # Requires platform admin approval
spec:
  resources:
    requests:
      storage: 100Gi
```

---

## LoadBalancer Services

### Creating LoadBalancers

LoadBalancers require explicit approval (cost control):

```yaml
apiVersion: v1
kind: Service
metadata:
  name: webapp-lb
  namespace: acme-frontend
  annotations:
    fedcore.io/loadbalancer-approved: "true"  # Requires platform admin approval
spec:
  type: LoadBalancer
  selector:
    app: webapp
  ports:
    - port: 80
      targetPort: 8080
```

**Note:** Most workloads should use Ingress instead of LoadBalancer.

---

## Troubleshooting

### "Namespace quota exceeded"

```bash
# Check current namespace count
kubectl get tenant acme -o jsonpath='{.status.namespaces}' | jq length

# Check quota
kubectl get tenant acme -o jsonpath='{.spec.namespaceOptions.quota}'
```

**Solution:** Delete unused namespaces or request quota increase from platform team.

### "Resource quota exceeded"

```bash
# Check resource usage
kubectl get resourcequota -n acme-frontend -o yaml
```

**Solution:**
- Delete unused resources
- Reduce resource requests/limits
- Request quota increase from platform team

### "Image pull from unauthorized registry"

**Error:**
```
policy Deployment/acme-frontend/webapp for resource violation:
restrict-tenant-image-registries: Images must be from approved registries
```

**Solution:** Push your image to an approved registry:
```bash
docker tag myapp:latest nexus.fedcore.io/tenant-acme/myapp:1.0.0
docker push nexus.fedcore.io/tenant-acme/myapp:1.0.0
```

### "Pod failed security validation"

Common issues:
- Running as root → Set `securityContext.runAsNonRoot: true`
- Missing resource limits → Add `resources.requests` and `resources.limits`
- Privileged container → Remove `securityContext.privileged: true`
- Dangerous capabilities → Add `capabilities.drop: ["ALL"]`

### Viewing Policy Violations

```bash
# Get admission logs
kubectl get events -n acme-frontend --sort-by='.lastTimestamp'

# Get Kyverno policy reports
kubectl get policyreport -n acme-frontend -o yaml
```

---

## Managing Your Tenant

### Listing Your Namespaces

```bash
# As tenant owner, list all your namespaces
kubectl get namespaces -l capsule.clastix.io/tenant=acme
```

### Deleting Namespaces

```bash
# Tenant owners can delete their own namespaces
kubectl delete namespace acme-staging
```

### Viewing Tenant Status

```bash
# Check tenant details
kubectl describe tenant acme

# View tenant documentation
kubectl get configmap acme-tenant-info -n capsule-system -o yaml
```

### Requesting Quota Increases

Contact the platform team with:
1. Current quota usage (output of `kubectl get resourcequota`)
2. Reason for increase
3. Expected new workload requirements

---

## Best Practices

### 1. Resource Efficiency

- Set requests close to actual usage (not over-provisioned)
- Set limits to prevent runaway processes
- Use HorizontalPodAutoscaler for variable load

### 2. High Availability

- Run at least 3 replicas for production workloads
- Create PodDisruptionBudgets
- Use readiness/liveness probes
- Spread across zones with topology constraints

### 3. Security

- Always run as non-root user
- Use read-only root filesystems where possible
- Mount writable paths as emptyDir volumes
- Drop all capabilities unless specifically needed

### 4. Observability

- Add standard labels (`app.kubernetes.io/*`)
- Export metrics for Prometheus
- Use structured logging
- Enable distributed tracing

### 5. Cost Management

- Right-size resource requests
- Clean up unused resources
- Use pod priorities for non-critical workloads
- Monitor with cost allocation labels (auto-added)

---

## Environment-Specific Policies

### Production Clusters

- Image registry: **Enforce** (nexus.fedcore.io only)
- Latest tag: **Blocked**
- Resource limits: **Required**
- Seccomp: **Required**
- Security baseline: **Strict**

### Lab/Dev Clusters

- Image registry: **Audit** (allows docker.io for testing)
- Latest tag: **Allowed**
- Resource limits: **Recommended** (not enforced)
- Seccomp: **Recommended**
- Security baseline: **Relaxed**

---

## Getting Help

### Platform Team Contact

- GitHub Issues: File issues in the platform repository
- GitHub Discussions: For questions and general discussions
- Documentation: See [Handbook Introduction](HANDBOOK_INTRO.md)

### Common Requests

- Tenant quota increases
- LoadBalancer service approvals
- Large PVC approvals
- Custom policy exceptions
- Multi-cluster deployments

---

## Reference

### Useful Commands

```bash
# List all tenants
kubectl get tenants.capsule.clastix.io

# Get tenant details
kubectl describe tenant <tenant-name>

# List tenant namespaces
kubectl get ns -l capsule.clastix.io/tenant=<tenant-name>

# Check namespace resource usage
kubectl top pods -n <namespace>

# View Kyverno policies
kubectl get clusterpolicies

# View policy reports
kubectl get policyreports -A

# Test permissions
kubectl auth can-i create namespaces --as=user@example.com
```

---

## Related Documentation

- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - For platform administrators
- [Tenant Advanced Topics](TENANT_ADVANCED_TOPICS.md) - Cross-namespace communication, service mesh, GitOps
- [Security Overview](SECURITY_OVERVIEW.md) - Security model and policies
- [Kyverno Policies](KYVERNO_POLICIES.md) - Policy details and examples

---

## Navigation

[← Previous: Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) | [Next: Tenant Advanced Topics →](TENANT_ADVANCED_TOPICS.md)

**Handbook Progress:** Page 14 of 35 | **Level 3:** Tenant Management

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
