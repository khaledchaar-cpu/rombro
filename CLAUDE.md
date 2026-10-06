# ROMBRO – Arbeitsanweisungen für Claude

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
| Tests Core | `cargo test -q -p rombro-core` |
| CLI | `cargo run -q -p rombro-cli -- <cmd>` |
| App dev | `cd ui && pnpm tauri dev` (nur Frontend: `pnpm dev`, Mock-IPC) |
| RDBs + Arcade-DATs → SQLite | `cargo run -q --release -p rombro-cli -- db sync` (lädt DATs aus dem Netz) |
| Lookup | `cargo run -q --release -p rombro-cli -- db lookup <crc/sha1/md5/serial>` |
| DB-Statistik | `cargo run -q --release -p rombro-cli -- db stats [--db <file>]` (inkl. DAT-Versionen) |
| Benchmarks | `cargo bench -q -p rombro-rdb` / `-p rombro-core` |
| Scan | `cargo run -q --release -p rombro-cli -- scan <dir> [--unknown]` |
| 1G1R prüfen | `cargo run -q --release -p rombro-cli -- g1r "<System>" [--filter <text>]` |
| Import / Audit | `cargo run -q --release -p rombro-cli -- import <inbox> <lib> --dry-run` / `audit <lib>` |
| Testset bauen | `./scripts/make-testset.sh "<Sammlung>"` → `~/rombro-test`, dann `--db ~/rombro-test/test.db` |
| Regeln zeigen/laden | `… -- rules [--set rules.json] [--ignore/--unignore <pfad>]` |
| RetroArch-Export | `… -- retroarch <lib> [--dry-run] [--cfg retroarch.cfg] [--no-playlists/--no-bios] [--install-cores] [--core "Sys=id"] [--list-cores]` |
| Undo / Resolve | `… -- undo` / `… -- resolve <file> [n]` |
| Echte RDBs testen | `cargo test -q --release -p rombro-rdb -- --ignored` |
| RDB-Ort (Linux) | `~/.config/retroarch/database/rdb/` (146 Dateien), Erkennung: `core::paths` |
| App-Bundle lokal | `cd ui && pnpm tauri build --bundles deb` |

## Code-Konventionen
- Rust edition 2024, `cargo fmt`, `clippy -D warnings`. Libs: `thiserror`; Bins: `anyhow`. Kein `unwrap()` außerhalb Tests.
- Core bleibt GUI-frei. Dateisystem-Änderungen ausschließlich über den Planner (Plan → Execute → Journal).
- Performance: kein unnötiges Allokieren in Hot Paths, Streaming-I/O, rayon für CPU-Arbeit. Benchmarks in `benches/` für Parser/Hashing.
- Tests: Unit-Tests neben dem Code, Integrationstests mit kleinen Fixtures in `tests/fixtures/` (keine echten ROMs einchecken, synthetische Dateien erzeugen).
- Frontend: SolidJS + TS strict, Design-Tokens nur in `ui/src/styles/tokens.css`, keine UI-Lib.

## Git
- Conventional Commits: `feat(core): …`, `fix(rdb): …`, `chore(m2): wrap-up`.
- Direkt auf `main` (Solo-Projekt), kleine Commits. Remote: `origin` = github.com/khaledchaar-cpu/rombro (öffentlich), pushen nach grünem check.
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
