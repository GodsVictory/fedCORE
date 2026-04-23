# Project Structure

  my-project/
  ├── platform/
  │   ├── clusters/schema.yaml   Cluster schema (shared)
  │   ├── clusters/
  │   │   └── <name>/
  │   │       ├── cluster.yaml   Cluster config (ytt data values)
  │   │       └── overlays/      Cluster-specific ytt overlays
  │   ├── components/
  │   │   └── <name>/
  │   │       ├── component.yaml Helm/manifest config
  │   │       ├── base/          Static manifests and ytt templates
  │   │       └── overlays/
  │   │           ├── <aws|azure|onprem>/
  │   │           └── <dev|staging|prod>/
  │   ├── rgds/                  Resource Group Definitions
  │   └── bootstrap/
  │       └── component-sources/ OCI source templates for Flux
  └── dist/                      Build output (gitignored)
