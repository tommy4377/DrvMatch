# AGENTS.md — DrvMatch

## Product

DrvMatch is a Windows desktop driver manager. Its core rule is:

> Recommend the best driver for this exact machine, not merely the newest version.

The product is already at 1.0.0. Preserve existing behavior and visual language unless
the task explicitly changes them. Prefer small, production-quality changes over broad
rework.

DrvMatch must remain conservative, transparent, explainable, and trustworthy.
Never use alarmist "driver booster" behavior or copy.

## Stack and boundaries

- Tauri 2
- Rust for native/system/backend behavior
- Svelte 5 + TypeScript for UI
- Windows-first desktop application
- strict domain types; avoid `any` except unavoidable interop boundaries

Use modern Svelte 5 patterns in new code. Keep system/native logic out of presentation
components. The UI consumes typed domain/application models; it must not implement
driver matching, installation, or Windows device discovery itself.

Preserve layer separation:

UI / presentation
→ application services
→ driver/device domain
→ source adapters + Windows integration
→ native APIs / network / storage

Source-specific retrieval/parsing must stay separate from ranking logic.

## Driver safety

Suitability beats raw recency.

Never:
- force-install a mismatched INF
- silently install unsigned/untrusted drivers
- replace an OEM-specific package with a generic one only because its version is newer
- hide the driver source or recommendation reason
- treat every newer candidate as recommended
- perform destructive/background installation behavior without the intended user flow

Prefer exact hardware/subsystem/OEM match, correct Windows build/architecture,
trusted source, valid signing/WHQL where applicable, stable branches, and transparent
alternatives.

Potentially destructive operations require explicit confirmation.
Preserve backup/rollback/reboot-state behavior where relevant.

## UI invariants

Treat the current 1.0.0 UI as the source of truth.

Maintain:
- Windows desktop utility feel, not a SaaS/dashboard aesthetic
- compact top-level navigation
- review-first driver workflow
- raw hardware as secondary/advanced information
- one obvious primary action at a time
- restrained motion, borders, rounding, shadows, and acrylic
- coherent light, dark, and acrylic-off states
- keyboard/focus accessibility

Do not introduce:
- PC-health scores or gamified warnings
- giant statistic cards, marketing heroes, gradient headings, or decorative noise
- "everything is outdated" language
- macOS traffic-light controls
- tooltip/title attributes that merely repeat a control name

Do not import application code from `design-refence/`; if that directory is present,
it is reference material only.

## Working with the repository

Inspect the relevant implementation before editing. Prefer semantic/targeted navigation
over dumping large parts of the repository.

Use current documentation for APIs that may have changed, especially Svelte 5, Tauri 2,
Rust crates, Windows-facing APIs, and third-party dependencies.

Do not:
- discard or reset unrelated user changes
- rewrite Git history or force-push
- make unrelated refactors
- install large dependency/component sets without need
- invent native APIs

## Agent orchestration

The root Codex session is the engineering lead. For non-trivial tasks, delegate
independent work when it improves quality or keeps noisy exploration out of the root
context. Do not spawn agents for trivial edits.

Available specialists:
- `explorer`: locate code, trace execution/state/dependencies; read-only
- `docs-researcher`: verify current APIs/docs; read-only
- `architect`: difficult architecture/root-cause decisions; read-only
- `coder`: primary implementation
- `runner`: builds/tests/logs/CLI diagnostics; does not edit source
- `tester`: independent QA and meaningful test work
- `reviewer`: strict final code review; read-only
- `security-auditor`: security/trust/privilege-sensitive changes; read-only

Routing rules:
1. Explore/research in parallel when the subtasks are independent.
2. Keep final scope, architectural decisions, and integration in the root thread.
3. Use `architect` only for genuinely difficult or high-impact decisions.
4. Give implementation ownership to one `coder` for a given overlapping code area.
5. Never have multiple agents concurrently edit the same files/feature.
6. Use `runner` for mechanical verification and diagnostics.
7. Use `tester` after meaningful behavior changes.
8. Use `reviewer` after substantial implementations.
9. Add `security-auditor` when work touches elevation, driver installation/downloads,
   signatures/trust, process execution, networking, parsing, secrets, or unsafe native code.
10. Synthesize subagent evidence before making important decisions; do not blindly accept it.

Bias toward action: infer routine details from repository context and complete the task.
Ask the user only when missing information would materially change the intended outcome.

## Completion

Before finishing:
- inspect the final diff for unrelated edits
- run checks appropriate to the changed area
- verify driver-safety invariants when relevant
- verify keyboard/focus and light/dark/acrylic states when UI surfaces changed
- report what changed, what was validated, and any real remaining risk

Do not broaden testing repeatedly once the relevant checks pass unless a failure or new
risk justifies it.
