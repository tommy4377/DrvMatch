# DESIGN.md — DrvMatch

## 1. Product character

DrvMatch is a Windows driver manager built around one idea:

> Recommend the right driver, not simply the newest driver.

The interface must communicate precision, transparency and control.

It should feel like a serious Windows desktop utility that happens to be modern,
not like:
- a website wrapped in Tauri
- a SaaS dashboard
- an "AI PC optimizer"
- a driver-booster/scareware product

The UI is information-first.

---

## 2. Design references

Reference files live under:

`design-refence/`

They are inspiration and implementation references only. They are not application
source files.

### Ravyn — primary structural reference

Use Ravyn as the main reference for:
- app shell
- sidebar/navigation
- list density
- command bars
- list selection
- details pane
- status presentation
- settings navigation
- update state presentation
- subtle borders
- desktop-oriented controls

Especially relevant files:

```text
ravyn/
  tokens.css
  AppShell.svelte
  NavigationView.svelte
  StatusBar.svelte
  PageScaffold.svelte
  PageCommandBar.svelte
  ListDetailsLayout.svelte
  DetailsPane.svelte
  DownloadsView.svelte
  JobRow.svelte
  JobDetailsPane.svelte
  SettingsNavigation.svelte
  SettingsView.svelte
  UpdateSettings.svelte
```

DrvMatch should feel related to Ravyn in architectural maturity and density, but
must not look like a reskin of Ravyn.

### TMC — persistent operation state

Use TMC mainly for:
- simple settings rows
- persistent progress/action footer
- direct state communication

Relevant files:

```text
tmc/
  BasicSettings.svelte
  FullView.svelte
  ProgressFooter.svelte
  tokens.css
```

Do not copy TMC's older visual language verbatim.

### Rustify — material discipline

Use Rustify for:
- intentionally limited material layers
- restrained elevation
- focus states
- overlay behavior
- motion polish

Relevant files:

```text
rustify/
  app.css
  App.svelte
  FriendsPanel.svelte
  Settings.svelte
```

Do not copy:
- sepia palette
- media-player atmosphere
- ambient blobs
- media-oriented visual hierarchy

Take the discipline, not the theme.

### TommyTweaker — limited reference

Use only for:
- compact list information structure
- straightforward section headers

Relevant files:

```text
tommytweaker/
  TweakList.svelte
  SectionHeader.svelte
```

Do not copy TommyTweaker's global visual language.

In particular avoid:
- feature-card grids
- heavy glass cards
- large hover shadows
- hover lift
- icon-in-a-circle cards
- dashboard composition

---

## 3. Window

### 3.1 Fixed-size window

The main DrvMatch window is intentionally non-resizable.

Requirements:

- `resizable: false`
- no edge/corner resize affordances
- no drag-to-resize behavior
- no layout depending on arbitrary viewport widths

Initial target size:

```text
1180 × 760 logical px
```

This can be adjusted once during the initial shell implementation if the real UI
needs a small correction, but after the shell is approved it should remain fixed.

High-DPI scaling must still work correctly.

### 3.2 Rounded outer border

The application window should have visibly rounded outer corners.

Target visual radius:

```text
12px
```

The radius should be subtle and Windows-like, not a giant mobile-app radius.

Do not fake the window shape using a large rounded card floating inside a square
transparent window if a proper supported Windows/Tauri solution is available.

### 3.3 Titlebar

Use a custom titlebar that visually follows Windows conventions.

Top-right controls:

```text
—    □    ×
```

Order:
1. minimize
2. maximize-style square
3. close

Never use Apple-style red/yellow/green traffic-light controls.

The close button should use the familiar Windows destructive hover treatment.
Minimize and middle-square controls should use restrained neutral hover states.

Because the app is non-resizable, the maximize/restore action is disabled by
default. The visual control may remain to preserve the Windows control cluster,
but it must not enable resizing/maximizing behind the user's back.

The draggable titlebar region must not overlap buttons or interactive controls.

---

## 4. Acrylic

Acrylic is the default shell material.

Requirements:

- enabled by default
- user can disable it in Settings
- state persists
- disabling it changes material only, not layout
- solid fallback must remain fully readable

Acrylic belongs primarily to:
- outer shell
- navigation/titlebar layer
- selected transient overlays where appropriate

Acrylic does NOT mean every panel becomes a glass card.

Main content should use controlled translucent/solid surfaces so lists and text
remain crisp.

### 4.1 Acrylic off

When disabled:

- use a solid Windows-like background
- preserve the same borders, spacing, typography and hierarchy
- do not replace acrylic with gradients
- do not alter component dimensions

### 4.2 Material hierarchy

Use at most a few clearly defined layers:

```text
Window backdrop / acrylic
Navigation layer
Content layer
Transient overlay/flyout layer
```

Do not create a unique glass/elevation treatment for every component.

---

## 5. No hover-name popups

DrvMatch must not display automatic name popups/tooltips when the mouse hovers over
normal controls.

This is a strict design preference.

Do not:
- add `title="..."` to normal buttons/icons
- wrap every icon button in a tooltip
- show the control name after hovering
- use tooltips to repeat visible information

Accessible icon buttons still require non-visual accessible names through:
- `aria-label`
- `aria-labelledby`

If text is truncated, the full value should be available through:
- the details pane
- an explicit "copy" action
- an explicit details/technical view

not through an automatic hover tooltip.

---

## 6. Overall shell

Main structure:

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ app icon  DrvMatch                                         —   □   ×   │
├──────────────┬───────────────────────────────────────────────────────────┤
│              │                                                           │
│ Drivers      │ page                                                       │
│ History      │                                                           │
│              │                                                           │
│              │                                                           │
│              │                                                           │
│ Settings     │                                                           │
│              │                                                           │
├──────────────┴───────────────────────────────────────────────────────────┤
│ status / selection / install progress                                    │
└──────────────────────────────────────────────────────────────────────────┘
```

Primary navigation:

```text
Drivers
History
Settings
```

No separate dashboard/home page.

The default route is **Drivers**.

The application should open directly into useful information.

---

## 7. Sidebar

Target width:

```text
184px
```

Use:
- text label
- simple 16–18px icon
- subtle active background
- thin accent indicator
- Settings anchored near the bottom

Do not use:
- large category icons
- icon bubbles
- badges unless they communicate a real state
- 15+ navigation categories

The main sidebar should remain visually quiet.

---

## 8. Typography

Primary UI font:

```text
Segoe UI Variable
```

Fallback:

```text
"Segoe UI", system-ui, sans-serif
```

Technical monospace:

```text
Cascadia Code
```

Fallback:

```text
Consolas, ui-monospace, monospace
```

Use monospace only where it adds meaning:

- Hardware IDs
- Compatible IDs
- INF names
- version strings when aligned in technical tables
- hashes
- package identifiers
- raw metadata

Do not use a display font for page titles.

Suggested scale:

```text
Page title        22px / 600
Section title     15px / 600
Body              13px / 400
Strong body       13px / 600
Caption           11.5–12px
Technical mono    11.5–12px
```

Use tabular numerals for versions, dates and aligned numeric information where
supported.

---

## 9. Spacing and sizing

Use a 4px base rhythm.

Core spacing:

```text
4
8
12
16
20
24
32
```

Controls:

```text
small control     28px
default control   32px
large control     40px
```

Driver row:

```text
52px
```

Status bar:

```text
28px normal
```

Selection/install action bar:

```text
48–52px
```

Default corner radii:

```text
small             4px
control           6px
surface           8px
window            12px
pill              only when semantically appropriate
```

Do not use 16–24px radii by default.

---

## 10. Color

Prefer the Windows accent color when it can be obtained reliably.

Fallback accent:

```text
Windows-like blue
```

There should be one main accent.

Semantic colors are reserved for real states:

- success
- warning
- error
- informational

Do not make each hardware category a different color.

Do not use rainbow status chips.

Do not use gradients for headings or normal buttons.

---

## 11. Borders, shadows and elevation

Borders do most of the structural work.

Prefer:
- 1px subtle dividers
- slight tone changes
- restrained background layers

Use shadows mainly for:
- dialogs
- menus
- floating overlays
- transient drawers

Normal rows, settings groups and content areas should not look like floating cards.

No hover lift:

```text
transform: translateY(...)
```

on ordinary rows/cards/buttons.

Hover should usually mean:
- subtle background change
- subtle border change
- icon/text color change

not physical movement.

---

## 12. Drivers page

Drivers is the main product surface.

Recommended structure:

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Drivers                                         Scan again               │
│ Last scan 2 min ago · 32 devices                                        │
├──────────────────────────────────────────────────────────────────────────┤
│ Recommended 3   Optional 2   Current 27   Missing 0                      │
│                                                                          │
│ Search devices...                              Filter   Sort             │
├───────────────────────────────────────────────┬──────────────────────────┤
│ DEVICE LIST                                   │ DETAILS                  │
│                                               │                          │
│ Radeon RX 7600                   Current      │ Realtek PCIe 2.5GbE      │
│ AMD · Display                                 │                          │
│                                               │ Recommended              │
│ Realtek PCIe 2.5GbE              Update       │ 11.26.520.2026           │
│ 10.73 → 11.26                                 │                          │
│                                               │ Installed                │
│ AMD SMBus                        Current      │ 10.73.815.2024           │
│                                               │                          │
│ MediaTek Bluetooth               Optional     │ Why this driver          │
│                                               │ ✓ Exact Hardware ID      │
│                                               │ ✓ Exact subsystem        │
│                                               │ ✓ WHQL                   │
│                                               │ ✓ Trusted source         │
│                                               │                          │
│                                               │ [View candidates]        │
│                                               │              [Install]   │
└───────────────────────────────────────────────┴──────────────────────────┘
```

The list is the product.

Do not replace it with a dashboard of summary cards.

---

## 13. Driver list

The row should communicate enough to scan quickly.

Recommended row anatomy:

```text
[device icon] Device name
              vendor · device class

Installed → recommended        Source        Status/action
```

Keep the device icon neutral.

Primary emphasis:
1. device name
2. recommendation status
3. installed/recommended version relation

Secondary:
- vendor
- class
- source
- branch/signing shorthand

Use separators between rows rather than a separate rounded card for every device.

Selected row:
- subtle accent-tinted background
- thin accent indicator at the left
- no large glow

Hover:
- subtle background only
- no tooltip
- no lift

---

## 14. Driver states

Main states:

### Recommended

A different driver is meaningfully preferable.

Examples:
- better exact match
- stable/recommended branch
- important fix
- correct OEM package
- security-related update

### Optional

A valid alternative/update exists, but there is no strong reason to change.

### Current

The installed driver remains the best recommendation.

Important:

A newer candidate may exist while the installed driver is still **Current**.

### Missing

No suitable driver is installed for a detected device.

### Not recommended

A candidate exists but DrvMatch advises against installing it.

The UI must never reduce this to old/new version comparison.

---

## 15. Page header and command bar

Page header should be compact.

Example:

```text
Drivers                                      [Scan again]
Last scan 2 min ago · 32 devices
```

No giant title.

Under it, use a command/filter area:

```text
Recommended 3  Optional 2  Current 27  Missing 0

[ Search devices... ]                     [Filter] [Sort]
```

These are navigation/filter states, not metric cards.

When items are selected, the command bar can enter selection mode:

```text
3 selected                         [Clear] [Install selected] […]
```

---

## 16. Details pane

Open the details pane by selecting/opening a driver/device.

Target width:

```text
380px
```

The main fixed window should be designed so the list remains usable while this pane
is open.

Tabs:

```text
Overview
Candidates
Technical
```

### Overview

Show:
- device identity
- installed driver
- recommended driver
- source
- signing/WHQL state
- recommendation label
- short explanation
- install/rollback action when relevant

### Why this driver?

This is a first-class section.

Example:

```text
Why this driver

✓ Exact Hardware ID
✓ Exact subsystem
✓ Windows 11 x64 compatible
✓ Microsoft-signed
✓ WHQL
✓ Stable/recommended branch
```

If a factor is negative, explain it plainly.

Do not show an unexplained `94/100` as the main trust signal.

### Candidates

Show all meaningful compatible alternatives.

Example:

```text
★ Recommended
11.26.520.2026
Microsoft Update Catalog
Exact HW ID · Exact SUBSYS · WHQL

11.28.610.2026
Realtek
Exact VEN/DEV · Generic SUBSYS · WHQL

Newer, but less specific for this machine.
```

Candidates should explain why one ranks above another.

### Technical

Show:
- Hardware IDs
- Compatible IDs
- Class GUID
- provider
- INF
- signer
- package identity
- driver date
- raw version
- architecture
- source metadata
- internal rank/score if useful
- scoring factor breakdown

This is where technical density belongs.

---

## 17. Ranking presentation

DrvMatch may internally calculate a score.

Normal users should see semantic language:

```text
Recommended
Good alternative
Optional
Not recommended
```

Advanced/Technical may show:

```text
Internal score: 94
```

but never pretend that a driver recommendation can be summarized perfectly by a
single number.

"Why" matters more than "94".

---

## 18. Source presentation

Source is first-class information.

Examples:

```text
AMD
NVIDIA
Intel
ASUS
Dell
Lenovo
HP
Windows Update
Microsoft Update Catalog
```

Show source in the list/details without making it a giant badge.

When source hierarchy matters, explain it.

Example:

```text
Current OEM package recommended

A newer generic Realtek package exists, but the installed ASUS package contains
an exact subsystem match and OEM-specific extensions.
```

This kind of explanation is a core product feature.

---

## 19. Install flow

Installing drivers must feel controlled.

### Pre-install review

Before executing a group install:

```text
Ready to install

AMD Chipset
Realtek Ethernet
MediaTek Bluetooth

Safety
✓ Current packages will be recorded
✓ Restore/checkpoint will be created when available
✓ Rollback information will be preserved

Restart required after AMD Chipset

[Cancel]                          [Install 3 drivers]
```

Avoid fear language.

### During installation

Use the persistent bottom operation bar.

Example:

```text
Installing Realtek Ethernet     ████████░░░ 72%     2 of 3
```

The user should not need to keep a modal open to understand progress.

A detailed progress dialog/pane may exist, but the bottom bar remains the global
state indicator.

### After installation

Show:
- success/failure
- reboot requirement
- rollback availability
- installed version

---

## 20. Bottom status/action bar

Normal state:

```text
32 devices · Last scan 2 min ago                        Ready
```

When items are selected:

```text
3 drivers selected                         [Install selected]
```

During work:

```text
Installing AMD Chipset     ██████████░░ 81%      1 of 3
```

After completion:

```text
3 drivers installed · Restart required                  [Restart later]
```

This pattern is inspired by TMC's persistent operation footer but should visually
match DrvMatch/Ravyn-style density.

---

## 21. History page

History should be useful for rollback and diagnosis, not decorative.

Use a compact chronological list/table.

Example:

```text
Today

18:42  Realtek PCIe 2.5GbE
       10.73 → 11.26
       Installed successfully                    [Rollback]

September 8

21:12  AMD Radeon RX 7600
       26.8.1 → 26.9.1
       Rolled back
```

Show:
- device
- old/new version
- source
- timestamp
- install result
- restart event if relevant
- rollback availability

Do not turn each event into a huge card.

---

## 22. Settings

Settings is a real page, not a modal.

Suggested internal categories:

```text
General
Appearance
Sources
Safety & Rollback
Advanced
About
```

Use a secondary settings navigation similar to Ravyn.

### Appearance

Include:

```text
Theme
- System
- Light
- Dark

Use acrylic
- On by default

Use Windows accent color
- On by default

Reduce motion
- Follow system by default
```

Acrylic toggle must apply cleanly without requiring a redesign.

### Sources

Allow visibility/control for supported source families when implemented.

Example:

```text
Windows Update
Microsoft Update Catalog
AMD
NVIDIA
Intel
OEM sources
```

Do not let users accidentally disable all safe sources without clear feedback.

### Safety & Rollback

Potential settings:

```text
Create restore point before driver installation
Back up current package when possible
Require confirmation for optional drivers
Automatically offer rollback after a failed install
```

Do not default to reckless automation.

### Advanced

Technical options belong here:
- show exact IDs
- expose internal ranking score
- logging verbosity
- cache/index controls
- experimental source adapters

---

## 23. Dialogs and transient surfaces

Dialogs should be used for:
- confirmation
- destructive operations
- install review
- explicit errors requiring a decision

Do not use modal dialogs for normal navigation.

Menus/popovers:
- compact
- acrylic/overlay material allowed
- subtle shadow
- 6–8px radius
- keyboard navigable

No tooltip-on-every-icon behavior.

---

## 24. Empty, loading and error states

### Empty

Plain and useful.

Good:

```text
No recommended updates

32 devices checked
Last scan: 00:51
```

Bad:

```text
✨ You're all caught up!
Your PC is running beautifully!
```

No decorative illustration is required.

### Loading

Use:
- skeleton rows
- compact progress
- real status text

Avoid giant centered spinners when the rest of the shell can remain usable.

### Error

Explain:
- what failed
- what is still usable
- what action the user can take

Example:

```text
Microsoft Update Catalog could not be reached.

Windows Update and vendor sources are still available.

[Retry]
```

---

## 25. Copy style

Tone:
- calm
- technical when needed
- concise
- non-alarmist

Preferred wording:

```text
Recommended update
Optional update
Current driver is recommended
Driver missing
Restart required
Exact device match
OEM package
Microsoft-signed
WHQL
View candidates
Why this driver?
```

Avoid:

```text
CRITICAL
AT RISK
FIX NOW
BOOST
OPTIMIZE PC
DRIVERS OUTDATED
HEALTH SCORE
AI POWERED
```

unless a future feature genuinely requires the term and it is technically
accurate.

---

## 26. Iconography

Prefer Lucide-style line icons unless a native Windows glyph is clearly better.

Typical size:

```text
16–18px
```

Do not:
- place every icon inside a colored circle
- use emojis as product icons
- assign decorative icon colors by category

Titlebar controls should use Windows-like glyphs rather than Lucide-looking
generic window controls if a native-looking implementation is available.

---

## 27. Motion

Motion is functional, not decorative.

Target durations:

```text
fast      120–160ms
normal    180–220ms
```

Allowed:
- pane opening
- menu/flyout appearance
- progress transitions
- subtle state transitions

Avoid:
- card lift
- bounce
- large scale effects
- animated gradient
- constantly pulsing normal states

Respect reduced motion.

---

## 28. Accessibility

Required:
- visible keyboard focus
- non-color status communication
- semantic labels
- correct ARIA
- keyboard-operable menus/dialogs/tabs
- adequate contrast
- reduced-motion support
- high-contrast/forced-colors support where practical

Because hover tooltips are disabled, icon-only controls must still have proper
accessible names through ARIA.

Do not sacrifice accessibility to avoid tooltips.

---

## 29. Anti-slop checklist

Before approving a new screen, ask:

- Is the main information shown as a list/table where appropriate?
- Did we create cards only because they were easy?
- Is there a giant heading that adds no value?
- Are there too many pills?
- Are icons decorative instead of informative?
- Is glass being used as decoration instead of hierarchy?
- Does anything lift on hover?
- Is there a gradient where a solid color would work?
- Is a status shown as a fake score?
- Is there marketing copy inside a system tool?
- Did we add an automatic hover tooltip?
- Would this still look like a Windows utility if the accent color changed?

If several answers are bad, simplify the screen.

---

## 30. Canonical visual direction

In one sentence:

> DrvMatch should combine Ravyn's desktop structure and density, TMC's persistent
> operation feedback, Rustify's material discipline, and none of TommyTweaker's
> dashboard-card excess.

And in one more:

> It should look like a thoughtfully designed Windows utility made by someone who
> uses it, not a generated dashboard trying to look impressive.
