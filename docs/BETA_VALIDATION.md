# 0.9.0 beta validation

This document records the beta release checks used for DrvMatch 0.9.0. Commands are run from a clean Git worktree with the committed npm and Cargo lockfiles.

## Automated checks

```powershell
npm ci
npm test
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo build --manifest-path src-tauri/Cargo.toml --locked --release
npm audit
cargo audit --file src-tauri/Cargo.lock
npm run tauri build
```

The normal Rust suite includes offline fixtures for Microsoft Catalog, vendor pages, OEM catalog parsing, ranking boundaries, schema migration, malformed-cache eviction, corrupted-cache recreation, and interrupted-download cleanup. Two network-dependent smoke tests remain ignored by default and are documented in `KNOWN_LIMITATIONS.md`.

Release result on 2026-09-13: 54 normal Rust tests and 2 opt-in live Microsoft source tests passed; 5 frontend presentation tests passed; Svelte diagnostics reported 0 errors and 0 warnings; Clippy passed with warnings denied.

## Manual release matrix

The packaged Windows build is checked for:

- System, Light, and Dark themes;
- acrylic on and solid acrylic-off surfaces;
- keyboard traversal, list Up/Down/Home/End behavior, tab arrow navigation, Ctrl+F, F5, Escape, dialog focus containment, and visible focus;
- forced-colors rules and semantic state legibility without color alone;
- fixed 1180 × 760 logical sizing and Windows-native DPI scaling;
- clean installation and an in-place update from the 0.8.0 NSIS package;
- startup, inventory, source timing entries in the bounded local activity log; and
- generated MSI and NSIS bundle size.

The packaged app initialized its local stores in 10–23 ms and enumerated 217 present devices in approximately 1.43 seconds on the release host. These values are diagnostic observations, not universal performance guarantees.

The 0.9.0 release binary is 8,282,112 bytes. The MSI is 4,874,240 bytes (SHA-256 `57B8795739E312B4AC981FA9E9D7EBCA5AEA1967DC7E3C8ADCBA19151771B830`) and the NSIS installer is 2,833,814 bytes (SHA-256 `4BDAF87D1852C0380018E15720E17CF1B3A06C14478255620B4B0D5E3213354C`). The NSIS clean-install baseline registered 0.8.0, and installation of the 0.9.0 bundle over that baseline registered 0.9.0 while retaining the application data directory.

## Reproducibility

The npm and Cargo lockfiles are committed. Release builds use `npm ci`, `cargo build --locked --release`, and Tauri's configured static frontend adapter. Binary hashes can differ across build times because Windows installer metadata is timestamped; reproducibility here means the same locked source and toolchain produce functionally equivalent signed-input application bundles, not bit-for-bit identical installer containers.

## Security boundaries reviewed

- Tauri exposes only named commands; there is no general shell or privileged-command API.
- Elevation accepts narrowly typed install/rollback requests whose serialized request and selected files are hashed across the elevation boundary.
- Package and metadata URLs require HTTPS, bounded redirects, and provider-specific host allowlists.
- Driver packages are locally SHA-256 fingerprinted, official checksums are enforced when published, Windows trust is verified, and CAB installation requires one uniquely best matching INF.
- Metadata and package response sizes are bounded; cache corruption is quarantined; SQLite waits are bounded; and transient metadata retries are limited to one retry.
- Normal installation never uses a force-install flag and never silently installs an unsigned or incompatible package.

## Dependency audit result

- `npm audit` reports zero known vulnerabilities after updating SvelteKit and overriding its transitive `cookie` package to the patched 0.7 line.
- RustSec reports zero vulnerable locked Rust crates. It reports unmaintained `unic-*` crates inherited through Tauri's `urlpattern`, plus warnings in `glib` and `proc-macro-error` that are not present in the Windows target graph. These are upstream transitive dependencies rather than directly selected DrvMatch libraries; the beta retains the current Tauri-compatible versions and will re-audit them for 1.0.
