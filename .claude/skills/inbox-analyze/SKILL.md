---
name: inbox-analyze
description: Analyze the user's real Romburak inbox read-only before an import (dry-run against the real library) and report per system what would be placed, duplicates, conflicts, errors and unknown systems. Use when the user says they put new systems/ROMs into the inbox and wants an analysis before importing.
---
# Inbox analysieren (nur lesend)

1. `./scripts/inbox-report.sh <scratchpad>` – liest Inbox/Library aus der App-DB, baut, macht `import --dry-run`
   (nie `audit` ohne `--dry-run`, nie ohne `--dry-run` importieren!), fasst zusammen. Dauer: wenige Minuten.
2. Bricht der Dry-Run ab (Panic/Stack Overflow) → Bug: Backtrace per `coredumpctl debug romburak`, fixen + Test, dann erneut.
3. Ordner ohne Ops: `romburak scan <ordner>` → keine RDB fürs System = bleibt unangetastet (erwähnen).
4. Auffällige Fälle stichprobenartig im `dry.txt` nachsehen (Konflikte: abweichender Inhalt; TIE/AMBIG: App entscheidet).
5. Bericht an User auf Deutsch, knapp: Tabelle je Inbox-Ordner (einsortiert / Duplikate·1G1R / unbekannt),
   dann „vorher ansehen“ (Konflikte, Lesefehler, mehrdeutig). Nichts in Inbox/Library verändern.
