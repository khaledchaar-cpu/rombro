#!/usr/bin/env bash
# Builds a mixed test inbox (copies only – the collection is read, never changed) plus an
# empty library and a copy of the DB in $T (default ~/rombro-test).
# Usage: scripts/make-testset.sh "<collection root>" [target dir]
set -euo pipefail
R="${1:?collection root}"; T="${2:-$HOME/rombro-test}"
rm -rf "$T"; mkdir -p "$T"/lib "$T"/inbox/{snes,gb,arcade,ps1,dos,scummvm,bios,junk,keep-out}
S="$R/Nintendo - Super Nintendo Entertainment System"; G="$R/Nintendo - Game Boy"
cp "$S/Super Mario World 2 - Yoshi's Island (USA, Asia) (Rev 1).zip" \
   "$S/Super Mario World 2 - Yoshi's Island (USA) (Rev 1).zip" \
   "$S/Super Mario World 2 - Yoshi's Island (USA).zip" "$S/Chrono Trigger (USA).zip" \
   "$S/Chrono Trigger (Japan) (Sample).zip" "$S/Street Fighter II (USA).zip" \
   "$S/Super Mario World (USA).zip" "$T/inbox/snes/"
cp "$S/Super Mario World (USA).zip" "$T/inbox/snes/smw-copy.zip"          # duplicate
cp "$G/Tetris (World) (Rev A).zip" "$T/inbox/gb/"
d=$(mktemp -d)                                                            # multi-ROM archive
(cd "$d" && unzip -q "$G/Tetris 2 (USA, Europe).zip" && unzip -q "$G/Tetris Blast (USA, Europe).zip" \
  && zip -q "$T/inbox/gb/tetris-pack.zip" ./*)
rm -rf "$d"
cp "$R/Mame - Arcade/"{1943.zip,1943kai.zip,sf2ce.zip,pacmanf.zip} "$R/SNK - Neo Geo/2020bb.zip" \
   "$R/00bios/neogeo.zip" "$T/inbox/arcade/"
cp "$R/Sony - Playstation/Chrono Cross (USA) (Disc "{1,2}").chd" "$R/Sony - Playstation/40 Winks (USA).chd" \
   "$T/inbox/ps1/"
cp -r "$R/PC - DOS/Abuse.dos" "$T/inbox/dos/"
cp -r "$R/ScummVM/Beneath a Steel Sky.scummvm" "$T/inbox/scummvm/"
cp "$R/00bios/scph1001.bin" "$T/inbox/bios/"
head -c 300000 /dev/urandom > "$T/inbox/junk/mystery.bin"; echo notes > "$T/inbox/junk/readme.txt"
head -c 4096 /dev/urandom > "$T/inbox/snes/corrupt.sfc"                  # unknown → quarantine
cp "$S/Secret of Mana (USA).zip" "$T/inbox/keep-out/"                     # for ignore tests
cp "${XDG_DATA_HOME:-$HOME/.local/share}/rombro/rombro.db" "$T/test.db"
find "$T/inbox" -type f | sort | md5sum > "$T/before.txt"
echo "test set in $T ($(du -sh "$T/inbox" | cut -f1)); use --db $T/test.db"
