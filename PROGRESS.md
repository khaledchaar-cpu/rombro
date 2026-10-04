# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ✅ done |
| M3 Scanner & Hashing | ✅ done |
| M4 Disc-Support | ✅ done |
| M5 Naming & 1G1R | ✅ done |
| M6 Planner & Import | ✅ done |
| M7 App-Shell | ⏳ next |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M6 fertig: `core::plan` (`build` → `Plan{ops, decisions, …}`, `execute` → Journal `Vec<Done>`, `undo`,
`lpl::render`), `store` Migration v2 (`journal`, `resolution`), `Store::items(report, in_library)`.
CLI: `rombro import <inbox> <lib> [--dry-run] [--mode move|copy|hardlink] [--playlists DIR|--no-playlists]`,
`rombro audit <lib>`, `rombro undo`, `rombro resolve <file> [n]`. Ende-zu-Ende-Test in `store/src/tests.rs`.

## Nächste Schritte (M7 App-Shell)
1. SPEC §7/§8 lesen. Tauri 2 + SolidJS + Vite in `ui/`, `crates/app` als Tauri-Shell.
2. Design-Tokens `ui/src/styles/tokens.css`, Layout (Sidebar, Topbar), Command-Palette, Dashboard-Dummy.
3. IPC: erste Commands (`db_stats`, `scan` mit Fortschritts-Events) als dünne Adapter auf core/store.
4. Offen aus M6: Reflink-Modus, Discs in ZIP/7z + CHD, Archive mit mehreren ROMs (werden übersprungen),
   konfigurierbare 1G1R-Regeln/Templates (aktuell Defaults), Journal-Liste/Undo älterer Läufe.

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.
- MD5 vervierfacht Hash-Zeit → im Scanner aus; `identify` fällt dann für MD5-only-Einträge auf `CrcOnly` zurück.
- `scripts/check.sh | tail` maskiert den Exit-Code – vor Commit ohne Pipe prüfen.
- Disc-RDBs enthalten nur den Datentrack; Serial-Treffer sind mehrdeutig (FF7 Disc 1–3 teilen `SCUS-94163*`).
- 1G1R-Restgleichstände sind meist echte Varianten (andere Disc-Sets, Editionen) → bewusst User-Entscheidung.
- Planner: Library-Items zuerst übergeben (gewinnen bei gleichem Namen); `_trash/_quarantine/_playlists` werden nie neu geplant.
- Resolution speichert (system, name) statt entry_id – IDs ändern sich bei `db sync`.
- Store hängt jetzt von Core ab (für `Hashes`); Core bleibt store-frei.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
- 2026-10-04: M2 Store & Index abgeschlossen.
- 2026-10-04: M3 Scanner & Hashing abgeschlossen.
- 2026-10-04: M4 Disc-Support abgeschlossen.
- 2026-10-05: M5 Naming & 1G1R abgeschlossen.
- 2026-10-05: M6 Planner & Import abgeschlossen.
