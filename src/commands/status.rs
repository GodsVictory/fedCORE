use anyhow::{Result, Context, bail};
use std::process::Command;
use serde_json::Value;
use crate::commands;
use crate::output;

const LABEL: &str = "app.kubernetes.io/part-of=fedcore";

pub fn execute(
    cluster: Option<String>,
    namespace: Option<String>,
    component: Option<String>,
) -> Result<()> {
    output::header("Status");

    if !commands::command_exists("kubectl") {
        bail!("kubectl is not installed or not in PATH");
    }

    let context = match cluster {
        Some(ref c) => c.clone(),
        None => commands::get_current_context()?,
    };

    output::summary(&context);

    if let Some(comp) = component {
        show_component_status(&context, namespace.as_deref(), &comp)?;
    } else {
        show_cluster_overview(&context, namespace.as_deref())?;
    }

    Ok(())
}

fn kubectl_get(context: &str, args: &[&str]) -> Result<Option<Value>> {
    let mut full_args = vec!["--context", context, "--request-timeout=10s"];
    full_args.extend_from_slice(args);
    output::cmd("kubectl", &full_args);
    let result = Command::new("kubectl")
        .arg("--context")
        .arg(context)
        .arg("--request-timeout=10s")
        .args(args)
        .output()
        .context("Failed to execute kubectl")?;

    if !result.status.success() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_slice(&result.stdout)?))
}

fn list_args<'a>(
    resource: &'a str,
    label: &'a str,
    namespace: Option<&'a str>,
) -> Vec<&'a str> {
    let mut args = vec!["get", resource, "-l", label, "-o", "json"];
    if let Some(ns) = namespace {
        args.push("-n");
        args.push(ns);
    } else {
        args.push("-A");
    }
    args
}

fn show_cluster_overview(context: &str, namespace: Option<&str>) -> Result<()> {
    let ns_args = vec!["get", "namespaces", "-l", LABEL, "-o", "json"];
    let oci_args = list_args("ocirepository", LABEL, namespace);
    let kust_args = list_args("kustomization", LABEL, namespace);
    let deploy_args = list_args("deployments", LABEL, namespace);
    let sts_args = list_args("statefulsets", LABEL, namespace);
    let ds_args = list_args("daemonsets", LABEL, namespace);
    let pod_args = list_args("pods", LABEL, namespace);

    let (ns_json, oci_json, kust_json, deploy_json, sts_json, ds_json, pod_json) =
        std::thread::scope(|s| {
            let ns = s.spawn(|| kubectl_get(context, &ns_args));
            let oci = s.spawn(|| kubectl_get(context, &oci_args));
            let kust = s.spawn(|| kubectl_get(context, &kust_args));
            let deploy = s.spawn(|| kubectl_get(context, &deploy_args));
            let sts = s.spawn(|| kubectl_get(context, &sts_args));
            let ds = s.spawn(|| kubectl_get(context, &ds_args));
            let pod = s.spawn(|| kubectl_get(context, &pod_args));

            (
                ns.join().unwrap(),
                oci.join().unwrap(),
                kust.join().unwrap(),
                deploy.join().unwrap(),
                sts.join().unwrap(),
                ds.join().unwrap(),
                pod.join().unwrap(),
            )
        });

    output::section("Namespaces");
    render_namespaces(ns_json?)?;

    output::section("OCIRepository Sources");
    render_resources(oci_json?, "ocirepositories", true)?;

    output::section("Kustomizations");
    render_resources(kust_json?, "kustomizations", false)?;

    output::section("Deployments");
    render_workloads(deploy_json?, "deployments", WorkloadKind::ReplicaBased)?;

    output::section("StatefulSets");
    render_workloads(sts_json?, "statefulsets", WorkloadKind::ReplicaBased)?;

    output::section("DaemonSets");
    render_workloads(ds_json?, "daemonsets", WorkloadKind::DaemonSet)?;

    output::section("Pods");
    render_pods(pod_json?)?;

    Ok(())
}

fn show_component_status(
    context: &str,
    namespace: Option<&str>,
    component: &str,
) -> Result<()> {
    output::section(&format!("Component: {}", component));

    let label = format!("fedcore.io/component={}", component);
    let oci_args = list_args("ocirepository", &label, namespace);
    let kust_args = list_args("kustomization", &label, namespace);
    let deploy_args = list_args("deployments", &label, namespace);
    let sts_args = list_args("statefulsets", &label, namespace);
    let ds_args = list_args("daemonsets", &label, namespace);
    let pod_args = list_args("pods", &label, namespace);

    let (oci_json, kust_json, deploy_json, sts_json, ds_json, pod_json) =
        std::thread::scope(|s| {
            let oci = s.spawn(|| kubectl_get(context, &oci_args));
            let kust = s.spawn(|| kubectl_get(context, &kust_args));
            let deploy = s.spawn(|| kubectl_get(context, &deploy_args));
            let sts = s.spawn(|| kubectl_get(context, &sts_args));
            let ds = s.spawn(|| kubectl_get(context, &ds_args));
            let pod = s.spawn(|| kubectl_get(context, &pod_args));

            (
                oci.join().unwrap(),
                kust.join().unwrap(),
                deploy.join().unwrap(),
                sts.join().unwrap(),
                ds.join().unwrap(),
                pod.join().unwrap(),
            )
        });

    output::log("OCIRepository");
    render_resources(oci_json?, "ocirepositories", true)?;

    output::log("Kustomization");
    render_resources(kust_json?, "kustomizations", false)?;

    output::log("Deployments");
    render_workloads(deploy_json?, "deployments", WorkloadKind::ReplicaBased)?;

    output::log("StatefulSets");
    render_workloads(sts_json?, "statefulsets", WorkloadKind::ReplicaBased)?;

    output::log("DaemonSets");
    render_workloads(ds_json?, "daemonsets", WorkloadKind::DaemonSet)?;

    output::log("Pods");
    render_pods(pod_json?)?;

    Ok(())
}

fn extract_items(json: Option<Value>, resource_name: &str) -> Option<Vec<Value>> {
    let json = match json {
        Some(json) => json,
        None => {
            output::item_warn(&format!("Could not query {}", resource_name));
            return None;
        }
    };

    match json["items"].as_array() {
        Some(items) if !items.is_empty() => Some(items.clone()),
        _ => {
            output::item_warn("None found");
            None
        }
    }
}

fn render_namespaces(json: Option<Value>) -> Result<()> {
    let items = match extract_items(json, "namespaces") {
        Some(items) => items,
        None => return Ok(()),
    };

    for item in &items {
        let name = item["metadata"]["name"].as_str().unwrap_or("unknown");
        let phase = item["status"]["phase"].as_str().unwrap_or("Unknown");
        output::item_status(phase == "Active", name);
    }

    Ok(())
}

fn render_resources(
    json: Option<Value>,
    resource_name: &str,
    show_detail: bool,
) -> Result<()> {
    let items = match extract_items(json, resource_name) {
        Some(items) => items,
        None => return Ok(()),
    };

    for item in &items {
        let name = item["metadata"]["name"].as_str().unwrap_or("unknown");
        let ns = item["metadata"]["namespace"].as_str().unwrap_or("unknown");
        let ready = item["status"]["conditions"]
            .as_array()
            .and_then(|conds| conds.iter().find(|c| c["type"].as_str() == Some("Ready")))
            .and_then(|c| c["status"].as_str())
            .unwrap_or("Unknown");

        output::item_status(ready == "True", &format!("{}/{}", ns, name));

        if show_detail {
            let url = item["spec"]["url"].as_str().unwrap_or("unknown");
            let revision = item["status"]["artifact"]["revision"]
                .as_str()
                .unwrap_or("N/A");
            output::detail(&format!("{} @ {}", url, revision));
        }
    }

    Ok(())
}

enum WorkloadKind {
    ReplicaBased,
    DaemonSet,
}

fn render_workloads(
    json: Option<Value>,
    resource_name: &str,
    kind: WorkloadKind,
) -> Result<()> {
    let items = match extract_items(json, resource_name) {
        Some(items) => items,
        None => return Ok(()),
    };

    for item in &items {
        let name = item["metadata"]["name"].as_str().unwrap_or("unknown");
        let ns = item["metadata"]["namespace"].as_str().unwrap_or("unknown");

        let (ready, total) = match kind {
            WorkloadKind::ReplicaBased => (
                item["status"]["readyReplicas"].as_i64().unwrap_or(0),
                item["status"]["replicas"].as_i64().unwrap_or(0),
            ),
            WorkloadKind::DaemonSet => (
                item["status"]["numberReady"].as_i64().unwrap_or(0),
                item["status"]["desiredNumberScheduled"]
                    .as_i64()
                    .unwrap_or(0),
            ),
        };

        let ok = ready == total && total > 0;
        output::item_status(ok, &format!("{}/{} ({}/{})", ns, name, ready, total));
    }

    Ok(())
}

fn render_pods(json: Option<Value>) -> Result<()> {
    let items = match extract_items(json, "pods") {
        Some(items) => items,
        None => return Ok(()),
    };

    for item in &items {
        let name = item["metadata"]["name"].as_str().unwrap_or("unknown");
        let ns = item["metadata"]["namespace"].as_str().unwrap_or("unknown");
        let phase = item["status"]["phase"].as_str().unwrap_or("Unknown");

        let containers = item["status"]["containerStatuses"].as_array();
        let (all_ready, total_restarts, worst_state) = match containers {
            Some(statuses) => {
                let all_ready = statuses
                    .iter()
                    .all(|c| c["ready"].as_bool().unwrap_or(false));
                let restarts: i64 = statuses
                    .iter()
                    .map(|c| c["restartCount"].as_i64().unwrap_or(0))
                    .sum();
                let worst = statuses.iter().find_map(|c| {
                    if let Some(waiting) = c["state"]["waiting"]["reason"].as_str() {
                        Some(waiting.to_string())
                    } else if let Some(terminated) = c["state"]["terminated"]["reason"].as_str() {
                        if !c["ready"].as_bool().unwrap_or(false) {
                            Some(terminated.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
                (all_ready, restarts, worst)
            }
            None => (false, 0, None),
        };

        let ok =
            matches!(phase, "Running" | "Succeeded") && all_ready && worst_state.is_none();
        let status_str = worst_state.as_deref().unwrap_or(phase);
        let restart_suffix = if total_restarts > 0 {
            format!(", {} restarts", total_restarts)
        } else {
            String::new()
        };
        output::item_status(
            ok,
            &format!("{}/{} ({}{})", ns, name, status_str, restart_suffix),
        );
    }

    Ok(())
}
