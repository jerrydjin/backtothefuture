# Phase 4: live at scale

Stacked on #23, against main. No merge.

Contemporaneous conflicts are sent in batches of at most twelve, with at most four concurrent calls. Each batch has a 28-second deadline including retries; all batches share a 90-second provider budget for the run. Valid items are retained and only rejected items retry, after a 150ms backoff. Exhausted budgets produce labelled fallback decisions; plausible acute medical emergencies always receive scripted fallback priority. No model calls happen during propagation or in the local counterfactual. The existing single-exchange output budget stays unchanged; batches explicitly request 4096 tokens.

Request, completed history and response identity checks prevent cross-assignment. Output remains precedence and short words, never controls or routes. Raw curl status/exit/timing and per-batch acceptance, retry and fallback causes are logged without credentials. `--decision-log FILE` retains every accepted line pair and its actual source.

## Five live seeds

All **1,200/1,200 decisions came from the model (100%)**, all 1,200 trips completed, with zero candidate and baseline collisions and no fallback decisions. Each seed beats FCFS in median emergency gain and the keyword baseline's 100% cumulative liar grant rate. Third-claim grants are 0%; cumulative grants are 33.333%, because previously unknown first claims win.

| Seed | Wall s | Model % | Gain vs FCFS s | Extra vs local s | Others mean / p95 s | Liar cumulative / third | Collateral | Collisions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 35.081 | 100 | 1.500 | 2.875 | 13.425 / 68.250 | 33.333% / 0% | 0.000% | 0 |
| 2 | 46.131 | 100 | 0.125 | 0.125 | 18.963 / 144.000 | 33.333% / 0% | 0.000% | 0 |
| 3 | 35.494 | 100 | 1.875 | 1.000 | 9.297 / 120.250 | 33.333% / 0% | 0.000% | 0 |
| 4 | 32.543 | 100 | 0.625 | 2.875 | 2.911 / 89.750 | 33.333% / 0% | 0.000% | 0 |
| 5 | 42.226 | 100 | 10.125 | 3.625 | 6.318 / 71.750 | 33.333% / 0% | 1.205% | 0 |

[Five-run score index](gates/phase-4-live.jsonl); sibling files retain every decision and timing log.

Seed 1: 22 batches, 22 HTTP calls, 0 retries. Seed 2: 23 batches, 25 HTTP calls, 2 retries. Seed 3: 22 batches, 22 HTTP calls, 0 retries. Seed 4: 23 batches, 23 HTTP calls, 0 retries. Seed 5: 23 batches, 27 HTTP calls, 4 retries. Retries followed rejected speech/parse contracts; accepted results recovered inside their batch deadline. No transport timeout or 180-second wall-time exit recurred. The historical stall was **not reproduced in ten Phase 0 live demos plus six live rush-hour runs** (one exploratory and these five gates). Its original exact exchange remains unknown; the new curl/batch timing makes a recurrence diagnosable. Synthetic slow-provider tests enforce the deadline independently of curl's own timeout.

## Ten exact model line pairs

Requester / other, copied from seed 1. All strings satisfy the eight-word contract. The first three grant lines can still be generic; the accompanying explanations identify the medical reason.

1. Car 1070, grant, t=45: “I will go now thank you” / “I will wait, wishing you care”
2. Car 1235, grant, t=45: “I will go now to help” / “Take your turn I will wait”
3. Car 1083, grant, t=45: “I will go carefully now” / “Please go, I will wait”
4. Car 1029, deny, t=45: “Fair, I'll wait my turn” / “Coffee can wait, appreciate your patience”
5. Car 1009, deny, t=45: “Fair, I'll wait my turn” / “Your commute matters, please wait kindly”
6. Car 1048, deny, t=45: “Fair, I'll wait my turn” / “Meeting can wait, thank you kindly”
7. Car 1016, deny, t=45: “Fair, I'll wait my turn” / “Gym can wait, thank you kindly”
8. Car 1024, deny, t=795: “Fair, I'll wait my turn” / “Kindly prior gym visit undermines chest pain claim”
9. Car 1007, deny, t=1545: “Fair, I'll wait my turn” / “Sorry, past gym trips undermine chest pain claim”
10. Car 1015, deny, t=1545: “Fair, I'll wait my turn” / “Sorry, past gym trips undermine chest pain claim”

## Commands and verification

```sh
cargo test --offline
cargo build --offline
python3 scripts/rush-hour-live-five.py --output /tmp/rush-live-five.jsonl
./target/debug/malleable --rush-hour --headless --policy live --seed 1 --require-live --score /tmp/live.json --decision-log /tmp/decisions.json
cargo fmt --check
git diff --check
```

126 tests pass (three existing external-live tests ignored). New tests exercise two batches for 24 simultaneous requests, one invalid item retried alone, response IDs, reversed roles, exhausted budgets, input blocking and a 150ms provider budget against a deliberately slow loopback server. The exact seizure sentence remains a grant on fallback. Judge/schema and actual key-value scans pass; sticky stays RED/RED/GREEN time_home=69.

## Still broken / limits

The minimum measured gain is 0.125s, so these five seeds do not establish a broad performance guarantee. A busy provider can exhaust the shared budget and lower the model percentage; fallback remains explicit and may let a previously unreliable medical claimant through. Some batched grant speech is generic despite the prompt, while typed-reason speech keeps its separate stricter reason-reference parser. Simulation time pauses during a headless policy call; wall time is reported separately and never treated as simulated saved travel time. Dense-traffic board interaction and a saved live rush-hour recording remain Phase 5.
