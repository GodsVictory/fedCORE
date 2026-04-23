# Development Guide

This guide covers development workflows, testing procedures, and contribution guidelines for the fedCORE Platform.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Testing](#testing)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Common Tasks](#common-tasks)
- [Troubleshooting](#troubleshooting)

---

## Getting Started

### Prerequisites

Install the required tools:

```bash
# ytt (required)
wget -O- https://carvel.dev/install.sh | bash

# Flux CLI (required for OCI operations)
curl -s https://fluxcd.io/install.sh | sudo bash

# yamllint (recommended)
pip install yamllint

# yq (required for scripts - Python version)
pip install yq

# kubectl (recommended)
# Follow instructions at https://kubernetes.io/docs/tasks/tools/

# jq (required for scripts)
sudo apt-get install jq  # Debian/Ubuntu
brew install jq          # macOS
```

### Clone the Repository

```bash
git clone https://github.com/fedcore/app-factory.git
cd app-factory
```

### Verify Your Setup

```bash
# Run validation
fedcore validate

# Discover build targets
fedcore matrix

# Build a test RGD artifact
fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-prod-use1 > dist/test-rgd.yaml

# Generate test bootstrap configuration
fedcore bootstrap --cluster platform/clusters/fedcore-lab-01 > dist/test-bootstrap.yaml
```

---

## Development Environment

### Directory Structure

```
app-factory/
├── platform/clusters/           # Flat directory of cluster files
├── templates/
│   ├── infrastructure/ # Tier 1: Infrastructure templates
│   └── rgds/          # Tier 2: RGD templates
├── scripts/            # Automation tools
└── .github/workflows/  # CI/CD pipelines
```

### ytt Basics

ytt is our templating engine. Key concepts:

```yaml
# Data value reference
#@ data.values.cloud_region

# Overlay to append resources
#@overlay/append
- id: newResource
  template:
    apiVersion: v1
    kind: ConfigMap

# Load libraries
#@ load("@ytt:overlay", "overlay")
#@ load("@ytt:data", "data")
```

Learn more: [ytt documentation](https://carvel.dev/ytt/docs/latest/)

---

## Testing

### Local Validation

Always validate before committing:

```bash
# Full validation suite
fedcore validate

# Build specific artifact
fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-prod-use1 > /tmp/test.yaml
fedcore bootstrap --cluster platform/clusters/fedcore-lab-01 > /tmp/test.yaml

# Check YAML syntax
yamllint templates/ platform/clusters/
```

### Testing RGD Templates

```bash
# Test base templates
ytt -f platform/rgds/webapps/base

# Test AWS overlay
ytt -f platform/rgds/webapps/base \
    -f platform/rgds/webapps/overlays/aws

# Test complete RGD build
fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-prod-use1 > /tmp/webapps-fedcore-prod-use1.yaml
ytt -f /tmp/webapps-fedcore-prod-use1.yaml  # Validate it's valid YAML
```

### Testing Infrastructure Templates

```bash
# Test bootstrap generation for a specific cluster
fedcore bootstrap --cluster platform/clusters/fedcore-lab-01 > /tmp/bootstrap.yaml

# Validate the generated configuration
ytt -f /tmp/bootstrap.yaml

# Check for required resources
grep -q "kind: TenantOnboarding" /tmp/bootstrap.yaml && echo "✓ Tenant definitions found"
grep -q "kind: OCIRepository" /tmp/bootstrap.yaml && echo "✓ Component sources found"
```

### Testing in a Cluster

```bash
# Build the bootstrap configuration
fedcore bootstrap --cluster platform/clusters/fedcore-lab-01 > dist/test.yaml

# Apply to test cluster
kubectl apply -f dist/test.yaml

# Watch for resource creation
kubectl get pods -n kro-system -w
kubectl get ocirepositories -n flux-system -w
```

---

## Making Changes

### Adding a New Cloud Provider

1. Create overlay directory:
   ```bash
   mkdir -p platform/rgds/webapps/overlays/gcp
   ```

2. Create overlay file with cloud-specific resources:
   ```yaml
   # platform/rgds/webapps/overlays/gcp/overlay.yaml
   #@ load("@ytt:overlay", "overlay")
   #@overlay/match by=overlay.subset({"kind": "ResourceGraphDefinition"})
   ---
   spec:
     #@overlay/match missing_ok=True
     resources:
       #@overlay/append
       - id: appBucket
         template:
           apiVersion: storage.cnpg.io/v1
           kind: Bucket
   ```

3. Add cluster configuration:
   ```bash
   cat > platform/clusters/fedcore-dev-gcp1.yaml <<EOF
   #@data/values
   ---
   cluster_name: "fedcore-dev-gcp1"
   cloud: gcp
   region: us-central1
   ingress_domain: "dev.gcp.fedcore.io"

   min_replicas: 2
   max_replicas: 5

   rgds:
     webapps:
       enabled: true
       version: "1.0.0"
   EOF
   ```

4. Test the build:
   ```bash
   fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-dev-gcp1 > dist/test-gcp.yaml
   fedcore bootstrap --cluster platform/clusters/fedcore-dev-gcp1 > dist/test-gcp-bootstrap.yaml
   fedcore validate
   ```

### Adding a Resource to an RGD

Example: Adding PostgreSQL to all clouds

1. Update base RGD (if schema changes needed):
   ```yaml
   # platform/rgds/webapps/base/rgd.yaml
   spec:
     schema:
       spec:
         postgresEnabled: boolean | default=false
         postgresVersion: string | default="15"
         postgresStorage: integer | default=20
   ```

2. Update AWS overlay:
   ```yaml
   # platform/rgds/webapps/overlays/aws/overlay.yaml
   #@overlay/append
   - id: appPostgres
     template:
       apiVersion: rds.services.k8s.aws/v1alpha1
       kind: DBInstance
       spec:
         engine: postgres
         engineVersion: ${schema.spec.postgresVersion}
         allocatedStorage: ${schema.spec.postgresStorage}
   ```

3. Update Azure overlay:
   ```yaml
   # platform/rgds/webapps/overlays/azure/overlay.yaml
   #@overlay/append
   - id: appPostgres
     template:
       apiVersion: dbforpostgresql.azure.com/v1api20230601
       kind: FlexibleServer
       spec:
         version: ${schema.spec.postgresVersion}
   ```

4. Update OnPrem overlay:
   ```yaml
   # platform/rgds/webapps/overlays/onprem/overlay.yaml
   #@overlay/append
   - id: appPostgres
     template:
       apiVersion: postgresql.cnpg.io/v1
       kind: Cluster
       spec:
         instances: 1
         postgresql:
           version: ${schema.spec.postgresVersion}
   ```

5. Validate all targets:
   ```bash
   fedcore validate
   ```

### Adding a New Cluster

1. Create cluster file:
   ```bash
   cat > platform/clusters/fedcore-staging-use1.yaml <<EOF
   #@data/values
   ---
   cluster_name: "fedcore-staging-use1"
   cloud: aws
   region: us-east-1
   ingress_domain: "staging.us-east-1.fedcore.io"

   # Staging-specific configuration
   min_replicas: 3
   max_replicas: 10

   rgds:
     webapps:
       enabled: true
       version: "1.0.0"  # Will promote to newer versions after testing
   EOF
   ```

2. Create GitHub Environment:
   - Name: `fedcore-staging-use1` (matches cluster_name)
   - Add AWS credentials as secrets
   - Optionally add approval requirements

3. Test locally:
   ```bash
   fedcore bootstrap --cluster platform/clusters/fedcore-staging-use1 > dist/staging-test.yaml
   ```

4. Commit and push - CI will handle the rest

### Adding a New RGD Template

1. Create directory structure:
   ```bash
   mkdir -p platform/rgds/databases/{base,overlays/{aws,azure,onprem}}
   ```

2. Create base RGD with Kro schema:
   ```yaml
   # platform/rgds/databases/base/rgd.yaml
   #@data/values
   ---
   apiVersion: kro.run/v1alpha1
   kind: ResourceGraphDefinition
   metadata:
     name: databases
   spec:
     schema:
       spec:
         engine: string | default="postgres"
         version: string | default="15"
         storage: integer | default=20
         backupEnabled: boolean | default=true
     resources: []
   ```

3. Create cloud-specific overlays with actual resources

4. Enable in cluster files:
   ```yaml
   rgds:
     webapps:
       enabled: true
       version: "1.0.0"
     databases:  # New RGD
       enabled: true
       version: "0.1.0"
   ```

5. Validate:
   ```bash
   fedcore matrix
   fedcore validate
   ```

---

## Pull Request Process

### Before Submitting

1. Run validation:
   ```bash
   fedcore validate
   ```

2. Test affected artifacts:
   ```bash
   # If you changed RGD templates
   fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-prod-use1 > /tmp/test.yaml

   # If you changed component templates
   fedcore build --artifact platform/components/kro --cluster platform/clusters/fedcore-lab-01 > /tmp/test.yaml

   # If you added a cluster
   fedcore bootstrap --cluster platform/clusters/YOUR_NEW_CLUSTER > /tmp/test.yaml
   ```

3. Check for secrets:
   ```bash
   # Ensure no secrets in cluster files
   grep -ri "password\|secret\|token\|key" platform/clusters/
   ```

4. Update documentation if needed

### PR Guidelines

- **Title**: Use conventional commit format
  - `feat: Add PostgreSQL to webapps RGD`
  - `fix: Correct ytt syntax in AWS overlay`
  - `docs: Update README with new examples`
  - `refactor: Simplify cluster structure`

- **Description**: Include:
  - What changed and why
  - Which artifacts/clusters are affected
  - Testing performed
  - Breaking changes (if any)

- **Size**: Keep PRs focused and reasonably sized
  - Prefer multiple small PRs over one large PR

### Review Process

1. Automated checks must pass:
   - Linting (yamllint)
   - Validation (ytt builds)
   - Security scanning

2. Code review from at least one maintainer

3. Manual testing for infrastructure changes

4. Approval required before merge

---

## Common Tasks

### Building All Artifacts Locally

```bash
mkdir -p dist

# Build all component artifacts
fedcore matrix | jq -c '.build_matrix[]' | while read -r artifact; do
  ARTIFACT_PATH=$(echo "$artifact" | jq -r '.artifact_path')
  CLUSTER=$(echo "$artifact" | jq -r '.cluster')
  TARGET_NAME=$(echo "$artifact" | jq -r '.target_name')
  echo "Building: ${TARGET_NAME}"
  fedcore build --artifact "$ARTIFACT_PATH" --cluster "$CLUSTER" > "dist/${TARGET_NAME}.yaml"
done

# Build all bootstrap configurations
fedcore matrix | jq -r '.cluster_matrix[] | .cluster' | while read cluster; do
  cluster_name=$(basename "$cluster")
  echo "Building bootstrap: $cluster_name"
  fedcore bootstrap --cluster "$cluster" > "dist/${cluster_name}-bootstrap.yaml"
done
```

### Comparing Artifacts Across Environments

```bash
# Compare dev vs prod for same RGD
diff \
  <(fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-dev-use1) \
  <(fedcore build --artifact platform/rgds/webapps --cluster platform/clusters/fedcore-prod-use1)

# Compare bootstrap configurations
diff \
  <(fedcore bootstrap --cluster platform/clusters/fedcore-dev-use1) \
  <(fedcore bootstrap --cluster platform/clusters/fedcore-prod-use1)
```

### Debugging ytt Issues

```bash
# Enable debug output
ytt -f platform/rgds/webapps/base --debug

# Check data values
ytt -f platform/clusters/fedcore-prod-use1.yaml --data-values-inspect

# Validate overlay syntax
ytt -f platform/rgds/webapps/base -f platform/rgds/webapps/overlays/aws
```

### Testing Discovery Script Changes

```bash
# Run discovery
fedcore matrix

# Pretty print output
fedcore matrix | jq '.'

# Check specific matrix
fedcore matrix | jq '.rgd_matrix'
fedcore matrix | jq '.infra_matrix'
```

---

## Troubleshooting

### Build Failures

**ytt compilation errors:**
```bash
# Check syntax
ytt -f platform/rgds/webapps/base

# Check overlay matching
ytt -f platform/rgds/webapps/base -f platform/rgds/webapps/overlays/aws --debug
```

**Missing overlay:**
```bash
# Ensure overlay directory exists
ls platform/rgds/webapps/overlays/aws/
```

**Cluster file errors:**
```bash
# Validate cluster YAML
yq -r '.cluster_name' platform/clusters/fedcore-prod-use1.yaml

# Check for required fields
yq -r '.cloud' platform/clusters/fedcore-prod-use1.yaml
yq -r '.region' platform/clusters/fedcore-prod-use1.yaml
```

### CI/CD Issues

**Workflow not triggering:**
- Check branch protection rules
- Verify workflow file syntax
- Check GitHub Actions permissions

**Matrix discovery failures:**
```bash
# Test discovery locally
fedcore matrix | jq '.'

# Check JSON formatting
fedcore matrix | jq . > /dev/null && echo "Valid JSON"
```

**Artifact push failures:**
- Verify Vault credentials are configured
- Check Nexus registry authentication
- Ensure artifact size is under OCI limits

### Validation Failures

**"Cluster not discovered":**
- Verify file is in `platform/clusters/` directory
- Check file has `.yaml` extension
- Ensure `cluster_name`, `cloud`, and `region` fields exist

**"Invalid YAML":**
- Run yamllint on the file
- Check indentation and syntax
- Validate with `ytt -f <file>`

**"RGD build failed":**
- Check if overlay exists for the cloud
- Validate overlay syntax with ytt
- Check for ytt annotation errors

---

## Code Style

### YAML Formatting

- Use 2 spaces for indentation
- Keep lines under 120 characters
- Use `---` document separator
- Comment complex logic

### ytt Conventions

- Use `#@` for ytt annotations (no space)
- Use `# ` for regular comments (with space)
- Group related overlays together
- Document overlay purpose with comments

### File Naming

- Use kebab-case: `cluster.yaml`, `overlay.yaml`, `rgd.yaml`
- Cluster files: `<cluster_name>.yaml`
- Match directory names to cloud providers

### Kro Schema Variables

- Always use Kro schema variables with defaults in RGDs
- Never use ytt data values in RGD templates
- Document schema fields with descriptions

---

## Getting Help

- Check [README.md](README.md) for architecture overview
- Check [.claude.md](.claude.md) for complete project context
- Review existing examples in the repository

---

## Navigation

[← Previous: Deployment Guide](DEPLOYMENT.md) | [Next: CI/CD Role Zero Permissions →](CICD_ROLE_ZERO_PERMISSIONS.md)

**Handbook Progress:** Page 17 of 35 | **Level 4:** Deployment & Development

[📚 Back to Handbook](HANDBOOK_INTRO.md) | [📖 Glossary](GLOSSARY.md) | [🔧 Troubleshooting](TROUBLESHOOTING.md)
- Check ytt documentation: https://carvel.dev/ytt/docs/latest/
- File issues on GitHub for bugs or questions