# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ✅ done |
| M1 RDB-Parser | ⏳ next |
| M2 Store & Index | – |
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
M0 fertig: Workspace `crates/{rdb,core,store,cli,app}` (Paketnamen `rombro-*`, CLI-Binary `rombro`),
`scripts/check.sh` (fmt, clippy -D warnings, test; scheitert korrekt bei Testfehlern).

## Nächste Schritte (M1 RDB-Parser)
1. `crates/rdb`: minimaler MessagePack-Leser (map, str, bin, uint, nil) über `&[u8]`, zero-copy.
2. `Entry`-Struct + `RdbFile::open(path)` → Iterator über Entries; Header `RARCHDB\0` prüfen.
3. Unit-Tests mit synthetisch erzeugter Mini-RDB; Integrationstest gegen echte RDBs nur wenn vorhanden (ignored).
4. CLI `rombro db stats [--path]`: Systeme + Eintragszahlen, Ladezeit; Benchmark aller 146 RDBs.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
- 2026-10-04: M0 Bootstrap abgeschlossen.
