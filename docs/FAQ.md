# Frequently Asked Questions (FAQ)

Common questions about the fedCORE platform, organized by topic.

**For deep architectural and methodology questions (why we build the way we do), see [Architecture & Methodology FAQ](FAQ_ARCHITECTURE.md).**

---

## Platform Design

### Why multi-cloud?

**Strategic flexibility and vendor lock-in avoidance.**

**Benefits:**
- **Workload portability:** Migrate workloads between clouds without code changes
- **Negotiating leverage:** Avoid vendor lock-in, negotiate better pricing
- **Risk mitigation:** Reduce dependency on single cloud provider
- **Geographic compliance:** Deploy to specific regions based on regulations
- **Hybrid cloud:** Support on-premises alongside public cloud

**Real-world example:**
A government agency requires data sovereignty (data must stay in-country). fedCORE enables deploying the same `WebApp` RGD to AWS GovCloud (US), Azure Government, or on-premises data centers without application changes.

**Trade-offs:**
- Platform complexity increases (must support multiple clouds)
- Not all cloud-native features available (e.g., AWS Lambda)
- Platform team must learn multiple cloud providers

**See also:** [Architect Quick Start](QUICKSTART_ARCHITECT.md), [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

---

### Why Kro instead of Crossplane?

**Simpler operational model and better Kubernetes-native integration.**

**Kro advantages:**
- **No external state:** Everything in Kubernetes etcd (no Terraform/Crossplane backends)
- **Simpler:** Pure Kubernetes CRDs, native controllers
- **Composability:** RGDs reference other RGDs naturally
- **Developer experience:** Single `kubectl apply` creates entire stack

**Crossplane advantages:**
- **Maturity:** Larger community, more pre-built compositions
- **Ecosystem:** More cloud provider support out-of-the-box
- **Features:** Advanced composition features (patches, transforms)

**Why we chose Kro:**
Simpler operational model outweighs maturity concerns. Platform team can create custom RGDs more easily with Kro than Crossplane compositions.

**See also:** [Architect Quick Start](QUICKSTART_ARCHITECT.md), [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md)

---

## Multi-Account Architecture

### Why multi-account per tenant?

**Security boundaries, billing isolation, and blast radius containment.**

**Benefits:**

1. **Security:** Hard account boundaries - tenant cannot access another tenant's resources even if IAM misconfigured
2. **Billing:** Complete cost isolation for chargeback/showback to finance
3. **Compliance:** Separate PCI/HIPAA/FedRAMP workloads into isolated accounts
4. **Blast radius:** Compromised tenant cannot affect other tenants
5. **Quota independence:** Each tenant has separate AWS service quotas

**Example:**
Team A accidentally sets an RDS instance to public access. Only Team A's resources are exposed. Team B's resources remain secure in a separate AWS account.

**Cost:**
- AWS charges minimal per-account fees (AWS Organizations is free)
- Cross-account IAM adds complexity but provides strong isolation

**See also:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md), [Multi-Account Implementation](MULTI_ACCOUNT_IMPLEMENTATION.md)

---

### How are costs allocated?

**AWS resource tagging and dedicated tenant accounts enable per-tenant cost reporting.**

**Cost Tracking Mechanisms:**

1. **Dedicated AWS Accounts:**
   - Each tenant has a separate AWS account
   - AWS Cost Explorer shows per-account spending
   - Finance can generate reports: "Team A spent $10K, Team B spent $15K"

2. **Resource Tagging:**
   - All AWS resources automatically tagged with:
     - `tenant: acme`
     - `cost-center: engineering`
     - `project: acme-app`
     - `environment: production`
   - AWS Cost Allocation Tags group costs by tag

3. **Kubernetes Resource Usage:**
   - Capsule tracks CPU, memory, storage per tenant
   - Platform team can export metrics to cost management tools
   - Cluster costs allocated proportionally based on usage

**Example Report:**

| Tenant | AWS Account | EC2 | RDS | S3 | Other | Total |
|--------|-------------|-----|-----|----|----|-------|
| acme   | 222222222222 | $3K | $5K | $1K | $1K | $10K |
| globex | 333333333333 | $2K | $8K | $3K | $2K | $15K |

**See also:** [Multi-Account Operations](MULTI_ACCOUNT_OPERATIONS.md), [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md)

---

### Can tenants access other tenants' data?

**No - multiple isolation layers prevent cross-tenant access.**

**Isolation Layers:**

1. **AWS Account Isolation:**
   - Each tenant has a dedicated AWS account
   - IAM policies prevent cross-account access
   - Even if IAM misconfigured in one account, other accounts are isolated

2. **Kubernetes Namespace Isolation:**
   - Capsule enforces tenant namespace boundaries
   - RBAC prevents tenants from accessing other tenants' namespaces

3. **Network Policy Isolation:**
   - Network policies deny all traffic between tenant namespaces
   - Pods cannot communicate across tenant boundaries

4. **IAM Permission Boundaries:**
   - Prevent privilege escalation
   - Tenants cannot modify their own IAM policies to gain access

5. **Runtime Monitoring:**
   - Tetragon detects and alerts on suspicious cross-tenant access attempts
   - Security team receives alerts for investigation

**Example Attack Scenario:**
- **Attack:** Tenant A tries to access Tenant B's RDS database
- **Prevention:**
  1. Network policies block pod-to-pod traffic between tenants
  2. IAM roles scoped to Tenant A's AWS account only
  3. Tetragon detects unauthorized network connection attempt
  4. Security alert sent to platform team

**See also:** [Security Overview](SECURITY_OVERVIEW.md), [Runtime Security](RUNTIME_SECURITY.md)

---

## Operations

### What happens if quota exceeded?

**Kubernetes blocks resource creation and sends alerts.**

**Behavior:**

1. **Resource Quota Exceeded:**
   - Developer tries to create a new pod
   - Kubernetes checks tenant's aggregate quota
   - If quota exceeded, Kubernetes rejects the request with error message

2. **Error Message:**
   ```
   Error from server (Forbidden): error when creating "deployment.yaml":
   pods "myapp-xyz" is forbidden: exceeded quota: tenant-quota,
   requested: cpu=2, used: cpu=99, limited: cpu=100
   ```

3. **Alerts Sent:**
   - Platform team receives alert
   - Tenant admin receives notification
   - Grafana dashboard shows quota usage metrics

**Resolution:**

- **Short-term:** Delete unused resources to free quota
- **Long-term:** Request quota increase from platform team

**How to request quota increase:**

1. Open a GitHub issue in the platform repository with justification (e.g., "Black Friday traffic spike")
2. Platform team reviews request and updates tenant manifest
3. Quota increase takes effect within 5 minutes

**See also:** [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md), [Troubleshooting](TROUBLESHOOTING.md)

---

### How do I request policy exemption?

**Contact platform team with business justification - exemptions granted case-by-case.**

**Exemption Process:**

1. **Identify the policy violation:**
   - Check policy reports: `kubectl get policyreport -n <namespace>`
   - Note the policy name and violation details

2. **Contact platform team:**
   - Open a GitHub issue in the platform repository
   - Include:
     - Policy name (e.g., `restrict-image-registries`)
     - Business justification
     - Affected workload
     - Duration needed (temporary vs permanent)

3. **Platform team review:**
   - Evaluate security risk
   - Propose alternatives if possible
   - Approve or deny exemption

4. **Implementation:**
   - **Namespace-level exemption:** Annotate namespace with exemption
   - **Resource-level exemption:** Add policy annotation to resource
   - **Platform-level change:** Modify policy if justified

**Example Exemption Request:**

> **Policy:** `restrict-image-registries`
> **Workload:** `data-science-notebook`
> **Justification:** Need to pull Jupyter notebook image from Dockerhub for data science work. No equivalent image in approved registry.
> **Duration:** Permanent
> **Security considerations:** Image is from official Jupyter project, will scan for vulnerabilities.

**Platform team response:**

> **Approved with conditions:**
> - Use specific image tag (not `latest`)
> - Run in isolated namespace with network policies
> - Scan image monthly for vulnerabilities
> - Exemption annotation: `policies.kyverno.io/exempt: "restrict-image-registries"`

**See also:** [Kyverno Policies](KYVERNO_POLICIES.md), [Security Policy Reference](SECURITY_POLICY_REFERENCE.md)

---

### How long does tenant onboarding take?

**5-10 minutes automated, down from 30-60 minutes manual process.**

**Timeline:**

1. **Create tenant file:** 2 minutes
2. **Commit and push:** 1 minute
3. **CI/CD pipeline:** 2-3 minutes
4. **Flux sync to cluster:** 1-2 minutes
5. **Kro provisions resources:** 1-2 minutes
6. **AWS IAM roles created:** 2-3 minutes

**Total:** 5-10 minutes

**What was it before fedCORE?**
- Submit ticket to platform team: 1-2 days
- Platform engineer creates resources manually: 30-60 minutes
- Testing and validation: 1-2 hours
- Total: 3-5 business days

**See also:** [Admin Quick Start](QUICKSTART_ADMIN.md), [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md)

---

### Can I use kubectl directly?

**Yes - tenants use kubectl with their ServiceAccount credentials.**

**Access Methods:**

1. **CI/CD Pipelines:**
   - Use tenant deployer ServiceAccount
   - GitHub Actions example:
     ```yaml
     - name: Deploy to Kubernetes
       run: |
         kubectl apply -f manifests/ --namespace acme-dev
       env:
         KUBECONFIG: ${{ secrets.KUBECONFIG }}
     ```

2. **Developer Workstations:**
   - Generate kubeconfig with tenant ServiceAccount token
   - Use Azure AD / Okta for SSO authentication
   - Access restricted to tenant namespaces

3. **Interactive Debug Sessions:**
   ```bash
   kubectl exec -it <pod-name> -n acme-dev -- /bin/bash
   ```

**What you can do:**
- Create/update/delete resources in your tenant's namespaces
- View logs and describe resources
- Port-forward to pods for debugging
- Run one-off jobs

**What you cannot do:**
- Access other tenants' namespaces
- Modify cluster-wide resources (CRDs, namespaces, policies)
- Delete tenant itself (only platform admins can)

**See also:** [Tenant User Guide](TENANT_USER_GUIDE.md), [IAM Architecture](IAM_ARCHITECTURE.md)

---

## Security

### Why zero AWS permissions for CI/CD?

**Security, auditability, and least privilege.**

**Reasons:**

1. **Security:** CI/CD credentials cannot be misused to access AWS resources directly
2. **Least privilege:** Workloads use Pod Identity with minimal permissions, not broad CI/CD credentials
3. **Auditability:** All AWS actions traceable to specific pods, not generic CI/CD role
4. **Simplified model:** One authentication mechanism (Pod Identity), not two (CI/CD + Pod Identity)

**How it works:**

- **CI/CD role:** Only has kubectl permissions, zero AWS API permissions
- **Application pods:** Use Pod Identity to assume IAM roles with minimal permissions
- **Result:** Even if CI/CD credentials compromised, attacker cannot access AWS resources

**Example:**

**Before (traditional approach):**
```yaml
# CI/CD role has broad AWS permissions
CI/CD Role:
  - s3:*
  - dynamodb:*
  - rds:*
```
**Risk:** Compromised CI/CD credentials = full AWS access

**After (fedCORE):**
```yaml
# CI/CD role has zero AWS permissions
CI/CD Role:
  - kubernetes:CreateDeployment
  - kubernetes:ReadPod

# Application pod has minimal permissions
Application Pod IAM Role:
  - s3:GetObject (specific bucket only)
  - dynamodb:Query (specific table only)
```
**Security:** Compromised CI/CD credentials ≠ AWS access

**See also:** [CI/CD Role Zero Permissions](CICD_ROLE_ZERO_PERMISSIONS.md), [IAM Architecture](IAM_ARCHITECTURE.md)

---

### How is security monitored?

**Multiple layers: Tetragon runtime monitoring, Kyverno policies, Splunk logging, CloudTrail auditing.**

**Security Monitoring Stack:**

1. **Tetragon (Runtime Security):**
   - eBPF-based process, network, and file system monitoring
   - Detects:
     - Privilege escalation attempts
     - Suspicious process execution
     - Unauthorized network connections
     - File system tampering
   - Alerts sent to configured monitoring systems

2. **Kyverno (Admission Control):**
   - Validates resources at creation time
   - Blocks policy violations (enforce mode)
   - Reports violations (audit mode)
   - Policy reports available via kubectl

3. **Splunk (Log Aggregation):**
   - Collects logs from all platform components
   - Indexes Tetragon alerts, Kyverno violations, application logs
   - Security dashboards for SOC team
   - Automated alerting on suspicious patterns

4. **CloudTrail (AWS Auditing):**
   - Logs all AWS API calls
   - Tracks resource provisioning, IAM changes, data access
   - Integrated with AWS Config for compliance

5. **AWS Config (Compliance Monitoring):**
   - Continuously audits AWS resource configurations
   - Detects drift from security baselines
   - Sends alerts on non-compliant resources

**Security Workflow:**

```
1. Policy violation detected (Kyverno or Tetragon)
   ↓
2. Alert sent to Splunk
   ↓
3. Security dashboard updated
   ↓
4. If critical: Page security on-call
   ↓
5. Investigation using CloudTrail and logs
   ↓
6. Remediation (block user, delete resource, patch vulnerability)
```

**See also:** [Security Overview](SECURITY_OVERVIEW.md), [Runtime Security](RUNTIME_SECURITY.md), [Security Audit & Alerting](SECURITY_AUDIT_ALERTING.md)

---

### What clouds are supported?

**AWS, Azure, and on-premises environments.**

**Support Level:**

| Cloud Provider | Status | RGD Support | Notes |
|---------------|--------|-------------|-------|
| **AWS** | ✅ Production | Full | Primary cloud provider |
| **Azure** | ✅ Production | Full | Equal feature parity with AWS |
| **On-premises** | ✅ Production | Partial | Kubernetes + bare metal resources |
| **GCP** | 🔄 Planned | None | Roadmap for Q3 2026 |

**Cloud-Specific Features:**

**AWS:**
- EC2, RDS, DynamoDB, S3, ElastiCache, SQS, SNS
- EKS Pod Identity for authentication
- ACK controllers for resource provisioning
- CloudTrail, Config, GuardDuty for security

**Azure:**
- VMs, Azure SQL, CosmosDB, Blob Storage, Azure Cache, Service Bus
- Workload Identity for authentication
- ASO (Azure Service Operator) for resource provisioning
- Azure Monitor, Security Center for monitoring

**On-Premises:**
- Bare metal servers, PostgreSQL, MySQL, Redis, RabbitMQ
- StatefulSets for stateful workloads
- NFS/Ceph for storage
- Self-managed monitoring (Prometheus, Grafana)

**RGD Portability:**

Same `WebApp` manifest works across all clouds:
```yaml
apiVersion: v1alpha1
kind: WebApp
metadata:
  name: myapp
spec:
  database:
    enabled: true
    engine: postgres  # → RDS (AWS), Azure SQL (Azure), PostgreSQL (on-prem)
```

**See also:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md), [Development Guide](DEVELOPMENT.md)

---

## Build & Deployment Methodology

### Why do we pre-render Helm charts instead of using runtime HelmRelease?

**Pre-rendering at build time enables cluster-wide customizations.**

**Problem:** With runtime Flux HelmRelease, cluster-specific overlays (tolerations, node selectors) couldn't be applied to Helm-rendered resources.

**Solution:** Pre-render all Helm charts at build time:
1. Apply overlays to Helm values (pre-render phase)
2. Render Helm chart with modified values
3. Apply overlays to rendered manifests (post-render phase)
4. Push as OCI artifact to Nexus
5. Flux deploys plain manifests (not HelmRelease)

**Benefits:**
- ✅ Cluster overlays work on ALL components (tolerations, node selectors, labels)
- ✅ Deterministic deployments (same manifests every time)
- ✅ Faster Flux reconciliation (no Helm rendering at runtime)
- ✅ Air-gapped friendly (no Helm repository access needed)

**Trade-offs:**
- ❌ Longer build times (pre-rendering takes time)
- ❌ Must rebuild to change values (can't just update HelmRelease)

**See also:** [Architecture & Methodology FAQ - Pre-Render vs Runtime Helm](FAQ_ARCHITECTURE.md#why-pre-render-helm-charts-instead-of-using-flux-helmrelease-at-runtime)

---

### What is the two-phase overlay system?

**Pre-render and post-render overlays serve different purposes.**

**Pre-render overlays** - Modify Helm values BEFORE rendering:
```yaml
#! overlay-phase: pre-render
---
helm:
  values:
    tetragon:
      extraEnv:
        - name: CLOUD_PROVIDER
          value: "aws"
```

**Post-render overlays** - Modify rendered manifests AFTER Helm template:
```yaml
#! overlay-phase: post-render
---
apiVersion: cilium.io/v1alpha1
kind: TracingPolicy
metadata:
  name: aws-iam-monitoring
```

**Why two phases?**
- Pre-render leverages Helm chart's built-in templating (cleaner, simpler overlays)
- Post-render adds resources not in the chart or applies universal patches

**See also:**
- [Architecture & Methodology FAQ - Two-Phase Overlay System](FAQ_ARCHITECTURE.md#why-use-a-two-phase-overlay-system-pre-render-and-post-render)
- [Overlay System Reference](../platform/components/OVERLAY-SYSTEM.md)

---

### Why split istio into separate components?

**Independent versioning and clearer separation of concerns.**

**istio** was split into:
- `istio` - Control plane (istiod)
- `istio-gateway` - Ingress gateway

**Benefits:**
- ✅ Independent versioning (upgrade one without affecting others)
- ✅ Selective deployment (only deploy what's needed)
- ✅ Clearer ownership (one upstream per component)
- ✅ Simpler overlays (focused on one chart)

**See also:** [Architecture & Methodology FAQ - Multi-Chart Component Splitting](FAQ_ARCHITECTURE.md#multi-chart-component-splitting)

---

### Why use component.yaml instead of separate HelmRepository and HelmRelease files?

**Single source of truth for component metadata.**

**component.yaml** defines all component configuration:
- Chart name and version
- Repository URL and type (OCI vs default)
- Helm values
- Release metadata

**Benefits:**
- ✅ Single source of truth (one file to update)
- ✅ Pre-render support (overlays can modify Helm values)
- ✅ Consistency (same structure across all components)
- ✅ Version control (chart versions explicit, not discovered at runtime)

**See also:** [Architecture & Methodology FAQ - Component Architecture](FAQ_ARCHITECTURE.md#component-architecture)

---

## Troubleshooting

### My pod is stuck in Pending state

**Common causes: Resource quota exceeded, policy violation, image pull errors.**

**Diagnosis:**

```bash
# Check pod events
kubectl describe pod <pod-name> -n <namespace>

# Check tenant quota usage
kubectl describe tenant <tenant-name>

# Check policy violations
kubectl get policyreport -n <namespace>
```

**Common Issues:**

| Error | Cause | Solution |
|-------|-------|----------|
| `Insufficient cpu` | Quota exceeded | Delete unused pods or request quota increase |
| `ErrImagePull` | Unapproved registry | Use approved registry (e.g., nexus.fedcore.io) |
| `FailedScheduling` | No nodes available | Contact platform team (cluster capacity) |
| `Blocked by policy` | Kyverno violation | Fix policy violation (e.g., add resource limits) |

**See also:** [Troubleshooting Guide](TROUBLESHOOTING.md)

---

### How do I debug networking issues?

**Use network debugging tools and check network policies.**

**Debugging Steps:**

1. **Check pod-to-pod connectivity:**
   ```bash
   # Run debug pod
   kubectl run -it --rm debug --image=nicolaka/netshoot --restart=Never -- bash

   # Test connectivity
   curl http://<service-name>.<namespace>.svc.cluster.local
   ```

2. **Check service endpoints:**
   ```bash
   kubectl get endpoints <service-name> -n <namespace>
   ```

3. **Check network policies:**
   ```bash
   kubectl get networkpolicies -n <namespace>
   kubectl describe networkpolicy <policy-name> -n <namespace>
   ```

4. **Check DNS resolution:**
   ```bash
   kubectl exec -it <pod-name> -n <namespace> -- nslookup <service-name>
   ```

5. **Check Istio sidecar (if enabled):**
   ```bash
   kubectl logs <pod-name> -n <namespace> -c istio-proxy
   ```

**See also:** [Troubleshooting Guide](TROUBLESHOOTING.md), [Tenant Advanced Topics](TENANT_ADVANCED_TOPICS.md)

---

### Where do I find logs?

**Application logs in Splunk, system logs in CloudWatch/Azure Monitor.**

**Log Sources:**

1. **Application Logs:**
   - **Destination:** Splunk (configured per environment)
   - **Query:** `index=apps tenant=acme namespace=acme-prod`

2. **Kubernetes Events:**
   ```bash
   kubectl get events -n <namespace> --sort-by='.lastTimestamp'
   ```

3. **Pod Logs:**
   ```bash
   kubectl logs <pod-name> -n <namespace>
   kubectl logs <pod-name> -n <namespace> --previous  # Previous container
   ```

4. **Platform Component Logs:**
   - Kro: `kubectl logs -n kro-system -l app=kro-controller`
   - Kyverno: `kubectl logs -n kyverno -l app=kyverno`
   - Flux: `kubectl logs -n flux-system -l app=source-controller`

5. **AWS CloudTrail:**
   - AWS Console → CloudTrail → Event History
   - Query by tenant account ID

**See also:** [Runtime Security & Logging](RUNTIME_SECURITY_AND_LOGGING.md), [Troubleshooting](TROUBLESHOOTING.md)

---

## Getting Help

### How do I contact the platform team?

- **GitHub Issues:** File issues in the platform repository
- **GitHub Discussions:** For questions and general discussions
- **Docs:** [Troubleshooting Guide](TROUBLESHOOTING.md)

---

### Where is the documentation?

- **Handbook:** Start with [Handbook Intro](HANDBOOK_INTRO.md)
- **Glossary:** [Essential terminology](GLOSSARY.md)
- **Troubleshooting:** [Common issues and solutions](TROUBLESHOOTING.md)
- **API Reference:** [RGD schemas](DEVELOPMENT.md)

---

### How do I contribute to the platform?

- **Development Guide:** [Contributing workflow](DEVELOPMENT.md)
- **Create RGDs:** [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md)
- **Report Issues:** File GitHub issues in platform repository
- **Suggest Features:** Open a GitHub discussion or issue with feature request

---

## Additional Resources

### Documentation
- [📚 Handbook Intro](HANDBOOK_INTRO.md) - Complete platform guide
- [📖 Glossary](GLOSSARY.md) - Terminology reference
- [🏗️ Architecture & Methodology FAQ](FAQ_ARCHITECTURE.md) - Why we build the way we do
- [🎨 Architecture Diagrams](ARCHITECTURE_DIAGRAMS.md) - Visual guides
- [🔧 Troubleshooting](TROUBLESHOOTING.md) - Problem resolution

### Quick Starts
- [Admin Quick Start](QUICKSTART_ADMIN.md) - Tenant onboarding
- [Developer Quick Start](QUICKSTART_DEVELOPER.md) - Deploy first app
- [Architect Quick Start](QUICKSTART_ARCHITECT.md) - Platform design overview
- [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md) - Create RGDs

### Deep Dives
- [Security Overview](SECURITY_OVERVIEW.md) - Comprehensive security model
- [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md) - Tenant isolation
- [IAM Architecture](IAM_ARCHITECTURE.md) - Three-tier IAM design
- [Pod Identity](POD_IDENTITY_FULL.md) - AWS authentication

---

## Navigation

[← Previous: Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md) | [Next: Getting Started →](GETTING_STARTED.md)

**Handbook Progress:** Page 9 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
