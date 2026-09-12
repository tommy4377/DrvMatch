# DrvMatch

DrvMatch is a Windows desktop driver manager built around one conservative rule: recommend the driver that best fits the exact machine, not merely the package with the highest version number.

The project currently targets Windows 11 x64. Its architecture does not intentionally prevent future Windows 10 support, but that platform is not part of the first milestone's acceptance target.

## Current status

Version 0.2.0 provides real local device and installed-driver inventory:

- a fixed 1180 × 760 Tauri 2 window with a Windows-style custom title bar;
- acrylic enabled by default, a persistent solid-surface option, and System/Light/Dark themes;
- Drivers, History, and Settings navigation;
- real present-device enumeration through Windows SetupAPI;
- installed provider, version, INF date/path/section, matching ID, signer, signature class, catalog identity, and Windows driver rank where available;
- problem and missing-driver detection based on Windows-reported evidence;
- conservative detection of explicitly generic Microsoft drivers;
- searchable/filterable device inventory and a technical details view;
- SQLite-backed scan history that can reload earlier inventories;
- explicit loading and failure states without fabricated recommendations.

DrvMatch does not install or recommend drivers yet. Those controls remain absent until the application can normalize trusted candidates, verify compatibility, and explain a recommendation.

## Safety model

DrvMatch treats recency as evidence, not a verdict. Future recommendations must consider hardware and subsystem specificity, OEM applicability, Windows compatibility, source provenance, signing and WHQL state, release channel, and known stability information. The normal product path will not silently install unsigned or mismatched packages.

## Data sources

The current milestone reads local Plug and Play and installed-driver inventory directly from Windows and keeps completed scans in a local SQLite database. Planned candidate sources are Windows Update, Microsoft Update Catalog, AMD, NVIDIA, Intel, and applicable OEM catalogs. Source adapters will remain isolated from normalization and ranking.

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

The current build performs local device/driver enumeration and stores scan history under the application's local data directory. It does not query remote driver sources, upload hardware inventory, or install packages.

## License

MIT
