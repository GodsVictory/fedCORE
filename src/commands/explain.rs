use anyhow::Result;
use colored::Colorize;

const OVERVIEW: &str = include_str!("../explain/overview.md");
const TOPICS: &str = include_str!("../explain/topics.md");
const WORKFLOW: &str = include_str!("../explain/workflow.md");
const STRUCTURE: &str = include_str!("../explain/structure.md");
const COMPONENTS: &str = include_str!("../explain/components.md");
const CLUSTERS: &str = include_str!("../explain/clusters.md");
const BUILD: &str = include_str!("../explain/build.md");
const BOOTSTRAP: &str = include_str!("../explain/bootstrap.md");
const GITOPS: &str = include_str!("../explain/gitops.md");

pub fn execute(topic: Option<&str>) -> Result<()> {
    match topic {
        None => {
            render(OVERVIEW);
            render(TOPICS);
        }
        Some("workflow") => render(WORKFLOW),
        Some("structure") => render(STRUCTURE),
        Some("components") => render(COMPONENTS),
        Some("clusters") => render(CLUSTERS),
        Some("build") => render(BUILD),
        Some("bootstrap") => render(BOOTSTRAP),
        Some("gitops") => render(GITOPS),
        Some(other) => {
            eprintln!("  Unknown topic: {}\n", other);
            render(TOPICS);
        }
    }
    Ok(())
}

fn render(markdown: &str) {
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            eprintln!("\n  {}\n", heading.bright_cyan().bold());
        } else if line.is_empty() {
            eprintln!();
        } else {
            eprintln!("  {}", line);
        }
    }
    eprintln!();
}
