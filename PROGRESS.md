# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ✅ done |
| M2 Store & Index | ✅ done |
| M3 Scanner & Hashing | ⏳ next |
| M4 Disc-Support | – |
| M5 Naming & 1G1R | – |
| M6 Planner & Import | – |
| M7 App-Shell | – |
| M8 GUI Kern | – |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M2 fertig: `rombro-store` (rusqlite bundled, WAL, Migrations via `user_version`), `Store::sync_rdbs(dir)`
inkrementell per mtime+size, Metadaten-only-Einträge per serial→crc gemergt (Rest = orphaned, verworfen),
Lookup `by_crc(crc, size)`, `by_sha1`, `by_md5`, `by_serial(serial, system)`, `system_counts`.
CLI: `rombro db sync [--path] [--db]`, `rombro db lookup <crc|md5|sha1|serial>`.
Perf (release): Kalt-Import 146 RDBs/738k Einträge ~3 s, warm (unverändert) 2 ms, Lookup ~60 µs. DB ~225 MB.

## Nächste Schritte (M3 Scanner & Hashing)
1. SPEC F2 lesen. `rombro-core`: paralleler Dir-Walk, Streaming-Hash (crc32fast, sha1, md-5) in einem Durchlauf.
2. ZIP/7z-Inhalte hashen; Header-Stripping (iNES, SNES-Copier-Header 512 B, Lynx, FDS, A7800).
3. Match gegen Store (crc+size → sha1-Bestätigung), CLI `rombro scan <dir>` mit Verified/Unknown.

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
- 2026-10-04: M1 RDB-Parser abgeschlossen.
- 2026-10-04: M2 Store & Index abgeschlossen.
