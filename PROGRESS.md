# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ✅ done |
| M3 Scanner & Hashing | ✅ done |
| M4 Disc-Support | ✅ done |
| M5 Naming & 1G1R | ✅ done |
| M6 Planner & Import | ⏳ next |
| M7 App-Shell | – |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M5 fertig: `core::naming` (`tags::parse` → Titel, Regionen, Sprachen, Rev, Disc, Flags; `group_key`,
`release_name`, `sanitize_file_name`, `target_path`, `playlist_path`) und `core::g1r::select` (Gruppierung,
Scoring, Ausschlüsse, Begründungen, `needs_decision`, Duplikate). CLI `rombro g1r <system> [--filter]`.
Echte Daten: SNES 7696 Einträge → 2382 Spiele (104 brauchen Entscheidung), PS1 13507 → 6468 (259),
MD 7334 → 1625 (101; 702 Gruppen nur Hacks/Betas). M4: Disc-Support, mehrdeutige Treffer → `AMBIG`.

## Nächste Schritte (M6 Planner & Import)
1. SPEC F4 + Abschnitt 11a (Ambiguous/Resolution) lesen. `core::plan`: Plan = Liste Operationen
   (move/copy/hardlink/reflink, mkdir, write m3u), Dry-Run-Ausgabe; Konflikte (Ziel existiert) erkennen.
2. Pipeline: scan → identify → 1G1R über Treffer (+ vorhandene Library) → Plan; Unknown → `_quarantine`,
   verworfene Dubletten → `_trash/<ts>`; Ambiguous/needs_decision → offene Entscheidung (Tabelle `resolution`).
3. Execute + Journal (SQLite) + Undo; nur mit Tempdir-Fixtures testen.
4. `.lpl`-Export; CLI `rombro import <inbox> <library> --dry-run`, `rombro undo`.
5. Offen: Discs in ZIP/7z bzw. CHD (Vorschlag: hier mitnehmen).

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.
- MD5 vervierfacht Hash-Zeit → im Scanner aus; `identify` fällt dann für MD5-only-Einträge auf `CrcOnly` zurück.
- `scripts/check.sh | tail` maskiert den Exit-Code – vor Commit ohne Pipe prüfen.
- Disc-RDBs enthalten nur den Datentrack; Serial-Treffer sind mehrdeutig (FF7 Disc 1–3 teilen `SCUS-94163*`).
- 1G1R-Restgleichstände sind meist echte Varianten (andere Disc-Sets, Editionen) → bewusst User-Entscheidung.
- Store hängt jetzt von Core ab (für `Hashes`); Core bleibt store-frei.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
- 2026-10-04: M2 Store & Index abgeschlossen.
- 2026-10-04: M3 Scanner & Hashing abgeschlossen.
- 2026-10-04: M4 Disc-Support abgeschlossen.
- 2026-10-05: M5 Naming & 1G1R abgeschlossen.
