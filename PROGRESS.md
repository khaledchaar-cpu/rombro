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
| M10 Gamification | ✅ done (Sichtprüfung 2026-10-09) |
| M11 Release | ✅ done (CI grün auf 3 OS) |
| M12 Übertragen & Container | ✅ done |
| M12b Arcade & Sichtprüfung | ✅ done |
| M12c Echte Sammlung (Probelauf) | ✅ done |
| M13 Regeln transparent & einstellbar | ✅ done (Rules-Seite abgenommen 2026-10-06; Entscheidungen zeigen Grund + Datum) |
| M14 Arcade-DATs | ✅ done |
| M14b Core-Wahl & -Installation | ✅ done (Sichtprüfung 2026-10-09) |
| v0.3.0 Release + Inbox-Importe | ✅ released 2026-10-06; Nachträge 2026-10-07 (CSO, OpenBOR, Library-Perf) |
| v0.4.0 Release (Perf, Settings-Umbau) | ✅ released 2026-10-07 |
| v0.5.0 Release (Firmware/BIOS, Daphne, Disc-Serials, neue UI) | ✅ released 2026-10-08 |
| v0.5.1 Release (Arcade-DAT-Erkennung, Mehrdisk, Perf) | ✅ released 2026-10-08 |
| M15a Verwaltetes RetroArch | ✅ done (Linux e2e + Sichtprüfung 2026-10-08; Win/macOS ungetestet) |
| M15b Starten | ✅ done (Linux e2e + Sichtprüfung 2026-10-08) |
| M15c Spielzeit & Favoriten | ✅ done (Sichtprüfung + ScummVM-Start 2026-10-08) |
| M15d Altlasten | ✅ done (2026-10-08) |
| M16a RetroAchievements: Hashing + Library-Badge | ✅ done (Sichtprüfung 2026-10-08) |
| M16b RA-Login, Hardcore, Fortschritt | ✅ done (echter Login + Freischaltung + Sichtprüfung 2026-10-09) |
| v0.6.0 Release (M15 Launcher-Basis) | ✅ released 2026-10-08 |
| M16c RA Disc-Systeme + NDS | ✅ done (2026-10-09, gegen rcheevos verifiziert; Sichtprüfung PSP + PS1-.pbp-Hinweis ok) |
| M17 Cover | ✅ done (Sichtprüfung 2026-10-09) |
| M18 Dashboard | ✅ done (Sichtprüfung 2026-10-09) |
| M19 Savestates | ✅ done (Sichtprüfung 2026-10-09) |
| M20 Globale RA-Einstellungen | ✅ done (Sichtprüfung 2026-10-09) |
| M21 Statistik | ✅ done (Sichtprüfung 2026-10-09) |
| v0.8.0 Release (v2 Launcher M17–M21) | ✅ released 2026-10-09 |
| v0.8.1 Release (RDB-Download, README) | ✅ released 2026-10-09 |
| v0.8.2 Release (Atari-ST-Teile, Sufami-Turbo-Zips, CI-e2e Win/macOS) | ✅ released 2026-10-09 |
| M22 Beliebtheit (RA-Spielerzahlen) | ✅ done (Sichtprüfung 2026-10-09) |
| v0.9.0 Release (M22 Beliebtheit) | ✅ released 2026-10-09 |
| M23 UX-Überarbeitung | ✅ gebaut 2026-10-09 (Stichproben User; Details SPEC §6 „M23“) |
| v0.10.0 Release (M23 UX) | ✅ released 2026-10-09 |
| v0.11.0 Release (Doppelten-Check, Import-Fixes) | ✅ getaggt 2026-10-10 |

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
v0.5.1 veröffentlicht 2026-10-08. Nächste Schritte: M15a; Atari-ST-Mehrdisk-Teile (Boot/Data, (A)/(B)) nicht als TIE behandeln;
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
**Nächster Milestone: M15b Starten** (Schnitt M15a/b/c festgelegt 2026-10-08, SPEC §6).

M15a (2026-10-08): `retroarch::managed` (Download → SHA-256 bei gepinnter Version → 7z/dmg entpacken in Temp →
atomarer Rename → `current`-Marker, alte Versionen weg), `managed::config` (eigene `retroarch.cfg`, nur Ordner-Keys
gesetzt, Rest bleibt; `system_directory` = `<library>/_bios`, wird nicht angelegt). CLI `romburak ra status [--check] |
install [--version|--latest] [--library] | config`; App: Panel „Managed RetroArch“ im RetroArch-Tab
(`ra_status`/`ra_install`, `ra://progress` in MB). Streaming-Download `romburak_store::http_download`.
E2E Linux (Scratch-XDG): 179 MB in 1:43 min, 1.22.2 startet headless fceumm 300 Frames, Saves/States im
verwalteten Ordner. Panel inkl. Entpack-Fortschritt vom User abgenommen. Offen: Windows/macOS real testen (dmg via `hdiutil`).
Nächste Schritte M15b: `info`-Dateien + Cores in den verwalteten Ordner (Buildbot `info.zip`), `romburak play`,
Play-Button, Core-Override, Auto-Core-Install.

M15c (2026-10-08): Schema v16 `play_stats` + `favorite`, Schlüssel `sha1:<hex>` der ersten indizierten ROM
(`Store::game_key/game_keys`, Fallback `path:<abs>` z. B. für `.m3u`/`.cue`). Läufe < 30 s zählen nicht
(`MIN_PLAY_SECS`). CLI `play` misst und zeigt Spielzeit; App: `play://ended` aktualisiert die Zeile lokal,
Library-Spalten ★ + „Played“, Filter-Chips „★ favorites“/„played“, Details mit Spielzeit/zuletzt gespielt.
Alter RetroArch-Export entfernt (core `export`, `Dirs`, CLI `retroarch`, App `retroarch_export`, RetroArchPanel).
Ersatz: `romburak ra cores [--set Sys=id]`, Settings → RetroArch „Cores per system“ (verwaltete Cores, Systeme =
Top-Ordner der Library). Core-Systemdateien (blueMSX, PPSSPP, Dolphin, PCSX2, ECWolf) lädt jetzt `play` nach
`download/assets/` und entpackt nach `<library>/_bios` über den Journal (`Store::execute_journaled`, undo-bar).
Geprüft: `play --dry-run` Coleco → bluemsx + 300 Systemdateien geplant.
Sichtprüfung abgenommen (User, 2026-10-08), 2 ScummVM-Titel starten ohne Export-`scummvm.ini`.
M15d (2026-10-08): Planner schreibt keine `.lpl` mehr (CLI `--playlists/--no-playlists`, `plan::lpl`, `Options.playlists`
weg); vorhandene `_playlists/*.lpl` wandern beim nächsten Plan nach `_trash/playlists/`. `retroarch::scummvm` +
`scummvm-engines.tsv` entfernt. `play`: `.m3u` → erste Disk, wenn der Core kein `m3u` kann (FDS/FCEUmm, `Core.extensions`).
Abgenommen (User, 2026-10-08): `.lpl` → `_trash/playlists/` in der echten Library. Fensteranzeige: Omarchy-Override in `~/.config/hypr/hyprland.lua` (`fullscreen = false`). v0.6.0 getaggt. Nächster Milestone (User wählt): 
Savestate-Übersicht oder RetroAchievements (SPEC §6).

M16a (2026-10-08): `core::cheevos` (rcheevos-Hashing, `consoles.tsv` RDB-System → RA-Konsole + Methode, `title_key`),
Store v17 `ra_console/ra_game/ra_hash/ra_file` (`ra_sync` mit Pause + Retry bei 429, `ra_hash` cached nach size+mtime),
CLI `romburak cheevos key|sync|scan`, App: Settings → Databases „RetroAchievements“ (Key, Sync + Hashing mit Fortschritt),
Library-Spalte 🏆 (◌ = andere Version unterstützt), Filter-Chip, Details. Echte Library: 41 Konsolen, 7.737 Spiele,
2.685/9.083 Dateien mit Achievements (erster Hash-Lauf 1:27 min über CIFS). Niedrige Quoten (N64 35/193) = EU-Versionen,
die RA nicht führt (geprüft: Mario Kart 64 EU Rev 1 nicht in RA, Dr. Mario 64 `.n64` erkannt).
Nächste Schritte: M16b (Login → `cheevos_token`, Hardcore-Schalter, Fortschritt je Spiel).

M16b (2026-10-08): `retroarch::managed::cheevos::Cheevos` (Keys `cheevos_enable/username/token/hardcore_mode_enable`,
`cheevos_password` immer leer), `Managed.cheevos`; Store `ra_account` (Settings `ra.user/ra.token/ra.hardcore`,
`ra_login` = POST `dorequest.php?r=login2`, 401 = falsches Passwort), `Store::ra_prefs(m)` setzt Display + Cheevos
überall, wo die Config geschrieben wird. Schema v18 `ra_progress` (`API_GetUserCompletionProgress`, 500/Seite) –
bei Sync, Login und nach gespielten RA-Spielen (nur betroffene Zeilen gepatcht). Library-Zelle `3/40` bzw. `★ 40`
(mastered/completed), Details: „x of y unlocked“, Liste via `API_GetGameInfoAndUserProgress` mit ✔/✔ hardcore,
gesperrte grau. CLI `cheevos login <user>` (Passwort per TTY oder stdin), `logout`, `hardcore on|off`.
Getestet 2026-10-09: echter Login, Freischaltung in RetroArch (Metal Slug 4, FBNeo), Sichtprüfung. Fix: Liste lädt neu, wenn sich die Zahl der Freischaltungen ändert.
Nächste Schritte: M16c (Disc-Systeme + NDS hashen).

M16c (2026-10-09): `cheevos::cd` (Tracks nach Nummer/erster Daten-/letzter Track, absolute LBA aus Raw-Header bzw.
`.gdi`, cue mit `INDEX 01`-Offset, ISO9660-Suche wie `rc_cd_find_file_sector`), `cheevos::disc` (PS1, PS2, PSP inkl.
`.pbp` ganz, Sega CD/Saturn, PCE-CD, Dreamcast, 3DO, NDS/DSi); `ChdTrack::open_number/track_list`.
`library_files` liefert bei Disc-Systemen jede Disc (auch in Mehrdisk-Ordnern), keine Track-Dateien eines Sheets.
CLI `cheevos scan` nutzt jetzt `library_files`. Verifiziert: alle cue/iso/nds/PSP-ISO der echten Library bit-identisch
zu rcheevos (Referenz-Binary, s. CLAUDE.md), CSO entpackt identisch. Echte Library: PCE-CD 10/22, DC 4/18, Sega CD 5/10,
3DO 5/35, NDS 20/67, PSP 27/111, Saturn 0/8 (RA führt diese Dumps nicht, Hashes = rcheevos). Nicht unterstützt:
PS1-`.pbp` (rcheevos auch nicht), PCE-GameExpress (BOOT.BIN), PC-FX, Neo Geo CD, Jaguar CD, Discs in Zips.
Sichtprüfung 2026-10-09 (PSP ok). PS1-`.pbp` (RetroArch erkennt sie auch nicht): Library zeigt `⊘` + Hinweis „convert to CHD or CUE/BIN“ (`cheevosUnhashable`); automatische Umwandlung bewusst nicht gebaut.
Doppelte Spielordner (2026-10-09): gleicher Titel (System + RDB-Name) in zwei Spielordnern → TBD „Duplicate“ mit
behaltenem Ordner (Library vor Inbox, mehr Dateien, Pfad; `plan::build::dup_folders`); Discard trasht den ganzen Ordner
nach `_trash/<Ordner>/`. Echt: `Quake/quake` verworfen (Journal #93), `Quake/tyrquake` behalten. Loses
`Quake/tyrquake/pak0.pak` vom User entfernt.
Umbenennung ROMBRO → Romburak (2026-10-09): Crates `romburak-*`, CLI `romburak`, GitHub-Repo `romburak`
(alte URLs leiten weiter). Datenordner `<data>/rombro` + `<cache>/rombro` werden beim ersten Start verschoben,
`rombro.db` → `romburak.db`, Pfade in verwalteter `retroarch.cfg` und `.lpl` umgeschrieben (`paths::legacy`; echte
Daten migriert). Bewusst unverändert: Tauri-ID `dev.rombro.app`, localStorage-Keys `rombro.*`, `~/rombro-test`.
Nächste Schritte: v0.6.0 (M15–M16) releasen.

## Nächste Schritte
**Als Nächstes (User, 2026-10-09), nacheinander autonom:** (a) ✅ 2026-10-09: CI-Job `e2e-retroarch` (`scripts/e2e-ra.sh`, Windows grün: install/status, fceumm, synthetische NES-ROM 120 Frames headless) – war: Windows-CI-e2e-Job für verwaltetes RetroArch
(ra install/status, Core + synthetische Test-ROM headless) bis grün; (b) ✅ 2026-10-09: Atari-ST-Teile `(Boot)`, `(Intro)`, `(Game Disk)`, `(Data Disk…)`, `[Disk 1 and 2]` = ein Release
(`naming::release_name`, Reihenfolge `media_rank`); echte Library unverändert (Dry-Run); (c) ✅ 2026-10-09: Sufami-Turbo-Kombi-Images in Zips → Member `<image>/<Slot A.st>` (`scan_zip`, `archive::write_member`);
mit Kopie eines echten Kombi-Images gezippt verifiziert (Scan + Import). In der echten Sammlung gibt es keine solchen Zips. Wikipedia-Ratings: verworfen.
Stand 2026-10-09 (alte Einträge erledigt, Details in der Git-Historie dieser Datei):
1. v0.8.1 (RDB-Download vom Buildbot, README neu) getaggt 2026-10-09; Claude veröffentlicht nach CI. (v0.8.0 = M17–M21.) (v0.7.0 = M16 + Umbenennung.)
2. ✅ 2026-10-09: verwaltetes RetroArch per CI-e2e auf Windows + macOS-14 (arm64) grün; echte Bedienung dort gestrichen (User).
3. v2 Launcher geklärt (Grilling 2026-10-09, SPEC §6): M17 Cover → M18 Dashboard → M19 Savestates → M20 globale RA-Einstellungen → M21 Statistik. M17 gebaut: `thumb://`-Protokoll (`thumbs::protocol`, Browser lädt lazy + cached), unscharfer Abgleich über die
   Server-Verzeichnisliste (`thumbnail::best_match`, Liste 30 Tage in `.index`), Library Liste ↔ Grid (`rombro.libraryView`),
   Mini-Boxart in der Liste. Sichtprüfung ok.
   M18 gebaut: Dashboard „Continue playing“ (`DashboardContinue`, zuletzt gespielt, dann ungespielte Favoriten, max. 12,
   Klick startet), Library-Sortauswahl (auch Grid): System/Name/zuletzt gespielt/Spielzeit/neu/Favoriten. Sichtprüfung ok.
   M19 gebaut: `retroarch::states` (alle Core-Ordner unter `states/`, Slot/Auto, `.png`), Details → Savestates mit
   Screenshot (`thumb://localhost/state/<pfad>`, nur aus dem States-Ordner), Löschen mit Inline-Bestätigung, Start per
   `--entryslot=N` (vor der ROM) mit dem Core des States. Auto-State nicht per Slot startbar. Sichtprüfung ok.
   M20 gebaut: `managed::video` – Shader global als `<root>/config/global.slangp` (`#reference`, auf Linux `video_driver = glcore`,
   echt verifiziert: RetroArch lädt + kompiliert), Seitenverhältnis (`aspect_ratio_index`), Shader-/Autoconfig-Ordner aus dem
   RA-Bundle, sonst Buildbot-Paket (`ensure_package`; Autoconfig beim Start). `rgui_config_directory` = `<root>/config`, alter
   Ordner wird einmalig kopiert (ohne Presets). UI Settings → RetroArch → Picture; CLI `ra shader`/`ra aspect`.
   Live-Shaderwechsel: `stdin_cmd_enable`, App hält stdin des Kinds (`live.rs`), `SET_SHADER <preset>` / leer = aus
   (echt verifiziert Linux; Windows-stdin ungetestet). Linux immer `video_driver = glcore`.
   Sichtprüfung ok (inkl. Live-Wechsel).
   M21 gebaut: Schema v19 `play_session` (je gezähltem Lauf; Altbestand = 1 Lauf je Spiel am letzten Spieltag), Seite „Stats“
   (Ctrl+7): Übersicht, Verlauf 7/30 Tage/12 Monate (lokale Zeit), meistgespielt, Zeit je System, Achievements.
   Sichtprüfung ok. v2 Launcher (M17–M21) komplett.

M23 (2026-10-09): Blättern statt Scrollen (`components/Pager.tsx`: `createPaged` mit Anker-Item, `fitCount`
misst live), Library-Filter Mehrfachauswahl + Jahrzehnte (`year` aus RDB), Suche Name+System inkl. Kürzel,
Library-Pfad/Needs attention/Trash → Settings → Library, Details mit Reitern, Stats im Dashboard aufgegangen,
Dashboard-Einträge springen in die Library (`state/jump.ts`), Graph „Library growth“, Core-Override am Inhalt.
Nachträge 2026-10-09: Dashboard in Reiter Play/Stats/Collection/Goals (jede Seite passt ohne Scrollen, Stats-Listen 5 je Seite); Library-Zeilen tragen `franchise` (`Store::release_meta`), Filter „franchise“, Franchise-Ziele springen dorthin; Gamification-Sprünge filtern auf `known` (gleiche Basis wie die Zahlen). Rest-Abweichung gewollt: Completeness/Franchise zählen 1G1R-Gruppen, die Library jede Fassung (Tooltip). Offen: Sichtprüfung User, dann v0.10.0.

## Stolpersteine
- Chrome-Tab im Hintergrund (`visibilityState: hidden`): kein rAF, Screenshots hängen – Tests per JS, `fitCount` misst erst sichtbar.
- RetroArch headless testen: nur mit eigener Config-Kopie (`--config <scratch>/ra.cfg`, Treiber null), **nie
  `--appendconfig`** – hat am 2026-10-06 die Null-Treiber in die echte `retroarch.cfg` gespeichert.
  (Exit 0 = läuft). Schreibt in `playlists/builtin/content_history.lpl` → Testeinträge danach entfernen.
- quick-xml 0.41: `unescape_value` deprecated → `normalized_value(XmlVersion::Implicit1_0)`.
- DOS/ScummVM-RDBs identifizieren über *eine* Datei, die oft zwischen Spielen geteilt ist (`dosbox.bat`, `ADL.DRV`) → Ordnername statt DB-Name.
- Kompletter Probelauf: `romburak import "<Sammlung>" <scratch-lib> --dry-run` (nur lesend), ~3 h für 1,2 TB.
- Leere Dateien matchen RDB-Einträge mit Leer-Hash (z. B. PSP-DLC) → Scanner ignoriert 0-Byte-Dateien.
- Release: CI erstellt nur einen Draft; Veröffentlichen (`gh release edit --draft=false --latest`) macht Claude selbst. CLI-Assets heißen `romburak-<os>-<arch>`.
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
- 2026-10-05: Öffentliches Repo github.com/khaledchaar-cpu/romburak, CI grün (Linux/macOS/Windows). Commit-Mail = GitHub-noreply.
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

M15b (2026-10-08): `retroarch::managed::launch` – `Managed::prepare` installiert bei Bedarf RetroArch (gepinnt),
Info-Dateien (`assets/frontend/info.zip` → `<root>/info`) und den Core (`nightly/<os>/<arch>/latest/<core>.zip` →
`<root>/cores`, atomar), schreibt die Config und baut `retroarch --config … -L core rom`. System = oberster Ordner
unter der Library (sonst nächster Elternordner, den ein Core kennt). Core: Override pro Spiel (Setting `core:<pfad>`,
`Store::core_override`) > Systemwahl/Empfehlung > spezialisiertester Core. CLI `romburak play <file> [--core id [--save]]
[--dry-run]`; App: `game_cores`/`set_game_core`/`play` (`play://progress`), Komponente `GamePlay` in der Detailansicht.
E2E Linux (Scratch-XDG): leerer Ordner → RetroArch + Info + fceumm in 1:45 min, Befehl korrekt.
Play-Button abgenommen; Cores nur aus Buildbot-Index (`cores.index`). Core-Assets (`assets::missing`) werden beim Start geladen und journaled nach `_bios` entpackt (5f35a85; 2026-10-09 + DirkSimple, QEMU).
Override hängt seit 2026-10-09 am Inhalt (`core:sha1:…`, alter Pfad-Schlüssel wird noch gelesen).
Nächste Schritte M15c: Spielzeit (Child in `play` abwarten, < 30 s ignorieren, `play_stats` über Hash), Favoriten,
alten Export (Playlists, `retroarch`-CLI, CorePicker mit System-RetroArch) entfernen.
