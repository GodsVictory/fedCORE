# GitOps with Flux

FedCore follows a GitOps model using Flux CD. The flow is:

  Git  You commit cluster + component YAML to git
       │
  CI   CI runs fedcore build --all --push
       │    Renders artifacts and pushes to OCI registry
       │
  Flux Flux (running in-cluster) watches OCIRepository sources
       │    Configured during bootstrap
       │
  Flux Flux detects new artifact versions
       │    Pulls the OCI artifact
       │
  Flux Flux applies the manifests via Kustomization
            Reconciles desired state → actual state

# Key Flux Resources

  OCIRepository    Points Flux at an OCI artifact in your registry.
                   Created by bootstrap for each enabled component.

  Kustomization    Tells Flux to apply the manifests from an OCIRepository.
                   Handles dependencies, health checks, and pruning.

  Use fedcore status to see these resources on a live cluster.
