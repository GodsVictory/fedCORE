use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ClusterConfig {
    pub cluster_name: String,
    #[serde(default)]
    pub flux: FluxConfig,
    #[serde(default)]
    pub overlays: Vec<String>,
    #[serde(default)]
    pub components: Vec<ComponentEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FluxConfig {
    #[serde(default = "default_true")]
    pub install: bool,
    #[serde(default = "default_flux_namespace")]
    pub namespace: String,
    #[serde(default)]
    pub exclude_kinds: Vec<String>,
}

impl Default for FluxConfig {
    fn default() -> Self {
        Self {
            install: true,
            namespace: default_flux_namespace(),
            exclude_kinds: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentEntry {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub helm_flags: Option<Vec<String>>,
}

impl ComponentEntry {
    pub fn id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    pub fn namespace(&self) -> &str {
        self.namespace.as_deref().unwrap_or_else(|| self.id())
    }

}

fn default_true() -> bool {
    true
}
fn default_flux_namespace() -> String {
    "flux-system".to_string()
}

#[derive(Debug, Deserialize)]
pub struct MergedComponent {
    #[serde(default)]
    pub helm: Option<HelmConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HelmConfig {
    pub chart: String,
    pub version: String,
    #[serde(rename = "sourceRepo")]
    pub source_repo: String,
    #[serde(rename = "resolvedChartRef", default)]
    pub resolved_chart_ref: String,
    #[serde(default = "default_empty_object")]
    pub values: serde_json::Value,
    #[serde(default)]
    pub flags: Vec<String>,
}

fn default_empty_object() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMatrixEntry {
    pub artifact_path: String,
    pub cluster: String,
    pub cluster_name: String,
    pub target_name: String,
    #[serde(default)]
    pub component_id: String,
    #[serde(default)]
    pub component_namespace: String,
    #[serde(default)]
    pub helm_flags: Option<Vec<String>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMatrixEntry {
    pub cluster: String,
    pub cluster_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildMatrix {
    pub build_matrix: Vec<BuildMatrixEntry>,
    pub cluster_matrix: Vec<ClusterMatrixEntry>,
}
