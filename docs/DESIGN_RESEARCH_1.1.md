# DrvMatch 1.1 — design research notes

The 1.1 UI pass was designed in Figma before implementation and then translated back into the existing Svelte/CSS token system. Figma is a design reference, not a runtime dependency.

## Figma references

File: `DrvMatch 1.0 UI Review`

- `5:2` — DrvMatch 1.1 Drivers Review: edge-to-edge title/navigation chrome, header-level Check drivers action, flat machine state strip, dense installed → recommended rows, right-side device inspector, operational footer.
- `6:2` — DrvMatch 1.1 Settings: horizontal category tabs, single settings sheet, native-scale rows/toggles, sticky Save/Discard surface.

Figma did not expose Segoe UI in the connected design environment, so Inter was used only as a metric/layout proxy. Production continues to use Segoe UI Variable.

## External Windows UI references

These are references for behavior and information hierarchy, not code to copy wholesale.

- Microsoft WinUI 3 Gallery — canonical Windows controls, adaptive behavior, focus/accessibility examples: https://github.com/microsoft/WinUI-Gallery
- Microsoft PowerToys — mature Windows utility settings patterns, explicit dialogs for consequential/focused editing, keyboard/accessibility consistency: https://github.com/microsoft/PowerToys
- UniGetUI — Windows-first package-management utility with dense package lists, clear operations, metadata/detail views and bulk-management patterns: https://github.com/Devolutions/UniGetUI
- Files — modern Windows desktop information hierarchy and Fluent restraint: https://github.com/files-community/Files
- SettingsUI — small WinUI settings-page implementation derived from PowerToys/Windows Settings patterns: https://github.com/BobbyESP/SettingsUI

## Decisions applied to DrvMatch

- No dashboard health score and no wall of equally important cards.
- One obvious primary action per state: Check drivers before review; Review install for a selected recommendation.
- Raw PnP inventory remains secondary under All hardware.
- Use flat rows and separators for dense comparisons; reserve raised surfaces for mode controls, dialogs, and focused settings sheets.
- Keep the right inspector dense and independently scrollable.
- Settings categories are horizontal; the application does not contain a second permanent navigation sidebar.
- Toggles use familiar Windows-scale switch proportions rather than browser-default checkboxes.
- The operational footer is edge-to-edge and only shows real state/progress; there is no decorative idle progress rail.
- Muted indigo is the product accent. Windows accent remains opt-in.
- Native Windows/DWM owns the outer window corner. The WebView paints the full client area and does not fake another rounded inset window.

## Explicit anti-slop checks

Before accepting future UI work, reject it if it introduces:

- repeated card containers without information-hierarchy value;
- large marketing copy or decorative hero areas inside the desktop utility;
- random icon colors or colored circles for every row;
- gradients/blur merely to make the UI look “modern”;
- huge paddings that reduce usable information density;
- hover lift/scale animations;
- inconsistent control heights or missing CSS for markup classes;
- a second navigation system inside Settings;
- a progress rail that is not tied to real work;
- warning/error colors for generic Microsoft drivers merely because they are generic.
