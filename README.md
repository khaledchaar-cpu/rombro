# ROMBRO

ROM curator built on the RetroArch databases: verify files by hash, reduce sets to
one game per region (1G1R), import an inbox into a clean library, undo every step.

## Install

Download the bundle for your OS from the releases page (AppImage/deb, dmg, msi/exe).
The `rombro` CLI ships next to it as a separate binary.

The bundles are **not code-signed**, so the OS warns on first launch:

- **macOS:** right-click the app → *Open* → *Open*, or run
  `xattr -dr com.apple.quarantine /Applications/ROMBRO.app`.
- **Windows:** SmartScreen → *More info* → *Run anyway*.
- **Linux:** AppImage needs `chmod +x`; the deb installs normally.

## First run

1. Install RetroArch and update its databases (*Online Updater → Update Databases*).
2. Open ROMBRO → Dashboard → **Sync RDBs**. The folder is detected automatically:

   | OS | RetroArch RDB folder |
   |---|---|
   | Linux | `~/.config/retroarch/database/rdb` (also Flatpak, Snap, `/usr/share/libretro`) |
   | macOS | `~/Library/Application Support/RetroArch/database/rdb` |
   | Windows | `%APPDATA%\RetroArch\database\rdb`, `C:\RetroArch-Win64\database\rdb` |

   Elsewhere: **Choose folder…**, or set `ROMBRO_RDB_DIR`.
3. Pick an inbox and a library folder, review the plan, execute. Undo is always available.

## Library layout

```
<library>/
  <System>/            games, one folder per RetroArch system
  _playlists/          RetroArch playlists (.lpl)
  _bios/               console BIOS → point RetroArch's System/BIOS directory here
  _bios/fbneo/         FBNeo BIOS sets (neogeo.zip, …)
  _quarantine/         unknown or broken files, never deleted
  _trash/              discarded files, restorable via Undo
```

BIOS files are placed automatically and hidden in the app. MAME cores only look next to
the romsets, so their BIOS sets stay in the `MAME …` system folder.

ROMBRO's own data lives in the OS data directory (`rombro/rombro.db`), thumbnails in the
cache directory (`rombro/thumbnails`).

## CLI

```
rombro db sync [<rdb-dir>]          # import RDBs
rombro scan <dir> [--unknown]       # identify files
rombro g1r "<System>" [--filter x]  # show 1G1R picks
rombro import <inbox> <lib> --dry-run
rombro audit <lib> | undo | resolve <file> [n]
```

## Build from source

Rust (stable, ≥ 1.85), Node 22 and pnpm; on Linux additionally
`libwebkit2gtk-4.1-dev librsvg2-dev`.

```
cd ui && pnpm install && pnpm tauri build   # app bundles in target/release/bundle/
cargo build --release -p rombro-cli         # CLI
./scripts/check.sh                          # fmt, clippy, tests, typecheck
```

CI (`.github/workflows/ci.yml`) runs the checks and builds bundles for Linux, macOS and
Windows; pushing a `v*` tag creates a draft release.

## License

MIT
