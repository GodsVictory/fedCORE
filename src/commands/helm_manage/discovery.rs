use anyhow::Result;
use std::fs;
use walkdir::WalkDir;
use serde::Deserialize;
use serde_json::Value;
use crate::paths;

pub struct ComponentInfo {
    pub name: String,
    pub chart: String,
    pub repo: String,
    pub version: String,
    pub component_path: String,
}

pub fn discover_components() -> Result<Vec<ComponentInfo>> {
    let mut components = Vec::new();

    for entry in WalkDir::new(paths::COMPONENTS_DIR)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let component_file = entry.path().join("component.yaml");
        if !component_file.exists() {
            continue;
        }

        let name = entry
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let content = fs::read_to_string(&component_file)?;
        let data = parse_ytt_yaml(&content);

        let chart = data["helm"]["chart"].as_str().unwrap_or("").to_string();
        let repo = data["helm"]["sourceRepo"].as_str().unwrap_or("").to_string();
        let version = data["helm"]["version"].as_str().unwrap_or("").to_string();

        if name.is_empty() || chart.is_empty() || repo.is_empty() {
            continue;
        }

        let component_path = entry
            .path()
            .strip_prefix("platform")
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        components.push(ComponentInfo {
            name,
            chart,
            repo,
            version,
            component_path,
        });
    }

    Ok(components)
}

fn parse_ytt_yaml(content: &str) -> Value {
    for doc in serde_yaml::Deserializer::from_str(content) {
        if let Ok(value) = Value::deserialize(doc) {
            if !value.is_null() {
                return value;
            }
        }
    }
    Value::Null
}

pub fn get_repo_name(name: &str) -> String {
    name.to_lowercase().replace("_", "-")
}
