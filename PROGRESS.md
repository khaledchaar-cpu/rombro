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
| M9 GUI Feinschliff | ✅ done |
| M10 Gamification | ✅ done (Sichtprüfung offen) |
| M11 Release | ✅ done (CI grün auf 3 OS) |
| M12 Übertragen & Container | ✅ done |
| M12b Arcade & Sichtprüfung | ✅ done |

## Aktuell
M12b: **Arcade-Erkennung** (FBNeo/MAME über Hash des ganzen Zips, Kurzname bleibt, kein 1G1R, CHD-Ordner mit,
BIOS → `_bios/<core>/`, sonst unsichtbar). Unbekannte Archive werden nie zerlegt (ganz in Quarantäne).
UI: Fortschritt zeitbasiert mit Phase + Sekunden, klarere Rejected-Gründe (+ System), Sync-Feedback,
Completeness-Layout. Echte CHDs (PSX) und Amiga-`.lha` erkannt; Import auf Testordner ausgeführt.

## Nächste Schritte
1. Sichtprüfung: MAME-BIOS liegt jetzt neben den Romsets – in RetroArch mit Neo-Geo-Set gegenprüfen.
2. Sichtprüfung Rest: Gamification-Panels, Effects off.
3. Echte 7z mit cue/bin und Multi-ROM-Zips (Konsole) testen.
4. Erster Release per Tag `v*` (unsigniert – keine Developer-Accounts; Signierung entfällt bis auf Weiteres).
5. v2 Launcher (SPEC §6) – **erst ganz zum Schluss** (User-Wunsch).

## Stolpersteine
- Arcade-RDB-Einträge (FBNeo/MAME) hashen das **ganze Zip** → `ScanReport.archives`; Treffer werden `Files::Set` (Kurzname, kein 1G1R, CHDs aus `<set>/`), BIOS → `Ident::Bios` → FBNeo `_bios/fbneo/`, MAME-Cores neben die Sets (`arcade::bios_dir`).
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
- Thumbnail-Name: nur ``&*/:`<>?\|`` → `_` (nicht `"`), anders als `sanitize_file_name`.
- `data-effects="off"`-Block muss in tokens.css **nach** den Theme-Blöcken stehen (gleiche Spezifität).
- Achievements bleiben einmal erreicht freigeschaltet; dynamische (`full-set:<System>`) verschwinden aber aus der Liste, wenn die Bedingung wegfällt.
- Pfade unter macOS/Windows weichen ab: DB liegt via `dirs::data_dir()` (Linux unverändert `$XDG_DATA_HOME`).
- `pnpm tauri`-Script setzt Env per Shell-Syntax → in CI Env-Vars über `env:` + `pnpm exec tauri`.
- Trash-Größe = `stat` der Trash-Ziele aus den Journalen; nach Leeren des Trashs 0 (zählt Belegung, nicht Historie).
- Store hängt jetzt von Core ab (für `Hashes`); Core bleibt store-frei.
- Archiv-Items: `Op::Extract.from` = Archiv, Member-Name separat; im Builder liefert `name_source()` den Member-Namen.
- Disc-Archive: kein Serial-Fallback (Tracks liegen nur gepackt vor); mehrere Sheets im Archiv → Skip.
- CHD-Frames sind 2448 Byte (2352 + Subcode), Tracks auf 4 Frames gepaddet (GD: `PAD:`), `PGTYPE:V…` = Pregap gespeichert.
- Scan-Cache: Archive ohne Whole-Hash-Eintrag (`member: None`) werden neu gehasht (Migration ohne Schema-Änderung).
- Fortschritt: `Throttle` (100 ms) statt alle N Dateien; Phase `planning` ohne Zähler.
- Kein `chdman` lokal → Test baut unkomprimierte CHD v5 selbst (`crates/core/tests/chd.rs`).

## Log
- 2026-10-05: Öffentliches Repo github.com/khaledchaar-cpu/rombro, CI grün (Linux/macOS/Windows). Commit-Mail = GitHub-noreply.
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
- 2026-10-05: M9 GUI Feinschliff abgeschlossen.
- 2026-10-05: M10 Gamification abgeschlossen.
- 2026-10-05: M11 Release abgeschlossen.
- 2026-10-05: M12b Arcade-Erkennung & Sichtprüfungs-Fixes.
- 2026-10-05: F6-Reste (Trash-Größe, Toast, Franchise-Ziele).
