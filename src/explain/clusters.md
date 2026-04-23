# Clusters

A cluster represents a Kubernetes cluster with its own identity,
cloud provider, and set of enabled components.

# cluster.yaml

  The main config file uses ytt data values format:

  #@data/values
  ---
  cluster_name: "my-cluster"
  cloud: aws
  region: us-east-1
  environment: prod
  overlays:
    - aws
    - prod

  components:
    - name: capsule
      enabled: true
    - name: kro
      enabled: true

# Cluster Overlays

Place ytt overlay files in <cluster>/overlays/ to customize any
rendered manifest for that specific cluster. These are applied as
the final step of the build pipeline (post-render).

# Overlay IDs

The overlays list (e.g., [aws, prod]) selects overlay directories
from each component's overlays/<id>/ directory. This lets you maintain
one component definition with per-cloud and per-environment customizations,
and add arbitrary overlay dimensions without changing the build tool.
