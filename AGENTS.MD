# AGENTS.md — DrvMatch

## 1. Project intent

DrvMatch is a Windows desktop driver manager whose main differentiator is not
"find the newest driver", but "find the best driver for this exact machine".

The product must be conservative, transparent, explainable and trustworthy.
It must never behave like alarmist "driver booster" software.

Core product language:

- Recommended
- Optional
- Current
- Missing
- Not recommended

Do not call a driver "outdated" only because a numerically newer version exists.

Every recommendation must be explainable in terms of hardware match, OEM/device
specificity, OS compatibility, source trust, signing/WHQL status, release branch,
known stability information and other relevant signals.

---

## 2. Read this before making changes

Before implementing or modifying UI:

1. Read `DESIGN.MD`.
2. Inspect the relevant files under `design-refence/`.
3. Identify which reference is relevant to the task.
4. Implement the new DrvMatch component from the design rules, not by cloning an
   old application wholesale.

The reference directory is intentionally named:

`design-refence/`

Do not rename it unless the user explicitly asks.

### Reference hierarchy

**TMC — primary identity/reference**
- product personality and recognizability
- immediate, low-cognitive-load hierarchy
- top-level tab treatment
- tactile rounded surfaces without dashboard-card excess
- straightforward settings rows
- persistent operation/progress footer
- warm/mineral light-surface character

DrvMatch must not copy TMC literally, but it should inherit TMC's strongest trait:
when the window opens, the user should know where to look and what the program is for.

**Ravyn — secondary structural reference for dense/technical surfaces**
- list/detail inspector behavior
- candidate/technical information density
- keyboard selection patterns
- status presentation
- update/install state handling

Ravyn is no longer the source of DrvMatch's overall shell or personality. In particular,
do not recreate Ravyn's permanent left navigation + full raw-list default composition.

**Rustify — secondary polish reference**
- disciplined material hierarchy
- focus states
- overlays
- motion restraint
- interaction polish

**TommyTweaker — limited reference**
- list information layout
- compact section headings

Do NOT derive the overall DrvMatch visual language from TommyTweaker.

---

## 3. Reference files are read-only design material

Nothing inside `design-refence/` is application source code.

Never import from it.

Bad:

```ts
import Button from "../../design-refence/ravyn/Button.svelte";
```

Good:

- inspect the reference
- understand the behavior/design decision
- implement the DrvMatch equivalent under `src/`

If a reference component is worth reusing, copy/rewrite it into the real source
tree and adapt it to DrvMatch. After that, the new DrvMatch version becomes the
source of truth.

Old reference files may contain Svelte 4-era patterns, old APIs or project-specific
business logic. They are examples, not compatibility requirements.

---

## 4. Technology direction

Primary stack:

- Tauri 2
- Rust for native/system/backend functionality
- Svelte 5
- TypeScript
- CSS variables/design tokens
- Windows-first desktop UI

Prefer modern Svelte 5 patterns in new code:

- `$state`
- `$derived`
- `$effect`
- snippets
- typed props

Do not copy older Svelte patterns from references merely because they exist there.

Use strict, explicit types for domain objects. Avoid `any` except at unavoidable
interop boundaries.

Keep system/native behavior out of presentation components.

The UI should consume typed domain models; it should not contain driver matching,
driver installation or Windows-device-discovery logic.

---

## 5. MCP/tool usage

The intended coding environment includes:

- Serena
- Context7
- Tauri MCP
- shadcn MCP
- Git/GitHub MCP

Use them deliberately.

### Serena

Use Serena first when understanding an existing code path, symbol, component or
dependency graph.

Prefer semantic navigation over broad text dumping.

Before a cross-cutting refactor:
- inspect affected symbols
- inspect callers/importers
- understand state flow
- make the smallest coherent change

### Context7

Use Context7 for current documentation when working with:
- Svelte 5
- Tauri 2
- Rust crates
- Bits UI / shadcn-svelte
- Windows-facing libraries
- other third-party dependencies

Do not rely on remembered APIs when a current reference is available.

### Tauri MCP

Use Tauri-specific tooling/documentation for:
- window configuration
- native titlebar behavior
- acrylic/backdrop implementation
- Windows window effects
- permissions/capabilities
- native dialogs
- tray integration
- updater behavior

Do not invent native APIs.

### shadcn MCP

shadcn is a source of interaction primitives, not DrvMatch's design language.

It may be used for things such as:
- dialog
- alert dialog
- dropdown/menu
- popover
- select
- checkbox
- switch
- tabs
- context menu
- scroll area
- command primitives

Rules:
- prefer Svelte-compatible primitives
- inspect before adding
- install only what is needed
- restyle to DrvMatch tokens
- do not import the default shadcn visual language
- do not create a card dashboard because shadcn makes cards easy

Never respond to "build this page" by installing a large collection of components
without first checking the existing DrvMatch component library.

### Git/GitHub

Use Git to understand history and review diffs.

Do not:
- rewrite history
- force push
- discard user changes
- reset unrelated work
- perform destructive repository operations

unless explicitly requested.

Prefer small, coherent changes.

---

## 6. UI design rules

`DESIGN.MD` is authoritative for visual decisions.

High-level rules:

- Windows desktop utility, not a web/SaaS dashboard
- **Review-first:** the default Drivers surface answers "do I need to do anything?" before exposing raw inventory
- **Raw hardware is secondary:** All hardware remains one click away for technical users, grouped by meaningful device family
- TMC-like clarity/personality, not a literal TMC clone
- Ravyn-level density only in the details/candidates/technical inspector where it earns its keep
- primary app navigation uses the compact top tab strip; do not reintroduce the old permanent left sidebar
- compact but readable
- custom design system with warm/mineral surfaces in light mode and restrained charcoal surfaces in dark mode
- Windows accent may color actions, but layout/material identity must survive any accent color
- subtle borders, 8–12px purposeful rounding, restrained shadows
- one obvious primary action at a time
- no visual noise
- no unnecessary glass cards
- no giant statistic cards
- no hero marketing sections
- no gradient headings
- no "PC health" score
- no gamified warnings
- no decorative illustrations
- no "27 DRIVERS OUTDATED!!!" language
- no hover-lift animation on normal controls
- no Apple traffic-light window controls
- never dump every PnP device on the default screen merely because it exists

### Drivers information hierarchy

The default Drivers mode is `Review`.

It should show, in order:
1. identity of the current machine and scan freshness;
2. whether Windows reports missing/problem devices;
3. generic Microsoft-driver findings as a **review opportunity**, never automatically as an update/problem;
4. a small hardware-category map for deliberate exploration.

`All hardware` is the secondary mode. It may show the complete Windows inventory, but must:
- group devices by product-facing hardware categories;
- keep search/status/category filters visible;
- open the dense details inspector only after explicit selection;
- never auto-select the first raw device after a scan.

A candidate/recommendation is only "Recommended" after DriverRank has actually evaluated source evidence.
Local inventory findings must not impersonate a remote-source recommendation.

### No hover tooltips

Do not add hover tooltips that display the name of the control.

Do not use HTML `title` attributes on normal interactive controls.

Do not use tooltip components merely to repeat a visible label or icon name.

Accessible naming must use:
- visible text when possible
- `aria-label`
- `aria-labelledby`
- other non-hover accessibility mechanisms

For truncated technical values, expose the full value in the selected details pane
or through an explicit copy/details action rather than an automatic hover tooltip.

A tooltip is allowed only if the user later explicitly requests one for a specific
interaction.

---

## 7. Window behavior

DrvMatch uses a custom Windows-style titlebar.

Required behavior:

- fixed-size/non-resizable main window
- no edge/corner resize handles
- rounded outer window corners
- Windows control order at top-right:
  - minimize
  - maximize-style square
  - close
- never use macOS red/yellow/green traffic-light controls

Because the main window is intentionally non-resizable, the maximize/restore
control should remain visually Windows-like but be disabled/non-operative unless
the user later changes the non-resizable requirement.

Do not silently enable resizing to make layout work. Fix the layout instead.

Acrylic is enabled by default and can be disabled in Settings.

When acrylic is disabled, switch to a solid fallback surface without changing the
layout or information hierarchy.

Use a supported Tauri/Windows implementation for rounded corners and acrylic.
Research the current supported API before coding it.

---

## 8. Driver-domain safety rules

DrvMatch is allowed to be sophisticated, but never reckless.

The default recommendation engine must prioritize suitability over raw recency.

Do not:
- force-install a mismatched INF
- silently install unsigned drivers
- automatically replace a specific OEM driver with a generic driver only because
  the generic version number is newer
- hide the source of a driver
- hide why a driver was selected
- perform background driver installations without explicit product behavior and
  user consent
- label every newer candidate as recommended

Prefer:
- exact hardware ID match
- exact subsystem match
- exact OEM model match when relevant
- correct Windows version/build/architecture
- trusted/official source
- valid signature
- WHQL/recommended/stable branch where applicable
- OEM-specific packages for hardware that depends on OEM extensions
- transparent alternatives

Installation flows should support, where feasible:
- restore point/checkpoint
- current driver/package backup
- install history
- rollback
- clear reboot requirement state

Potentially destructive operations need explicit confirmation.

---

## 9. Presentation vs engine

The recommendation engine may have a numerical internal score.

The normal UI should not turn that into a fake precision metric.

Good:
- Recommended
- Good alternative
- Optional
- Not recommended

Technical/advanced view may show:
- score
- individual scoring factors
- exact IDs
- INF metadata
- provider
- signer
- driver date
- package identity

The normal UI should answer:

1. What device is this?
2. What is installed?
3. What does DrvMatch recommend?
4. Why?
5. Where does that driver come from?
6. What are the alternatives?
7. What happens if I install it?

---

## 10. Architecture expectations

Keep domain layers separate.

Suggested conceptual boundaries:

```text
UI / presentation
        ↓
application services
        ↓
driver/device domain
        ↓
source adapters + Windows integration
        ↓
native APIs / network / storage
```

Source adapters should be isolated from ranking logic.

Examples:

```text
WindowsUpdateSource
MicrosoftCatalogSource
AmdSource
NvidiaSource
IntelSource
OemSource
```

The ranking engine consumes normalized candidates; it should not contain scraping
or provider-specific networking logic.

The UI should not know how a source was scraped or queried.

---

## 11. Naming and copy

Current product name: **DrvMatch**

Write copy like a calm system tool.

Good:
- "3 recommended updates"
- "Current driver is recommended"
- "A newer driver exists, but the installed OEM package is a better match"
- "Restart required"
- "Exact hardware match"
- "Microsoft-signed"
- "Source: AMD"

Bad:
- "Your PC is at risk"
- "Critical driver health"
- "Fix all problems"
- "Boost performance now"
- "Outdated!"
- "PC health: 92%"
- "AI optimized driver"

Avoid unnecessary marketing language inside the application.

---

## 12. Interaction quality

Keyboard behavior matters.

New interactive components should have:
- visible focus
- correct ARIA roles
- predictable Tab order
- Enter/Space activation where appropriate
- Escape behavior for transient surfaces
- selection behavior consistent with desktop applications

Respect:
- `prefers-reduced-motion`
- Windows high contrast/forced colors where practical
- system scale/DPI

Do not add animation unless it explains a state transition.

Default motion should be short and subtle.

---

## 13. Performance

DrvMatch should feel like a native utility.

Avoid:
- unnecessary polling
- expensive UI rerenders
- large always-mounted hidden views
- animations on large lists
- loading every candidate's deep metadata before it is needed

Prefer:
- event-driven updates
- lazy loading of deep technical details
- virtualized lists only when justified by dataset size
- cached normalized metadata
- debounced search

Do not optimize prematurely, but do not introduce obviously wasteful behavior.

---

## 14. Before finishing a task

Before considering work complete:

1. Re-read the relevant section of `DESIGN.MD`.
2. Check the relevant files under `design-refence/`.
3. Confirm no reference files are imported.
4. Confirm no unwanted tooltip/title behavior was introduced.
5. Confirm the window constraint is respected.
6. Confirm light/dark and acrylic-off states remain coherent if the change affects
   shell/surfaces.
7. Confirm keyboard/focus behavior.
8. Run relevant formatting/typechecking/tests.
9. Review the diff for unrelated edits.
10. Explain any deliberate deviation from the design instead of silently changing
    the design system.

When uncertain about a design decision, preserve the existing DrvMatch design
system rather than introducing a new visual pattern.
