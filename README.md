# fedCORE Platform (App Factory)

**Internal Developer Platform for Multi-Cloud Infrastructure Provisioning**

GitOps-based platform providing self-service infrastructure to development teams across AWS, Azure, and on-premises environments. Each tenant receives isolated namespaces and a dedicated AWS account.

---
![](fedcore.png)
---

## 📚 Documentation

**Start here:** [**fedCORE Platform Handbook**](docs/HANDBOOK_INTRO.md)

The handbook provides a complete learning journey from beginner to advanced topics, organized across 34 pages:

### Quick Access by Role

**Choose your role to get started:**

- **👨‍💼 Platform Administrators** → [Admin Quick Start](docs/QUICKSTART_ADMIN.md) (5 min)
  - *You manage the platform:* Onboard tenants, configure clusters, manage quotas and policies
  - *Example tasks:* Create tenant for "acme" team, increase CPU quota, grant namespace access

- **💻 Application Developers** → [Developer Quick Start](docs/QUICKSTART_DEVELOPER.md) (5 min)
  - *You use the platform:* Deploy applications, create databases, configure ingress
  - *Example tasks:* Deploy web app with PostgreSQL, create DynamoDB table, expose service

- **🏗️ Architects / Decision-Makers** → [Architect Quick Start](docs/QUICKSTART_ARCHITECT.md) (10 min)
  - *You evaluate the platform:* Understand design philosophy, architectural trade-offs, TCO
  - *Example questions:* Should we adopt fedCORE? How does multi-cloud work? What's the security model?

- **⚙️ Platform Engineers** → [Platform Engineer Quick Start](docs/QUICKSTART_PLATFORM_ENGINEER.md) (15 min)
  - *You extend the platform:* Create new RGDs, add cloud integrations, contribute features
  - *Example tasks:* Build Redis RGD, add Azure support, create custom abstractions

### Essential References

- [Glossary](docs/GLOSSARY.md) - Essential terminology reference
- [Troubleshooting Guide](docs/TROUBLESHOOTING.md) - Common issues and solutions
- [FAQ](docs/FAQ.md) - Frequently asked questions
- [Architecture & Methodology FAQ](docs/FAQ_ARCHITECTURE.md) - Why we build the way we do

---

## What is fedCORE?

A Kubernetes-based internal developer platform that provides:

- **Multi-tenant isolation** via Capsule and dedicated AWS accounts per tenant
- **Self-service infrastructure** via Kro ResourceGraphDefinitions (platform APIs)
- **Multi-cloud support** via ACK (AWS), ASO (Azure), and Operators (on-prem)
- **GitOps delivery** via Flux CD with OCI artifact distribution
- **Security by default** with Kyverno policies, Tetragon runtime security, and Splunk logging

**See [fedCORE Purposes](docs/FEDCORE_PURPOSES.md) for detailed platform overview and [Architecture Diagrams](docs/ARCHITECTURE_DIAGRAMS.md) for visual guides.**

---

## Repository Structure

```
app-factory/
├── platform/
│   ├── clusters/              # Cluster-specific configurations
│   │   └── <cluster-name>/
│   │       ├── cluster.yaml   # Main cluster config (policies, RGDs, quotas)
│   │       └── config/       # Tenant onboarding CRs
│   ├── components/            # Platform components (Kro, Capsule, Kyverno, etc.)
│   └── rgds/                  # Platform API templates
│       ├── tenant/            # Tenant onboarding RGD (multi-account setup)
│       └── webapps/           # Example WebApp RGD
├── scripts/                   # Build and validation automation
├── .github/workflows/         # CI/CD pipeline (GitHub Actions)
└── docs/                      # Complete documentation handbook (34 pages)
```

**See [Cluster Structure](docs/CLUSTER_STRUCTURE.md) for detailed directory organization.**

---

## CI/CD Pipeline

Push to `main` branch triggers:
1. **Validate** - yamllint checks on all YAML
2. **Discover** - Find clusters and RGDs to build
3. **Build** - Create artifacts in parallel for all targets
4. **Push** - Upload artifacts to Nexus OCI registry
5. **Deploy** - Flux detects changes and reconciles clusters automatically

**See [Deployment Guide](docs/DEPLOYMENT.md) for complete pipeline configuration.**

---

## Common Tasks

### Onboarding & Management

- **Onboard a new tenant** → [Admin Quick Start](docs/QUICKSTART_ADMIN.md) or [Tenant Admin Guide](docs/TENANT_ADMIN_GUIDE.md)
- **Add a new cluster** → [Cluster Structure - Adding a Cluster](docs/CLUSTER_STRUCTURE.md#adding-a-new-cluster)
- **Set up GitHub secrets** → [Environment Setup](docs/ENVIRONMENT_SETUP.md)

### Development & Deployment

- **Deploy an application** → [Developer Quick Start](docs/QUICKSTART_DEVELOPER.md)
- **Create a new RGD** → [Platform Engineer Quick Start](docs/QUICKSTART_PLATFORM_ENGINEER.md)
- **Contribute changes** → [Development Guide](docs/DEVELOPMENT.md)

### Multi-Account Architecture

- **Understand architecture** → [Multi-Account Architecture](docs/MULTI_ACCOUNT_ARCHITECTURE.md)
- **Technical implementation** → [Multi-Account Implementation](docs/MULTI_ACCOUNT_IMPLEMENTATION.md)
- **LZA integration** → [LZA Tenant IAM Specification](docs/LZA_TENANT_IAM_SPECIFICATION.md)
- **IAM role model** → [IAM Architecture](docs/IAM_ARCHITECTURE.md)

### Security & Compliance

- **Security overview** → [Security Overview](docs/SECURITY_OVERVIEW.md)
- **Policy reference** → [Security Policy Reference](docs/SECURITY_POLICY_REFERENCE.md)
- **Runtime security** → [Runtime Security & Logging](docs/RUNTIME_SECURITY_AND_LOGGING.md)

### Troubleshooting

- **General troubleshooting** → [Troubleshooting Guide](docs/TROUBLESHOOTING.md)
- **Deployment issues** → [Deployment - Monitoring](docs/DEPLOYMENT.md#monitoring-deployments)
- **Pod Identity issues** → [Troubleshooting - Pod Identity](docs/TROUBLESHOOTING.md#pod-identity-issues)

---

## Key Features

### 🏢 Multi-Tenancy
- **Capsule** for namespace isolation and quotas
- **Kyverno** for policy enforcement (30+ security policies)
- **Dedicated AWS accounts** per tenant via LZA
- **Cost allocation** with automatic resource tagging

**See [Tenant Admin Guide](docs/TENANT_ADMIN_GUIDE.md) for complete tenant management.**

### 🔐 Security
- **Multi-layered security**: Account isolation → IAM boundaries → Network policies → Pod security → Runtime enforcement
- **Tetragon eBPF** for runtime security monitoring
- **Permission boundaries** prevent privilege escalation
- **Immutable infrastructure** via GitOps

**See [Security Overview](docs/SECURITY_OVERVIEW.md) for complete security architecture.**

### 📊 Observability
- **Splunk integration** for centralized logging
- **Automatic tenant labeling** on all logs and metrics
- **Security event monitoring** with Tetragon
- **Audit trail** via git history and CloudTrail

**See [Runtime Security & Logging](docs/RUNTIME_SECURITY_AND_LOGGING.md) for logging configuration.**

### ☁️ Multi-Cloud
- **AWS** via ACK Controllers (S3, RDS, DynamoDB, IAM)
- **Azure** via Azure Service Operator (ASO)
- **On-Premises** via CloudNativePG, MinIO, operators

**See [Multi-Account Architecture](docs/MULTI_ACCOUNT_ARCHITECTURE.md) for multi-cloud design.**

### 🚀 Developer Experience
- **Self-service APIs** via ResourceGraphDefinitions (RGDs)
- **Single YAML** to provision multi-cloud resources
- **Automatic cloud selection** based on cluster location
- **Version-controlled infrastructure** via GitOps

**See [Developer Quick Start](docs/QUICKSTART_DEVELOPER.md) to deploy your first application.**

---

## Architecture

### Two-Tier OCI Artifacts
- **Tier 1 (Infrastructure)**: Kro operator, Flux sources, cloud controllers per cluster
- **Tier 2 (RGDs)**: Platform APIs with cloud-specific overlays, version-tagged

### Multi-Account Model
- Tenant AWS accounts provisioned via Landing Zone Accelerator (LZA)
- EKS clusters run in central "cluster account"
- Cross-account resource provisioning via ACK controllers with Pod Identity
- Permission boundaries prevent privilege escalation

**Complete architecture details:**
- Visual guides: [Architecture Diagrams](docs/ARCHITECTURE_DIAGRAMS.md)
- Multi-account design: [Multi-Account Architecture](docs/MULTI_ACCOUNT_ARCHITECTURE.md)
- IAM role model: [IAM Architecture](docs/IAM_ARCHITECTURE.md)
- Pod Identity: [Pod Identity Full Guide](docs/POD_IDENTITY_FULL.md)

---

## Contributing

1. Read [Development Guide](docs/DEVELOPMENT.md)
2. Create feature branch from `main`
3. Make changes and test locally with validation scripts
4. Submit PR with conventional commit format
5. Await review and CI/CD checks

**Questions or issues?** → Open a [GitHub issue](../../issues) or [GitHub discussion](../../discussions)

---

## Getting Help

- **📚 Documentation:** Start with [Handbook Introduction](docs/HANDBOOK_INTRO.md)
- **🔍 Search:** Use [Troubleshooting Guide](docs/TROUBLESHOOTING.md) symptom index
- **💬 Questions:** Open a [GitHub discussion](../../discussions)
- **🐛 Issues:** File a [GitHub issue](../../issues)
- **📖 Terminology:** Check the [Glossary](docs/GLOSSARY.md)

---

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

---

**Project maintained by the Platform Engineering Team**
