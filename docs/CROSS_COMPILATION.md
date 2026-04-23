# Cross-Compilation Guide

This guide covers building the fedCORE CLI for Windows from a RHEL 9 (or WSL2) host.

---

## Prerequisites

The following must already be installed:

- Rust toolchain (`rustc`, `cargo`) — installed via the standalone installer to `/usr/local`
- mingw64 cross-compiler packages (from RHEL repos)

---

## Setup

### 1. Install the MinGW Cross-Compiler

```bash
sudo dnf install mingw64-gcc mingw64-winpthreads-static
```

### 2. Install the Rust Windows Standard Library

The `rust-std-static-x86_64-pc-windows-gnu` RPM is not available in RHEL 9 repos.
Download the stdlib component directly from the Rust release server instead.

Match the version to your installed Rust (`rustc --version`):

```bash
RUST_VERSION=$(rustc --version | awk '{print $2}')

cd /tmp
curl -LO "https://static.rust-lang.org/dist/rust-std-${RUST_VERSION}-x86_64-pc-windows-gnu.tar.xz"
tar xf "rust-std-${RUST_VERSION}-x86_64-pc-windows-gnu.tar.xz"
cd "rust-std-${RUST_VERSION}-x86_64-pc-windows-gnu"
sudo ./install.sh --prefix=/usr/local
```

This installs the Windows target libraries alongside your existing Linux Rust installation.

### 3. Configure Cargo Linker

Create or edit `~/.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
```

---

## Building

```bash
# Linux (default)
cargo build --release

# Windows
cargo build --release --target x86_64-pc-windows-gnu
```

Output locations:
- Linux: `target/release/fedcore`
- Windows: `target/x86_64-pc-windows-gnu/release/fedcore.exe`

---

## Staging Both Platforms

```bash
make stage
```

This builds both Linux and Windows binaries and pushes them to the OCI registry with platform-specific tags.

---

## Troubleshooting

### `error[E0463]: can't find crate for std`

The Windows stdlib is not installed. Re-run step 2 above, ensuring the version matches your `rustc --version` output.

### Linker errors about `x86_64-w64-mingw32-gcc` not found

The mingw64-gcc package is missing or not in PATH:

```bash
sudo dnf install mingw64-gcc
which x86_64-w64-mingw32-gcc
```

### `static.rust-lang.org` is unreachable

This domain is on Cloudfront, not `*.rs`. Ensure your proxy allows it, or download the tarball from a machine with access and transfer it manually.

### Version mismatch between stdlib and rustc

The stdlib version must exactly match your installed rustc version. Check with:

```bash
rustc --version
ls /usr/local/lib/rustlib/ | grep windows
```

If they differ, download the correct version of the stdlib tarball.
