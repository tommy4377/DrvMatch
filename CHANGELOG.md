# Changelog

All notable changes to DrvMatch are documented here. The project follows semantic versioning.

## [0.5.0] - 2026-09-13

### Added

- Isolated first-party AMD, NVIDIA, and Intel source adapters with bounded HTTP clients and source-specific caching.
- Public vendor package versions, release dates, release channels, package families, official detail links, and release-notes links where available.
- AMD Radeon Recommended, Optional, WHQL, Preview, and Beta semantics plus explicit AMD installer modeling.
- NVIDIA Game Ready, Studio, WHQL, Beta, and Hotfix channel semantics without assuming a user preference.
- Intel graphics, Wi-Fi, Bluetooth, Ethernet, chipset/system, and storage package routing.
- GPU display-package handling and grouped AMD/Intel chipset packages.
- Cross-source package reconciliation that retains alternate provenance and selects the richer metadata record.
- Fixture-backed parsers and routing tests for all three vendors.

### Safety

- Vendor package versions are not numerically compared with Windows driver versions because the schemes are not equivalent.
- First-party source identity contributes evidence but never acts as an absolute ranking override.
- Broad NVIDIA, Intel, and chipset-family packages remain Needs review until product support is confirmed.
- Optional, Preview, Beta, and Hotfix channels do not inherit stable-channel preference merely because another label includes WHQL.

## [0.4.0] - 2026-09-12

### Added

- DriverRank as a pure, deterministic recommendation layer over normalized source candidates.
- Hard rejection for incompatible device IDs, declared architecture mismatches, and unsigned packages.
- Decomposed ranking factors covering match specificity, OEM applicability, source trust, signing, release channel, recency, fixes, security relevance, and known penalties.
- Meaningful-margin comparison against the installed package so the current driver can validly remain preferred.
- Recommended, Optional, Current, Missing, and Not recommended semantic outcomes.
- Ranked candidate alternatives with a newest-but-not-best explanation when recency loses to stronger suitability evidence.
- A first-class Why this driver section and an advanced Technical factor/score view.
- Fixture scenarios for exact subsystem matching, stable-versus-optional channels, OEM keep-current behavior, missing drivers, known regressions, architecture rejection, determinism, and generated explanations.

### Safety

- Recency contributes only limited weight and cannot overpower a materially stronger hardware or release-channel match.
- Catalog candidates that still need package-level OS and architecture inspection cannot displace the installed driver.
- Known regressions carry a disqualifying recommendation penalty and remain visible as Not recommended.
- Internal scores are presented only as technical evidence, never as a user-facing confidence or health percentage.

## [0.3.0] - 2026-09-12

### Added

- A direct Windows Update Agent adapter for applicable, uninstalled, visible driver offers.
- An isolated Microsoft Update Catalog adapter with exact-hardware-ID search parsing and on-demand package URL resolution.
- A shared normalized candidate model covering provenance, versions, dates, supported products, identifiers, package metadata, and compatibility evidence.
- Deterministic hardware-ID compatibility filtering with explicit compatible, needs-review, and rejected states.
- SQLite source metadata caching with source-specific TTLs and visible cached/fresh state.
- Per-source health reporting so Windows Update or Catalog failures remain contained.
- A Candidates details tab with source status, package metadata, matching evidence, and explained exclusions.
- Fixture-backed Catalog parser/download tests and opt-in live Microsoft source checks.

### Safety

- Candidate discovery does not rank, recommend, download, or install a driver.
- Catalog matches remain marked for package review until OS and architecture applicability can be verified from package contents.
- Source failures do not invalidate local inventory or fabricate an empty recommendation.
- Catalog queries disclose only the selected device's exact hardware ID rather than the complete machine inventory.

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
