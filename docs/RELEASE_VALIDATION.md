# DrvMatch 1.0.0 release validation

This document is the release gate for the first stable DrvMatch build. It deliberately separates checks that can run in a source-only environment from checks that require Windows 11, the Rust MSVC toolchain, WebView2, and the Tauri bundler.

## Source checks

```powershell
npm ci
npm test
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
npm audit
cargo audit --file src-tauri/Cargo.lock
```

The dependency-free frontend presentation suite contains seven tests covering source names, conservative recommendation language, compatibility wording, source/signature state, package-size formatting, hardware categorization, and review ordering.

## Windows end-to-end gate

Run on a Windows 11 x64 release host:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --locked --release
npm run tauri build
```

Then verify:

- clean launch with the custom fixed-size titlebar and no exposed grey client-area frame;
- System, Light, Dark, acrylic-on, and solid-material modes;
- Drivers Review opens without auto-selecting arbitrary hardware;
- **Check drivers** evaluates missing/problem/generic review targets and does not turn healthy specific drivers into an update count;
- All hardware search, category/status filters, keyboard roving selection, and explicit inspector opening;
- Overview, Candidates, and Technical inspector tabs scroll to the final row on long content;
- long History details, Settings sheets, and activity logs remain independently scrollable;
- source partial failure leaves local inventory and other source results usable;
- missing-device recommendation requires proven package/device applicability;
- package download, redirect allowlist, checksum (when published), SHA-256 fingerprint, Windows trust verification, and pre-install review;
- elevation re-hashes the reviewed artifact and selected target;
- CAB/INF install refuses unsigned, incompatible, or ambiguous multi-INF packages and never force-installs;
- vendor installer handoff records the observed result without fabricating a Windows driver transition;
- System Restore and prior-package export outcomes are reported honestly;
- History records before/after state and interrupted attempts;
- native rollback is offered only when the recorded Windows change is actually eligible;
- settings persist across restart and at least one trusted source must remain enabled;
- forced-colors, reduced-motion, keyboard focus, Ctrl+F, F5, Escape, and dialog focus behavior;
- clean install and in-place update from the latest beta package;
- MSI/NSIS bundle creation and version metadata all report `1.0.0`.

## Release evidence

Do not write binary sizes, hashes, signing claims, or “all checks pass” into the changelog until the Windows release host has actually produced and exercised those exact artifacts. Store final MSI/NSIS SHA-256 values alongside the published release assets.

## Reproducibility

The npm and Cargo lockfiles are committed. Release builds use `npm ci`, Cargo `--locked`, and the configured static SvelteKit frontend. Windows installer metadata can make container hashes timestamp-dependent; reproducibility means the locked source/toolchain produces functionally equivalent application bundles, not necessarily bit-for-bit identical installer containers.
