# Progress

| Milestone | Status |
|---|---|
| M0 Bootstrap | ⏳ next |
| M1 RDB-Parser | – |
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
Projekt aufgesetzt: CLAUDE.md, SPEC.md, PROGRESS.md. Toolchain vorhanden: cargo 1.99, node 26.

## Nächste Schritte (M0)
1. Cargo-Workspace mit `crates/{rdb,core,store,cli,app}` (app erst als Platzhalter).
2. `scripts/check.sh` (fmt --check, clippy -D warnings, test).
3. `.gitignore`, erster Commit.

## Log
- 2026-10-04: Projekt-Dokumente erstellt, RDB-Format verifiziert (siehe SPEC §3).
