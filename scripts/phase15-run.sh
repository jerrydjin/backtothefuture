#!/usr/bin/env bash
# Score only after the corresponding committed preregistration and freeze.
set -euo pipefail
cd "$(dirname "$0")/.."
part=${1:?tree or birth}
out=${2:?output JSONL}
case "$part" in tree|birth) ;; *) exit 2;; esac
cargo run --offline --release --example phase15_gate -- "part=$part" seed=1 end=10 > "$out"
