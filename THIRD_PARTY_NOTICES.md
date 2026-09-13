# Third-party notices

DrvMatch 1.0.0 directly relies on these principal Rust libraries for Windows integration, source discovery, parsing, and persistence:

- `windows` 0.62.2 — Microsoft Windows API bindings; MIT OR Apache-2.0.
- `reqwest` 0.13.5 — HTTP client; MIT OR Apache-2.0.
- `scraper` 0.27.0 — HTML parsing and CSS selectors; ISC.
- `rusqlite` 0.37.0 — SQLite bindings used for local inventory and metadata persistence; MIT.
- `cab` 0.6 — CAB archive decoding used for Dell and HP OEM catalog metadata; MIT.
- `roxmltree` 0.21 — read-only XML parsing used by Dell, Lenovo, and HP catalog adapters; MIT OR Apache-2.0.
- `windows-registry` 0.6 — Windows registry access used to detect system/OEM identity; MIT OR Apache-2.0.

Transitive dependencies retain their respective licenses. Package metadata in `Cargo.lock` and `package-lock.json` identifies the resolved dependency versions used for this release.
