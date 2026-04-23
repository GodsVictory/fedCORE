# Components

A component is an infrastructure service deployed to one or more clusters.
Each component lives in platform/components/<name>/ and has:

  component.yaml   Defines the component type, helm chart details,
                   release name, namespace, and values.

  base/            Static Kubernetes manifests and ytt templates that
                   get merged with the helm output.

  overlays/        Overlay patches selected by cluster overlays:
                   overlays/aws/patch.yaml
                   overlays/prod/patch.yaml

  overlay.yaml     Optional ytt data values overlay applied at bootstrap.
                   Used for component-level config like depends_on.
                   Automatically detected and passed to ytt.

# Component Types

  helm        component.yaml specifies a Helm chart + repo + version.
              The build pipeline runs helm template, then applies
              base/ manifests and overlays via ytt.

  manifests   No Helm chart. Just raw YAML in base/ processed by ytt
              with cluster data values.

  kustomize   (placeholder) Kustomize-based rendering.

# Overlay Phases

Overlays can run at two points in the build pipeline:

  pre-render   Applied BEFORE helm template (modifies component.yaml values)
  post-render  Applied AFTER helm template (patches rendered manifests)

Set the phase with a comment at the top of the overlay file:
  #! overlay-phase: pre-render
