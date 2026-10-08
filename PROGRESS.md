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
| M12c Echte Sammlung (Probelauf) | ✅ done |
| M13 Regeln transparent & einstellbar | ✅ done (Rules-Seite abgenommen 2026-10-06; Entscheidungen zeigen Grund + Datum) |
| M14 Arcade-DATs | ✅ done |
| M14b Core-Wahl & -Installation | ✅ done (Sichtprüfung App offen) |
| v0.3.0 Release + Inbox-Importe | ✅ released 2026-10-06; Nachträge 2026-10-07 (CSO, OpenBOR, Library-Perf) |
| v0.4.0 Release (Perf, Settings-Umbau) | ✅ released 2026-10-07 |
| v0.5.0 Release (Firmware/BIOS, Daphne, Disc-Serials, neue UI) | ✅ released 2026-10-08 |

## Aktuell
Stand 2026-10-07: v0.3.0 veröffentlicht. Seitdem: `.cso` (PSP) wird entpackt gehasht + Serial (`disc::cso`),
`openbor` als Namensordner (auch in der User-DB eingetragen), Library listet MSU-1-Ordner als ein Spiel,
Library-Scan beim Planen/Export vertraut dem Index (`HashCache.trusted`, ~16 s → ~3 s; Rescan prüft voll).
Nachträge 2026-10-07 (2): `[Patched]`/`(Patched)` = Hack-Flag; Name-Ordner in der Rules-Seite editierbar
(Regel `name-only`); Audit der echten Library 10,8 s → 1,8 s bei identischem Plan: Disc-Serial wird mit dem
ersten Track im Hash-Cache gespeichert (`CachedRom.disc`), Zips nur bei DAT-bekanntem Namen geöffnet
(`Store::dat_knows`), MSU-1-Suche nur neben SNES-ROMs, ScummVM-Launcher-Reparatur nur für ScummVM-Ordner.
Sichtprüfung neuer UI-Teile abgenommen 2026-10-07 (Mock + echte App).
Nächste Schritte:
GUI aufgeräumt (2026-10-07): Rules + Settings = ein Tab „Settings“ mit Sektionen Rules/Databases/RetroArch/
Appearance (`state/settingsNav`, auch per Command-Palette); DB-Sync nach Settings → Databases, Dashboard zeigt
nur noch Hinweis bei leerer DB; Quick scan entfernt; eigene Checkboxen (Tokens, beide Themes).
v0.4.0 veröffentlicht 2026-10-07 (u. a. hardlink/reflink entfernt).

Stand 2026-10-08: v0.5.0 veröffentlicht. Danach auf `main` (für v0.5.1), alle aus echten Inbox-Läufen:
- Arcade: neu gepackte Sets/BIOS über DAT erkannt (`Store::complete_sets`, `bios_set`); Indizes
  `dat_set(name)`, `entry(rom_name)`, DAT-Set-Cache → Library-Neuerkennung 51 s → 8,6 s (22k Dateien, CIFS).
- Neu gepackte Zips mit gleichen Membern = Duplikat statt Konflikt (`same_content`).
- Snapshot bleibt nach eigenen Läufen erhalten, nur berührte Pfade werden neu erkannt (`snapshot_dirty`, Schema v14).
- Disc-Serials: Dreamcast/Sega-CD-Schreibweisen, exakter Treffer vor Suffix, Release vor Beta.
- Daphne-Sammlungen als Einheit (`Daphne/`, kein Core auf Linux-Buildbot); Library-Ansicht 1 Eintrag je `.daphne`.
- `_trash` wird nicht gescannt; Trash-Leeren parallel; `.p2k.cfg` = Frontend-Meta; `n64dd` kein Name-Ordner mehr.
- Fix: Session-Fehler beim Start setzte einen alten localStorage-Pfad als Library.
- CI: apt-Schritte mit 10-min-Timeout (Linux-Runner hingen stundenlang).
Stolpersteine: CLI `audit` führt ohne `--dry-run` aus (nur mit `import --dry-run` analysieren!); App läuft im
Dev-Modus, Codeänderungen starten sie neu → nie während Plan/Import des Users editieren.
Nächste Schritte: v0.5.1 releasen; Atari-ST-Mehrdisk-Teile (Boot/Data, (A)/(B)) nicht als TIE behandeln;
Sufami-Turbo-Kombi-Images in Zips; M15 Launcher-Basis.
Stolperstein: App läuft beim User per `pnpm tauri dev` – Code-Änderungen starten sie neu (laufende Scans brechen ab).

M14: `core::arcade::dat` (Logiqx/listxml-Parser, Prüfung über Zip-Verzeichnis), `archive::members`, Store-Tabellen
`dat_source`/`dat_set` (v7), `db sync` lädt 8 DATs (21 s, versioniert, offline → alte + Warnung), `catalog::dat_pick`
wählt den ersten vollständigen Core, sonst `Ident::Incomplete` → Quarantäne mit Regel `arcade-dat` + Begründung
(CLI `[regel: detail]`, App „why“). Abnahme: `1943`/`1943kai` → MAME 2003-Plus, läuft headless 300 Frames in
RetroArch; aktuelles MAME scheitert weiter (Exit 1). Echte Sammlung (Dry-Run Arcade-Ordner): 11× arcade-dat-Quarantäne
mit plausiblen Gründen (fehlend, misnamed, Eltern fehlt, nicht im DAT).
Nachträge: Eltern-Suche über Scan + Library, Begründung „skipped …“ bei platzierten Sets, `db stats` zeigt
DAT-Versionen; Neo Geo (`2020bb`) läuft in RetroArch mit mame2016 und FBNeo (BIOS neben dem Set).
**Nächster Milestone: M15 Launcher-Basis** (SPEC §6) – Schnitt noch festlegen.

## Nächste Schritte
-1. RetroArch-Stichprobe (2026-10-08, 2 Spiele/Playlist, 142 Starts headless): 121 OK. Rest nicht ROMBRO:
   Sega CD `bios_CD_U.bin` fehlt, Satellaview vermutl. `BS-X.bin`, melonDS-Core (exec stack, glibc), PPSSPP-Assets
   fehlen, blueMSX (Coleco/MSX2) „Failed to load content“ ungeklärt, Snes9x ignoriert `--max-frames` (lädt aber).
   ROMBRO-Fix: Arcade-BIOS in alle MAME-Ordner, deren DAT es führt (MAME 2010 fand `neogeo.zip` nicht).
   **Offen:** `audit` auf echter Library ausführen (40 Kopien, User-Freigabe).
0. Nachträge 2026-10-06 (abends): RetroArch-Export räumt verwaiste Playlists (alle Einträge in der Library, System
   leer oder ohne Core) nach `_trash/playlists`; Systeme ohne jeden Core (Solarus) bekommen keine RA-Playlist.
   Name-Ordner (`rules.name_folders`, Regel `name-only`, `Ident::Named`, SPEC 11a): n64dd-Cartridge-Umbauten → N64,
   solarus → Solarus. GUI: „Last run“-Bericht nach Execute, Meldung bei leerem Plan. **Offen:** Sichtprüfung der neuen GUI-Teile;
   **Aufräumen GUI-Elemente**.
   Stolperstein: nach Schema-Wechseln mit DAT-Drop erst `db sync`, sonst greifen Arcade-Prüfungen nicht.
0. M14b (2026-10-06): Core-Empfehlung (Batocera) + Auswahl + Installation beim Export, siehe SPEC 11a. CLI e2e in
   Scratch-RA verifiziert (4 Cores geladen/entpackt, Playlists mit Pfad, Undo). **Offen:** Sichtprüfung CorePicker in
   der App (Settings → RetroArch); installierten Core einmal headless in RetroArch starten.
   Nachträge: Dashboard „Sync databases“ + Fortschritt (RDB/DAT), Export-Fortschritt (scan/download/write,
   `PhaseProgress`), Plan-Panel „Left in inbox“ + „Move to trash“ (`plan::inbox`, Journal → `_trash/inbox-<ts>/`),
   CLI-Import nutzt Hash-Cache. Inbox auf CIFS: erster Scan netzgebunden (~118 MB/s), danach Cache (0,2 s).
   Sichtprüfung dieser UI-Teile ebenfalls offen (Chrome-Extension war nicht verbunden).
   Weitere Nachträge (2026-10-06): Import-Tab (Inbox+Plan), Library zählt Ordner-Spieldaten zum Spiel, Regel
   `frontend-meta` (gamelist/Medien → `_trash/frontend`), Unbekanntes → `_trash/unknown` statt Quarantäne
   (`unknown_to_trash`), geleerte Ordner werden entfernt, lose Arcade-Chips nicht mehr als Spiel, DAT-Prüfung mit
   CHDs (`<disk>`) und Treiberstatus (`arcade_working_only`), verwaiste Playlists → `_trash/playlists`.
   Schema v11: DATs müssen einmal neu geladen werden („Sync databases“).
   **Offen:** Sichtprüfung App; doppeltes Quake (`Quake/quake` vs. `tyrquake`); RetroArch-Export entfernt keine
   veralteten Playlists im RetroArch-Ordner (z. B. alte `MAME.lpl`).
   Import 4 Systeme (2026-10-06): DAT-benannte ZIPs ohne RDB-Treffer bekommen den DAT-Grund (Model 3 „not working“
   → Trash statt Quarantäne, ≥75 % Member-Treffer), `astrocde` → `Bally - Astrocade` (Namensordner, kein Core),
   GX4000-`.cpr` bleiben bewusst in `Amstrad - CPC` (RDB führt sie dort), geleerte Inbox-Bäume samt leerer
   `media/*/default_*` werden entfernt. 2048-Byte-`.iso` (3DO u. a.) werden zusätzlich als rohe MODE1-Sektoren (EDC/ECC) gehasht (`hash::hash_iso`, Ergebnis in `headerless`). RetroArch-Export schreibt ScummVM-Targets in `scummvm.ini` (`retroarch::scummvm`, Engine-Tabelle `data/scummvm-engines.tsv` aus scummvm-web `games.yaml`). MSU-1: Ordner mit `.msu` (oder ROM + `-N.pcm`) → `Nintendo - Super Nintendo Entertainment System (MSU-1)/<Ordner>/` als Ganzes (`plan::build::msu`, Regel `msu1`, fehlender Marker wird angelegt), Core snes9x.
   Atomiswave/Naomi (2026-10-06): RDB nennt nur einen Key-Chip pro Spiel → ZIP mit diesem Chip = Set
   (`store::chipset`, `arcade::CHIP_KEYED`), vor der MAME-„not working“-Regel (Flycast, nicht MAME);
   `awbios`/`naomi`/`naomi2` → `_bios/dc/` (Export → `system/dc/`).
   CHD-Pregap (2026-10-06): RDB hasht den Datentrack wie Redumps `.bin` inkl. Pregap; nicht gespeicherter Pregap
   wird nachgebaut (Stille + 150 leere MODE1-Sektoren mit EDC/ECC, `disc::cdsector`, `ChdTrack::open_redump`).
   PCE CD 1 → 16/28 erkannt; Migration v12 hasht CHDs neu.
1. ✅ Rules-Seite abgenommen (User, 2026-10-06). Alte Entscheidungen ohne gespeicherten Grund → „reason not recorded“.
2. ✅ Echter Import auf Testset (2026-10-06, `scripts/make-testset.sh`): Import → Audit (0 Ops) → Undo bitgleich;
   Ignore, Quarantäne aus, SNES-Override Japan verifiziert. Bug gefixt: ScummVM-Ordner wanderte beim Audit nach DOS.
   ✅ Komplett-Probelauf (2026-10-06, 2 h 05 min, `~/rombro-test/fullrun.txt`): 225.720 Items, 16.522 zu platzieren,
   5.270 Quarantäne (wie M12c), 1.672 Entscheidungen, 39.635 Ops, 3 Konflikte (bitgleiche Doppel: 2 FBNeo-BIOS in
   `Commodore - C64/FBNeo - Arcade/` + `00bios`, Wolfenstein 3D in `ECWolf/` + `Ports/`), 18 Lesefehler (MAME-/CD-i-
   HDD-CHDs ohne CD-Track, 2 CHDs mit Dekompressionsfehler = vermutlich defekt, 2 Zips).
   Danach: bitgleiche Doppel → Duplikat in der TBD-Queue statt Konflikt (BIOS-Fälle verifiziert; Wolfenstein ist
   echter Konflikt: Shareware vs. Vollversion). CLI: `rules --ignore/--unignore <pfad>`.
3. ✅ (2026-10-06) Prüfung in RetroArch: `2020bb` (Neo Geo) läuft headless 300 Frames mit mame2016 und FBNeo,
   jeweils mit `neogeo.zip` neben dem Set aus `ra-lib`.
   Ursprünglich: MAME-BIOS neben den Sets (BIOS-abhängiges Spiel, z. B. Neo Geo, startet mit MAME-Core
   und FBNeo aus der importierten Library).
   Vorbereitet (2026-10-06): `~/rombro-test/ra-lib` (Copy-Import Arcade-Testset), Playlist `ROMBRO Test.lpl`,
   Cores fbneo + mame2016 + mame (0.289) vom Buildbot. User-`neogeo.zip` ist nicht DB-konform (4 Dateien fehlen, 2 anders
   benannt) → Quarantäne ist korrekt; für den Test manuell neben die Sets kopiert. Logik bleibt (User-Entscheidung).
4. ✅ M14 Arcade-DATs. ✅ Ideen umgesetzt: platzierte geprüfte Sets zeigen übersprungene Cores
   (`[arcade-set: skipped MAME: …]`, `Files::Set.dat_note`); ~~Eltern-Set auch in der Library suchen~~ ✅ (Eltern zählen, wenn sie irgendwo im Scan oder in der Library liegen: `set_names`/`items_with`); ✅ `db stats` zeigt DAT-Versionen (`--db`).
4b. ✅ (2026-10-06) Kombinierte Sufami-Turbo-Images → BIOS + Carts extrahiert (8 echte Dateien als Kopie: 7 Carts +
   BIOS SHA1-verifiziert, Undo bitgleich). Schema v9 erzwingt Rehash betroffener SNES-Dateien im Cache.
4c. ✅ (2026-10-06) RetroArch-Export (Playlists mit Core, BIOS aus System.dat). Getestet mit Scratch-cfg:
   ra-lib → 3 Playlists mit FBNeo/MAME 2016/MAME; Sufami-Kopie → `STBIOS.bin` SHA1-korrekt. App-Panel nicht visuell
   geprüft. Idee: System.dat bei `db sync` aktualisieren; Core-Wahl pro System einstellbar.
5. Sichtprüfung in der App (M10-Rest): Gamification-Panels mit echten Daten plausibel; Effects off in beiden Themes.
6. v2 Launcher (SPEC §6) – **erst ganz zum Schluss**.

## Stolpersteine
- RetroArch headless testen: nur mit eigener Config-Kopie (`--config <scratch>/ra.cfg`, Treiber null), **nie
  `--appendconfig`** – hat am 2026-10-06 die Null-Treiber in die echte `retroarch.cfg` gespeichert.
  (Exit 0 = läuft). Schreibt in `playlists/builtin/content_history.lpl` → Testeinträge danach entfernen.
- quick-xml 0.41: `unescape_value` deprecated → `normalized_value(XmlVersion::Implicit1_0)`.
- DOS/ScummVM-RDBs identifizieren über *eine* Datei, die oft zwischen Spielen geteilt ist (`dosbox.bat`, `ADL.DRV`) → Ordnername statt DB-Name.
- Kompletter Probelauf: `rombro import "<Sammlung>" <scratch-lib> --dry-run` (nur lesend), ~3 h für 1,2 TB.
- Leere Dateien matchen RDB-Einträge mit Leer-Hash (z. B. PSP-DLC) → Scanner ignoriert 0-Byte-Dateien.
- Release: CI erstellt nur einen Draft; Veröffentlichen (`gh release edit --draft=false`) macht der User. CLI-Assets heißen `rombro-<os>-<arch>`.
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
- 2026-10-06: Kompletter Probelauf über echte Sammlung (1,2 TB, 2:44 h). Behoben: Spielordner (DOS/ScummVM/Ports) wandern ganz, Arcade-Duplikate, Multi-Disk-Zips nie teilweise entpacken, Dreamcast-Serials ohne Bindestrich, N64 .v64/.n64, GC/Wii per Spiel-ID, unbekannte Ordner/BIOS-Ordner bleiben liegen.
- 2026-10-05: Echter MAME-Ordner (674 Zips) als Probelauf: Chip-Treffer zerlegten Zips (behoben), `.keep`/`.DS_Store` (behoben), 63 Sets anderer MAME-Version → Quarantäne.
- 2026-10-05: v0.1.0 veröffentlicht; README mit Screenshots (`docs/screenshots/`, Mock-Daten via Headless-Chrome/CDP).
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
- Neue `Rules`-Felder brauchen Defaults im `Default`-Impl (`#[serde(default)]` nimmt die) – alte gespeicherte JSONs bleiben gültig.
- Override-Exclude pro System gibt es nur im Backend/JSON, nicht in der UI.
- Testset: `scripts/make-testset.sh "<Sammlung>"` → `~/rombro-test` (inbox, lib, DB-Kopie); immer mit `--db ~/rombro-test/test.db`, damit Testeinstellungen nicht in der echten DB landen.
