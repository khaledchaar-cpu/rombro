#!/usr/bin/env bash
# Read-only inbox analysis: import dry-run against the real library, summarized.
# Usage: scripts/inbox-report.sh [out-dir]   (paths from the app DB settings)
set -euo pipefail
cd "$(dirname "$0")/.."
DB="${XDG_DATA_HOME:-$HOME/.local/share}/romburak/romburak.db"
INBOX=$(sqlite3 "$DB" "select value from settings where key='inbox'")
LIB=$(sqlite3 "$DB" "select value from settings where key='library'")
OUT="${1:-${TMPDIR:-/tmp}}"; mkdir -p "$OUT"; R="$OUT/dry.txt"
echo "inbox: $INBOX"; echo "library: $LIB"
echo "== inbox folders (files, size, top extensions)"
for d in "$INBOX"/*/; do
  n=$(find "$d" -type f | wc -l); s=$(du -sh "$d" | cut -f1)
  ext=$(find "$d" -type f | sed 's/.*\.//' | sort | uniq -c | sort -rn | head -3 | awk '{printf "%s×%s ",$1,$2}')
  echo "  $(basename "$d"): $n files, $s  [$ext]"
done
cargo build -q --release -p romburak-cli
set +e; ./target/release/romburak import "$INBOX" "$LIB" --dry-run >"$R" 2>&1; rc=$?; set -e
[ $rc -ne 0 ] && { echo "!! dry-run failed (exit $rc):"; tail -5 "$R"; exit $rc; }
echo "== summary"; tail -1 "$R"
echo "== kinds"; awk '{print $1}' "$R" | grep -E '^[A-Z?]+$' | sort | uniq -c | sort -rn
echo "== targets (MOVE/COPY/EXTRACT by top folder)"
grep -E '^(MOVE|COPY|EXTRACT)' "$R" | sed -E 's/.* -> //; s/  \[.*//' | sed "s|^$LIB/||" \
  | awk -F/ '{print ($1 ~ /^_/ ? $1"/"$2 : $1)}' | sed -E 's|^(_trash)/[^u].*|\1 (discarded)|' | sort | uniq -c | sort -rn
echo "== TBD by inbox folder / reason"
grep '^TBD' "$R" | grep -o "inbox/[^/]*/" | sort | uniq -c
grep '^TBD' "$R" | grep -oE '\([A-Za-z]+\); kept' | sort | uniq -c
echo "== inbox folders without any planned op (unknown system / untouched)"
for d in "$INBOX"/*/; do b=$(basename "$d"); grep -qF "/$b/" "$R" || echo "  $b"; done
echo "== needs a look"; grep -E '^(CONFLICT|ERROR|AMBIG|TIE|\?)' "$R" | sed "s|$INBOX/||; s|$LIB/||g" | cut -c1-200
echo "full report: $R"
