# PROMPT.MD — DrvMatch Master Project Brief

## 0. Role

You are the lead engineer responsible for designing, implementing, testing, documenting,
versioning and shipping **DrvMatch** from an empty repository to a trustworthy Windows
driver-management application.

Treat this file as the project's master implementation brief.

Before doing any meaningful work:

1. Read `AGENTS.MD`.
2. Read `DESIGN.MD`.
3. Inspect the relevant files under `design-refence/`.
4. Inspect the current repository state and Git history.
5. Use the available MCP tools deliberately:
   - Serena
   - Context7
   - Tauri MCP
   - shadcn MCP
   - Git/GitHub MCP
6. Only then plan and implement the next coherent unit of work.

Direct instructions from the user always override repository documents.

When repository documents appear to conflict:
- `PROMPT.MD` defines product scope, architecture goals, feature behavior and release milestones.
- `AGENTS.MD` defines coding/agent/tooling rules.
- `DESIGN.MD` defines visual and interaction rules.
- `design-refence/` contains examples, never authoritative application code.

Do not silently reinterpret or weaken explicit requirements.

---

# 1. Product

## 1.1 Name

**DrvMatch**

The core concept is:

> Find the right driver for the exact machine, not simply the newest driver.

DrvMatch must be positioned as a transparent, conservative and technically serious
Windows utility.

It must NOT behave like:
- Driver Booster-style scareware
- a fake "PC health" optimizer
- an adware driver updater
- a generic "update everything" tool
- a web dashboard wrapped in Tauri

DrvMatch is a **driver recommendation, inspection, installation and rollback tool**.

---

# 2. Core philosophy

The most important product rule is:

> "Newer" is a data point, not the recommendation.

DrvMatch may conclude that:
- the installed driver is already the best choice
- an older OEM package is preferable to a newer generic package
- a WHQL Recommended branch is preferable to a newer Optional branch
- a newer driver is useful only for a specific bug/security fix
- a candidate is compatible but should not be recommended
- a missing device driver needs to be installed
- there is insufficient evidence to recommend changing anything

Do not reduce driver selection to:

```text
available_version > installed_version
```

The application exists specifically to do better than that.

---

# 3. Supported platform

Primary platform:

```text
Windows 11 x64
```

Design the architecture so Windows 10 support is not needlessly prevented, but do not
sacrifice implementation quality to support every historical Windows version during early
milestones.

Initial application stack:

```text
Tauri 2
Rust
Svelte 5
TypeScript
SQLite
```

Prefer native Windows APIs through Rust / `windows-rs` or another well-maintained crate
where appropriate.

Use WMI only when it is genuinely useful. Do not make WMI the only source of truth for
device/driver matching if SetupAPI / Configuration Manager / PnP APIs expose better data.

DrvMatch does NOT require a custom kernel driver for the MVP.

---

# 4. Repository initialization and GitHub

This is mandatory.

At the beginning of the project:

1. Initialize Git if the repository is not already initialized.
2. Ensure the default branch is `main`.
3. Using the authenticated Git/GitHub tooling, create a GitHub repository named:

```text
DrvMatch
```

under the user's connected GitHub account if it does not already exist.

If repository visibility has not been specified:
- use **private** as the safe default
- do not make the repository public without explicit user instruction

4. Add the GitHub repository as `origin`.
5. Push the initial project bootstrap to `main`.

Do not create duplicate repositories if one already exists.

If GitHub access is unavailable or authentication is missing:
- continue local implementation if possible
- clearly report the blocker
- do not pretend the repository was created or pushed

---

# 5. Versioning and release discipline

Use semantic versioning:

```text
MAJOR.MINOR.PATCH
```

During development, the planned milestone sequence is:

```text
0.1.0
0.2.0
0.3.0
0.4.0
0.5.0
0.6.0
0.7.0
0.8.0
0.9.0
1.0.0
```

A milestone is not complete until its acceptance criteria are met.

Every completed major milestone MUST result in all of the following:

1. All milestone acceptance criteria satisfied.
2. Relevant tests passing.
3. Frontend typecheck/build passing.
4. Rust checks/tests passing.
5. Program version updated consistently.
6. `CHANGELOG.md` updated.
7. Project progress documentation updated if applicable.
8. A release commit created.
9. An annotated Git tag created.
10. Commit pushed to GitHub.
11. Tag pushed to GitHub.

Use tags exactly in this style unless the existing repository establishes a different
convention:

```text
0.1.0
0.2.0
0.3.0
...
```

Do not silently switch to `v0.1.0` after the project has started.

Recommended release commit format:

```text
release: 0.1.0
release: 0.2.0
```

Update the version everywhere relevant, including at minimum when those files exist:

```text
package.json
src-tauri/Cargo.toml
src-tauri/tauri.conf.json
```

Keep lockfiles consistent with package metadata.

Do not tag:
- broken builds
- incomplete milestones
- placeholder-only functionality
- milestones whose tests are failing

Normal development commits should also be made as coherent work is completed.
Do not wait until the entire application is finished to create the first useful commit.

Never:
- force-push without explicit instruction
- rewrite published history without explicit instruction
- delete tags to hide mistakes
- discard unrelated user changes

---

# 6. Project documentation

Maintain:

```text
README.md
CHANGELOG.md
AGENTS.MD
DESIGN.MD
PROMPT.MD
```

README should eventually contain:
- what DrvMatch is
- supported Windows versions
- screenshots
- installation instructions
- safety model
- data sources
- explanation of "best driver, not newest"
- privacy behavior
- development/build instructions

CHANGELOG should follow the release milestones.

Do not modify `AGENTS.MD`, `DESIGN.MD` or `PROMPT.MD` merely to make implementation easier.
If a requirement becomes technically impossible or contradictory, explain the issue and
request a decision before changing the specification.

---

# 7. Reference UI files

Reference files currently live in:

```text
design-refence/
├── ravyn/
├── rustify/
├── tmc/
└── tommytweaker/
```

Note the directory is intentionally named `design-refence`.

Do not rename it unless explicitly asked.

Nothing inside `design-refence/` is application code.

Never import from it.

Use:
- TMC as the primary identity reference: clarity, personality, tactile grouped surfaces, immediate hierarchy and the persistent operation/footer pattern
- Ravyn only as a secondary structural reference for genuinely dense technical surfaces such as Candidates, Technical, History details and the selected-device inspector
- Rustify for material discipline, focus states and interaction polish
- TommyTweaker only for limited list/header inspiration

Do not make DrvMatch look like a Ravyn reskin. The application should be recognizable from its hierarchy even before reading the product name.

Full design requirements are in `DESIGN.MD`.

---

# 8. Window and shell requirements

The main window is intentionally:

```text
non-resizable
```

Initial target logical size:

```text
1180 × 760
```

The exact dimensions may receive one early adjustment after the first complete shell is
implemented, but should then remain fixed unless the user explicitly changes this decision.

Required:

- rounded outer window corners
- Windows-like custom titlebar
- acrylic backdrop enabled by default
- acrylic can be disabled in Settings
- Windows-style controls at the upper right:

```text
—   □   ×
```

Do NOT use Apple red/yellow/green traffic lights.

Because the window is non-resizable:
- minimize works
- close works
- the square/maximize-style control remains visually Windows-like
- it must not secretly make the window resizable/maximizable
- it should be disabled/non-operative unless the requirement changes

No automatic hover tooltips that display the name of controls.

Do not use HTML `title` attributes for normal controls.

Accessible labels must use ARIA without producing annoying hover-name popups.

---

# 9. Main navigation

Primary navigation is a compact top tab strip below the Windows titlebar:

```text
Drivers    History    Settings
```

Do not use a permanent left navigation rail in the current design.

The default page is `Drivers`, and the default Drivers mode is `Review`.

Drivers has two deliberately different levels:

```text
Review       → decisions, local findings and meaningful entry points
All hardware → the complete grouped Windows inventory for inspection
```

The application must not open by dumping every Plug and Play device into a flat list. Raw inventory is a secondary technical surface.

Do not create a separate dashboard/home page. `Review` is the product's decision surface and should answer quickly whether anything deserves attention.

---

# 10. Core domain model

Model these concepts explicitly.

## 10.1 Device

A device record should support, where available:

```text
device instance ID
device description
friendly name
manufacturer
device class
class GUID
hardware IDs
compatible IDs
VEN/DEV
SUBSYS
REV
USB VID/PID
ACPI IDs
parent/child relationship if useful
problem/status code
present/connected state
system model applicability
```

Do not assume every device is PCI.

Support conceptual device families such as:
- PCI/PCIe
- USB
- ACPI
- Bluetooth
- display
- audio
- network
- chipset/system
- storage
- HID/input
- monitor
- other PnP devices

## 10.2 Installed driver

Record:

```text
device
provider
manufacturer
INF
published OEM INF name
version
date
class
signer
signature validity
WHQL state when determinable
driver store package identity
matching ID used
installed source when known
installation timestamp when known
```

## 10.3 Driver candidate

Normalize every source into one shared candidate model.

Candidate should support:

```text
candidate ID
source
source-specific ID
provider
manufacturer
display name
version
date
supported OS
supported architecture
hardware IDs
compatible IDs
subsystem specificity
OEM model applicability
INF/package identity
download URL
release notes URL
release channel
WHQL/signature state
hash/checksum when available
security classification if known
known issues if known
package type
installer type
reboot expectations
metadata retrieval timestamp
```

Do not let source-specific metadata leak everywhere through the application.

## 10.4 Recommendation

The recommendation model must explain:

```text
recommended candidate or "keep current"
recommendation level
confidence/evidence level
positive factors
negative factors
hard compatibility decisions
reason current driver may be preferable
reason newer candidates were not selected
```

---

# 11. Driver sources

DrvMatch must use multiple trusted sources.

Do not rely on a giant proprietary/torrent driver pack as the main architecture.

Do not host a mirrored driver collection as the core system.

Prefer downloading directly from official/trusted sources.

---

# 12. Phase-one sources

The first source adapters are:

```text
Windows Update
Microsoft Update Catalog
AMD
NVIDIA
Intel
```

Each source must implement a normalized adapter boundary.

Conceptually:

```text
trait DriverSource {
    discover(...)
    fetch_metadata(...)
    resolve_download(...)
}
```

The actual API may differ, but the separation must remain.

Source networking/parsing logic must not be embedded into ranking code.

---

# 13. Windows Update source

Windows Update should be treated as a high-trust applicability source.

Investigate current supported Windows Update Agent / Windows APIs rather than scraping the
Windows Settings UI.

DrvMatch should be able to discover driver updates applicable to the current system where
Windows exposes them.

Useful metadata may include:
- provider
- model
- matching device information
- driver date/version
- applicability

Do not assume Windows Update offering a driver means it is automatically the best choice.
It is one strong signal.

---

# 14. Microsoft Update Catalog source

Microsoft Update Catalog is an important candidate source and historical/fallback source.

Use Hardware IDs and related identifiers to discover candidates.

The adapter must be isolated because the Catalog does not provide a clean modern public API
for every desired operation.

Do not spread HTML/parser assumptions across the project.

If scraping or an unofficial endpoint is required:
- isolate it
- cache responsibly
- make parsing testable
- fail gracefully when the website changes
- never pretend Catalog is available when parsing failed

Catalog candidates still pass through the same compatibility and ranking engine.

---

# 15. AMD source

Support official AMD packages for relevant hardware:

- Radeon
- Radeon Pro when relevant
- Ryzen chipset packages
- AMD integrated graphics where appropriate

Preserve branch/channel distinctions where AMD exposes them, for example:

```text
Recommended
Optional
WHQL
Preview/Beta
```

A newer Optional package must not automatically outrank an older Recommended package.

Where a package is an official AMD installer rather than a simple INF bundle, model that
fact explicitly.

---

# 16. NVIDIA source

Support official NVIDIA driver families.

Preserve distinctions such as:

```text
Game Ready
Studio
WHQL
Beta / hotfix where relevant
```

User/workload relevance may affect recommendation.

Example:
- Gaming-oriented user: Game Ready may receive preference
- Creation/stability-oriented profile: Studio may receive preference

Do not overfit based on assumptions. Provide explicit setting/profile behavior if user
profile becomes part of ranking.

Use official NVIDIA download endpoints/metadata where feasible.

---

# 17. Intel source

Support official Intel packages where relevant:

- graphics
- Wi-Fi
- Bluetooth
- Ethernet
- chipset/system components
- storage when applicable

Prefer current documented Intel endpoints/metadata.

Do not hardcode undocumented endpoints without an isolated adapter and tests.

---

# 18. OEM sources

After phase-one sources, expand to OEM-specific sources.

Priority OEM families:

```text
Dell
Lenovo
HP
ASUS
MSI
Gigabyte
ASRock
Acer
```

Add others only when justified.

OEM sources are especially important for:

- laptops
- prebuilt desktops
- custom audio packages
- touchpad
- hotkeys
- sensors
- power-management devices
- camera
- fingerprint readers
- firmware-linked packages
- vendor-specific extensions/APOs
- OEM-specific networking packages

Use structured OEM catalogs when available.

Examples of desirable source mechanisms:
- Dell driver pack catalogs
- Lenovo package descriptors/catalogs
- HP SoftPaq/HPIA metadata

For motherboard vendors and less structured sources, isolate provider-specific parsing.

---

# 19. Realtek / MediaTek / Qualcomm / similar vendors

Do not assume the chipset vendor's newest generic driver is preferable.

For these devices, source priority often depends on context.

Example:

```text
ASUS OEM audio package
vs
Microsoft Catalog Realtek package
vs
generic Realtek package
```

All may be valid candidates.

The recommendation engine must consider package specificity and OEM extensions.

When a trustworthy first-party chipset-vendor source is available, support it through its own
adapter. Otherwise, OEM + Microsoft sources may be the safer recommendation path.

---

# 20. Community sources

Community driver packs/repositories are NOT enabled as primary sources.

Do not use SDIO-style torrent indexes as a core dependency.

A future advanced/experimental source may be considered only if:
- it is optional
- clearly labeled
- never outranks trusted official sources merely because it is newer
- package signatures are verified
- provenance is clear

This is not required for 1.0.0.

---

# 21. Compatibility filtering

Before ranking, perform hard compatibility filtering.

Reject or strongly block candidates that are clearly wrong.

Examples:

- wrong architecture
- unsupported Windows version/build
- no relevant device ID match
- package intended for another hardware family
- invalid/corrupt signature when a valid signature is required
- known blocked/revoked driver
- impossible OEM applicability
- malformed package metadata

Compatibility must be deterministic and testable.

Do not "try installing it and see" as a matching strategy.

---

# 22. Match specificity

Match quality is one of the strongest signals.

Conceptual ordering:

```text
Exact hardware ID + exact subsystem/model context
>
Exact hardware ID
>
Specific compatible ID
>
Generic compatible ID
```

But do not hardcode one simplistic ordering for every device class.

For OEM-specific packages:
- exact system model applicability is a strong signal
- exact SUBSYS can be a strong signal
- extension INF / component INF relationships matter

For devices without SUBSYS, do not penalize them for not having one.

---

# 23. Source trust

Maintain a source trust model.

Potential high-trust sources:

```text
exact OEM package
component manufacturer official source
Windows Update
Microsoft Update Catalog
```

Source trust is only one factor.

Do NOT implement:

```text
OEM = always wins
```

or:

```text
Microsoft = always wins
```

A recommendation is based on the combined evidence.

---

# 24. Signing and integrity

Before installation:

- verify package integrity
- verify Authenticode/signing where applicable
- inspect certificate/signer
- validate hash if an authoritative expected hash is available
- reject obviously tampered/corrupt downloads
- record verification results

Use Windows-supported verification APIs such as WinVerifyTrust where appropriate.

Unsigned driver installation must not be a normal supported flow.

Do not disable Windows driver-signing protections.

---

# 25. WHQL and release channels

WHQL/recommended/stable status is meaningful but not absolute.

Possible signals:

```text
WHQL
Recommended
Stable
Production
Optional
Preview
Beta
Hotfix
```

Default ranking should prefer stable/recommended channels unless:
- a newer branch fixes a relevant serious problem
- the user explicitly opts into optional/beta channels
- a device-specific case justifies another choice

Do not hide the channel from the user.

---

# 26. Recency

Version and date are ranking signals with deliberately limited weight.

Recency should help distinguish otherwise similar candidates.

Recency must NOT overpower:
- a much stronger hardware match
- exact OEM applicability
- a stable recommended branch
- known regressions
- signature/trust problems
- OS incompatibility

---

# 27. Known issues and regressions

Architecture must allow candidates to carry:

```text
known issues
known regressions
fixed issues
security relevance
affected hardware/software
```

Early releases may not have a comprehensive global regression database, but the data model
must not make this impossible later.

When reliable vendor release notes expose known issues:
- parse/store relevant metadata where practical
- link to release notes
- do not invent issue interpretations

A known severe regression should strongly penalize a recommendation for affected systems.

---

# 28. Security fixes

Security-related driver updates may deserve stronger recommendation weight.

Only mark a candidate security-relevant when supported by reliable source data.

Do not label ordinary updates "security updates" to make them look important.

---

# 29. Recommendation engine

Create a dedicated ranking/recommendation subsystem.

Suggested conceptual pipeline:

```text
Enumerate devices
        ↓
Read installed packages
        ↓
Query sources
        ↓
Normalize candidates
        ↓
Hard compatibility filter
        ↓
Feature extraction
        ↓
Candidate ranking
        ↓
Keep-current comparison
        ↓
Recommendation + explanation
```

The ranking engine must operate on normalized candidate data.

It must not fetch webpages directly.

---

# 30. Internal score

An internal numerical score is allowed and useful.

The scoring system should be:
- deterministic
- decomposable into factors
- testable
- explainable
- configurable without rewriting the entire engine

Possible factors include:

```text
match specificity
OEM applicability
source trust
signature/WHQL
release channel
OS/build match
architecture match
recency
security fix relevance
known regression penalty
known fix relevance
package completeness
user profile relevance
```

Do not expose the raw number as fake certainty in the normal UI.

The normal UI uses:

```text
Recommended
Good alternative
Optional
Current
Not recommended
```

Advanced/Technical view may expose:
- internal score
- factor breakdown
- rejected-candidate reasons

---

# 31. Keep-current logic

DrvMatch must compare the best available candidate against the currently installed driver.

The engine should be able to conclude:

```text
KEEP CURRENT
```

Reasons may include:

- installed package is exact OEM match
- newer package is generic
- newer package is Optional while installed is Recommended
- improvement is negligible
- candidate has a known issue
- installed driver has required OEM extensions
- user has no relevant issue that the newer branch fixes

This is a core feature, not an edge case.

---

# 32. Recommendation thresholds

Avoid recommending a change when evidence is weak.

Conceptually use thresholds such as:

```text
candidate must be valid
candidate must beat current suitability by meaningful margin
candidate must not carry disqualifying penalties
```

Exact thresholds should be covered by tests and tuned with real hardware data.

Do not create recommendations merely to make the app look useful.

It is completely valid for a scan to return:

```text
No recommended updates
```

---

# 33. Device inventory

DrvMatch must scan and present all relevant devices.

The inventory should distinguish:

```text
Current
Recommended update
Optional update
Missing driver
Problem device
```

Do not flood the main UI with irrelevant virtual/system devices unless they matter.

Provide filters or grouping for advanced users.

Potential categories:

```text
Display
Chipset/System
Network
Bluetooth
Audio
Storage
USB
Input
Monitor
Other
```

Category is for navigation/filtering, not decorative color coding.

---

# 34. Missing-driver workflow

Fresh Windows installs are a major use case.

Expected flow:

```text
Scanning hardware...

32 devices
21 Current
6 Recommended
3 Generic Microsoft driver
2 Missing
```

For a missing device:

```text
Unknown Network Controller

Hardware ID:
PCI\VEN_.......

Identified as:
Realtek ...

Recommended driver:
...

Source:
...

Exact match:
Yes
```

Then allow explicit installation.

Unknown devices should be identified from reliable hardware IDs.

Do not guess a device identity from a vague marketing name when exact IDs disagree.

---

# 35. Generic Microsoft drivers

The app should recognize situations where Windows is using a generic class driver.

A generic Microsoft driver is not automatically bad.

Label it carefully.

Possible status:

```text
Generic driver installed
Vendor driver available
```

Recommendation depends on:
- functionality
- device class
- vendor package quality
- exact applicability
- real benefit

Do not replace every generic class driver by default.

---

# 36. GPU handling

GPU drivers deserve specialized presentation.

For AMD:

```text
AMD Radeon RX 7600
Adrenalin 26.x
Recommended / Optional
```

For NVIDIA:

```text
GeForce ...
Game Ready / Studio
```

Do not expose only raw internal Windows driver version numbers when the vendor has a more
meaningful public package version.

Installation may use the official vendor installer instead of forcing a raw INF install.

Model this explicitly.

Where possible:
- show release notes
- show channel
- show reboot expectation
- preserve vendor installer behavior

Do not silently strip vendor components unless DrvMatch eventually implements an explicit
advanced "minimal install" feature. That is not required for 1.0.

---

# 37. Chipset packages

Chipset packages may install multiple related components.

Treat them as packages, not as unrelated version bumps on every child device.

Do not recommend repeatedly installing the same chipset package for each child component.

Present the user-facing package coherently.

---

# 38. Audio drivers

Be conservative with OEM audio.

OEM audio packages may include:

```text
extension INF
software components
APO
DTS/Dolby/vendor processing
jack configuration
control applications
```

A newer generic Realtek package may be a worse user experience.

This is a strong example of why DrvMatch exists.

---

# 39. Firmware and BIOS

BIOS/UEFI and firmware updates are NOT ordinary drivers.

Do not silently merge them into the standard "Install recommended drivers" flow.

If detected/supported, classify separately:

```text
Firmware / BIOS
```

For early milestones:
- show metadata if safely discoverable
- direct the user to the official OEM workflow
- do not implement aggressive automated BIOS flashing

Automated BIOS update is outside the 1.0 core scope unless explicitly added later.

---

# 40. Driver download subsystem

Downloads must:

- come from trusted URLs
- support cancellation
- show progress
- validate expected size where known
- compute SHA-256
- verify signatures/packages
- use a managed cache directory
- handle partial/corrupt files safely
- never execute an unverified partial download

Cache metadata in SQLite.

Do not keep unlimited stale packages forever.

Implement safe cleanup.

---

# 41. Installation subsystem

Driver installation requires elevated privileges.

Principles:

- main app should run unelevated when possible
- request elevation only for operations that require it
- do not run the entire application permanently as administrator without need
- never bypass UAC/security protections

For INF packages, prefer supported Windows mechanisms such as:
- PnPUtil
- SetupAPI / documented Windows driver installation APIs

Research current documented best practices before implementing.

Do not construct unsafe shell command strings from untrusted metadata.

Use structured process invocation.

---

# 42. Vendor installers

Some sources provide EXE/MSI/vendor installers instead of raw INF packages.

Model installer type:

```text
InfPackage
VendorExe
Msi
OtherSupportedPackage
```

Do not apply one installation mechanism to all packages.

For vendor installers:
- use documented silent switches only when reliable
- otherwise launch official installer explicitly
- preserve trust/signature validation
- report that control is being handed to the official installer when applicable

---

# 43. Restore/checkpoint strategy

Before risky driver changes, DrvMatch should create recovery context where feasible.

Potential mechanisms:

- system restore point
- record installed package identity
- export/backup current driver package when supported
- record current device state
- preserve enough metadata to guide rollback

Settings should include:

```text
Create restore point before installation
Back up current package when possible
```

Safe defaults should be ON where technically reasonable.

If restore-point creation fails:
- tell the user
- do not silently claim protection exists
- allow user to decide whether to continue when appropriate

---

# 44. Rollback

History should expose rollback when technically possible.

Rollback strategy may include:
- Windows device rollback
- reinstalling previous package
- restoring backed-up INF package
- vendor-specific rollback where applicable

Do not display a Rollback button unless the application has a real rollback path.

Track rollback result.

---

# 45. Reboot handling

Every install should track whether reboot is:

```text
not required
recommended
required
unknown
```

Do not force reboot without explicit user action.

The bottom status area may show:

```text
Restart required
```

Offer:
- restart now
- restart later

Only implement "restart now" after explicit user interaction.

---

# 46. Installation queue

Users may select multiple recommended updates.

Before executing:
- resolve package relationships
- avoid duplicate package installs
- establish safe order
- detect operations likely to disrupt network/display/audio
- present a review

Example:

```text
Ready to install

1. AMD Chipset
2. Realtek Ethernet
3. MediaTek Bluetooth
```

Queue state must survive UI rerenders and report per-item result.

If one package fails, do not blindly continue when the failure makes later operations unsafe.

---

# 47. History

Maintain a persistent driver operation history.

Store at minimum:

```text
timestamp
device/package
previous version
new version
source
operation
result
error if any
reboot requirement
rollback availability
rollback result
```

History UI must support:
- chronological review
- details
- rollback action where available

History should be stored locally.

---

# 48. Scan history

Store useful scan metadata:

```text
scan timestamp
device count
source availability
candidate counts
recommendations
source errors
```

Do not store enormous duplicated blobs unnecessarily.

The product should be able to explain when a recommendation changed because source metadata
changed between scans.

This does not need to become a full PC Change Tracker in 1.0.

---

# 49. SQLite

Use SQLite for local structured state.

Suggested data areas:

```text
devices
installed_drivers
driver_candidates
sources
scans
recommendations
downloads
install_history
rollback_records
settings
metadata_cache
```

Use migrations.

Do not mutate schema ad hoc without migration support once persistence is established.

Do not store sensitive secrets unless necessary.

DrvMatch should not require an account or cloud backend.

---

# 50. Privacy

DrvMatch should work locally and not require telemetry.

Default:
- no analytics
- no account
- no cloud synchronization
- no sale/sharing of device inventory

Network requests are made only to retrieve driver metadata/packages from configured sources.

If telemetry is ever introduced later:
- opt-in
- clearly documented
- never required for core operation

Do not add telemetry during the initial project unless explicitly requested.

---

# 51. Networking

Use a consistent HTTP client layer.

Requirements:

- TLS verification enabled
- reasonable timeouts
- cancellation
- bounded retries
- descriptive source errors
- user agent identifying DrvMatch where appropriate
- cache validators (ETag/Last-Modified) where useful
- rate-limit respectful behavior

Do not disable certificate verification.

---

# 52. Source failure behavior

A source failure must not make the entire scan fail when other sources are available.

Example:

```text
Microsoft Update Catalog unavailable

Windows Update, AMD and Intel completed normally.
```

Display partial-source status calmly.

Keep source health information available in advanced/details/settings.

---

# 53. UI: Drivers page

This is the main product surface, but it is not a raw inventory screen.

The current hierarchy is:

```text
Windows titlebar
Top navigation: Drivers / History / Settings
Compact Drivers header
Review / All hardware mode switch
Main content
Persistent bottom operation rail
```

## Review — default

Review should answer in a few seconds:

> Does this machine need attention, and where should I look?

It contains:
- machine identity and scan freshness
- a calm overall local-device verdict
- missing/problem counts
- missing/problem devices surfaced individually when present
- generic Microsoft packages presented as a review opportunity, not an alarm
- a hardware-category map that provides intentional entry points into the full inventory

Do not put all 100–300 PnP devices on screen by default.

Do not auto-select the first device after a scan.

Do not use a fake health percentage or convert the Review screen into a grid of SaaS statistic cards.

## All hardware — secondary

This is the complete technical inventory. It supports:
- search
- category filtering
- status filtering
- keyboard navigation
- grouped hardware categories
- explicit device selection
- the technical inspector

Group devices into meaningful categories such as:

```text
Display
Network
Audio
Bluetooth
Storage
Input
System
USB
Camera
Other
```

The inventory should feel deliberate rather than like a prettier Device Manager dump.

---

# 54. UI: review states and filters

Local Review states include:

```text
Problem
Missing driver
Generic Microsoft package
Healthy/current
```

`Generic Microsoft` is not inherently bad. Treat it as calm context and only elevate it when better evidence exists.

All hardware may filter by:

```text
All devices
Needs attention
Problems
Missing driver
Generic Microsoft
Hardware category
```

Candidate recommendation states remain:

```text
Recommended
Optional
Current
Missing
Not recommended
```

Counts should be integrated into controls/content. Do not turn them into oversized metric cards.

---

# 55. Hardware rows and explicit inspection

A row in All hardware should communicate:

```text
device identity
hardware category / manufacturer
local condition
installed package/version when one exists
```

Rows are compact and separated by structure rather than individual floating cards.

Selecting a row opens the inspector. Full driver-source discovery is an explicit action such as `Check this device` or `Compare trusted drivers`; do not imply that local inventory enumeration has already queried every online driver source.

Use responsive truncation inside the fixed window where needed. The full information belongs in the inspector, not an automatic hover tooltip.

---

# 56. UI: details pane

Tabs:

```text
Overview
Candidates
Technical
```

Overview:
- device
- installed driver
- recommended driver/current recommendation
- source
- signature/WHQL
- "Why this driver?"
- install/rollback action

Candidates:
- ranked candidates
- reason for ranking
- reason newest may not be recommended
- source/channel
- compatibility specificity

Technical:
- hardware IDs
- compatible IDs
- INF
- signer
- provider
- package identity
- architecture
- driver date
- exact matching ID
- internal score/factors
- source metadata

---

# 57. UI: Why this driver?

This is one of the signature features.

Example:

```text
Why this driver

✓ Exact hardware ID
✓ Exact subsystem
✓ Correct Windows build
✓ Microsoft-signed
✓ WHQL
✓ Stable branch
```

Also explain tradeoffs:

```text
A newer generic Realtek driver exists, but this OEM package is more specific to this
machine and includes vendor extensions.
```

Do not just expose an unexplained score.

---

# 58. UI: candidates

Candidates must be transparent.

Example:

```text
★ Recommended
11.26.520.2026
Microsoft Update Catalog
Exact HW ID · Exact SUBSYS · WHQL

Alternative
11.28.610.2026
Realtek
Exact VEN/DEV · Generic SUBSYS · WHQL

Newer, but less specific for this machine.
```

If a candidate is rejected:
- advanced users should be able to see why
- do not offer normal installation for incompatible candidates

---

# 59. UI: progress/footer

Use a persistent bottom operation/status area.

Normal:

```text
32 devices · Last scan 2 min ago                    Ready
```

Selected:

```text
3 drivers selected                    [Install selected]
```

Installing:

```text
Installing Realtek Ethernet   ███████░░ 72%   2 of 3
```

Completed:

```text
3 drivers installed · Restart required              [Restart later]
```

Do not rely on a blocking modal for all progress.

---

# 60. UI: History

History page uses compact rows/table-like structure.

Do not make each event a giant card.

Show:
- timestamp
- device/package
- previous/new version
- source
- result
- rollback

---

# 61. UI: Settings

Settings is a focused full workspace using a horizontal category strip rather than a second permanent sidebar.

Settings categories:

```text
Appearance
Sources
Safety & Rollback
Advanced
About
```

Potential General options:
- scan on startup
- optional background check
- notifications
- language later if localization is added

Appearance:
- System / Light / Dark
- Acrylic on/off
- Windows accent color on/off
- Reduce motion

Sources:
- source enable/disable controls
- source status
- last successful refresh
- cache refresh where appropriate

Safety & Rollback:
- restore point
- backup current package
- confirm optional updates
- rollback behavior

Advanced:
- show internal rank score
- technical logging
- beta/preview channels
- metadata/cache controls
- experimental sources later

About:
- version
- repository
- licenses
- source acknowledgements

---

# 62. Acrylic

Acrylic is ON by default.

It must be user-disableable.

When OFF:
- switch shell/navigation/transient surfaces to solid fallback
- do not change layout
- do not change spacing
- do not introduce gradients to compensate

Use current supported Tauri/Windows mechanisms.

Do not use undocumented hacks if a maintained solution exists.

---

# 63. No hover-name tooltips

Strict rule:

Do not display automatic tooltips when hovering over normal controls.

Do not add `title` attributes to normal interactive UI.

Do not wrap icon buttons in a tooltip just to repeat:
- "Settings"
- "Close"
- "Scan"
- "More"
- etc.

Use ARIA names instead.

If a feature genuinely needs explanatory help, prefer:
- inline copy
- details pane
- explicit info action

Only add a tooltip if the user later explicitly asks for that particular tooltip.

---

# 64. Accessibility

Even though hover tooltips are disabled:

- icon-only controls need ARIA labels
- keyboard focus must be visible
- dialogs must trap/restore focus correctly
- tabs need keyboard support
- list selection must be keyboard usable
- status cannot rely only on color
- respect reduced motion
- support forced colors/high contrast where practical

---

# 65. Theme

Support:

```text
System
Light
Dark
```

Do not build dark mode first and leave light mode broken.

Both themes are first-class.

Use the Windows accent color when enabled and safely obtainable.

Provide a coherent fallback accent.

---

# 66. Visual rules

Follow `DESIGN.MD`.

Especially avoid:

- giant rounded cards
- card grid dashboards
- gradient headings
- oversized shadows
- hover lift
- excessive glass
- excessive pills
- colored icon bubbles everywhere
- decorative illustrations
- fake system-health numbers
- unnecessary marketing copy

DrvMatch should look like a carefully designed Windows utility.

---

# 67. shadcn usage

shadcn/shadcn-svelte is not the design system.

Use it only for useful accessible primitives.

Before adding a shadcn component:

1. Search the registry.
2. Inspect its implementation.
3. Confirm Svelte 5 compatibility.
4. Check whether DrvMatch already has an equivalent.
5. Add only if it provides real value.
6. Restyle completely through DrvMatch tokens.

Do not let default shadcn styling dictate:
- border radii
- card layouts
- page composition
- typography
- color system

---

# 68. Component system

Create a coherent reusable component layer.

Potential primitives:

```text
Button
IconButton
Dialog
ConfirmDialog
Tabs
Menu
MenuButton
Dropdown/Select
SearchBox
StatusBadge
ProgressBar
ToggleSwitch
Checkbox
RadioGroup
AdvancedDisclosure
InlineError
Skeleton
Surface
```

Potential domain components:

```text
DriverRow
DriverDetailsPane
CandidateRow
RecommendationBadge
WhyThisDriver
SourceLabel
InstallQueue
OperationFooter
HistoryRow
DeviceIdentity
TechnicalMetadata
```

Do not prematurely create abstractions for components used once.

---

# 69. Frontend state

Keep state explicit and typed.

Suggested areas:

```text
navigation
scan state
device inventory
recommendations
selection
installation queue
operation progress
history
settings
notifications
source health
```

Avoid one giant global store containing unrelated data.

Live native/backend events should update stores through well-defined event handling.

---

# 70. Backend/frontend boundary

The Rust side should own:
- device enumeration
- installed driver inspection
- source adapters where appropriate
- recommendation engine
- downloads
- verification
- installation
- rollback
- SQLite persistence
- privileged operations
- source cache

The Svelte side should own:
- presentation
- interaction
- filtering/sorting of already-normalized view models where reasonable
- local UI selection
- dialogs
- accessibility behavior

Do not duplicate recommendation logic in TypeScript.

---

# 71. Error model

Create structured error types.

Do not pass opaque `"something failed"` strings everywhere.

Errors should identify:
- subsystem
- source
- operation
- retryability
- user-safe message
- technical detail/log context

UI shows concise user-safe messaging.

Technical logs preserve diagnostic detail.

---

# 72. Logging

Use structured local logging.

Log:
- scan lifecycle
- source failures
- candidate normalization problems
- recommendation decisions at debug level
- download verification
- install/rollback operations
- unexpected native API errors

Do not log secrets.

Provide an advanced way to open/copy logs later.

Keep log retention bounded.

---

# 73. Tests

Testing is mandatory.

## Rust tests

Cover:
- hardware ID normalization
- version parsing/comparison
- OS/architecture filtering
- match specificity
- ranking factor behavior
- keep-current decisions
- source normalization
- package verification helpers
- database migrations
- install state transitions where testable

Use fixtures representing real-ish devices.

Examples:
- Realtek NIC exact SUBSYS vs generic newer package
- OEM Realtek audio vs generic package
- AMD Recommended vs newer Optional
- NVIDIA Game Ready vs Studio profile
- missing Wi-Fi controller
- generic class driver currently installed

## Frontend tests

Cover:
- status presentation
- selection
- filters
- settings persistence presentation
- install queue state
- details/candidate rendering
- disabled/incompatible actions
- keyboard interactions for important components

Do not test only snapshots.

---

# 74. Fixtures

Maintain test fixtures for normalized source data.

Do not make tests depend on live Microsoft/AMD/NVIDIA websites.

Network parsers should have captured sanitized fixtures.

Live integration tests can be optional/separate.

This protects the project from source websites changing during normal test runs.

---

# 75. Development quality gates

Before milestone release:

Frontend:
```text
format
lint if configured
typecheck
tests
production build
```

Rust:
```text
cargo fmt --check
cargo clippy
cargo test
cargo check/build
```

Tauri:
- application builds
- packaged configuration valid

Do not create milestone tags with known failures.

---

# 76. Security posture

DrvMatch downloads and installs privileged code.

Treat security as core product behavior.

Never:
- execute arbitrary source metadata as shell
- disable TLS verification
- disable driver signing
- use unquoted concatenated shell commands
- trust file extensions
- run downloaded binaries before verification
- allow a webview to invoke unrestricted privileged commands
- expose broad Tauri capabilities unnecessarily

Use least-privilege Tauri capabilities.

Validate every privileged command argument.

---

# 77. Tauri security

Configure capabilities narrowly.

Frontend commands should call explicit backend operations.

Bad:

```text
run_any_command(command: string)
```

Good:

```text
scan_devices()
install_candidate(candidate_id)
rollback_install(history_id)
```

Do not expose a generic privileged shell.

---

# 78. Update mechanism for DrvMatch itself

DrvMatch may eventually use Tauri's application updater.

Keep application self-update separate from driver updates.

The UI should clearly distinguish:

```text
DrvMatch update
Driver update
```

Do not confuse the two.

If self-update is implemented:
- signed updates
- official repository/release source
- explicit version display
- safe restart behavior

---

# 79. Search and filters

Search should match useful local metadata:

- device name
- vendor
- class
- hardware ID
- provider
- package/source where useful

Search should be debounced.

Do not trigger live web queries for every keystroke.

Filters operate on scan/candidate state.

---

# 80. Sorting

Useful sort modes may include:

```text
Recommendation
Device name
Device class
Source
Installed version
```

Default sort should surface recommended/missing/problem devices first while remaining stable.

---

# 81. Advanced mode

Do not create an entirely separate "power user UI" initially.

Instead expose deeper data through:

```text
Technical tab
Advanced settings
optional internal score visibility
```

The base UI remains understandable.

Advanced users should be able to inspect exact matching decisions.

---

# 82. User profiles

Do not require a profile for normal recommendations.

A future optional workload preference may include:

```text
Balanced
Gaming
Creator / Studio
```

Only use it when it meaningfully affects vendor branch selection.

Do not create fake personalization everywhere.

Not required for 0.1.0–0.4.0 unless needed for tests.

---

# 83. Notifications

Use notifications sparingly.

Useful:
- scan completed with recommendations
- install finished
- restart required
- rollback finished
- source unavailable when relevant

Do not spam on every source request.

Do not use notifications as marketing.

---

# 84. Tray behavior

Tray support is optional before 1.0.

If implemented:
- open DrvMatch
- scan
- show current operation
- quit

Do not let tray features distract from core reliability.

---

# 85. Background behavior

DrvMatch must not become a heavy resident process.

Default should be lightweight.

Optional future checks can be:
- at app startup
- scheduled at low frequency
- user controlled

Do not continuously poll driver sources.

No persistent hardware telemetry is needed for the core product.

---

# 86. Offline behavior

When offline:
- inventory and installed driver inspection still work
- cached candidate metadata may be shown as cached/stale
- installs requiring downloads are unavailable unless package cached
- clearly label source data freshness

Do not show cached candidates as newly verified online.

---

# 87. Cache

Cache:
- source metadata
- downloaded verified packages as configured
- normalized candidate data

Track:
- fetched_at
- source version/etag where available
- hash
- verification state

Provide cleanup policies.

Do not let cache grow indefinitely.

---

# 88. Candidate provenance

Every candidate must retain provenance.

UI/technical view should be able to answer:

```text
Where did this candidate come from?
When was metadata fetched?
What was matched?
What package is being downloaded?
What signature was verified?
```

Provenance is part of trust.

---

# 89. Explanation engine

Recommendation explanations should be generated from structured factors, not an LLM.

DrvMatch must work without AI services.

Example structured factors:

```text
ExactHardwareId
ExactSubsystem
ExactOemModel
Whql
StableChannel
NewerVersion
SecurityFix
GenericMatch
KnownRegression
OptionalChannel
```

Map factors to clear copy.

This makes explanations deterministic and testable.

---

# 90. No cloud AI dependency

Do not introduce:
- OpenAI API
- Anthropic API
- local LLM
- cloud recommendation model

The ranking engine is deterministic software.

AI is being used to develop DrvMatch, not required to run it.

---

# 91. Milestone 0.1.0 — Foundation

Goal:

> A polished, buildable DrvMatch desktop shell with real local device enumeration and the
> foundational project architecture.

Required:

- create GitHub repository
- initialize Tauri 2 + Svelte 5 + Rust project
- implement repository structure
- add SQLite migration framework
- implement DrvMatch design tokens
- implement custom non-resizable Windows titlebar
- implement Windows-style `— □ ×` controls
- implement rounded outer window
- implement acrylic enabled by default
- implement acrylic setting + solid fallback
- implement System/Light/Dark
- implement primary navigation for:
  - Drivers
  - History
  - Settings

Historical note: the initial implementation used a sidebar. The current post-0.9 design supersedes that layout with the top navigation defined in `DESIGN.MD`.
- implement bottom status bar
- implement real Windows device enumeration
- display device list from real data
- basic device details:
  - friendly name
  - class
  - manufacturer
  - hardware IDs where available
- no fake driver recommendations yet
- loading/error states
- baseline tests
- README bootstrap
- CHANGELOG

Acceptance:
- app launches on Windows
- window behaves per design
- acrylic can be disabled
- device inventory is real, not mocked
- no reference code imported from `design-refence/`
- clean production build
- tests/checks pass

Release:
- update version to `0.1.0`
- commit
- create annotated tag `0.1.0`
- push commit and tag

---

# 92. Milestone 0.2.0 — Installed driver inventory

Goal:

> DrvMatch understands what driver is currently installed for each relevant device.

Required:

- enumerate installed driver package metadata
- connect installed package to device
- provider
- version
- date
- INF
- published OEM INF when available
- signer/signature metadata
- matching ID where possible
- driver store identity where possible
- detect obvious missing/problem devices
- detect generic Microsoft driver situations where reliably possible
- populate Technical tab
- persist scans/inventory in SQLite
- add search and basic filters
- add real History page skeleton for scans

Acceptance:
- real installed driver metadata visible
- IDs/versions are correctly associated with devices
- missing/problem devices distinguishable
- persisted scan can be reloaded
- tests for normalization/parsing

Release:
- version `0.2.0`
- changelog
- release commit
- tag `0.2.0`
- push commit and tag

---

# 93. Milestone 0.3.0 — Microsoft sources

Goal:

> DrvMatch discovers real candidate drivers from Microsoft sources.

Required:

- Windows Update source adapter
- Microsoft Update Catalog source adapter
- normalized candidate model
- metadata cache
- source health/error states
- compatibility pre-filter
- candidate list in UI
- source/provenance shown
- package download metadata resolution where feasible
- parser/network fixtures
- no automatic recommendation yet beyond hard compatibility

Acceptance:
- real current machine can discover Microsoft candidates
- source failures do not crash scan
- candidates are normalized
- incompatible candidates are rejected/explained
- live web dependency not required for unit tests

Release:
- version `0.3.0`
- changelog
- release commit
- tag `0.3.0`
- push commit and tag

---

# 94. Milestone 0.4.0 — DriverRank / recommendation engine

Goal:

> The application can choose the best candidate or intentionally keep the current driver.

Required:

- hard compatibility filtering
- feature extraction
- internal scoring/ranking
- keep-current comparison
- recommendation states:
  - Recommended
  - Optional
  - Current
  - Missing
  - Not recommended
- structured explanation factors
- `Why this driver?`
- Candidates tab with ranked alternatives
- newest-but-not-best explanation
- advanced internal score/factor view
- extensive fixture tests

Core test scenarios:
- exact SUBSYS older package beats generic newer package
- AMD Recommended older package beats newer Optional package by default
- current exact OEM package beats generic candidate
- missing device recommends exact compatible candidate
- known penalty can prevent recommendation
- incompatible architecture is rejected

Acceptance:
- ranking is deterministic
- explanation is generated from factors
- current driver can validly win
- UI never labels all newer drivers as updates

Release:
- version `0.4.0`
- changelog
- release commit
- tag `0.4.0`
- push commit and tag

---

# 95. Milestone 0.5.0 — AMD / NVIDIA / Intel

Goal:

> First-party component-vendor sources enrich DrvMatch recommendations.

Required:

- AMD source
- NVIDIA source
- Intel source
- branch/channel metadata
- public vendor package versions
- release notes links where available
- vendor-specific presentation
- GPU special handling
- chipset package grouping
- candidate de-duplication across sources
- source priority is evidence, not absolute override

Acceptance:
- supported hardware discovers vendor candidates
- GPU UI shows meaningful vendor versions/channels
- duplicate Microsoft/vendor candidates are reconciled where possible
- ranking includes stable/optional channel semantics
- tests use fixtures

Release:
- version `0.5.0`
- changelog
- release commit
- tag `0.5.0`
- push commit and tag

---

# 96. Milestone 0.6.0 — Safe install and rollback

Goal:

> DrvMatch can safely install recommended packages.

Required:

- managed downloads
- progress/cancel
- SHA-256
- signature verification
- install queue
- explicit pre-install review
- least-privilege elevation flow
- INF installation using supported Windows mechanism
- vendor installer handoff/support where implemented
- restore point attempt
- current package backup/metadata preservation where possible
- install result tracking
- reboot requirement tracking
- rollback where technically supported
- persistent operation footer
- no generic privileged shell API

Acceptance:
- package is verified before install
- install requires explicit user action
- failure produces actionable state
- history records before/after
- rollback shown only when real
- no driver-signing bypass

Release:
- version `0.6.0`
- changelog
- release commit
- tag `0.6.0`
- push commit and tag

---

# 97. Milestone 0.7.0 — History, settings and source management

Goal:

> The core app is manageable and transparent.

Required:

- complete History UI
- install details
- rollback details
- source settings
- source health
- Safety & Rollback settings
- Appearance settings
- Advanced settings
- About/version/repository
- cache management
- log access/copy
- settings persistence
- dirty/save behavior where appropriate
- no hover-name tooltips

Acceptance:
- settings survive restart
- acrylic/theme/source/safety controls work
- history can explain what changed
- version shown matches release metadata

Release:
- version `0.7.0`
- changelog
- release commit
- tag `0.7.0`
- push commit and tag

---

# 98. Milestone 0.8.0 — OEM and fresh-install workflows

Goal:

> DrvMatch becomes especially useful after a clean Windows installation.

Required:

- initial OEM adapters, prioritizing:
  - Dell
  - Lenovo
  - HP
- then add supported motherboard/laptop OEMs where feasible:
  - ASUS
  - MSI
  - Gigabyte
  - ASRock
  - Acer
- OEM model detection
- OEM applicability matching
- missing-driver workflow
- unknown-device identification
- exact hardware ID search
- OEM-vs-generic recommendation logic
- careful OEM audio handling
- fresh-install summary

Acceptance:
- at least major structured OEM catalogs work
- missing network/device scenario is usable
- exact OEM packages can beat newer generic packages
- no "update everything" behavior

Release:
- version `0.8.0`
- changelog
- release commit
- tag `0.8.0`
- push commit and tag

---

# 99. Milestone 0.9.0 — Beta hardening

Goal:

> Feature-complete beta ready for broader testing.

Required:

- UX polish against DESIGN.MD
- light/dark/acrylic-off QA
- keyboard navigation QA
- high DPI QA
- forced-colors/high-contrast improvements
- source parser resilience
- retry/cancellation auditing
- database migration testing
- corrupted cache recovery
- interrupted install/download recovery
- source timeouts
- performance profiling
- startup time optimization
- package size review
- security review
- dependency audit
- README complete
- screenshots
- known limitations documented

Acceptance:
- no known critical data-loss/driver-install safety issue
- clean install works
- update from previous milestone works
- core sources stable enough for beta
- release build reproducible

Release:
- version `0.9.0`
- changelog
- release commit
- tag `0.9.0`
- push commit and tag

---

# 100. Milestone 1.0.0 — Stable release

Goal:

> A trustworthy first stable release.

Required:

- resolve release-blocking beta issues
- final ranking calibration
- final source validation
- install/rollback verification
- final UI consistency pass
- final accessibility pass
- final security pass
- final privacy documentation
- final licenses/notices
- final README
- final CHANGELOG
- packaging/install testing
- version consistency
- clean repository
- no placeholder UI
- no fake/mock source in production path

Acceptance:
- all core flows work end to end
- scan
- inspect
- compare candidates
- understand recommendation
- download
- verify
- install
- record history
- rollback where supported
- handle missing device
- settings persist
- safe failures
- all release checks pass

Release:
- version `1.0.0`
- changelog
- release commit
- annotated tag `1.0.0`
- push commit and tag

Do not call the project 1.0.0 merely because the UI looks finished.

---

# 101. Post-1.0 ideas — not required unless explicitly requested

Do not implement these early unless they naturally fall out of the architecture.

Possible future features:

- more OEM sources
- optional background scan scheduler
- export/import driver report
- offline driver cache
- portable diagnostics bundle
- LAN/shared package cache
- richer known-regression database
- optional community source adapters
- enterprise fleet features
- richer firmware presentation
- vendor-specific minimal GPU install
- restore-point manager
- device change timeline
- source contribution/plugin architecture

Do not let post-1.0 ideas cause scope creep before core quality is solid.

---

# 102. Engineering workflow

For each task:

1. Inspect relevant existing code using Serena.
2. Read current docs using Context7/Tauri MCP when working with external APIs.
3. Inspect `DESIGN.MD` for UI work.
4. Inspect relevant `design-refence/` files.
5. Write a short implementation plan internally/in task notes.
6. Implement the smallest coherent change.
7. Add/update tests.
8. Run targeted checks.
9. Review diff.
10. Commit coherent work.
11. Push when appropriate.
12. At milestone completion, perform full release procedure.

Do not generate thousands of lines without checking the architecture already present.

---

# 103. Research discipline

For Windows APIs, Tauri, Svelte and vendor sources:

- prefer official documentation
- use Context7 for current library docs
- use Tauri MCP for current Tauri behavior
- verify undocumented web endpoints before relying on them
- do not fabricate API methods
- do not base permanent architecture on a random blog snippet

If a source has no stable API:
- isolate the parser
- add fixtures/tests
- document fragility

---

# 104. Code quality

Prefer:

- small modules with clear responsibility
- explicit domain types
- exhaustive enums
- Result-based error handling
- dependency injection/interfaces around source/network/native boundaries where it aids tests
- deterministic pure functions for ranking
- comments explaining non-obvious decisions
- tests around dangerous logic

Avoid:

- huge god files
- catch-all utility modules
- magic numbers throughout UI
- stringly-typed state
- duplicated version parsing
- hidden global mutable state
- `unwrap()` in recoverable runtime paths
- generic privileged commands

---

# 105. Dependency policy

Before adding a dependency:

- verify it is maintained
- verify license
- verify need
- prefer small focused dependencies
- avoid duplicate libraries for the same job
- do not add large UI frameworks that fight the custom design system

Record important third-party licenses for release.

---

# 106. Performance expectations

The app should feel native.

Target behavior:
- fast startup
- device inventory quickly visible
- source network work asynchronous
- UI remains responsive during scan
- details lazily fetch deep metadata
- no unnecessary polling
- no animation-heavy list
- no loading all release notes for all candidates up front

Use concurrency carefully with rate limits.

---

# 107. Scan UX

A scan can have stages:

```text
Inspecting devices
Reading installed drivers
Checking Windows Update
Checking Microsoft Catalog
Checking vendor sources
Ranking candidates
Done
```

Expose high-level progress calmly.

Do not fake exact percentages when the total work is unknowable.

Indeterminate progress is acceptable.

---

# 108. Partial results

Allow useful results to appear as sources complete.

Do not require the user to wait for every slow source before seeing local inventory.

However:
- clearly indicate scan still running
- do not mark a recommendation final until required sources for that device have finished or
  the ranking engine knows the result is sufficiently complete

---

# 109. Recommendation freshness

Every recommendation should be associated with:
- scan time
- source metadata freshness

When cached:
- mark as cached if meaningful
- do not imply a fresh live check

---

# 110. Candidate de-duplication

The same driver/package may appear through:
- Windows Update
- Microsoft Catalog
- vendor source

Do not show obvious duplicates as three independent choices when they are the same package.

Create a robust identity strategy using:
- INF/package identity
- version/date
- hardware applicability
- hash where available
- provider/source IDs

Still preserve provenance from multiple sources.

---

# 111. Package relationships

Modern Windows drivers may involve:
- base driver
- extension INF
- software component
- companion app

Do not assume every driver is one INF.

Architecture should allow a candidate/package to contain multiple installable components.

This matters especially for:
- audio
- GPU
- OEM devices

---

# 112. Windows driver rank awareness

Windows itself uses driver ranking/matching rules.

DrvMatch should understand those concepts and expose them where useful.

Do not blindly duplicate Windows rank, because DrvMatch's product goal includes:
- source/channel stability
- OEM suitability
- known issues
- user-facing package semantics

But Windows matching/rank behavior is valuable evidence.

Research and test this carefully.

---

# 113. Version comparison

Do not compare driver versions as plain strings.

Support multi-part numeric versions and provider-specific package version schemes.

Keep public vendor package version separate from low-level INF version when they differ.

When version ordering is ambiguous:
- preserve raw value
- do not invent ordering

---

# 114. Dates

Windows INF driver dates can be misleading or intentionally set for ranking reasons.

Do not treat driver date as "release date" without source evidence.

Keep fields conceptually separate:

```text
INF driver date
vendor release date
metadata publication date
```

---

# 115. User trust

Every install recommendation should be understandable without requiring the user to trust
DrvMatch blindly.

The interface must make it easy to answer:

- Who published this?
- Is it signed?
- Is it WHQL/stable?
- Does it exactly match my device?
- Why is it better than what I have?
- Why is a newer option not recommended?
- Can I roll back?
- Will I need to restart?

This is the competitive advantage.

---

# 116. Language and wording

The initial UI language can be English unless localization is explicitly prioritized later.

Write concise system-tool copy.

Do not use hype.

Do not use emojis in production UI.

No:
```text
Amazing!
Supercharge your PC!
17 critical fixes!
AI-powered optimization!
```

Use:
```text
3 recommended updates
2 optional updates
Current driver is recommended
Restart required
```

---

# 117. No fake functionality

Never mark a feature complete when it is:
- a hardcoded mock
- a placeholder button
- a fake progress animation
- a candidate list with fabricated versions
- a rollback button that does nothing

Mocks/fixtures are allowed in tests and explicit development/demo modes.

Production paths must use real implementations before milestone acceptance.

---

# 118. Handling blockers

If blocked by:
- missing authentication
- unavailable vendor API
- undocumented behavior
- hardware-specific testing requirement
- admin permission behavior
- unavailable signing certificate

Do not fake completion.

Instead:
1. isolate the blocker
2. continue independent work where possible
3. document exact unresolved issue
4. request user input only when needed

---

# 119. Hardware testing

Never assume one development PC represents all Windows hardware.

Use:
- fixtures
- virtual/test devices where possible
- real machine observations
- multiple vendor metadata examples

For dangerous install behavior, prefer safe test devices/VMs where feasible.

Do not intentionally destabilize the user's primary machine just to test rollback.

---

# 120. Commit discipline

Use meaningful commit messages.

Examples:

```text
feat: enumerate Windows PnP devices
feat: add Microsoft Catalog candidate adapter
feat: explain driver recommendation factors
fix: preserve OEM subsystem specificity
test: cover AMD recommended vs optional ranking
docs: document driver source trust model
release: 0.4.0
```

Avoid:

```text
update stuff
changes
final
fix
```

Keep commits coherent.

---

# 121. Mandatory milestone Git procedure

At the end of every completed milestone:

```text
1. Verify working tree.
2. Run all release checks.
3. Update package.json version.
4. Update src-tauri/Cargo.toml version.
5. Update src-tauri/tauri.conf.json version.
6. Update CHANGELOG.md.
7. Update README/progress docs if needed.
8. Commit:
   release: X.Y.Z
9. Create annotated tag:
   X.Y.Z
10. Push main:
    git push origin main
11. Push the tag:
    git push origin X.Y.Z
12. Verify remote contains the commit and tag.
```

If repository tooling provides equivalent structured actions, use those instead of shelling out.

Do not skip the remote verification step.

---

# 122. Final product standard

DrvMatch is successful when it feels like:

> a trustworthy modern Windows utility built by someone who understands driver matching

not:

> a reskinned driver updater that shouts about version numbers.

The hardest parts deserve the most care:

- hardware identification
- package applicability
- candidate normalization
- source provenance
- ranking
- installation safety
- rollback
- explanation

Do not spend disproportionate time on decorative UI while these are weak.

---

# 123. First action when receiving this repository

When beginning from the current repository:

1. Read `AGENTS.MD`.
2. Read `DESIGN.MD`.
3. Read this `PROMPT.MD`.
4. Inspect the entire `design-refence/` tree.
5. Inspect Git state.
6. Create/verify GitHub repository `DrvMatch`.
7. Establish `main` and `origin`.
8. Scaffold the Tauri 2 + Svelte 5 project if it does not already exist.
9. Create the initial architecture and design-token foundation.
10. Begin milestone `0.1.0`.

Do not ask the user to restate requirements already present in these files.

Proceed autonomously until:
- a real blocker requires user input
- a consequential product decision is not specified
- an external permission/connection must be approved

Otherwise continue implementing the project milestone by milestone.

---

# 124. Definition of done

The project is not done because:
- it builds
- it looks polished
- it can display a list of driver versions

The project is done when DrvMatch can reliably:

```text
inspect the machine
identify devices
understand installed drivers
find trusted candidates
reject incompatible candidates
rank suitable candidates
recommend changing or keeping the current driver
explain why
download from trusted sources
verify packages
install safely
record what changed
report restart requirements
roll back when supported
handle missing drivers
survive source failures
preserve history/settings
```

and when the final UI follows `DESIGN.MD` without generic AI/shadcn dashboard aesthetics.

That is the standard for DrvMatch.
