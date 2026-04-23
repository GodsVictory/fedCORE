# fedCORE Platform Handbook

Welcome to the fedCORE Platform Handbook - your comprehensive guide to building, deploying, and managing multi-cloud infrastructure at scale.

## What is fedCORE?

fedCORE is a **multi-cloud internal developer platform** that provides self-service infrastructure to development teams across AWS, Azure, and on-premises environments. It enables platform teams to offer standardized, secure, and compliant infrastructure abstractions while giving developers the freedom to provision resources on-demand.

**Core Value Proposition:**
- **Multi-cloud by design** - Deploy identical abstractions across AWS, Azure, and on-prem
- **Self-service** - Developers provision infrastructure without platform team intervention
- **Multi-tenant isolation** - Each tenant gets isolated namespaces and dedicated AWS accounts
- **Security by default** - Policy enforcement, runtime monitoring, and compliance built-in
- **GitOps driven** - All changes managed through git with automated deployment pipelines

## Who Should Read This Handbook?

This handbook is organized as a **learning journey** from beginner to advanced concepts. Choose your starting point based on your role:

### 🎯 **I'm a Platform Administrator**
**You manage the platform:** Onboard tenants, configure clusters, manage quotas and policies

**What you do:**
- Create tenants for development teams
- Configure resource quotas and access controls
- Manage cluster-wide policies and security settings
- Handle tenant requests (quota increases, access grants)

**Quick Start:**
1. [Glossary](GLOSSARY.md) - Learn the terminology (5 min)
2. [Platform Overview](FEDCORE_PURPOSES.md) - Understand the architecture (10 min)
3. [Admin Quick Start](QUICKSTART_ADMIN.md) - Create your first tenant (5 min)
4. [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Deep dive into tenant management (30 min)

**Then explore:** Cluster Structure, Environment Setup, Multi-Account Architecture

### 💻 **I'm an Application Developer**
**You use the platform:** Deploy applications, create databases, configure ingress

**What you do:**
- Deploy web applications using WebApp RGDs
- Provision databases, caches, and message queues
- Configure ingress and TLS certificates
- Manage application configurations and secrets

**Quick Start:**
1. [Glossary](GLOSSARY.md) - Learn the terminology (5 min)
2. [Developer Quick Start](QUICKSTART_DEVELOPER.md) - Deploy your first app (5 min)
3. [Tenant User Guide](TENANT_USER_GUIDE.md) - Learn self-service operations (30 min)

**Then explore:** Security Policies, Ingress Management, AppDynamics Integration

### 🏗️ **I'm an Architect/Decision-Maker**
**You evaluate the platform:** Understand design philosophy, architectural trade-offs, TCO

**What you do:**
- Evaluate fedCORE for your organization
- Understand multi-cloud strategy and vendor lock-in avoidance
- Review security model and compliance capabilities
- Assess costs and return on investment

**Quick Start:**
1. [Architect Quick Start](QUICKSTART_ARCHITECT.md) - Design overview (10 min)
2. [Platform Overview](FEDCORE_PURPOSES.md) - Three core purposes (10 min)
3. [Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - Visual reference (15 min)
4. [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Tenant isolation strategy (30 min)

**Then explore:** Security Overview, IAM Architecture, Pod Identity

### ⚙️ **I'm a Platform Engineer**
**You extend the platform:** Create new RGDs, add cloud integrations, contribute features

**What you do:**
- Build new Resource Graph Definitions (RGDs)
- Add support for new cloud services (AWS, Azure, on-prem)
- Contribute to platform capabilities and abstractions
- Develop and test infrastructure templates

**Quick Start:**
1. [Glossary](GLOSSARY.md) - Learn the terminology (5 min)
2. [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md) - Create your first RGD (15 min)
3. [Development Guide](DEVELOPMENT.md) - Contribution workflow (30 min)

**Then explore:** Cluster Structure, Deployment Pipeline, Kyverno Policies

## How to Use This Handbook

### 📖 **Sequential Reading (Recommended for Beginners)**

Follow the handbook page-by-page for a complete learning journey:

**Level 0: Start Here** (15 min)
- Understand what you're learning and why

**Level 1: Foundation** (2-3 hours)
- Core concepts, visual diagrams, and quick wins by role

**Level 2-4: Operations** (3-4 hours)
- Platform setup, tenant management, deployment, and development

**Level 5: Security** (2-3 hours)
- Comprehensive security model and compliance

**Level 6: Multi-Account** (3-4 hours)
- AWS multi-account architecture and IAM design

**Level 7: Advanced** (2-3 hours)
- Specialized topics (ingress, Helm charts, monitoring)

**Total Time:** 12-17 hours (can be split across multiple sessions)

### 🔍 **Reference Mode (For Experienced Users)**

Jump directly to specific topics:
- **Common tasks** - Use the quick start guides
- **Troubleshooting** - See [Troubleshooting Guide](TROUBLESHOOTING.md)
- **Terminology** - See [Glossary](GLOSSARY.md)
- **Specific topics** - Use the handbook navigation at the bottom of each page

### 🗺️ **Navigation Features**

Every handbook page includes:
- **Previous/Next links** - Navigate the learning journey sequentially
- **Progress indicator** - Track your position (e.g., "Page 5 of 35")
- **Level indicator** - Know which section you're in
- **Quick links** - Jump back to this handbook, glossary, or troubleshooting

## Handbook Contents

### Level 0: Start Here
1. **HANDBOOK_INTRO** (this page) - Introduction and navigation guide
2. [GLOSSARY](GLOSSARY.md) - Essential terminology reference

### Level 1: Foundation & Quick Starts
3. [fedCORE Purposes](FEDCORE_PURPOSES.md) - Platform overview
4. [Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - Visual reference
5. [Admin Quick Start](QUICKSTART_ADMIN.md) - 5-minute tenant onboarding
6. [Developer Quick Start](QUICKSTART_DEVELOPER.md) - 5-minute app deployment
7. [Architect Quick Start](QUICKSTART_ARCHITECT.md) - Platform design overview
8. [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md) - RGD development
9. [FAQ](FAQ.md) - Frequently asked questions

### Level 2: Platform Setup & Structure
10. [Getting Started](GETTING_STARTED.md) - Detailed onboarding walkthrough
11. [Cluster Structure](CLUSTER_STRUCTURE.md) - Directory organization
12. [Environment Setup](ENVIRONMENT_SETUP.md) - GitHub configuration

### Level 3: Tenant Management
13. [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md) - Creating and managing tenants
14. [Tenant User Guide](TENANT_USER_GUIDE.md) - Self-service operations
15. [Tenant Advanced Topics](TENANT_ADVANCED_TOPICS.md) - Advanced networking

### Level 4: Deployment & Development
16. [Deployment](DEPLOYMENT.md) - CI/CD workflow
17. [Development](DEVELOPMENT.md) - Contributing to the platform
18. [RGD Schema Evolution](RGD_SCHEMA_EVOLUTION.md) - Migrating RGD schemas safely
19. [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md) - IAM design rationale
20. [Troubleshooting](TROUBLESHOOTING.md) - Comprehensive problem resolution

### Level 5: Security & Compliance
21. [Security Overview](SECURITY_OVERVIEW.md) - Security architecture
22. [Kyverno Policies](KYVERNO_POLICIES.md) - Admission control
23. [Runtime Security](RUNTIME_SECURITY.md) - Runtime monitoring
24. [Security Audit & Alerting](SECURITY_AUDIT_ALERTING.md) - Compliance
25. [Security Policy Reference](SECURITY_POLICY_REFERENCE.md) - Quick lookup
26. [Runtime Security & Logging](RUNTIME_SECURITY_AND_LOGGING.md) - Splunk integration

### Level 6: IAM & Multi-Account Architecture
27. [IAM Architecture](IAM_ARCHITECTURE.md) - Three-tier IAM model
28. [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Design principles
29. [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md) - Technical details
30. [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md) - Procedures
31. [LZA Tenant IAM Specification](LZA_TENANT_IAM_SPECIFICATION.md) - LZA requirements
32. [Pod Identity](POD_IDENTITY_FULL.md) - EKS Pod Identity

### Level 7: Advanced Features
33. [Ingress Management](INGRESS_MANAGEMENT.md) - Istio and NGINX
34. [Helm Charts](HELM_CHARTS.md) - OCI registry
35. [Tenant AppDynamics](TENANT_APPDYNAMICS.md) - APM integration

## Quick Reference

### Essential Links
- [📖 Glossary](GLOSSARY.md) - Terminology reference
- [🔧 Troubleshooting](TROUBLESHOOTING.md) - Problem resolution
- [❓ FAQ](FAQ.md) - Common questions
- [🎨 Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - Visual guides

### By Topic
- **Tenant Management** - Pages 13-15
- **Security** - Pages 20-25
- **Multi-Account** - Pages 26-31
- **Quick Starts** - Pages 5-8

### By Difficulty
- **Beginner** - Levels 0-2 (Pages 1-12)
- **Intermediate** - Levels 3-4 (Pages 13-19)
- **Advanced** - Levels 5-7 (Pages 20-34)

## Getting Help

### Documentation Issues
- **Found an error?** - File an issue in the repository
- **Missing information?** - Contact the platform team
- **Unclear documentation?** - Let us know what needs improvement

### Platform Support
- **GitHub Issues:** File issues in the platform repository
- **GitHub Discussions:** For questions and general discussions
- **Troubleshooting Guide:** [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

## What You'll Learn

By completing this handbook, you'll be able to:

✅ Understand fedCORE's multi-cloud architecture and design philosophy
✅ Create and manage tenants with multi-account isolation
✅ Deploy applications using platform abstractions (RGDs)
✅ Configure security policies and compliance controls
✅ Troubleshoot common issues across the platform
✅ Contribute new features and abstractions
✅ Design and implement cross-account resource provisioning
✅ Monitor and audit platform security and operations

---

## Navigation

[Next: Glossary →](GLOSSARY.md)

**Handbook Progress:** Page 1 of 35 | **Level 0:** Start Here

[📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
