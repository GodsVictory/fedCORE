# fedCORE Platform: Three Core Purposes

fedCORE serves three distinct but interconnected purposes as an Internal Developer Platform (IDP).

## 1. Cluster Bootstrapping

**Purpose:** Establish foundational platform infrastructure on new Kubernetes clusters

**What it provides:**
- Core platform components (Kro, Capsule, Kyverno, Istio)
- Cloud controllers (ACK for AWS, ASO for Azure, Operators for on-prem)
- Security policies and runtime enforcement (Tetragon, Twistlock)
- Observability integrations (Splunk, AppDynamics)
- Multi-tenancy isolation framework

**Mechanism:**
- Built as Tier 1 OCI artifacts per cluster
- Deployed via GitOps (Flux) from `platform/components/`
- Cloud and environment-specific overlays applied automatically

**Example:** Setting up fedcore-prod-use1 deploys ACK controllers, AWS permission boundaries, and production-grade policies.

## 2. Tenant Bootstrapping

**Purpose:** Automated onboarding of development teams with complete isolation and dedicated cloud resources

**What it provides:**
- Capsule tenant with namespace isolation and quotas
- CI/CD namespace and ServiceAccount with cloud IAM
- Dedicated AWS account (via LZA) or Azure resource group
- Cross-account IAM roles with permission boundaries (AWS)
- Pod Identity associations or Workload Identity (Azure)
- Istio service mesh integration (optional)

**Mechanism:**
- Tenant RGD (`platform/rgds/tenant/`) creates full stack via Kro
- TenantOnboarding CR triggers automated provisioning
- 30-60 minute manual process reduced to 5 minutes

**Example:** Creating `TenantOnboarding` for "acme" provisions namespaces, AWS IAM roles, and deployment ServiceAccount automatically.

## 3. RGDs for App-Factory Functionality

**Purpose:** Self-service infrastructure APIs for development teams to deploy applications and cloud resources

**What it provides:**
- High-level abstractions (WebApp, DynamoDB, etc.) that hide cloud complexity
- Single YAML to provision multi-resource stacks
- Automatic cloud resource selection based on cluster location
- Version-controlled, testable infrastructure definitions

**Mechanism:**
- Built as Tier 2 OCI artifacts per cluster
- RGDs combine base templates with cloud-specific overlays
- Kro orchestrates resource provisioning from custom resources
- Cloud controllers (ACK/ASO) provision actual infrastructure

**Example:** Developer creates `WebApp` CR → Kro provisions Deployment + S3 bucket (AWS) or Storage Account (Azure) without knowing cloud specifics.

---

## How They Work Together

Cluster Bootstrap (Tier 1) → Installs Kro + Controllers + Policies → Tenant Bootstrap (Tier 2 RGD) → Creates isolated tenant with cloud IAM → App Factory RGDs (Tier 2 RGDs) → Tenants deploy applications via self-service APIs

**The Result:** A complete platform where tenants can deploy production workloads with security, isolation, and multi-cloud abstraction built in.

**Visual architecture guides:** See [Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) for detailed visual representations of:
- Platform layers and components
- GitOps workflow
- Multi-account structure
- Security layers

---

## Why This Architecture?

### The Problems We Solve

**Without fedCORE:**
- ❌ Manual tenant onboarding takes 30-60 minutes per team
- ❌ Cloud resources require specialized knowledge (AWS IAM, Azure RBAC)
- ❌ Inconsistent security policies across environments
- ❌ Vendor lock-in with cloud-specific tooling
- ❌ No standardized way to provision multi-cloud infrastructure
- ❌ Difficult to enforce resource quotas and cost allocation

**With fedCORE:**
- ✅ Automated tenant onboarding in 5 minutes
- ✅ Self-service infrastructure via simple Kubernetes APIs
- ✅ Consistent security policies enforced automatically
- ✅ Write once, deploy anywhere (AWS/Azure/on-prem)
- ✅ Standardized abstractions for common patterns
- ✅ Built-in quotas, cost tracking, and chargeback

### Design Philosophy

**1. Kubernetes-Native**
- Everything is a Kubernetes resource (no custom CLIs or portals)
- Standard kubectl workflows for all operations
- GitOps-friendly configuration management

**2. Multi-Cloud by Design**
- Cloud-agnostic base templates with provider-specific overlays
- Same developer experience across AWS, Azure, and on-premises
- Avoid vendor lock-in while leveraging cloud-native services

**3. Security by Default**
- Multiple isolation layers (multi-account, network policies, IAM boundaries)
- Policy enforcement via Kyverno (admission control)
- Runtime security monitoring with Tetragon eBPF
- Audit logging to Splunk for all platform events

**4. Self-Service with Guardrails**
- Developers provision resources without platform team intervention
- Automated policy enforcement prevents misconfigurations
- Tenant-scoped permissions limit blast radius

**5. Zero Trust Security Model**
- Pod Identity for workload authentication (no static credentials)
- Least privilege IAM roles with permission boundaries
- Network segmentation via Capsule and Istio
- Continuous monitoring and alerting

### Key Technical Decisions

| Decision | Rationale | Trade-offs |
|----------|-----------|------------|
| **Kro (not Crossplane)** | Simpler, Kubernetes-native resource graphs | Less mature ecosystem, fewer providers |
| **Multi-account per tenant** | Strong billing and security isolation | More AWS account management overhead |
| **Pod Identity (not IRSA)** | Simpler trust policies, no OIDC provider | Requires EKS 1.24+ |
| **Flux (not ArgoCD)** | OCI registry support for air-gapped environments | Fewer UI features |
| **ytt (not Helm)** | More powerful templating, overlays, and validation | Steeper learning curve |

---

## Practical Examples

### Example 1: Onboarding a New Team

**Before fedCORE (30-60 minutes):**
1. Create AWS account via LZA
2. Manually configure IAM roles and policies
3. Set up Kubernetes namespaces and RBAC
4. Create network policies
5. Configure resource quotas
6. Set up CI/CD ServiceAccount
7. Document everything for the team

**With fedCORE (5 minutes):**
```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: TenantOnboarding
metadata:
  name: acme
spec:
  tenantName: acme
  aws:
    accountId: "123456789012"
  owners:
    - kind: User
      name: admin@acme-corp.com
```

Git commit + push → Everything automated!

**See:** [Admin Quick Start](QUICKSTART_ADMIN.md) for complete walkthrough | [Tenant RGD schema](../platform/rgds/tenant/base/tenant-rgd.yaml) for all fields

### Example 2: Deploying a Web Application

**Before fedCORE:**
- Create Deployment YAML (50+ lines)
- Create Service YAML
- Create Ingress YAML
- Provision RDS database (AWS Console or Terraform)
- Create IAM role for database access
- Configure Pod Identity association
- Set up database credentials

**With fedCORE:**
```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: WebApp
metadata:
  name: myapp
spec:
  image: myapp:v1.0.0
  replicas: 3
  database:
    engine: postgres
```

RGD automatically creates: Deployment, Service, Ingress, RDS (AWS) or Azure SQL, IAM roles, and credentials!

**See:** [Developer Quick Start](QUICKSTART_DEVELOPER.md) for complete examples | [WebApp RGD examples](../platform/rgds/webapps/examples/)

### Example 3: Multi-Cloud Consistency

**Same RGD works across clouds.** Example with DynamoDB RGD:

```yaml
apiVersion: platform.fedcore.io/v1alpha1
kind: DynamoDB
metadata:
  name: user-data
spec:
  billingMode: PAY_PER_REQUEST
  hashKey: userId
```

- **AWS cluster:** Provisions DynamoDB table
- **Azure cluster:** Would provision CosmosDB (with adapter)
- **On-prem cluster:** Would use compatible database

Developers don't need to know cloud-specific details!

**See:** [DynamoDB RGD examples](../platform/rgds/dynamodb/examples/) for complete configurations

---

## Key Terminology

New to fedCORE? These terms are essential:

- **[RGD (Resource Graph Definition)](GLOSSARY.md#rgd-resource-graph-definition)** - Custom Kubernetes API that provisions multiple resources
- **[Kro](GLOSSARY.md#kro-kube-resource-orchestrator)** - Kubernetes operator that processes RGDs
- **[Capsule](GLOSSARY.md#capsule)** - Multi-tenancy operator for namespace isolation
- **[Kyverno](GLOSSARY.md#kyverno)** - Policy engine for admission control
- **[ACK (AWS Controllers for Kubernetes)](GLOSSARY.md#ack-aws-controllers-for-kubernetes)** - Provisions AWS resources from Kubernetes
- **[Pod Identity](GLOSSARY.md#pod-identity)** - AWS authentication for pods without static credentials
- **[Tenant](GLOSSARY.md#tenant)** - Isolated unit of organization (team, project, application)

**See [Glossary](GLOSSARY.md) for complete terminology reference.**

---

## Related Documentation

**Next Steps:**
- [Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - Visual platform overview
- [Quick Start: Admin](QUICKSTART_ADMIN.md) - Create your first tenant (5 min)
- [Quick Start: Developer](QUICKSTART_DEVELOPER.md) - Deploy your first app (5 min)

**Deep Dives:**
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Account isolation strategy
- [Security Overview](SECURITY_OVERVIEW.md) - Security model and policies
- [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Complete tenant management

**For Platform Engineers:**
- [Development Guide](DEVELOPMENT.md) - Contributing to fedCORE
- [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md) - Create new RGDs

---

## Navigation

[← Previous: Glossary](GLOSSARY.md) | [Next: Architecture Diagrams →](ARCHITECTURE_DIAGRAMS.md)

**Handbook Progress:** Page 3 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
