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
| M8 GUI Kern | ✅ done |
| M9 GUI Feinschliff | – |
| M10 Gamification | – |
| M11 Release | – |

## Aktuell
M8 abgeschlossen: Import-Flow (Inbox → Plan → Execute/Undo), Library-Tabelle, TBD-Queue in der Plan-View
(Ambiguous auflösen, Tie via `prefer`, Rejected keep/trash, Re-plan, entschiedene Items rücken nach),
Dashboard (Library-Zusammenfassung, offene Entscheidungen + Review, Trash mit endgültigem Leeren, Recent runs + Undo).
Letzte Panels (Tie-Buttons, Library-Panel, Recent runs) vom User freigegeben ohne ausführlichen Einzeltest.

## Nächste Schritte (M9 GUI-Feinschliff)
1. ✅ Persistenter Library-Index: Tabelle `file` (Pfad → Größe/mtime/ROM-Hashes als JSON) dient als Hash-Cache
   für Library *und* Inbox (`scan_cached`), Execute/Undo verschieben Index-Zeilen mit (`index_executed/undone`),
   Library-Pfad als Setting `library`, App lädt Library beim Start (`initLibrary`), „Rescan“ statt „Load“.
   Offen: Startzeit mit großer echter Library prüfen.
   Danach (User-Feedback): Inbox-Pfad/Modus in DB, Grund je Plan-Op (`Plan.why`), `.lpl` nie als ROM,
   Library-Filter (System/Region/Status/Hinzugefügt; `file.added` in DB), eigene Dropdown-Komponente
   `components/Select.tsx` (Cyberpunk-Stil, Portal) – vom User abgenommen.
2. Manuelle Sichtprüfung der neuen Dashboard-Panels im Tauri-Fenster; Layout `.rows` mit 3 Spalten (Recent runs) prüfen.
3. ✅ Settings-View: 1G1R-Regeln-Editor (Regionen/Sprachen per Drag (Pointer-Events) + ↑↓, Ausschluss-Flags),
   gespeichert als JSON-Setting `rules`, Import nutzt sie (vom User getestet). Rahmenloses Fenster: Topbar ist
   Drag-Region, eigene Neon-Fensterbuttons (abgenommen). Offen: CLI nutzt noch Default-Regeln.
4. Rest M9: Thumbnails (F7) als Nächstes, dann Effekte, Light-Theme.
5. Übertragen aus M6/M8 (nach M9 einplanen): Reflink, Discs in Archiven/CHD, Multi-ROM-Archive, 1G1R-Regel-Config.

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
- Tie-Entscheidungen sind in der UI per `decisionKey` eindeutig (Tie-`path` ist das System).
- `pkill -f <muster>` in Bash killt die eigene Shell mit (Muster steht in der Kommandozeile) → `pgrep`/PID nutzen.
- Index-Präfixsuche per `substr(path,1,n)`; Pfade als lossy UTF-8. Disc-ID wird bei Cache-Treffer neu gelesen (billig).
- Virtuelle Listen: Zeilen per Accessor (`() => view()[i]`) lesen, sonst bleiben sie nach Filtern stale.
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
- 2026-10-05: M8 GUI Kern abgeschlossen.
