# Infrastructure Templates

This directory contains the core platform infrastructure components that are deployed to every cluster.

## Directory Structure

```
platform/components/
│
│  # Core Platform
├── kro/                    # Kubernetes Resource Orchestrator (RGD engine)
├── capsule/                # Multi-tenant namespace isolation
├── kyverno/                # Kyverno admission controller (operator)
├── kyverno-policies/       # Kyverno policy definitions (base + overlays)
├── cloud-permissions/      # Cloud IAM permission boundaries (overlays per cloud)
│
│  # Service Mesh & Ingress
├── istio/                  # Istio service mesh for mTLS encryption
├── istio-gateway/          # Istio ingress gateway
├── ingress-nginx/          # NGINX ingress controller
├── kong/                   # Kong API gateway
│
│  # AWS Controllers (ACK)
├── ack-iam-controller/     # IAM roles and policies
├── ack-s3-controller/      # S3 buckets
├── ack-rds-controller/     # RDS databases
├── ack-dynamodb-controller/# DynamoDB tables
├── ack-ec2-controller/     # EC2 resources
├── ack-ecr-controller/     # ECR repositories
├── ack-eks-controller/     # EKS clusters and access entries
├── ack-lambda-controller/  # Lambda functions
├── ack-sns-controller/     # SNS topics
├── ack-sqs-controller/     # SQS queues
├── ack-elasticache-controller/  # ElastiCache
├── ack-eventbridge-controller/  # EventBridge
├── ack-route53-controller/      # Route53 DNS
├── ack-secretsmanager-controller/ # Secrets Manager
├── ack-cloudwatchlogs-controller/ # CloudWatch Logs
├── ack-acm-controller/     # Certificate Manager
│
│  # Azure
├── azure-service-operator/ # Azure Service Operator (ASO)
│
│  # Observability & Security
├── splunk-connect/         # Splunk log forwarding
├── splunk-otel-collector/  # OpenTelemetry collector for Splunk
├── appdynamics-config/     # AppDynamics controller configuration
├── tetragon/               # eBPF runtime security observability
├── twistlock-defender/     # Prisma Cloud CNAPP
├── audit-service-account/  # Cluster audit service account
│
│  # Autoscaling & Operations
├── karpenter/              # Node autoscaling (AWS)
├── keda/                   # Event-driven pod autoscaling
├── velero/                 # Backup and disaster recovery
│
│  # UI & Tenant Management
├── headlamp/               # Kubernetes web UI dashboard
└── tenant-instances/       # Tenant onboarding CRs (from cluster.yaml tenants)
```

Each component follows the structure: `base/` for universal manifests, `overlays/{id}/` for cloud/environment-specific resources, and an optional `component.yaml` for Helm-based components. See [OVERLAY-SYSTEM.md](OVERLAY-SYSTEM.md) for the two-phase overlay system.

## Component Overlay Structure

Each infrastructure component can have its own cloud-specific and environment-specific overlays:

```
{component}/
├── base/                          # Base configuration (applies to all clusters)
└── overlays/
    ├── aws/                       # AWS-specific resources
    ├── azure/                     # Azure-specific resources
    ├── onprem/                    # On-premises-specific resources
    ├── prod/                      # Production environment
    ├── dev/                       # Development environment
    └── staging/                   # Staging environment
```

## Overlay Hierarchy

Overlays are applied in order of increasing specificity. Each layer can override or extend previous layers:

```
1. Component Base Templates
   ↓
2. Component Cloud Overlays ({component}/overlays/{cloud}/)
   ↓
3. Component Environment Overlays ({component}/overlays/{env}/)
   ↓
4. Cluster Overlays (platform/clusters/{cluster}/overlays/)
```

Each layer has **higher precedence** than the previous, allowing fine-grained control.

## When to Use Each Layer

### Base Templates
**Location:** `platform/components/{component}/base/`
**Scope:** All clusters, all environments, all clouds

Use for:
- Core platform components that apply universally
- Default configurations
- Universal policies

**Example:** Base Kyverno policies for image registries

### Component Cloud Overlays
**Location:** `platform/components/{component}/overlays/{cloud}/`
**Scope:** Cloud-specific for that component only

Use for:
- Component-specific cloud configurations
- Controllers that differ per cloud (ACK vs ASO)
- Cloud-specific resource definitions within a component

**Examples:**
- `ack-s3-controller/overlays/aws/` - ACK S3 controller for AWS
- `azure-service-operator/overlays/azure/` - ASO for Azure
- `kro/overlays/aws/` - AWS-specific KRO configurations

**Key benefit:** Each infrastructure component manages its own cloud-specific resources, keeping related configurations together.

### Component Environment Overlays
**Location:** `platform/components/{component}/overlays/{env}/`
**Scope:** Environment-specific for that component only

Use for:
- Environment-specific policies (strict in prod, relaxed in dev)
- Monitoring/alerting configurations per environment
- Component-specific resource quotas
- Environment-specific feature flags

**Examples:**
- `kyverno/overlays/prod/` - Production-specific policies:
  - `require-resource-limits.yaml` - Enforce CPU/memory limits
  - `disallow-latest-tag.yaml` - Require specific image versions

**Rationale:** Production demands predictability and safety. Environment-specific overlays allow strict controls in production while maintaining flexibility in development.

### Cluster Overlays
**Location:** `platform/clusters/{cluster}/overlays/`
**Scope:** Single specific cluster only

Use for:
- Cluster-unique configurations
- Special hardware (GPU nodes)
- Specific integrations
- One-off customizations

**Example:** `platform/clusters/fedcore-ml-gpu1/overlays/` for GPU-specific policies

## Examples

### Example 1: Add Policy to All Production Clusters

To add a policy that applies to **all production clusters** (regardless of cloud):

```yaml
# platform/components/kyverno/overlays/prod/kyverno-policies/require-probes.yaml
#@ load("@ytt:data", "data")
---
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-probes
  labels:
    platform.fedcore.io/environment: production
spec:
  validationFailureAction: Enforce
  rules:
  - name: check-probes
    match:
      any:
      - resources:
          kinds: [Pod]
    validate:
      message: "Production pods must define liveness and readiness probes"
      pattern:
        spec:
          containers:
          - livenessProbe: "?*"
            readinessProbe: "?*"
```

This will automatically apply to **all prod clusters** (AWS and Azure) without updating each cluster individually.

### Example 2: Add AWS-Specific Controller

To add a controller that only applies to **AWS clusters**:

```yaml
# platform/components/ack-rds-controller/overlays/aws/values.yaml
#@ load("@ytt:data", "data")
---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: ack-rds-controller
  namespace: ack-system
spec:
  chart:
    spec:
      chart: rds-chart
      version: "1.0.0"
      sourceRef:
        kind: HelmRepository
        name: aws-ack-charts
```

This will apply to **all AWS clusters** (prod and dev) but not Azure clusters.

### Example 3: Component-Specific Cross-Cutting Concern

If you need to add the same configuration to multiple components, add it to each component's overlay:

```bash
# Add strict resource limits to all production clusters for both Kyverno and KRO
platform/components/kyverno/overlays/prod/strict-limits.yaml
platform/components/kro/overlays/prod/strict-limits.yaml
```

This approach makes dependencies explicit and allows components to evolve independently.

## Verification

```bash
# Build a specific component for a cluster
fedcore build --artifact platform/components/kro --cluster platform/clusters/fedcore-prod-use1

# Build all components for a cluster
fedcore build --cluster platform/clusters/fedcore-prod-use1

# Generate bootstrap configuration
fedcore bootstrap --cluster platform/clusters/fedcore-prod-use1
```

The build script automatically applies overlays from the cluster's `overlays` array (e.g., `["aws", "prod"]`).

## Infrastructure Components

### cloud-permissions

**Purpose:** Foundational cloud permission controls to prevent tenant privilege escalation (cloud-specific overlays)

**AWS Implementation:**
- Tenant permission boundary IAM policy (via ACK)
- Namespace for IAM resources (`aws-iam-policies`)
- ConfigMap with policy ARN reference

**Azure Implementation:** TODO - Azure RBAC roles or Azure Policy assignments

**On-Premises:** No cloud-specific permission controls needed

**Why it matters:** Cloud permission boundaries are critical security controls that prevent tenant privilege escalation. For AWS, all tenant IAM roles reference this boundary, which denies:
- IAM policy/role modifications
- Credential harvesting
- Cross-account role assumption
- Access to billing/organizations

See [Cloud Permissions Documentation](cloud-permissions/README.md) for details.

### capsule

**Purpose:** Multi-tenant namespace isolation and resource quotas

**Creates:**
- Capsule Tenant CRD and operator
- Tenant-scoped network policies
- Namespace quota enforcement

### kro

**Purpose:** Kubernetes Resource Orchestrator for complex resource graphs

**Creates:**
- KRO operator
- ResourceGraphDefinition CRD
- Support for declarative resource composition

### kyverno

**Purpose:** Kyverno admission controller and policy engine operator

**Creates:**
- Kyverno admission controller (webhook-based validation/mutation)
- Background controller for policy reports
- Reports controller for violation tracking
- Cleanup controller for resource management

**Dependencies:** None (foundational component)

See [kyverno component documentation](kyverno/README.md) for details.

### kyverno-policies

**Purpose:** Policy definitions for tenant security and compliance

**Policies:**
- Image registry restrictions
- Security baselines (pod security standards)
- Network isolation
- Resource limit enforcement
- Tenant onboarding validation
- Cloud-specific policies (AWS ACK cross-account annotations)
- Environment-specific policies (production strictness)
- Istio service mesh security policies

**Dependencies:** Requires kyverno component to be installed first

See [kyverno-policies component documentation](kyverno-policies/README.md) for details.

### istio

**Purpose:** Service mesh providing service-to-service mTLS encryption and Layer 7 security

**Features:**
- Automatic mutual TLS (mTLS) between services
- Identity-based authorization policies
- Request-level observability and tracing
- Multi-tenant isolation at Layer 7
- STRICT mTLS mode enforced in production

**Cloud-Specific Overlays:**
- **AWS**: Network Load Balancer (NLB), EKS Pod Identity integration
- **Azure**: Azure Load Balancer Standard SKU, AKS Workload Identity
- **On-Prem**: MetalLB or NodePort for ingress gateway

**Dependencies:** Optional component - tenants opt-in via TenantOnboarding CR

See [istio component documentation](istio/README.md) for details.

### appdynamics-config

**Purpose:** Centralized AppDynamics controller configuration for tenant applications

**Features:**
- Shared AppDynamics controller credentials and endpoint configuration
- Automatic RBAC for all tenant ServiceAccounts
- Explicit NetworkPolicy egress rules for AppDynamics controller
- Independent of general internet egress settings
- Works in air-gapped and restricted network environments

**What It Provides:**
- `appdynamics-config` namespace with controller configuration secret
- ClusterRole + ClusterRoleBinding for tenant access
- NetworkPolicy allowing HTTPS (443) and HTTP (8090) egress to AppDynamics
- ConfigMap with endpoint information

**Cloud-Specific Overlays:**
- **AWS**: Integration with AWS Secrets Manager for credential management

**Dependencies:** None - standalone component

**Tenant Usage:** Reference `appdynamics-controller-config` secret from `appdynamics-config` namespace in application deployments

See [appdynamics-config component documentation](appdynamics-config/README.md) for details.

### twistlock-defender

**Purpose:** Comprehensive Cloud Native Application Protection Platform (CNAPP) providing runtime security, vulnerability management, and compliance monitoring

**Replaces:** Falco and Kyverno runtime policies with unified CNAPP solution

**Features:**
- **Runtime Protection**: Process, network, and filesystem monitoring with ML-based anomaly detection
- **Vulnerability Management**: Image scanning (ECR/ACR/Nexus), host scanning, and automated remediation tracking
- **Compliance Monitoring**: NIST 800-53 (default), PCI-DSS, HIPAA, CIS benchmarks
- **Admission Control**: Policy enforcement at deployment time (fail-closed)
- **Secrets Detection**: Static and runtime detection of hardcoded credentials
- **Cloud Integration**: Native integration with AWS GuardDuty, Azure Defender, Splunk SIEM

**Cloud-Specific Overlays:**
- **AWS**: Pod Identity monitoring, GuardDuty correlation, CloudTrail integration, ECR scanning
- **Azure**: Workload Identity monitoring, Defender for Cloud correlation, Activity Log integration, ACR scanning
- **On-Prem**: Splunk integration, local registry scanning (Nexus/Harbor), air-gapped support

**Dependencies:** Requires Prisma Cloud Console (SaaS or self-hosted)

See [twistlock-defender component documentation](twistlock-defender/README.md) for details.

### ingress-nginx

**Purpose:** NGINX-based ingress controller for HTTP/HTTPS routing

**Creates:**
- NGINX Ingress Controller
- Default IngressClass
- Cloud-specific load balancer configuration

See [ingress-nginx component documentation](ingress-nginx/README.md) for details.

### tetragon

**Purpose:** eBPF-based runtime security observability (process, network, file access monitoring)

**Creates:**
- Tetragon DaemonSet (eBPF agent)
- TracingPolicy CRDs for security monitoring
- Cloud-specific tracing policies (AWS credential access, Azure identity)

See [tetragon component documentation](tetragon/README.md) for details.

### tenant-instances

**Purpose:** Generates TenantOnboarding and NamespaceProvisioning CRs from the `tenants` array in cluster.yaml

**Creates:**
- TenantOnboarding CRs for `type: tenant` entries
- NamespaceProvisioning CRs for `type: namespace` entries
- Flux Kustomization with dependency ordering

**Dependencies:** Resolved via `overlay.yaml` (defaults to `depends_on: [namespace]`)

See [tenant-instances component documentation](tenant-instances/README.md) for details.

### ACK Controllers (AWS)

16 AWS Controllers for Kubernetes (ACK) components are available for managing AWS resources declaratively. Each follows the same pattern: Helm chart + IAM role overlay. See individual README files under `ack-*/`.

### headlamp

**Purpose:** Modern web-based Kubernetes dashboard for cluster management and observability

**Features:**
- Real-time cluster resource visualization
- Pod logs and shell access
- Resource editing and management (with appropriate RBAC)
- Multi-cluster support
- OIDC authentication support
- Cloud-native resource awareness (ACK, ASO, Capsule, Kyverno, Istio)
- Plugin architecture for customization

**Cloud-Specific Overlays:**
- **AWS**: NGINX/ALB ingress, optional AWS Cognito OIDC
- **Azure**: NGINX/AGIC ingress, optional Azure AD (Entra ID) OIDC
- **On-Prem**: NodePort/MetalLB, optional Keycloak OIDC

**RBAC:** Read-only cluster access by default (configurable per cluster)

**Access:** Exposed via ingress at `https://headlamp.{cluster-name}.{domain}/`

**Dependencies:** None - standalone component (ingress-nginx recommended)

See [headlamp component documentation](headlamp/README.md) for details.

## Creating a New Component

### 1. Directory Structure

```bash
mkdir -p platform/components/{name}/{base,overlays/aws}
```

```
platform/components/{name}/
├── component.yaml              # Helm chart config + values (ytt-templated, optional for plain manifests)
├── default-values.yaml         # Chart's default values.yaml (reference copy, not rendered)
├── base/
│   ├── namespace.yaml          # Namespace creation
│   └── {other-resources}.yaml  # Additional plain manifests (RBAC extensions, etc.)
└── overlays/
    ├── aws/                    # AWS-specific resources (IAM roles/policies via ACK)
    ├── azure/                  # Azure-specific resources
    └── onprem/                 # On-prem specific resources
```

### 2. component.yaml (Helm Type)

The component name is derived from the directory name. The type (helm vs plain manifests) is inferred from the presence of the `helm:` key.

```yaml
#@ load("@ytt:data", "data")
---
#! Helm chart configuration
helm:
  sourceRepo: {upstream-chart-repo}
  chart: {chart-name}
  version: "{version}"
  mirrorRepo: #@ data.values.helm_repositories.oci_registry_url if data.values.helm_repositories.use_mirror else "{upstream-chart-repo}"

  release:
    name: {release-name}
    namespace: {namespace}

  flags: ["--include-crds"]  # Extra flags passed to `helm template`

  values:
    #! Only set values that DIFFER from default-values.yaml
    #! Image overrides: hardcode to the mirror registry, don't use ytt templating
```

**Key rules:**
- Copy the chart's full `values.yaml` into `default-values.yaml` for reference
- Only override values in `component.yaml` that differ from chart defaults
- Hardcode image registries to the mirror (e.g., `registry.example.com/...`)
- Conditional monitoring: wrap with `#@ if data.values.monitoring.enabled:`

### 3. base/namespace.yaml

```yaml
#@ load("@ytt:data", "data")
---
apiVersion: v1
kind: Namespace
metadata:
  name: {namespace}
  labels:
    name: {namespace}
```

### 4. Pod Placement (Platform Components)

Platform components should run on managed nodes, not Karpenter nodes:

```yaml
nodeSelector:
  workload-type: platform

tolerations:
  - key: workload-type
    operator: Equal
    value: platform
    effect: NoSchedule

affinity:
  nodeAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      nodeSelectorTerms:
        - matchExpressions:
            - key: workload-type
              operator: In
              values:
                - platform
```

### 5. AWS IAM Roles (via ACK)

When a component needs AWS IAM permissions, create `overlays/aws/iam-role.yaml`:

```yaml
#@ load("@ytt:data", "data")
#@ load("@ytt:json", "json")

#@ region = data.values.region
#@ account_id = data.values.aws.account_id
#@ cluster = data.values.cluster_name

#! Use json.encode() with ytt functions for readable policy documents
#@ def my_policy():
#@   return {
#@     "Version": "2012-10-17",
#@     "Statement": [...]
#@   }
#@ end

---
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Role
metadata:
  name: #@ cluster + "-{component}-role"
  namespace: {namespace}
  labels:
    platform.fedcore.io/cluster: #@ cluster
    platform.fedcore.io/component: {component}
spec:
  name: #@ cluster + "-{ComponentRole}"
  assumeRolePolicyDocument: #@ json.encode({"Version": "2012-10-17", "Statement": [{"Effect": "Allow", "Principal": {"Service": "pods.eks.amazonaws.com"}, "Action": ["sts:AssumeRole", "sts:TagSession"]}]})
---
apiVersion: iam.services.k8s.aws/v1alpha1
kind: Policy
metadata:
  name: #@ cluster + "-{component}-policy"
  namespace: {namespace}
spec:
  name: #@ cluster + "-{ComponentPolicy}"
  policyDocument: #@ json.encode(my_policy())
```

**Best practices:**
- Scope permissions with tag conditions (e.g., `kubernetes.io/cluster/{name}`)
- Use `data.values.region` and `data.values.aws.account_id` for ARN construction
- Label everything with `platform.fedcore.io/cluster` and `platform.fedcore.io/component`
- Trust policy for Pod Identity uses principal `pods.eks.amazonaws.com`

### 6. RBAC Extensions

When a component introduces CRDs that tenants need access to, create aggregated ClusterRoles in `base/rbac-extensions.yaml`. These automatically extend the built-in admin/edit/view roles:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: {component}-admin-extension
  labels:
    rbac.authorization.k8s.io/aggregate-to-admin: "true"
rules:
- apiGroups: ["{api-group}"]
  resources: ["{crd-resources}"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
---
#! Edit: typically excludes cluster-scoped CRDs
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: {component}-edit-extension
  labels:
    rbac.authorization.k8s.io/aggregate-to-edit: "true"
rules: [...]
---
#! View: read-only
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: {component}-view-extension
  labels:
    rbac.authorization.k8s.io/aggregate-to-view: "true"
rules: [...]
```

### 7. Register in Cluster

Add to `platform/clusters/{cluster}/cluster.yaml` under `components:`:

```yaml
- name: {component-name}
```

Components are enabled by being listed — there is no `enabled` flag. `depends_on` is resolved automatically from the component's `overlay.yaml` (if present), or can be overridden in cluster.yaml.

### 8. Component Overlay (Optional)

If a component needs to inject data into the bootstrap process (e.g., setting `depends_on`), create an `overlay.yaml` at the component root:

```yaml
#@data/values
---
#@overlay/match missing_ok=True
components:
#@overlay/match by=lambda idx,old,new: old["name"] == "{component-name}"
- depends_on:
  - {dependency}
```

During `fedcore bootstrap`, the CLI collects `overlay.yaml` files from all listed components and includes them as ytt overlays. This allows components to declare their own dependencies without requiring users to specify them in cluster.yaml.

### Schema

Cluster data values schema is at `platform/clusters/schema.yaml`. Components receive `data.values` from the cluster's `cluster.yaml` merged with schema defaults. Available values include `cluster_name`, `cloud`, `region`, `environment`, `aws.account_id`, `monitoring.enabled`, and more.

## Related Documentation

- [Cluster Configuration Reference](../../platform/clusters/README.md)
- [Cluster Overlays Documentation](../../platform/clusters/README.md#cluster-specific-overlays)
- [Build Scripts](../../scripts/README.md)
