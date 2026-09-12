# Changelog

All notable changes to DrvMatch are documented here. The project follows semantic versioning.

## [0.2.0] - 2026-09-12

### Added

- Installed-driver metadata associated with each Windows device, including provider, version, INF date, published INF, INF section, matching ID, driver key, and Windows driver rank.
- Installed INF signature verification through SetupAPI with signer, catalog identity, and conservative WHQL/inbox/Authenticode classification.
- Windows problem-code and problem-status inventory with explicit Current, Missing, and Problem states.
- Narrow detection for drivers explicitly identified as generic and published by Microsoft.
- SQLite persistence for complete inventory snapshots and scan summaries.
- Reloadable History view for the latest 100 local scans.
- Device and driver search with problem, missing-driver, and generic-Microsoft filters.
- Expanded Overview and Technical tabs for installed package and signing evidence.
- Tests for SQLite round trips, signature-score normalization, generic-driver classification, and live installed-driver association.

### Safety

- Driver dates remain labeled as INF dates rather than release dates.
- Unknown signing, package, or problem metadata remains explicitly unknown instead of being inferred.
- Historical scans are identified as stored inventory so they are not mistaken for a fresh machine inspection.

## [0.1.0] - 2026-09-12

### Added

- Tauri 2, Svelte 5, TypeScript, and Rust project foundation.
- Fixed-size Windows shell with a custom Windows-style title bar and rounded presentation.
- Acrylic backdrop with a persistent solid fallback.
- Persistent System, Light, and Dark appearance modes.
- Drivers, History, and Settings navigation with a compact status bar.
- Real Windows Plug and Play enumeration through SetupAPI.
- Typed device inventory with friendly name, description, manufacturer, class, instance ID, hardware IDs, and compatible IDs.
- List/detail device inspection, technical identifiers, and calm loading/error states.
- Windows inventory and UTF-16 parsing tests.

### Safety

- No mock recommendations or install controls are exposed.
- The interface explicitly withholds recommendation language until installed-driver and candidate evidence exists.
