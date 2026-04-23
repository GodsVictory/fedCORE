# Quick Start: Developer

**Time to complete:** 5 minutes

## What You'll Do

Deploy a sample web application with database using platform RGDs (Resource Graph Definitions) - no cloud-specific configuration required.

## Prerequisites

Before you begin:

1. **Tenant access** - Your platform administrator has created a tenant for you
2. **kubectl access** - Configured with credentials for your tenant namespace
3. **Namespace** - Create a namespace in your tenant (e.g., `acme-dev`, `acme-prod`)
4. **Basic Kubernetes knowledge** - Familiarity with kubectl and YAML manifests

**Verify your access:**

```bash
# Check your current context
kubectl config current-context

# List your accessible namespaces
kubectl get namespaces

# Create a namespace (if needed)
kubectl create namespace <tenant-name>-dev
```

## Step 1: Create WebApp RGD

Create a file named `webapp.yaml` with a WebApp RGD that provisions a complete application stack.

**Minimal example:**

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: WebApp
metadata:
  name: myapp
  namespace: acme-dev
spec:
  application:
    image: nginx:1.25
    replicas: 3

  database:
    enabled: true
    engine: postgres

  ingress:
    enabled: true
    host: myapp.acme-dev.example.com
```

**Complete examples in repository:**
- [Simple webapp with ingress](../platform/rgds/webapps/examples/webapp-with-ingress.yaml)
- [Webapp with dedicated gateway](../platform/rgds/webapps/examples/webapp-with-dedicated-gateway.yaml)
- [Webapp with Kubernetes ingress](../platform/rgds/webapps/examples/webapp-with-kubernetes-ingress.yaml)

**Full field reference:** [WebApp RGD schema](../platform/rgds/webapps/base/rgd.yaml)

**What this creates:**

- **Deployment** with your application replicas
- **Service** to expose your application
- **Ingress** for external access with TLS
- **Database** (RDS on AWS, Azure SQL on Azure, PostgreSQL on-prem)
- **Secret** with database credentials
- **ServiceAccount** with IAM role for database access (AWS/Azure)

## Step 2: Apply Manifest

Deploy your application:

```bash
# Apply the WebApp manifest
kubectl apply -f webapp.yaml

# Watch the deployment progress
kubectl get webapp myapp -n acme-dev --watch
```

**What happens next:**

1. Kro detects the WebApp custom resource
2. Kro generates Deployment, Service, Ingress resources
3. ACK/ASO controllers provision the database in your tenant's cloud account
4. Pod Identity associations grant pods access to the database
5. Application pods start and connect to the database

**Timing:** Database provisioning takes 5-10 minutes. Application pods start immediately.

## Step 3: Verify Deployment and Access

Check the status of all created resources:

```bash
# Check WebApp status
kubectl describe webapp myapp -n acme-dev

# Verify pods are running
kubectl get pods -n acme-dev -l app=myapp

# Check service endpoint
kubectl get service myapp -n acme-dev

# Verify ingress configuration
kubectl get ingress myapp -n acme-dev

# Check database status (AWS example)
kubectl get rds -n acme-dev

# View database credentials
kubectl get secret myapp-db-creds -n acme-dev -o jsonpath='{.data.connection-string}' | base64 -d
```

**Access your application:**

```bash
# Get the ingress URL
INGRESS_URL=$(kubectl get ingress myapp -n acme-dev -o jsonpath='{.spec.rules[0].host}')
echo "Application URL: https://$INGRESS_URL"

# Test the endpoint
curl https://$INGRESS_URL/health
```

**Expected output:**

```json
{
  "status": "healthy",
  "database": "connected",
  "version": "1.0.0"
}
```

## What You Created

Your WebApp RGD provisioned:

1. **Kubernetes Resources**
   - Deployment with 3 replicas
   - Service (ClusterIP)
   - Ingress with TLS termination
   - ServiceAccount for IAM authentication
   - ConfigMap for application configuration
   - Secret for database credentials

2. **Cloud Resources** (automatically selected based on cluster)
   - **AWS:** RDS PostgreSQL instance, IAM role, Security Group
   - **Azure:** Azure SQL Database, Managed Identity, NSG rules
   - **On-prem:** PostgreSQL StatefulSet, PersistentVolumeClaim

3. **Security Features**
   - TLS encryption for ingress traffic
   - Network policies for pod isolation
   - IAM roles with least-privilege permissions
   - Encrypted database storage
   - Automatic secret rotation (configurable)

4. **Monitoring & Observability**
   - Metrics exported to Prometheus
   - Logs forwarded to Splunk
   - AppDynamics agent injected (if enabled)
   - Health check endpoints configured

## Updating Your Application

To update your application, modify the manifest and reapply:

```bash
# Edit the manifest (e.g., change replica count)
vim webapp.yaml

# Apply changes
kubectl apply -f webapp.yaml

# Monitor rollout
kubectl rollout status deployment/myapp -n acme-dev
```

**Example: Scale to 5 replicas**

```yaml
spec:
  application:
    replicas: 5  # Changed from 3
```

**Example: Update image version**

```yaml
spec:
  application:
    image: nginx:1.26  # Changed from 1.25
```

## Deleting Your Application

To remove all resources:

```bash
# Delete the WebApp (cascades to all child resources)
kubectl delete webapp myapp -n acme-dev

# Verify resources are deleted
kubectl get all -n acme-dev -l app=myapp
```

**Note:** The database is deleted after a configurable retention period (default: 7 days). Backups are retained according to your organization's policy.

## Next Steps

Explore more platform capabilities:

- **[Tenant User Guide](TENANT_USER_GUIDE.md)** - Comprehensive self-service operations
- **[RGD Catalog](DEVELOPMENT.md#available-rgds)** - Browse available abstractions (Queue, Cache, Bucket, etc.)
- **[Security Policies](KYVERNO_POLICIES.md)** - Understanding policy enforcement
- **[Ingress Management](INGRESS_MANAGEMENT.md)** - Advanced routing and TLS configuration
- **[Troubleshooting Guide](TROUBLESHOOTING.md)** - Common issues and solutions

## Troubleshooting

### Pods not starting

Check pod events:
```bash
kubectl describe pod <pod-name> -n acme-dev
```

Common issues:
- **Image pull errors:** Verify image is in approved registry
- **Resource quota exceeded:** Contact tenant admin to increase quotas
- **Policy violations:** Check Kyverno policy reports

### Database connection failures

Verify database credentials:
```bash
kubectl get secret myapp-db-creds -n acme-dev -o yaml
```

Check Pod Identity association:
```bash
kubectl get podidentityassociation -n acme-dev
```

Test database connectivity from a pod:
```bash
kubectl exec -it <pod-name> -n acme-dev -- psql $DATABASE_URL -c "SELECT 1"
```

### Ingress not accessible

Check ingress status:
```bash
kubectl describe ingress myapp -n acme-dev
```

Verify DNS resolution:
```bash
nslookup myapp.acme-dev.example.com
```

Test from within the cluster:
```bash
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -- \
  curl http://myapp.acme-dev.svc.cluster.local
```

### Policy violations

View policy reports:
```bash
kubectl get policyreport -n acme-dev
kubectl describe policyreport <report-name> -n acme-dev
```

Common violations:
- Missing resource limits (audit only - warning)
- Unapproved image registry (enforce - blocks deployment)
- Missing security context (audit only - warning)

See [Kyverno Policies](KYVERNO_POLICIES.md) for full policy reference.

## Additional Examples

### DynamoDB Table

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: DynamoDB
metadata:
  name: myapp-data
  namespace: acme-dev
spec:
  tableName: myapp-data
  billingMode: PAY_PER_REQUEST
  hashKey: id
  rangeKey: timestamp
```

**Complete DynamoDB examples in repository:**
- [Basic table](../platform/rgds/dynamodb/examples/basic-table.yaml)
- [Advanced table with GSI](../platform/rgds/dynamodb/examples/advanced-table.yaml)
- [Comprehensive table](../platform/rgds/dynamodb/examples/comprehensive-table.yaml)
- [Provisioned capacity table](../platform/rgds/dynamodb/examples/provisioned-table.yaml)

**Full field reference:** [DynamoDB RGD schema](../platform/rgds/dynamodb/base/rgd.yaml)

### Istio Gateway

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: Gateway
metadata:
  name: myapp-gateway
  namespace: acme-dev
spec:
  hosts:
    - myapp.example.com
  tls:
    enabled: true
```

**Complete Gateway examples in repository:**
- [Basic gateway](../platform/rgds/gateway/examples/basic-gateway.yaml)
- [High-traffic gateway](../platform/rgds/gateway/examples/high-traffic-gateway.yaml)

**Full field reference:** [Gateway RGD schema](../platform/rgds/gateway/base/gateway-rgd.yaml)

---

## Navigation

[← Previous: Admin Quick Start](QUICKSTART_ADMIN.md) | [Next: Architect Quick Start →](QUICKSTART_ARCHITECT.md)

**Handbook Progress:** Page 6 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
