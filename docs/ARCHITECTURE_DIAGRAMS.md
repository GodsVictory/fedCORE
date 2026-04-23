# Architecture Diagrams

Visual architecture reference for the fedCORE platform. Use these diagrams to understand system components, data flows, and integration patterns.

---

## Platform Layers Diagram

The fedCORE platform is built in layers, with each layer building on the previous:

```mermaid
graph TD
    subgraph "Layer 4: Applications"
        A1[Web Applications]
        A2[Microservices]
        A3[Batch Jobs]
        A4[Data Pipelines]
    end

    subgraph "Layer 3: Developer Abstractions - RGDs"
        R1[WebApp RGD]
        R2[Database RGD]
        R3[Queue RGD]
        R4[Cache RGD]
        R5[Bucket RGD]
    end

    subgraph "Layer 2: Tenant Bootstrap"
        T1[Capsule Tenant]
        T2[Namespaces]
        T3[IAM Roles]
        T4[Resource Quotas]
        T5[Network Policies]
    end

    subgraph "Layer 1: Cluster Bootstrap"
        C1[Kro - RGD Runtime]
        C2[Capsule - Multi-tenancy]
        C3[Kyverno - Policies]
        C4[ACK/ASO - Cloud Controllers]
        C5[Tetragon - Runtime Security]
        C6[Flux - GitOps]
    end

    subgraph "Layer 0: Infrastructure"
        I1[Kubernetes - EKS/AKS/On-prem]
        I2[AWS Account]
        I3[Azure Subscription]
        I4[On-premises Data Center]
    end

    A1 --> R1
    A2 --> R2
    A3 --> R3
    A4 --> R4

    R1 --> T1
    R2 --> T1
    R3 --> T1
    R4 --> T1
    R5 --> T1

    T1 --> C1
    T2 --> C2
    T3 --> C4
    T4 --> C2
    T5 --> C3

    C1 --> I1
    C2 --> I1
    C3 --> I1
    C4 --> I2
    C4 --> I3
    C5 --> I1
    C6 --> I1

    classDef layer4Style fill:#2d5016,stroke:#90ee90,stroke-width:2px,color:#fff
    classDef layer3Style fill:#004080,stroke:#66b3ff,stroke-width:2px,color:#fff
    classDef layer2Style fill:#665200,stroke:#ffdb4d,stroke-width:2px,color:#fff
    classDef layer1Style fill:#4d0080,stroke:#b366ff,stroke-width:2px,color:#fff
    classDef layer0Style fill:#800040,stroke:#ff66b3,stroke-width:2px,color:#fff

    class A1,A2,A3,A4 layer4Style
    class R1,R2,R3,R4,R5 layer3Style
    class T1,T2,T3,T4,T5 layer2Style
    class C1,C2,C3,C4,C5,C6 layer1Style
    class I1,I2,I3,I4 layer0Style
```

**Key Takeaways:**
- Each layer abstracts complexity from the layer above
- Bootstrap layers (0-2) are managed by platform team
- RGDs (Layer 3) are created by platform engineers
- Applications (Layer 4) are deployed by developers

---

## Pod Identity Flow

How pods authenticate with AWS services using EKS Pod Identity:

```mermaid
sequenceDiagram
    participant Pod as Application Pod
    participant Agent as Pod Identity Agent
    participant CR as Cluster IAM Role
    participant TR as Tenant IAM Role
    participant AWS as AWS Service (RDS/S3)

    Note over Pod,AWS: 1. Pod Starts with ServiceAccount
    Pod->>Agent: Request AWS credentials
    Note over Agent: Agent reads ServiceAccount<br/>and finds Pod Identity Association

    Note over Pod,AWS: 2. Cluster Role Assumes Tenant Role
    Agent->>CR: Assume cluster IAM role
    CR->>TR: Assume cross-account tenant role
    Note over TR: Uses external ID for security<br/>fedcore-cluster-tenant

    Note over Pod,AWS: 3. Temporary Credentials Injected
    TR-->>Agent: Return temporary credentials (STS)
    Agent-->>Pod: Inject credentials as env vars<br/>AWS_ACCESS_KEY_ID<br/>AWS_SECRET_ACCESS_KEY<br/>AWS_SESSION_TOKEN

    Note over Pod,AWS: 4. Pod Accesses AWS Resources
    Pod->>AWS: API call with credentials
    AWS-->>Pod: Resource access granted
    Note over AWS: Credentials expire after 1 hour<br/>Agent automatically refreshes
```

**Components:**
- **Pod Identity Agent:** DaemonSet running on every node
- **Cluster IAM Role:** Lives in cluster AWS account, can assume tenant roles
- **Tenant IAM Role:** Lives in tenant AWS account, restricted by permission boundary
- **External ID:** Prevents "confused deputy" attacks

**See Also:** [Pod Identity Full Documentation](POD_IDENTITY_FULL.md)

---

## Multi-Account Architecture

Tenant isolation using dedicated AWS accounts:

```mermaid
graph TB
    subgraph "Cluster Account - 111111111111"
        EKS[EKS Cluster]
        PODS[Application Pods]
        ACK[ACK Controllers]
        FLUX[Flux GitOps]

        subgraph "Cluster IAM Roles"
            CLUSTERROLE[EKS Pod Identity Role]
        end
    end

    subgraph "Tenant A Account - 222222222222"
        subgraph "AWS Resources"
            RDS_A[RDS Database]
            S3_A[S3 Buckets]
            DDB_A[DynamoDB Tables]
        end

        subgraph "Tenant IAM Roles"
            ACK_ROLE_A[ACK Provisioner Role]
            APP_ROLE_A[Application IAM Role]
            DEPLOYER_A[Tenant Deployer Role]
        end

        BOUNDARY_A[Permission Boundary]
    end

    subgraph "Tenant B Account - 333333333333"
        subgraph "AWS Resources"
            RDS_B[RDS Database]
            S3_B[S3 Buckets]
            SQS_B[SQS Queues]
        end

        subgraph "Tenant IAM Roles"
            ACK_ROLE_B[ACK Provisioner Role]
            APP_ROLE_B[Application IAM Role]
            DEPLOYER_B[Tenant Deployer Role]
        end

        BOUNDARY_B[Permission Boundary]
    end

    subgraph "GitOps Flow"
        GIT[Git Repository]
        CICD[CI/CD Pipeline]
        NEXUS[Nexus OCI Registry]
    end

    GIT -->|1. Push| CICD
    CICD -->|2. Build artifacts| NEXUS
    FLUX -->|3. Pull artifacts| NEXUS
    FLUX -->|4. Apply manifests| PODS

    PODS -->|5. Request AWS access| CLUSTERROLE
    CLUSTERROLE -.->|6. Assume role| ACK_ROLE_A
    CLUSTERROLE -.->|6. Assume role| ACK_ROLE_B

    ACK -->|7. Provision resources| ACK_ROLE_A
    ACK -->|7. Provision resources| ACK_ROLE_B

    ACK_ROLE_A -->|8. Create resources| RDS_A
    ACK_ROLE_A -->|8. Create resources| S3_A
    ACK_ROLE_A -->|8. Create IAM roles| APP_ROLE_A

    ACK_ROLE_B -->|8. Create resources| RDS_B
    ACK_ROLE_B -->|8. Create resources| S3_B
    ACK_ROLE_B -->|8. Create IAM roles| APP_ROLE_B

    BOUNDARY_A -.->|Restricts| ACK_ROLE_A
    BOUNDARY_A -.->|Restricts| APP_ROLE_A
    BOUNDARY_A -.->|Restricts| DEPLOYER_A

    BOUNDARY_B -.->|Restricts| ACK_ROLE_B
    BOUNDARY_B -.->|Restricts| APP_ROLE_B
    BOUNDARY_B -.->|Restricts| DEPLOYER_B

    classDef clusterStyle fill:#004080,stroke:#66b3ff,stroke-width:2px,color:#fff
    classDef tenantStyle fill:#800040,stroke:#ff66b3,stroke-width:2px,color:#fff
    classDef resourceStyle fill:#2d5016,stroke:#90ee90,stroke-width:2px,color:#fff
    classDef roleStyle fill:#665200,stroke:#ffdb4d,stroke-width:2px,color:#fff
    classDef gitopsStyle fill:#4d0080,stroke:#b366ff,stroke-width:2px,color:#fff
    classDef boundaryStyle fill:#800000,stroke:#ff6666,stroke-width:2px,color:#fff

    class EKS,PODS,ACK,FLUX,CLUSTERROLE clusterStyle
    class RDS_A,S3_A,DDB_A,RDS_B,S3_B,SQS_B resourceStyle
    class ACK_ROLE_A,APP_ROLE_A,DEPLOYER_A,ACK_ROLE_B,APP_ROLE_B,DEPLOYER_B roleStyle
    class GIT,CICD,NEXUS gitopsStyle
    class BOUNDARY_A,BOUNDARY_B boundaryStyle
```

**Key Points:**
- Each tenant has a dedicated AWS account
- Cluster account hosts Kubernetes workloads
- ACK controllers assume cross-account roles to provision resources
- Permission boundaries prevent privilege escalation
- Tenants cannot access each other's AWS accounts

**See Also:** [Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)

---

## Security Layers

Defense-in-depth security model with seven isolation layers:

```mermaid
graph TD
    subgraph "Layer 7: Audit & Compliance"
        L7A[CloudTrail Logging]
        L7B[AWS Config Rules]
        L7C[Splunk Integration]
        L7D[Security Dashboards]
    end

    subgraph "Layer 6: Runtime Monitoring"
        L6A[Tetragon eBPF]
        L6B[Process Monitoring]
        L6C[Network Monitoring]
        L6D[File System Monitoring]
    end

    subgraph "Layer 5: Admission Control"
        L5A[Kyverno Policies]
        L5B[Image Registry Validation]
        L5C[Security Baselines]
        L5D[Resource Limits]
    end

    subgraph "Layer 4: IAM Isolation"
        L4A[Permission Boundaries]
        L4B[Least-Privilege Roles]
        L4C[External ID Validation]
        L4D[Temporary Credentials]
    end

    subgraph "Layer 3: Network Isolation"
        L3A[Network Policies]
        L3B[Service Mesh - Istio]
        L3C[Ingress Controls]
        L3D[DNS Policies]
    end

    subgraph "Layer 2: Namespace Isolation"
        L2A[Capsule Tenants]
        L2B[Resource Quotas]
        L2C[RBAC Boundaries]
        L2D[Namespace Labels]
    end

    subgraph "Layer 1: Account Isolation"
        L1A[Dedicated AWS Accounts]
        L1B[Cross-Account Roles]
        L1C[SCPs - Service Control Policies]
        L1D[Billing Isolation]
    end

    L7A --> L6A
    L7B --> L6B
    L7C --> L6C
    L7D --> L6D

    L6A --> L5A
    L6B --> L5B
    L6C --> L5C
    L6D --> L5D

    L5A --> L4A
    L5B --> L4B
    L5C --> L4C
    L5D --> L4D

    L4A --> L3A
    L4B --> L3B
    L4C --> L3C
    L4D --> L3D

    L3A --> L2A
    L3B --> L2B
    L3C --> L2C
    L3D --> L2D

    L2A --> L1A
    L2B --> L1B
    L2C --> L1C
    L2D --> L1D

    classDef layer7Style fill:#2d5016,stroke:#90ee90,stroke-width:2px,color:#fff
    classDef layer6Style fill:#004080,stroke:#66b3ff,stroke-width:2px,color:#fff
    classDef layer5Style fill:#665200,stroke:#ffdb4d,stroke-width:2px,color:#fff
    classDef layer4Style fill:#800040,stroke:#ff66b3,stroke-width:2px,color:#fff
    classDef layer3Style fill:#4d0080,stroke:#b366ff,stroke-width:2px,color:#fff
    classDef layer2Style fill:#803300,stroke:#ff9966,stroke-width:2px,color:#fff
    classDef layer1Style fill:#003d4d,stroke:#66cccc,stroke-width:2px,color:#fff

    class L7A,L7B,L7C,L7D layer7Style
    class L6A,L6B,L6C,L6D layer6Style
    class L5A,L5B,L5C,L5D layer5Style
    class L4A,L4B,L4C,L4D layer4Style
    class L3A,L3B,L3C,L3D layer3Style
    class L2A,L2B,L2C,L2D layer2Style
    class L1A,L1B,L1C,L1D layer1Style
```

**Defense-in-Depth Philosophy:**
- **Multiple layers:** Compromise of one layer doesn't breach security
- **Fail-safe:** If admission control fails, runtime monitoring detects
- **Audit trail:** All security events logged and alerted
- **Preventive + Detective:** Block bad actions, detect anomalies

**See Also:** [Security Overview](SECURITY_OVERVIEW.md)

---

## GitOps Workflow

End-to-end deployment pipeline from git commit to cluster:

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant Git as GitHub Repo
    participant CI as GitHub Actions
    participant Scan as Security Scans
    participant Nexus as Nexus OCI Registry
    participant Flux as Flux in Cluster
    participant K8s as Kubernetes API

    Dev->>Git: 1. Commit and push changes
    Note over Git: Branch: main<br/>Path: platform/rgds/webapp/

    Git->>CI: 2. Trigger workflow
    Note over CI: Workflow: build-and-deploy.yml

    CI->>CI: 3. Checkout code
    CI->>CI: 4. Validate YAML (ytt)

    CI->>Scan: 5. Security scanning
    Note over Scan: - Kyverno policy check<br/>- Image vulnerability scan<br/>- Secret detection

    CI->>CI: 6. Build OCI artifact per cluster
    Note over CI: ytt -f base/ -f overlays/aws/<br/>flux push artifact

    CI->>Nexus: 7. Push artifacts
    Note over Nexus: OCI URL: nexus.io/platform/webapp:v1.2.3

    Note over Flux: Flux polls every 10 minutes
    Flux->>Nexus: 8. Check for new artifacts
    Nexus-->>Flux: New version available

    Flux->>Nexus: 9. Pull artifact
    Flux->>Flux: 10. Extract manifests

    Flux->>K8s: 11. Apply manifests
    Note over K8s: - Kyverno validates<br/>- Kro processes RGDs<br/>- ACK provisions cloud resources

    K8s-->>Flux: 12. Apply succeeded
    Flux->>Git: 13. Update status (optional)
```

**Pipeline Stages:**

1. **Source:** Developer commits changes to git
2. **Validate:** CI validates YAML syntax and schemas
3. **Security:** Scans for vulnerabilities, secrets, policy violations
4. **Build:** ytt generates cloud-specific manifests
5. **Package:** Flux bundles manifests into OCI artifacts
6. **Publish:** Artifacts pushed to Nexus OCI registry
7. **Sync:** Flux pulls artifacts and applies to cluster
8. **Reconcile:** Kro and ACK/ASO provision resources

**See Also:** [Deployment Pipeline](DEPLOYMENT.md)

---

## Tenant Isolation

How Capsule, Kyverno, IAM, and Network Policies enforce tenant isolation:

```mermaid
graph TB
    subgraph "Tenant A - acme"
        subgraph "Capsule Tenant: acme"
            NS_A1[acme-dev]
            NS_A2[acme-prod]
            NS_A3[acme-cicd]
        end

        subgraph "Namespace Resources"
            POD_A[Pods]
            SVC_A[Services]
            ING_A[Ingress]
        end

        subgraph "Tenant Quotas"
            QUOTA_A[CPU: 100 cores<br/>Memory: 200Gi<br/>Namespaces: 10]
        end

        subgraph "Network Policies"
            NP_A[Allow within tenant<br/>Deny cross-tenant]
        end
    end

    subgraph "Tenant B - globex"
        subgraph "Capsule Tenant: globex"
            NS_B1[globex-dev]
            NS_B2[globex-staging]
            NS_B3[globex-cicd]
        end

        subgraph "Namespace Resources"
            POD_B[Pods]
            SVC_B[Services]
            ING_B[Ingress]
        end

        subgraph "Tenant Quotas"
            QUOTA_B[CPU: 50 cores<br/>Memory: 100Gi<br/>Namespaces: 5]
        end

        subgraph "Network Policies"
            NP_B[Allow within tenant<br/>Deny cross-tenant]
        end
    end

    subgraph "Platform Enforcement"
        CAPSULE[Capsule Controller]
        KYVERNO[Kyverno Policies]
        TETRAGON[Tetragon Runtime Security]
    end

    subgraph "Cross-Cutting Controls"
        RBAC[RBAC: Tenant Owners<br/>can only access own namespaces]
        IAM[IAM: Pods can only access<br/>own tenant's AWS account]
        AUDIT[Audit Logs: All actions<br/>tracked per tenant]
    end

    CAPSULE -->|Enforces namespace naming| NS_A1
    CAPSULE -->|Enforces namespace naming| NS_B1
    CAPSULE -->|Enforces quotas| QUOTA_A
    CAPSULE -->|Enforces quotas| QUOTA_B

    KYVERNO -->|Validates resources| POD_A
    KYVERNO -->|Validates resources| POD_B
    KYVERNO -->|Blocks violations| SVC_A
    KYVERNO -->|Blocks violations| SVC_B

    TETRAGON -->|Monitors runtime| POD_A
    TETRAGON -->|Monitors runtime| POD_B

    NP_A -.->|Blocks| POD_B
    NP_B -.->|Blocks| POD_A

    RBAC -.->|Restricts| NS_A1
    RBAC -.->|Restricts| NS_B1

    IAM -.->|Isolates| POD_A
    IAM -.->|Isolates| POD_B

    classDef tenantAStyle fill:#2d5016,stroke:#90ee90,stroke-width:2px,color:#fff
    classDef tenantBStyle fill:#004080,stroke:#66b3ff,stroke-width:2px,color:#fff
    classDef platformStyle fill:#665200,stroke:#ffdb4d,stroke-width:2px,color:#fff
    classDef controlStyle fill:#4d0080,stroke:#b366ff,stroke-width:2px,color:#fff

    class NS_A1,NS_A2,NS_A3,POD_A,SVC_A,ING_A,QUOTA_A,NP_A tenantAStyle
    class NS_B1,NS_B2,NS_B3,POD_B,SVC_B,ING_B,QUOTA_B,NP_B tenantBStyle
    class CAPSULE,KYVERNO,TETRAGON platformStyle
    class RBAC,IAM,AUDIT controlStyle
```

**Isolation Mechanisms:**

1. **Capsule:** Enforces namespace naming (`tenant-*` pattern), quotas, and ownership
2. **Network Policies:** Deny all traffic between tenant namespaces
3. **RBAC:** Tenant owners can only access their own namespaces
4. **IAM:** Pods can only assume roles in their tenant's AWS account
5. **Kyverno:** Validates all resources comply with security policies
6. **Tetragon:** Monitors runtime for suspicious cross-tenant access attempts

**See Also:** [Tenant Admin Guide](TENANT_ADMIN_GUIDE.md), [Security Overview](SECURITY_OVERVIEW.md)

---

## RGD Composition

How RGDs compose multiple resources into a single abstraction:

```mermaid
graph LR
    subgraph "Developer Interface"
        DEV[Developer creates<br/>WebApp manifest]
    end

    subgraph "RGD Layer - WebApp"
        WEBAPP[WebApp RGD]
    end

    subgraph "Generated Kubernetes Resources"
        DEPLOY[Deployment]
        SVC[Service]
        ING[Ingress]
        CM[ConfigMap]
        SECRET[Secret]
        SA[ServiceAccount]
    end

    subgraph "Nested RGDs"
        DB_RGD[Database RGD]
        CACHE_RGD[Cache RGD]
    end

    subgraph "Cloud Resources - AWS"
        RDS[RDS Instance]
        ELASTICACHE[ElastiCache]
        IAM_ROLE[IAM Role]
        SG[Security Group]
    end

    subgraph "Cloud Resources - Azure"
        AZURE_SQL[Azure SQL]
        REDIS_CACHE[Azure Cache]
        MI[Managed Identity]
        NSG[Network Security Group]
    end

    DEV -->|kubectl apply| WEBAPP

    WEBAPP -->|Generates| DEPLOY
    WEBAPP -->|Generates| SVC
    WEBAPP -->|Generates| ING
    WEBAPP -->|Generates| CM
    WEBAPP -->|Generates| SECRET
    WEBAPP -->|Generates| SA

    WEBAPP -->|Composes| DB_RGD
    WEBAPP -->|Composes| CACHE_RGD

    DB_RGD -->|AWS| RDS
    DB_RGD -->|Azure| AZURE_SQL

    CACHE_RGD -->|AWS| ELASTICACHE
    CACHE_RGD -->|Azure| REDIS_CACHE

    RDS -->|Needs| IAM_ROLE
    RDS -->|Needs| SG

    AZURE_SQL -->|Needs| MI
    AZURE_SQL -->|Needs| NSG

    classDef devStyle fill:#2d5016,stroke:#90ee90,stroke-width:2px,color:#fff
    classDef rgdStyle fill:#004080,stroke:#66b3ff,stroke-width:2px,color:#fff
    classDef k8sStyle fill:#665200,stroke:#ffdb4d,stroke-width:2px,color:#fff
    classDef nestedStyle fill:#4d0080,stroke:#b366ff,stroke-width:2px,color:#fff
    classDef awsStyle fill:#803300,stroke:#ff9966,stroke-width:2px,color:#fff
    classDef azureStyle fill:#004d80,stroke:#66b3ff,stroke-width:2px,color:#fff

    class DEV devStyle
    class WEBAPP rgdStyle
    class DEPLOY,SVC,ING,CM,SECRET,SA k8sStyle
    class DB_RGD,CACHE_RGD nestedStyle
    class RDS,ELASTICACHE,IAM_ROLE,SG awsStyle
    class AZURE_SQL,REDIS_CACHE,MI,NSG azureStyle
```

**RGD Composition Benefits:**
- **Abstraction:** Single manifest creates entire stack
- **Reusability:** WebApp RGD reuses Database and Cache RGDs
- **Cloud Portability:** Same manifest works across AWS, Azure, on-prem
- **Best Practices:** RGDs enforce organizational standards
- **Versioning:** RGDs can evolve without breaking existing apps

**See Also:** [Development Guide](DEVELOPMENT.md), [Platform Engineer Quick Start](QUICKSTART_PLATFORM_ENGINEER.md)

---

## Additional Resources

### Related Documentation
- **[fedCORE Purposes](FEDCORE_PURPOSES.md)** - Platform overview and design goals
- **[Multi-Account Architecture](MULTI_ACCOUNT_ARCHITECTURE.md)** - Deep dive into account isolation
- **[Security Overview](SECURITY_OVERVIEW.md)** - Comprehensive security model
- **[Pod Identity](POD_IDENTITY_FULL.md)** - AWS authentication mechanism
- **[Deployment Pipeline](DEPLOYMENT.md)** - GitOps workflow details

### Diagram Sources

All diagrams use Mermaid syntax and can be edited in any Markdown editor with Mermaid support:
- [Mermaid Live Editor](https://mermaid.live/)
- VS Code with Mermaid extension
- GitHub renders Mermaid natively

### Contributing

To add new diagrams:
1. Use Mermaid syntax for consistency
2. Include a "Key Points" or "Key Takeaways" summary
3. Link to related documentation
4. Test rendering in GitHub preview

---

## Navigation

[← Previous: fedCORE Purposes](FEDCORE_PURPOSES.md) | [Next: Admin Quick Start →](QUICKSTART_ADMIN.md)

**Handbook Progress:** Page 4 of 35 | **Level 1:** Foundation & Quick Starts

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
