# DrvMatch

DrvMatch is a Windows desktop driver manager built around one conservative rule: recommend the driver that best fits the exact machine, not merely the package with the highest version number.

The project currently targets Windows 11 x64. Its architecture does not intentionally prevent future Windows 10 support, but that platform is not part of the first milestone's acceptance target.

## Current status

Version 0.7.0 adds persistent source, safety, appearance, and advanced management while preserving DriverRank's conservative suitability decisions:

- a fixed 1180 × 760 Tauri 2 window with a Windows-style custom title bar;
- acrylic enabled by default, a persistent solid-surface option, and System/Light/Dark themes;
- Drivers, History, and Settings navigation;
- real present-device enumeration through Windows SetupAPI;
- installed provider, version, INF date/path/section, matching ID, signer, signature class, catalog identity, and Windows driver rank where available;
- problem and missing-driver detection based on Windows-reported evidence;
- conservative detection of explicitly generic Microsoft drivers;
- searchable/filterable device inventory and a technical details view;
- SQLite-backed scan history that can reload earlier inventories;
- applicable driver discovery through the Windows Update Agent API;
- exact-hardware-ID searches through an isolated Microsoft Update Catalog adapter;
- first-party AMD, NVIDIA, and Intel adapters with public package versions and release channels;
- GPU-specific Game Ready, Studio, Radeon Recommended/Optional, WHQL, Preview, Beta, and Hotfix presentation;
- chipset and system package grouping so one vendor installer is not misrepresented as unrelated per-device updates;
- evidence-based duplicate reconciliation across Microsoft and vendor sources with alternate provenance retained;
- normalized candidates with source provenance, compatibility evidence, and explained exclusions;
- cached source metadata with visible freshness and per-source failure states;
- on-demand Catalog package URL resolution without automatic downloading;
- hard rejection of incompatible architecture, device-ID, and unsigned-package candidates when that metadata is available;
- decomposed ranking factors for hardware specificity, OEM applicability, source trust, signing, release channel, recency, fixes, security relevance, and known penalties;
- meaningful-margin comparison against the installed driver, including a valid Current/keep-current result;
- Recommended, Optional, Current, Missing, and Not recommended semantic states;
- ranked alternatives, newest-but-not-best explanations, and a plain-language Why this driver section;
- internal scores and factor breakdowns confined to the Technical view.
- managed official-source downloads with visible progress and cancellation before system changes;
- SHA-256 fingerprints and Windows trust/signature verification before installation;
- explicit pre-install review with package, source, version transition, and safety actions;
- INF installation through the supported Windows driver-install API without forcing a lower-ranked match;
- verified vendor installer handoff for supported EXE/MSI packages;
- restore-point attempts, current-package export, result/reboot tracking, and persistent installation history;
- rollback through the Windows driver rollback API only when a prior package was actually preserved;
- a persistent operation footer that remains visible while downloads and installations run;
- a list/detail history workspace that explains each version transition, package source, verification result, safety action, reboot requirement, and rollback eligibility;
- SQLite-backed settings for source selection, install safeguards, theme, acrylic, Windows accent, reduced motion, technical visibility, and log verbosity;
- persistent per-source health, metadata cache inspection and clearing, and bounded local activity-log access with copy and clear actions;
- explicit unsaved-settings behavior with Save and Discard controls; and
- runtime About information sourced from the packaged application version and project repository.

DrvMatch installs only candidates with completed compatibility evidence and verifies the downloaded artifact again at the elevated boundary. Broad Catalog and vendor-family packages remain in review until package-level applicability is known and therefore cannot enter the normal install path.

## Safety model

DrvMatch treats recency as evidence, not a verdict. DriverRank gives stronger weight to hardware and subsystem specificity, OEM applicability, Windows compatibility, source provenance, signing and WHQL state, release channel, and known stability information. The normal product path does not recommend unsigned, mismatched, or known-regressed packages.

## Data sources

The current milestone reads local Plug and Play inventory through SetupAPI, discovers applicable offers through Windows Update Agent, performs exact-ID Microsoft Update Catalog searches, and queries official AMD, NVIDIA, and Intel support pages for supported component families. Each adapter is isolated from shared normalization, compatibility filtering, DriverRank, and the installation coordinator. Applicable system-OEM catalogs remain planned sources.

## Development

Prerequisites:

- Windows 11
- Node.js and npm
- stable Rust with the MSVC toolchain
- Microsoft C++ Build Tools and WebView2, as required by Tauri 2

Install dependencies and start the desktop application:

```powershell
npm install
npm run tauri dev
```

Run the validation suite:

```powershell
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project documents

- `PROMPT.MD` defines product scope and milestones.
- `AGENTS.MD` defines engineering and safety rules.
- `DESIGN.MD` defines the visual and interaction system.
- `design-refence/` contains read-only design material and is never imported by the application.

## Privacy

The current build stores inventory, source metadata, managed packages, safety backups, SHA-256 fingerprints, and installation history under the application's local data directory. Checking candidates asks the local Windows Update Agent for applicable drivers, sends only the selected device's exact hardware ID to Microsoft Update Catalog, and requests the relevant public AMD, NVIDIA, or Intel support page for recognized hardware. Packages are downloaded only after the user opens and confirms an install review; DrvMatch does not upload the complete inventory or install drivers in the background.

## License

MIT

Important third-party dependency licenses are recorded in `THIRD_PARTY_NOTICES.md`.
