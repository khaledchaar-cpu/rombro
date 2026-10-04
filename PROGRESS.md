# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ✅ done |
| M3 Scanner & Hashing | ✅ done |
| M4 Disc-Support | ⏳ next |
| M5 Naming & 1G1R | – |
| M6 Planner & Import | – |
| M7 App-Shell | – |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M3 fertig: `rombro-core` mit `hash` (CRC32+SHA1 in einem Durchlauf, MD5 optional), `header` (iNES, FDS, Lynx,
A7800, SNES-Copier 512 B), `scan` (walkdir + rayon, ZIP/7z-Member, headerless-Variante im selben Pass).
`rombro-store::Store::identify(&Hashes)` → `Verified | CrcOnly | Unknown` (crc+size, SHA1/MD5-Bestätigung,
widersprechende Kandidaten verworfen). CLI `rombro scan <dir> [--db] [--unknown]` inkl. Duplikat-Erkennung.
Perf: Hashing 1,96 GiB/s/Kern (mit MD5 nur 0,53); 10k Dateien/2,3 GB (warm) in 0,11 s. Echter NDS-ZIP verifiziert.

## Nächste Schritte (M4 Disc-Support)
1. SPEC F2 (Disc-Zeile) lesen. `core::disc`: `.cue` parsen (Tracks → bin), `.gdi`, `.m3u`, `.iso`.
2. Serial-Extraktion: PS1/PS2 (`SYSTEM.CNF` im ISO9660), PSP (`UMD_DATA.BIN`/`PARAM.SFO`), Saturn/SegaCD (Header).
3. Match per `by_serial`; Track-Hashes per crc; Disc-Fixtures synthetisch (Mini-ISO9660) erzeugen.
4. Scanner: Track-Dateien eines Cue nicht doppelt als Einzel-ROMs melden.

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.
- MD5 vervierfacht Hash-Zeit → im Scanner aus; `identify` fällt dann für MD5-only-Einträge auf `CrcOnly` zurück.
- `scripts/check.sh | tail` maskiert den Exit-Code – vor Commit ohne Pipe prüfen.
- Store hängt jetzt von Core ab (für `Hashes`); Core bleibt store-frei.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
- 2026-10-04: M2 Store & Index abgeschlossen.
- 2026-10-04: M3 Scanner & Hashing abgeschlossen.
