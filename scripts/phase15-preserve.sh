#!/usr/bin/env bash
# Reproduce all earlier frozen scores, then protect base assets and physics.
set -euo pipefail
cd "$(dirname "$0")/.."
out=${1:?supply output directory}
mkdir -p "$out"
out=$(cd "$out" && pwd)
bash scripts/phase13h-preserve.sh "$out/13h"
bash scripts/phase13i-run.sh "$out/13i"
git diff --exit-code bc5d51b -- docs/gates maps src/judge.rs src/schema.rs \
    ':(exclude)docs/gates/phase-15*'
printf '%s\n' 'Phase 15 preservation: all cmp OK; protected base files unchanged.'
