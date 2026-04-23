# Bootstrap

Bootstrap prepares a bare cluster for GitOps by installing Flux
and configuring it to watch your OCI registry for component artifacts.

# What It Does

  1. Generates Flux install manifests (if flux.install: true)
     Points Flux at your private registry for airgapped installs
     Use exclude_kinds to filter out resource types you can't create

  2. Collects component overlay.yaml files
     If a component has an overlay.yaml in its root directory,
     it is passed to ytt as a data values overlay (e.g., depends_on)

  3. Renders component-source templates via ytt
     Creates OCIRepository + Kustomization for each enabled component
     so Flux knows where to pull artifacts from

  4. Applies cluster-specific overlays
     Resource limits, annotations, cloud-specific patches

  5. Substitutes secrets from environment variables
     OCI_DOCKERCONFIG_JSON, SPLUNK_HEC_HOST, SPLUNK_HEC_TOKEN

  6. With --deploy, applies everything via kubectl

# Component Dependencies

  Dependencies are declared in an overlay.yaml file in the component
  root directory. This is a standard ytt data values overlay:

    #@data/values
    ---
    #@overlay/match missing_ok=True
    components:
    #@overlay/match by=lambda idx,old,new: old["name"] == "namespace"
    - depends_on:
      - kro

  Bootstrap automatically detects and includes these overlays.
  The same result can be reproduced manually:

    ytt -f schema.yaml -f cluster.yaml \
        -f component-sources/base/ \
        -f components/my-component/overlay.yaml

# Namespace-Scoped Flux (--admin-prep)

  For clusters where you don't have cluster-admin, use --admin-prep to
  generate a minimal manifest for the cluster administrator to apply.

  It includes only cluster-scoped prerequisites:
    - Flux CRDs (OCIRepository, Kustomization)
    - Flux namespace and ServiceAccounts
    - Namespace-scoped Roles/RoleBindings for Flux controllers
    - Deployer RBAC in each target namespace

  Target namespaces are derived automatically by building each enabled
  component and extracting namespace fields from the rendered output.

  Use exclude_kinds to control which resources the regular bootstrap
  skips (since admin-prep already handles them):

    flux:
      install: true
      exclude_kinds:
        - Namespace
        - CustomResourceDefinition
        - ClusterRole
        - ClusterRoleBinding
        - ServiceAccount
        - NetworkPolicy
        - ResourceQuota

  Then generate and hand off:

    fedcore bootstrap -c platform/clusters/my-cluster --admin-prep

  After the admin applies the output, run the normal bootstrap
  (without --admin-prep) to deploy Flux controllers and components.
