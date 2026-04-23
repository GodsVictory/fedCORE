# Build Pipeline

For each component × cluster combination, the build runs:

  ┌─────────────────────────────────────────────────────────┐
  │ 1. Read cluster.yaml + component.yaml via ytt           │
  │         Apply pre-render overlays (modify values)        │
  │                         │                                │
  │                         ▼                                │
  │ 2. helm template with merged values                      │
  │         (skipped for plain manifest components)          │
  │                         │                                │
  │                         ▼                                │
  │ 3. Merge helm output with base/ manifests via ytt        │
  │                         │                                │
  │                         ▼                                │
  │ 4. Apply post-render overlays (patch manifests)          │
  │         + cluster-specific overlays                      │
  │                         │                                │
  │                         ▼                                │
  │ 5. Write to dist/<component>-<cluster>.yaml              │
  └─────────────────────────────────────────────────────────┘

# Push (CI)

With --push, the pipeline additionally:
  - Packages dist/<name>.yaml into an OCI layout
  - Pushes to oci://<registry>/fedcore/<name>:<version> via flux push
  - Tags with git source, ref, and SHA for traceability
