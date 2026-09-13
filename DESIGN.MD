# DESIGN.md — DrvMatch

## 1. Product character

DrvMatch is a Windows driver manager built around one idea:

> Recommend the right driver for this exact machine, not simply the newest driver.

The interface must make that idea visible before the user reads documentation.

DrvMatch should feel:
- deliberate
- calm
- technical when requested
- easy to understand at first glance
- recognisable as its own product

It must not feel like:
- Device Manager with nicer colors
- Ravyn with "Downloads" replaced by "Drivers"
- a SaaS dashboard
- a generated shadcn admin panel
- a driver-booster/scareware product
- an AI optimizer

The default screen is **not** the raw Windows PnP inventory.

The default screen answers:

> Does anything on this PC deserve my attention?

Raw inventory exists one click away for technical users.

---

## 2. Reference hierarchy

Reference files live under:

`design-refence/`

They are read-only inspiration. Never import them directly.

### 2.1 TMC — primary identity reference

TMC is the main reference for DrvMatch's **product personality**, not for literal visual copying.

Take from TMC:
- immediate hierarchy
- low cognitive load
- compact top tabs
- tactile rounded surfaces
- obvious current mode
- one strong primary action
- persistent operational footer
- settings that read like settings instead of a dashboard
- a warm/material character that remains recognisable without decoration

The lesson from TMC is:

> The user should remember the program after seeing one screenshot.

Do not copy:
- exact beige palette values
- old custom cursor system
- memory-specific layouts
- macOS traffic lights
- old Svelte implementation patterns

### 2.2 Ravyn — secondary technical reference

Ravyn remains valuable for the places where DrvMatch genuinely becomes dense:
- list/detail inspector behavior
- ranked candidate presentation
- technical metadata density
- keyboard selection patterns
- install/progress states
- history details

Ravyn is **not** the shell reference anymore.

Do not recreate:
- permanent left navigation
- raw list as the default product surface
- "database frontend" feeling

### 2.3 Rustify — polish reference

Use Rustify for:
- restrained material layering
- focus states
- overlays
- subtle motion
- interaction polish

Do not copy its music-player composition or sepia/media atmosphere.

### 2.4 TommyTweaker — limited reference

Use only for:
- concise list copy
- compact headings

Do not copy its card-grid/glass-dashboard language.

---

## 3. Canonical product hierarchy

The main navigation is:

```text
Drivers   History   Settings
```

It is presented as a compact **top tab strip**, below the Windows titlebar.

Do not reintroduce a permanent left sidebar.

### Drivers

Drivers has two internal modes:

```text
Review   All hardware
```

`Review` is the default.

### Review

Review shows, in order:
1. current PC identity and scan freshness;
2. missing/problem devices reported by Windows;
3. generic Microsoft-driver findings as review opportunities, not alarms;
4. hardware-family shortcuts for intentional exploration.

It must never imply that a generic Microsoft class driver is automatically wrong.

### All hardware

All hardware is the complete Windows inventory.

It must:
- be searchable;
- support status filtering;
- support hardware-category filtering;
- group devices by meaningful product-facing category;
- open the inspector only after explicit selection;
- never auto-select the first raw device after a scan.

The complete inventory is a technical tool, not the opening experience.

---

## 4. Window

Main window:

```text
1180 × 760 logical px
resizable: false
maximizable: false
```

The outer radius is approximately:

```text
12px
```

The window uses a custom Windows-style titlebar.

Controls at top-right:

```text
—   □   ×
```

Rules:
- minimize works;
- close works;
- square remains visually Windows-like but disabled while fixed-size;
- close uses familiar destructive Windows hover;
- never use Apple red/yellow/green traffic lights;
- draggable region must never overlap interactive controls.

---

## 5. Brand mark

DrvMatch uses a small "matching rails" mark rather than a generic letter tile.

Concept:

```text
[  ][  ]
   ─
```

Two opposing structures visually suggest matching two sides of a decision.

The mark:
- uses the accent color;
- is small;
- belongs in the titlebar/footer;
- is not a decorative illustration.

Do not turn it into a giant logo/hero.

---

## 6. Material and color

### 6.1 Identity

Light mode uses a restrained **warm mineral** surface family.

It should feel related to TMC's warmth without being a beige clone.

Suggested hierarchy:

```text
outer shell       warm mineral / acrylic
navigation        slightly lighter mineral
content           warm off-white
panel             soft warm translucent surface
strong panel      more opaque warm surface
control           strong panel tone
```

Dark mode uses:

```text
charcoal shell
ink navigation
soft graphite content
slightly raised graphite panels
```

Avoid pure #000 and pure #fff as dominant surfaces.

### 6.2 Accent

Windows accent color may be used by default.

Fallback accent should be a muted indigo/violet rather than generic web blue.

The app's identity must still work if Windows provides:
- blue
- purple
- green
- orange
- another valid accent

Therefore identity comes from hierarchy/material/layout, not accent alone.

### 6.3 Semantic colors

Use semantic colors only for real meaning:
- green = healthy/success
- amber = caution/review
- red = real problem/failure

A generic Microsoft driver is **not red**.

---

## 7. Acrylic

Acrylic remains enabled by default and can be disabled in Settings.

Acrylic belongs to:
- outer shell
- title/navigation material
- selected transient overlays

Main information panels should remain crisp.

When Acrylic is disabled:
- use equivalent solid warm/charcoal surfaces;
- do not alter layout;
- do not alter spacing;
- do not replace acrylic with gradients.

---

## 8. Typography

Primary UI font:

```text
Segoe UI Variable Text
```

Display variant for page titles:

```text
Segoe UI Variable Display
```

Technical mono:

```text
Cascadia Code
Consolas
```

Suggested scale:

```text
Page title        21–23px / 650
Panel title       12–14px / 650
Body              11.5–13px
Caption           9.5–11px
Technical mono    10–12px
```

The UI should feel compact, not tiny.

---

## 9. Spacing and radius

Use a 4px rhythm.

Common spacing:

```text
4
8
12
16
20
24
```

Purposeful radii:

```text
controls      8px
layers        10px
panels        12px
window        12px
```

Rounded surfaces are part of the identity, inspired by TMC.

Do not turn every text fragment into a pill.

---

## 10. Titlebar and primary navigation

The titlebar is approximately 39–40px.

Below it is a 48px primary navigation strip.

Tabs:

```text
Drivers   History   Settings
```

Active tab:
- slightly raised/stronger surface;
- thin accent line;
- normal Windows-scale icon;
- no giant active capsule.

At the far right, show small machine context when useful:

```text
● MSI MS-7D14
```

This is context, not a status badge wall.

---

## 11. Drivers header

The page header is compact.

Example:

```text
DRIVER WORKBENCH
Drivers                              Inventory ready
MSI MS-7D14                          Last scan 14:31
```

No giant hero.

No health percentage.

No marketing text.

---

## 12. Review / All hardware switch

Use a two-part tactile mode control inspired by TMC's profile selector.

Example:

```text
┌──────────────────────────┬──────────────────────────┐
│ Review                   │ All hardware             │
│ Start here               │ 217 devices              │
└──────────────────────────┴──────────────────────────┘
```

This is a mode selector, not a pair of dashboard metric cards.

Selected mode:
- stronger background;
- subtle accent edge;
- no hover lift.

---

## 13. Review mode

### 13.1 Machine board

The first surface should communicate the state of the machine without inventing a score.

Example:

```text
This PC
MSI MS-7D14
24 hardware classes · 217 devices

✓ No hardware problems found
  Windows reports no missing/problem devices.

Missing 0   Problems 0   Generic 19
```

Important:
- `Generic` is informational;
- `Missing` / `Problems` are actual local findings;
- no "92% healthy" metric.

### 13.2 What deserves a look

Only missing/problem devices appear individually by default.

If none exist:

```text
✓ No missing or problem devices
  Windows reports the local hardware inventory as present and working.
```

Generic drivers are shown as one aggregate row:

```text
G   19 generic Microsoft drivers
    Often normal for class devices. Review only when a more specific
    package may add value.                                      ›
```

This prevents audio endpoints, monitors, filters, and virtual devices from flooding the opening screen.

### 13.3 Hardware map

Show product-facing hardware categories in one compact panel.

Examples:
- Display
- Network
- Audio
- Bluetooth
- Storage
- Input
- System
- USB
- Camera
- Other

The map is for navigation, not decoration.

Use one neutral icon language and one color family.

Do not rainbow-code categories.

---

## 14. Hardware categories

Category assignment is presentation-only.

It must never alter compatibility/ranking.

The category classifier may use:
- Windows class name
- friendly name
- description

Examples:

```text
Display / Monitor                  → Display
Net / Ethernet / Wi-Fi            → Network
AudioEndpoint / Media / Sound     → Audio
Bluetooth                         → Bluetooth
Disk / Storage / NVMe / SCSI      → Storage
Keyboard / Mouse / HID            → Input
System / Processor / Computer     → System
USB                               → USB
Camera / Image                    → Camera
fallback                          → Other
```

Keep classifier tests because friendly-name substrings can be misleading.

---

## 15. All hardware mode

All hardware contains:
- Back to Review action;
- search;
- category filter;
- status filter;
- grouped device list;
- optional right-side inspector.

Group headers are sticky.

Device row:

```text
[icon] Realtek PCIe 2.5GbE         Current        10.73.815.2024
       Realtek · Net                              Realtek
```

Selected row:
- accent left edge;
- subtle tinted background;
- no shadow;
- no lift.

No automatic hover tooltip.

---

## 16. Inspector

The right inspector is intentionally dense.

It appears only after explicit device selection.

Target width:

```text
~372px
```

Tabs:

```text
Overview   Candidates   Technical
```

Tabs use a small segmented-control treatment rather than Ravyn's long underline-only tab bar.

### Overview

Show:
- local condition;
- identity;
- installed driver;
- recommendation if evaluated;
- direct action to evaluate trusted sources if not evaluated.

Do not tell the user to hunt for another tab just to start the core action.

Good:

```text
Not evaluated yet
Local inventory says the device is working.
[Check this device]
```

### Generic driver

Good:

```text
Generic Microsoft driver
This may be normal. DrvMatch can compare trusted Microsoft, vendor,
and applicable OEM packages before suggesting a change.
[Compare trusted drivers]
```

Do not label generic as outdated/bad.

### Candidates

Preserve Ravyn-level density because this is where comparison matters.

Show:
- source
- compatibility
- version/package version
- channel
- date
- package
- evidence
- ranking state
- alternate provenance

### Technical

Show:
- instance ID
- class GUID
- hardware identity
- machine applicability
- INF/store metadata
- signer
- exact IDs when enabled
- DriverRank factors when enabled

---

## 17. Recommendation language

Only DriverRank may create:

```text
Recommended
Optional
Current
Missing
Not recommended
```

Local inventory findings are not recommendations.

Examples:

Correct:

```text
19 generic Microsoft drivers
```

Incorrect:

```text
19 recommended updates
```

until those devices have actually been evaluated against sources.

---

## 18. Correct local-condition wording

A device may be working without exposing a standalone installed package.

Therefore never always say:

> Windows reports an installed driver.

Use:

```text
Windows reports no device problem. This device does not expose a standalone
installed driver package.
```

when appropriate.

This is particularly important for software/virtual/filter/class devices.

---

## 19. Persistent footer

The footer is one of the strongest TMC-derived ideas.

It is a rounded operational surface inside the shell.

Idle on Drivers:

```text
[ Scan again ]   ──────────────   ● Ready   217 present devices   Scanned 14:31
```

During installation:

```text
● Installing Realtek Ethernet   █████████░░   2 of 3   [Cancel]
```

Rules:
- remains visible across long operations;
- progress is real, never decorative fake percentage;
- scan action may live here while on Drivers;
- other pages may show product/status context instead.

---

## 20. Settings

Settings is a full page.

The secondary settings sidebar is removed.

Use compact top category tabs:

```text
Appearance   Sources   Safety & Rollback   Advanced   About
```

Below them is one clear settings sheet.

This is intentionally closer to TMC than Ravyn.

Settings rows:
- label + explanation on left;
- control on right;
- subtle separators;
- no card per setting;
- no tooltip dependency.

Unsaved changes use a floating/sticky save surface near the bottom:

```text
Unsaved changes                          [Discard] [Save changes]
```

---

## 21. History

History may remain list/detail because this information is naturally dense.

Use:
- driver changes list;
- stored scan rows;
- selected change details;
- rollback only where real.

History should inherit the warm/mineral shell instead of looking like a separate Ravyn page.

---

## 22. Footer and progress truthfulness

Never fake progress.

If work is indeterminate:
- use an indeterminate treatment;
- show current phase text.

If progress is known:
- show actual fraction/percentage.

Do not use decorative constantly-moving progress bars when idle.

An idle footer rail may be a neutral material separator but must not be described as progress.

---

## 23. Controls

Default height:

```text
30–32px
```

Primary action:
- accent fill;
- high contrast;
- only one obvious primary action per local task.

Secondary actions:
- neutral surface;
- subtle border.

Text actions:
- accent text;
- no large button chrome when unnecessary.

Hover:
- background/border/color change only;
- never translate/lift.

---

## 24. Icons

Use simple 16–18px line icons.

Hardware-category icons should be meaningful but neutral.

Do not:
- put every icon inside a colored circle;
- assign random category colors;
- use emojis in production UI;
- use decorative illustrations.

Small neutral rounded icon containers are allowed when they improve scanning.

---

## 25. No hover-name tooltips

Strict rule.

Do not:
- add HTML `title` attributes to ordinary controls;
- wrap icon buttons in automatic tooltips just to repeat their name;
- show delayed hover labels.

Use:
- visible text where possible;
- `aria-label` for accessibility;
- explicit detail/copy actions for long technical values.

---

## 26. Accessibility

Required:
- visible focus;
- proper ARIA labels;
- keyboard-operable tabs;
- keyboard-operable device list;
- non-color status communication;
- reduced-motion support;
- forced-colors support where practical;
- focus restoration for dialogs.

Interactive behavior should use semantic buttons/inputs instead of clickable divs.

Svelte 5 accessibility/compiler warnings should remain clean.

---

## 27. Motion

Use motion only to explain state.

Target durations:

```text
fast      120–160ms
normal    180–220ms
```

Allowed:
- panel transition
- menu/dialog appearance
- progress transition
- subtle state change

Avoid:
- bounce
- hover lift
- card scaling
- animated gradients
- ambient pulsing during normal idle state.

---

## 28. Empty states

Good:

```text
No missing or problem devices
Windows reports the local hardware inventory as present and working.
```

Good:

```text
No candidates returned
The installed driver can remain Current when sources have no suitable alternative.
```

Bad:

```text
✨ Amazing! Your PC is perfect!
```

No confetti, illustrations, or marketing tone.

---

## 29. Error states

Errors should identify:
- what failed;
- what remains usable;
- what the user can do.

Example:

```text
Microsoft Update Catalog unavailable
Vendor and Windows Update results remain available.
[Retry]
```

Do not convert every source outage into a global red app state.

---

## 30. Installation review

The install review remains modal because it is consequential.

Show:
- device
- previous version
- candidate version
- source/channel
- package type
- signature/hash behavior
- restore-point behavior
- package backup behavior

The modal must remain visually part of the same material system.

---

## 31. Anti-clone checklist

Before accepting a UI change, ask:

- Does this make the default screen more useful than raw Device Manager?
- Does the default screen show decisions before inventory?
- Would this still be recognisable without the DrvMatch wordmark?
- Are we copying Ravyn structure just because it already exists?
- Are generic class drivers being framed carefully?
- Is technical density confined to places where it helps?
- Did we add a card because it communicates hierarchy, or because cards are easy?
- Is there an obvious next action?
- Did we accidentally create a health score?
- Did we add a tooltip the user explicitly dislikes?

If the answer indicates "Ravyn Drivers Edition", redesign it.

---

## 32. Canonical visual direction

In one sentence:

> DrvMatch should have TMC's personality and immediacy, Ravyn's technical competence only where density is required, and Rustify's material polish.

In product terms:

> Open DrvMatch → understand the PC state → review only what matters → inspect all hardware only when you choose to.

That hierarchy is part of the product, not merely styling.
