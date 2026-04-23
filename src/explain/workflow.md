# Workflow: From Zero to Running Cluster

  1. Create a new project
     $ fedcore init project

  2. Create a cluster configuration
     $ fedcore init cluster
     → Prompts for name, cloud, region, environment
     → Creates platform/clusters/<name>/cluster.yaml

  3. Add or customize components
     $ fedcore init component
     → Choose helm, manifests, or kustomize type
     → Edit platform/components/<name>/component.yaml

  4. Validate everything
     $ fedcore validate
     → Checks tools (ytt, helm, flux)
     → Validates schema, builds all components, generates all bootstraps

  5. Build artifacts
     $ fedcore build --all
     → Renders every component × cluster combination
     → Writes to dist/<component>-<cluster>.yaml

  6. Push to OCI registry (CI pipeline)
     $ fedcore build --all --push --registry registry.example.com
     → Packages each artifact and pushes to the registry

  7. Bootstrap a cluster
     $ fedcore bootstrap --cluster platform/clusters/my-cluster --deploy
     → Installs Flux, configures OCI sources, applies config

  8. Check status
     $ fedcore status
     → Shows OCIRepositories, Kustomizations, Deployments, Pods
