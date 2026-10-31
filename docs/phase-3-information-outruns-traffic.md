# Phase 3: information outruns traffic

Stacked on #22, against main. No merge.

A grant publishes one deterministic route bulletin. It relays through cars and junctions along the planned route at 10 times the maximum vehicle speed (150m/s), with 0.1s per hop. Recipients compare their next planned movement with the beneficiary's path and estimated arrival over four minutes. Queued traffic ahead receives precedence to clear; conflicting approaches hold only around the arrival window. Cars can yield an unused crossing reservation only before entry and with comfortable stopping distance. Nobody revokes an occupied crossing or commands a velocity. All integration, following, gaps, exit room and crossing occupancy use the existing kernel.

The first controller lost time by leaving unused conflicting reservations in place. Broad or persistent holds made queues worse and were discarded. The final controller safely yields those unused reservations. The model is finished after grant/deny: 240 decisions total, regardless of message count. The local comparison replays exactly those decisions, with no extra model calls. Both modes use corrected along-link distances for neighbouring cars and the same precedence tiers.

The rush-hour map has the same topology at three times its original geometric scale. Car dimensions, speeds, gaps and physics remain in metres; legacy map geometry and trajectories stay unchanged. This explicitly gives the experiment kilometre-scale routes rather than relabelling the small legacy map.

## Measured five-seed gate

All 240 trips complete for each spread/local pair and FCFS baseline, with zero collisions. Every seed has strictly positive median extra emergency gain. Collateral is below 10% on every seed. Cumulative liar grants remain 33.333%; third-claim grants remain zero.

| Seed | Gain vs FCFS s | Extra vs local s | Cleared ahead m / s | Farthest told m / seconds early | Collateral | Collisions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1.500 | 2.875 | 769.7 / 95 | 1981.9 / 291.2 | 0% | 0 |
| 2 | 0.125 | 0.125 | 933.0 / 105 | 1779.3 / 235.2 | 0% | 0 |
| 3 | 1.875 | 1.000 | 699.2 / 71 | 1870.6 / 252.7 | 0% | 0 |
| 4 | 0.625 | 2.875 | 577.2 / 61 | 1970.0 / 212.5 | 0% | 0 |
| 5 | 10.125 | 3.625 | 893.5 / 98 | 1940.4 / 228.7 | 1.205% | 0 |

[Raw paired scores and measured wall times](gates/phase-3-propagation.jsonl).

Clearance uses actual recorded motion: a recipient's rear clears the crossing, or it stops outside it, before the emergency reaches that point. Metres and seconds belong to the same farthest observed clearance. Lead distance measures a real recipient present at receipt and actual subsequent beneficiary arrival. These are maximum witnessed distances, not averages or guarantees. Frames are recorded each second.

Collateral is the fraction of cars delayed by more than a quarter second versus local whose planned route neither shares a link nor conflicts at a crossing with any overlapping active granted route. It includes indirect delays, but this dense network makes many routes relevant. It does **not** say other drivers have low delay: their mean extra cost versus FCFS is 2.911–18.963s; p95 is retained in the raw scores.

## Commands

```sh
cargo test --offline
cargo test --offline --test propagation
cargo build --offline
python3 scripts/propagation-gate.py --output /tmp/propagation-five-seeds.jsonl
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --score /tmp/spread.json
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --local-only --score /tmp/local.json
# Configurable propagation and forecast horizon:
./target/debug/malleable --rush-hour --headless --policy mock --seed 1 --message-speed 10 --hop-delay 0.1 --lookahead-minutes 4
cargo fmt --check
git diff --check
```

122 tests pass (three existing external live tests ignored). Regressions verify ordered causal hops, grants gating messages, no self-holds, exactly 240 policy calls including the counterfactual, valid configurations and safe completion. Protected judge/schema files and key-value scans pass. Sticky stays RED/RED/GREEN time_home=69.

## Still broken / limits

The seed 2 benefit is only 0.125s; five-seed evidence is narrow and does not guarantee general improvement. Forecast ETA uses route speed limits, not an oracle for future congestion. Granting many competing routes can still add substantial delay. Relays are abstract route-ordered delivery, not a radio-range or radio-latency model. No lane change or reroute is implemented; clearing and yielding use existing movements. This phase is mock evidence; live batching and the dense-traffic board/recording remain Phases 4 and 5.
