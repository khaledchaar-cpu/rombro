# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ⏳ next |
| M3 Scanner & Hashing | – |
| M4 Disc-Support | – |
| M5 Naming & 1G1R | – |
| M6 Planner & Import | – |
| M7 App-Shell | – |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M1 fertig: `rombro-rdb` (zero-copy MessagePack-Reader, `RdbFile::open/from_bytes`, `entries()`-Iterator,
`declared_count()`), CLI `rombro db stats [--path]`. Alle 146 echten RDBs fehlerfrei, Zählung = Metadaten-`count`.
Perf: 146 RDBs (738k Einträge) in ~52 ms (rayon, release); SNES-RDB 1,5 ms (`cargo bench -p rombro-rdb`).

## Nächste Schritte (M2 Store & Index)
1. SPEC §7/§10 zu M2 lesen; Store-Format/Index festlegen (Hash→Entry-Lookup über crc/md5/sha1/serial).
2. Metadaten-only-Einträge (ohne `name`) per serial/crc mit Spieleinträgen mergen.

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- `clippy.toml`: `allow-unwrap-in-tests = true`.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
