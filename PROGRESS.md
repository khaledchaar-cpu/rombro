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
| M7 App-Shell | ✅ done |
| M8 GUI Kern | 🔶 in progress |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M8 Teil 1: IPC `plan_import` (Event `import://progress`, Plan wird in App-State gecacht), `execute_plan`
(journalisiert), `undo_last`, `library_list`. UI: Inbox-View (Ordnerwahl via plugin-dialog, Modus, Audit/Plan),
Plan-View (KPIs, virtualisierte Op-Liste, Entscheidungen nach Typ gefiltert, Execute/Undo, auch in Ctrl+K),
Library-Tabelle (virtualisiert, Filter, Sortierung). State in `ui/src/state/importStore.ts`, Library-Pfad in localStorage.
Import-Flow am 2026-10-05 vom User im echten Tauri-Fenster getestet: ok.

## Nächste Schritte (M8 Rest)
1. ~~Versionskonflikt~~ erledigt: npm `plugin-dialog` auf ~2.7.3 gepinnt (Rust-Seite durch MSRV 1.85 auf 2.7 begrenzt).
2. ~~TBD-Queue-Aktionen~~ erledigt: IPC `resolve_ambiguous`, `set_verdict` (keep/discard, Tabelle `verdict`), Buttons + „Re-plan“ in Plan-View. **Manueller Test im Tauri-Fenster steht aus.** Tie-Auflösung via Verdict `prefer` ebenfalls erledigt.
3. Dashboard: ~~offene Entscheidungen (aus letztem Plan, localStorage) + „Review“, Trash-Panel (Liste, endgültig leeren mit 2-Klick)~~ erledigt, manueller Test aus. Library-Zusammenfassung (aus letztem Library-Scan, localStorage) und „Recent runs“ (IPC `journal_list`, Undo-Button) ebenfalls erledigt – manueller Test aus.
4. Offen aus M6: Reflink, Discs in Archiven/CHD, Multi-ROM-Archive, 1G1R-Regel-Config.

## Stolpersteine
- Header-Offset ist **big-endian** u64; Einträge enden mit `nil` (0xc0), danach Map `{"count": n}`.
- rusqlite braucht Feature `fallible_uint` für u64.
- Bulk-Import (>4 Dateien) droppt Lookup-Indizes und baut sie danach neu (9 s → 3 s).
- `clippy.toml`: `allow-unwrap-in-tests = true`.
- MD5 vervierfacht Hash-Zeit → im Scanner aus; `identify` fällt dann für MD5-only-Einträge auf `CrcOnly` zurück.
- `scripts/check.sh | tail` maskiert den Exit-Code – vor Commit ohne Pipe prüfen.
- Disc-RDBs enthalten nur den Datentrack; Serial-Treffer sind mehrdeutig (FF7 Disc 1–3 teilen `SCUS-94163*`).
- 1G1R-Restgleichstände sind meist echte Varianten (andere Disc-Sets, Editionen) → bewusst User-Entscheidung.
- Planner: Library-Items zuerst übergeben (gewinnen bei gleichem Namen); `_quarantine/_playlists` werden nie neu geplant.
- Resolution speichert (system, name) statt entry_id – IDs ändern sich bei `db sync`.
- Tauri-Rust-Crate und `@tauri-apps/api` müssen gleiche Minor haben (aktuell 2.11) – sonst Fehler beim `tauri dev`.
- `pnpm tauri` setzt `TAURI_APP_PATH=../crates/app`; Fortschritts-Events nur alle 64 Dateien.
- App-IPC-Fehler sind Strings; Plan-DTOs (`OpView`/`DecisionView`) leben in `crates/app/src/import.rs`.
- Verdicts/Resolutions überleben Undo (gewollt) → zweiter Lauf zeigt weniger Entscheidungen. Keep wird bei `Duplicate` ignoriert (gleiches Ziel wie der Pick).
- Verdicts gelten pro (system, name); Discard verschiebt immer (auch bei Copy-Modus) nach `<lib>/_trash/`, undo-bar.
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
- 2026-10-05: M7 App-Shell abgeschlossen.
