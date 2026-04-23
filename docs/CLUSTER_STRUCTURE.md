# Cluster Directory Structure

## Overview

Clusters are organized in individual directories with a main configuration file and subdirectories for modular configuration. This structure allows for better organization, cleaner git diffs, and easier management as your platform scales.

## Directory Structure

```
platform/clusters/
├── fedcore-prod-use1/          # AWS production cluster
│   ├── cluster.yaml            # Main cluster configuration
│   └── tenants/                # Tenant definitions
│       ├── acme.yaml
│       └── platform-team.yaml
├── fedcore-prod-azeus/         # Azure production cluster
│   ├── cluster.yaml
│   └── tenants/
│       └── acme.yaml
├── fedcore-lab-01/             # On-prem lab cluster
│   ├── cluster.yaml
│   └── tenants/
│       └── test-tenant.yaml
└── fedcore-prod-usw2/          # Additional cluster (example)
    ├── cluster.yaml
    └── tenants/
        └── example.yaml
```

## Directory Format

Each cluster directory follows this pattern:

```
platform/clusters/<cluster_name>/
├── cluster.yaml      # Required: Main cluster configuration
└── tenants/          # Optional: Tenant definitions
    ├── tenant1.yaml  # One file per tenant
    ├── tenant2.yaml
    └── tenant3.yaml
```

**Key points:**
- Directory name should match the `cluster_name` field in `cluster.yaml`
- `cluster.yaml` is required and contains all cluster-level configuration
- `tenants/` is optional but recommended for multi-tenancy
- One YAML file per tenant in the `tenants/` subdirectory

## Why This Structure?

### Scalability

- **Modular configuration**: Break large configs into logical pieces
- **One tenant per file**: Clean git diffs when adding/removing tenants
- **Easy to extend**: Add new subdirectories for other resources (namespaces, policies, etc.)

### Git-Friendly

```bash
# Adding a new tenant shows only what changed
git add platform/clusters/fedcore-prod-use1/tenants/new-tenant.yaml
git diff --cached
# Shows only the new tenant config, not the entire cluster file
```

### Multiple Clusters Per Environment

You can still have multiple clusters in the same environment:

```
platform/clusters/
├── fedcore-prod-use1/          # Primary production cluster
├── fedcore-prod-use1-blue/     # Blue/green deployment
└── fedcore-prod-use1-canary/   # Canary cluster
```

### Discovery

The discovery script finds all cluster directories and extracts metadata from `cluster.yaml`:

```json
{
  "target_name": "fedcore-prod-use1",
  "cluster": "platform/clusters/fedcore-prod-use1",
  "cloud": "aws",
  "region": "us-east-1",
  "env": "prod",
  "cluster_name": "fedcore-prod-use1",
  "tenant_count": 2
}
```

## Cluster Configuration

### cluster.yaml

The main cluster configuration file contains all cluster-level settings:

```yaml
#@data/values
---
#! Physical facts
cluster_name: "fedcore-prod-use1"
cloud: aws
region: us-east-1
ingress_domain: "prod.us-east-1.fedcore.io"

#! Configuration
min_replicas: 5
max_replicas: 20

#! Tenant policies
tenant_policies:
  enforce_image_registry: true
  allowed_registries:
    - "nexus.fedcore.io/tenant-"
  require_resource_limits: true

#! RGD Deployments
rgds:
  webapps:
    enabled: true
    version: "1.0.0"
```

**Key fields:**
- `cluster_name`: Must match the directory name
- `cloud`: Cloud provider (`aws`, `azure`, `onprem`)
- `region`: Cloud region or datacenter identifier
- All other cluster-level configuration

### Tenant Files

Each tenant is defined in a separate file in the `tenants/` subdirectory:

**File:** `platform/clusters/fedcore-prod-use1/tenants/acme.yaml`

```yaml
#@data/values
---
#! Tenant: acme
#! Owner: Acme Corp Engineering Team

tenants:
  acme:
    owners:
      - kind: User
        name: john@acme-corp.com
        apiGroup: rbac.authorization.k8s.io
      - kind: Group
        name: acme-admins
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 10
    resources:
      cpu: "100"
      memory: "200Gi"
      storage: "1Ti"
      max_pvcs: 50
    cost_center: "engineering"
    billing_contact: "finance@acme-corp.com"
    allow_loadbalancer: true
```

**Important:** Each tenant file uses the same `#@data/values` and `tenants:` structure. ytt will merge all data values together when building the artifact.

## Adding a New Cluster

Create a new cluster directory with the required structure:

```bash
# 1. Create cluster directory
mkdir -p platform/clusters/fedcore-dev-usw2/tenants

# 2. Create cluster.yaml
cat > platform/clusters/fedcore-dev-usw2/cluster.yaml <<'EOF'
#@data/values
---
cluster_name: "fedcore-dev-usw2"
cloud: aws
region: us-west-2
ingress_domain: "dev.us-west-2.fedcore.io"

min_replicas: 2
max_replicas: 5

tenant_policies:
  enforce_image_registry: false
  allow_internet_egress: true

monitoring_enabled: true

rgds:
  webapps:
    enabled: true
    version: "1.0.0"
EOF

# 3. (Optional) Add tenants
cat > platform/clusters/fedcore-dev-usw2/tenants/dev-team.yaml <<'EOF'
#@data/values
---
tenants:
  dev-team:
    owners:
      - kind: Group
        name: developers
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 5
    resources:
      cpu: "50"
      memory: "100Gi"
      storage: "500Gi"
    cost_center: "development"
EOF

# 4. Commit and push
git add platform/clusters/fedcore-dev-usw2/
git commit -m "Add fedcore-dev-usw2 cluster"
git push origin main
```

**Next steps:**
1. Create GitHub Environment named `fedcore-dev-usw2`
2. Add cloud-specific secrets to the environment
3. CI/CD automatically discovers and deploys

## Adding a Tenant to Existing Cluster

Simply add a new YAML file to the cluster's `tenants/` directory:

```bash
# Create new tenant file
cat > platform/clusters/fedcore-prod-use1/tenants/data-science.yaml <<'EOF'
#@data/values
---
#! Tenant: data-science
#! Owner: Data Science Team

tenants:
  data-science:
    owners:
      - kind: Group
        name: data-scientists
        apiGroup: rbac.authorization.k8s.io
    namespace_quota: 8
    resources:
      cpu: "80"
      memory: "160Gi"
      storage: "2Ti"
      max_pvcs: 40
    cost_center: "analytics"
    billing_contact: "team-name"
    allow_loadbalancer: false
EOF

# Commit and push
git add platform/clusters/fedcore-prod-use1/tenants/data-science.yaml
git commit -m "Add data-science tenant to fedcore-prod-use1"
git push origin main
```

The CI/CD pipeline automatically:
1. Discovers the cluster has changed
2. Rebuilds the infrastructure artifact (includes new tenant)
3. Deploys to the cluster via Flux

## Removing a Tenant

Delete the tenant file and commit:

```bash
git rm platform/clusters/fedcore-prod-use1/tenants/old-tenant.yaml
git commit -m "Remove old-tenant from fedcore-prod-use1"
git push origin main
```

The tenant will be removed from the infrastructure artifact, but **Capsule Tenant resource will remain in the cluster** until manually deleted (for safety).

## GitHub Environment Names

GitHub Environments use the **cluster name** (from `cluster.yaml`) as the environment name:

| Cluster Directory | Environment Name |
|------------------|------------------|
| `platform/clusters/fedcore-prod-use1/` | `fedcore-prod-use1` |
| `platform/clusters/fedcore-prod-azeus/` | `fedcore-prod-azeus` |
| `platform/clusters/fedcore-lab-01/` | `fedcore-lab-01` |

## Artifact Names

Infrastructure artifacts are named after the cluster:

| Cluster | Artifact Name |
|---------|---------------|
| `fedcore-prod-use1` | `fedcore-prod-use1.yaml` |
| `fedcore-prod-azeus` | `fedcore-prod-azeus.yaml` |
| `fedcore-lab-01` | `fedcore-lab-01.yaml` |

RGD artifacts are built per-cluster (with cloud-specific overlays applied):

| RGD + Cluster | Artifact Name |
|---------------|---------------|
| `webapps` + `fedcore-prod-use1` | `rgd-webapps-fedcore-prod-use1.yaml` |
| `webapps` + `fedcore-prod-azeus` | `rgd-webapps-fedcore-prod-azeus.yaml` |
| `tenant` + `fedcore-prod-use1` | `rgd-tenant-fedcore-prod-use1.yaml` |
| `tenant` + `fedcore-prod-azeus` | `rgd-tenant-fedcore-prod-azeus.yaml` |

## Naming Conventions

### Cluster Directory Names

Follow this pattern: `<org>-<env>-<region-code>`

**Examples:**
- `fedcore-prod-use1` - Production in US East 1
- `fedcore-dev-usw2` - Development in US West 2
- `fedcore-lab-01` - Lab environment, datacenter 01

### Tenant File Names

Use descriptive, kebab-case names:

**Examples:**
- `acme.yaml` - Simple tenant name
- `platform-team.yaml` - Hyphenated name
- `data-science.yaml` - Multi-word tenant

**Avoid:**
- `tenant-acme.yaml` - Redundant prefix
- `Acme.yaml` - Capital letters
- `acme_corp.yaml` - Underscores

### Best Practices

1. **Consistent naming**: All clusters follow the same pattern
2. **Include region code**: Makes it clear where the cluster is
3. **Keep it short**: Used in many places (environments, artifacts, logs)
4. **No special characters**: Alphanumeric and hyphens only
5. **Lowercase**: Easier to type and consistent

## Troubleshooting

### Error: "cluster directory not found"

Verify the directory exists:
```bash
ls -la platform/clusters/fedcore-prod-use1/
```

### Missing cluster.yaml

Each cluster directory must have a `cluster.yaml` file:
```bash
ls -la platform/clusters/fedcore-prod-use1/cluster.yaml
```

### Cluster Name Mismatch

Your directory name should match the `cluster_name` field:
```bash
# Directory name: fedcore-prod-use1
yq -r '.cluster_name' platform/clusters/fedcore-prod-use1/cluster.yaml
# Should output: fedcore-prod-use1
```

### Cluster Not Discovered

Check that:
1. Directory is in `platform/clusters/` directory
2. Directory contains `cluster.yaml` file
3. `cluster.yaml` has valid YAML with required fields
4. Run discovery manually: `fedcore matrix`

### Building Infrastructure Artifact

Test building the artifact locally:
```bash
fedcore bootstrap --cluster platform/clusters/fedcore-prod-use1 > /tmp/test.yaml
```

Check the output for errors.

### ytt Data Values Merging

ytt automatically merges all `#@data/values` files. You can test this:

```bash
# Show merged data values
ytt -f platform/clusters/fedcore-prod-use1/ --data-values-inspect
```

---

## Navigation

[← Previous: Getting Started](GETTING_STARTED.md) | [Next: Environment Setup →](ENVIRONMENT_SETUP.md)

**Handbook Progress:** Page 11 of 35 | **Level 2:** Platform Setup & Structure

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)

This shows how cluster.yaml and all tenant files are merged together.
