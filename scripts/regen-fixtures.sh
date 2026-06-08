#!/usr/bin/env bash
# Title: Minibench
# Subtitle: regenerate demo fixtures from the REAL workbench adapters
# Author: Christopher Lester
# Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.
#
# Replaces hand-written demo fixtures with genuine workbench_snapshot/v1
# output from the live thesium-workflows adapters — the "live tenant feed".
# Runs the `bb workbench:*` tasks on the non-personal inputs in
# fixtures/inputs/ and writes their validated snapshots into fixtures/.
#
# Requires: a thesium-workflows checkout (the producers) + babashka.
# Override its location with THESIUM_WORKFLOWS. NEVER point this at real
# tenant data — the committed fixtures must stay synthetic.
#
# Usage:  scripts/regen-fixtures.sh
#         THESIUM_WORKFLOWS=/path/to/thesium-workflows scripts/regen-fixtures.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TW="${THESIUM_WORKFLOWS:-$ROOT/../thesium-career/thesium-workflows}"
IN="$ROOT/fixtures/inputs"
EXP="$ROOT/fixtures/experiments"

if [ ! -d "$TW" ]; then
  echo "thesium-workflows not found at: $TW" >&2
  echo "Set THESIUM_WORKFLOWS to your checkout." >&2
  exit 1
fi

# Run a bb task from the producer repo (cwd must be the workspace root).
tw() { ( cd "$TW" && bb "$@" ); }

EXPERIMENT="career.lens.acme-l4-eval"

# Career lens — two divergent variants of ONE experiment. Same logical
# task, different model/method -> the permutation matrix (pass vs fail).
tw workbench:snapshot --mode lens --input "$IN/career-lens-opus.edn" \
   --out "$EXP/opus-semantic.json" \
   --experiment-id "$EXPERIMENT" --label opus+semantic \
   --model claude-opus-4-8 --method semantic

tw workbench:snapshot --mode lens --input "$IN/career-lens-haiku.edn" \
   --out "$EXP/haiku-mechanical.json" \
   --experiment-id "$EXPERIMENT" --label haiku+mechanical \
   --model claude-haiku-4-5 --method mechanical

# Portfolio — a second real tenant, single daily snapshot.
tw workbench:portfolio-snapshot --readiness "$IN/portfolio-readiness.json" \
   --out "$ROOT/fixtures/portfolio-daily.json" \
   --experiment-id portfolio.daily --label baseline --model risk-pipeline

echo "Regenerated fixtures from real adapter output."
echo "  cargo run -p minibench-cli -- compare fixtures/experiments"
