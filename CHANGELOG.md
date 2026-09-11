# Changelog

All notable changes to DrvMatch are documented here. The project follows semantic versioning.

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
