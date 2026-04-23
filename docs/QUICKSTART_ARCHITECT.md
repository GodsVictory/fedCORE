# Quick Start: Architect

**Time to read:** 10 minutes

## What This Guide Covers

A high-level overview of fedCORE's design philosophy, architectural decisions, and trade-offs for architects and decision-makers evaluating the platform.

## Why fedCORE Exists

### Problems It Solves

**1. Cloud Vendor Lock-In**
- **Problem:** Teams build directly on AWS-specific services (Lambda, DynamoDB, etc.)
- **Impact:** Migrating workloads between clouds requires complete rewrites
- **fedCORE Solution:** Cloud-agnostic RGDs that work identically across AWS, Azure, and on-prem
- **Example:** A `Database` RGD provisions RDS on AWS, Azure SQL on Azure, PostgreSQL on-prem - same manifest

**2. Slow Infrastructure Provisioning**
- **Problem:** Manual ticket-based infrastructure requests take days or weeks
- **Impact:** Development velocity bottlenecked by infrastructure team
- **fedCORE Solution:** Self-service RGDs allow developers to provision infrastructure instantly via kubectl
- **Example:** Developer creates `Queue` manifest → SQS provisioned in minutes without tickets

**3. Security and Compliance Debt**
- **Problem:** Each team implements security differently, leading to inconsistent controls
- **Impact:** Audit failures, security incidents, compliance violations
- **fedCORE Solution:** Platform-enforced policies (Kyverno, Tetragon) that cannot be bypassed
- **Example:** All containers must use approved registries, run as non-root, have resource limits

**4. Cost Visibility and Allocation**
- **Problem:** Shared AWS accounts make it impossible to track team-specific costs
- **Impact:** No accountability for cloud spending, over-provisioning, waste
- **fedCORE Solution:** Dedicated AWS account per tenant with complete cost isolation
- **Example:** Finance can generate reports showing Team A spent $10K, Team B spent $15K

**5. Inconsistent Development Practices**
- **Problem:** Every team builds deployment pipelines and infrastructure differently
- **Impact:** Duplication of effort, inconsistent quality, maintenance burden
- **fedCORE Solution:** Standardized platform with golden paths for common use cases
- **Example:** All teams use same WebApp RGD → consistent monitoring, logging, security

## Design Philosophy

### Core Principles

**1. Multi-Cloud by Default**
- Abstract cloud-specific details behind uniform APIs
- Enable workload portability without application changes
- Maintain cloud-native features through provider-specific overlays
- Support hybrid deployments (AWS + on-prem simultaneously)

**2. Self-Service Within Guardrails**
- Developers provision infrastructure without platform team intervention
- Platform team defines what's possible through RGDs and policies
- Security policies enforced automatically (cannot be disabled)
- Audit trails for all actions (Splunk, CloudTrail, AWS Config)

**3. Security by Default**
- Least-privilege IAM roles for all workloads
- Network isolation between tenants
- Runtime security monitoring (Tetragon)
- Admission control policies (Kyverno)
- Encrypted storage and transit
- No long-lived credentials (Pod Identity/Workload Identity)

**4. Multi-Account Isolation**
- Each tenant gets dedicated AWS account (or Azure resource group)
- Blast radius containment (compromise of one tenant doesn't affect others)
- Billing isolation for chargeback/showback
- Compliance boundaries (PCI, HIPAA, FedRAMP workloads separated)

**5. GitOps-Driven Operations**
- All changes managed through git (infrastructure as code)
- Declarative configuration (desired state, not imperative scripts)
- Automated reconciliation (Flux syncs cluster to match git)
- Rollback capability (revert git commit)
- Audit trail (git history)

## Key Architectural Decisions

### Decision 1: Why Kro Instead of Crossplane?

**Alternatives Considered:**
- **Crossplane:** Mature, feature-rich, large community
- **Terraform Operator:** Existing Terraform modules, familiar to ops teams
- **Helm Charts:** Simple, widely adopted

**Why Kro:**
- **Simpler:** Pure Kubernetes CRDs, no external state (Terraform/Crossplane backends)
- **Kubernetes-Native:** Uses native controllers, RBAC, admission control
- **Composability:** RGDs reference other RGDs (WebApp → Database → IAM Role)
- **Multi-Cloud:** Overlays enable cloud-specific resources without forking
- **Developer Experience:** Single kubectl apply creates entire stack

**Trade-offs:**
- **Less mature:** Kro is newer, smaller community than Crossplane
- **Limited ecosystem:** Fewer pre-built compositions available
- **Platform team overhead:** More custom RGD development required

**Verdict:** Simpler operational model and better developer experience outweigh maturity concerns.

### Decision 2: Why Dedicated AWS Account Per Tenant?

**Alternatives Considered:**
- **Shared account with IAM isolation:** Use IAM policies to isolate tenants
- **Namespace-based isolation only:** All resources in same AWS account

**Why Multi-Account:**
- **Security:** Hard boundaries - tenant cannot access another tenant's resources even if IAM misconfigured
- **Billing:** Complete cost isolation for chargeback/showback
- **Compliance:** Separate PCI/HIPAA workloads into isolated accounts
- **Blast radius:** Compromised tenant cannot affect other tenants
- **Quota independence:** Each tenant has separate AWS service quotas

**Trade-offs:**
- **Complexity:** Cross-account IAM roles, trust policies, external IDs
- **Cost:** AWS charges per account (minimal, but non-zero)
- **Management overhead:** More accounts to monitor and audit

**Verdict:** Security and billing benefits far outweigh complexity costs.

### Decision 3: Why Pod Identity Instead of IRSA?

**Alternatives Considered:**
- **IRSA (IAM Roles for Service Accounts):** Older AWS mechanism using OIDC
- **Static IAM credentials:** Store access keys in Kubernetes Secrets

**Why Pod Identity:**
- **Simpler setup:** No OIDC provider configuration required
- **Faster credential rotation:** Credentials refresh more frequently
- **Better trust policies:** Simpler to audit and understand
- **Multi-cluster support:** Works across EKS clusters without reconfiguration
- **AWS recommendation:** Official replacement for IRSA as of 2023

**Trade-offs:**
- **EKS-only:** Requires EKS (doesn't work on self-managed Kubernetes)
- **Newer:** Less community documentation than IRSA

**Verdict:** Simplicity and AWS recommendation make it clear choice for EKS.

### Decision 4: Why Zero AWS Permissions for CI/CD Roles?

**Alternatives Considered:**
- **CI/CD with AWS permissions:** Allow pipelines to provision AWS resources directly
- **CI/CD with limited AWS permissions:** Scope permissions to specific resources

**Why Zero AWS Permissions:**
- **Security:** CI/CD credentials cannot be misused to access AWS resources
- **Least privilege:** Workloads use Pod Identity, not CI/CD credentials
- **Auditability:** All AWS actions traceable to specific pods, not generic CI/CD role
- **Simpler model:** One authentication mechanism (Pod Identity), not two

See [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md) for detailed rationale.

**Trade-offs:**
- **Learning curve:** Teams accustomed to CI/CD with AWS credentials must adapt
- **Infrastructure as Code:** Cannot use Terraform/CloudFormation from CI/CD (by design)

**Verdict:** Security and auditability benefits justify the paradigm shift.

## Multi-Account Isolation Strategy

### Tenant Isolation Layers

fedCORE implements **defense-in-depth** with seven isolation layers:

```
1. AWS Account Isolation
   └─ Each tenant in separate AWS account

2. Kubernetes Namespace Isolation
   └─ Capsule enforces tenant namespace boundaries

3. Network Policy Isolation
   └─ Deny cross-tenant pod communication

4. IAM Isolation
   └─ Permission boundaries prevent privilege escalation

5. Admission Control
   └─ Kyverno policies enforce security baselines

6. Runtime Security
   └─ Tetragon monitors for suspicious behavior

7. Audit & Compliance
   └─ Splunk, CloudTrail, AWS Config log all actions
```

**Defense-in-Depth Philosophy:**
- Compromise of one layer doesn't breach security
- Multiple controls must fail for security incident
- Audit trails detect and alert on anomalies

See [Security Overview](SECURITY_OVERVIEW.md) for complete model.

### Resource Provisioning Flow

**How tenants provision AWS resources without AWS credentials:**

```
1. Developer creates RGD (e.g., Database)
   ↓
2. Kro generates Kubernetes manifests + ACK resources
   ↓
3. ACK controller (running in cluster account) assumes role into tenant account
   ↓
4. ACK provisions AWS resource (RDS) in tenant account
   ↓
5. ACK creates IAM role for application (restricted by permission boundary)
   ↓
6. Pod Identity association grants pod access to IAM role
   ↓
7. Application pod accesses database using temporary credentials
```

**Key points:**
- Developer never touches AWS credentials
- Cluster account has permissions to assume into tenant accounts
- Permission boundaries prevent tenant from escalating privileges
- All actions logged to CloudTrail in tenant account

See [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) for detailed design.

## Security Model Overview

### Authentication and Authorization

**Identity Sources:**
- **Human Users:** Azure AD / Okta → Kubernetes RBAC
- **CI/CD Pipelines:** ServiceAccount with RBAC (no AWS permissions)
- **Application Pods:** ServiceAccount + Pod Identity → AWS IAM roles

**Authorization Model:**
- **Platform Admins:** Full cluster access, can onboard tenants
- **Tenant Admins:** Manage namespaces and resources within tenant
- **Tenant Users:** Deploy applications within tenant namespaces
- **Application Pods:** Least-privilege IAM roles per application

### Policy Enforcement

**Kyverno Policies (Admission Control):**
- **Enforce mode:** Block policy violations (e.g., unapproved registries)
- **Audit mode:** Report policy violations (e.g., missing readiness probes)
- **Mutation:** Automatically inject sidecars, labels, annotations

**Tetragon (Runtime Security):**
- **Process monitoring:** Detect suspicious process execution
- **Network monitoring:** Detect unauthorized network connections
- **File monitoring:** Detect tampering with critical files
- **Policy violations:** Alert on privilege escalation attempts

See [Kyverno Policies](KYVERNO_POLICIES.md) and [Runtime Security](RUNTIME_SECURITY.md).

## When to Use fedCORE

### Ideal Use Cases

fedCORE is **well-suited** for:

1. **Multi-cloud strategy** - Organizations committed to cloud portability
2. **Large development teams** - 10+ teams needing isolated infrastructure
3. **Regulated industries** - PCI, HIPAA, FedRAMP compliance requirements
4. **Cost accountability** - Need chargeback/showback per team
5. **Self-service culture** - Empower developers, reduce platform team toil
6. **Standardization** - Consolidate fragmented infrastructure practices

### When NOT to Use fedCORE

fedCORE is **not ideal** for:

1. **Single cloud commitment** - If deeply invested in AWS-specific services (Lambda, API Gateway)
2. **Small teams** - Fewer than 5 teams may not justify platform overhead
3. **Highly specialized workloads** - Machine learning, HPC, real-time trading (need cloud-native features)
4. **Greenfield startups** - Early-stage companies should optimize for speed, not portability
5. **Legacy applications** - Non-containerized applications requiring VM-based infrastructure

### Alternatives to Consider

| Alternative | When to Use |
|-------------|-------------|
| **AWS-native** | Single-cloud strategy, need AWS-specific features (Lambda, AppSync) |
| **Crossplane** | More mature ecosystem, existing Crossplane compositions |
| **Terraform** | Operations-heavy team, existing Terraform modules |
| **Kubernetes + Helm** | Simple deployments, don't need cloud resource provisioning |
| **Serverless (Lambda, Fargate)** | Event-driven workloads, minimal state, variable traffic |

## Cost Considerations

### Platform Costs

**One-time setup:**
- Platform team development (3-6 months for initial platform)
- RGD template creation (1-2 weeks per RGD)
- Policy development and testing (2-4 weeks)
- Training and documentation (2-4 weeks)

**Ongoing costs:**
- Platform team operations (1-2 FTEs)
- AWS account fees (minimal per account)
- EKS control plane ($0.10/hour per cluster)
- Monitoring and logging (Splunk, AppDynamics licenses)

### Cost Savings

**Efficiency gains:**
- **Reduced infrastructure requests:** 30-60 minute provisioning vs. 3-5 day tickets
- **Standardization:** Eliminate duplicate infrastructure development
- **Self-service:** Reduce platform team toil by 40-60%
- **Cost visibility:** Chargeback enables cost optimization

**Example ROI calculation:**
- **Before fedCORE:** 10 teams × 2 hours/week infrastructure requests × 50 weeks = 1,000 hours/year
- **After fedCORE:** 10 teams × 0.5 hours/week self-service × 50 weeks = 250 hours/year
- **Savings:** 750 developer hours/year (~$75K at $100/hour)

### Cost Allocation

**How costs are tracked:**
- AWS resources tagged with tenant, cost center, project
- Dedicated AWS accounts enable complete cost isolation
- Finance reports show per-tenant spending
- Teams receive monthly cost reports and optimization recommendations

See [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) for cost reporting details.

## Next Steps

Based on your role, continue to:

- **Platform Overview** - [fedCORE Purposes](FEDCORE_PURPOSES.md) - Detailed platform capabilities
- **Visual Reference** - [Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - System diagrams
- **Multi-Account Design** - [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Deep dive into isolation strategy
- **Security Deep Dive** - [Security Overview](SECURITY_OVERVIEW.md) - Comprehensive security model
- **IAM Design** - [IAM Architecture](IAM_ARCHITECTURE.md) - Three-tier role model

## Additional Resources

- **[Glossary](GLOSSARY.md)** - Platform terminology
- **[FAQ](FAQ.md)** - Frequently asked questions
- **[Troubleshooting](TROUBLESHOOTING.md)** - Common issues and resolutions
- **[Development Guide](DEVELOPMENT.md)** - Contributing to the platform

---

## Navigation

[← Previous: Developer Quick Start](QUICKSTART_DEVELOPER.md) | [Next: Platform Engineer Quick Start →](QUICKSTART_PLATFORM_ENGINEER.md)

**Handbook Progress:** Page 7 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
