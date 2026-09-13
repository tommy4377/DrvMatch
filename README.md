# DrvMatch

DrvMatch is a Windows desktop driver manager built around one conservative rule: recommend the driver that best fits the exact machine, not merely the package with the highest version number.

The project currently targets Windows 11 x64. Its architecture does not intentionally prevent future Windows 10 support, but that platform is not part of the first milestone's acceptance target.

## Current status

Version 1.1.0 is the first post-stable product/UI refinement. It keeps the conservative OEM/fresh-install and managed-install safety model while making DriverRank's recommendation workflow denser, more Windows-native, and easier to audit:

- a Review-first Drivers home that shows the machine state and meaningful local findings before exposing the complete PnP inventory;
- a compact `Review / All hardware` mode switch and grouped hardware browser for Display, Network, Audio, Bluetooth, Storage, Input, System, USB, Camera, and Other;
- compact top-level `Drivers / History / Settings` tabs instead of a permanent left sidebar, plus a persistent edge-to-edge operational status footer;
- low-chroma neutral light surfaces and restrained charcoal dark surfaces that keep DrvMatch recognisable through hierarchy and muted indigo rather than decorative card/glass effects;
- a fixed 1180 × 760 Tauri 2 window with a Windows-style custom title bar;
- acrylic enabled by default, a persistent solid-surface option, and System/Light/Dark themes;
- Drivers, History, and Settings navigation;
- real present-device enumeration through Windows SetupAPI;
- persisted machine identity from Windows (manufacturer, model, SKU/family, baseboard, BIOS, Windows DisplayVersion, and build metadata) for exact OEM/OS applicability;
- normalized hardware identity fields for PCI/USB/HD Audio/other devices when IDs expose vendor/device/subsystem data;
- installed provider, version, INF date/path/section, matching ID, signer, signature class, catalog identity, and Windows driver rank where available;
- problem and missing-driver detection based on Windows-reported evidence;
- conservative detection of explicitly generic Microsoft drivers;
- searchable status/category-filtered grouped hardware inventory and a technical details inspector that opens only after explicit device selection and scrolls independently for long specifications;
- SQLite-backed scan history that can reload earlier inventories together with the machine identity captured for that scan;
- applicable driver discovery through the Windows Update Agent API;
- exact-hardware-ID searches through an isolated Microsoft Update Catalog adapter;
- first-party AMD, NVIDIA, and Intel discovery adapters with public package versions and release channels; vendor pages remain Needs review until package-level IDs/applicability are independently proven;
- live Dell, Lenovo, and HP OEM adapters: Dell platform catalogs resolved from `CatalogIndexPC.cab`, Lenovo machine-type Windows 11 catalogs/package descriptors, and HP HPIA platform/reference catalogs;
- GPU-specific Game Ready, Studio, Radeon Recommended/Optional, WHQL, Preview, Beta, and Hotfix presentation;
- chipset and system package grouping so one vendor installer is not misrepresented as unrelated per-device updates;
- conservative duplicate reconciliation when package identity evidence overlaps, with alternate provenance retained;
- normalized candidates with source provenance, machine-model applicability, package-level PnP IDs, official OEM checksums when published, compatibility evidence, and explained exclusions;
- cached source metadata with visible freshness and per-source failure states;
- self-healing source metadata cache entries, quarantined corrupt cache databases, bounded SQLite waits, and tested migration from the pre-machine-identity scan schema;
- explicit source connect/total timeouts, one bounded retry for transient metadata GET failures, and response-size limits for public catalog/vendor payloads;
- on-demand Catalog package URL resolution without automatic downloading;
- hard rejection of incompatible architecture, device-ID, and unsigned-package candidates when that metadata is available;
- decomposed ranking factors for hardware specificity, OEM applicability, source trust, signing, release channel, recency, fixes, security relevance, and known penalties;
- meaningful-margin comparison against the installed driver, including a valid Current/keep-current result;
- Recommended, Optional, Current, Missing, and Not recommended semantic states, including an explicit install-review path for compatible missing-device recommendations;
- ranked alternatives, newest-but-not-best explanations, a plain-language Why this driver section, and a fresh-install summary for missing/generic-driver findings;
- internal scores and factor breakdowns confined to the Technical view.
- managed official-source downloads with visible progress and cancellation before system changes;
- SHA-256 fingerprints and Windows trust/signature verification before installation, with both the reviewed source artifact and selected install target re-hashed after elevation;
- explicit pre-install review with package, source, version transition, and safety actions;
- INF installation through the supported Windows driver-install API without forcing a lower-ranked match; multi-INF archives must resolve to one uniquely most-specific signed INF for the selected device or installation is blocked;
- verified vendor installer handoff for supported EXE/MSI packages;
- paired System Restore BEGIN/END operations, current-package export, result/reboot tracking, and persistent installation history;
- native rollback attempts through the Windows driver rollback API only after a completed INF change; exported prior packages are tracked separately as recovery artifacts, not treated as proof that native rollback will succeed;
- a persistent operation footer that remains visible while downloads and installations run;
- interrupted-session recovery that removes only incomplete `.part` downloads and preserves a conservative History record before package work can reach Windows;
- a list/detail history workspace that explains each version transition, package source, verification result, safety action, reboot requirement, and rollback eligibility;
- SQLite-backed settings for source selection, install safeguards, theme, acrylic, Windows accent, reduced motion, technical visibility, and log verbosity;
- persistent per-source health, metadata cache inspection and clearing, and bounded local activity-log access with copy and clear actions;
- explicit unsaved-settings behavior with Save and Discard controls; and
- runtime About information sourced from the packaged application version and project repository.
- roving keyboard navigation for the device list, strengthened forced-colors behavior, and measured startup/inventory/source timings in the local activity log.
- machine-level **Check drivers** review for missing/problem/generic targets, with per-device DriverRank findings and no blanket “update everything” scan.

DrvMatch installs only candidates with completed compatibility evidence. It re-hashes the reviewed download and the exact selected install target at the elevated boundary. For CAB/INF packages, automatic installation is blocked unless one signed INF can be proven to match the selected device more specifically than every alternative. Vendor product pages never borrow the selected device's IDs as synthetic applicability evidence; until package metadata proves the match, those candidates remain Needs review and cannot enter the normal install path.

## Interface

The 1.1.0 interface remains intentionally Review-first: raw Windows hardware is available under **All hardware**, while the default Review surface stays focused on source-backed decisions. The shell now disables Tauri's undecorated-window shadow, suppresses the DWM border explicitly, paints the WebView edge-to-edge, and keeps native Windows rounded corner clipping. Drivers uses a denser list/detail workspace, Settings uses horizontal categories rather than a nested sidebar, and long inspector/settings/history surfaces scroll independently.

## Safety model

DrvMatch treats recency as evidence, not a verdict. DriverRank gives stronger weight to hardware and subsystem specificity, OEM applicability, Windows compatibility, source provenance, signing and WHQL state, release channel, and known stability information. The normal product path does not recommend unsigned, mismatched, or known-regressed packages.

## Data sources

The current milestone reads local Plug and Play inventory through SetupAPI, discovers applicable offers through Windows Update Agent, performs exact-ID Microsoft Update Catalog searches, and queries official AMD, NVIDIA, and Intel support pages for recognized component families. These component-vendor pages remain discovery metadata until package-level applicability is proven.

For supported system OEMs, DrvMatch now queries official structured metadata rather than inventing model matches: Dell uses the per-platform catalog referenced by `CatalogIndexPC.cab`; Lenovo uses the detected four-character machine type's Windows 11 catalog and package descriptors; HP uses HPIA's platform list and the exact platform + current Windows DisplayVersion reference image. OEM candidates enter the normal compatibility path only when the catalog proves machine applicability and exposes a PnP device ID that matches the selected device. BIOS, firmware, app-only entries, and packages without device-level evidence do not become normal driver recommendations.

ASUS, MSI, Gigabyte, ASRock, Acer, and other OEM families are intentionally not represented by synthetic sources in 1.1.0. They should be added only when DrvMatch has a stable official feed that can prove both machine and device/package applicability.

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

Run the validation suite (or use `./scripts/release-validate.ps1` on the Windows release host):

```powershell
npm test
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo build --manifest-path src-tauri/Cargo.toml --locked --release
npm audit
npm run tauri build
```

## Project documents

- `PROMPT.MD` defines product scope and milestones.
- `AGENTS.MD` defines engineering and safety rules.
- `DESIGN.MD` defines the visual and interaction system.
- `design-refence/` contains read-only design material and is never imported by the application.
- `docs/RELEASE_VALIDATION.md` records the 1.1 release-validation and reproducibility process.
- `docs/BETA_VALIDATION.md` is retained as the historical 0.9 validation record.
- `docs/KNOWN_LIMITATIONS.md` documents the supported boundary and deliberately conservative failure cases.
- `docs/DESIGN_RESEARCH_1.1.md` records the Figma frames, Windows/GitHub UI references, and anti-slop decisions used for the 1.1 pass.
- `docs/screenshots/` contains screenshots from the packaged Windows application.

## Privacy

The current build stores inventory, machine identity, source metadata, managed packages, safety backups, SHA-256 fingerprints, and installation history under the application's local data directory. Checking candidates asks the local Windows Update Agent for applicable drivers, sends only the selected device's exact hardware ID to Microsoft Update Catalog, requests the relevant public AMD/NVIDIA/Intel support page for recognized hardware, and—only when the detected system manufacturer is applicable—queries the corresponding Dell, Lenovo, or HP public catalog using local model/platform identifiers. Packages are downloaded only after the user opens and confirms an install review; DrvMatch does not upload the complete inventory or install drivers in the background.

## License

MIT

Important third-party dependency licenses are recorded in `THIRD_PARTY_NOTICES.md`.
