# What is FedCore?

FedCore is a platform CLI for managing Kubernetes deployments across
multiple clusters and clouds. It takes a declarative, GitOps-first
approach: you define your clusters and components as YAML, and FedCore
renders, packages, and deploys them as versioned OCI artifacts.

# Key Concepts

  Cluster    A Kubernetes cluster with its own config, cloud provider,
             region, environment, and tenant list. Lives under
             platform/clusters/<name>/cluster.yaml.

  Component  An infrastructure service (Capsule, Istio, monitoring, etc.)
             defined as a Helm chart or plain manifests. Lives under
             platform/components/<name>/.

  Bootstrap  The initial cluster setup — installs Flux, configures OCI
             sources, and applies cluster-specific overlays.

  Overlay    Cluster- or environment-specific YAML patches applied during
             the build pipeline via ytt.

  Artifact   The rendered output for a specific component + cluster
             combination, packaged as an OCI image and pushed to a registry.

# How It All Fits Together

  1. Define clusters and components as YAML
       │
  2. fedcore build renders each component for each cluster
       │         using ytt + helm template + overlays
       │
  3. fedcore build --push packages output as OCI artifacts
       │
  4. fedcore bootstrap installs Flux and configures the cluster
       │         to pull artifacts from the OCI registry
       │
  5. Flux watches the registry and applies changes automatically
