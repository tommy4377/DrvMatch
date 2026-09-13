# DrvMatch 1.1.0 patch manifest

This manifest exists so a release patch can be audited without guessing how much work is hidden inside the monolithic route/backend files.

## Native window / backend

- `src-tauri/tauri.conf.json` — version 1.1.0, disables Tauri's undecorated-window shadow so Windows does not add its 1 px frame.
- `src-tauri/src/platform/windows.rs` — applies `DWMWA_COLOR_NONE` to the DWM border and keeps the Windows 11 rounded-corner preference.
- `src-tauri/src/lib.rs` — applies native chrome policy at startup and reapplies it after Acrylic changes.
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` — release metadata synchronized to 1.1.0.

## Product UI

- `src/routes/+page.svelte` — Drivers 1.1 hierarchy, header-level Check drivers action, denser source-backed recommendation rows, horizontal Settings workflow, and 1.1 version fallback.
- `src/lib/styles/workbench.css` — route UI styles moved out of the Svelte component plus the 1.1 desktop design pass, scroll containment, controls, settings tabs, inspector, footer, and recommendation-table styling.
- `src/app.css` — neutral desktop token palette, DrvMatch indigo identity, solid/acrylic surface values.
- `scripts/ui-audit.mjs` — new dependency-free regression audit for unstyled literal Svelte classes and forbidden hover `title=` attributes.

## Release / documentation

- `package.json`, `package-lock.json` — version 1.1.0 and `ui:audit` script.
- `scripts/release-validate.ps1` — includes UI audit and reports 1.1.0.
- `DESIGN.MD` — neutral low-chroma desktop direction, edge-to-edge operational footer, no idle fake progress rail.
- `AGENTS.MD` — future agent guidance synchronized with the 1.1 design direction.
- `README.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md` — 1.1.0 product/release documentation.
- `docs/KNOWN_LIMITATIONS.md`, `docs/RELEASE_VALIDATION.md` — 1.1.0 release boundary and Windows DWM/WebView validation matrix.
- `docs/DESIGN_RESEARCH_1.1.md` — Figma frame references, GitHub/WinUI inspiration, and explicit anti-slop decisions.

## Validation already possible in this source environment

- `node --test src/lib/presentation.test.mjs` — 7/7 passing.
- `node scripts/ui-audit.mjs` — passing; every literal Svelte class has application CSS and no forbidden `title=` tooltip exists.
- Structural Svelte audit — balanced major HTML elements, `{#if}` / `{/if}`, and `{#each}` / `{/each}` blocks.

## Validation that still requires the Windows release host

This source environment does not contain the Rust/MSVC/Tauri Windows toolchain. `cargo fmt`, Clippy, Rust tests, Tauri build, DWM visual QA, MSI/NSIS packaging, and real driver install/rollback must be run with `scripts/release-validate.ps1` on Windows before publishing binaries.
