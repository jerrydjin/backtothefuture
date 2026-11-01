# Phase 15a is blocked before preregistration

As of 1 November 2026, this branch starts at `bc5d51b` and contains evidence
preparation only. There is no Phase 15 simulation implementation,
preregistration, code freeze, model reply file, scoring run or result claim.
Phase 15b has not started.

The task explicitly requires: “If you cannot install torch or download the
weights, do NOT fake them: commit the script and STOP before the
preregistration, saying the cached SAM output must be supplied.”

The environment has no installed torch or transformers, and no SAM snapshot in
the default Hugging Face cache. The enforced network policy returns HTTP 403
to both the Hugging Face model URL and the Wikimedia source page. The original
photograph, weights, `mask.png` and `sam.json` are absent. Genuine cached SAM
output must be supplied, or the network configuration must permit downloading
the original photograph and model snapshot in a separate preparation step.
The provided SAM script itself always stays offline.

After installing CPU torch/torchvision, transformers (the API was inspected in 4.57.1),
and Pillow, put the hash-matching photograph in `assets/phase15/` and cache
the requested model snapshot with its processor files. Then run:

```sh
python3 scripts/phase15-sam.py
python3 scripts/phase15-sam.py --check
```

Alternatively, supply all three genuine evidence files using the metadata
schema written by the script, then run `--check`. Checking verifies the fixed
photograph hash, mask hash, model identity and full revision, prompt points,
CPU provenance fields, dimensions, binary pixels, bounds and area. It does
not independently prove that inference produced a supplied mask. Inference
has not been validated here because its required inputs are unavailable.

Continue the tree implementation only after that blocker is resolved. Commit
all implementation, tests, assets, scenarios and recorded replies before the
preregistration and code freeze. Score all ten seeds and all three ranges
only after that separate preregistration commit. Retain every failure and
unfinished trip. The claim remains at 300 m with the bars in the task; 60 m
and 150 m remain hypothetical sensitivity results. No preregistered bar or
scenario has been changed.

The freeze deadline remains **Monday 2 November 2026, 23:00 UK (GMT)**.
Nothing in this preparation constitutes shipping 15a or 15b.
