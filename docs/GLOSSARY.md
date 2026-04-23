# Glossary

Essential terminology reference for the fedCORE Platform. Bookmark this page for quick lookups while reading the handbook.

---

## Platform Components

### ACK (AWS Controllers for Kubernetes)
Kubernetes controllers that provision AWS resources directly from Kubernetes manifests. fedCORE uses ACK to create RDS databases, DynamoDB tables, S3 buckets, and other AWS resources from within the cluster.

**Example:** When a tenant creates a `Database` RGD, ACK provisions the actual RDS instance in their AWS account.

**Related:** [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)

### ASO (Azure Service Operator)
Similar to ACK but for Azure resources. Enables Kubernetes-native provisioning of Azure resources like CosmosDB, Storage Accounts, and App Services.

**Related:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

### Capsule
Multi-tenancy operator that provides tenant isolation within Kubernetes clusters. Enforces namespace boundaries, resource quotas, and tenant-level policies.

**Key Features:**
- Tenant namespace ownership
- Aggregated resource quotas
- Network policy enforcement
- Tenant-scoped RBAC

**Related:** [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md), [Security Overview](SECURITY_OVERVIEW.md)

### Flux
GitOps continuous delivery tool that automatically syncs Kubernetes manifests from OCI registries to clusters. fedCORE uses Flux to deploy platform components, tenants, and RGDs.

**Workflow:** Git Push → CI/CD Build → Nexus OCI → Flux Sync → Cluster Apply

**Related:** [Deployment](DEPLOYMENT.md)

### Kro (Kube Resource Orchestrator)
Kubernetes operator that defines Resource Graph Definitions (RGDs). Kro enables creating custom abstractions that compose multiple Kubernetes and cloud resources into simple developer-facing APIs.

**Example:** A `WebApp` RGD might create a Deployment, Service, Ingress, and AWS RDS database with a single manifest.

**Related:** [fedCORE Purposes](FEDCORE_PURPOSES.md), [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md)

### Kyverno
Policy engine for Kubernetes that enforces admission control, validation, and mutation policies. fedCORE uses Kyverno in both enforce mode (blocking violations) and audit mode (reporting only).

**Policy Types:**
- **Enforce:** Image registry restrictions, security baselines, resource limits
- **Audit:** Best practices like readiness probes, PodDisruptionBudgets, labels

**Related:** [Kyverno Policies](KYVERNO_POLICIES.md), [Security Overview](SECURITY_OVERVIEW.md)

### Tetragon
eBPF-based runtime security and observability tool from Cilium. Provides kernel-level visibility into process execution, network connections, and file access without sidecars or code changes.

**Use Cases:**
- Detecting privileged container escapes
- Monitoring suspicious process execution
- Network policy violations
- File system tampering

**Related:** [Runtime Security](RUNTIME_SECURITY.md), [Runtime Security & Logging](RUNTIME_SECURITY_AND_LOGGING.md)

---

## Architecture Concepts

### Base Template
The cloud-agnostic portion of an RGD that defines the Kro schema and common Kubernetes resources. Base templates use ytt placeholders for cloud-specific values.

**Location:** `platform/rgds/<name>/base/`

**Related:** [Development Guide](DEVELOPMENT.md)

### Bootstrap
The initial cluster configuration that installs core platform components (Capsule, Kyverno, Kro, Flux, ACK, ASO). Bootstrap runs once per cluster to prepare it for tenant onboarding.

**Components Installed:**
- Multi-tenancy (Capsule)
- Policy engine (Kyverno)
- RGD runtime (Kro)
- GitOps (Flux)
- Cloud controllers (ACK, ASO)

**Related:** [fedCORE Purposes](FEDCORE_PURPOSES.md), [Cluster Structure](CLUSTER_STRUCTURE.md)

### Cluster Account
The AWS account that hosts the EKS cluster and runs the control plane. This is separate from tenant accounts.

**Responsibilities:**
- Run EKS control plane
- Host cluster infrastructure
- Run platform operators (Capsule, Kyverno, Kro)
- Assume roles into tenant accounts for resource provisioning

**Related:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

### Overlay
Cloud-specific configurations that extend the base RGD template with provider-specific resources. Overlays use ytt to add AWS, Azure, or on-prem-specific resources.

**Examples:**
- AWS overlay: Adds RDS, S3, IAM roles
- Azure overlay: Adds CosmosDB, Storage Accounts, Managed Identities
- On-prem overlay: Adds bare-metal databases, NFS storage

**Location:** `platform/rgds/<name>/overlays/`

**Related:** [Development Guide](DEVELOPMENT.md)

### RGD (Resource Graph Definition)
A custom Kubernetes CRD that defines a high-level abstraction composed of multiple underlying resources. RGDs provide developer-friendly APIs for complex infrastructure patterns.

**Examples:**
- `WebApp`: Creates Deployment + Service + Ingress + Database
- `Queue`: Creates SQS (AWS) or Service Bus (Azure) or RabbitMQ (on-prem)
- `Cache`: Creates ElastiCache (AWS) or Redis Cache (Azure) or Redis (on-prem)

**Related:** [fedCORE Purposes](FEDCORE_PURPOSES.md), [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md)

### Tenant
An isolated unit of organization within the platform representing a team, project, or application. Tenants get:
- Isolated namespaces (following `<tenant>-*` naming pattern)
- Dedicated AWS account (in multi-account mode)
- Resource quotas (CPU, memory, storage, namespaces)
- RBAC permissions for tenant owners

**Related:** [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md), [Tenant User Guide](TENANT_USER_GUIDE.md)

### Tenant Account
The AWS account dedicated to a single tenant for resource provisioning. Each tenant gets their own account for billing isolation and security boundaries.

**What Lives Here:**
- RDS databases
- S3 buckets
- DynamoDB tables
- ElastiCache clusters
- IAM roles for applications

**What Doesn't Live Here:**
- Kubernetes workloads (run in cluster account)
- Persistent volumes (use cluster account EBS/EFS)
- Network infrastructure (managed by cluster)

**Related:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

---

## AWS-Specific Terms

### External ID
A unique identifier used in cross-account IAM role trust policies to prevent the "confused deputy" problem. fedCORE uses external IDs when the cluster account assumes roles into tenant accounts.

**Format:** `fedcore-<cluster-name>-<tenant-name>`

**Example:** `fedcore-prod-use1-acme`

**Related:** [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md), [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md)

### IRSA (IAM Roles for Service Accounts)
Legacy AWS authentication mechanism using OIDC federation. fedCORE uses **Pod Identity** instead, which is simpler and more secure.

**Why Not IRSA:**
- Requires OIDC provider setup per cluster
- Complex trust policy management
- Limited to EKS
- Replaced by Pod Identity in 2023

**Related:** [Pod Identity](POD_IDENTITY_FULL.md)

### LZA (Landing Zone Accelerator)
AWS solution for deploying multi-account environments following best practices. fedCORE integrates with LZA to provision tenant AWS accounts with pre-configured IAM roles and permission boundaries.

**LZA Responsibilities:**
- Create tenant AWS accounts
- Provision permission boundary policy
- Create ACK provisioner IAM role
- Configure AWS Config and CloudTrail

**Platform Responsibilities:**
- Deploy Kubernetes resources
- Create tenant deployer IAM role
- Provision application-specific IAM roles

**Related:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md), [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md)

### Permission Boundary
An IAM policy that sets the maximum permissions an IAM role can have, regardless of its own policy. fedCORE uses permission boundaries to prevent tenants from escalating privileges.

**Key Restrictions:**
- Cannot create IAM users
- Cannot modify permission boundaries
- Cannot access other tenant resources
- Can only operate within tenant's AWS account

**Related:** [IAM Architecture](IAM_ARCHITECTURE.md), [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)

### Pod Identity
Modern AWS authentication mechanism for pods that replaces IRSA. Uses an EKS add-on and agent to inject temporary credentials into pods via environment variables and files.

**Advantages over IRSA:**
- No OIDC provider needed
- Simpler trust policies
- Faster credential refresh
- Works across EKS clusters

**Components:**
- EKS Pod Identity add-on (control plane)
- Pod Identity agent (DaemonSet in cluster)
- Pod Identity associations (maps ServiceAccount → IAM role)

**Related:** [Pod Identity](POD_IDENTITY_FULL.md), [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)

---

## IAM Role Types

### ACK Provisioner Role
IAM role used by ACK controllers to provision AWS resources in tenant accounts. Has broad permissions but is restricted by the permission boundary.

**Assumed By:** ACK controllers running in cluster account

**Permissions:**
- Create/update/delete RDS, DynamoDB, S3, ElastiCache, SQS, etc.
- Create IAM roles for applications (restricted by permission boundary)
- Tag resources for cost allocation

**Trust Policy:** Cluster account ACK controllers via Pod Identity

**Related:** [IAM Architecture](IAM_ARCHITECTURE.md), [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)

### Tenant Deployer Role
IAM role used by tenant CI/CD pipelines to deploy Kubernetes manifests. Has kubectl permissions but **zero AWS API permissions**.

**Assumed By:** GitHub Actions workflows, GitLab CI, tenant automation

**Permissions:**
- Create/update/delete Kubernetes resources in tenant namespaces
- Read Kubernetes resources cluster-wide
- **No AWS API permissions** (by design)

**Why No AWS Permissions:** Kubernetes workloads use Pod Identity to access AWS, not CI/CD credentials. See [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md)

**Related:** [IAM Architecture](IAM_ARCHITECTURE.md), [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md)

### Application-Specific IAM Role
IAM role created for a specific application to access AWS resources. These roles have minimal permissions tailored to the application's needs.

**Example:** A WebApp might have a role that can:
- Read from a specific S3 bucket
- Write to a specific DynamoDB table
- Publish to a specific SQS queue

**Created By:** ACK controllers using the ACK Provisioner role

**Assumed By:** Application pods via Pod Identity

**Related:** [IAM Architecture](IAM_ARCHITECTURE.md), [Pod Identity](POD_IDENTITY_FULL.md)

---

## Multi-Cloud Terms

### Cloud-Agnostic Abstraction
A platform API (RGD) that works identically across AWS, Azure, and on-premises environments. Developers use the same manifest regardless of cloud provider.

**Example:** A `Database` RGD creates:
- RDS on AWS
- Azure SQL on Azure
- PostgreSQL on on-prem

**Related:** [fedCORE Purposes](FEDCORE_PURPOSES.md), [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

### Cloud-Specific Overlay
Provider-specific implementation of an RGD that adds cloud-native resources. Overlays extend the base template with AWS-specific, Azure-specific, or on-prem-specific resources.

**Related:** [Development Guide](DEVELOPMENT.md)

---

## GitOps Terms

### Artifact
A Kubernetes manifest YAML file bundled and pushed to an OCI registry (Nexus). Artifacts are versioned and immutable.

**Types:**
- **Bootstrap artifacts:** Core platform components
- **RGD artifacts:** Resource Graph Definitions
- **Tenant artifacts:** Tenant configurations

**Build Process:** `ytt` → validate → `flux push artifact` → Nexus OCI

**Related:** [Deployment](DEPLOYMENT.md)

### Flux Kustomization
A Flux CRD that defines what to sync from an OCI repository and where to apply it. Kustomizations watch OCI artifacts and automatically apply updates.

**Related:** [Deployment](DEPLOYMENT.md)

### OCI Repository
An OCI-compliant container registry (Nexus) used to store Kubernetes manifests as artifacts. Flux pulls artifacts from OCI repositories.

**Why OCI Instead of Git:**
- Better for large binary assets
- Supports air-gapped environments
- Decouples source code from deployment artifacts
- Enables immutable versioning

**Related:** [Deployment](DEPLOYMENT.md), [Helm Charts](HELM_CHARTS.md)

### ytt (YAML Templating Tool)
Carvel tool for templating Kubernetes YAML with logic, functions, and overlays. fedCORE uses ytt to generate cloud-specific manifests from base templates.

**Features:**
- Data values for configuration
- Overlays for modifying existing YAML
- Functions for dynamic generation
- Validation and type checking

**Related:** [Development Guide](DEVELOPMENT.md)

---

## Security Terms

### Admission Control
Kubernetes feature that intercepts API requests before resources are persisted. Kyverno uses admission control to enforce policies.

**Modes:**
- **Validating:** Accept or reject requests
- **Mutating:** Modify requests before persistence

**Related:** [Kyverno Policies](KYVERNO_POLICIES.md), [Security Overview](SECURITY_OVERVIEW.md)

### Enforce Mode
Kyverno policy mode that blocks resource creation if policies are violated. Used for critical security controls.

**Examples:**
- Block images from non-approved registries
- Block privileged containers
- Require resource limits

**Related:** [Kyverno Policies](KYVERNO_POLICIES.md)

### Audit Mode
Kyverno policy mode that allows resource creation but logs violations. Used for best practice guidance without blocking developers.

**Examples:**
- Warn about missing readiness probes
- Suggest PodDisruptionBudgets
- Recommend standard labels

**Related:** [Kyverno Policies](KYVERNO_POLICIES.md)

### Network Policy
Kubernetes resource that defines allowed network traffic between pods. fedCORE uses network policies to isolate tenant namespaces.

**Default Behavior:**
- Deny all cross-tenant traffic
- Allow traffic within tenant namespaces
- Allow traffic to cluster DNS and egress

**Related:** [Runtime Security](RUNTIME_SECURITY.md), [Tenant User Guide](TENANT_USER_GUIDE.md)

---

## Troubleshooting Terms

### Policy Violation
When a Kubernetes resource fails admission control due to a Kyverno policy. Violations are logged and (in enforce mode) block resource creation.

**Common Violations:**
- Using disallowed image registry
- Missing resource limits
- Running as root user
- Requesting privileged access

**Related:** [Troubleshooting](TROUBLESHOOTING.md), [Kyverno Policies](KYVERNO_POLICIES.md)

### Pod Identity Association
The mapping between a Kubernetes ServiceAccount and an AWS IAM role. Enables pods using that ServiceAccount to assume the IAM role via Pod Identity.

**Created By:** ACK `PodIdentityAssociation` controller or AWS CLI

**Related:** [Pod Identity](POD_IDENTITY_FULL.md), [Troubleshooting](TROUBLESHOOTING.md)

---

## Quick Reference

### Platform Stack
```
┌─────────────────────────────────────┐
│ Developer Abstractions (RGDs)      │ ← App-specific APIs
├─────────────────────────────────────┤
│ Tenant Bootstrap (Capsule)         │ ← Multi-tenancy
├─────────────────────────────────────┤
│ Cluster Bootstrap (Kro, Kyverno)   │ ← Platform runtime
├─────────────────────────────────────┤
│ Kubernetes (EKS, AKS, on-prem)     │ ← Compute layer
├─────────────────────────────────────┤
│ Cloud Providers (AWS, Azure)       │ ← Infrastructure
└─────────────────────────────────────┘
```

### Key File Locations
- **RGD Templates:** `platform/rgds/<name>/`
- **Cluster Configs:** `platform/clusters/<cluster-name>/`
- **Tenant Definitions:** `platform/clusters/<cluster>/tenants/`
- **Policy Definitions:** `platform/components/kyverno-policies/`
- **Build Commands:** `fedcore build`, `fedcore bootstrap`, `fedcore matrix`, `fedcore validate`

### Common Commands
```bash
# Validate templates
fedcore validate

# Build RGD artifact
fedcore build --artifact platform/rgds/<name> --cluster <cluster-dir>

# Check tenant status
kubectl get tenants
kubectl describe tenant <name>

# Check Pod Identity associations
kubectl get podidentityassociations -A
```

---

## Navigation

[← Previous: Handbook Intro](HANDBOOK_INTRO.md) | [Next: fedCORE Purposes →](FEDCORE_PURPOSES.md)

**Handbook Progress:** Page 2 of 35 | **Level 0:** Start Here

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
