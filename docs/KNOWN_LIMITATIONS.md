# Known limitations — 1.0.0

DrvMatch 1.0.0 targets Windows 11 x64 as its supported stable platform. Its conservative failure behavior is deliberate: unavailable evidence results in **Needs review**, **Optional**, or no recommendation rather than a guessed match.

## Platform and packaging

- Windows 11 x64 is the supported release target. Windows 10 and Windows on ARM are not release-tested.
- Release bundles produced without a configured publisher certificate are not Authenticode-signed. Driver packages are still independently checked through Windows trust APIs before installation.
- The main window is intentionally fixed at 1180 × 760 logical pixels. Tauri and WebView2 provide DPI scaling, but uncommon accessibility scaling combinations may reduce the amount of information visible at once.

## Sources

- Structured OEM discovery is limited to Dell, Lenovo, and HP machines whose official catalogs expose both exact model/platform evidence and device-level PnP IDs.
- AMD, NVIDIA, and Intel product pages provide discovery metadata. A candidate remains **Needs review** until package-level applicability can be independently proven.
- Microsoft Update Catalog and vendor web adapters depend on public response formats that providers can change. Requests have explicit timeouts, bounded transient retries, response-size limits, visible per-source failures, and cached last-successful metadata.
- Live-source tests are opt-in because they require external services and can be affected by provider availability. Offline parser fixtures remain part of the normal test suite.

## Installation and recovery

- Cancellation is available only while bytes are being downloaded. Once hashing, signature verification, safety preparation, or an elevated Windows operation begins, DrvMatch keeps the operation non-cancellable to avoid abandoning a system change mid-flight.
- System Restore can be disabled or unavailable under local Windows policy. DrvMatch records the attempt and does not claim that a restore point exists unless Windows reports success.
- Exporting the previous package is recovery evidence, not proof that Windows native rollback will succeed. Rollback is exposed only when the recorded change meets the native eligibility rules.
- Vendor EXE/MSI packages may implement their own UI, restart behavior, and exit codes. DrvMatch verifies trust and records the handoff, but cannot reinterpret undocumented vendor behavior.
- If DrvMatch or Windows stops unexpectedly, incomplete `.part` downloads are removed on the next launch and an install-attempt record written before system work remains visible in History. The user should still inspect the device in Windows before retrying an interrupted operation.

## Privacy and telemetry

- DrvMatch has no telemetry or cloud account. Local inventory, cache, logs, managed packages, backups, and history remain in the application data directory.
- Source queries necessarily disclose the selected hardware ID or applicable system model/platform identifier to the corresponding official provider, as described in the README.
