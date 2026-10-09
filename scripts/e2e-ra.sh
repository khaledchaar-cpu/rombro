#!/usr/bin/env bash
# End-to-end test of the managed RetroArch: install, status, core download via `play`,
# then a synthetic NES ROM headless for 120 frames. Uses the platform's data dir
# (on Linux set XDG_DATA_HOME to a scratch dir first).
set -euo pipefail
BIN=${BIN:-target/release/romburak}
WORK=$(mktemp -d)
native() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }

"$BIN" ra install
status=$("$BIN" ra status)
echo "$status"
root=$(sed -n 's/^folder: *//p' <<<"$status")
exe=$(sed -n 's/^binary: *//p' <<<"$status")
[ -n "$exe" ] || { echo "no binary after install"; exit 1; }

# 32 KiB PRG (reset vector → JMP $8000 loop) + 8 KiB CHR
lib="$WORK/lib"
sys="$lib/Nintendo - Nintendo Entertainment System"
mkdir -p "$sys"
rom="$sys/Test.nes"
python3 - "$rom" <<'PY'
import sys
prg = bytearray(32768)
prg[0:3] = b"\x4c\x00\x80"            # JMP $8000
prg[0x7ffa:0x8000] = b"\x00\x80" * 3  # NMI/RESET/IRQ → $8000
open(sys.argv[1], "wb").write(b"NES\x1a\x02\x01" + bytes(10) + prg + bytes(8192))
PY

db=$(native "$WORK/e2e.db")
out=$("$BIN" play "$(native "$rom")" --library "$(native "$lib")" --dry-run --db "$db")
echo "$out"
core=$(find "$root/cores" -name '*_libretro.*' -type f | head -1)
[ -n "$core" ] || { echo "no core installed"; exit 1; }

# headless: copy of the managed config with null drivers (never --appendconfig)
cfg="$WORK/ra.cfg"
grep -v -E '^(video|audio|input|joypad)_driver' "$root/retroarch.cfg" >"$cfg" || true
printf 'video_driver = "null"\naudio_driver = "null"\ninput_driver = "null"\njoypad_driver = "null"\n' >>"$cfg"
"$exe" --config "$(native "$cfg")" -L "$(native "$core")" "$(native "$rom")" --max-frames=120 --verbose \
  >"$WORK/ra.log" 2>&1 || { tail -40 "$WORK/ra.log"; echo "RetroArch failed"; exit 1; }
grep -i -m3 -E 'content loading|loaded core|Frames' "$WORK/ra.log" || true
echo "e2e ok"
