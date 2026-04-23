# Tenant Advanced Topics

**Advanced Tenant Capabilities**

This guide covers advanced topics for tenant users, including cross-namespace communication, service mesh integration, GitOps workflows, and automation patterns.

---

## Cross-Namespace Communication

To allow one of your apps to talk to another across tenant namespaces:

```yaml
# In acme-frontend namespace
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-from-api
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
              capsule.clastix.io/tenant: acme
          podSelector:
            matchLabels:
              app: api-server
```

**Key Points:**
- Use `namespaceSelector` with tenant label to allow same-tenant communication
- Use `podSelector` to restrict to specific pods
- Cannot allow traffic from other tenants (Kyverno will block)

### Service Discovery Across Namespaces

Services are accessible via DNS across namespaces in the same tenant:

```yaml
# In acme-frontend namespace, call service in acme-backend
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
  namespace: acme-frontend
data:
  API_URL: "http://api-server.acme-backend.svc.cluster.local:8080"
```

DNS format: `<service-name>.<namespace>.svc.cluster.local`

---

## Service Mesh Integration

If your cluster has Istio or Linkerd enabled, you can opt into the service mesh for enhanced security and observability.

### Enabling Istio for Your Tenant

**Option 1: Via TenantOnboarding CR (requires platform admin)**

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme
  settings:
    istio:
      enabled: true        # Enable Istio sidecar injection
      strictMTLS: true     # Enforce STRICT mTLS mode
```

**Option 2: Label your namespaces (if you're a tenant owner)**

```bash
kubectl label namespace acme-frontend istio-injection=enabled
kubectl label namespace acme-backend istio-injection=enabled
```

**What happens:**
- All new pods get an Envoy sidecar automatically injected
- Service-to-service traffic is encrypted with mTLS
- Request metrics and distributed tracing enabled
- Layer 7 authorization policies applied

### Istio AuthorizationPolicy for Tenant Isolation

Create fine-grained access control between your services:

```yaml
apiVersion: security.istio.io/v1beta1
kind: AuthorizationPolicy
metadata:
  name: tenant-isolation
  namespace: acme-frontend
spec:
  action: ALLOW
  rules:
    - from:
        - source:
            namespaces: ["acme-*"]
    - to:
        - operation:
            methods: ["GET", "POST"]
            paths: ["/api/*"]
```

**Best Practices:**
- Use STRICT mTLS mode in production
- Restrict source namespaces to same tenant
- Define explicit ALLOW rules (deny-by-default)
- Monitor Envoy access logs in Splunk

### Service Mesh Observability

**View service metrics:**
```bash
kubectl port-forward -n istio-system svc/kiali 20001:20001
# Open http://localhost:20001
```

**View distributed traces:**
```bash
kubectl port-forward -n istio-system svc/jaeger 16686:16686
# Open http://localhost:16686
```

**Query Envoy access logs in Splunk:**
```spl
index=k8s_fedcore_all namespace="acme-*" envoy
| table _time, source_principal, destination_principal, response_code, request_duration
```

**See:** [Runtime Security - Istio mTLS Architecture](RUNTIME_SECURITY.md#istio-mtls-architecture)

---

## GitOps with Tenant Namespaces

Use Flux or ArgoCD to manage your tenant namespaces declaratively.

### Flux Kustomization Example

```yaml
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: acme-frontend
  namespace: acme-frontend
spec:
  interval: 5m
  path: ./apps/frontend
  prune: true
  sourceRef:
    kind: GitRepository
    name: acme-apps
  targetNamespace: acme-frontend
```

### GitOps Repository Structure

```
acme-apps/
├── apps/
│   ├── frontend/
│   │   ├── kustomization.yaml
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   └── ingress.yaml
│   ├── backend/
│   │   ├── kustomization.yaml
│   │   ├── deployment.yaml
│   │   └── service.yaml
├── infrastructure/
│   ├── kustomization.yaml
│   └── namespace.yaml
└── clusters/
    ├── prod/
    │   └── kustomization.yaml
    └── staging/
        └── kustomization.yaml
```

### GitOps Best Practices

1. **Separate Repos for Tenant Apps**
   - Platform repo: Tenant definitions and platform components
   - Tenant repo: Application manifests

2. **Environment Overlays**
   - Base manifests in `apps/`
   - Environment-specific overlays in `clusters/`

3. **Automated Deployments**
   - CI/CD pipeline pushes to git
   - Flux automatically applies changes
   - No manual kubectl needed

4. **RBAC for GitOps**
   - CI/CD ServiceAccount has deployment permissions
   - Developers commit to git, don't need cluster access

---

## Advanced Automation with CI/CD

### Using ServiceAccount for CI/CD

If your tenant was created with KRO TenantOnboarding, you have a CI/CD ServiceAccount with Pod Identity:

```bash
# List CI/CD resources
kubectl get sa -n acme-cicd
kubectl get role -n acme-cicd
kubectl get rolebinding -n acme-cicd
```

### GitHub Actions Example

```yaml
name: Deploy to Production

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Configure kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: service-account
          k8s-url: ${{ secrets.KUBERNETES_URL }}
          k8s-secret: ${{ secrets.KUBERNETES_SA_TOKEN }}

      - name: Deploy with Kustomize
        run: |
          kubectl apply -k apps/frontend -n acme-frontend
```

### AWS Pod Identity for CI/CD

If using AWS multi-account architecture:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ci-cd-job
  namespace: acme-cicd
spec:
  serviceAccountName: acme-deployer  # Has Pod Identity annotation
  containers:
    - name: deploy
      image: bitnami/kubectl:latest
      command:
        - kubectl
        - apply
        - -k
        - ./apps/frontend
      env:
        # AWS credentials provided by Pod Identity
        - name: AWS_REGION
          value: us-east-1
```

**See:** [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md)

---

## Resource Management Patterns

### Horizontal Pod Autoscaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: webapp-hpa
  namespace: acme-frontend
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: webapp
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### Pod Disruption Budgets

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: webapp-pdb
  namespace: acme-frontend
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: webapp
```

### Vertical Pod Autoscaling

```yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: webapp-vpa
  namespace: acme-frontend
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: webapp
  updatePolicy:
    updateMode: "Auto"
  resourcePolicy:
    containerPolicies:
      - containerName: webapp
        minAllowed:
          cpu: 100m
          memory: 128Mi
        maxAllowed:
          cpu: 2
          memory: 2Gi
```

---

## Multi-Region Deployments

### Active-Active Pattern

Deploy the same application to multiple clusters:

```bash
# Deploy to US cluster
kubectl --context=fedcore-prod-use1 apply -k apps/frontend -n acme-frontend

# Deploy to EU cluster
kubectl --context=fedcore-prod-azeus apply -k apps/frontend -n acme-frontend
```

### Global Load Balancing

Use external DNS and traffic management:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: webapp-ingress
  namespace: acme-frontend
  annotations:
    external-dns.alpha.kubernetes.io/hostname: app.acme-corp.com
    external-dns.alpha.kubernetes.io/target: us-east-1-lb.example.com
spec:
  ingressClassName: nginx
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

---

## Advanced Security Patterns

### Secrets Management with External Secrets Operator

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: database-credentials
  namespace: acme-backend
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secrets-manager
    kind: SecretStore
  target:
    name: db-credentials
    creationPolicy: Owner
  data:
    - secretKey: username
      remoteRef:
        key: acme/prod/db-credentials
        property: username
    - secretKey: password
      remoteRef:
        key: acme/prod/db-credentials
        property: password
```

### Workload Identity for AWS Resources

Access AWS resources from your pods using Pod Identity:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: s3-uploader
  namespace: acme-backend
spec:
  template:
    spec:
      serviceAccountName: acme-s3-uploader  # Has Pod Identity annotation
      containers:
        - name: uploader
          image: nexus.fedcore.io/tenant-acme/uploader:v1.0.0
          env:
            - name: S3_BUCKET
              value: acme-data-prod-use1
          # AWS SDK automatically uses Pod Identity
```

### Network Security Policies

```yaml
# Allow traffic only from specific services
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: backend-isolation
  namespace: acme-backend
spec:
  podSelector:
    matchLabels:
      app: api-server
  policyTypes:
    - Ingress
  ingress:
    # Allow from frontend only
    - from:
        - namespaceSelector:
            matchLabels:
              capsule.clastix.io/tenant: acme
          podSelector:
            matchLabels:
              app: webapp
      ports:
        - protocol: TCP
          port: 8080
    # Allow from ingress controller
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
```

---

## Observability and Monitoring

### Prometheus ServiceMonitor

Expose custom metrics for Prometheus:

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: webapp-metrics
  namespace: acme-frontend
spec:
  selector:
    matchLabels:
      app: webapp
  endpoints:
    - port: metrics
      interval: 30s
      path: /metrics
```

### Grafana Dashboard

Create custom dashboards for your tenant:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: webapp-dashboard
  namespace: acme-frontend
  labels:
    grafana_dashboard: "1"
data:
  webapp-dashboard.json: |
    {
      "dashboard": {
        "title": "ACME WebApp Dashboard",
        "panels": [...]
      }
    }
```

### Distributed Tracing with OpenTelemetry

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: webapp
  namespace: acme-frontend
spec:
  template:
    spec:
      containers:
        - name: webapp
          image: nexus.fedcore.io/tenant-acme/webapp:v1.0.0
          env:
            - name: OTEL_EXPORTER_OTLP_ENDPOINT
              value: "http://otel-collector.observability:4317"
            - name: OTEL_SERVICE_NAME
              value: "acme-frontend-webapp"
```

---

## Cost Optimization

### Pod Priority and Preemption

```yaml
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: acme-low-priority
value: 1000
globalDefault: false
description: "Low priority for batch jobs"

---
apiVersion: v1
kind: Pod
metadata:
  name: batch-job
  namespace: acme-backend
spec:
  priorityClassName: acme-low-priority
  containers:
    - name: batch
      image: nexus.fedcore.io/tenant-acme/batch:v1.0.0
```

### Resource Requests Optimization

Use VPA recommendations to right-size:

```bash
# View VPA recommendations
kubectl describe vpa webapp-vpa -n acme-frontend

# Apply recommendations
kubectl set resources deployment webapp -n acme-frontend \
  --requests=cpu=150m,memory=256Mi \
  --limits=cpu=500m,memory=512Mi
```

### Scheduled Scaling

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: scale-down-evening
  namespace: acme-frontend
spec:
  schedule: "0 20 * * 1-5"  # 8 PM weekdays
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: acme-deployer
          containers:
            - name: scale
              image: bitnami/kubectl:latest
              command:
                - kubectl
                - scale
                - deployment/webapp
                - --replicas=1
          restartPolicy: OnFailure
```

---

## Advanced Patterns with RGDs

### Creating Custom Resource Abstractions

Use Resource Graph Definitions (RGDs) to simplify complex deployments:

```yaml
apiVersion: example.org/v1
kind: WebApp
metadata:
  name: my-app
  namespace: acme-frontend
spec:
  image: nexus.fedcore.io/tenant-acme/webapp:v1.2.3
  replicas: 3
  storage:
    enabled: true
    size: 100Gi
  ingress:
    enabled: true
    hostname: my-app.acme-corp.com
```

**Platform automatically creates:**
- Deployment
- Service
- Ingress
- PVC
- HPA
- NetworkPolicies

**See:** [Platform RGDs Documentation](../platform/rgds/README.md)

---

## Related Documentation

- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Creating and managing tenants
- [Tenant User Guide](TENANT_USER_GUIDE.md) - Basic tenant operations
- [Security Overview](SECURITY_OVERVIEW.md) - Security architecture
- [Runtime Security](RUNTIME_SECURITY.md) - Istio mTLS and network security
- [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) - AWS multi-account workflows

---

## Navigation

[← Previous: Tenant User Guide](TENANT_USER_GUIDE.md) | [Next: Deployment Guide →](DEPLOYMENT.md)

**Handbook Progress:** Page 15 of 35 | **Level 3:** Tenant Management

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
