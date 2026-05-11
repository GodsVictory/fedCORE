// Embedded template files for project initialization

// --- Bootstrap Templates ---

pub const BOOTSTRAP_README: &str = include_str!("../../../platform/bootstrap/component-sources/base/README.md");
pub const BOOTSTRAP_COMPONENT_SOURCES: &str = include_str!("../../../platform/bootstrap/component-sources/base/component-sources.yaml");
pub const BOOTSTRAP_CLUSTER_META_KUSTOMIZATION: &str = include_str!("../../../platform/bootstrap/cluster/base/meta-kustomization.yaml");
pub const BOOTSTRAP_CLUSTER_SECRETS: &str = include_str!("../../../platform/bootstrap/cluster/base/bootstrap-secrets.yaml");
pub const BOOTSTRAP_CLUSTER_FLUX_CA_CERTS: &str = include_str!("../../../platform/bootstrap/cluster/base/flux-ca-certificates.yaml");

// --- Cluster Schema ---

pub const CLUSTER_SCHEMA: &str = include_str!("../../../platform/clusters/schema-minimal.yaml");

// --- Tenant Instances Component ---

pub const TENANT_INSTANCES_README: &str = include_str!("../../../platform/components/tenant-instances/README.md");
pub const TENANT_INSTANCES_NAMESPACE_YAML: &str = include_str!("../../../platform/components/tenant-instances/base/namespace.yaml");
pub const TENANT_INSTANCES_TENANT_YAML: &str = include_str!("../../../platform/components/tenant-instances/base/tenant.yaml");


// --- Capsule Component ---

pub const CAPSULE_README: &str = include_str!("../../../platform/components/capsule/README.md");
pub const CAPSULE_COMPONENT_YAML: &str = include_str!("../../../platform/components/capsule/component.yaml");
pub const CAPSULE_NAMESPACE_YAML: &str = include_str!("../../../platform/components/capsule/base/namespace.yaml");
pub const CAPSULE_DEFAULT_VALUES: &str = include_str!("../../../platform/components/capsule/default-values.yaml");

// --- KRO Component ---

pub const KRO_README: &str = include_str!("../../../platform/components/kro/README.md");
pub const KRO_INSTALL_YAML: &str = include_str!("../../../platform/components/kro/base/install.yaml");
pub const KRO_CORE_RBAC: &str = include_str!("../../../platform/components/kro/base/core-resources-rbac.yaml");
pub const KRO_PLATFORM_RBAC: &str = include_str!("../../../platform/components/kro/base/platform-fedcore-rbac.yaml");
pub const KRO_DEFAULT_ROLES_RBAC: &str = include_str!("../../../platform/components/kro/base/default-clusterroles-rbac.yaml");
pub const KRO_ENABLE_CRD_DELETION: &str = include_str!("../../../platform/components/kro/base/enable-crd-deletion.yaml");
pub const KRO_IMAGE_OVERLAY: &str = include_str!("../../../platform/components/kro/base/image-overlay.yaml");
