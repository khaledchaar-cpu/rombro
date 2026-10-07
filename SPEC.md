# ROMBRO – Spezifikation

> Lebendes Dokument. Wird bei jeder neuen Erkenntnis aktualisiert (siehe CLAUDE.md → Workflow).
> Stand: 2026-10-04 · Version: v1 (Curator) · v2 (Launcher) ist Ausblick.

## 1. Vision
ROMBRO liest alle RetroArch-Datenbanken (`.rdb`) aus, verifiziert neue ROM-Dateien per Hash/Serial und importiert sie
in eine verwaltete Verzeichnisstruktur. Ergebnis: eine saubere, dublettenfreie Sammlung nach **1G1R** (One Game – One ROM).
Optional: Gamification (Vollständigkeit, KPIs, Achievements). v2: vollwertiger Launcher (RetroArch-Cores).

## 2. Kernbegriffe
| Begriff | Bedeutung |
|---|---|
| **RDB** | RetroArch-Datenbank: Header `RARCHDB\0` + u64 Offset, danach MessagePack-Maps pro Eintrag |
| **Entry** | Ein RDB-Datensatz (name, rom_name, size, crc, md5, sha1, serial, region, Metadaten) |
| **Game** | Gruppe von Entries, die dasselbe Spiel darstellen (Regionen, Revisionen, Sprachen) |
| **1G1R-Pick** | Der eine bevorzugte Entry pro Game nach User-Prioritäten |
| **Library** | Verwaltetes Zielverzeichnis (Single Source of Truth im Filesystem) |
| **Inbox** | Beliebige Quellverzeichnisse mit neuen/unsortierten Dateien |
| **Plan** | Berechnete Liste von Datei-Operationen (Dry-Run), erst nach Bestätigung ausgeführt |

## 3. Fakten zu den RDB-Daten (verifiziert)
- Ort: `~/.config/retroarch/database/rdb/*.rdb` (Linux), 146 Dateien, ~150 MB. Pfad aus `retroarch.cfg` → `content_database_path`.
- Layout: Header (`RARCHDB\0` + **big-endian** u64 Offset auf Metadaten), Einträge bis `nil`, dann `{"count": n}`.
- Ein Eintrag = eine MessagePack-Map, Keys als Strings; `crc`/`md5`/`sha1` als **Binärdaten** (bin8), nicht Hex-Strings.
- Felder: `name, description, rom_name, size, crc, md5, sha1, serial, region, releaseyear, releasemonth, genre,
  developer, publisher, franchise, users, esrb_rating, edge_rating, edge_issue, elspa_rating, rumble, analog,
  enhancement_hw, origin` (nicht alle immer gesetzt).
- SNES: 7762 Einträge, Hashes nahezu vollständig. PS1: 13524 Einträge, **jeder** mit `serial`, Hashes pro Track.
- Manche Einträge sind reine Metadaten (nur serial/crc ohne name) → beim Laden mergen bzw. separat behandeln.
- Keine Parent/Clone-Info in RDB → Game-Gruppierung muss aus den No-Intro/Redump-Namen abgeleitet werden.
- Name-Konvention: `Titel (Region) (Sprachen) (Rev X) (Flags)`, z. B. `(USA)`, `(Europe) (En,Fr,De)`, `(Beta)`, `(Proto)`, `(Unl)`, `[b]`.

## 4. Funktionsumfang v1
### F1 – Datenbank-Engine
- RDB-Parser (eigener, zero-copy MessagePack-Leser, kein externer Crate nötig) → In-Memory-Index + persistenter Cache.
- Indizes: `crc+size`, `sha1`, `md5`, `serial` → Entry; `system` → Entries.
- Inkrementell: Re-Import nur bei geänderter mtime/Größe der `.rdb`.
- Ziel: alle 146 RDBs < 2 s kalt, < 100 ms warm (Cache).

### F2 – Scanner & Verifikation
- Rekursiver Scan der Inbox(es), parallel (rayon), Streaming-Hashing (CRC32 + SHA1 in einem Durchlauf).
- Container: `.zip`, `.7z` (Inhalt hashen; bei ZIP CRC aus Header als Schnelltest), später `.chd` (SHA1 aus Header).
- Header-Handling: iNES/FDS/A7800/Lynx-Header strippen und beide Varianten prüfen; SNES-Copier-Header (512 B).
- Disc-Images: `.cue/.bin`, `.gdi`, `.iso`, `.m3u`; Serial aus Disc lesen (PS1/PS2/Saturn/SegaCD/PSP).
- System-Erkennung: primär per Hash-Treffer; Fallback Dateiendung + Ordnername.
- Ergebnis je Datei: `Verified(entry)` | `Unknown` | `BadDump/Hack (per Name-Flags)` | `Duplicate(of)`.

### F3 – 1G1R-Engine
- Gruppierung: normalisierter Titel (Tags entfernt, Artikel/Satzzeichen vereinheitlicht) + System.
- Optional: echte Parent/Clone-Infos aus No-Intro-DATs (Import) → überschreibt Heuristik.
- Scoring nach konfigurierbaren Prioritäten: Regionen (Default EU > World > USA > DE > JP), Sprachen, neueste Rev,
  Ausschlüsse (Beta, Proto, Demo, Kiosk, Unl, Pirate, BIOS, Virtual Console …).
- Ausgabe: pro Game genau ein Pick + Liste der verworfenen Kandidaten mit Begründung.

### F4 – Library & Import
- Zielstruktur (Standard, konfigurierbar per Template):
  ```
  <library>/<System>/<Name>.<ext>                       # Cartridge
  <library>/<System>/<Name>/<Name> (Disc N).<ext> + .m3u # Multi-Disc
  <library>/_quarantine/<System>/...                     # Unknown/BadDump
  ```
- Dateinamen = RDB-`name` (Thumbnail-kompatibel: `&*/:<>?\|` → `_`).
- Operationen: move | copy; optional (ent)zippen. (hardlink/reflink 2026-10-07 entfernt)
- Immer: Plan → Dry-Run-Anzeige → Ausführen. Jede Ausführung schreibt ein **Journal** → Undo möglich.
- Ports (2026-10-07): ROMBRO verwaltet nur RetroArch-relevante Titel. Ports mit libretro-Core, aber ohne RDB,
  erkennt es an der Datei, die der Core lädt (`.info` `supported_extensions`, Tabelle `plan::PORTS`, z. B. `smw.game`),
  und verschiebt sie als Spiele-Ordner. Standalone-Ports ohne Core (Batocera: xash3d, devilutionx, cdogs) bleiben
  unerkannt und landen über „Inbox-Reste“ im Trash.
- Daphne (2026-10-07): Ein Ordner mit `roms/` und `*.daphne/` ist eine Laserdisc-Sammlung; sie wandert komplett
  (Struktur unverändert) nach `Daphne/`, weil der Core `roms/<spiel>.zip` lädt und Video/Framefile aus
  `../<spiel>.daphne/` liest. Playlist `Daphne.lpl`: nur Spiele mit `.daphne`-Ordner. Core `daphne`.
- Arcade-BIOS (2026-10-07): Neu gepackte BIOS-Zips (`neogeo.zip`, `stvbios.zip`) treffen nie den Ganzdatei-Hash;
  ein Zip, dessen Name im DAT ein `isbios`-Set ist und dessen Member zu ≥ 75 % passen, gilt als BIOS des
  bestplatzierten Cores. Der Library-`_trash` wird nicht gescannt.
- Library-Snapshot (2026-10-07): RomBro verwaltet die Library allein, daher gilt der Index als Wahrheit.
  Die identifizierte Library (Items + Arcade-Setnamen) liegt als Snapshot in der DB (`snapshot`, gzip-JSON).
  Plan-Import und Library-View laden ihn (~60 ms) ohne Ordnerdurchlauf und ohne DB-Lookups. Verworfen wird er
  per Trigger bei RDB-/DAT-Sync und Resolutions sowie bei Index-Änderungen unter der Library (Execute/Undo,
  geänderte Scans); danach baut ihn ein Trusted-Scan neu auf. Wartung: „Rescan library“ prüft jede Datei auf der Platte.
- Library-Audit: bestehende Library prüfen, falsch benannte/doppelte/nicht-1G1R-Dateien finden und Plan erzeugen.
- Export: RetroArch-Playlists (`.lpl`, JSON) pro System, inkl. CRC → sofort nutzbar in RetroArch.

### F5 – GUI
- Dashboard: Systeme als Kacheln, Vollständigkeit, letzte Imports.
- Import-View (Inbox + Plan in einem Tab, 2026-10-06): Ordner/Modus oben, darunter Scan-Fortschritt, Plan-Vorschau als Diff, Entscheidungen, Inbox-Reste.
- Navigation (2026-10-07): vertikale Sidebar (einklappbar): Dashboard, Import, Library, RetroArch (Export; v2 Launcher),
  Regeln (eine Regel pro Unterseite), System (Datenbanken, Darstellung). Themes: Neon, Neon light, Pin-up '40s
  (Farben/Fonts in tokens.css, Formen in pinup.css).
- Library-View: virtualisierte Tabelle (100k+ Zeilen flüssig), Detail-Panel mit Metadaten + Thumbnail.
- 1G1R-Regeln-Editor (Drag&Drop Prioritäten).
- Command-Palette (Ctrl/Cmd+K), vollständige Tastaturbedienung.

### F6 – Gamification (optional, abschaltbar)
- Vollständigkeit pro System (gegen 1G1R-Set, nicht gegen alle Varianten).
- KPIs: Anzahl Games, verifizierte Quote, Dubletten entfernt, gesparter Speicher, Regionen-Mix, Genres, Jahrzehnte.
- Achievements (z. B. „Full Set: Virtual Boy“, „Clean Sweep: 0 Unknowns“), XP/Level, Streaks.
- Optionale Ziele: Franchise komplettieren (Feld `franchise`).
  Entscheidung: gezählt über die 1G1R-Sets der Systeme, auf denen man schon Spiele besitzt; Ziel erst ab 1 eigenem und ≥ 3 Spielen.

### F7 – Thumbnails (nice to have v1)
- Download von `thumbnails.libretro.com` (Boxart/Snap/Title), lokaler Cache, Namensmapping wie RetroArch.

### F8 – Arcade-DAT-Prüfung (M14)
Grundsatz (User, 2026-10-06): **RDBs bestimmen, ob ein Set in die Sammlung kommt; DATs helfen, das richtige System zu bestimmen.**
- Anlass: `MAME.rdb` mischt Set-Versionen (52.617 Einträge / 37.798 Namen) → alte Sets (z. B. `1943`, 0.78-Format)
  landeten in `MAME/` und starten im aktuellen MAME-Core nicht.
- DATs (Member-Listen: Name/Größe/CRC je Datei im Zip) für alle 9 Arcade-Cores, `db sync` lädt automatisch die
  jeweils neueste Version aus den libretro-Quellen; offline → zuletzt geladene + Warnung; ohne DATs → alte Logik.
- Ohne RDB-Treffer kein Eintritt (bleibt Quarantäne, auch bei DAT-Vollständigkeit).
- DAT-Prüfung nur bei unsicheren Treffern: nur `MAME.rdb` oder mehrere Arcade-Cores. Exakte Treffer in versionierten
  DBs gelten ungeprüft (bewusst: split-Clones ohne Eltern-Set werden dort nicht erkannt).
- Ziel = erster Core in `arcade_order`, für den RDB-Treffer **und** DAT-Vollständigkeit gelten (Prüfung über
  CRCs aus dem Zip-Verzeichnis, ohne Entpacken; split/merged/non-merged, split-Clones brauchen Eltern-Set).
- Kein solcher Core → Quarantäne mit Begründung (fehlend/falsch benannt/Eltern fehlt) in Dry-Run, Audit, App.
- Quarantäne wird nicht erneut geprüft; keine Reparatur (Umbenennen/Rebuild) in M14.

## 5. Nicht-Ziele v1
Emulation/Start von Spielen, Cloud-Sync, ScreenScraper-Integration, Netplay.

## 6. v2-Ausblick – Launcher
Core-Erkennung (`*.info`), Start via RetroArch-CLI (`retroarch -L core rom`), Spielzeit-Tracking, Favoriten,
Controller-Navigation (Gamepad-UI-Modus), Savestate-Übersicht, RetroAchievements-Status.
Architektur v1 muss das vorbereiten: GUI-Navigation per Fokus-System, Daten-Modell mit `play_stats`-Tabelle reservieren.

## 7. Architektur
```
rombro/
├─ crates/
│  ├─ rdb/        # RDB/MessagePack-Parser, no deps, zero-copy        (lib)
│  ├─ core/       # Domain: Hashing, Scanner, Naming, 1G1R, Planner    (lib)
│  ├─ store/      # Persistenz: SQLite (rusqlite, WAL), Migrations     (lib)
│  ├─ cli/        # `rombro` CLI – headless, für Tests & Power-User     (bin)
│  └─ app/        # Tauri-2-Shell, Commands/Events → core              (bin)
├─ ui/            # SolidJS + TypeScript + Vite, Cyberpunk-Design-System
├─ .claude/skills/  # projektspezifische Skills
└─ CLAUDE.md · SPEC.md · PROGRESS.md
```
**Stack-Entscheidungen**
| Bereich | Wahl | Begründung |
|---|---|---|
| Sprache Core | Rust (edition 2024) | Performance, Cross-Platform, Sicherheit |
| GUI-Shell | Tauri 2 | klein, nativ auf macOS/Linux/Windows, Rust-Backend |
| Frontend | SolidJS + TS + Vite | feingranulare Reaktivität, kein VDOM, sehr schnell |
| Styling | Vanilla CSS + Custom Properties | keine Runtime, volle Kontrolle für Cyberpunk-Effekte |
| Tabellen | eigene Virtualisierung (TanStack Virtual) | 100k+ Zeilen |
| Persistenz | SQLite (rusqlite, bundled) | robust, einzelne Datei, schnelle Queries |
| Parallelität | rayon (CPU), crossbeam-channel (Streaming-Events) | |
| Hashing | crc32fast, sha1 (asm), md-5 | SIMD |
| Archive | zip, sevenz-rust2 | |
| Fehler | thiserror (libs), anyhow (bins) | |
| Logging | tracing | |
| Tests | cargo test + insta (Snapshots), Fixtures unter `tests/fixtures` | |

**Prinzipien:** Core ist GUI-agnostisch; CLI und App sind dünne Adapter. Alle Dateioperationen laufen über den
Planner (nie direkt). Lange Jobs streamen Fortschritt über Events. Keine Blockierung des UI-Threads.

**Daten-Modell (SQLite, Entwurf)**
`rdb_source(id, path, mtime, size)` · `entry(id, system, name, rom_name, size, crc, md5, sha1, serial, region, meta_json)` ·
`game(id, system, norm_title)` · `entry_game(entry_id, game_id)` · `file(id, path, size, mtime, crc, sha1, entry_id, status)` ·
`journal(id, ts, plan_json, state)` · `settings(key, value)` · `achievement(id, unlocked_at)` · `play_stats` (v2, reserviert).

## 8. UI/UX – Cyberpunk Design System
- Dunkel als Default (Light-Theme optional), Hintergrund #07070d, Neon-Akzente: Cyan #00f0ff, Magenta #ff2bd6, Gelb #f5ff3b, Grün #39ff88 (OK), Rot #ff3b5c (Fehler).
- Fonts: „Orbitron“/„Rajdhani“ für Headlines, „JetBrains Mono“ für Daten/Hashes.
- Effekte sparsam & abschaltbar (`prefers-reduced-motion`): Glow, Scanlines, Glitch bei Statuswechsel, abgeschrägte Ecken (clip-path).
- HUD-Ästhetik: Rahmen mit Eck-Markern, Terminal-artige Log-Streams, Fortschrittsbalken als Segmentanzeigen.
- UX: Dry-Run immer sichtbar vor destruktiven Aktionen, Undo überall, Tastatur zuerst, < 100 ms Reaktionszeit.

## 9. Performance-Ziele
| Vorgang | Ziel |
|---|---|
| Kaltstart App | < 1 s bis interaktiv |
| Alle RDBs laden (warm, Cache) | < 100 ms |
| Scan 10k Cartridge-ROMs (SSD) | < 10 s |
| Hash-Durchsatz | ≥ I/O-Limit (≥ 1 GB/s auf NVMe) |
| UI-Scroll 100k Zeilen | 60 fps |

## 10. Milestones
Jeder Milestone ist so geschnitten, dass er in **einer Session** abschließbar ist. Status in PROGRESS.md.

| # | Milestone | Inhalt | Done wenn |
|---|---|---|---|
| M0 | Bootstrap | Cargo-Workspace, Crates-Skelett, CI-Skript (fmt/clippy/test), git | `cargo test` grün |
| M1 | RDB-Parser | `crates/rdb`: MessagePack-Leser, Entry-Struct, Laden aller 146 RDBs, Benchmark | CLI `rombro db stats` listet Systeme+Counts |
| M2 | Store & Index | SQLite-Schema, Migrations, Import/Cache der RDBs, Lookup-API | Lookup per crc/sha1/serial in < 1 ms |
| M3 | Scanner & Hashing | Paralleler Scan, Hashing, ZIP/7z, Header-Stripping, Match | `rombro scan <dir>` zeigt Verified/Unknown |
| M4 | Disc-Support | cue/bin, gdi, iso, m3u, Serial-Extraktion PS1/PS2/PSP/Saturn | Disc-Fixtures werden erkannt |
| M5 | Naming & 1G1R | Tag-Parser für No-Intro-Namen, Gruppierung, Scoring, Regeln-Config | Snapshot-Tests für Picks |
| M6 | Planner & Import | Plan/Dry-Run/Execute, Journal, Undo, Quarantäne, Audit, `.lpl`-Export | `rombro import --dry-run` + Undo getestet |
| M7 | App-Shell | Tauri 2 + SolidJS-Gerüst, Design-Tokens, Layout, Command-Palette, IPC | App startet auf Linux mit Dashboard-Dummy |
| M8 | GUI Kern | Dashboard, Inbox-View (Streaming), Plan-Diff, Library-Tabelle | Import-Flow komplett per GUI |
| M9 | GUI Feinschliff | Persistenter Library-Index (§10 M9), 1G1R-Regeln-Editor, Settings, Thumbnails, Effekte, Light-Theme | UX-Review bestanden |
| M10 | Gamification | KPIs, Vollständigkeit, Achievements, XP | Dashboard zeigt KPIs |
| M11 | Release | Packaging (AppImage/deb, dmg, msi), Pfad-Erkennung je OS, Doku | Builds für 3 OS via CI |
| M14 ✅ | Arcade-DATs | F8: DAT-Download/Import, Member-Prüfung, Systemwahl, Quarantäne-Begründung | 1943 → MAME 2003-Plus, startet in RetroArch |
| M14b ✅ | Core-Wahl & -Installation | Empfehlung pro System, Auswahl (Regeln), Download fehlender Cores beim Export | 4 Cores in Scratch-RA installiert, Undo entfernt sie |

## 11a. Entscheidungen
- M2: Persistenter Cache = SQLite statt eigenem Binärformat. Kalt-Import ~3 s (einmalig, SQLite-Insert-bound),
  warm 2 ms. Ziel „< 2 s kalt" gilt für das Parsen (52 ms); Import ggf. später im Hintergrund.
- M2: Systemname = Dateiname der RDB ohne `.rdb`. Metadaten-only-Einträge ohne Treffer werden verworfen.
- M2: DB-Pfad Default `$XDG_DATA_HOME/rombro/rombro.db` (Linux), Felder als Spalten statt `meta_json`.

- M3: MD5 wird beim Scan nicht berechnet (4× langsamer); Verifikation über CRC+Größe → SHA1. Header-ROMs werden
  zuerst headerless gematcht. Matching lebt in `rombro-store` (`identify`), Store → Core-Abhängigkeit.
- M3: ZIP-Header-CRC als Schnelltest entfällt vorerst – SHA1 wird ohnehin gebraucht.

- M4: Disc-RDBs (PS1, SegaCD, Saturn, DC) listen **nur den Datentrack** (Track 1 bzw. 3) – Disc gilt als
  verifiziert, sobald irgendein Track per Hash trifft. Sonst Serial-Fallback (`DiscMatch::Serial`, schwächer).
- M4: Serial-Normalisierung `SLUS_005.94` → `SLUS-00594`; Sega `MK-81020` → zusätzlich `81020`; Multi-Disc-
  Serials in der RDB teils mit Suffix (`SCUS-94163-0`) → Prefix-Fallback. Serials sind in der RDB nicht eindeutig.
- M4: Sektorlayout per Sync-Pattern erkannt (2048 cooked / 2352 raw Mode1 bzw. Mode2-XA), nicht aus dem Cue.
  Track-Dateien eines Sheets und `.m3u` werden nicht als lose ROMs gemeldet. Discs in ZIP/7z: noch nicht.
- **Mehrdeutige Zuordnungen (User-Wunsch):** Liefert ein Treffer (Hash oder Serial) mehrere verschiedene Spiele
  (`store::candidates` > 1, dedupliziert nach System+Name), wird nie still der erste genommen. Status „ambiguous“
  + Kandidatenliste. CLI zeigt `AMBIG` mit allen Kandidaten. M6: Planner importiert Ambiguous nicht automatisch,
  sondern legt eine offene Entscheidung an; Wahl des Users wird persistiert (Tabelle `resolution`:
  Datei-SHA1 → entry_id) und bei erneutem Scan wiederverwendet. M8: GUI-Ansicht „Zu klären“ mit Kandidaten.

- M5: 1G1R-Score (lexikografisch): Region → bevorzugte Sprache (Default En, De) → Varianten-Malus (Alt, Digital/
  Neuauflage/Edition, Virtual Console, Aftermarket) → neueste Rev/Version → mehr Sprachen. Hart ausgeschlossen
  (Default): Beta, Proto, Demo, Kiosk, Sample, Unl, Pirate, BIOS, Hack, Übersetzung, Bad Dump.
- M5: Gleichstand nach allen Kriterien → `needs_decision` (User entscheidet, wie bei mehrdeutigen Treffern).
  Gleichnamige Einträge (Reprint mit identischem Dump, andere Serial) → `Reason::Duplicate`, nur einer gewählt.
- M5: Gruppierung rein heuristisch über `group_key` (Artikel/Satzzeichen); Titel, die sich regional unterscheiden
  (z. B. „Street Fighter II - The World Warrior (Japan)“), landen in eigenen Gruppen → Parent/Clone-DATs später.
- M5: Multi-Disc-Release = Name ohne `(Disc|Disk|Side N)`; alle Discs des Picks werden zusammen gewählt.
- M6: Import = Library + Inbox gemeinsam scannen und planen (Audit = Import ohne Inbox). Library-Dateien werden
  immer verschoben; Inbox per Modus move/copy. Von 1G1R verworfene Releases
  (auch Beta/Hack etc.) werden **nicht** getrasht (User, 2026-10-05), sondern bleiben liegen und landen in der
  **TBD-Queue** (`Decision::Rejected` mit Grund + gewähltem Release); der User entscheidet später (M8: GUI).
  Unbekannte → `_quarantine/<Dateiname>` (System unbekannt). Tie/Ambiguous/Konflikt/Rejected → Item bleibt unangetastet.
- M6: Discs: Tracks werden umbenannt (`<Name>.bin` bzw. `<Name> (Track N).bin`), Cue/GDI-Text wird angepasst
  (Write-Op mit gesichertem Altinhalt). Multi-Disc → Ordner + `.m3u`.
- M12: Multi-ROM-Archive (zip/7z): jedes ROM ist ein eigenes Item und wird per `Op::Extract` entpackt in die Library
  gelegt (Quarantäne ebenso, Discard = nicht entpacken). Sind alle Member erledigt und ist das Archiv in der Library
  oder Modus=Move, wandert das Archiv nach `_trash/` (undo-bar); bei offenen Entscheidungen bleibt es liegen.
- M12: Discs in Archiven: Archiv mit `.cue`/`.gdi` = eine Disc (`Files::ArchivedSheet`), Erkennung nur per Track-Hash
  (kein Serial-Fallback), Tracks werden umbenannt entpackt, Sheet neu geschrieben. Mehrere Sheets im Archiv → Skip.
- M12: CHD (`DiscKind::Chd`, Crate `chd`): erster Nicht-Audio-Track wird ohne Subcode gestreamt (`disc::chd::ChdTrack`,
  Read+Seek) → Hash + Serial wie bei `.bin`. CHDs werden unverändert als `<Name>.chd` abgelegt; keine Konvertierung.
- M6: Playlists `.lpl` (v1.5, `core_path: DETECT`) nach `<library>/_playlists/<System>.lpl` (konfigurierbar).
- M6: Journal = JSON der ausgeführten Ops inkl. angelegter Ordner und überschriebener Inhalte; `undo` revertiert
  den letzten Lauf. Ausführung stoppt beim ersten Fehler, Teilfortschritt wird trotzdem journalisiert.
- M8: TBD-Queue: Rejected → Verdict `keep` (zusätzlich einsortieren) oder `discard` (Move nach `<library>/_trash/`,
  undo-bar). Endgültig gelöscht wird nur per „Empty trash“ im Dashboard (2-Klick-Bestätigung,
  einzige irreversible Aktion; ein Undo eines Laufs, dessen Dateien so gelöscht wurden, schlägt fehl). Persistiert pro (System, Name); wirkt beim nächsten Plan.
- M8: 1G1R-Gleichstand → Verdict `prefer` auf ein Release (Name ohne Disc-Tag); das wird Pick, die übrigen
  landen als Rejected in der TBD-Queue (dort Keep/Trash).
- M9 (User, 2026-10-05): Die Library ist eine von RomBro **kontinuierlich gemanagte** Sammlung und muss beim
  App-Start sofort da sein (kein manuelles Laden). Umsetzung:
  1. Library-Pfad als Einstellung in der DB (nicht localStorage); Schema erlaubt später mehrere Libraries.
  2. Persistenter Index (Tabelle `file`, vgl. §Schema): wird von Execute und Undo direkt fortgeschrieben;
     Library-View und Dashboard lesen nur aus der DB.
  3. Rescan (Button, optional beim Start), inkrementell über Größe/mtime. Von außen hinzugefügte/umbenannte
     Dateien werden wie Inbox-Items geplant; verschwundene Dateien fliegen aus dem Index.
  4. Planner nutzt den Index statt die Library jedes Mal neu zu hashen.
  Umsetzung: Index = Hash-Cache je Datei (Größe+mtime-ns), gilt auch für die Inbox; Fremddateien laufen wie bisher
  über den Library-Audit (in_library-Items) in den Plan.
- M10: Vollständigkeit = besessene 1G1R-Gruppen (beliebiges Release der Gruppe) / Gruppen mit wählbarem Release
  (aktuelle Regeln). XP = 10/Spiel + Achievement-XP; Level n ab 100·n² XP. Streak = aufeinanderfolgende Tage mit
  ausgeführtem Lauf (endet heute oder gestern). „Verifiziert" = Status `known`. Gamification abschaltbar (DB-Setting).
- Default-Regionspriorität (User, 2026-10-04): **Europe > World > USA > Germany > Japan**; konfigurierbar.

- M11: Pfaderkennung in `core::paths` (`dirs`-Crate); RDB-Ordner = erster Kandidat mit `.rdb`-Datei, Override per
  `ROMBRO_RDB_DIR`. CLI wird als separates Binary neben den Bundles ausgeliefert. Releases als Draft per Tag `v*`.
- Archive mit unbekannten Membern (z. B. Arcade-Sets) werden nie zerlegt: bekannte Member werden entpackt, das Archiv geht danach als Ganzes nach `_quarantine/`.
- **Arcade (FBNeo/MAME)**: Erkennung über CRC+Größe des *ganzen* Zips (RDB-Einträge beschreiben das Archiv, nicht Member). Nur exakte Treffer; Name passt, CRC nicht → normale Quarantäne (RDB ist Single Point of Truth).
  - Mehrfachtreffer: FBNeo > MAME (neueste zuerst: MAME, 2016, 2015, 2010, 2003-Plus, 2003, 2000) > HBMAME. Ein Zip landet genau einmal in der Library (keine CRC-Dubletten).
  - Kurzname (`burningf.zip`) bleibt erhalten; Arcade ist von 1G1R ausgenommen (alle exakt passenden Sets/Clones bleiben).
  - BIOS (Arcade-Zips wie `neogeo.zip` und Konsolen-`[BIOS]`-Einträge, Konsole → `_bios/` = `system`-Wurzel) → `<lib>/_bios/` (FBNeo: `_bios/fbneo/` = RetroArch-`system`-Layout; MAME-Cores: neben den Romsets), sonst komplett ignoriert (keine Anzeige in Bibliothek, Statistik, Achievements; im Import-Plan als zugeklappte Gruppe „BIOS → `_bios`“, User 2026-10-07).
  - MAME-CHDs: gleichnamiger Ordner neben erkanntem Zip (`kinst/kinst.chd`) wird ungeprüft mitgenommen; CHD ohne Zip → Quarantäne.

- **M13 Regeln transparent & einstellbar (User, 2026-10-06):**
  - Zielgruppe Power-User (knappe, technische Texte).
  - Jede Plan-Op trägt eine Regel-ID + Grund; Seite „Regeln“ (GUI) zeigt pro Regel Erklärung, aktuellen Wert
    (editierbar, „Zurücksetzen“ auf Default) und Treffer-Zahl des letzten Laufs.
  - Einstellbar: Regionen, Sprachen, Ausschluss-Flags; Arcade-DB-Reihenfolge + Arcade-1G1R an/aus;
    Ordner-Systeme (FOLDER_SYSTEMS) erweiterbar; Quarantäne an/aus. Global mit Overrides pro System.
  - Fest: m3u-Variante B, Pfade `_bios`/`_playlists`/`_quarantine`/`_trash`.
  - Ausnahmen: „immer behalten“, „Release bevorzugen“, „Pfad nie anfassen“; bestehende Verdicts/Resolutions
    werden als editierbare Ausnahmeliste gezeigt. „System zuordnen“ erst bei Bedarf.
  - Regeländerung → Audit-Plan zur Prüfung, Ausführung undo-bar (nie automatisch).
  - Speicherung in der DB (wie Library-Pfad). CLI: `rules show` + „Warum“ in dry-run-Ausgabe; Setzen via GUI/Datei.
- **BIOS-Erkennung nur per Hash (User, 2026-10-06):** BIOS-Sets mit unbekanntem Hash (z. B. eigenes `neogeo.zip`-Paket) werden *nicht* am Kurznamen erkannt → Quarantäne.
- Bitgleiche Kopie an einem schon belegten/geplanten Ziel = 1G1R-Duplikat (TBD-Queue, `discard`-Verdict → `_trash`), nur abweichender Inhalt ist ein Konflikt (User, 2026-10-06). Gilt auch für BIOS.
- **Firmware-Import per `System.dat` (User, 2026-10-07):** Eine lose Inbox-Datei (oder ein ganzes Zip), deren Größe + SHA1 exakt einem Eintrag der libretro-`System.dat` entspricht, ist BIOS – vor jeder RDB-/DAT-Erkennung (z. B. `SGB1.sfc` nicht als SNES-Spiel). Sie wird unter **jedem** dort gelisteten, in `_bios/` noch fehlenden Pfad abgelegt (Kopien + ein Move, damit jeder Core seinen Namen findet). Ist schon alles belegt: bitgleich → Duplikat (TBD), sonst Konflikt. Dateien in der Bibliothek bleiben, wo sie sind.
- **`_bios` bleibt sauber (User, 2026-10-07):** Nach `_bios/` kommt nur, was unter dem Namen landet, den ein Core erwartet: `System.dat`-Treffer unter ihren gelisteten Pfaden, Arcade-BIOS-Sets nur, wenn der Dateiname dem DAT-Kurznamen entspricht (`neogeo.zip`; sonst Skip mit Hinweis), Konsolen-`[BIOS]`-RDB-Einträge ohne `System.dat`-Treffer gar nicht (Skip).
- Arcade-DATs ergänzen RDBs nur zur Systemwahl, siehe F8 (User, 2026-10-06).
- F8-Defaults (M14, Claude): Quellen = libretro-Core-Repos (FBNeo `dats/…Arcade only).dat`, mame2000/2003/2003-plus/
  2010/2015/2016 `metadata/`), aktuelles MAME = `mame*lx.zip` des neuesten mamedev-Releases. Version = Commit-SHA
  bzw. Release-Tag; unverändert → kein Download. **HBMAME hat keine DAT** → ungeprüft (alte Logik); ebenso jeder Core
  ohne geladene DAT (zählt als vollständig). BIOS-ROMs (`isbios`) werden im Set nicht verlangt (BIOS-Regel).
  Eltern-Set „vorhanden“ = Archiv `<parent>.*` irgendwo im selben Scan oder in der Library (`set_names`;
  bewusst ohne Prüfung, ob es im selben Core-Ordner landet). Übersprungene Cores stehen beim platzierten Set als
  Begründung (`[arcade-set: skipped …]`, `Files::Set.dat_note`). Zuordnung nach Dateiname **und** CRC
  (nur CRC passt → „misnamed“). Set-Name fürs DAT = `rom_name` des RDB-Eintrags.
- Name-Ordner (2026-10-06): `rules.name_folders` (Ordner → System, Default `n64dd` → Nintendo 64, `solarus` → Solarus)
  identifiziert Dateien ohne DB-Treffer direkt in so einem Ordner (bzw. im Systemordner der Library) über den
  Dateinamen (`Ident::Named`, Regel `name-only`). Keine Notizen/Medien (Endungs-Denylist). 1G1R nur unter
  Name-only-Releases desselben Systems, nie gegen verifizierte Dumps; Verlierer → Entscheidung, nie Trash.
  Grund: Batoceras `n64dd`-Dateien sind gepatchte Cartridge-Umbauten (kein `.ndd`, nicht in DBs), Solarus hat keine RDB.
- Library-Scan beim Planen/Export vertraut dem Index (`HashCache.trusted`): nur Ordnerlisten werden gelesen (neue/gelöschte Dateien), bekannte Dateien ohne `stat`. Überschreiben an Ort und Stelle fällt erst beim Library-„Rescan“ (volle Prüfung) auf. Inbox wird immer voll geprüft. Library-Phase ~16 s → ~3 s bei 15k Dateien auf CIFS.

## 11. Offene Fragen
- Ziel (User): vollständige Sammlung im Sinne der RetroArch-Datenbanken. Andere Versionen, Derivate, Formate interessieren nicht, solange sie in keiner RDB stehen → Quarantäne ist richtig für Unbekanntes in erkannten System-Ordnern.
- Entschieden: 1G1R auch für Arcade (User). Keine Parent/Clone-Infos in den RDBs → Gruppierung über den Titel vor der ersten Klammer, über alle DBs, die ein Set listen (Union-Find; `Gradius III: Densetsu…` findet über den MAME-Namen `Gradius III (Japan)` in die Gruppe). Auswahl: Regionen-Reihenfolge der Regeln → Original vor Bootleg/Hack/Proto → neueste Fassung (Name absteigend: Datum/Revision). Abgelehnte → Entscheidungen (wie Konsole). Code: `arcade/g1r.rs`.
- Entschieden: Verschiedene Spiele mit gleichem Arcade-Kurznamen weichen auf das nächste System aus, das das Set listet; Sets mit den wenigsten Ausweich-Systemen wählen zuerst.
- Entschieden: Multi-Disk-Archive (alle Member erkannt, ein System, Disk/Side-Tags) → Variante B: `<System>/<Zipname>/` mit Original-Membernamen + `.m3u`; Spielordner mit eigener `.m3u` in der Bibliothek gelten als fertig (kein Umbenennen nach DB-Namen).
- Entschieden: GameCube/Wii-Container (`.rvz/.wia/.wbfs/.ciso`) und GC/Wii-ISOs werden über die Spiel-ID im Disc-Header erkannt (wie Serial-Fallback); Revision/Disc-Nr. aus dem Header wählen zwischen `(Rev n)`/`(Disc n)`. Keine Hash-Prüfung möglich.
- Entschieden: BIOS-Ordner im Inbox (`bios`, `00bios`, `system`) bleiben wie sie sind; nur erkannte BIOS werden nach `_bios/` einsortiert.
- Entschieden: Spielordner (DOS, ScummVM, Ports/Engines – Liste in `plan/build/folders.rs`): ein DB-Treffer identifiziert den Ordner, der Ordner wandert komplett nach `<System>/<Ordnername ohne .dos/.scummvm>/`. Name/System nie aus dem Treffer allein (Schlüsseldateien wie `dosbox.bat`, `ADL.DRV` sind geteilt). Endung entscheidet das System, sonst spezifischer Port vor DOS/ScummVM, sonst Mehrheit.
- Entschieden: Unbekannte Dateien in Ordnern ohne jeden erkannten Eintrag bleiben unangetastet (Spielinstallationen, Frontend-Medien); nur Ordner mit Treffern und die Inbox-Wurzel werden in die Quarantäne gekehrt.
- Entschieden: Archive mit unbekannten Membern werden nie teilweise entpackt (ganz in Quarantäne).
- Entschieden (User, 2026-10-06): Export nach RetroArch (`core::retroarch`, CLI `retroarch <lib>`, App Settings →
  RetroArch). Ordner aus `retroarch.cfg`. Playlists aus `<lib>/_playlists/` → `playlist_directory` (gleicher Name,
  `Op::Write` → Undo stellt die alten wieder her), `default_core_*` = installierter Core, dessen `.info`-`database`
  das System nennt (wenigste Systeme gewinnt). BIOS: libretro `System.dat` (Snapshot in `crates/core/data/`) per SHA1
  gegen den Library-Index, Arcade-Zips per Name aus `_bios/`; `Op::Copy` nur wenn Ziel fehlt, abweichende Dateien
  bleiben (Konflikt-Hinweis); fehlende BIOS nur für Systeme mit Library-Ordner gemeldet.
- Entschieden (User, 2026-10-06): Arcade-DATs enthalten auch `<disk>`-Einträge (CHDs). Ein Set, dessen eigene Disk
  nicht als `<set>/<disk>.chd` neben dem Zip liegt, ist unvollständig (z. B. Laserdisc-Spiele ohne CHD) → Trash mit
  Begründung, keine Playlist. Geerbte Disks (`merge`) werden nicht geprüft. Schema v10 verwirft alte DATs → einmal
  „Sync databases“.
- Entschieden (User, 2026-10-06): Sets mit Treiberstatus `preliminary` („not working“) gelten für den jeweiligen
  Core als unvollständig; nächster Core in der Reihenfolge, sonst Trash mit Grund (`rules.arcade_working_only`,
  Default an). Schema v11 (Spalte `working`, DATs neu laden). Playlists von Systemen ohne Spiele → `_trash/playlists/`.
- Entschieden (User, 2026-10-06): Unerkannte Dateien (falsche Dumps, Systeme ohne RDB wie Daphne, unvollständige
  Arcade-Sets) gehen per Default nach `_trash/unknown/<Pfad>` statt `_quarantine` (`rules.unknown_to_trash`, Default an;
  aus = alte Quarantäne). Eine bestehende `_quarantine` wird dabei geleert; inzwischen erkannte Dateien bleiben.
  Undo holt alles zurück, endgültig gelöscht wird nur über „Papierkorb leeren“.
- Entschieden (User, 2026-10-06): Frontend-Metadaten (Batocera/EmulationStation) wandern automatisch nach
  `_trash/frontend/<Pfad>` (Regel `frontend-meta`, Schalter `rules.frontend_trash`, Default an): `gamelist*` und
  `_info.txt`/`_readme.txt`/`_lisezmoi.txt` in einem Ordner mit `gamelist*.xml` (auch in `_quarantine`) sowie Bilder/Videos/PDFs in dessen `images/`, `videos/`, `media/`,
  `manuals/`, `downloaded_*`. Nur solche Ordner zählen, damit Spieldaten nie betroffen sind. Startdateien
  (`.libretro`, `.quake` …) bleiben. Ordner-Spiele werden ohne die Metadaten verschoben.
- Entschieden (User, 2026-10-06): Core-Wahl & -Installation beim RetroArch-Export (`retroarch::pick`).
  Empfehlung pro RDB-System aus den **Batocera-x86_64-Defaults** (`batocera-launch/resources/defaults/config.yml`
  + `config-x86_64.yml`), übersetzt in libretro-Kerne: `crates/core/src/retroarch/recommended.tsv`. Wo Batocera einen
  Standalone-Emulator nutzt, der libretro-Port (N64 mupen64plus_next, PSP ppsspp, GC/Wii dolphin, PS2 pcsx2) bzw.
  Alternative (Jaguar virtualjaguar, Quake II/III vitaquake2/3). Arcade nicht in der Tabelle: spezialisiertester Core.
  Reihenfolge: User-Wahl (`rules.cores`, System → Core-ID) > Empfehlung > spezialisiertester installierter Core.
  Auswahl: alle Cores, deren `.info` das System nennt (RetroArch liefert Infos aller Cores). Installation nur auf
  Anforderung (`--install-cores` / Checkbox): `<core_updater_buildbot_cores_url>/<core>_libretro.so.zip` → Cache
  `~/.cache/rombro/cores/`, dann `Op::Extract` in `libretro_directory` (Journal, Undo entfernt). Ohne Install bekommt
  die Playlist den besten installierten Core, der gewünschte wird als fehlend gemeldet.
- Entschieden (User, 2026-10-06): Kombinierte Sufami-Turbo-Images (`.smc/.sfc`: BIOS 256 KiB ×4 gespiegelt, dann
  1–2 Carts ab `0x100000`, Erkennung über `BANDAI SFC-ADX`) gelten als Pseudo-Archiv (`core::sufami`): Member
  `SuFami Turbo (Japan).sfc` + `Slot A/B.st` werden wie Zip-Member erkannt und extrahiert, das Original geht danach
  nach `_trash/`. Bitgleiche Duplikat-Member (gleicher DB-Name wie der Pick) brauchen keine Entscheidung, sie werden
  nicht extrahiert und zählen fürs Archiv als erledigt (gilt für alle Archive).
- Entschieden: Arcade-Treffer auf einzelne Chips in einem Zip zählen nicht (Arcade-RDBs hashen ganze Sets) → Zip bleibt ganz, ohne Set-Treffer in Quarantäne.
- Entschieden: Quarantäne übernimmt den Pfad relativ zum Inbox (`_quarantine/<unterordner>/<datei>`), damit gleiche Namen nicht kollidieren.
- Entschieden: OS-Müll (`.DS_Store`, `._*`, `Thumbs.db`, `desktop.ini`) und leere Dateien werden beim Scan ignoriert und nie verschoben.
- Entschieden: MAME-Cores (2000/2003/2003-Plus/aktuell) suchen BIOS nur im Romset-Ordner → BIOS-Zips liegen neben den Sets (`<System>/neogeo.zip`, in der UI ausgeblendet); nur FBNeo nutzt `_bios/fbneo/`.
- Discs in Archiven (`.zip`/`.7z` mit cue/bin) und `.chd` – in M6 nicht umgesetzt; Vorschlag: eigener Schritt nach M8.
- ZIP als Default-Format in der Library oder entpackt? (Vorschlag: Cartridges zippen, Discs als CHD/entpackt).
- Umgang mit Arcade (MAME/FBNeo-Sets): v1 nur verifizieren, nicht 1G1R-reduzieren?
- No-Intro-DAT-Import für echte Parent/Clone-Daten in v1 oder später?
