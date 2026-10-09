# Romburak – Arbeitsanweisungen für Claude

ROM-Curator auf Basis der RetroArch-Datenbanken (1G1R, Import, Gamification; v2 Launcher).
**Was** gebaut wird steht in `SPEC.md`, **wo wir stehen** in `PROGRESS.md`. Diese Datei regelt **wie**.

## Sprache
- Kommunikation mit dem User: Deutsch. Code, Kommentare, Commits, Identifier: Englisch.

## Autonomie
- Volle Berechtigung: bauen, testen, committen, Abhängigkeiten installieren, Skills anlegen – ohne Rückfrage.
- Rückfragen nur bei echten Produktentscheidungen, die SPEC.md nicht beantwortet. Sonst sinnvollen Default wählen,
  in SPEC.md unter „Entscheidungen“/„Offene Fragen“ notieren, weitermachen.
- Bei grundlegend unklaren Anforderungen (neues Feature, widersprüchliche Ziele): `grilling`-Skill nutzen, Ergebnis in SPEC.md festhalten.
- Nie direkt Dateien in echten ROM-Sammlungen des Users verändern; Tests nur mit Fixtures oder Kopien.

## Session-Ablauf (ein Milestone pro Session)
1. `PROGRESS.md` lesen → aktuellen Milestone + nächste Schritte. Nur die SPEC-Abschnitte lesen, die er braucht.
2. Arbeiten in kleinen, testbaren Schritten. Nach jedem grünen Teilschritt committen.
3. **Wrap-up** am Ende des Milestones (oder wenn der Kontext knapp wird):
   - `PROGRESS.md` aktualisieren (Status, erledigt, nächste Schritte, Stolpersteine).
   - Neue Erkenntnisse → `SPEC.md` (Fakten/Entscheidungen) bzw. hier (Konventionen/Befehle).
   - `./scripts/check.sh` grün, dann Commit `chore(mX): wrap-up`.
4. Dem User kurz berichten: was fertig ist, was offen ist, nächster Milestone.

## Token-Sparsamkeit
- Gezielt lesen: `rg`/`grep -n` + `sed -n 'a,bp'` statt ganzer Dateien; große Dateien nie komplett.
- Keine Code-Dumps im Chat, keine Wiederholung von Dateiinhalten. Antworten knapp.
- Build-/Test-Output filtern: `cargo test -q 2>&1 | tail -30`, `cargo clippy -q`.
- Module klein halten (< ~300 Zeilen), damit sie einzeln lesbar sind.
- Wiederkehrende Abläufe (> 2× gebraucht) als Skill unter `.claude/skills/<name>/SKILL.md` ablegen und hier listen.
- Kein Subagent ohne Not.

## Befehle
| Zweck | Befehl |
|---|---|
| Alle Checks | `./scripts/check.sh` (fmt, clippy -D warnings, test) |
| Tests Core | `cargo test -q -p romburak-core` |
| CLI | `cargo run -q -p romburak-cli -- <cmd>` |
| App dev | `cd ui && pnpm tauri dev` (nur Frontend: `pnpm dev`, Mock-IPC) |
| RDBs + Arcade-DATs → SQLite | `cargo run -q --release -p romburak-cli -- db sync [--path <rdb-dir>]` (lädt RDBs + DATs aus dem Netz; Test: `XDG_DATA_HOME=<scratch>`) |
| Lookup | `cargo run -q --release -p romburak-cli -- db lookup <crc/sha1/md5/serial>` |
| DB-Statistik | `cargo run -q --release -p romburak-cli -- db stats [--db <file>]` (inkl. DAT-Versionen) |
| Benchmarks | `cargo bench -q -p romburak-rdb` / `-p romburak-core` |
| Scan | `cargo run -q --release -p romburak-cli -- scan <dir> [--unknown]` |
| 1G1R prüfen | `cargo run -q --release -p romburak-cli -- g1r "<System>" [--filter <text>]` |
| Import / Audit | `cargo run -q --release -p romburak-cli -- import <inbox> <lib> --dry-run` / `audit <lib>` |
| Testset bauen | `./scripts/make-testset.sh "<Sammlung>"` → `~/rombro-test`, dann `--db ~/rombro-test/test.db` |
| Regeln zeigen/laden | `… -- rules [--set rules.json] [--ignore/--unignore <pfad>]` |
| Cores je System | `… -- ra cores [--library <lib>] [--set "Sys=id"]` (`Sys=` = Empfehlung) |
| Spielen | `… -- play <datei> [--core id [--save]] [--slot n] [--states] [--dry-run]` (zählt Spielzeit ≥ 30 s) |
| Verwaltetes RetroArch | `… -- ra status [--check]` / `ra install [--latest] [--library <lib>]` (Test: `XDG_DATA_HOME=<scratch>`) |
| Anzeige RetroArch | `… -- ra display [fullscreen|window[:1-6]|auto]` · `ra shader [<preset>|off|auto] [--list <text>]` · `ra aspect [core|4:3|16:9|square|full|auto]` |
| RetroAchievements | `… -- cheevos key <key>` / `cheevos sync` / `cheevos scan [--list]` / `cheevos login <user>` / `logout` / `hardcore [on\|off]` |
| Undo / Resolve | `… -- undo` / `… -- resolve <file> [n]` |
| Echte RDBs testen | `cargo test -q --release -p romburak-rdb -- --ignored` |
| RDB-Ort (Linux) | `~/.config/retroarch/database/rdb/` (146 Dateien), Erkennung: `core::paths` |
| rcheevos-Referenzhash | `git clone --depth 1 https://github.com/RetroAchievements/rcheevos`, `gcc -o rh rh.c -Ircheevos/include -Ircheevos/src rcheevos/src/hash/*.c rcheevos/src/util/*.c` (rh.c: `rc_hash_init_default_cdreader` + `rc_hash_generate_from_file`; kann kein CHD/CSO) |
| App-Bundle lokal | `cd ui && pnpm tauri build --bundles deb` |

## Code-Konventionen
- Rust edition 2024, `cargo fmt`, `clippy -D warnings`. Libs: `thiserror`; Bins: `anyhow`. Kein `unwrap()` außerhalb Tests.
- Core bleibt GUI-frei. Dateisystem-Änderungen ausschließlich über den Planner (Plan → Execute → Journal).
- Performance: kein unnötiges Allokieren in Hot Paths, Streaming-I/O, rayon für CPU-Arbeit. Benchmarks in `benches/` für Parser/Hashing.
- Tests: Unit-Tests neben dem Code, Integrationstests mit kleinen Fixtures in `tests/fixtures/` (keine echten ROMs einchecken, synthetische Dateien erzeugen).
- Frontend: SolidJS + TS strict, Design-Tokens nur in `ui/src/styles/tokens.css`, keine UI-Lib.

## Git
- Conventional Commits: `feat(core): …`, `fix(rdb): …`, `chore(m2): wrap-up`.
- Direkt auf `main` (Solo-Projekt), kleine Commits. Remote: `origin` = github.com/khaledchaar-cpu/romburak (öffentlich), pushen nach grünem check.
- Commit-Mail muss die GitHub-noreply-Adresse sein (Repo-`user.email` gesetzt), sonst lehnt GitHub den Push ab.

## Skills
| Skill | Zweck |
|---|---|
| `grilling` (global) | Anforderungen klären |
| *(projektspezifische Skills hier eintragen, sobald angelegt)* | |

## Erkenntnisse / Stolpersteine
- RetroArch headless **nie mit `--appendconfig`** (bei `config_save_on_exit` landen die Null-Treiber in der echten
  Config!): `cp retroarch.cfg <scratch>/ra.cfg`, Treiber dort auf null, `retroarch --config <scratch>/ra.cfg -L <core.so> <zip> --max-frames=300`.
- RDB: Hashes sind MessagePack-`bin`, nicht Hex. Header 16 Byte (`RARCHDB\0` + u64-Offset auf Metadaten). Siehe SPEC §3.
