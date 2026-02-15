# GitHub Actions Workflows

This document describes the automated CI/CD workflows for the RustClaw project.

## Workflows

### 1. CI Workflow (`.github/workflows/ci.yml`)

Runs on every push and pull request to `main`, `master`, and `develop` branches.

**Jobs:**
- **test**: Runs tests, clippy lints, and format checks
- **build**: Builds the project in release mode
- **security-audit**: Runs `cargo audit` for security vulnerabilities

### 2. Release Workflow (`.github/workflows/release.yml`)

Automatically builds and releases binaries for multiple platforms.

**Triggers:**
- Push tags matching `v*` (e.g., `v0.1.1`)
- Manual dispatch via GitHub UI

**Commit Message Tags:**
Control which platforms to build by including tags in commit messages:

- `[ci-linux]` - Build Linux binaries only
- `[ci-macos]` - Build macOS binaries only  
- `[ci-windows]` - Build Windows binaries only
- `[ci-all]` - Build all platforms (default)

**Manual Trigger:**
Use the "Run workflow" button in GitHub Actions UI with `build_target` parameter:
- `all` - Build all platforms
- `linux` - Linux only
- `macos` - macOS only
- `windows` - Windows only

## Release Process

### Automated Release (Recommended)

1. **Update version in Cargo.toml**
   ```toml
   version = "0.2.0"  # Update this
   ```

2. **Commit and push changes**
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 0.2.0"
   git push
   ```

3. **Create and push tag**
   ```bash
   ./create-tag.sh --push
   ```

4. **Monitor the release**
   - GitHub Actions will build binaries for all platforms
   - A release will be created automatically
   - Binaries will be uploaded to the release

### Manual Release

1. Create a tag manually:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

## Built Binaries

The workflow produces binaries for the following platforms:

### Linux
- **x86_64-unknown-linux-gnu**: Linux x86_64 (Intel/AMD 64-bit)
- **aarch64-unknown-linux-gnu**: Linux ARM64 (aarch64)

### macOS
- **x86_64-apple-darwin**: macOS Intel (x86_64)
- **aarch64-apple-darwin**: macOS Apple Silicon (M1/M2/M3)

### Windows
- **x86_64-pc-windows-msvc**: Windows x86_64

## Artifacts

Each binary is packaged as:
- Linux: `.tar.gz` archive
- macOS: `.zip` archive
- Windows: `.zip` archive

SHA256 checksums are provided for all binaries.

## Installation

### Linux/macOS
```bash
# Download and extract
tar -xzf rustclaw-*.tar.gz  # Linux
unzip rustclaw-*.zip        # macOS

# Make executable
chmod +x rustclaw-gateway

# Run
./rustclaw-gateway
```

### Windows
Extract `rustclaw-*.zip` and run `rustclaw-gateway.exe`

## GitHub Actions Used

- `actions/checkout@v4` - Checkout repository
- `dtolnay/rust-toolchain@stable` - Install Rust
- `actions/cache@v4` - Cache cargo dependencies
- `actions/upload-artifact@v4` - Upload build artifacts
- `actions/download-artifact@v4` - Download artifacts
- `softprops/action-gh-release@v2` - Create GitHub releases

## Troubleshooting

### Build Failures

1. **Check the logs** in GitHub Actions
2. **Run locally** to reproduce:
   ```bash
   cargo build --release --target <target>
   ```
3. **Check dependencies** are available for the target platform

### Cross-Compilation Issues

For ARM64 builds on Linux:
```bash
sudo apt-get install gcc-aarch64-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
cargo build --release --target aarch64-unknown-linux-gnu
```

## Security

- All workflows use pinned action versions
- Dependencies are cached to speed up builds
- Security audits run on every push
- Binaries are checksummed for verification
