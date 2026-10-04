# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ✅ done |
| M3 Scanner & Hashing | ✅ done |
| M4 Disc-Support | ✅ done |
| M5 Naming & 1G1R | ⏳ next |
| M6 Planner & Import | – |
| M7 App-Shell | – |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M4 fertig: `core::disc` – `sheet` (cue/gdi/m3u), `iso9660` (Sektorlayout per Sync, Root-Datei lesen,
`testimg` für synthetische Images), `serial` (PS1/PS2 `SYSTEM.CNF`, PSP `UMD_DATA.BIN`, Saturn/SegaCD/DC-Header).
Scanner liefert `discs` (Tracks gehasht, `missing`) + `playlists`; Track-Dateien nicht doppelt.
`Store::identify_disc` → `Hash(Match)` | `Serial(records)` | `Unknown`; `by_serial_prefix` für Multi-Disc.
CLI `scan` zeigt Discs (OK/CRC/SERIAL/UNKNOWN + Serial). Smoke-Test mit synthetischem PS1/PSP-Image gegen echte DB ok;
echte Disc-Images liegen auf dem System keine.

## Nächste Schritte (M5 Naming & 1G1R)
1. SPEC F3 (1G1R) + Naming-Abschnitt lesen. Titel-Normalisierung (Tags, Artikel, Satzzeichen) in `core::naming`.
2. No-Intro/Redump-Tags parsen (Region, Sprachen, Rev/v, Beta/Proto/Demo, [b]/Hack) → Flags.
3. Gruppierung + Auswahl nach Regionspriorität (offene Frage Default!) → Tests mit realen Namen aus der DB.
4. Ziel-Pfade nach SPEC-Schema (inkl. Multi-Disc + `.m3u`).

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.
- MD5 vervierfacht Hash-Zeit → im Scanner aus; `identify` fällt dann für MD5-only-Einträge auf `CrcOnly` zurück.
- `scripts/check.sh | tail` maskiert den Exit-Code – vor Commit ohne Pipe prüfen.
- Disc-RDBs enthalten nur den Datentrack; Serial-Treffer sind mehrdeutig (FF7 Disc 1–3 teilen `SCUS-94163*`).
- Store hängt jetzt von Core ab (für `Hashes`); Core bleibt store-frei.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
- 2026-10-04: M2 Store & Index abgeschlossen.
- 2026-10-04: M3 Scanner & Hashing abgeschlossen.
- 2026-10-04: M4 Disc-Support abgeschlossen.
