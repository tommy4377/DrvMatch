# Changelog

All notable changes to DrvMatch are documented here. The project follows semantic versioning.

## [0.8.0] - 2026-09-13

### Added

- Completed the first OEM/fresh-install milestone with live official-catalog adapters for Dell, Lenovo, and HP.
- Dell discovery now uses the current `CatalogIndexPC.cab` flow, resolves the exact platform catalog from system/model identity, extracts device-level PCI/PnP applicability, Windows 11/architecture metadata, package URLs, criticality, release notes, size, and published SHA-256 data when available.
- Lenovo discovery now derives the four-character machine type, queries the model-specific Windows 11 catalog, follows package descriptors, and only emits driver candidates when the descriptor contains a PnP ID that can be compared with the selected device.
- HP discovery now uses HPIA `platformList.cab`, resolves the reference image for the detected platform ID and the machine's exact Windows DisplayVersion, and maps `ImagePal/Devices` PnP IDs to driver `UpdateInfo`/SoftPaq records.
- Added persisted machine identity (system manufacturer/model/SKU/family, baseboard identity, BIOS version, Windows DisplayVersion, and build) for exact OEM/OS applicability and stored-scan replay.
- Added normalized hardware identity for enumerated PCI, USB, HD Audio, ACPI, and other devices, including vendor/device/subsystem fields where Windows exposes them.
- Added published OEM SHA-256 metadata to normalized candidates and enforce it before installation when the official catalog supplies a checksum.
- Added a fresh-install findings strip with direct Missing and Generic Microsoft filters, machine identity, and an install path for a compatible recommended driver on a missing device.
- Added explicit missing-device identification in Overview from the selected candidate's package/provider/matched PnP ID, plus machine and parsed hardware identity in Technical details.
- Added OEM-specific source health, model-aware metadata caching, and settings for Dell, Lenovo, and HP.
- Added careful OEM-audio ranking evidence so exact-system audio bundles can outrank newer generic packages when extensions/APOs/vendor configuration may matter.
- Added OEM parser/ranking/compatibility/checksum fixtures for exact Dell subsystem IDs, Lenovo descriptors, HP HPIA device-to-SoftPaq relationships, Windows 11 filtering, ARM64 normalization, exact-model installation evidence, and OEM-audio preference.
- Hardened OEM catalog networking to remain on the expected Dell/Lenovo/HP HTTPS domains, added bounded parallel Lenovo descriptor retrieval, and require HP HPIA metadata to match the current Windows release instead of falling forward to a different reference image.
- Expanded OEM parser coverage for Dell Windows 11 display metadata, baseboard/SystemID matching, Lenovo off-domain descriptor rejection, and legacy scheme-less official HP SoftPaq URLs.

### Changed

- DriverRank now distinguishes exact OEM-model applicability from weaker provider alignment instead of treating the two signals as equivalent.
- Candidate reconciliation now prefers richer provenance when official OEM metadata contributes checksums, model applicability, OS/architecture data, or direct package metadata.
- Candidate discovery copy and Settings now describe all enabled trusted sources rather than Microsoft-only discovery or unfinished OEM groundwork.
- The current Dell index endpoint follows Dell's current `dl.dell.com/catalog/CatalogIndexPC.cab` catalog cadence.
- Secondary OEMs without a verified stable model-and-device applicability feed remain outside 0.8 rather than being represented by brittle/synthetic candidates.

### Integrated from 0.7.1

- Includes the full 0.7.1 installation hardening: paired restore points, elevated request/package re-hashing, exact single-INF selection for CAB packages, honest `Staged` results, conservative vendor applicability, and corrected rollback semantics.
- Includes the 0.7.1 frontend presentation cleanup/tests and removal of safety controls that had no backend effect.

### Safety

- OEM candidates become normally installable only when an official catalog is applicable to the detected machine and a package-level PnP ID matches the selected Windows device. Model name alone never creates compatibility.
- Dell entries explicitly limited to another operating system are filtered before ranking; Lenovo uses its Windows 11 model catalog and HP uses the HPIA reference image for the detected Windows 11 DisplayVersion.
- BIOS, firmware, app-only entries, packages without device IDs, malformed official checksums, and non-OEM package hosts are excluded or blocked instead of guessed.
- A published OEM checksum is verified in addition to the existing local SHA-256 fingerprint and Windows signature checks.
- Missing-device installation still requires an explicit review and the same privileged verification path as every other install.

### Tests

- Frontend dependency-free presentation suite remains 5/5.
- Added Rust unit coverage for the new OEM catalog parsers, OEM compatibility boundary, OEM audio ranking, OS/architecture filtering, and published checksum enforcement.

## [0.7.1] - 2026-09-13

### Fixed

- Paired every successful System Restore `BEGIN_SYSTEM_CHANGE` with `END_SYSTEM_CHANGE` using the returned restore-point sequence number.
- Re-hash both the reviewed source artifact and the exact selected INF/vendor installer after elevation, closing the review-to-UAC file-change gap.
- Pin the serialized privileged install/rollback request with a SHA-256 passed in the elevated process command line, blocking request-file race edits after UAC review.
- Removed `DIIRFLAG_INSTALL_AS_SET`; CAB packages now resolve to one uniquely most-specific signed INF for the selected device, and ambiguous or non-matching multi-INF packages are blocked instead of guessed.
- Installation history no longer substitutes a candidate-advertised version when Windows still reports the old driver. Successful INF handoff with no observed target-driver change is recorded as `Staged` rather than falsely reported as installed.
- Native `DiRollbackDriver` eligibility is no longer conflated with `pnputil /export-driver`. Exported packages remain recorded as separate recovery artifacts and rollback failures explain that distinction.
- AMD/NVIDIA/Intel discovery candidates no longer copy the selected device's hardware IDs or claim Windows/architecture applicability that was not obtained from package metadata.
- Removed the unsafe `VEN_1022 -> B550` fallback; AMD chipset routing now waits for real platform identification instead of guessing the motherboard chipset.
- Removed two persisted Safety settings that had no backend effect, so Settings no longer offers controls the product cannot honor.
- Removed the obsolete hidden duplicate Settings page and centralized repeated presentation labels/formatting in a tested frontend module.
- Replaced the hard-coded downloader user agent with the packaged Cargo version and synchronized 0.7.1 release metadata/notices.

### Tests

- Added five dependency-free frontend presentation tests covering source names, recommendation language, compatibility states, source/signature states, and package-size formatting.
- Added source compatibility coverage ensuring vendor discovery without proven package IDs remains `NeedsReview`.

### Safety

- DrvMatch now refuses ambiguous multi-INF installation rather than staging an entire set whose active device result cannot be reported honestly.
- Vendor discovery metadata is explicitly separated from package applicability evidence, preventing a product page from manufacturing an exact hardware match.
- Rollback UI/copy distinguishes Windows' native backup-driver mechanism from exported recovery files.

## [0.7.0] - 2026-09-13

### Added

- A compact list/detail History workspace that explains installed-version transitions, package provenance, trust verification, restore-point and backup outcomes, reboot state, and rollback eligibility.
- Persistent SQLite-backed settings organized into Appearance, Sources, Safety & Rollback, Advanced, and About sections.
- System, Light, and Dark theme preferences; acrylic control; optional Windows accent integration; and reduced-motion behavior.
- Persisted source enablement with a required-one-source invariant and per-source health, freshness, cache, candidate-count, and error presentation.
- Safety controls that drive restore-point and current-package backup behavior at the backend install boundary.
- Metadata cache statistics and clearing, plus bounded local activity-log reading, copying, verbosity selection, and clearing.
- Runtime application version and repository details in About, sourced from release metadata rather than duplicated UI constants.
- Explicit Save and Discard behavior, unsaved-change navigation protection, and restart-persistent preferences.

### Safety

- Disabled sources are enforced by the backend discovery command rather than trusted from transient UI state.
- Installation safety preferences are read from persistent backend settings when the install review is prepared.
- Source management refuses to save a configuration with no enabled sources.
- History reports recorded safety outcomes and exposes rollback only when the installation record proves it is supported.
- Settings overlays preserve keyboard focus visibility, avoid hover tooltips, and keep the fixed non-resizable Windows shell unchanged.

## [0.6.0] - 2026-09-13

### Added

- Managed HTTPS package downloads with byte progress, safe cancellation before system changes, and per-operation staging.
- SHA-256 package fingerprints and mandatory Windows trust verification before any installer is started.
- A time-limited pre-install review listing the exact device, package, source, channel, version transition, and safety actions.
- A sequential installation queue with persistent global progress, actionable failure state, and restart-required tracking.
- Narrow administrator elevation through internal INF-install, vendor-installer, and rollback helpers; no general privileged command API is exposed.
- Supported Windows INF installation through `DiInstallDriverW` without the force-install flag, plus verified vendor EXE/MSI handoff.
- Restore-point attempts and current OEM package export before installation when Windows makes those facilities available.
- SQLite installation history with before/after versions, SHA-256, signature result, safety outcomes, source, result, and reboot state.
- Conservative rollback through `DiRollbackDriver`, exposed only after a successful INF change with a real exported prior package.
- A compact install history surface and TMC-inspired persistent operation footer.

### Safety

- Unsigned packages, non-HTTPS downloads, unapproved source hosts, incomplete compatibility evidence, and mismatched recorded device IDs are rejected.
- Download redirects are checked against the same official-source allowlist as the original URL.
- Catalog CAB contents are isolated and every INF is verified against its Windows-trusted catalog before elevation.
- The elevated helper repeats signature verification and never uses `DIIRFLAG_FORCE_INF`.
- Cancellation is disabled once Windows system changes begin.
- Restore-point and backup outcomes are recorded honestly; DrvMatch does not claim rollback support when no prior package was preserved.

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
