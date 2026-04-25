use anyhow::{Result, Context, bail};
use colored::Colorize;
use similar::{ChangeTag, TextDiff};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::output;
use crate::commands::inspect;
use crate::commands::build;

pub fn execute(
    targets: Vec<String>,
    artifact: Option<String>,
    cluster: Option<String>,
    id: Option<String>,
    context: Option<String>,
    namespace: Option<String>,
    registry: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<()> {
    output::header("Compare");

    let local_build = match (artifact, cluster) {
        (Some(a), Some(c)) => Some((a, c)),
        (None, None) => None,
        _ => unreachable!("clap requires both --artifact and --cluster together"),
    };

    if let Some((artifact_path, cluster_dir)) = local_build {
        if targets.len() != 1 {
            bail!(
                "Provide exactly 1 target when using --artifact/--cluster (got {})",
                targets.len()
            );
        }

        output::section("Fetching remote artifact");
        let remote_dir = inspect::resolve_target(
            &targets[0],
            context.as_deref(),
            namespace.as_deref(),
            registry,
            username,
            password,
        )?;

        output::section("Building local artifact");
        let mut entries: Vec<_> = crate::commands::matrix::discover_cluster_artifacts(&cluster_dir)?
            .into_iter()
            .filter(|e| e.artifact_path == artifact_path)
            .collect();
        if let Some(id) = &id {
            entries.retain(|e| e.component_id == *id);
        }
        if entries.len() > 1 {
            let ids: Vec<_> = entries.iter().map(|e| e.component_id.as_str()).collect();
            bail!("Multiple instances found for '{}': {}. Use --id to select one.", artifact_path, ids.join(", "));
        }
        let entry = entries.first()
            .context(format!("Component '{}' not found in cluster config", artifact_path))?;
        let content = build::build_single_artifact(entry, false)
            .context("Local build failed")?;
        output::item_ok("Local build complete");

        let local_dir = tempfile::tempdir()?;
        fs::write(local_dir.path().join("platform.yaml"), &content)?;

        diff_dirs(remote_dir.path(), local_dir.path())
    } else {
        if targets.len() != 2 {
            bail!(
                "Provide 2 targets, or 1 target with --artifact/--cluster (got {})",
                targets.len()
            );
        }

        output::section("Fetching left artifact");
        let left_dir = inspect::resolve_target(
            &targets[0],
            context.as_deref(),
            namespace.as_deref(),
            registry.clone(),
            username.clone(),
            password.clone(),
        )?;

        output::section("Fetching right artifact");
        let right_dir = inspect::resolve_target(
            &targets[1],
            context.as_deref(),
            namespace.as_deref(),
            registry,
            username,
            password,
        )?;

        diff_dirs(left_dir.path(), right_dir.path())
    }
}

fn diff_dirs(left: &Path, right: &Path) -> Result<()> {
    output::section("Diff");

    let left_files = collect_files(left)?;
    let right_files = collect_files(right)?;

    let all_files: BTreeSet<_> = left_files.union(&right_files).cloned().collect();

    let mut added = 0usize;
    let mut removed = 0usize;
    let mut changed = 0usize;
    let mut identical = 0usize;

    for file in &all_files {
        let in_left = left_files.contains(file);
        let in_right = right_files.contains(file);

        match (in_left, in_right) {
            (true, false) => {
                removed += 1;
                eprintln!("    {} {}", "-".bright_red(), file.bright_red());
            }
            (false, true) => {
                added += 1;
                eprintln!("    {} {}", "+".bright_green(), file.bright_green());
            }
            (true, true) => {
                let left_content = fs::read_to_string(left.join(file))?;
                let right_content = fs::read_to_string(right.join(file))?;

                if left_content == right_content {
                    identical += 1;
                } else {
                    changed += 1;
                    eprintln!("    {} {}", "~".bright_yellow(), file.bright_yellow());
                    print_unified_diff(&left_content, &right_content);
                }
            }
            _ => unreachable!(),
        }
    }

    eprintln!();
    if added == 0 && removed == 0 && changed == 0 {
        output::done("Artifacts are identical");
    } else {
        let parts: Vec<String> = [
            (changed > 0).then(|| format!("{} changed", changed)),
            (added > 0).then(|| format!("{} added", added)),
            (removed > 0).then(|| format!("{} removed", removed)),
            (identical > 0).then(|| format!("{} identical", identical)),
        ]
        .into_iter()
        .flatten()
        .collect();

        output::done(&parts.join(", "));
    }

    Ok(())
}

fn collect_files(root: &Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    for entry in WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        files.insert(rel.display().to_string());
    }
    Ok(files)
}

fn print_unified_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        eprintln!(
            "      {} {}",
            "@@".dimmed(),
            format!("{}", hunk.header()).dimmed()
        );
        for change in hunk.iter_changes() {
            let (sign, line) = match change.tag() {
                ChangeTag::Delete => ("-", change.to_string_lossy().red()),
                ChangeTag::Insert => ("+", change.to_string_lossy().green()),
                ChangeTag::Equal => (" ", change.to_string_lossy().dimmed()),
            };
            let line = line.to_string();
            let line = line.trim_end_matches('\n');
            eprintln!("      {}{}", sign, line);
        }
    }
}
