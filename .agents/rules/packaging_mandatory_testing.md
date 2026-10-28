# PACKAGING AUTOMATION, VERSIONING & ARTIFACT DELIVERY
ID: `RULE-PKG-001`
Status: MANDATORY / CI & RELEASE STANDARD

---

## 1. Automated Packaging Standard
- In every testing release cycle or stakeholder demonstration, deliverables must be generated using automated packaging scripts.
- Presenting raw, unversioned executables directly from `target/debug` or `target/release` is strictly prohibited.

---

## 2. Platform Packaging Matrix
- **Windows**: Inno Setup script (`packaging/windows/setup.iss`) or WiX Toolset generating signed `.exe`/`.msi` installers containing app binary, assets, licenses, and uninstall support.
- **Linux**: Standalone AppImage, Flatpak manifest, or `.deb` bundle packaging.
- **macOS**: `.app` bundle structure with `Info.plist`, entitlements, and signed `.dmg` disk image.

---

## 3. Semantic Versioning & Version Bumps
- Follow Semantic Versioning 2.0.0 (`MAJOR.MINOR.PATCH`).
- Automated scripts in `scripts/bump_version.ps1` / `scripts/package.ps1` must:
  1. Increment the version in workspace `Cargo.toml`.
  2. Synchronize version in installer metadata files.
  3. Validate that build artifacts embed the exact version string matching `--version`.
