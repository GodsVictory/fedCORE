//! Streamlined output formatting for the FedCore CLI.
//!
//! Visual hierarchy (default mode):
//!   ▸ Command Name
//!     summary line
//!     Section
//!       ✓ item passed
//!       ✗ item failed
//!       [1/5] building thing... ✓
//!     ✓ Done message
//!
//! Verbose mode adds detail lines and config key-value pairs
//! under their parent context.

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(verbose: bool) {
    VERBOSE.store(verbose, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

// ── Top-level ───────────────────────────────────────────────

/// Command header: "▸ Bootstrap"
pub fn header(text: &str) {
    eprintln!("{} {}", "▸".bright_cyan(), text.bright_cyan().bold());
}

/// Final success: "  ✓ All builds passed"
pub fn done(text: &str) {
    eprintln!("  {} {}", "✓".bright_green(), text.bright_green().bold());
}

/// Final failure: "  ✗ Build failed"
pub fn fail(text: &str) {
    eprintln!("  {} {}", "✗".bright_red(), text.bright_red().bold());
}

// ── Sections ────────────────────────────────────────────────

/// Section label: "  Deploying"
pub fn section(text: &str) {
    eprintln!("  {}", text.bold());
}

/// Dim summary line after header: "  my-cluster (aws/us-east-1/dev)"
pub fn summary(text: &str) {
    eprintln!("  {}", text.dimmed());
}

// ── Items (indented under sections) ─────────────────────────

/// Success item: "    ✓ text"
pub fn item_ok(text: &str) {
    eprintln!("    {} {}", "✓".bright_green(), text);
}

/// Failure item: "    ✗ text"
pub fn item_fail(text: &str) {
    eprintln!("    {} {}", "✗".bright_red(), text);
}

/// Warning item: "    ⚠ text"
pub fn item_warn(text: &str) {
    eprintln!("    {} {}", "⚠".bright_yellow(), text);
}

/// Status item: delegates to item_ok or item_fail
pub fn item_status(ok: bool, text: &str) {
    if ok { item_ok(text) } else { item_fail(text) }
}

// ── Key-value pairs ─────────────────────────────────────────

/// Config line (verbose only): "    label  value"
pub fn config(label: &str, value: &str) {
    if is_verbose() {
        eprintln!("    {:20} {}", label.bright_cyan(), value);
    }
}

// ── Detail / verbose lines ──────────────────────────────────

/// Detail line (verbose only): "      text"
pub fn detail(text: &str) {
    if is_verbose() {
        eprintln!("      {}", text.dimmed());
    }
}

/// Log line (verbose only): "    text"
pub fn log(text: &str) {
    if is_verbose() {
        eprintln!("    {}", text.dimmed());
    }
}

// ── Inline progress ─────────────────────────────────────────

/// Start inline progress: "    [1/5] Building capsule..."
pub fn progress(current: usize, total: usize, text: &str) {
    if is_verbose() {
        eprintln!("    [{}/{}] {}...", current, total, text);
    } else {
        eprint!("    [{}/{}] {}...", current, total, text);
    }
}

/// Complete inline progress with ✓ or ✗
pub fn progress_done(ok: bool) {
    if is_verbose() {
        return;
    }
    if ok {
        eprintln!(" {}", "✓".bright_green());
    } else {
        eprintln!(" {}", "✗".bright_red());
    }
}

// ── Progress bar ────────────────────────────────────────────

pub fn progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("    [{bar:30}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb
}

// ── Command tracing ─────────────────────────────────────────

pub fn cmd(command: &str, args: &[&str]) {
    if is_verbose() {
        let full = std::iter::once(command).chain(args.iter().copied())
            .collect::<Vec<_>>().join(" ");
        eprintln!("    {} {}", "$".dimmed(), full.dimmed());
    }
}


// ── Warnings / errors (standalone) ──────────────────────────

/// Warning line: "  ⚠ text"
pub fn warn(text: &str) {
    eprintln!("  {} {}", "⚠".bright_yellow(), text);
}

